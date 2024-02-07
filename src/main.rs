use std::{error::Error, fs::File, thread, time, path::Path, ptr, io::Write};
use chrono::{DateTime, Utc};
use rusqlite::OptionalExtension;
use geo::LatLon;
use simconnect::{DispatchResult, SimConnector};
use crate::sim_string::{SimString, SimStringError};

mod sim_string;

/// All the data we want to fetch from the sim
// #[derive(Debug)]
// rust adds padding to the the struct, pack it to avoid adding extra nulls
#[repr(C, packed)]
struct RawSimData {
    // https://docs.flightsimulator.com/html/Programming_Tools/SimVars/Aircraft_SimVars/Aircraft_Misc_Variables.htm
    title: SimString<128>,
    eng_combustion_1: f64,
    eng_combustion_2: f64,
    eng_combustion_3: f64,
    eng_combustion_4: f64,
    latitude: f64,
    longitude: f64,
    // https://docs.flightsimulator.com/html/Programming_Tools/SimVars/Miscellaneous_Variables.htm
    sim_on_ground: f64,
    // https://docs.flightsimulator.com/html/Programming_Tools/SimVars/Aircraft_SimVars/Aircraft_RadioNavigation_Variables.htm
    // may or may not contain aircraft registration
    atc_id: SimString<32>,
}

#[derive(Clone, Debug)]
struct Aircraft {
    title: String,
    position: LatLon,
    registration: String,
    engines_on: [bool; 4],
    on_ground: bool,
}

impl Aircraft {
    fn any_engine_on(&self) -> bool {
        self.engines_on.contains(&true)
    }
}

impl TryFrom<RawSimData> for Aircraft {
    type Error = SimStringError;

    fn try_from(raw: RawSimData) -> Result<Self, Self::Error> {
        Ok(Self {
            title: raw.title.to_string()?,
            position: LatLon::from_radians(raw.latitude, raw.longitude),
            // TODO: consider fetching data from other sources first
            registration: raw.atc_id.to_string()?,
            engines_on: [
                raw.eng_combustion_1 != 0.0,
                raw.eng_combustion_2 != 0.0,
                raw.eng_combustion_3 != 0.0,
                raw.eng_combustion_4 != 0.0,
            ],
            on_ground: raw.sim_on_ground != 0.0,
        })
    }
}

// some fields aren't used, but are useful for debugging
#[allow(dead_code)]
#[derive(Clone, Debug)]
struct Airport {
    id: i64,
    ident: String,
    position: LatLon,
}

