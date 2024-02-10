use std::{
    io::Write,
    net::{SocketAddr, TcpListener, TcpStream},
};
use xplm::data::borrowed::{DataRef, FindError};
use xplm::data::{ArrayRead, DataRead, ReadOnly, StringRead};
use xplm::flight_loop::{FlightLoop, FlightLoopCallback, LoopState};
use xplm::plugin::{Plugin, PluginInfo};
use xplm::xplane_plugin;

/// extension of xplm::debugln! that prints the plugin name before the
/// log message.
macro_rules! debugln {
    () => (xplm::debugln!());
    ($($arg:tt)*) => ({
        xplm::debugln!("[Logbook]: {}", std::format_args!($($arg)*))
    });
}

pub const SERVER_ADDR: &str = "127.0.0.1:52000";

struct FlightLoopHandler {
    tcp_listener: std::net::TcpListener,
    tcp_connections: Vec<(TcpStream, SocketAddr)>,
    is_in_replay: DataRef<bool, ReadOnly>,
    // datarefs for transfer
    icao: DataRef<[u8], ReadOnly>,
    name: DataRef<[u8], ReadOnly>,
    registration: DataRef<[u8], ReadOnly>,
    latitude: DataRef<f64, ReadOnly>,
    longitude: DataRef<f64, ReadOnly>,
    engine_on: DataRef<[i32], ReadOnly>,
    on_ground: DataRef<bool, ReadOnly>,
}

impl FlightLoopHandler {
    fn new() -> Result<Self, FindError> {
        // these should basically never happen, so its fine if the plugin aborts
        let tcp_listener = TcpListener::bind(SERVER_ADDR)
            .unwrap_or_else(|_| panic!("failed to open TCP server on {SERVER_ADDR}"));
        tcp_listener
            .set_nonblocking(true)
            .expect("set_nonblocking failed");

        debugln!("TCP server listening on {SERVER_ADDR}...");

        Ok(Self {
            tcp_listener,
            tcp_connections: vec![],
            is_in_replay: DataRef::find("sim/time/is_in_replay")?,
            icao: DataRef::find("sim/aircraft/view/acf_ICAO")?,
            name: DataRef::find("sim/aircraft/view/acf_ui_name")?,
            registration: DataRef::find("sim/aircraft/view/acf_tailnum")?,
            latitude: DataRef::find("sim/flightmodel/position/latitude")?,
            longitude: DataRef::find("sim/flightmodel/position/longitude")?,
            engine_on: DataRef::find("sim/flightmodel/engine/ENGN_running")?,
            // according to the docs: "User Aircraft is on the ground when this is set to 1"
            on_ground: DataRef::find("sim/flightmodel/failures/onground_any")?,
        })
    }

    fn as_record(&self) -> Vec<String> {
        let icao = self.icao.get_as_string().unwrap_or(String::from("UNKNOWN"));
        let name = self.name.get_as_string().unwrap_or(String::from("UNKNOWN"));
        let reg = self
            .registration
            .get_as_string()
            .unwrap_or(String::from("UNKNOWN"));
        let lat = self.latitude.get().to_string();
        let lon = self.longitude.get().to_string();
        let engine_on = self.engine_on.as_vec().iter().any(|x| *x == 1).to_string();
        let on_ground = self.on_ground.get().to_string();
        // TODO: consider using a better format where field order doesn't matter
        // OR create a new type that's shared between this and the main software
        // that has the same serialization/deserialization methods as this.
        vec![icao, name, reg, lat, lon, engine_on, on_ground]
    }
}

// NOTE: be careful! we can't panic here, it will crash the sim.
//
// in other places of the code we can panic just fine, xplm will handle it.
impl FlightLoopCallback for FlightLoopHandler {
    fn flight_loop(&mut self, _: &mut LoopState) {
        if self.is_in_replay.get() {
            // dont do anything if we're replaying, it will break the logging
            debugln!("entered replay mode, pausing transmissions.");
            return;
        }

        match self.tcp_listener.accept() {
            Ok((socket, addr)) => {
                debugln!("{addr} connected!");
                if socket.set_nonblocking(true).is_ok() {
                    self.tcp_connections.push((socket, addr));
                } else {
                    // should also basically never happen, but we want to be sure
                    // never to panic here
                    debugln!("{addr} WARNING!!! failed to set TCP socket to non_blocking, will ignore the connection");
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(e) => debugln!("could not open listener: {e}"),
        }

        let record_line = format!("{}\r\n", self.as_record().join(","));
        self.tcp_connections.retain_mut(|(stream, addr)| {
            match stream.write_all(record_line.as_bytes()) {
                Ok(_) => true,
                // client closed connection
                Err(ref e) if e.kind() == std::io::ErrorKind::ConnectionAborted => {
                    debugln!("{addr} closed connection...");
                    false
                }
                Err(e) => {
                    debugln!("{addr} client error: {e}");
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
        debugln!("plugin started!");
        let flight_loop = FlightLoop::new(FlightLoopHandler::new()?);
        Ok(LogbookPlugin { flight_loop })
    }

    fn enable(&mut self) -> Result<(), Self::Error> {
        self.flight_loop
            .schedule_after(std::time::Duration::from_secs(1));
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
