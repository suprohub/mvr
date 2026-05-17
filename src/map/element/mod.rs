use disponent::declare;
use geo::Geometry;
use ndarray::Array2;
use nonany::NonMinI16;

use crate::{
    elevation::Elevation,
    map::element::types::building::Building,
    world::{EditorImpl, SubstanceSolver},
};

pub mod processing;
pub mod types;

declare!(
    pub enum ElementKind {
        Building(Building),
    }

    pub trait ProcessElement<V: Clone + std::fmt::Debug, S: SubstanceSolver<V>>
    where
        Self: Sized,
    {
        #[doc(hidden)]
        fn process_element_impl(
            self,
            _editor: &mut impl EditorImpl<V, S>,
            _elevation: &mut Elevation,
            _geometry: Geometry<i32>,
        ) {
            unimplemented!()
        }

        fn process_element(self, _editor: &mut impl EditorImpl<V, S>, _elevation: &mut Elevation) {
            unimplemented!()
        }
    }
);

pub struct Element {
    pub kind: ElementKind,
    pub geometry: Geometry<i32>,
}

impl<V: Clone + std::fmt::Debug, S: SubstanceSolver<V>> ProcessElement<V, S> for Element {
    fn process_element(self, editor: &mut impl EditorImpl<V, S>, elevation: &mut Elevation) {
        self.kind
            .process_element_impl(editor, elevation, self.geometry);
    }
}
