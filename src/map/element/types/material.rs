use derive_more::From;
use image::Rgb;

#[derive(Debug, Clone, PartialEq, Eq, Hash, From)]
pub struct Substance {
    pub typ: SubstanceType,
    pub kind: SubstanceKind,
}

impl From<Material> for Substance {
    fn from(value: Material) -> Self {
        Self {
            typ: Default::default(),
            kind: SubstanceKind::from(value),
        }
    }
}

// added: allow creating Substance from a kind only, using default type
impl From<SubstanceKind> for Substance {
    fn from(kind: SubstanceKind) -> Self {
        Self {
            typ: SubstanceType::default(),
            kind,
        }
    }
}

impl From<(SubstanceType, Material)> for Substance {
    fn from((typ, mat): (SubstanceType, Material)) -> Self {
        Self {
            typ,
            kind: SubstanceKind::from(mat),
        }
    }
}

impl From<(SubstanceType, MultiMaterial)> for Substance {
    fn from((typ, mm): (SubstanceType, MultiMaterial)) -> Self {
        Self {
            typ,
            kind: SubstanceKind::Material(mm),
        }
    }
}

impl From<(SubstanceType, Rgb<u8>)> for Substance {
    fn from((typ, color): (SubstanceType, Rgb<u8>)) -> Self {
        Self {
            typ,
            kind: SubstanceKind::Color(color),
        }
    }
}

impl From<(SubstanceType, Rgb<u8>, Material)> for Substance {
    fn from((typ, color, mat): (SubstanceType, Rgb<u8>, Material)) -> Self {
        Self {
            typ,
            kind: SubstanceKind::ColorfulMaterial(color, MultiMaterial::from(mat)),
        }
    }
}

impl From<(SubstanceType, Rgb<u8>, MultiMaterial)> for Substance {
    fn from((typ, color, mm): (SubstanceType, Rgb<u8>, MultiMaterial)) -> Self {
        Self {
            typ,
            kind: SubstanceKind::ColorfulMaterial(color, mm),
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, From)]
#[non_exhaustive]
pub enum SubstanceType {
    #[default]
    Universal,
    Surface,
    Wall,
    Roof,
    Continuation,
    Cover,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, From)]
pub enum SubstanceKind {
    Color(Rgb<u8>),
    Material(MultiMaterial),
    ColorfulMaterial(Rgb<u8>, MultiMaterial),
}

impl From<Material> for SubstanceKind {
    fn from(value: Material) -> Self {
        SubstanceKind::Material(MultiMaterial::from(value))
    }
}

// added: convert a Substance into its kind (discard type)
impl From<Substance> for SubstanceKind {
    fn from(substance: Substance) -> Self {
        substance.kind
    }
}

// added: convenience constructors for ColorfulMaterial using Material instead of MultiMaterial
impl From<(Rgb<u8>, Material)> for SubstanceKind {
    fn from((color, mat): (Rgb<u8>, Material)) -> Self {
        SubstanceKind::ColorfulMaterial(color, MultiMaterial::from(mat))
    }
}

impl From<(Material, Rgb<u8>)> for SubstanceKind {
    fn from((mat, color): (Material, Rgb<u8>)) -> Self {
        SubstanceKind::ColorfulMaterial(color, MultiMaterial::from(mat))
    }
}

// added: also accept a Vec<Material> for the second field
impl From<(Rgb<u8>, Vec<Material>)> for SubstanceKind {
    fn from((color, materials): (Rgb<u8>, Vec<Material>)) -> Self {
        SubstanceKind::ColorfulMaterial(color, MultiMaterial::Combined(materials))
    }
}

/// Represents a combination of multiple materials.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, From)]
pub enum MultiMaterial {
    /// A single material.
    Single(Material),
    /// A combination of several materials.
    Combined(Vec<Material>),
}

impl From<MultiMaterial> for Material {
    fn from(value: MultiMaterial) -> Self {
        match value {
            MultiMaterial::Combined(v) => v[0],
            MultiMaterial::Single(s) => s,
        }
    }
}

