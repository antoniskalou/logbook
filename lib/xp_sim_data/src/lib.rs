use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct SimData {
    pub icao: String,
    pub name: String,
    pub registration: String,
    pub latitude: f64,
    pub longitude: f64,
    pub engine_on: bool,
    pub on_ground: bool,
}

impl SimData {
    pub fn from_csv(csv: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let mut wrt = csv::ReaderBuilder::new()
            .has_headers(false)
            .terminator(csv::Terminator::CRLF)
            .from_reader(csv.as_bytes());
        let mut iter = wrt.deserialize();
        if let Some(result) = iter.next() {
            Ok(result?)
        } else {
            Err("Invalid CSV record".into())
        }
    }

    pub fn to_csv(&self) -> Result<String, Box<dyn std::error::Error>> {
        let mut wrt = csv::WriterBuilder::new()
            .has_headers(false)
            .terminator(csv::Terminator::CRLF)
            .from_writer(vec![]);
        wrt.serialize(self)?;
        wrt.flush()?;
        Ok(String::from_utf8(wrt.into_inner()?)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_csv() {
        let csv = "CL60,Challenger 650,C-FAAV,32.000123,42.000123,false,true";
        let sim_data = SimData::from_csv(csv).unwrap();
        assert_eq!(sim_data.icao, String::from("CL60"));
        assert_eq!(sim_data.name, String::from("Challenger 650"));
        assert_eq!(sim_data.registration, String::from("C-FAAV"));
        assert_eq!(sim_data.latitude, 32.000123);
        assert_eq!(sim_data.longitude, 42.000123);
        assert!(!sim_data.engine_on);
        assert!(sim_data.on_ground);
    }

    #[test]
    fn test_to_csv() {
        let sim_data = SimData {
            icao: String::from("CL60"),
            name: String::from("Challenger 650"),
            registration: String::from("C-FAAV"),
            latitude: 32.000123,
            longitude: 42.000123,
            engine_on: false,
            on_ground: true,
        };
        let csv = sim_data.to_csv().unwrap();
        assert_eq!(csv, String::from("CL60,Challenger 650,C-FAAV,32.000123,42.000123,false,true\n"));
    }
}
