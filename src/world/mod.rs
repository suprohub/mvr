use std::{path::Path, sync::Arc};

use anyhow::Result;
use disponent::declare;
use geo::{Coord, LineString, Polygon, TriangulateEarcut};
use glam::{DVec3, IVec2, IVec3, Vec3Swizzles};
use ndarray::Array2;
use nonany::NonMinI16;
use rand::{distr::Distribution, seq::IndexedRandom};

use crate::{
    elevation::Elevation, map::element::types::material::Substance, util::bresenham::Bresenham,
    world::minecraft::java::VanillaSolver,
};

pub mod minecraft;

declare!(
    /// Enumeration of supported world editor backends.
    pub enum Editors {
        /// Editor for Minecraft Java Edition worlds.
        #[fallback]
        MinecraftJava(minecraft::Java<VanillaSolver>),
    }

    /// Trait for editing a voxel world.
    ///
    /// The generic parameter `V` is the voxel type.
    /// Implementors provide methods to read and write individual voxels,
    /// persist the world, and convert material descriptions into voxel choices (`Choice`).
    /// Optionally, implementors can provide methods for performing batch operations and filling regions.
    ///
    /// Разделить на части:
    /// часть которая реализовать надо
    /// и часть которая реализуется сама
    ///
    pub trait EditorImpl<V: Clone + std::fmt::Debug, S: SubstanceSolver<V>> {
        fn get_global_offset(&mut self) -> IVec3;
        fn set_global_offset(&mut self, offset: IVec3);
        fn substance_solver_ref(&self) -> &S;
        fn substance_solver(&self) -> Arc<S>;

        /// Reads the voxel at the given position.
        fn get_impl(&mut self, pos: IVec3) -> V;

        /// Reads multiple voxels at once.
        ///
        /// The default implementation simply calls [`get`](Self::get) for each position.
        /// Override this if a batched read can be performed more efficiently.
        fn get_batch_impl(&mut self, positions: impl Iterator<Item = IVec3>) -> Vec<V> {
            let (min, max) = positions.size_hint();
            let mut vec = Vec::with_capacity(max.unwrap_or(min));
            for pos in positions {
                vec.push(self.get_impl(pos));
            }
            vec
        }

        /// Writes a single voxel at the given position.
        fn set_impl(&mut self, pos: IVec3, voxel: V);

        /// Writes multiple voxels.
        ///
        /// The default implementation calls [`set`](Self::set) for each pair.
        /// Override this if a batched write can be performed more efficiently.
        fn set_batch_impl(&mut self, voxels: impl Iterator<Item = (IVec3, V)>) {
            for (pos, voxel) in voxels {
                self.set_impl(pos, voxel);
            }
        }

        /// Fills a rectangular region with voxels produced by a generator function.
        ///
        /// `pos1` and `pos2` define the inclusive-exclusive bounds of the region.
        /// The closure receives the position of each voxel and should return
        /// the voxel to place there.
        fn fill_impl(&mut self, pos1: IVec3, pos2: IVec3, substance: impl Into<Substance>) {
            let solver = self.substance_solver();
            let substance = substance.into();
            self.set_batch_impl(
                (pos1.x..pos2.x)
                    .zip((pos1.y..pos2.y).zip(pos1.z..pos2.z))
                    .map(|(x, (y, z))| {
                        let pos = IVec3::new(x, y, z);
                        (pos, solver.substance_to_voxel(substance.clone()))
                    }),
            );
        }

        /// Persists the world to the given path.
        ///
        /// The editor is consumed in the process.
        fn save(self, path: &Path) -> Result<()>;
    }
);

pub trait SubstanceSolver<V: Clone + std::fmt::Debug> {
    /// Converts a [`Material`] description into a [`Choice`] of voxel values.
    ///
    /// Implementations may map materials to different block types depending
    /// on the target platform (e.g., Minecraft, Hytale, Luanti and etc).
    fn substance_to_choice(&self, material: Substance) -> Choice<V>;

    fn apply_choice(&self, choice: Choice<V>) -> V {
        match choice {
            Choice::Single(v) => v,
            Choice::Uniform(options) => options.choose(&mut rand::rng()).cloned().unwrap(),
            Choice::Weighted(weighted) => {
                let dist = rand::distr::weighted::WeightedIndex::new(
                    weighted.iter().map(|(w, _)| *w as f64),
                )
                .unwrap();
                let idx = dist.sample(&mut rand::rng());
                weighted[idx].1.clone()
            }
        }
    }

    fn substance_to_voxel(&self, substance: impl Into<Substance>) -> V {
        self.apply_choice(self.substance_to_choice(substance.into()))
    }
}

pub trait Editor<V: Clone + std::fmt::Debug, S: SubstanceSolver<V>>: EditorImpl<V, S> {
    /// Reads the voxel at the given position.
    fn get(&mut self, pos: IVec3) -> V {
        let offset = self.get_global_offset();
        self.get_impl(pos + offset)
    }