/// Material used for a surface, structure or object.
///
/// The enum includes all values found in OSM's `material` key
/// (common, rare, misspelled or in other languages).
#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Material {
    /// Italian for steel.
    Acciaio,
    /// A transparent thermoplastic often used as a lightweight or shatter-resistant substitute for glass.
    Acrylic,
    /// Spanish for cobblestone or paving stone.
    Adoquin,
    /// Sun-dried brick made of sand, clay, water and fibrous material.
    Adobe,
    /// A synthetic surface suitable for all weather conditions.
    AllWeather,
    /// The main material is aluminium (aluminum).
    Aluminium,
    /// Alternative spelling of aluminium (aluminum).
    Aluminum,
    /// A dark, fine-grained volcanic rock.
    Andesite,
    /// Anti-slip surface treatment.
    AntiSlip,
    /// Portuguese for sand.
    Areia,
    /// Spanish for sand.
    Arena,
    /// Artificial material, not specified further.
    Artificial,
    /// Artificial clay-based surface.
    ArtificialClay,
    /// Artificial stone-like material.
    ArtificialStone,
    /// Artificial grass (synthetic turf).
    ArtificialTurf,
    /// Volcanic ash, often used as a lightweight aggregate.
    Ash,
    /// Asphalt (bituminous concrete). Qualify with [`AsphaltType`].
    Asphalt(AsphaltType),
    /// Compacted asphalt.
    AsphaltCompacted,
    /// Asphalt surface divided into lanes.
    AsphaltLanes,
    /// Asphalt with visible ruts.
    AsphaltRuts,
    /// Spanish glazed ceramic tile (azulejo).
    Azulejo,
    /// Loose aggregate used as a base for railway tracks or roads.
    Ballast,
    /// The main material is bamboo.
    Bamboo,
    /// Bare ground without vegetation or surfacing.
    BareGround,
    /// Exposed bedrock.
    BareRock,
    /// Shredded tree bark, often used as mulch or soft surfacing.
    Bark,
    /// Bark‑based mulch.
    BarkMulch,
    /// The main material is basalt.
    Basalt,
    /// French for basalt.
    Basalte,
    /// Indonesian / Malay for brick.
    Bata,
    /// Indonesian / Malay for brick (alternative spelling).
    BatuBata,
    /// Solid rock underlying soil or loose material.
    Bedrock,
    /// French / German / Slavic shorthand for concrete (béton / beton).
    Beton,
    /// French for a two‑layer surface (bi‑couche).
    Bicouche,
    /// Mining spoil tip.
    Bing,
    /// Bitumen‑macadam (a type of asphalt surface).
    Bitmac,
    /// A black viscous mixture obtained from crude oil, used in road surfaces.
    Bitumen,
    /// A surface containing bitumen as a binder.
    Bituminous,
    /// Surface that has been blacktopped (sealed with asphalt).
    Blacktopped,
    /// Burnt shale used as a path or road surface.
    Blaes,
    /// Paving made from blocks, such as concrete blocks.
    BlockPaving,
    /// Generic concrete or stone blocks.
    Blocks,
    /// A dense, hard‑wearing bluish‑grey stone (commonly basalt).
    Bluestone,
    /// A walkway constructed of wooden planks.
    Boardwalk,
    /// French for wood.
    Bois,
    /// A large rock, typically used in landscaping.
    Boulder,
    /// More than one large rock (boulders).
    Boulders,
    /// Cut or fallen branches, often used for informal paths or barriers.
    Branches,
    /// The main material is brass, a copper‑zinc alloy.
    Brass,
    /// A common building material made from fired clay.
    Brick,
    /// Fired clay bricks (plural form).
    Bricks,
    /// Brick paving laid in a decorative weave pattern.
    BrickWeave,
    /// The main material is an alloy consisting primarily of copper and generally tin (or other metals).
    Bronze,
    /// Cut brushwood, often used for fences or informal surfacing.
    Brushwood,
    /// A single shrub or bush.
    Bush,
    /// Multiple shrubs or bushy vegetation.
    Bushes,
    /// A surface treatment that creates a roughened texture by hammering.
    BushHammering,
    /// Steel wire rope or electrical cable.
    Cable,
    /// French for limestone.
    Calcaire,
    /// A heavy, strong fabric used for tents, sails, etc.
    Canvas,
    /// A type of cactus used as a living fence or barrier.
    Cardon,
    /// Textile floor covering.
    Carpet,
    /// A high‑quality white or blue‑grey marble from Carrara, Italy.
    CarraraMarble,
    /// Iron cast into shape, typically used for ornamental or structural elements.
    CastIron,
    /// Cement (binder). Qualify with [`CementType`].
    Cement(CementType),
    /// Blocks made from cement (concrete blocks).
    CementBlock,
    /// Cement blocks with a plastered finish.
    CementBlockPlastered,
    /// Cement blocks that are plastered and painted.
    CementBlockPlasteredPaint,
    /// The main material is ceramic.
    Ceramic,
    /// Ceramic items or tiles (plural form).
    Ceramics,
    /// Spanish for grass / turf.
    Cesped,
    /// A series of linked metal rings (chain).
    Chain,
    /// A fencing material made of interwoven steel wires.
    ChainLink,
    /// Alternative spelling of chain‑link.
    Chainlink,
    /// A soft, white, porous limestone.
    Chalk,
    /// A road surface treatment using a layer of bitumen covered with stone chips.
    Chipseal,
    /// Partially burnt coal or other fuel, used as a surfacing material.
    Cinder,
    /// Lightweight concrete blocks made with cinders.
    Cinderblock,
    /// A natural, fine‑grained soil material, plastic when wet.
    Clay,
    /// Paving made from clinker (hard, dense bricks or tiles).
    ClinkerPlates,
    /// Woven fabric used for clothing or other flexible items.
    Cloth,
    /// Fossil fuel, solid combustible mineral.
    Coal,
    /// Natural, rounded, water‑worn stones used for paving. See [`CobblestoneType`].
    Cobblestone(CobblestoneType),
    /// Material that has been mechanically compacted. See [`CompactedType`].
    Compacted(CompactedType),
    /// A material made of two or more constituent materials.
    Composite,
    /// Composite material made to resemble wood (e.g., wood‑plastic composite).
    CompositeWood,
    /// The main material is concrete, a hard material composed of a mixture of sand, water and aggregates.
    /// Qualify with [`ConcreteType`].
    Concrete(ConcreteType),
    /// Blocks made of concrete.
    ConcreteBlock,
    /// A combination of concrete and wood.
    ConcreteWood,
    /// Common misspelling of concrete.
    Concret,
    /// Material used for buildings or structures under construction.
    Construction,
    /// The main material is copper.
    Copper,
    /// Material derived from coral, often sand or rubble.
    Coral,
    /// Sand composed primarily of coral fragments.
    CoralSand,
    /// Weathering steel (Corten) that forms a stable rust‑like appearance.
    CorTen,
    /// Short for Corten steel.
    Corten,
    /// Weathering steel, known as Corten.
    CortenSteel,
    /// Corrugated sheets made of iron.
    CorrugatedIron,
    /// Corrugated sheets made of steel.
    CorrugatedSteel,
    /// Granite from Cornwall, UK.
    CornishGranite,
    /// Fine dust produced by crushing rock, used as a path or sub‑base material.
    CrusherDust,
    /// Fine material from rock crushing, similar to crusher dust.
    CrusherFines,
    /// Granite that has been mechanically crushed.
    CrushedGranite,
    /// Limestone that has been mechanically crushed.
    CrushedLimestone,
    /// Shells that have been crushed, used as decorative surfacing or path material.
    CrushedShells,
    /// Stone that has been mechanically crushed.
    CrushedStone,
    /// Rubble or scattered waste material.
    Debris,
    /// A brand name for a synthetic sports surface.
    Decoturf,
    /// Decomposed granite, a fine, stable gravel‑like material.
    DecomposedGranite,
    /// A fine‑ to medium‑grained igneous rock.
    Diabase,
    /// Unpaved, compacted earth surface.
    Dirt,
    /// A mix of dirt and rock.
    DirtRock,
    /// A common sedimentary rock composed of calcium magnesium carbonate.
    Dolomite,
    /// Dry stone construction, built without mortar.
    DryStone,
    /// Ductile (nodular) cast iron.
    DuctileIron,
    /// Sand from dunes.
    DuneSand,
    /// A brand of reinforced polymer‑based panels.
    Durapol,
    /// Soil or ground (synonym for dirt, earth).
    Earth,
    /// Chemical element symbols used as material markers. See [`Element`].
    Element(Element),
    /// Spanish for cobblestone paving.
    Empedrado,
    /// Enamelled lava stone (used for decorative signs or plaques).
    EnameledLava,
    /// Particular kind of resin derived from polymers, used to replace wood or concrete in many moulded, laminate or coated industrial appliances.
    Epoxy,
    /// Fibre‑cement (asbestos‑free), a composite building material.
    Eternit,
    /// Concrete with the fine aggregate (sand) removed to expose the larger stones.
    ExposedAggregateConcrete,
    /// A flexible material made by weaving or knitting fibres.
    Fabric,
    /// The main material is fiberglass.
    Fiberglass,
    /// Fibre‑reinforced polymer grate (FRP grate).
    FibreReinforcedPolymerGrate,
    /// Alternative spelling of fiberglass.
    Fibreglass,
    /// Spanish for fibre‑cement.
    Fibrocemento,
    /// Fine‑grained gravel, often compacted.
    FineGravel,
    /// Fine‑grained sand.
    FineSand,
    /// A flagstone (flat slab of stone).
    Flag,
    /// Flat stone slabs used for paving (flagstone).
    Flagstone,
    /// A hard, dark grey form of quartz that breaks with a conchoidal fracture.
    Flint,
    /// An area with planted flowers.
    Flowerbed,
    /// Portuguese / Italian for cast iron.
    Fonte,
    /// A path used primarily by pedestrians (footway).
    Footway,
    /// Fibre‑Reinforced Polymer.
    Frp,
    /// Fibre‑Reinforced Polymer grate.
    FrpGrate,
    /// A cage filled with rocks, concrete or sometimes sand and soil.
    Gabion,
    /// Gaseous material, often used for pipelines carrying natural gas.
    Gas,
    /// Geotextile fabric used for soil stabilisation or separation.
    Geofabric,
    /// Permanent body of ice (glacier).
    Glacier,
    /// The main material is glass.
    Glass,
    /// Glass fibres used as reinforcement.
    GlassFibre,
    /// A high‑grade metamorphic rock with banded texture.
    Gneiss,
    /// The main material is gold.
    Gold,
    /// Harvested grain, stored or transported.
    Grain,
    /// Spanish / Portuguese for grass.
    Grama,
    /// A coarse‑grained igneous rock composed mostly of quartz and alkali feldspar.
    /// Qualify with [`GraniteType`].
    Granite(GraniteType),
    /// Granite stone (redundant form).
    GraniteStone,
    /// French / German / other languages for granite.
    Granit,
    /// Living grass vegetation.
    Grass,
    TallGrass,
    /// An area dominated by grasses (grassland).
    Grassland,
    /// Grass reinforcement grid or paver. See [`GrassPaverType`].
    GrassPaver(GrassPaverType),
    /// Grass mixed with scrub vegetation.
    GrassScrub,
    /// Loose, rounded rock fragments (gravel). See [`GravelType`].
    Gravel(GravelType),
    /// Colour or condition indicating green vegetation.
    Green,
    /// A type of grass paver that stays green.
    GreenPaver,
    /// A brand name for a synthetic tennis court surface.
    Greenset,
    /// A type of greywacke sandstone.
    Greywacke,
    /// Small particles of stone, often used as an abrasive or path surface.
    Grit,
    /// Natural ground surface. See [`GroundType`].
    Ground(GroundType),
    /// Weathered granite gravel (common in central Europe).
    Grus,
    /// A soft sulphate mineral used in plaster, drywall and fertiliser.
    Gypsum,
    /// Hard, compact surface.
    Hard,
    /// Hardcore, broken bricks, concrete or stones used as a sub‑base.
    Hardcore,
    /// A hard court surface (usually asphalt or concrete) for sports.
    HardCourt,
    /// Timber from deciduous trees.
    Hardwood,
    /// Dried grass or legume stems used as animal fodder.
    Hay,
    /// Hot mix asphalt.
    Hotmix,
    /// Frozen water.
    Ice,
    /// A road constructed on ice.
    IceRoad,
    /// A drawing or print made with ink on paper.
    InkOnPaper,
    /// Interlocking paving blocks.
    Interlock,
    /// An intermediate material or layer.
    Intermediate,
    /// The main material is iron.
    Iron,
    /// Iron ore, the raw material for iron production.
    IronOre,
    /// A type of limestone pavement (karral / karren).
    Karral,
    /// Spanish for brick.
    Ladrillo,
    /// Volcanic mudflow deposit.
    Lahar,
    /// The main material is laterite.
    Laterite,
    /// Molten rock from a volcano.
    Lava,
    /// A heavy, malleable metal (chemical symbol Pb).
    Lead,
    /// Decomposed leaves and plant litter on the ground.
    LeafLitter,
    /// Material made from animal hide.
    Leather,
    /// Fallen leaves, often covering a path.
    Leaves,
    /// Light‑weight material or light‑emitting element.
    Light,
    /// A type of porous limestone (limerock).
    Limerock,
    /// Sedimentary rock composed mainly of calcium carbonate.
    Limestone,
    /// A durable floor covering made from linseed oil, pine rosin and wood flour.
    Linoleum,
    /// A single section of a tree trunk.
    Log,
    /// Tree trunks used as construction material or path edging.
    Logs,
    /// Loose earth, uncompacted.
    LooseEarth,
    /// Loose fine gravel, uncompacted.
    LooseFineGravel,
    /// Loose gravel, uncompacted.
    LooseGravel,
    /// Loose sand, uncompacted.
    LooseSand,
    /// A type of road surface of crushed stone mixed with tar or bitumen.
    Macadam,
    /// Spanish for wood.
    Madera,
    /// The main material is marble. Qualify further if possible.
    Marble,
    /// A single marble plate.
    MarblePlate,
    /// Multiple marble plates.
    MarblePlates,
    /// Powdered marble, often used as filler or in resin‑bound surfaces.
    MarblePowder,
    /// Wetland area, soft and waterlogged.
    Marsh,
    /// Masonry, a structure built from individual units laid in mortar.
    Masonry,
    /// A floor mat.
    Mat,
    /// Multiple mats.
    Mats,
    /// Medium Density Fibreboard, an engineered wood product.
    MDF,
    /// A grassland area, often used for hay.
    Meadow,
    /// A net or mesh material.
    Mesh,
    /// The main material is some kind of metal. If possible, qualify further by type.
    Metal,
    /// Metal in a specific form. See [`MetalForm`].
    MetalForm(MetalForm),
    /// Metal grid (expanded metal or welded mesh).
    MetalGrid,
    /// German / Swedish for metal.
    Metall,
    /// Reflective glass used in mirrors.
    Mirror,
    /// A mixture of various materials.
    Mixed,
    /// A single‑coat render or plaster (Spanish: monocapa).
    Monocapa,
    /// A decorative pattern made from small pieces of stone, glass or tile.
    Mosaic,
    /// Small, non‑vascular plants that form a dense green mat.
    Moss,
    /// A mixture of water and fine soil, soft and sticky when wet.
    Mud,
    /// Shredded plant material used as a protective cover for soil.
    Mulch,
    /// Multiple or mixed materials (abbreviated).
    Multi,
    /// A local name for laterite gravel used as a road surface (East Africa).
    Murram,
    /// Shell‑bearing limestone (German: Muschelkalk).
    Muschelkalk,
    /// Natural, untreated material.
    Natural,
    /// Unworked natural stone.
    NaturalStone,
    /// Natural stones (plural form).
    NaturalStones,
    /// A mesh or web‑like material.
    Net,
    /// No material.
    No,
    /// No material (used to indicate absence).
    None,
    /// A type of rhyolite from Northbrae, California.
    NorthbraeRhyolite,
    /// Wood from oak trees.
    Oak,
    /// Crude oil or petroleum product.
    Oil,
    /// A brand name for a seal coating or surface treatment.
    Ottaseal,
    /// Ground covered by vegetation that has overgrown a previous surface.
    Overgrown,
    /// Compacted gravel surface, often used for driveways or paths.
    PackedGravel,
    /// A liquid mixture applied to a surface for protection or decoration.
    Paint,
    /// A surface that has been painted.
    Painted,
    /// Wood that has been painted.
    PaintedWood,
    /// A frond from a palm tree, used as thatch or fencing.
    PalmFrond,
    /// Leaves from palm trees, used as thatch.
    PalmLeaves,
    /// Misspelling of palm leaves.
    PalmLeavs,
    /// A pre‑fabricated panel.
    Panel,
    /// Thin sheets of paper, used for packaging or crafts.
    Paper,
    /// A generic path surface, unspecified.
    Path,
    /// A single paving unit (paver).
    Paver,
    /// Multiple paving units (pavers).
    Pavers,
    /// A surface that has been paved (generic).
    Paved,
    /// Paving made from stone slabs.
    PavedStones,
    /// Paving blocks, often concrete or stone.
    PavingBlock,
    /// A single paving slab.
    PavingSlab,
    /// Multiple paving slabs.
    PavingSlabs,
    /// Paving stones. See [`PavingStonesType`].
    PavingStones(PavingStonesType),
    /// Partly decomposed organic material found in bogs.
    Peat,
    /// A roughcast exterior render with small pebbles thrown into the wet plaster.
    Pebbledash,
    /// A stone consisting of small rounded pebbles cemented together.
    Pebblestone,
    /// French for stone.
    Pierre,
    /// French for dry stone.
    PierreSeche,
    /// Italian for stone.
    Pietra,
    /// Wooden planks.
    Planks,
    /// A plant (as a material).
    Plant,
    /// Plants used as a surface or feature.
    Plants,
    /// A mixture of lime or gypsum with sand and water, applied as a coating.
    Plaster,
    /// The main material is some kind of plastic. If possible, qualify further.
    Plastic,
    /// Flat stone or concrete plates.
    Plates,
    /// A transparent plastic (polymethyl methacrylate).
    Plexiglass,
    /// A long, slender piece of wood or metal, used as a structural support.
    Pole,
    /// A category of synthetic resins.
    Polyester,
    /// Alternative spelling of polyethylene.
    Polyethene,
    /// A common thermoplastic polymer.
    Polyethylene,
    /// A synthetic resin used in foams, elastomers and coatings.
    Polyurethane,
    /// A hard, white ceramic material.
    Porcelain,
    /// A type of volcanic tuff (porphyritic tuff).
    Porphyrtuff,
    /// A high‑quality limestone from the Isle of Portland, UK.
    PortlandStone,
    /// Prefabricated structure or element.
    Prefab,
    /// Prefabricated concrete elements.
    PrefabConcrete,
    /// Polyvinyl chloride, a common plastic.
    PVC,
    /// A hard, metamorphic rock consisting mainly of quartz.
    Quartzite,
    /// Old railway sleepers (ties), often reused as landscaping timber.
    RailwaySleepers,
    /// Rammed earth: a mixture of soil, sand and clay compressed into forms.
    RammedEarth,
    /// Material that has been recycled.
    RecycledMaterial,
    /// Plastic that has been recycled.
    RecycledPlastic,
    /// A durable, rot‑resistant timber from western red cedar.
    RedCedar,
    /// Red‑coloured earth or soil.
    RedEarth,
    /// Red‑coloured sandstone.
    RedSandstone,
    /// Timber from redwood trees (sequoia).
    Redwood,
    /// Tall, thin plants that grow in wetlands, used for thatch.
    Reed,
    /// A coat of plaster or cement applied to a wall.
    Render,
    /// A solid or semi‑solid organic material, natural or synthetic.
    Resin,
    /// Any solid mineral material. If possible, qualify further.
    Rock,
    /// Finely crushed rock, used as a base or surface.
    RockDust,
    /// Multiple rocks.
    Rocks,
    /// A surface that is rocky.
    Rocky,
    /// Roman‑style paving (large, flat stones).
    RomanPaving,
    /// Fired clay tiles used for roofing.
    RoofTiles,
    /// Tree roots exposed on the surface.
    Roots,
    /// A strong cord made from twisted fibres.
    Rope,
    /// An elastic material made from natural or synthetic latex.
    Rubber,
    /// Granules made from recycled rubber, often used in artificial turf.
    RubberCrumb,
    /// Mulch made from recycled rubber.
    RubberMulch,
    /// Tiles made from rubber.
    RubberTiles,
    /// A surface that has been rubberized.
    Rubberized,
    /// Broken fragments of stone, brick or concrete.
    Rubble,
    /// A Portuguese term for a crushed stone or gravel surface.
    Saibro,
    /// Sodium chloride, used for de‑icing or in salt‑domes.
    Salt,
    /// The main material is sand.
    Sand,
    /// A bag filled with sand, used for flood protection or temporary barriers.
    Sandbag,
    /// The main material is a sedimentary rock composed mainly of sand‑sized grains.
    Sandstone,
    /// Fine particles of wood produced by sawing.
    Sawdust,
    /// Discarded metal items.
    ScrapMetal,
    /// A term used for crushed rock or gravel (from Old Norse).
    Scree,
    /// A screen or mesh material.
    Screen,
    /// Vegetation consisting of stunted trees and shrubs.
    Scrub,
    /// Shells from marine animals, crushed or whole.
    SeaShells,
    /// A surface that has been sealed (e.g., asphalt sealcoat).
    Sealed,
    /// A rectangular quarried stone used for paving. See [`SettType`].
    Sett(SettType),
    /// A fabric used to provide shade.
    Shadecloth,
    /// A fine‑grained sedimentary rock that splits easily into layers.
    Shale,
    /// Metal in the form of thin sheets.
    SheetMetal,
    /// Hard outer covering of marine molluscs.
    Shell,
    /// Limestone composed mainly of shells.
    ShellLimestone,
    /// Multiple shells.
    Shells,
    /// Small, rounded pebbles used as a surface, especially on beaches.
    Shingle,
    /// Grass that is kept short.
    ShortGrass,
    /// A shrub or bush.
    Shrub,
    /// An area covered with shrubs.
    Shrubbery,
    /// Silicate (sand‑lime) brick.
    SilicateBrick,
    /// A narrow, single‑track path or trail.
    Singletrack,
    /// Flat pieces of stone or concrete used for paving.
    Slabs,
    /// The glassy waste product from smelting ore.
    Slag,
    /// A fine‑grained metamorphic rock that splits into thin layers.
    Slate,
    /// Frozen precipitation.
    Snow,
    /// A hard crust formed on snow.
    SnowCrust,
    /// Turf or grass sods.
    Sods,
    /// A soft, yielding surface.
    Soft,
    /// The upper layer of earth in which plants grow.
    Soil,
    /// The main material is an iron‑based alloy containing chromium.
    StainlessSteel,
    /// The main material is steel, an alloy of iron and carbon.
    Steel,
    /// A grating made of steel.
    SteelGrating,
    /// Large, flat stones set in a path or walkway.
    SteppingStones,
    /// Italian for dirt road (sterrato).
    Sterrato,
    /// The main material is some kind of stone. If possible, qualify further. See [`StoneType`].
    Stone(StoneType),
    /// Fine powder made from stone.
    StoneDust,
    /// Multiple stones.
    Stones,
    /// Fired clay or stoneware (ceramic).
    Stoneware,
    /// A surface containing many stones.
    Stony,
    /// Dried stalks of grain plants, used for thatch or animal bedding.
    Straw,
    /// A plaster or render finish, usually decorative.
    Stucco,
    /// An unspecified surface material.
    Surface,
    /// A human‑made synthetic material.
    Synthetic,
    /// Synthetic material that imitates wood.
    SyntheticWood,
    /// Textured paving designed to be detected by visually impaired pedestrians.
    TactilePaving,
    /// Mine tailings, the waste material left after mineral extraction.
    Tailings,
    /// Indonesian / Malay for clay.
    TanahLiat,
    /// A surface treated with tar.
    Tared,
    /// A surface made of tarmacadam.
    Tarmac,
    /// A road surfaced with tar.
    TarRoad,
    /// A synthetic sports surface.
    Tartan,
    /// Earth or soil (Latin / Italian).
    Terra,
    /// Fired clay, often used for pots, tiles or decorative elements.
    Terracotta,
    /// A brand of synthetic sports surface.
    Terraway,
    /// French for rammed earth or compacted soil.
    TerreBattue,
    /// A composite material made of marble chips set in cement, polished to a high finish.
    Terrazzo,
    /// A type of plastic that becomes pliable when heated.
    Thermoplastic,
    /// Spanish for earth or soil.
    Tierra,
    /// Thin slabs of ceramic or other material.
    Tiles,
    /// A single tile.
    Tile,
    /// Wood prepared for building or carpentry.
    Timber,
    /// A thin sheet of tin or tin‑plated steel.
    Tin,
    /// A rubber covering for a wheel.
    Tire,
    /// Turkish for soil.
    Toprak,
    /// A rough path or track.
    Track,
    /// Aggregate used as a bed for railway tracks.
    TrackBallast,
    /// A light‑coloured volcanic rock.
    Trachyte,
    /// A path or trail.
    Trail,
    /// Misspelling of travertine.
    Travertine,
    /// A perennial woody plant.
    Tree,
    /// More than one tree.
    Trees,
    /// The main stem of a tree.
    TreeTrunk,
    /// A type of concrete paving slab (Polish: trylinka).
    Trylinka,
    /// A soft sedimentary rock, similar to limestone, formed in fresh water, springs, rivers, especially around waterfalls.
    Tufa,
    /// Volcanic tuff, a porous rock.
    Tuff,
    /// Grass plus the soil beneath it.
    Turf,
    /// British spelling of tires (rubber tyres).
    Tyres,
    /// An uneven surface.
    Uneven,
    /// Cobblestone that has not been dressed or flattened.
    UnhewnCobblestone,
    /// A surface that has not been paved.
    Unpaved,
    /// An unspecified or generic material.
    Unspecified,
    /// The main material is vinyl.
    Vinyl,
    /// A wall made of stone.
    WallStone,
    /// Liquid H₂O.
    Water,
    /// Weathering steel (Corten) that forms a stable rust‑like appearance.
    WeatheringSteel,
    /// A flexible material woven from willow or other twigs.
    Wicker,
    /// Wood from willow trees.
    Willow,
    /// A road that is only usable in winter when frozen.
    WinterRoad,
    /// A mesh made of wire.
    WireMesh,
    /// The main material is wood.
    Wood,
    /// Small chips of wood, often used as mulch or playground surfacing.
    WoodChips,
    /// A combination of wood and metal.
    WoodMetal,
    /// A pole made of wood.
    WoodPole,
    /// A post made of wood.
    WoodPost,
    /// Iron that has been heated and hammered into shape.
    WroughtIron,
    /// Used as a generic value for material=yes.
    Yes,
    /// A type of sandstone from York, UK.
    YorkStone,
    /// A bluish‑white metallic element used in galvanising and alloys.
    Zinc,
}

