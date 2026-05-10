use crate::aircraft::Aircraft;
use crate::sim_connection::{SimConnection, SimMessage};
use chrono::{DateTime, Utc};
use geo::LatLon;
use indicatif::{ProgressBar, ProgressStyle};
use log::debug;
use rusqlite::OptionalExtension;
use std::str::FromStr;
use std::{error::Error, fs::File, path::Path};

mod aircraft;
mod msfs;
mod sim_connection;
mod xplane;

// some fields aren't used, but are useful for debugging
#[allow(dead_code)]
#[derive(Clone, Debug)]
struct Airport {
    id: i64,
    ident: String,
    position: LatLon,
}

#[derive(Clone, Debug)]
enum AirportOrCoord {
    Airport(Airport),
    Position(LatLon),
}

impl std::fmt::Display for AirportOrCoord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            AirportOrCoord::Airport(airport) => airport.ident.clone(),
            AirportOrCoord::Position(latlon) => latlon.as_compact(),
        };
        write!(f, "{}", s)
    }
}

fn search_within(
    navdata: &rusqlite::Connection,
    origin: LatLon,
) -> Result<Option<Airport>, Box<dyn Error>> {
    let mut stmt = navdata.prepare(
        "
select airport_id, ident, laty, lonx
  from airport
  where airport_id in (
    select airport_id from airport_coords where
        left_lonx <= ?1 and right_lonx >= ?1 and
        bottom_laty <= ?2 and top_laty >= ?2
  );
    ",
    )?;
    stmt.query_row([origin.longitude(), origin.latitude()], |row| {
        Ok(Airport {
            id: row.get(0)?,
            ident: row.get(1)?,
            position: LatLon::new(row.get(2)?, row.get(3)?),
        })
    })
    // it is acceptable to not receive a record
    .optional()
    // convert rusqlite::Error into error::Error
    .map_err(|e| e.into())
}

pub const DATE_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

fn date_to_string(dt: &DateTime<Utc>) -> String {
    dt.format(DATE_FORMAT).to_string()
}

#[derive(Clone, Copy, Debug)]
enum FlightState {
    Preflight,
    Taxi,
    EnRoute,
    Landed,
    Complete,
}

#[derive(Clone, Debug)]
struct Flight {
    aircraft: Aircraft,
    state: FlightState,
    taxi_out: Option<DateTime<Utc>>,
    departure: Option<(AirportOrCoord, DateTime<Utc>)>,
    arrival: Option<(AirportOrCoord, DateTime<Utc>)>,
    shutdown: Option<DateTime<Utc>>,
}

impl Flight {
    fn new(aircraft: &Aircraft) -> Self {
        Flight {
            aircraft: aircraft.clone(),
            state: FlightState::Preflight,
            taxi_out: None,
            departure: None,
            arrival: None,
            shutdown: None,
        }
    }

    fn arrive(&mut self, airport: Option<&Airport>, time: &DateTime<Utc>) {
        let arrival = self.airport_or_coord(airport);
        self.arrival = Some((arrival, *time));
    }

    fn depart(&mut self, airport: Option<&Airport>, time: &DateTime<Utc>) {
        let departure = self.airport_or_coord(airport);
        self.departure = Some((departure, *time));
    }

    fn to_record(&self) -> Vec<Option<String>> {
        vec![
            Some(self.aircraft.title.clone()),
            Some(self.aircraft.icao.clone()),
            Some(self.aircraft.registration.clone()),
            self.taxi_out.map(|dt| date_to_string(&dt)),
            self.departure.as_ref().map(|d| d.0.to_string()),
            self.departure.as_ref().map(|d| date_to_string(&d.1)),
            self.arrival.as_ref().map(|a| a.0.to_string()),
            self.arrival.as_ref().map(|a| date_to_string(&a.1)),
            self.shutdown.map(|dt| date_to_string(&dt)),
        ]
    }

    fn airport_or_coord(&self, airport: Option<&Airport>) -> AirportOrCoord {
        airport
            .map(|a| AirportOrCoord::Airport(a.clone()))
            .unwrap_or_else(|| AirportOrCoord::Position(self.aircraft.position))
    }
}

pub const CSV_HEADER: [&str; 9] = [
    "Aircraft Name",
    "Aircraft ICAO",
    "Registration",
    "Taxi Time",
    "Departure ICAO",
    "Departure Time",
    "Arrival ICAO",
    "Arrival Time",
    "Shutdown Time",
];

struct Logbook(File);

impl Logbook {
    fn new(path: &Path) -> Result<Self, Box<dyn Error>> {
        let should_add_header = !path.exists();
        let f = File::options().create(true).append(true).open(path)?;

        if should_add_header {
            csv::Writer::from_writer(&f).write_record(CSV_HEADER)?;
        }

        Ok(Logbook(f))
    }

    fn log(&mut self, flight: &Flight) -> Result<(), Box<dyn Error>> {
        let mut csv = csv::Writer::from_writer(&self.0);
        // change None to ""
        for field in flight.to_record() {
            csv.write_field(field.unwrap_or("".to_string()))?;
        }
        csv.write_record(None::<&[u8]>)?;
        csv.flush()?;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug)]
