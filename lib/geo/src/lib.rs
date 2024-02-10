use geographiclib_rs::{DirectGeodesic, Geodesic, InverseGeodesic};

use crate::math::heading_to_point;

mod math;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Cardinal {
    North,
    South,
    East,
    West,
}

impl std::fmt::Display for Cardinal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Cardinal::North => "N",
            Cardinal::South => "S",
            Cardinal::East => "E",
            Cardinal::West => "W",
        };
        write!(f, "{}", s)
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct DMS {
    pub degrees: u16,
    pub minutes: u8,
    pub seconds: f64,
    pub cardinal: Option<Cardinal>,
}

impl DMS {
    pub fn new(degrees: u16, minutes: u8, seconds: f64, cardinal: Cardinal) -> Self {
        Self {
            degrees,
            minutes,
            seconds,
            cardinal: Some(cardinal),
        }
    }

    pub fn from_degrees(degrees: f64) -> Self {
        let d = degrees.abs().floor();
        let m = ((degrees.abs() - d) * 60.0).floor();
        let s = (degrees.abs() - d - m / 60.0) * 3600.0;

        Self {
            degrees: d as u16,
            minutes: m as u8,
            seconds: s,
            cardinal: None,
        }
    }

    pub fn from_degrees_latitude(lat: f64) -> Self {
        let mut d = Self::from_degrees(lat);

        if lat < 0.0 {
            d.cardinal = Some(Cardinal::South);
        } else {
            d.cardinal = Some(Cardinal::North);
        }

        d
    }

    pub fn from_degrees_longitude(lon: f64) -> Self {
        let mut d = Self::from_degrees(lon);

        if lon < 0.0 {
            d.cardinal = Some(Cardinal::West);
        } else {
            d.cardinal = Some(Cardinal::East);
        }

        d
    }

    pub fn to_degrees(&self) -> f64 {
        let d = self.degrees as f64 + self.minutes as f64 / 60.0 + self.seconds / 3600.0;

        self.cardinal
            .map(|cardinal| {
                if cardinal == Cardinal::South || cardinal == Cardinal::West {
                    -d
                } else {
                    d
                }
            })
            .unwrap_or(d)
    }
}

impl std::fmt::Display for DMS {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(cardinal) = self.cardinal {
            write!(
                f,
                "{}°{}'{:.2}\"{}",
                self.degrees, self.minutes, self.seconds, cardinal
            )
        } else {
            write!(f, "{}°{}'{:.2}\"", self.degrees, self.minutes, self.seconds)
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct LatLon {
    pub lat: f64,
    pub lon: f64,
}

impl LatLon {
    /// Create a new LatLon using the latitude & longitude
    /// in degrees.
    pub fn new(lat: f64, lon: f64) -> Self {
        Self { lat, lon }
    }

    pub fn from_radians(lat: f64, lon: f64) -> Self {
        Self {
            lat: lat.to_degrees(),
            lon: lon.to_degrees(),
        }
    }

    pub fn to_radians(&self) -> (f64, f64) {
        (self.lat.to_radians(), self.lon.to_radians())
    }

    pub fn from_dms(lat: DMS, lon: DMS) -> Self {
        Self {
            lat: lat.to_degrees(),
            lon: lon.to_degrees(),
        }
    }

    pub fn to_dms(&self) -> (DMS, DMS) {
        (
            DMS::from_degrees_latitude(self.latitude()),
            DMS::from_degrees_longitude(self.longitude()),
        )
    }

    pub fn latitude(&self) -> f64 {
        self.lat
    }

    pub fn longitude(&self) -> f64 {
        self.lon
    }

    /// Return the distance in meters on the X axis (longitude)
    /// and on the Y axis (latitude).
    pub fn distance_xy(&self, other: &LatLon) -> (f64, f64) {
        // FIXME: for some reason distance & azimuth aren't corrent unless a 4 tuple
        let (distance, azimuth, _, _) =
            Geodesic::wgs84().inverse(self.lat, self.lon, other.lat, other.lon);
        let p = heading_to_point(azimuth.round() as i32);
        (p.x as f64 * distance, p.y as f64 * distance)
    }

    /// Return a new latitude/longitude offset by a distance in meters and a bearing
    /// in degrees.
    pub fn destination(&self, bearing: f64, distance: f64) -> LatLon {
        let (lat, lon) = Geodesic::wgs84().direct(self.lat, self.lon, bearing, distance);
        Self { lat, lon }
    }

    /// Return the distance in meters between this and another latitude and longitude.
    pub fn distance(&self, other: &LatLon) -> f64 {
        Geodesic::wgs84().inverse(self.lat, self.lon, other.lat, other.lon)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::math::round_decimal;

    const NM_TO_KM: f64 = 1.852;

    // Paphos Airport
    const LCPH: LatLon = LatLon {
        lat: 34.717778,
        lon: 32.485556,
    };
    // Larnaca Airport
    const LCLK: LatLon = LatLon {
        lat: 34.875,
        lon: 33.624722,
    };

    #[test]
    fn test_latlon_destination() {
        let distance = (120.0 * NM_TO_KM) * 1000.0;
        let dest = LCPH.destination(54.0, distance);
        assert_eq!(35.9, round_decimal(dest.latitude(), 1));
        assert_eq!(34.5, round_decimal(dest.longitude(), 1));
        assert_eq!(distance.round(), LCPH.distance(&dest).round());
    }

    #[test]
    fn test_latlon_distance() {
        assert_eq!(105_698., LCPH.distance(&LCLK).round());
    }

    #[test]
    fn test_latlon_distance_xy() {
        let dest = LCPH.destination(0.0, 10.0);
        assert_eq!(0.0, LCPH.distance_xy(&dest).0.round());
        assert_eq!(10.0, LCPH.distance_xy(&dest).1.round());

        let dest = LCPH.destination(45.0, 10.0);
        assert_eq!(7.0, LCPH.distance_xy(&dest).0.round());
        assert_eq!(7.0, LCPH.distance_xy(&dest).1.round());

        let dest = LCPH.destination(90.0, 10.0);
        assert_eq!(10.0, LCPH.distance_xy(&dest).0.round());
        assert_eq!(0.0, LCPH.distance_xy(&dest).1.round());

        let dest = LCPH.destination(180.0, 10.0);
        assert_eq!(0.0, LCPH.distance_xy(&dest).0.round());
        assert_eq!(-10.0, LCPH.distance_xy(&dest).1.round());

        let dest = LCPH.destination(270.0, 10.0);
        assert_eq!(-10.0, LCPH.distance_xy(&dest).0.round());
        assert_eq!(0.0, LCPH.distance_xy(&dest).1.round());
    }
}
