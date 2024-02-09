use geo::LatLon;

#[derive(Clone, Debug)]
pub struct Aircraft {
    pub title: String,
    pub icao: String,
    pub registration: String,
    pub position: LatLon,
    pub engines_on: [bool; 4],
    pub on_ground: bool,
}

impl Aircraft {
    pub fn any_engine_on(&self) -> bool {
        self.engines_on.contains(&true)
    }
}