enum Simulator {
    Msfs,
    Xp12,
}

impl FromStr for Simulator {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_uppercase().as_str() {
            "MSFS" | "MSFS2020" | "MSFS2024" => Ok(Simulator::Msfs),
            "XP12" | "XPLANE" | "X-PLANE" => Ok(Simulator::Xp12),
            _ => Err(format!(
                "Invalid simulator '{}'. Valid options are: MSFS, XP12",
                s
            )),
        }
    }
}

fn pick_sim() -> Simulator {
    std::env::args()
        .nth(1)
        .expect("USAGE: logbook.exe <SIM NAME>")
        .parse()
        .unwrap()
}

fn make_progress_spinner() -> ProgressBar {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(ProgressStyle::with_template("{spinner:.green} {msg}").unwrap());
    spinner.enable_steady_tick(std::time::Duration::from_millis(120));
    spinner.set_message("Waiting for simulator connection...");
    spinner
}

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let sim_choice = pick_sim();
    let navdata_path = match sim_choice {
        Simulator::Msfs => "navdata/msfs.sqlite",
        Simulator::Xp12 => "navdata/xp12.sqlite",
    };
    let navdata = rusqlite::Connection::open(navdata_path)?;
    navdata.execute(
        "
        create virtual table if not exists airport_coords using rtree(
            airport_id, left_lonx, right_lonx, bottom_laty, top_laty
        )
    ",
        (),
    )?;
    navdata.execute(
        "
        insert or ignore into airport_coords
            select airport_id, left_lonx, right_lonx, bottom_laty, top_laty from airport
    ",
        (),
    )?;

    let mut sim: Box<dyn SimConnection<Error = Box<dyn std::error::Error>>> = match sim_choice {
        Simulator::Msfs => Box::new(msfs::Msfs::connect()),
        Simulator::Xp12 => Box::new(xplane::Xplane::connect()?),
    };
    let mut logbook = Logbook::new(Path::new("logbook.csv"))?;
    let mut current_flight: Option<Flight> = None;

    let mut progress_spinner = make_progress_spinner();
    loop {
        match sim.next_message() {
            Ok(SimMessage::SimData(aircraft)) => {
                // stop spinner, sim is ready!
                if !progress_spinner.is_finished() {
                    progress_spinner.finish_with_message("🔗 Simulator connected");
                }

                // reset flight if aircraft is changed
                if let Some(flight) = &current_flight {
                    if !flight.aircraft.is_same_airframe(&aircraft) {
                        current_flight = None;
                    }
                }

                // initialize current flight if there isn't one
                let flight = current_flight.get_or_insert_with(|| {
                    println!("\n\n✈  {} — {}", aircraft.registration, aircraft.title);
                    Flight::new(&aircraft)
                });
                debug!("{:?}", flight);

                match flight.state {
                    FlightState::Preflight => {
                        if aircraft.engine_on {
                            let time = Utc::now();
                            flight.taxi_out = Some(time);
                            flight.state = FlightState::Taxi;

                            println!("🟢 Engine(s) started at {}", time.format("%H:%M UTC"));
                        }
                    }
                    FlightState::Taxi => {
                        if !aircraft.on_ground {
                            let closest_airport = search_within(&navdata, aircraft.position)?;
                            flight.depart(closest_airport.as_ref(), &Utc::now());
                            flight.state = FlightState::EnRoute;

                            if let Some((airport, time)) = &flight.departure {
                                println!("🛫 Departed {} at {}", airport, time.format("%H:%M UTC"));
                            }
                        }
                    }
                    FlightState::EnRoute => {
                        if aircraft.on_ground {
                            let closest_airport = search_within(&navdata, aircraft.position)?;
                            flight.arrive(closest_airport.as_ref(), &Utc::now());
                            flight.state = FlightState::Landed;

                            if let Some((airport, time)) = &flight.arrival {
                                println!("🛬 Arrived {} at {}", airport, time.format("%H:%M UTC"));
                            }
                        }
                    }
                    FlightState::Landed => {
                        if !aircraft.on_ground {
                            println!("🔄 Go-around / touch-and-go detected");
                            // did a touch and go or a go around
                            flight.state = FlightState::EnRoute;
                        } else if !aircraft.engine_on {
                            flight.shutdown = Some(Utc::now());
                            flight.state = FlightState::Complete;

                            if let Some(time) = flight.shutdown {
                                println!("🛑 Engine(s) shutdown at {}", time.format("%H:%M UTC"));
                            }
                        }
                    }
                    FlightState::Complete => {
                        println!("✅ Flight logged");
                        // store record
                        logbook.log(flight)?;
                        // reset flight
                        current_flight = None;
                    }
                }
            }
            Ok(SimMessage::Connecting) => {
                if progress_spinner.is_finished() {
                    progress_spinner = make_progress_spinner();
                }
            }
            Ok(SimMessage::Disconnected) => {
                println!("⚠  Lost simulator connection");
                progress_spinner = make_progress_spinner();
            }
            msg => debug!("Unhandled message received: {:?}", msg),
        }
    }
}
