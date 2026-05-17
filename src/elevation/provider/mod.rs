use std::fmt::{Debug, Display};

use anyhow::Result;
use derive_more::Display;
use disponent::declare;
use glam::USizeVec2;
use ndarray::Array2;
use nonany::NonMinI16;

use crate::{
    elevation::{Elevation, geo_distance},
    util::bbox::BBox,
};

pub mod aws;

declare!(
    #[derive(Debug, Display)]
    pub enum ElevationProviders {
        Aws(aws::Aws),
    }

    pub trait ElevationProvider: Send + Sync + Debug + Display {
        /// Approximate native resolution in meters per pixel (lower = better resolution);
        fn resolution(&mut self) -> f64;
        fn coverage(&mut self) -> &[BBox];
        async fn fetch_raw(
            &mut self,
            bbox: BBox,
            width: usize,
            height: usize,
        ) -> Result<Array2<f64>>;
        async fn fetch(&mut self, bbox: BBox, scale: f64) -> Result<Elevation> {
            let base_scale = geo_distance(bbox.min, bbox.max);
            // Apply same floor() and scale operations as CoordTransformer.llbbox_to_xzbbox()
            let scale_factor = base_scale.floor() * scale;
            // World block positions span 0..=scale_factor (inclusive), so there are
            // scale_factor+1 distinct positions.
            let world_wh = scale_factor.as_usizevec2() + 1;
            // Cap grid dimensions to avoid WMS server rejections.
            let grid_wh = world_wh.clamp(USizeVec2::splat(2), USizeVec2::splat(16384));
            Ok(self.fetch_raw(bbox, grid_wh.x, grid_wh.y).await?.into())
        }
    }
);
