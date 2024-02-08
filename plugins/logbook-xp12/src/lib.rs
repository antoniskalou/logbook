use std::{io::Write, net::TcpListener};
use xplm::data::borrowed::{DataRef, FindError};
use xplm::data::{ArrayRead, DataRead, ReadOnly, StringRead};
use xplm::flight_loop::{FlightLoop, FlightLoopCallback, LoopState};
use xplm::plugin::{Plugin, PluginInfo};
use xplm::{debugln, xplane_plugin};

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
        // TODO: handle errors
        let tcp_listener = TcpListener::bind("127.0.0.1:52000").unwrap();
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

    fn as_record(&self) -> Vec<String> {
        let icao = self.icao
            .get_as_string()
            .unwrap_or(String::from("UNKNOWN"));
        let name = self.name
            .get_as_string()
            .unwrap_or(String::from("UNKNOWN"));
        let reg = self.registration
            .get_as_string()
            .unwrap_or(String::from("UNKNOWN"));
        let lat = self.latitude.get().to_string();
        let lon = self.longitude.get().to_string();
        let engine_on = self.engine_on
            .as_vec()
            .iter()
            .any(|x| *x == 1)
            .to_string();
        let on_ground = self.on_ground.get().to_string();
        vec![icao, name, reg, lat, lon, engine_on, on_ground]
    }
}

impl FlightLoopCallback for FlightLoopHandler {
    fn flight_loop(&mut self, _: &mut LoopState) {
        if self.is_in_replay.get() {
            // dont do anything if we're replaying, it will break the logging
            debugln!("Entered replay mode, pausing transmissions.");
            return;
        }

        match self.tcp_listener.accept() {
            Ok((socket, addr)) => {
                debugln!("new client: {addr:?}");
                socket.set_nonblocking(true).unwrap();
                self.tcp_connections.push(socket);
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {},
            Err(e) => debugln!("IO error: {e}"),
        }

        let record_line = format!("{}\r\n", self.as_record().join(","));
        self.tcp_connections.retain_mut(|stream| {
            match stream.write_all(record_line.as_bytes()) {
                Ok(_) => {
                    debugln!("Wrote to stream: {stream:?}");
                    true
                },
                // client closed connection
                Err(ref e) if e.kind() == std::io::ErrorKind::ConnectionAborted => {
                    debugln!("Client closed connection...");
                    false
                }
                Err(e) => {
                    debugln!("TCP client error: {e}");
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
        debugln!("Logbook plugin started.");
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
