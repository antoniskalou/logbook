#[derive(Clone, Debug)]
pub struct Vec2 {
    pub x: f64,
    pub y: f64,
}

impl Vec2 {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    pub fn zero() -> Self {
        Self { x: 0., y: 0. }
    }
}

/// Rotate a point by an angle (in radians) around an origin (clockwise)
pub fn rotate_point(origin: Vec2, point: Vec2, angle: f64) -> Vec2 {
    let cos = angle.cos();
    let sin = angle.sin();

    Vec2::new(
        (point.x - origin.x) * cos + (point.y - origin.y) * sin + origin.x,
        (point.y - origin.y) * cos - (point.x - origin.x) * sin + origin.y,
    )
}

pub fn heading_to_point(heading: i32) -> Vec2 {
    rotate_point(
        Vec2::zero(),
        Vec2::new(0.0, 1.0), // north
        (heading as f64).to_radians(),
    )
}

#[cfg(test)]
pub fn round_decimal(val: f64, decimal_points: u32) -> f64 {
    let multiplier = 10f64.powi(decimal_points as i32);
    (val * multiplier).round() / multiplier
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_round_decimal() {
        assert_eq!(1., round_decimal(1.43, 0));
        assert_eq!(1.4, round_decimal(1.43, 1));
        assert_eq!(1.44, round_decimal(1.435, 2));
        assert_eq!(1.435, round_decimal(1.4351, 3));
    }

    #[test]
    fn test_heading_to_point() {
        assert_eq!((0.0, 1.0), (heading_to_point(0).x, heading_to_point(0).y));
        assert_eq!(
            (1.0, 0.0),
            (
                heading_to_point(90).x.trunc(),
                heading_to_point(90).y.trunc()
            )
        );
        assert_eq!(
            (0.0, -1.0),
            (
                heading_to_point(180).x.trunc(),
                heading_to_point(180).y.trunc()
            )
        );
        assert_eq!(
            (-1.0, 0.0),
            (
                heading_to_point(270).x.trunc(),
                heading_to_point(270).y.trunc()
            )
        );
    }
}
