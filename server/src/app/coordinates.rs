const EARTH_RADIUS: f32 = 6371000_f32;

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct Coordinates {
    pub latitude: f32,
    pub longitude: f32,
}

impl Coordinates {
    // http://www.movable-type.co.uk/scripts/latlong.html
    // https://rust-lang-nursery.github.io/rust-cookbook/science/mathematics/trigonometry.html#distance-between-two-points-on-the-earth
    pub(crate) fn distance(&self, other: &Self) -> f32 {
        let paris_latitude = self.latitude.to_radians();
        let london_latitude = other.latitude.to_radians();

        let delta_latitude = (self.latitude - other.latitude).to_radians();
        let delta_longitude = (self.longitude - other.longitude).to_radians();

        let central_angle_inner = (delta_latitude / 2.0).sin().powi(2)
            + paris_latitude.cos() * london_latitude.cos() * (delta_longitude / 2.0).sin().powi(2);
        let central_angle = 2.0 * central_angle_inner.sqrt().asin();

        central_angle * EARTH_RADIUS
    }
}

#[cfg(test)]
mod tests {
    use crate::app::coordinates::Coordinates;

    #[test]
    fn it_computes_distance_between_two_points() {
        let paris = Coordinates {
            longitude: -2.34880_f32,
            latitude: 48.85341_f32,
        };
        let london = Coordinates {
            longitude: -0.12574_f32,
            latitude: 51.50853_f32,
        };
        assert_eq!(paris.distance(&london), 334955.9_f32);
        assert_eq!(london.distance(&paris), 334955.9_f32);
    }
}
