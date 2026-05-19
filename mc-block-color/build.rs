use std::collections::HashMap;
use std::fs;
use std::path::Path;

use futures::{StreamExt, stream};
use image::GenericImageView;
use palette::{FromColor, LinSrgb, Oklab, Srgb};
use quote::quote;
use reqwest::ClientBuilder;
use serde::{Deserialize, Serialize};
use steel_registry::{Registry, blocks::shapes::AABB};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Meta {
    #[serde(default)]
    pub variants: HashMap<String, Variant>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Variant {
    Single(Model),
    Multiple(Vec<Model>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Model {
    pub model: String,
    #[serde(default)]
    pub x: u16,
    #[serde(default)]
    pub y: u16,
    #[serde(default)]
    pub z: u16,
    #[serde(default)]
    pub uvlock: bool,
}

fn average_color_oklab(img: &image::DynamicImage) -> Oklab {
    let mut sum_lr = 0.0f32;
    let mut sum_lg = 0.0f32;
    let mut sum_lb = 0.0f32;
    let mut count = 0u32;

    for (_, _, px) in img.pixels() {
        if px.0[3] == 0 {
            continue;
        }
        let linear = Srgb::new(
            px.0[0] as f32 / 255.0,
            px.0[1] as f32 / 255.0,
            px.0[2] as f32 / 255.0,
        )
        .into_linear();
        sum_lr += linear.red;
        sum_lg += linear.green;
        sum_lb += linear.blue;
        count += 1;
    }

    if count == 0 {
        return Oklab::new(0.0, 0.0, 0.0);
    }

    let inv_count = 1.0 / count as f32;
    Oklab::from_color(LinSrgb::new(
        sum_lr * inv_count,
        sum_lg * inv_count,
        sum_lb * inv_count,
    ))
}

#[tokio::main]
async fn main() {
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("block_colors.rs");

    let client = ClientBuilder::new().build().unwrap();
    let registry = Registry::new_vanilla();
    let blocks = stream::iter(registry
        .blocks
        .iter()
        .filter_map(|(_, block)| {
            if block.get_collision_shape(0) == [AABB::FULL_BLOCK] {
                Some(&block.key.path)
            } else {
                None
            }
        }))
        .map(|block| tokio::spawn({
            let client = client.clone();
            async move {
                (block, client
                    .get(format!("https://raw.githubusercontent.com/misode/mcmeta/refs/heads/assets-tiny/assets/minecraft/blockstates/{block}.json"))
                    .send().await.unwrap().text().await.unwrap())
            }
        }))
        .buffer_unordered(8)
        .map(|r| {
            let (block, v) = r.unwrap();
            let result = serde_json::from_str::<Meta>(&v).unwrap();
            let variants: HashMap<String, Vec<Model>> = result.variants.into_iter().map(|(k, v)| (k, match v {
                Variant::Single(v) => vec![v],
                Variant::Multiple(m) => m,
            })).collect();

            let client = client.clone();
            async move {
                let mut valid_variants  = Vec::new();
                for (props, models) in variants {
                    if models.is_empty() {
                        continue;
                    }
                    let first_model = &models[0];
                    let texture_path = first_model.model.trim_start_matches("minecraft:");
                    let url = format!(
                        "https://raw.githubusercontent.com/misode/mcmeta/refs/heads/assets-tiny/assets/minecraft/textures/{texture_path}.png"
                    );

                    let resp = client.get(&url).send().await;
                    let bytes = match resp {
                        Ok(r) if r.status().is_success() => r.bytes().await.ok(),
                        _ => None,
                    };
                    let bytes = match bytes {
                        Some(b) => b,
                        None => continue,
                    };

                    let img = image::load_from_memory(&bytes);
                    let img = match img {
                        Ok(i) => i,
                        Err(_) => continue,
                    };

                    let has_transparent = img.pixels().any(|(_, _, p)| {
                        p.0.len() >= 4 && p.0[3] < 255
                    });
                    if has_transparent {
                        continue;
                    }

                    let avg_color = average_color_oklab(&img);
                    valid_variants.push((props, avg_color));
                }
                (block.to_string(), valid_variants)
            }
        })
        .buffer_unordered(8)
        .map(|(block, variants)| {
            let mut variants = variants;
            if variants.len() > 1 {
                let first_color = variants[0].1;
                let all_same = variants.iter().all(|(_, c)| {
                    (c.l - first_color.l).abs() < 1e-6
                        && (c.a - first_color.a).abs() < 1e-6
                        && (c.b - first_color.b).abs() < 1e-6
                });
                if all_same {
                    variants = vec![(String::new(), first_color)];
                }
            }
            (block, variants)
        })
        .collect::<Vec<_>>()
        .await;

    let entries = blocks.into_iter().map(|(block_name, variants)| {
        let block_lit = block_name;
        let variant_entries = variants.into_iter().map(|(props, color)| {
            let props_lit = props;
            let Oklab { l, a, b } = color;
            quote! {
                (#props_lit, Oklab::new(#l, #a, #b))
            }
        });
        quote! {
            (#block_lit, vec![#(#variant_entries),*])
        }
    });

    let tokens = quote! {
        use palette::Oklab;
        use std::collections::HashMap;

        #[allow(clippy::all)]
        #[rustfmt::skip]
        pub fn get_block_colors() -> HashMap<&'static str, Vec<(&'static str, Oklab)>> {
            HashMap::from([
                #(#entries),*
            ])
        }
    };

    fs::write(&dest_path, tokens.to_string()).unwrap();
}
