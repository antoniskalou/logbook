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
