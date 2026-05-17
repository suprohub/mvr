use derive_more::{Deref, DerefMut, From};
use glam::DVec2;
use ndarray::Array2;
use nonany::NonMinI16;

pub mod provider;

#[inline]
pub fn geo_distance(a: DVec2, b: DVec2) -> DVec2 {
    DVec2::new(
        lon_distance((a.x + b.x) / 2.0, a.y, b.y),
        lat_distance(a.x, b.x),
    )
}

// Haversine but optimized for a latitude delta of 0
// returns meters
fn lon_distance(lat: f64, lon1: f64, lon2: f64) -> f64 {
    const R: f64 = 6_371_000.0;
    let d_lon: f64 = (lon2 - lon1).to_radians();
    let a: f64 =
        lat.to_radians().cos() * lat.to_radians().cos() * (d_lon / 2.0).sin() * (d_lon / 2.0).sin();
    let c: f64 = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());

    R * c
}

// Haversine but optimized for a longitude delta of 0
// returns meters
fn lat_distance(lat1: f64, lat2: f64) -> f64 {
    const R: f64 = 6_371_000.0;
    let d_lat: f64 = (lat2 - lat1).to_radians();
    let a: f64 = (d_lat / 2.0).sin() * (d_lat / 2.0).sin();
    let c: f64 = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());

    R * c
}

#[derive(Debug, Default, Clone, Deref, DerefMut, From)]
pub struct Elevation {
    #[deref]
    pub heights_orig: Array2<Option<NonMinI16>>,
    #[deref_mut]
    pub heights_mod: Array2<Option<NonMinI16>>,
}

impl From<Array2<f64>> for Elevation {
    fn from(value: Array2<f64>) -> Self {
        let data = value.mapv(|x| NonMinI16::new(x.round() as i16));
        Self {
            heights_orig: data.clone(),
            heights_mod: data,
        }
    }
}
