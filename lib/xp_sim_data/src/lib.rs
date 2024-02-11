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
            .from_writer(vec![]);
        wrt.serialize(self)?;
        wrt.flush()?;
        Ok(String::from_utf8(wrt.into_inner()?)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
}