    /// Reads multiple voxels at once.
    ///
    /// The default implementation simply calls [`get`](Self::get) for each position.
    /// Override this if a batched read can be performed more efficiently.
    fn get_batch(&mut self, positions: impl Iterator<Item = IVec3>) -> Vec<V> {
        let offset = self.get_global_offset();
        self.get_batch_impl(positions.map(|p| p + offset))
    }

    /// Writes a single voxel at the given position.
    fn set(&mut self, pos: IVec3, voxel: V) {
        let offset = self.get_global_offset();
        self.set_impl(pos + offset, voxel);
    }

    /// Writes multiple voxels.
    ///
    /// The default implementation calls [`set`](Self::set) for each pair.
    /// Override this if a batched write can be performed more efficiently.
    fn set_batch(&mut self, voxels: impl Iterator<Item = (IVec3, V)>) {
        let offset = self.get_global_offset();
        self.set_batch_impl(voxels.map(|(p, v)| (p + offset, v)));
    }

    /// Fills a rectangular region with voxels produced by a generator function.
    ///
    /// `pos1` and `pos2` define the inclusive-exclusive bounds of the region.
    /// The closure receives the position of each voxel and should return
    /// the voxel to place there.
    fn fill(&mut self, pos1: IVec3, pos2: IVec3, substance: impl Into<Substance>) {
        let offset = self.get_global_offset();
        self.fill_impl(pos1 + offset, pos2 + offset, substance);
    }

    fn fill_line(
        &mut self,
        start: IVec3,
        end: IVec3,
        substance: impl Into<Substance> + Clone,
        line_width: u32,
        stroke_width: u32,
        stroke_material: Option<impl Into<Substance> + Clone>,
    ) {
        if let Some(stroke_mat) = stroke_material
            && stroke_width > 0
        {
            self.fill_thick_line(start, end, stroke_mat, line_width + stroke_width);
        }
        self.fill_thick_line(start, end, substance, line_width);
    }

    fn fill_thick_line(
        &mut self,
        start: IVec3,
        end: IVec3,
        substance: impl Into<Substance> + Clone,
        radius: u32,
    ) {
        let r = radius as i32;
        let solver = self.substance_solver();
        let substance = substance.into();
        self.set_batch(
            Bresenham::new(start, end)
                .flat_map(|pos| {
                    (-r..=r).flat_map(move |dx| {
                        (-r..=r).flat_map(move |dy| {
                            (-r..=r).map(move |dz| IVec3::new(pos.x + dx, pos.y + dy, pos.z + dz))
                        })
                    })
                })
                .map(|p| (p, solver.substance_to_voxel(substance.clone()))),
        );
    }

