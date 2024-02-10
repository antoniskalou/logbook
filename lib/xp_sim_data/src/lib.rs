#[derive(Debug)]
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
        let record = csv.split(",").collect::<Vec<&str>>();
        if record.len() < 7 {
            return Err("Invalid CSV record".into());
        }
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

    pub fn to_csv(&self) -> String {
        let record = [
            self.icao.clone(),
            self.name.clone(),
            self.registration.clone(),
            self.latitude.to_string(),
            self.longitude.to_string(),
            self.engine_on.to_string(),
            self.on_ground.to_string(),
        ];
        record.join(",")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
}
