use std::io::Write;

use xplm::data::borrowed::{DataRef, FindError};
use xplm::data::{ArrayRead, DataRead, ReadOnly, StringRead};
use xplm::flight_loop::{FlightLoop, FlightLoopCallback, LoopState};
use xplm::plugin::{Plugin, PluginInfo};
use xplm::{debug, xplane_plugin};

struct FlightLoopHandler {
    tcp_listener: std::net::TcpListener,
    tcp_connections: Vec<std::net::TcpStream>,
    is_in_replay: DataRef<bool, ReadOnly>,
    // datarefs for transfer
    latitude: DataRef<f64, ReadOnly>,
    longitude: DataRef<f64, ReadOnly>,
    icao: DataRef<[u8], ReadOnly>,
    name: DataRef<[u8], ReadOnly>,
    registration: DataRef<[u8], ReadOnly>,
    engine_on: DataRef<[i32], ReadOnly>,
    on_ground: DataRef<bool, ReadOnly>,
}

impl FlightLoopHandler {
    fn new() -> Result<Self, FindError> {
        let tcp_listener = std::net::TcpListener::bind("127.0.0.1:52000").unwrap();
        tcp_listener.set_nonblocking(true).unwrap();

        Ok(Self {
            tcp_listener,
            tcp_connections: vec![],
            is_in_replay: DataRef::find("sim/time/is_in_replay")?,
            latitude: DataRef::find("sim/flightmodel/position/latitude")?,
            longitude: DataRef::find("sim/flightmodel/position/longitude")?,
            icao: DataRef::find("sim/aircraft/view/acf_ICAO")?,
            name: DataRef::find("sim/aircraft/view/acf_ui_name")?,
            registration: DataRef::find("sim/aircraft/view/acf_tailnum")?,
            engine_on: DataRef::find("sim/flightmodel/engine/ENGN_running")?,
            // according to the docs: "User Aircraft is on the ground when this is set to 1"
            on_ground: DataRef::find("sim/flightmodel/failures/onground_any")?,
        })
    }
}

impl FlightLoopCallback for FlightLoopHandler {
    fn flight_loop(&mut self, _: &mut LoopState) {
        if self.is_in_replay.get() {
            // dont do anything if we're replaying, it will break the logging
            debug("Entered replay mode, pausing transmissions.\n");
            return;
        }

        let lat = self.latitude.get();
        let lon = self.longitude.get();
        let icao = self.icao
            .get_as_string()
            .unwrap_or(String::from("Unknown"));
        let name = self.name
            .get_as_string()
            .unwrap_or(String::from("Unknown"));
        let reg = self.registration
            .get_as_string()
            .unwrap_or(String::from("Unknown"));
        let engine_on = self.engine_on
            .as_vec()
            .iter()
            .any(|x| *x == 1);
        let on_ground = self.on_ground.get();

        let record = [
            format!("lat={}", lat.to_string()),
            format!("lon={}", lon.to_string()),
            format!("icao={}", icao),
            format!("name={}", name),
            format!("reg={}", reg),
            format!("engine_on={}", engine_on.to_string()),
            format!("on_ground={}", on_ground.to_string()),
        ];
        let record_str = format!("{}\r\n", record.join(","));

        match self.tcp_listener.accept() {
            Ok((socket, addr)) => {
                debug(format!("new client: {addr:?}\n"));
                socket.set_nonblocking(true).unwrap();
                self.tcp_connections.push(socket);
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {},
            Err(e) => debug(format!("IO error: {e}\n")),
        }

        self.tcp_connections.retain_mut(|stream| {
            match stream.write_all(record_str.as_bytes()) {
                Ok(_) => {
                    debug(format!("Wrote to stream: {stream:?}\n"));
                    true
                },
                // client closed connection
                Err(ref e) if e.kind() == std::io::ErrorKind::ConnectionAborted => {
                    debug(format!("Client closed connection...\n"));
                    false
                }
                Err(e) => {
                    debug(format!("TCP client error: {e}"));
                    false
                }
            }
        });
    }
}

struct LogbookPlugin {
    flight_loop: FlightLoop,
}

impl Plugin for LogbookPlugin {
    type Error = FindError;

    fn start() -> Result<Self, Self::Error> {
        debug("Hello from logbook\n");
        let flight_loop = FlightLoop::new(FlightLoopHandler::new()?);
        Ok(LogbookPlugin { flight_loop, })
    }

    fn enable(&mut self) -> Result<(), Self::Error> {
        self.flight_loop.schedule_after(std::time::Duration::from_secs(1));
        Ok(())
    }

    fn disable(&mut self) {
        self.flight_loop.deactivate();
    }

    fn info(&self) -> PluginInfo {
        PluginInfo {
            name: String::from("Logbook"),
            signature: String::from("antoniskalou.logbook"),
            description: String::from("A plugin for sending log data over TCP"),
        }
    }
}

xplane_plugin!(LogbookPlugin);

#[cfg(test)]
mod tests {
    use super::*;
}