/// Chemical element used as a material marker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Element {
    /// Silver (chemical symbol Ag).
    Ag,
    /// Aluminium (chemical symbol Al).
    Al,
    /// Copper (chemical symbol Cu).
    Cu,
}

/// Specific form or shape of metal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MetalForm {
    /// Metal bars.
    Bars,
    /// Expanded metal or welded grid.
    Grid,
    /// Metal in sheet form.
    Sheet,
    /// Metal wire.
    Wire,
    /// Metal palisade (vertical stakes).
    Palisade,
    /// Metal grate.
    Grate,
    /// Corrugated metal sheets.
    Corrugated,
}

/// Sub‑types of concrete.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ConcreteType {
    /// Plain concrete without reinforcement.
    Plain,
    /// Reinforced concrete, that is concrete strengthened with embedded steel bars.
    Reinforced,
    /// Concrete plates.
    Plates,
    /// Concrete surface with lanes.
    Lanes,
    /// Concrete slabs.
    Slabs,
    /// Concrete tiles.
    Tiles,
    /// Concrete blocks.
    Blocks,
    /// Concrete with exposed pebbles.
    Pebbles,
}

/// Sub‑types of cement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CementType {
    /// Cement with brick fragments.
    Brick,
    /// Plain cement without additives.
    Plain,
}

