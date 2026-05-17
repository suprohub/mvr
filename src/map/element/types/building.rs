use image::Rgb;

use crate::map::element::types::material::Material;

pub struct Building {
    pub kind: BuildingKind,
    pub levels: u16,
    pub underground_levels: u8,
    pub color: Option<Rgb<u8>>,
    pub material: Option<Material>,
}

/// Type of building.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BuildingKind {
    /// A general tag for a building of an unspecified type.
    Unspecified,
    /// Residential buildings.
    Residential(ResidentialKind),
    /// Commercial buildings.
    Commercial(CommercialKind),
    /// Industrial buildings.
    Industrial(IndustrialKind),
    /// Agricultural buildings.
    Agricultural(AgriculturalKind),
    /// Civic and public buildings.
    Civic(CivicKind),
    /// Religious buildings.
    Religious(ReligiousKind),
    /// Transportation-related buildings.
    Transportation(TransportationKind),
    /// Storage buildings.
    Storage(StorageKind),
    /// Military buildings.
    Military(MilitaryKind),
    /// Other buildings not fitting the above categories.
    Other(OtherKind),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ResidentialKind {
    /// A general tag for a building constructed primarily for residential purposes.
    Residential,
    /// A single dwelling unit usually inhabited by one family.
    House,
    /// A detached house: a free-standing residential building usually housing a single family.
    Detached,
    /// A building arranged into individual dwellings, often on separate floors.
    /// May also have retail outlets on the ground floor.
    Apartments,
    /// A house that shares a common wall with another on one side.
    SemidetachedHouse,
    /// The outline of a linear row of residential dwellings, each of which normally has its own entrance,
    /// which form a terrace ("row-house" or "townhouse" in North American English).
    Terrace,
    /// Simple single-storey flat house.
    Bungalow,
    /// A small, roughly built house typically found in rural areas.
    Cabin,
    /// A mobile home (caravan) (semi)permanently left on a single site.
    StaticCaravan,
    /// A portable, round tent (yurt or ger).
    Ger,
    /// Sleeping and living quarters provided by an institution.
    Dormitory,
    /// A farmhouse is the main building of a farm.
    Farm,
    /// A small outbuilding for short visits in a allotment garden.
    AllotmentHouse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CommercialKind {
    /// A building constructed for non-specific commercial activities to take place there.
    Commercial,
    /// A building that was originally built as a retail building for selling goods to the public.
    Retail,
    /// A building that was originally built as an office building which
    /// contains spaces mainly designed to be used for offices.
    Office,
    /// A building designed with separate rooms available for overnight accommodation.
    Hotel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum IndustrialKind {
    /// A building constructed to house some manufacturing process.
    Industrial,
    /// A building constructed to house some manufacturing process, see also tag:building=industrial.
    Manufacture,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AgriculturalKind {
    /// A building on a farm that is not a dwelling.
    FarmAuxiliary,
    /// A building that was originally built as a barn - an agricultural building
    /// used for storageand as a covered workplace.
    Barn,
    /// A building in which plants are grown.
    Greenhouse,
    /// A silo is a building for storing bulk materials.
    Silo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CivicKind {
    /// A building originally built as a school building.
    School,
    /// A university building.
    University,
    /// A building constructed to house a college.
    College,
    /// A building which forms part of a hospital.
    Hospital,
    /// A generic kindergarten building.
    Kindergarten,
    /// A building originally built as civic building to hosting any civic amenity
    /// (town hall, library, swimming pool etc.).
    Civic,
    /// A building constructed as accessible to the public (town hall, police station, court house).
    Public,
    /// A train station building.
    TrainStation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ReligiousKind {
    /// A building that was built as a church.
    Church,
    /// Building built as chapel.
    Chapel,
    /// A building that was built as a mosque.
    Mosque,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TransportationKind {
    /// Denotes a single-owner private garage.
    Garage,
    /// A block of private garages each with a separate owner.
    Garages,
    /// A covered structure originally built as a carport used to offer
    /// limited protection to vehicles, primarily cars, from the elements.
    Carport,
    /// A building that was built as a hangar for the storage of aeroplanes, helicopters or space-craft.
    Hangar,
    /// A building for storing boats.
    Boathouse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StorageKind {
    /// A building that was originally built as a commercial building for storage of goods.
    Warehouse,
    /// A storage tank.
    StorageTank,
    /// A small, simple structure used as storage or workshop.
    Shed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MilitaryKind {
    /// A hardened military building.
    Bunker,
    /// An unspecific building constructed for military purposes.
    Military,
    /// A small building constructed to house guard(s).
    Guardhouse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum OtherKind {
    /// A less important building near to and on the same piece of land as a larger building.
    Outbuilding,
    /// Service building usually is a small unmanned building with certain machinery (like pumps or transformers).
    Service,
    /// A building open on at least two sides.
    Roof,
    /// Frequently used to mark ruined buildings. However, this usage conflicts with general use of building tag.
    /// In theory it should only be used for buildings constructed to look like ruins.
    Ruins,
    /// A building under construction.
    Construction,
    /// A small and crude shelter.
    Hut,
}
