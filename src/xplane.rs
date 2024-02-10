use std::{ffi, io::{self, Read}, net::TcpStream, time::Duration};
use geo::LatLon;
use crate::{aircraft::Aircraft, sim_connection::{SimConnection, SimMessage}};

pub const SERVER_ADDR: &str = "127.0.0.1:52000";

pub struct Xplane(TcpStream);

impl Xplane {
    pub fn connect() -> Result<Self, io::Error> {
        // todo: attempt reconnect if closed
        let conn = TcpStream::connect(SERVER_ADDR)?;
        conn.set_read_timeout(Some(Duration::from_secs(1)))?;
        Ok(Xplane(conn))
    }

    fn fetch_messages(
        &self,
        buf: &[u8]
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let str = match ffi::CStr::from_bytes_until_nul(buf) {
            // buffer contains nulls, we can treat it as a CString
            Ok(c_str) => String::from(c_str.to_str()?),
            // buffer has no nulls, read the entire thing
            Err(_) => String::from_utf8_lossy(buf).to_string(),
        };
        let messages = str
            .lines()
            .map(String::from)
            .collect::<Vec<String>>();
        Ok(messages)
    }
}

impl SimConnection for Xplane {
    type Error = Box<dyn std::error::Error>;

    fn next_message(&mut self) -> Result<SimMessage, Self::Error> {
        let mut buf = [0; 256];
        match self.0.read(&mut buf) {
            Ok(_) => {
                let messages = self.fetch_messages(&buf)?;
                println!("received {messages:?}");
                // todo: send all messages to caller
                let msg = messages.last().unwrap();
                let sim_data = SimData::from_csv(msg).unwrap();
                Ok(SimMessage::SimData(Aircraft::from(sim_data)))
            }
            Err(ref e) if e.kind() == io::ErrorKind::TimedOut => {
                Ok(SimMessage::Waiting)
            }
            Err(ref e) if e.kind() == io::ErrorKind::ConnectionAborted => {
                Ok(SimMessage::Quit)
            }
            Err(e) => Err(Box::new(e))
        }
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
        debug_assert!(record.len() >= 7);
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
            position: LatLon::new(sim_data.latitude, sim_data.longitude),
            engine_on: sim_data.engine_on,
            on_ground: sim_data.on_ground,
        }
    }
}