/// Sub‑types of asphalt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AsphaltType {
    /// Standard asphalt.
    Plain,
    /// Asphalt divided into lanes.
    Lanes,
    /// Asphalt with ruts.
    Ruts,
    /// Compacted asphalt.
    Compacted,
    /// Asphalt with grooved surface.
    Grooved,
}

/// Sub‑types of ground surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GroundType {
    /// Natural, untreated ground.
    Plain,
    /// Ground surface divided into lanes.
    Lanes,
}

/// Sub‑types of grass paver.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GrassPaverType {
    /// Solid grass paver.
    Solid,
    /// Grass paver arranged in lanes.
    Lanes,
}

/// Sub‑types of paving stones.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PavingStonesType {
    /// Generic paving stones.
    Plain,
    /// Paving stones arranged in lanes.
    Lanes,
    /// Paving stones of 20 cm width.
    V20,
    /// Paving stones of 50 cm width.
    V50,
}

/// Sub‑types of cobblestone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CobblestoneType {
    /// Natural, rounded cobblestone.
    Plain,
    /// Cobblestone that has been flattened.
    Flattened,
    /// Cobblestone arranged in lanes.
    Lanes,
}

/// Sub‑types of sett paving.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SettType {
    /// Standard sett paving.
    Plain,
    /// Sett paving in plate form.
    Plates,
}

/// Sub‑types of stone surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StoneType {
    /// Generic stone.
    Plain,
    /// Stone plates.
    Plates,
    /// Stone slabs.
    Slabs,
}

/// Sub‑types of gravel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GravelType {
    /// Standard loose gravel.
    Plain,
    /// Gravel surface with distinct lanes.
    Lanes,
}

/// Sub‑types of compacted surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CompactedType {
    /// Standard compacted surface.
    Plain,
    /// Compacted surface with lanes.
    Lanes,
}

/// Sub‑types of granite.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GraniteType {
    /// Standard granite.
    Plain,
    /// Granite plates.
    Plates,
}
