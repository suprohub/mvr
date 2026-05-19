use anyhow::Result;
use geo::{Coord, Geometry, LineString, Point, Polygon};
use image::Rgb;
use serde::Deserialize;
use std::{collections::HashMap, str::FromStr};
use tracing::debug;

use crate::{
    CLIENT,
    map::{
        element::{
            Element, ElementKind,
            types::{
                building::*,
                material::{Material, Substance, SubstanceType},
            },
        },
        provider::ElementProvider,
    },
    util::bbox::BBox,
};

#[derive(Debug, Deserialize)]
struct OsmData {
    elements: Vec<OsmElement>,
}

#[derive(Debug, Deserialize)]
struct OsmElement {
    #[serde(rename = "type")]
    element_type: String,
    id: u64,
    lat: Option<f64>,
    lon: Option<f64>,
    nodes: Option<Vec<u64>>,
    tags: Option<HashMap<String, String>>,
    #[serde(default)]
    members: Vec<OsmMember>,
}

#[derive(Debug, Deserialize)]
struct OsmMember {
    #[serde(rename = "type")]
    member_type: String,
    #[serde(rename = "ref")]
    ref_id: u64,
    role: String,
}

pub struct Osm;

impl ElementProvider for Osm {
    async fn fetch(&mut self, bbox: BBox) -> Result<Vec<Element>> {
        let lat_to_m = 111_111.0;
        let lon_to_m = 111_111.0 * (bbox.min.x.to_radians().cos());

        let to_local = |lon: f64, lat: f64| -> Coord<i32> {
            let x = (lon - bbox.min.y) * lon_to_m;
            let y = (bbox.max.x - lat) * lat_to_m;
            Coord {
                x: x.round() as i32,
                y: y.round() as i32,
            }
        };

        let query: String = format!(
            r#"[out:json][timeout:360][bbox:{},{},{},{}];
        (
            nwr["building"];
            nwr["building:part"];
            nwr["highway"];
            nwr["landuse"]["landuse"!="salt_pond"];
            nwr["natural"]["natural"!="coastline"]["natural"!="bay"]["natural"!="strait"];
            nwr["leisure"];
            nwr["water"]["water"!="bay"]["water"!="ocean"]["water"!="sea"]["tidal"!="yes"];
            nwr["waterway"]["waterway"!="tidal_channel"];
            nwr["amenity"];
            nwr["tourism"];
            nwr["bridge"];
            nwr["railway"];
            nwr["roller_coaster"];
            nwr["barrier"];
            nwr["entrance"];
            nwr["door"];
            nwr["power"];
            nwr["historic"];
            nwr["emergency"];
            nwr["advertising"];
            nwr["man_made"];
            nwr["aeroway"];
            way["place"]["place"!~"^(ocean|sea|bay|strait|sound|fjord)$"];
            way;
        )->.relsinbbox;
        (
            way(r.relsinbbox);
        )->.waysinbbox;
        (
            node(w.waysinbbox);
            node(w.relsinbbox);
        )->.nodesinbbox;
        .relsinbbox out body;
        .waysinbbox out body;
        .nodesinbbox out skel qt;"#,
            bbox.min.x, bbox.min.y, bbox.max.x, bbox.max.y,
        );

        let result = CLIENT
            .get("https://overpass-api.de/api/interpreter")
            .query(&[("data", query)])
            .send()
            .await?;

        debug!("Response: {:?}", result.status());
        let result = result.bytes().await?;

        let data: OsmData = serde_json::from_slice(&result)?;
        let mut node_coords: HashMap<u64, (f64, f64)> = HashMap::new();
        let mut elements = Vec::new();

        for el in &data.elements {
            if el.element_type == "node"
                && let (Some(lat), Some(lon)) = (el.lat, el.lon)
            {
                node_coords.insert(el.id, (lon, lat));
            }
        }

        let mut way_nodes: HashMap<u64, Vec<u64>> = HashMap::new();
        for el in &data.elements {
            if el.element_type == "way"
                && let Some(nodes) = &el.nodes
            {
                way_nodes.insert(el.id, nodes.clone());
            }
        }

        for el in data.elements {
            let tags = el.tags.as_ref();
            if tags.is_none() || !tags.unwrap().contains_key("building") {
                continue;
            }
            let tags = tags.unwrap();

            let building_value = tags.get("building").map(|s| s.as_str()).unwrap_or("yes");
            let kind = parse_building_kind(building_value);

            let levels: u16 = tags
                .get("building:levels")
                .and_then(|v| v.parse().ok())
                .unwrap_or(1);
            let underground_levels: u8 = tags
                .get("building:underground_levels")
                .and_then(|v| v.parse().ok())
                .unwrap_or(0);

            let wall_substance = parse_substance(
                tags.get("building:material").map(|s| s.as_str()),
                tags.get("building:colour").map(|s| s.as_str()),
                SubstanceType::Wall,
            );
            let roof_substance = parse_substance(
                tags.get("roof:material").map(|s| s.as_str()),
                tags.get("roof:colour").map(|s| s.as_str()),
                SubstanceType::Roof,
            );

            let building = Building {
                kind,
                levels,
                underground_levels,
                wall_substance,
                roof_substance,
            };

            match el.element_type.as_str() {
                "node" => {
                    if let (Some(lat), Some(lon)) = (el.lat, el.lon) {
                        let local = to_local(lon, lat);
                        elements.push(Element {
                            kind: ElementKind::Building(building),
                            geometry: Geometry::Point(Point::new(local.x, local.y)),
                        });
                    }
                }
                "way" => {
                    if let Some(node_ids) = &el.nodes {
                        if node_ids.len() < 3 {
                            continue;
                        }
                        let coords: Vec<_> = node_ids
                            .iter()
                            .filter_map(|id| node_coords.get(id).map(|&(x, y)| to_local(x, y)))
                            .collect();

                        if coords.len() >= 3 && coords.first() == coords.last() {
                            let ring = LineString::from(coords);
                            let polygon = Polygon::new(ring, vec![]);
                            elements.push(Element {
                                kind: ElementKind::Building(building),
                                geometry: Geometry::Polygon(polygon),
                            });
                        }
                    }
                }
                "relation" => {
                    let mut outer_ways: Vec<Vec<u64>> = Vec::new();
                    let mut inner_ways: Vec<Vec<u64>> = Vec::new();

                    for member in &el.members {
                        if member.member_type != "way" {
                            continue;
                        }
                        let role = member.role.trim().to_lowercase();
                        let Some(node_ids) = way_nodes.get(&member.ref_id).cloned() else {
                            continue;
                        };
                        if node_ids.len() < 2 {
                            continue;
                        }

                        match role.as_str() {
                            "outer" | "outline" => outer_ways.push(node_ids),
                            "inner" => inner_ways.push(node_ids),
                            _ => {}
                        }
                    }

                    let outer_ring_nodes = merge_ways_into_ring(&outer_ways);
                    let Some(outer_ring_nodes) = outer_ring_nodes else {
                        continue;
                    };

                    let outer_coords: Vec<_> = outer_ring_nodes
                        .iter()
                        .filter_map(|id| node_coords.get(id).map(|&(x, y)| to_local(x, y)))
                        .collect();

                    if outer_coords.len() < 3 || outer_coords.first() != outer_coords.last() {
                        continue;
                    }
                    let outer_ring = LineString::from(outer_coords);

                    let mut inner_rings = Vec::new();
                    for inner_way_nodes in &inner_ways {
                        let coords: Vec<_> = inner_way_nodes
                            .iter()
                            .filter_map(|id| node_coords.get(id).map(|&(x, y)| to_local(x, y)))
                            .collect();
                        if coords.len() >= 3 && coords.first() == coords.last() {
                            inner_rings.push(LineString::from(coords));
                        }
                    }

                    let polygon = Polygon::new(outer_ring, inner_rings);
                    elements.push(Element {
                        kind: ElementKind::Building(building),
                        geometry: Geometry::Polygon(polygon),
                    });
                }
                _ => {}
            }
        }

        Ok(elements)
    }
}

