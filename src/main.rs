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

    // 1. Obtain raw elevation data
    let mut provider = ElevationProviders::Aws(Aws);
    let bbox = BBox::from_str(&args.bbox)?;
    let mut elevation: Elevation = provider.fetch(bbox, 1.0).await?;

    // 2. Process map elements (roads, rivers, etc.) – they may modify the elevation
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

    // 3. Set a global vertical offset so that the lowest terrain sits at GROUND_LEVEL
    let (height, width) = elevation.dim();

    // 4. Generate voxels using raw elevation (global offset will be applied internally)
    let grass_block = editor.substance_to_voxel(Material::Grass);
    let grass = editor.substance_to_voxel((SubstanceType::Surface, Material::Grass));

    let mut infos = Vec::with_capacity(width * height);
    let mut air_check_positions = Vec::with_capacity(width * height);

    for z in 0..height {
        for x in 0..width {
            if let Some(h) = elevation.heights_mod[(z, x)] {
                let y = h.get() as i32;
                let base_pos = IVec3::new(x as i32, z as i32, y);
                infos.push(base_pos);
                air_check_positions.push(IVec3::new(x as i32, z as i32, y + 1));
            }
        }
    }

    let grass_blocks_batch = infos.iter().map(|pos| (*pos, grass_block.clone()));
    editor.set_batch(grass_blocks_batch);

    let air_voxels = editor.get_batch(air_check_positions.into_iter());

    let grass_batch = infos
        .into_iter()
        .zip(air_voxels.into_iter())
        .filter_map(|(base_pos, above)| {
            if above.name == "minecraft:air" {
                let grass_pos = IVec3::new(base_pos.x, base_pos.y, base_pos.z + 1);
                Some((grass_pos, grass.clone()))
            } else {
                None
            }
        });
    editor.set_batch(grass_batch);

    editor.save(&PathBuf::from(&args.out))?;

    Ok(())
}
