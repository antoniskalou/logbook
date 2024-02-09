use std::{io::Read, net::TcpStream, time::Duration};
use geo::LatLon;
use crate::{aircraft::Aircraft, sim_connection::{SimConnection, SimMessage}};

pub const SERVER_ADDR: &str = "127.0.0.1:52000";

pub struct Xplane(TcpStream);

impl Xplane {
    pub fn connect() -> Result<Self, std::io::Error> {
        let conn = TcpStream::connect(SERVER_ADDR)?;
        conn.set_read_timeout(Some(Duration::from_millis(100)))?;
        Ok(Xplane(conn))
    }
}

impl SimConnection for Xplane {
    type Error = std::io::Error;

    fn next_message(&mut self) -> Result<SimMessage, Self::Error> {
        let mut buf = String::from("");
        let _ = self.0.read_to_string(&mut buf)?;
        let messages = buf.split_terminator("\r\n");
        // todo: properly parse all messages
        let msg = messages.last().unwrap();
        let sim_data = SimData::from_csv(msg).unwrap();
        Ok(SimMessage::Unknown)
    }
}

#[derive(Debug)]
struct SimData {
    icao: String,
    name: String,
    registration: String,
    latitude: f64,
    longitude: f64,
    engine_on: bool,
    on_ground: bool
}

impl SimData {
    fn from_csv(csv: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let record = csv.split(",").collect::<Vec<&str>>();
        Ok(Self {
            icao: record[0].to_owned(),
            name: record[1].to_owned(),
            registration: record[2].to_owned(),
            latitude: record[3].parse()?,
            longitude: record[4].parse()?,
            engine_on: record[5].parse()?,
            on_ground: record[6].parse()?,
        })
    }

    fn to_csv(&self) -> Result<String, Box<dyn std::error::Error>> {
        let record = [
            self.icao.clone(),
            self.name.clone(),
            self.registration.clone(),
            self.latitude.to_string(),
            self.longitude.to_string(),
            self.engine_on.to_string(),
            self.on_ground.to_string(),
        ];
        Ok(record.join(","))
    }
}

impl From<SimData> for Aircraft {
    fn from(sim_data: SimData) -> Self {
        Self {
            title: sim_data.name,
            icao: sim_data.icao,
            registration: sim_data.registration,
            position: LatLon::from_radians(sim_data.latitude, sim_data.longitude),
            engine_on: sim_data.engine_on,
            on_ground: sim_data.on_ground,
        }
    }
}