fn merge_ways_into_ring(ways: &[Vec<u64>]) -> Option<Vec<u64>> {
    if ways.is_empty() {
        return None;
    }

    if ways.len() == 1 {
        let ring = &ways[0];
        if ring.first() == ring.last() {
            return Some(ring.clone());
        }
        let mut closed = ring.clone();
        closed.push(ring[0]);
        return Some(closed);
    }

    #[derive(Clone)]
    struct WaySegment {
        nodes: Vec<u64>,
        start: u64,
        end: u64,
    }

    let mut segments: Vec<WaySegment> = ways
        .iter()
        .map(|w| WaySegment {
            nodes: w.clone(),
            start: w[0],
            end: w[w.len() - 1],
        })
        .collect();

    let first = segments.remove(0);
    let mut ring = first.nodes.clone();
    let mut current_end = first.end;

    while !segments.is_empty() {
        let mut found = false;
        for i in 0..segments.len() {
            let seg = &segments[i];
            if seg.start == current_end {
                ring.extend_from_slice(&seg.nodes[1..]);
                current_end = seg.end;
                segments.remove(i);
                found = true;
                break;
            } else if seg.end == current_end {
                let reversed: Vec<u64> = seg.nodes.iter().rev().cloned().collect();
                ring.extend_from_slice(&reversed[1..]);
                current_end = seg.start;
                segments.remove(i);
                found = true;
                break;
            }
        }
        if !found {
            return None;
        }
    }

    if ring.first() == ring.last() {
        Some(ring)
    } else {
        None
    }
}

