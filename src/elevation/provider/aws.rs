use std::{fs, path::Path, sync::Arc};

use anyhow::Result;
use derive_more::Display;
use futures::{StreamExt, stream};
use fxhash::FxHashMap;
use glam::{DVec2, I64Vec2, IVec2, UVec2};
use ndarray::Array2;
use tracing::debug;

use crate::{CACHE, CLIENT, elevation::provider::ElevationProvider, util::bbox::BBox};

#[derive(Debug, Display)]
pub struct Aws;

impl ElevationProvider for Aws {
    fn resolution(&mut self) -> f64 {
        30.0
    }

    fn coverage(&mut self) -> &[BBox] {
        &[BBox::MAX]
    }

    async fn fetch_raw(&mut self, bbox: BBox, width: usize, height: usize) -> Result<Array2<f64>> {
        let lat_diff = (bbox.max.x - bbox.min.x).abs();
        let lng_diff = (bbox.max.y - bbox.min.y).abs();
        let zoom = ((-lat_diff.max(lng_diff).log2() + 20.0) as u8).clamp(10, 15);

        let n = 2.0_f64.powi(zoom as i32);
        let (pos1, pos2) = (pos_to_tile(bbox.min, n), pos_to_tile(bbox.max, n));
        let (min, max) = (pos1.min(pos2), pos1.max(pos2));
        let tiles: Vec<UVec2> = (min.x..=max.x)
            .flat_map(|x| (min.y..=max.y).map(move |y| UVec2::new(x, y)))
            .collect();

        debug!("Downloading {} elevation tiles from AWS", tiles.len());

        let cache = Arc::new(CACHE.join("elevation/aws"));
        if !cache.exists() {
            fs::create_dir_all(&*cache)?;
        }

        let tiles = stream::iter(tiles)
            .map(|tile| {
                let cache = cache.clone();
                tokio::spawn(async move {
                    let path = cache.join(format!("{zoom}_{}_{}.png", tile.x, tile.y));
                    if path.exists()
                        && let Ok(image) = image::open(&path)
                    {
                        (tile, image.to_rgb8())
                    } else {
                        let url = format!(
                            "https://s3.amazonaws.com/elevation-tiles-prod/terrarium/{zoom}/{}/{}.png",
                            tile.x, tile.y
                        );

                        let image = 'retry: loop {
                            for i in 0..3 {
                                if i == 1 {
                                    debug!("Fetching tile {tile}");
                                } else {
                                    debug!("Fetching tile {tile}, attempt {i}");
                                }

                                if let Ok(image) = download_image(&url, &path).await {
                                    break 'retry image;
                                }
                            }
                            todo!()
                        };
                        (tile, image)
                    }
                })
            })
            .buffer_unordered(8)
            .filter_map(|res| futures::future::ready(res.ok()))
            .collect::<FxHashMap<_, _>>()
            .await;

        debug!(
            "Bilinear sampling {} tiles into {width}x{height} grid",
            tiles.len()
        );

        let denom = (DVec2::new(width as f64, height as f64) - 1.0).max(DVec2::ONE);
        let range = DVec2::new(bbox.max.x - bbox.min.x, bbox.max.y - bbox.min.y);
        let n_tiles = n as i64;

        let mut grid = Array2::from_elem((height, width), f64::NAN);

        for h in 0..height {
            let v = h as f64 / denom.y;
            let merc_y = (1.0
                - (bbox.max.x - v * range.x).to_radians().tan().asinh() / std::f64::consts::PI)
                / 2.0
                * n;

            for w in 0..width {
                let u = w as f64 / denom.x;
                let merc = DVec2::new((bbox.min.y + u * range.y + 180.0) / 360.0 * n, merc_y);

                let base = merc
                    .floor()
                    .as_i64vec2()
                    .clamp(I64Vec2::ZERO, I64Vec2::splat(n_tiles - 1))
                    .as_uvec2();

                let p = (merc - base.as_dvec2()) * 256.0;
                let p0 = p.floor();
                let d = p - p0;
                let pix = p0.as_ivec2();

                let v00 = sample_tile_pixel(&tiles, base, pix);
                let v10 = sample_tile_pixel(&tiles, base, pix + IVec2::X);
                let v01 = sample_tile_pixel(&tiles, base, pix + IVec2::Y);
                let v11 = sample_tile_pixel(&tiles, base, pix + IVec2::ONE);

                if let (Some(v00), Some(v10), Some(v01), Some(v11)) = (v00, v10, v01, v11) {
                    grid[(h, w)] = v00
                        + (v10 - v00) * d.x
                        + (v01 - v00) * d.y
                        + (v11 - v10 - v01 + v00) * d.x * d.y;
                }
            }
        }

        println!("{min}");

        Ok(grid)
    }
}

async fn download_image(
    url: &str,
    path: &Path,
) -> Result<image::ImageBuffer<image::Rgb<u8>, Vec<u8>>> {
    let bytes = CLIENT.get(url).send().await?.bytes().await?;
    let image = image::load_from_memory(&bytes)?;
    fs::write(path, bytes)?;
    Ok(image.to_rgb8())
}

