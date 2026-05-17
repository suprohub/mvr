use std::collections::HashMap;

use anyhow::Result;
use geo::{Coord, Geometry, LineString, Point, Polygon};
use serde::Deserialize;

use crate::{
    CLIENT,
    map::{
        element::{Element, ElementKind, types::building::*},
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
            .await?
            .bytes()
            .await?;

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

            let building = Building {
                kind,
                levels,
                underground_levels,
                color: None,
                material: None,
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