fn search_within(
    navdata: &rusqlite::Connection,
    origin: LatLon,
) -> Result<Option<Airport>, Box<dyn Error>> {
    let mut stmt = navdata.prepare("
select airport_id, ident, laty, lonx
  from airport
  where airport_id in (
    select airport_id from airport_coords where
        left_lonx <= ?1 and right_lonx >= ?1 and
        bottom_laty <= ?2 and top_laty >= ?2
  );
    ")?;
    stmt
        .query_row([origin.longitude(), origin.latitude()], |row| {
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
    aircraft: String,
    registration: String,
    state: FlightState,
    taxi_out: Option<DateTime<Utc>>,
    departure: Option<(Airport, DateTime<Utc>)>,
    arrival: Option<(Airport, DateTime<Utc>)>,
    shutdown: Option<DateTime<Utc>>,
}

impl Flight {
    fn new(aircraft: &str, registration: &str) -> Self {
        Flight {
            aircraft: aircraft.to_owned(),
            registration: registration.to_owned(),
            state: FlightState::Preflight,
            taxi_out: None,
            departure: None,
            arrival: None,
            shutdown: None,
        }
    }

    fn arrive(&mut self, airport: &Airport, time: &DateTime<Utc>) {
        self.arrival = Some((airport.clone(), *time));
    }

    fn depart(&mut self, airport: &Airport, time: &DateTime<Utc>) {
        self.departure = Some((airport.clone(), *time));
    }

    fn to_record(&self) -> Vec<Option<String>> {
        vec![
            Some(self.aircraft.clone()),
            Some(self.registration.clone()),
            self.taxi_out.map(|dt| date_to_string(&dt)),
            self.departure.clone().map(|d| d.0.ident),
            self.departure.clone().map(|d| date_to_string(&d.1)),
            self.arrival.clone().map(|a| a.0.ident),
            self.arrival.clone().map(|a| date_to_string(&a.1)),
            self.shutdown.map(|dt| date_to_string(&dt)),
        ]
    }
}

pub const CSV_HEADER: [&str; 8] = [
    "Aircraft",
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
        let f = File::options()
            .create(true)
            .append(true)
            .open(path)?;

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

fn main() -> Result<(), Box<dyn Error>> {
    let navdata = rusqlite::Connection::open("navdata.sqlite")?;
    navdata.execute("
        create virtual table if not exists airport_coords using rtree(
            airport_id, left_lonx, right_lonx, bottom_laty, top_laty
        )
    ", ())?;
    navdata.execute("
        insert or ignore into airport_coords
            select airport_id, left_lonx, right_lonx, bottom_laty, top_laty from airport
    ", ())?;

    let mut conn = SimConnector::new();
    conn.connect("Logbook");
    conn.add_data_definition(
        0,
        "TITLE",
        "",
        simconnect::SIMCONNECT_DATATYPE_SIMCONNECT_DATATYPE_STRING128,
        u32::MAX,
        0.0
    );
    conn.add_data_definition(
        0,
        "ENG COMBUSTION:1",
        "Boolean",
        simconnect::SIMCONNECT_DATATYPE_SIMCONNECT_DATATYPE_FLOAT64,
        u32::MAX,
        0.0
    );
    conn.add_data_definition(
        0,
        "ENG COMBUSTION:2",
        "Boolean",
        simconnect::SIMCONNECT_DATATYPE_SIMCONNECT_DATATYPE_FLOAT64,
        u32::MAX,
        0.0
    );
    conn.add_data_definition(
        0,
        "ENG COMBUSTION:3",
        "Boolean",
        simconnect::SIMCONNECT_DATATYPE_SIMCONNECT_DATATYPE_FLOAT64,
        u32::MAX,
        0.0
    );
    conn.add_data_definition(
        0,
        "ENG COMBUSTION:4",
        "Boolean",
        simconnect::SIMCONNECT_DATATYPE_SIMCONNECT_DATATYPE_FLOAT64,
        u32::MAX,
        0.0
    );
    conn.add_data_definition(
        0,
        "PLANE LATITUDE",
        "Radians",
        simconnect::SIMCONNECT_DATATYPE_SIMCONNECT_DATATYPE_FLOAT64,
        u32::MAX,
        0.0
    );
    conn.add_data_definition(
        0,
        "PLANE LONGITUDE",
        "Radians",
        simconnect::SIMCONNECT_DATATYPE_SIMCONNECT_DATATYPE_FLOAT64,
        u32::MAX,
        0.0
    );
    conn.add_data_definition(
        0,
        "SIM ON GROUND",
        "Boolean",
        simconnect::SIMCONNECT_DATATYPE_SIMCONNECT_DATATYPE_FLOAT64,
        u32::MAX,
        0.0
    );
    conn.add_data_definition(
        0,
        "ATC ID",
        "",
        simconnect::SIMCONNECT_DATATYPE_SIMCONNECT_DATATYPE_STRING32,
        u32::MAX,
        0.0
    );

    // receive data related to the user aircraft
    conn.request_data_on_sim_object(
        0, // request id
        0, // define id
        0, // object id (user)
        simconnect::SIMCONNECT_PERIOD_SIMCONNECT_PERIOD_SECOND,
        0, // flags
        0, // origin
        0, // interval
        0, // limit
    );

    let mut logbook = Logbook::new(Path::new("logbook.csv"))?;
    let mut current_flight: Option<Flight> = None;
    loop {
        match conn.get_next_message() {
            Ok(DispatchResult::SimObjectData(data)) => unsafe {
                if data.dwDefineID == 0 {
                    let sim_data_ptr = ptr::addr_of!(data.dwData) as *const RawSimData;
                    let sim_data_value = ptr::read_unaligned(sim_data_ptr);
                    let aircraft = Aircraft::try_from(sim_data_value).unwrap();

                    let closest_airport = search_within(&navdata, aircraft.position)?;

                    // initialize current flight if there isn't one
                    if current_flight.is_none() {
                        current_flight = Some(Flight::new(&aircraft.title, &aircraft.registration));
                    }

                    let flight = current_flight.as_mut().unwrap();
                    println!("{:?}", flight);
                    match flight.state {
                        FlightState::Preflight => {
                            if aircraft.any_engine_on() {
                                flight.taxi_out = Some(Utc::now());
                                flight.state = FlightState::Taxi;
                            }
                        }
                        FlightState::Taxi => {
                            if !aircraft.on_ground {
                                let airport = closest_airport.expect("invalid takeoff airport");
                                flight.depart(&airport, &Utc::now());
                                flight.state = FlightState::EnRoute;
                            }
                        }
                        FlightState::EnRoute => {
                            if aircraft.on_ground {
                                let airport = closest_airport.expect("invalid landing airport");
                                flight.arrive(&airport, &Utc::now());
                                flight.state = FlightState::Landed;
                            }
                        }
                        FlightState::Landed => {
                            if !aircraft.on_ground {
                                // did a touch and go or a go around
                                flight.state = FlightState::EnRoute;
                            } else if !aircraft.any_engine_on() {
                                flight.shutdown = Some(Utc::now());
                                flight.state = FlightState::Complete;
                            }
                        }
                        FlightState::Complete => {
                            println!("Flight completed!");
                            // store record
                            logbook.log(flight)?;
                            // reset flight
                            current_flight = None;
                        }
                    }
                }
            }
            Ok(DispatchResult::Open(_)) => {
                println!("Simulator connection established.")
            }
            Ok(DispatchResult::Quit(_)) => {
                println!("Simulator connection closed.");
            }
            msg => eprintln!("Unhandled message received: {:?}", msg),
        }

        thread::sleep(time::Duration::from_secs(1));
    }
}
