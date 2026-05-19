use std::{fs, path::PathBuf, str::FromStr, sync::LazyLock};

use anyhow::Result;
use clap::Parser;
use glam::IVec3;
use reqwest::{Client, ClientBuilder};
use tracing::{Level, info};
use tracing_subscriber::FmtSubscriber;

use crate::{
    elevation::{
        Elevation,
        provider::{ElevationProvider, ElevationProviders, aws::Aws},
    },
    map::{
        element::{
            ProcessElement,
            types::material::{Material, SubstanceType},
        },
        provider::{ElementProvider, ElementProviders, osm::Osm},
    },
    util::bbox::BBox,
    world::{Editor, EditorImpl, Editors, minecraft::Java},
};

mod elevation;
mod map;
mod util;
mod world;

pub static CLIENT: LazyLock<Client> = LazyLock::new(|| {
    ClientBuilder::new()
        .user_agent(format!(
            "OMR/{} (+https://github.com/suprohub/omr)",
            env!("CARGO_PKG_VERSION")
        ))
        .build()
        .unwrap()
});

pub static CACHE: LazyLock<PathBuf> = LazyLock::new(|| {
    let dir = dirs::cache_dir().unwrap().join("OMR");
    if !dir.exists() {
        fs::create_dir(&dir).unwrap();
    }
    dir
});

#[derive(clap::Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(long, short)]
    bbox: String,

    #[arg(long, short)]
    out: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::DEBUG)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    let args = Args::parse();
    info!("OMR Started");

    let mut provider = ElevationProviders::Aws(Aws);
    let bbox = BBox::from_str(&args.bbox)?;
    let mut elevation: Elevation = provider.fetch(bbox, 1.0).await?;

    let mut editor = Editors::MinecraftJava(Java::new());

    let min_height = elevation
        .heights_mod
        .iter()
        .filter_map(|v| v.map(|v| v.get()))
        .min()
        .unwrap() as f64;
    let max_height = elevation
        .heights_mod
        .iter()
        .filter_map(|v| v.map(|v| v.get()))
        .max()
        .unwrap() as f64;
    println!("{min_height} {max_height}");

    editor.set_global_offset(IVec3::new(0, 0, -(min_height + 64.0) as i32));

    let mut elements = ElementProviders::Osm(Osm);
    for element in elements.fetch(bbox).await? {
        element.process_element(&mut editor, &mut elevation);
    }

    let (height, width) = elevation.dim();

    let grass_block = editor.substance_to_voxel(Material::Grass);
    let grass = editor.substance_to_voxel((SubstanceType::Cover, Material::Grass));
    let tall_grass_lower = editor.substance_to_voxel((SubstanceType::Cover, Material::TallGrass));
    let tall_grass_upper =
        editor.substance_to_voxel((SubstanceType::Continuation, Material::TallGrass));

    let mut infos: Vec<(IVec3, bool)> = Vec::with_capacity(width * height);
    let mut air_check_positions_y1 = Vec::with_capacity(width * height);

    for z in 0..height {
        for x in 0..width {
            if let Some(h) = elevation.heights_mod[(z, x)] {
                let y = h.get() as i32;
                let base_pos = IVec3::new(x as i32, z as i32, y);

                let is_lower = [
                    (x.wrapping_sub(1), z),
                    (x + 1, z),
                    (x, z.wrapping_sub(1)),
                    (x, z + 1),
                ]
                .iter()
                .any(|&(nx, nz)| {
                    if nx < width
                        && nz < height
                        && let Some(nh) = elevation.heights_mod[(nz, nx)]
                    {
                        return nh.get() as i32 > y;
                    }
                    false
                });

                infos.push((base_pos, is_lower));
                air_check_positions_y1.push(IVec3::new(x as i32, z as i32, y + 1));
            }
        }
    }

    let grass_blocks_batch = infos.iter().map(|(pos, _)| (*pos, grass_block.clone()));
    editor.set_batch(grass_blocks_batch);

    let air_y1 = editor.get_batch(air_check_positions_y1.into_iter());

    let mut air_check_positions_y2 = Vec::new();
    for (pos, is_lower) in &infos {
        if *is_lower {
            air_check_positions_y2.push(IVec3::new(pos.x, pos.y, pos.z + 2));
        }
    }
    let air_y2 = editor.get_batch(air_check_positions_y2.into_iter());

    let mut y2_iter = air_y2.into_iter();
    let mut vegetation_batch = Vec::new();

    for (i, (base_pos, is_lower)) in infos.iter().enumerate() {
        let above1 = &air_y1[i];
        let above1_is_air = above1.name == "minecraft:air";

        if *is_lower
            && let Some(above2) = y2_iter.next()
            && above1_is_air
            && above2.name == "minecraft:air"
        {
            let lower_pos = IVec3::new(base_pos.x, base_pos.y, base_pos.z + 1);
            let upper_pos = IVec3::new(base_pos.x, base_pos.y, base_pos.z + 2);
            vegetation_batch.push((lower_pos, tall_grass_lower.clone()));
            vegetation_batch.push((upper_pos, tall_grass_upper.clone()));
            continue;
        }

        if above1_is_air {
            let grass_pos = IVec3::new(base_pos.x, base_pos.y, base_pos.z + 1);
            vegetation_batch.push((grass_pos, grass.clone()));
        }
    }

    editor.set_batch(vegetation_batch.into_iter());

    editor.save(&PathBuf::from(&args.out))?;

    Ok(())
}
