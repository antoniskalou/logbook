use crate::{
    aircraft::Aircraft,
    sim_connection::{SimConnection, SimMessage},
};
use geo::LatLon;
use xp_sim_data::SimData;
use std::{
    io::{self, Read},
    net::TcpStream,
    time::Duration,
};

pub const SERVER_ADDR: &str = "127.0.0.1:52000";

pub struct Xplane {
    conn: TcpStream,
}

impl Xplane {
    pub fn connect() -> Result<Self, io::Error> {
        // todo: attempt reconnect if closed
        let conn = TcpStream::connect(SERVER_ADDR)?;
        conn.set_read_timeout(Some(Duration::from_secs(1)))?;
        Ok(Xplane { conn, })
    }
}

impl SimConnection for Xplane {
    type Error = Box<dyn std::error::Error>;

    fn next_message(&mut self) -> Result<SimMessage, Self::Error> {
        match read_packet(&mut self.conn) {
            Ok(buf) => {
                let msg = std::str::from_utf8(&buf)?;
                let sim_data = SimData::from_csv(msg)?;
                Ok(SimMessage::SimData(Aircraft::from(sim_data)))
            }
            Err(ref e) if e.kind() == io::ErrorKind::TimedOut => Ok(SimMessage::Waiting),
            Err(ref e) if e.kind() == io::ErrorKind::ConnectionAborted => Ok(SimMessage::Quit),
            Err(e) => Err(Box::new(e)),
        }
    }
}

fn read_packet(stream: &mut TcpStream) -> Result<Vec<u8>, std::io::Error> {
    let packet_size = next_packet_size(stream)?;
    let mut buf = vec![0; packet_size];
    stream.read_exact(&mut buf)?;
    Ok(buf)
}

fn next_packet_size(stream: &mut TcpStream) -> Result<usize, std::io::Error> {
    let mut buf = [0; 2];
    stream.read_exact(&mut buf)?;
    Ok(u16::from_le_bytes(buf) as usize)
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