pub fn parse_color(s: &str) -> Option<Rgb<u8>> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    // Hex parsing
    if let Some(hex) = s.strip_prefix('#') {
        let hex = if hex.len() == 3 {
            // #RGB -> #RRGGBB
            format!(
                "{}{}{}{}{}{}",
                &hex[0..1],
                &hex[0..1],
                &hex[1..2],
                &hex[1..2],
                &hex[2..3],
                &hex[2..3]
            )
        } else {
            hex.to_string()
        };
        if hex.len() == 6 {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            return Some(Rgb([r, g, b]));
        }
        return None;
    }

    // Named colours (case‑insensitive)
    let lower = s.to_lowercase();
    match lower.as_str() {
        // OSM overrides for very common colours that differ from CSS
        "brown" => Some(Rgb([0x80, 0x40, 0x00])), // OSM #804000
        "orange" => Some(Rgb([0xFF, 0x80, 0x00])), // OSM #FF8000

        // Standard CSS named colours
        "aliceblue" => Some(Rgb([0xF0, 0xF8, 0xFF])),
        "antiquewhite" => Some(Rgb([0xFA, 0xEB, 0xD7])),
        "aqua" | "cyan" => Some(Rgb([0x00, 0xFF, 0xFF])),
        "aquamarine" => Some(Rgb([0x7F, 0xFF, 0xD4])),
        "azure" => Some(Rgb([0xF0, 0xFF, 0xFF])),
        "beige" => Some(Rgb([0xF5, 0xF5, 0xDC])),
        "bisque" => Some(Rgb([0xFF, 0xE4, 0xC4])),
        "black" => Some(Rgb([0x00, 0x00, 0x00])),
        "blanchedalmond" => Some(Rgb([0xFF, 0xEB, 0xCD])),
        "blue" => Some(Rgb([0x00, 0x00, 0xFF])),
        "blueviolet" => Some(Rgb([0x8A, 0x2B, 0xE2])),
        // "brown" – overridden above
        "burlywood" => Some(Rgb([0xDE, 0xB8, 0x87])),
        "cadetblue" => Some(Rgb([0x5F, 0x9E, 0xA0])),
        "chartreuse" => Some(Rgb([0x7F, 0xFF, 0x00])),
        "chocolate" => Some(Rgb([0xD2, 0x69, 0x1E])),
        "coral" => Some(Rgb([0xFF, 0x7F, 0x50])),
        "cornflowerblue" => Some(Rgb([0x64, 0x95, 0xED])),
        "cornsilk" => Some(Rgb([0xFF, 0xF8, 0xDC])),
        "crimson" => Some(Rgb([0xDC, 0x14, 0x3C])),
        "cyan" => Some(Rgb([0x00, 0xFF, 0xFF])),
        "darkblue" => Some(Rgb([0x00, 0x00, 0x8B])),
        "darkcyan" => Some(Rgb([0x00, 0x8B, 0x8B])),
        "darkgoldenrod" => Some(Rgb([0xB8, 0x86, 0x0B])),
        "darkgray" | "darkgrey" => Some(Rgb([0xA9, 0xA9, 0xA9])),
        "darkgreen" => Some(Rgb([0x00, 0x64, 0x00])),
        "darkkhaki" => Some(Rgb([0xBD, 0xB7, 0x6B])),
        "darkmagenta" => Some(Rgb([0x8B, 0x00, 0x8B])),
        "darkolivegreen" => Some(Rgb([0x55, 0x6B, 0x2F])),
        "darkorange" => Some(Rgb([0xFF, 0x8C, 0x00])),
        "darkorchid" => Some(Rgb([0x99, 0x32, 0xCC])),
        "darkred" => Some(Rgb([0x8B, 0x00, 0x00])),
        "darksalmon" => Some(Rgb([0xE9, 0x96, 0x7A])),
        "darkseagreen" => Some(Rgb([0x8F, 0xBC, 0x8F])),
        "darkslateblue" => Some(Rgb([0x48, 0x3D, 0x8B])),
        "darkslategray" | "darkslategrey" => Some(Rgb([0x2F, 0x4F, 0x4F])),
        "darkturquoise" => Some(Rgb([0x00, 0xCE, 0xD1])),
        "darkviolet" => Some(Rgb([0x94, 0x00, 0xD3])),
        "deeppink" => Some(Rgb([0xFF, 0x14, 0x93])),
        "deepskyblue" => Some(Rgb([0x00, 0xBF, 0xFF])),
        "dimgray" | "dimgrey" => Some(Rgb([0x69, 0x69, 0x69])),
        "dodgerblue" => Some(Rgb([0x1E, 0x90, 0xFF])),
        "firebrick" => Some(Rgb([0xB2, 0x22, 0x22])),
        "floralwhite" => Some(Rgb([0xFF, 0xFA, 0xF0])),
        "forestgreen" => Some(Rgb([0x22, 0x8B, 0x22])),
        "fuchsia" | "magenta" => Some(Rgb([0xFF, 0x00, 0xFF])),
        "gainsboro" => Some(Rgb([0xDC, 0xDC, 0xDC])),
        "ghostwhite" => Some(Rgb([0xF8, 0xF8, 0xFF])),
        "gold" => Some(Rgb([0xFF, 0xD7, 0x00])),
        "goldenrod" => Some(Rgb([0xDA, 0xA5, 0x20])),
        "gray" | "grey" => Some(Rgb([0x80, 0x80, 0x80])),
        "green" => Some(Rgb([0x00, 0x80, 0x00])),
        "greenyellow" => Some(Rgb([0xAD, 0xFF, 0x2F])),
        "honeydew" => Some(Rgb([0xF0, 0xFF, 0xF0])),
        "hotpink" => Some(Rgb([0xFF, 0x69, 0xB4])),
        "indianred" => Some(Rgb([0xCD, 0x5C, 0x5C])),
        "indigo" => Some(Rgb([0x4B, 0x00, 0x82])),
        "ivory" => Some(Rgb([0xFF, 0xFF, 0xF0])),
        "khaki" => Some(Rgb([0xF0, 0xE6, 0x8C])),
        "lavender" => Some(Rgb([0xE6, 0xE6, 0xFA])),
        "lavenderblush" => Some(Rgb([0xFF, 0xF0, 0xF5])),
        "lawngreen" => Some(Rgb([0x7C, 0xFC, 0x00])),
        "lemonchiffon" => Some(Rgb([0xFF, 0xFA, 0xCD])),
        "lightblue" => Some(Rgb([0xAD, 0xD8, 0xE6])),
        "lightcoral" => Some(Rgb([0xF0, 0x80, 0x80])),
        "lightcyan" => Some(Rgb([0xE0, 0xFF, 0xFF])),
        "lightgoldenrodyellow" => Some(Rgb([0xFA, 0xFA, 0xD2])),
        "lightgray" | "lightgrey" => Some(Rgb([0xD3, 0xD3, 0xD3])),
        "lightgreen" => Some(Rgb([0x90, 0xEE, 0x90])),
        "lightpink" => Some(Rgb([0xFF, 0xB6, 0xC1])),
        "lightsalmon" => Some(Rgb([0xFF, 0xA0, 0x7A])),
        "lightseagreen" => Some(Rgb([0x20, 0xB2, 0xAA])),
        "lightskyblue" => Some(Rgb([0x87, 0xCE, 0xFA])),
        "lightslategray" | "lightslategrey" => Some(Rgb([0x77, 0x88, 0x99])),
        "lightsteelblue" => Some(Rgb([0xB0, 0xC4, 0xDE])),
        "lightyellow" => Some(Rgb([0xFF, 0xFF, 0xE0])),
        "lime" => Some(Rgb([0x00, 0xFF, 0x00])),
        "limegreen" => Some(Rgb([0x32, 0xCD, 0x32])),
        "linen" => Some(Rgb([0xFA, 0xF0, 0xE6])),
        "magenta" => Some(Rgb([0xFF, 0x00, 0xFF])),
        "maroon" => Some(Rgb([0x80, 0x00, 0x00])),
        "mediumaquamarine" => Some(Rgb([0x66, 0xCD, 0xAA])),
        "mediumblue" => Some(Rgb([0x00, 0x00, 0xCD])),
        "mediumorchid" => Some(Rgb([0xBA, 0x55, 0xD3])),
        "mediumpurple" => Some(Rgb([0x93, 0x70, 0xDB])),
        "mediumseagreen" => Some(Rgb([0x3C, 0xB3, 0x71])),
        "mediumslateblue" => Some(Rgb([0x7B, 0x68, 0xEE])),
        "mediumspringgreen" => Some(Rgb([0x00, 0xFA, 0x9A])),
        "mediumturquoise" => Some(Rgb([0x48, 0xD1, 0xCC])),
        "mediumvioletred" => Some(Rgb([0xC7, 0x15, 0x85])),
        "midnightblue" => Some(Rgb([0x19, 0x19, 0x70])),
        "mintcream" => Some(Rgb([0xF5, 0xFF, 0xFA])),
        "mistyrose" => Some(Rgb([0xFF, 0xE4, 0xE1])),
        "moccasin" => Some(Rgb([0xFF, 0xE4, 0xB5])),
        "navajowhite" => Some(Rgb([0xFF, 0xDE, 0xAD])),
        "navy" => Some(Rgb([0x00, 0x00, 0x80])),
        "oldlace" => Some(Rgb([0xFD, 0xF5, 0xE6])),
        "olive" => Some(Rgb([0x80, 0x80, 0x00])),
        "olivedrab" => Some(Rgb([0x6B, 0x8E, 0x23])),
        // "orange" – overridden above
        "orangered" => Some(Rgb([0xFF, 0x45, 0x00])),
        "orchid" => Some(Rgb([0xDA, 0x70, 0xD6])),
        "palegoldenrod" => Some(Rgb([0xEE, 0xE8, 0xAA])),
        "palegreen" => Some(Rgb([0x98, 0xFB, 0x98])),
        "paleturquoise" => Some(Rgb([0xAF, 0xEE, 0xEE])),
        "palevioletred" => Some(Rgb([0xDB, 0x70, 0x93])),
        "papayawhip" => Some(Rgb([0xFF, 0xEF, 0xD5])),
        "peachpuff" => Some(Rgb([0xFF, 0xDA, 0xB9])),
        "peru" => Some(Rgb([0xCD, 0x85, 0x3F])),
        "pink" => Some(Rgb([0xFF, 0xC0, 0xCB])),
        "plum" => Some(Rgb([0xDD, 0xA0, 0xDD])),
        "powderblue" => Some(Rgb([0xB0, 0xE0, 0xE6])),
        "purple" => Some(Rgb([0x80, 0x00, 0x80])),
        "rebeccapurple" => Some(Rgb([0x66, 0x33, 0x99])),
        "red" => Some(Rgb([0xFF, 0x00, 0x00])),
        "rosybrown" => Some(Rgb([0xBC, 0x8F, 0x8F])),
        "royalblue" => Some(Rgb([0x41, 0x69, 0xE1])),
        "saddlebrown" => Some(Rgb([0x8B, 0x45, 0x13])),
        "salmon" => Some(Rgb([0xFA, 0x80, 0x72])),
        "sandybrown" => Some(Rgb([0xF4, 0xA4, 0x60])),
        "seagreen" => Some(Rgb([0x2E, 0x8B, 0x57])),
        "seashell" => Some(Rgb([0xFF, 0xF5, 0xEE])),
        "sienna" => Some(Rgb([0xA0, 0x52, 0x2D])),
        "silver" => Some(Rgb([0xC0, 0xC0, 0xC0])),
        "skyblue" => Some(Rgb([0x87, 0xCE, 0xEB])),
        "slateblue" => Some(Rgb([0x6A, 0x5A, 0xCD])),
        "slategray" | "slategrey" => Some(Rgb([0x70, 0x80, 0x90])),
        "snow" => Some(Rgb([0xFF, 0xFA, 0xFA])),
        "springgreen" => Some(Rgb([0x00, 0xFF, 0x7F])),
        "steelblue" => Some(Rgb([0x46, 0x82, 0xB4])),
        "tan" => Some(Rgb([0xD2, 0xB4, 0x8C])),
        "teal" => Some(Rgb([0x00, 0x80, 0x80])),
        "thistle" => Some(Rgb([0xD8, 0xBF, 0xD8])),
        "tomato" => Some(Rgb([0xFF, 0x63, 0x47])),
        "turquoise" => Some(Rgb([0x40, 0xE0, 0xD0])),
        "violet" => Some(Rgb([0xEE, 0x82, 0xEE])),
        "wheat" => Some(Rgb([0xF5, 0xDE, 0xB3])),
        "white" => Some(Rgb([0xFF, 0xFF, 0xFF])),
        "whitesmoke" => Some(Rgb([0xF5, 0xF5, 0xF5])),
        "yellow" => Some(Rgb([0xFF, 0xFF, 0x00])),
        "yellowgreen" => Some(Rgb([0x9A, 0xCD, 0x32])),
        _ => None, // unknown name
    }
}

