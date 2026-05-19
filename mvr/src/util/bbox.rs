use std::str::FromStr;

use glam::DVec2;

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct BBox {
    pub min: DVec2,
    pub max: DVec2,
}

impl BBox {
    pub const ZERO: Self = Self::new(DVec2::ZERO, DVec2::ZERO);
    pub const ONE: Self = Self::new(DVec2::ONE, DVec2::ONE);
    pub const NEG_ONE: Self = Self::new(DVec2::NEG_ONE, DVec2::NEG_ONE);
    pub const MAX: Self = Self::new(DVec2::MIN, DVec2::MAX);

    pub const fn new(min: DVec2, max: DVec2) -> Self {
        Self { min, max }
    }
}

impl FromStr for BBox {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let numbers: Vec<_> = s.split(',').flat_map(|s| s.trim().parse::<f64>()).collect();
        Ok(Self {
            min: DVec2::new(numbers[0], numbers[1]),
            max: DVec2::new(numbers[2], numbers[3]),
        })
    }
}
