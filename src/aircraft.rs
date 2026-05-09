use geo::LatLon;

#[derive(Clone, Debug)]
pub struct Aircraft {
    pub title: String,
    pub icao: String,
    pub registration: String,
    pub position: LatLon,
    pub engine_on: bool,
    pub on_ground: bool,
}

impl Aircraft {
    pub fn is_same_airframe(&self, other: &Aircraft) -> bool {
        self.registration == other.registration && self.title == other.title
    }
}
