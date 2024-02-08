use xplm::data::borrowed::{DataRef, FindError};
use xplm::data::{ArrayRead, DataRead, ReadOnly, StringRead};
use xplm::plugin::{Plugin, PluginInfo};
use xplm::{debug, xplane_plugin};

struct LogbookPlugin {
    latitude: DataRef<f64, ReadOnly>,
    longitude: DataRef<f64, ReadOnly>,
    icao: DataRef<[u8], ReadOnly>,
    name: DataRef<[u8], ReadOnly>,
    registration: DataRef<[u8], ReadOnly>,
    engine_on: DataRef<[i32], ReadOnly>,
    on_ground: DataRef<bool, ReadOnly>,
}

impl Plugin for LogbookPlugin {
    type Error = FindError;

    fn start() -> Result<Self, Self::Error> {
        debug("Hello from logbook\n");
        Ok(LogbookPlugin {
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

    fn enable(&mut self) -> Result<(), Self::Error> {
        debug(format!("Latitude: {}\n", self.latitude.get()));
        debug(format!("Longitude: {}\n", self.longitude.get()));

        let icao = self.icao
            .get_as_string()
            .unwrap_or(String::from("Unknown"));
        let name = self.name
            .get_as_string()
            .unwrap_or(String::from("Unknown"));
        let reg = self.registration
            .get_as_string()
            .unwrap_or(String::from("Unknown"));
        debug(format!("ICAO: {}\n", icao));
        debug(format!("Name: {}\n", name));
        debug(format!("Registration: {}\n", reg));
        debug(format!("Engine On? {:?}\n", self.engine_on.as_vec()));
        debug(format!("On ground? {}\n", self.on_ground.get()));

        Ok(())
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
