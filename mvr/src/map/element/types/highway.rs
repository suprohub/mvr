use image::Rgb;

use crate::map::element::types::material::Material;

/// Represents a linear highway (road, path, etc.) mapped in OpenStreetMap.
pub struct Highway {
    /// The classification of the highway.
    pub kind: HighwayKind,
    /// If present, indicates whether the highway is one-way.
    /// `true` for one-way, `false` for explicitly two-way.
    /// Absent if the default one-way rules should be inferred.
    pub oneway: Option<bool>,
    /// Total number of marked traffic lanes (integer).
    pub lanes: Option<u8>,
    /// Maximum legal speed limit (as a string, e.g. "50", "30 mph").
    pub maxspeed: Option<String>,
    /// Minimum speed required (as a string, e.g. "50").
    pub minspeed: Option<String>,
    /// Name of the highway (e.g. "Tverskaya Street").
    pub name: Option<String>,
    /// Reference number or code (e.g. "M10", "B 6").
    pub ref_number: Option<String>,
    /// Surface material.
    pub surface: Option<Material>,
    /// Width of the carriageway in metres.
    pub width: Option<f64>,
    /// Indicates whether the road has the legal status of a motorroad
    /// (typically implies restrictions similar to a motorway).
    pub motorroad: Option<bool>,
    /// Whether the highway passes over a bridge.
    pub bridge: Option<bool>,
    /// Whether the highway runs through a tunnel.
    pub tunnel: Option<bool>,
    /// Dominant colour of the road surface (if any specific colouring).
    pub color: Option<Rgb<u8>>,
    /// Paved surface material information (may duplicate `surface` but kept for
    /// compatibility with the element model).
    pub material: Option<Material>,
}

/// Type of highway, corresponding to the OSM `highway` key.
///
/// Contains the main road classifications and some link types.
/// Not exhaustive – more specific values can be added in the future.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum HighwayKind {
    /// A high-capacity controlled-access highway, designated for fast motor traffic.
    /// Typically with grade-separated junctions and no at-grade intersections.
    Motorway,
    /// The slip roads / ramps leading to and from a motorway.
    MotorwayLink,
    /// An important road that is not a motorway. Typically forms the trunk network
    /// of a country, connecting major cities and carrying long-distance traffic.
    Trunk,
    /// The slip roads / ramps leading to and from a trunk road.
    TrunkLink,
    /// A major highway linking large towns. In urban areas it functions as a major arterial road.
    Primary,
    /// Connecting slip roads/ramps of primary highways.
    PrimaryLink,
    /// A highway linking smaller towns, or acting as a secondary arterial in a large city.
    Secondary,
    /// Connecting slip roads/ramps of secondary highways.
    SecondaryLink,
    /// A road linking small settlements, or a local centre within a large town or city.
    Tertiary,
    /// Connecting slip roads/ramps of tertiary highways.
    TertiaryLink,
    /// A public access road of the lowest classification, not residential.
    /// Often linking villages and hamlets, and serving through traffic.
    Unclassified,
    /// A road located in a residential area, primarily for access to housing.
    Residential,
    /// A road with very low speed limits and pedestrian-friendly traffic rules,
    /// often where pedestrians have priority over vehicles.
    LivingStreet,
    /// A minor access road, often to a specific destination or within an area
    /// (driveway, parking aisle, alley, etc.).
    Service,
    /// A minor land-access road, like a farm track, forest track, or similar.
    /// Usually unpaved and may be suitable only for certain vehicles.
    Track,
    /// A street or area mainly or exclusively for pedestrians.
    Pedestrian,
    /// A path mainly or exclusively for pedestrians (foot traffic).
    Footway,
    /// A generic, often multi-use path. Not designated for two-track vehicles unless
    /// subtagged. Used by pedestrians, bicycles, horses, etc.
    Path,
    /// A designated cycleway, primarily for bicycle traffic.
    Cycleway,
    /// A path for horses (bridleway).
    Bridleway,
    /// A flight of steps on a footway or path.
    Steps,
    /// A highway (road, track or path) currently under construction.
    Construction,
}
