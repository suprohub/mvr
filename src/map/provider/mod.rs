use anyhow::Result;
use disponent::declare;

use crate::{
    map::{element::Element, provider::osm::Osm},
    util::bbox::BBox,
};

pub mod osm;
pub mod overture;

declare!(
    pub enum ElementProviders {
        Osm(Osm),
    }

    pub trait ElementProvider {
        async fn fetch(&mut self, bbox: BBox) -> Result<Vec<Element>>;
    }
);