fn parse_substance(
    material_str: Option<&str>,
    colour_str: Option<&str>,
    typ: SubstanceType,
) -> Option<Substance> {
    let material = material_str.and_then(|s| Material::from_str(s).ok());
    let colour = colour_str.and_then(|s| parse_color(s));

    let a = match (material, colour) {
        (Some(mat), Some(col)) => Some(Substance::from((typ, col, mat))),
        (Some(mat), None) => Some(Substance::from((typ, mat))),
        (None, Some(col)) => Some(Substance::from((typ, col))),
        (None, None) => None,
    };
    println!("{a:?} {material_str:?}");
    a
}

fn parse_building_kind(value: &str) -> BuildingKind {
    match value {
        "yes" => BuildingKind::Unspecified,
        "residential" | "house" | "detached" | "apartments" | "semidetached_house" | "terrace"
        | "bungalow" | "cabin" | "static_caravan" | "ger" | "dormitory" | "farm"
        | "allotment_house" => BuildingKind::Residential(match value {
            "residential" => ResidentialKind::Residential,
            "house" => ResidentialKind::House,
            "detached" => ResidentialKind::Detached,
            "apartments" => ResidentialKind::Apartments,
            "semidetached_house" => ResidentialKind::SemidetachedHouse,
            "terrace" => ResidentialKind::Terrace,
            "bungalow" => ResidentialKind::Bungalow,
            "cabin" => ResidentialKind::Cabin,
            "static_caravan" => ResidentialKind::StaticCaravan,
            "ger" => ResidentialKind::Ger,
            "dormitory" => ResidentialKind::Dormitory,
            "farm" => ResidentialKind::Farm,
            "allotment_house" => ResidentialKind::AllotmentHouse,
            _ => unreachable!(),
        }),
        "commercial" | "retail" | "office" | "hotel" => BuildingKind::Commercial(match value {
            "commercial" => CommercialKind::Commercial,
            "retail" => CommercialKind::Retail,
            "office" => CommercialKind::Office,
            "hotel" => CommercialKind::Hotel,
            _ => unreachable!(),
        }),
        "industrial" | "manufacture" => BuildingKind::Industrial(match value {
            "industrial" => IndustrialKind::Industrial,
            "manufacture" => IndustrialKind::Manufacture,
            _ => unreachable!(),
        }),
        "farm_auxiliary" | "barn" | "greenhouse" | "silo" => {
            BuildingKind::Agricultural(match value {
                "farm_auxiliary" => AgriculturalKind::FarmAuxiliary,
                "barn" => AgriculturalKind::Barn,
                "greenhouse" => AgriculturalKind::Greenhouse,
                "silo" => AgriculturalKind::Silo,
                _ => unreachable!(),
            })
        }
        "school" | "university" | "college" | "hospital" | "kindergarten" | "civic" | "public"
        | "train_station" => BuildingKind::Civic(match value {
            "school" => CivicKind::School,
            "university" => CivicKind::University,
            "college" => CivicKind::College,
            "hospital" => CivicKind::Hospital,
            "kindergarten" => CivicKind::Kindergarten,
            "civic" => CivicKind::Civic,
            "public" => CivicKind::Public,
            "train_station" => CivicKind::TrainStation,
            _ => unreachable!(),
        }),
        "church" | "chapel" | "mosque" => BuildingKind::Religious(match value {
            "church" => ReligiousKind::Church,
            "chapel" => ReligiousKind::Chapel,
            "mosque" => ReligiousKind::Mosque,
            _ => unreachable!(),
        }),
        "garage" | "garages" | "carport" | "hangar" | "boathouse" => {
            BuildingKind::Transportation(match value {
                "garage" => TransportationKind::Garage,
                "garages" => TransportationKind::Garages,
                "carport" => TransportationKind::Carport,
                "hangar" => TransportationKind::Hangar,
                "boathouse" => TransportationKind::Boathouse,
                _ => unreachable!(),
            })
        }
        "warehouse" | "storage_tank" | "shed" => BuildingKind::Storage(match value {
            "warehouse" => StorageKind::Warehouse,
            "storage_tank" => StorageKind::StorageTank,
            "shed" => StorageKind::Shed,
            _ => unreachable!(),
        }),
        "bunker" | "military" | "guardhouse" => BuildingKind::Military(match value {
            "bunker" => MilitaryKind::Bunker,
            "military" => MilitaryKind::Military,
            "guardhouse" => MilitaryKind::Guardhouse,
            _ => unreachable!(),
        }),
        "outbuilding" | "service" | "roof" | "ruins" | "construction" | "hut" => {
            BuildingKind::Other(match value {
                "outbuilding" => OtherKind::Outbuilding,
                "service" => OtherKind::Service,
                "roof" => OtherKind::Roof,
                "ruins" => OtherKind::Ruins,
                "construction" => OtherKind::Construction,
                "hut" => OtherKind::Hut,
                _ => unreachable!(),
            })
        }
        _ => BuildingKind::Unspecified,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_single_closed_way() {
        let ways = vec![vec![1, 2, 3, 1]];
        let result = merge_ways_into_ring(&ways);
        assert_eq!(result, Some(vec![1, 2, 3, 1]));
    }

    #[test]
    fn merge_single_open_way() {
        let ways = vec![vec![1, 2, 3, 4]];
        let result = merge_ways_into_ring(&ways);
        assert_eq!(result, Some(vec![1, 2, 3, 4, 1]));
    }

    #[test]
    fn merge_two_ways_forming_ring() {
        let ways = vec![vec![1, 2, 3], vec![3, 4, 1]];
        let result = merge_ways_into_ring(&ways);
        assert_eq!(result, Some(vec![1, 2, 3, 4, 1]));
    }

    #[test]
    fn merge_two_ways_reversed() {
        let ways = vec![vec![1, 2, 3], vec![1, 4, 3]];
        let result = merge_ways_into_ring(&ways);
        assert_eq!(result, Some(vec![1, 2, 3, 4, 1]));
    }

    #[test]
    fn merge_three_ways() {
        let ways = vec![vec![1, 2], vec![2, 3], vec![3, 1]];
        let result = merge_ways_into_ring(&ways);
        assert_eq!(result, Some(vec![1, 2, 3, 1]));
    }

    #[test]
    fn unmergeable_fragments() {
        let ways = vec![vec![1, 2], vec![3, 4]];
        let result = merge_ways_into_ring(&ways);
        assert_eq!(result, None);
    }

    #[test]
    fn empty_input() {
        let ways: Vec<Vec<u64>> = vec![];
        let result = merge_ways_into_ring(&ways);
        assert_eq!(result, None);
    }

    #[test]
    fn test_parse_building_kind_known() {
        let kind = parse_building_kind("apartments");
        assert_eq!(kind, BuildingKind::Residential(ResidentialKind::Apartments));
    }

    #[test]
    fn test_parse_building_kind_unknown() {
        let kind = parse_building_kind("strange_structure");
        assert_eq!(kind, BuildingKind::Unspecified);
    }
}