fn sample_tile_pixel(
    tile_map: &FxHashMap<UVec2, image::ImageBuffer<image::Rgb<u8>, Vec<u8>>>,
    base: UVec2,
    p: IVec2,
) -> Option<f64> {
    let tx = base.x.wrapping_add((p.x.div_euclid(256)) as u32);
    let ty = base.y.wrapping_add((p.y.div_euclid(256)) as u32);
    let x = p.x.rem_euclid(256) as u32;
    let y = p.y.rem_euclid(256) as u32;

    let tile = tile_map.get(&UVec2::new(tx, ty))?;
    if x >= tile.width() || y >= tile.height() {
        return None;
    }
    let pixel = tile.get_pixel(x, y);
    let raw = (pixel[0] as u32) << 16 | (pixel[1] as u32) << 8 | pixel[2] as u32;
    let height = (raw as f64) / 256.0 - 32768.0;
    Some(height)
}

fn pos_to_tile(pos: DVec2, n: f64) -> UVec2 {
    UVec2::new(
        ((pos.y + 180.0) / 360.0 * n).clamp(0.0, n - 1.0) as u32,
        ((1.0 - pos.x.to_radians().tan().asinh() / std::f64::consts::PI) / 2.0 * n)
            .clamp(0.0, n - 1.0) as u32,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::bbox::BBox;
    use fxhash::FxHashMap;
    use glam::{DVec2, IVec2, UVec2};
    use image::RgbImage;

    fn make_test_tile() -> RgbImage {
        let mut img = RgbImage::new(256, 256);
        for y in 0..256u32 {
            for x in 0..256u32 {
                let height = (x * 256 + y) as f64;
                let raw = ((height + 32768.0) * 256.0) as u32;
                let r = ((raw >> 16) & 0xFF) as u8;
                let g = ((raw >> 8) & 0xFF) as u8;
                let b = (raw & 0xFF) as u8;
                img.put_pixel(x, y, image::Rgb([r, g, b]));
            }
        }
        img
    }

    #[test]
    fn pos_to_tile_origin() {
        let n = 4.0;
        let pos = DVec2::new(85.05112877980659, -180.0);
        let tile = pos_to_tile(pos, n);
        assert_eq!(tile, UVec2::new(0, 0));
    }

    #[test]
    fn pos_to_tile_max_corner() {
        let n = 4.0;
        let pos = DVec2::new(-85.05112877980659, 179.99999);
        let tile = pos_to_tile(pos, n);
        assert_eq!(tile, UVec2::new(3, 3));
    }

    #[test]
    fn pos_to_tile_clamping() {
        let n = 8.0;
        let pos = DVec2::new(90.0, 190.0);
        let tile = pos_to_tile(pos, n);
        assert!(tile.x <= 7);
        assert!(tile.y <= 7);
    }

    #[test]
    fn sample_tile_pixel_inside_one_tile() {
        let tile = make_test_tile();
        let mut map = FxHashMap::default();
        map.insert(UVec2::new(0, 0), tile);
        let height = sample_tile_pixel(&map, UVec2::new(0, 0), IVec2::new(100, 200)).unwrap();
        let expected = 100.0 * 256.0 + 200.0;
        assert!((height - expected).abs() < 1e-6);
    }

    #[test]
    fn sample_tile_pixel_cross_tile_boundary() {
        let tile0 = make_test_tile();
        let tile1 = make_test_tile();
        let mut map = FxHashMap::default();
        map.insert(UVec2::new(0, 0), tile0);
        map.insert(UVec2::new(1, 0), tile1);
        let height = sample_tile_pixel(&map, UVec2::new(0, 0), IVec2::new(300, 50)).unwrap();
        let expected = (44 * 256 + 50) as f64;
        assert!((height - expected).abs() < 1e-6);
    }

    #[test]
    fn sample_tile_pixel_missing_tile_returns_none() {
        let map = FxHashMap::default();
        assert!(sample_tile_pixel(&map, UVec2::new(0, 0), IVec2::new(10, 10)).is_none());
    }

    #[test]
    fn resolution_is_30() {
        let mut aws = Aws;
        assert!((aws.resolution() - 30.0).abs() < 1e-9);
    }

    #[test]
    fn coverage_is_max_bbox() {
        let mut aws = Aws;
        assert_eq!(aws.coverage(), &[BBox::MAX]);
    }

    #[tokio::test]
    #[ignore]
    async fn fetch_small_area_returns_valid_grid() {
        let mut aws = Aws;
        let bbox = BBox::new(DVec2::new(45.9, 6.8), DVec2::new(45.8, 6.7));
        let (width, height) = (64, 64);
        let grid = aws
            .fetch_raw(bbox, width, height)
            .await
            .expect("fetch failed");
        assert_eq!(grid.shape(), &[height, width]);
        let has_nonzero = grid.iter().any(|&v| v.abs() > 1.0);
        assert!(has_nonzero, "Expected some elevation variation");
    }

    #[test]
    fn zoom_and_tile_list_calculation_tiny_area() {
        let bbox = BBox::new(DVec2::new(45.0, 7.0), DVec2::new(44.999, 6.999));
        let lat_diff = (bbox.max.x - bbox.min.x).abs();
        let lng_diff = (bbox.max.y - bbox.min.y).abs();
        let zoom = ((-lat_diff.max(lng_diff).log2() + 20.0) as u8).clamp(10, 15);
        assert_eq!(zoom, 15);
        let n = 2.0_f64.powi(zoom as i32);
        let (min, max) = {
            let (p1, p2) = (pos_to_tile(bbox.min, n), pos_to_tile(bbox.max, n));
            (p1.min(p2), p1.max(p2))
        };
        assert!(max.x - min.x <= 2);
        assert!(max.y - min.y <= 2);
    }
}