    fn fill_polygon(
        &mut self,
        points: Vec<IVec3>,
        fill: Option<impl Into<Substance> + Clone>,
        edge: Option<impl Into<Substance> + Clone>,
    ) {
        if fill.is_none() && edge.is_none() {
            return;
        }

        let solver = self.substance_solver();
        let (fill, edge) = (fill.map(|v| v.into()), edge.map(|v| v.into()));
        let edge = edge.unwrap_or_else(|| fill.clone().unwrap());
        let n = points.len();

        fn project_uv(dominant: u32, p: IVec3) -> IVec2 {
            match dominant {
                0 => p.yz(),
                1 => p.xz(),
                _ => p.xy(),
            }
        }

        fn project_w(dominant: u32, p: IVec3) -> i32 {
            match dominant {
                0 => p.x,
                1 => p.y,
                _ => p.z,
            }
        }

        fn build_iv3(dominant: u32, uv: IVec2, w: i32) -> IVec3 {
            match dominant {
                0 => IVec3::new(w, uv.x, uv.y),
                1 => IVec3::new(uv.x, w, uv.y),
                _ => IVec3::new(uv.x, uv.y, w),
            }
        }

        if n < 3 {
            if n >= 2 {
                self.set_batch(
                    (0..n - 1)
                        .flat_map(|i| Bresenham::new(points[i], points[i + 1]))
                        .map(|p| (p, solver.substance_to_voxel(edge.clone()))),
                );
            }
            return;
        }

        if let Some(ref fill_mat) = fill {
            let mut fill_points = points.clone();

            let p0 = fill_points[0].as_dvec3();
            let mut normal = DVec3::ZERO;
            for i in 1..(n - 1) {
                let a = fill_points[i].as_dvec3();
                let b = fill_points[i + 1].as_dvec3();
                normal += (a - p0).cross(b - p0);
            }

            let abs_normal = normal.abs();
            let dominant = if abs_normal.x >= abs_normal.y && abs_normal.x >= abs_normal.z {
                0
            } else if abs_normal.y >= abs_normal.z {
                1
            } else {
                2
            };

            let mut uv_points: Vec<IVec2> = fill_points
                .iter()
                .map(|p| project_uv(dominant, *p))
                .collect();
            let mut w_values: Vec<i32> = fill_points
                .iter()
                .map(|p| project_w(dominant, *p))
                .collect();

            let area2 = {
                let m = uv_points.len();
                let mut sum = 0i64;
                for i in 0..m {
                    let j = (i + 1) % m;
                    let (x1, y1) = (uv_points[i].x as i64, uv_points[i].y as i64);
                    let (x2, y2) = (uv_points[j].x as i64, uv_points[j].y as i64);
                    sum += x1 * y2 - x2 * y1;
                }
                sum
            };

            if area2 < 0 {
                fill_points.reverse();
                uv_points = fill_points
                    .iter()
                    .map(|p| project_uv(dominant, *p))
                    .collect();
                w_values = fill_points
                    .iter()
                    .map(|p| project_w(dominant, *p))
                    .collect();
            }

            let coords: Vec<Coord<f64>> = uv_points
                .iter()
                .map(|p| Coord {
                    x: p.x as f64,
                    y: p.y as f64,
                })
                .collect();

            self.set_batch(
                Polygon::new(LineString::new(coords), vec![])
                    .earcut_triangles()
                    .iter()
                    .flat_map(|tri| {
                        let v0 = IVec2::new(tri.v1().x.round() as i32, tri.v1().y.round() as i32);
                        let v1 = IVec2::new(tri.v2().x.round() as i32, tri.v2().y.round() as i32);
                        let v2 = IVec2::new(tri.v3().x.round() as i32, tri.v3().y.round() as i32);

                        let idx0 = uv_points.iter().position(|&p| p == v0).unwrap();
                        let idx1 = uv_points.iter().position(|&p| p == v1).unwrap();
                        let idx2 = uv_points.iter().position(|&p| p == v2).unwrap();

                        let w0 = w_values[idx0] as f64;
                        let w1 = w_values[idx1] as f64;
                        let w2 = w_values[idx2] as f64;

                        let min_u = v0.x.min(v1.x).min(v2.x);
                        let max_u = v0.x.max(v1.x).max(v2.x);
                        let min_v = v0.y.min(v1.y).min(v2.y);
                        let max_v = v0.y.max(v1.y).max(v2.y);

                        let d1 = v1 - v0;
                        let d2 = v2 - v0;
                        let denom = d1.perp_dot(d2);
                        if denom == 0 {
                            return either::Either::Left(std::iter::empty());
                        }
                        let inv_denom = 1.0 / denom as f64;

                        let iter = (min_u..=max_u).flat_map(move |u| {
                            (min_v..=max_v).filter_map(move |v| {
                                let duv = IVec2::new(u - v0.x, v - v0.y);
                                let s = duv.perp_dot(d2) as f64 * inv_denom;
                                let t = d1.perp_dot(duv) as f64 * inv_denom;
                                if s >= -1e-9 && t >= -1e-9 && (s + t) <= 1.0 + 1e-9 {
                                    let w = (w0 + (w1 - w0) * s + (w2 - w0) * t).round() as i32;
                                    Some(build_iv3(dominant, IVec2::new(u, v), w))
                                } else {
                                    None
                                }
                            })
                        });
                        either::Either::Right(iter)
                    })
                    .map(|p| (p, solver.substance_to_voxel(fill_mat.clone()))),
            );
        }

        self.set_batch(
            (0..n)
                .flat_map(|i| {
                    let a = points[i];
                    let b = points[(i + 1) % n];
                    Bresenham::new(a, b)
                })
                .map(|p| (p, solver.substance_to_voxel(edge.clone()))),
        );
    }

    fn substance_to_voxel(&self, substance: impl Into<Substance>) -> V {
        self.substance_solver_ref().substance_to_voxel(substance)
    }
}

impl<V: Clone + std::fmt::Debug, S: SubstanceSolver<V>, T: EditorImpl<V, S>> Editor<V, S> for T {}

/// Represents a deferred voxel choice that can be resolved to a concrete value.
///
/// Used during world generation when a material description must be turned into
/// a specific block. The choice may be a single fixed value, a uniform random
/// selection from a list, or a weighted random selection.
///
/// Note: procedural choices (e.g., a function of position) are planned but not yet implemented.
pub enum Choice<V> {
    /// A single, fixed voxel value.
    Single(V),

    /// Pick one of the listed voxels with equal probability.
    Uniform(Vec<V>),

    /// Pick a voxel using the specified non‑normalised weights.
    Weighted(Vec<(f32, V)>),
    // TODO:
    // Procedural(Box<dyn Fn(IVec3) -> V>),
}

pub struct World {
    editor: Editors,
    elevation: Elevation,
}

impl World {
    pub fn new() -> Self {
        Self {
            editor: Editors::MinecraftJava(minecraft::java::Java::default()),
            elevation: Default::default(),
        }
    }
}
