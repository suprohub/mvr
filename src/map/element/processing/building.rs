use geo::Geometry;
use glam::{IVec2, IVec3};
use tracing::debug;

use crate::{
    elevation::Elevation,
    map::element::{
        ProcessElement,
        types::{
            building::Building,
            material::{Material, Substance},
        },
    },
    world::{Editor, EditorImpl, SubstanceSolver},
};

const FLOOR_HEIGHT: i32 = 3 + 1;

impl<V: Clone + std::fmt::Debug, S: SubstanceSolver<V>> ProcessElement<V, S> for Building {
    fn process_element_impl(
        self,
        editor: &mut impl EditorImpl<V, S>,
        elevation: &mut Elevation,
        geometry: Geometry<i32>,
    ) {
        match geometry {
            Geometry::Polygon(polygon) => {
                debug!("Polygon: {polygon:?}");
                let levels = self.levels as i32;
                let underground = self.underground_levels as i32;
                let exterior = polygon.exterior();

                let pts2d: Vec<IVec2> = exterior
                    .points()
                    .map(|c| IVec2::new(c.x(), c.y()))
                    .collect();

                let area_points = rasterize_polygon(&pts2d);

                let mut sum: i64 = 0;
                let mut count: i64 = 0;
                for p in &area_points {
                    if let Some(Some(h)) = elevation.get((p.y as usize, p.x as usize)) {
                        let height: i32 = h.get() as i32;
                        sum += height as i64;
                        count += 1;
                    }
                }
                let base_height: i32 = if count > 0 {
                    (sum / count) as i32
                } else {
                    todo!()
                };

                for p in &area_points {
                    if let Some(cell) = elevation.get_mut((p.y as usize, p.x as usize))
                        && let Some(val) = cell
                        && val.get() >= base_height as i16
                    {
                        *cell = None;
                    }
                }

                let wall_mat = Material::Stones;
                let slab_mat = Material::Stones;

                for i in 1..=underground {
                    let z_bottom = base_height - i * FLOOR_HEIGHT;
                    let pts3d: Vec<IVec3> = pts2d
                        .iter()
                        .map(|p| IVec3::new(p.x, p.y, z_bottom))
                        .collect();
                    editor.fill_polygon(pts3d, Some(slab_mat), None::<Substance>);

                    let z_start = z_bottom + 1;
                    let z_end = if i == 1 {
                        base_height - 1
                    } else {
                        base_height - (i - 1) * FLOOR_HEIGHT - 1
                    };
                    for z in z_start..=z_end {
                        let pts3d: Vec<IVec3> =
                            pts2d.iter().map(|p| IVec3::new(p.x, p.y, z)).collect();
                        editor.fill_polygon(pts3d, None::<Substance>, Some(wall_mat));
                    }
                }

                for i in 0..=levels {
                    let z = base_height + i * FLOOR_HEIGHT;
                    let pts3d: Vec<IVec3> = pts2d.iter().map(|p| IVec3::new(p.x, p.y, z)).collect();
                    editor.fill_polygon(pts3d, Some(slab_mat), None::<Substance>);
                }

                for i in 0..levels {
                    let z_start = base_height + i * FLOOR_HEIGHT + 1;
                    let z_end = base_height + (i + 1) * FLOOR_HEIGHT - 1;
                    for z in z_start..=z_end {
                        let pts3d: Vec<IVec3> =
                            pts2d.iter().map(|p| IVec3::new(p.x, p.y, z)).collect();
                        editor.fill_polygon(pts3d, None::<Substance>, Some(wall_mat));
                    }
                }

                let window_width: i32 = 2;
                let window_height: i32 = 2;
                let window_spacing: i32 = 2;
                let margin_from_corner: usize = 1;
                let sill_height: i32 = 1;
                let glass = Material::Glass;

                let edges: Vec<(&IVec2, &IVec2)> = pts2d
                    .iter()
                    .zip(pts2d.iter().cycle().skip(1))
                    .take(pts2d.len())
                    .collect();

                for i in 0..levels {
                    let z_wall_start = base_height + i * FLOOR_HEIGHT + 1;
                    let z_wall_end = base_height + (i + 1) * FLOOR_HEIGHT - 1;
                    let sill_z = z_wall_start + sill_height;
                    let top_z = sill_z + window_height - 1;
                    if top_z > z_wall_end {
                        continue;
                    }

                    for (p0, p1) in &edges {
                        let edge_points = points_on_line(**p0, **p1);
                        if edge_points.len() < 2 {
                            continue;
                        }
                        let start_index = margin_from_corner.min(edge_points.len());
                        let end_index = edge_points.len().saturating_sub(margin_from_corner + 1);
                        if start_index >= end_index {
                            continue;
                        }

                        let step = (window_width + window_spacing) as usize;
                        let mut pos = start_index;
                        while pos + window_width as usize <= end_index + 1 {
                            let win_start = pos;
                            let win_end = pos + window_width as usize - 1;
                            if win_end >= edge_points.len() {
                                break;
                            }
                            for idx in win_start..=win_end {
                                let pt = edge_points[idx];
                                for z in sill_z..=top_z {
                                    editor.set(
                                        IVec3::new(pt.x, pt.y, z),
                                        editor.substance_to_voxel(glass),
                                    );
                                }
                            }
                            pos += step;
                        }
                    }
                }
            }
            Geometry::Point(_) => {}
            _ => {}
        }
    }
}

fn point_in_polygon(p: IVec2, vertices: &[IVec2]) -> bool {
    let x = p.x;
    let y = p.y;
    let n = vertices.len();
    let mut inside = false;
    let mut j = n - 1;
    for i in 0..n {
        let xi = vertices[i].x;
        let yi = vertices[i].y;
        let xj = vertices[j].x;
        let yj = vertices[j].y;

        if (yi > y) != (yj > y) {
            let intersect = if yi < yj {
                (x - xi) * (yj - yi) < (xj - xi) * (y - yi)
            } else {
                (x - xi) * (yj - yi) > (xj - xi) * (y - yi)
            };
            if intersect {
                inside = !inside;
            }
        }
        j = i;
    }
    inside
}

fn rasterize_polygon(vertices: &[IVec2]) -> Vec<IVec2> {
    if vertices.len() < 3 {
        return vec![];
    }

    let min_x = vertices.iter().map(|p| p.x).min().unwrap();
    let max_x = vertices.iter().map(|p| p.x).max().unwrap();
    let min_y = vertices.iter().map(|p| p.y).min().unwrap();
    let max_y = vertices.iter().map(|p| p.y).max().unwrap();

    let mut result = Vec::new();

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let p = IVec2::new(x, y);
            if point_in_polygon(p, vertices) {
                result.push(p);
            }
        }
    }

    for i in 0..vertices.len() {
        let p0 = vertices[i];
        let p1 = vertices[(i + 1) % vertices.len()];
        result.extend(points_on_line(p0, p1));
    }

    result.sort_by_key(|p| (p.y, p.x));
    result.dedup();
    result
}

fn points_on_line(p0: IVec2, p1: IVec2) -> Vec<IVec2> {
    let mut points = Vec::new();
    let mut x = p0.x;
    let mut y = p0.y;
    let dx = (p1.x - p0.x).abs();
    let dy = -(p1.y - p0.y).abs();
    let sx = if p0.x < p1.x { 1 } else { -1 };
    let sy = if p0.y < p1.y { 1 } else { -1 };
    let mut err = dx + dy;

    loop {
        points.push(IVec2::new(x, y));
        if x == p1.x && y == p1.y {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
    points
}
