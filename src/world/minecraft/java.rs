use std::{
    cmp::Ordering,
    fs::{self, File},
    path::Path,
    sync::Arc,
};

use anyhow::Result;
use derive_more::{Deref, DerefMut};
use fxhash::{FxBuildHasher, FxHashMap};
use glam::{IVec2, IVec3};
use silverfish::{Block, Coords, Region};

use crate::world::{Choice, EditorImpl};
use crate::{map::element::types::material::*, world::SubstanceSolver};

#[derive(Debug, Default)]
pub struct Java<S: SubstanceSolver<Block>> {
    offset: IVec3,
    regions: FxHashMap<IVec2, Region>,
    solver: Arc<S>,
}

impl<S: SubstanceSolver<Block> + Default> Java<S> {
    pub fn new() -> Self {
        Self::default()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deref, DerefMut)]
struct OrderedIVec2(IVec2);

impl PartialOrd for OrderedIVec2 {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for OrderedIVec2 {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0
            .x
            .cmp(&other.0.x)
            .then_with(|| self.0.y.cmp(&other.0.y))
    }
}

impl<S: SubstanceSolver<Block>> EditorImpl<Block, S> for Java<S> {
    fn get_global_offset(&mut self) -> IVec3 {
        self.offset
    }

    fn set_global_offset(&mut self, offset: IVec3) {
        self.offset = offset;
    }

    fn substance_solver(&self) -> Arc<S> {
        self.solver.clone()
    }

    fn substance_solver_ref(&self) -> &S {
        &self.solver
    }

    fn get_impl(&mut self, pos: IVec3) -> Block {
        if let Some(region) = self.regions.get(&java_region_pos(pos))
            && let Ok(block) = region.get_block(java_block_pos(pos))
        {
            block
        } else {
            Block::new("minecraft:air")
        }
    }

    fn get_batch_impl(&mut self, positions: impl Iterator<Item = IVec3>) -> Vec<Block> {
        let positions: Vec<IVec3> = positions.collect();
        let total = positions.len();
        let mut result: Vec<Option<Block>> = vec![None; total];

        let mut position_groups: FxHashMap<IVec2, Vec<(usize, Coords)>> =
            FxHashMap::with_capacity_and_hasher(total, FxBuildHasher::default());

        for (i, &pos) in positions.iter().enumerate() {
            let rpos = java_region_pos(pos);
            let block_pos = java_block_pos(pos);
            position_groups
                .entry(rpos)
                .or_default()
                .push((i, block_pos));
        }

        for (rpos, group) in position_groups {
            if let Some(region) = self.regions.get(&rpos) {
                let coords_list: Vec<Coords> = group.iter().map(|&(_, c)| c).collect();
                if let Ok(paletted_container) = region.get_blocks(&coords_list) {
                    for (index, coords) in group {
                        match paletted_container.get(coords) {
                            Ok(Some(block)) => result[index] = Some(block),
                            _ => result[index] = Some(Block::new("minecraft:air")),
                        }
                    }
                } else {
                    for (index, _) in group {
                        result[index] = Some(Block::new("minecraft:air"));
                    }
                    println!("here");
                }
            } else {
                for (index, _) in group {
                    result[index] = Some(Block::new("minecraft:air"));
                }
                println!("here");
            }
        }

        result
            .into_iter()
            .map(|opt| opt.expect("All indices should have been set"))
            .collect()
    }

    fn set_impl(&mut self, pos: IVec3, block: Block) {
        let rpos = java_region_pos(pos);
        let region = self
            .regions
            .entry(rpos)
            .or_insert_with(|| Region::empty(rpos.into()));
        region.set_block(java_block_pos(pos), block).unwrap();
        region.write_blocks().unwrap();
    }

    fn set_batch_impl(&mut self, blocks: impl Iterator<Item = (IVec3, Block)>) {
        let (min, max) = blocks.size_hint();
        let mut position_groups: FxHashMap<IVec2, Vec<_>> =
            FxHashMap::with_capacity_and_hasher(max.unwrap_or(min), FxBuildHasher::default());
        for (pos, block) in blocks {
            position_groups
                .entry(java_region_pos(pos))
                .or_default()
                .push((java_block_pos(pos), block));
        }

        for (rpos, group) in position_groups {
            let region = self
                .regions
                .entry(rpos)
                .or_insert_with(|| Region::empty(rpos.into()));
            for (bpos, block) in group {
                let _ = region.set_block(bpos, block);
            }
            region.write_blocks().unwrap();
        }
    }

    fn save(self, path: &Path) -> Result<()> {
        let path = path.join("World");
        if path.exists() {
            fs::remove_dir_all(&path)?;
        }
        fs::create_dir(&path)?;

        let region_path = path.join("region/");
        fs::create_dir(&region_path)?;

        for (pos, region) in self.regions {
            region.write(&mut File::create(
                region_path.join(format!("r.{}.{}.mca", pos.x, pos.y)),
            )?)?;
        }

        fs::write(
            path.join("level.dat"),
            include_bytes!("../../../assets/minecraft/java/level.dat"),
        )?;

        Ok(())
    }
}

pub fn java_region_pos(pos: IVec3) -> IVec2 {
    IVec2::new(pos.x >> 9, pos.y >> 9)
}

pub fn java_block_pos(global: IVec3) -> Coords {
    Coords::new((global.x & 511) as u32, global.z, (global.y & 511) as u32)
}

pub fn local_block_pos(region: IVec2, local: Coords) -> IVec3 {
    IVec3::new(
        (region.x << 9) | local.x as i32,
        local.y,
        (region.y << 9) | local.z as i32,
    )
}

#[derive(Debug, Default)]
pub struct VanillaSolver;

impl SubstanceSolver<Block> for VanillaSolver {
    fn substance_to_choice(&self, substance: Substance) -> Choice<Block> {
        if substance.typ == SubstanceType::Surface {
            return Choice::Single(Block::new("grass"));
        }
        match substance.kind {
            SubstanceKind::Material(material) => material_to_choice(material.into()),
            _ => material_to_choice(Material::Stones),
        }
    }
}

fn material_to_choice(material: Material) -> Choice<Block> {
    match material {
        Material::Acciaio => Choice::Single(Block::new("iron_block")),
        Material::Acrylic => Choice::Single(Block::new("glass")),
        Material::Adoquin => Choice::Single(Block::new("cobblestone")),
        Material::Adobe => Choice::Single(Block::new("mud_bricks")),
        Material::AllWeather => Choice::Single(Block::new("grass_block")),
        Material::Aluminium | Material::Aluminum => Choice::Single(Block::new("iron_block")),
        Material::Andesite => Choice::Single(Block::new("andesite")),
        Material::AntiSlip => Choice::Single(Block::new("cobblestone")),
        Material::Areia | Material::Arena => Choice::Single(Block::new("sand")),
        Material::Artificial => Choice::Single(Block::new("stone")),
        Material::ArtificialClay => Choice::Single(Block::new("clay")),
        Material::ArtificialStone => Choice::Single(Block::new("stone")),
        Material::ArtificialTurf => Choice::Single(Block::new("green_concrete")),
        Material::Ash => Choice::Uniform(vec![
            Block::new("gray_concrete"),
            Block::new("light_gray_concrete"),
            Block::new("gravel"),
        ]),
        Material::Asphalt(t) => match t {
            AsphaltType::Plain => Choice::Weighted(vec![
                (0.5, Block::new("blackstone")),
                (0.3, Block::new("polished_blackstone")),
                (0.2, Block::new("polished_blackstone_bricks")),
            ]),
            AsphaltType::Lanes => {
                Choice::Uniform(vec![Block::new("blackstone"), Block::new("gray_concrete")])
            }
            AsphaltType::Ruts => Choice::Weighted(vec![
                (0.6, Block::new("blackstone")),
                (0.4, Block::new("gravel")),
            ]),
            AsphaltType::Compacted => Choice::Single(Block::new("blackstone")),
            AsphaltType::Grooved => Choice::Single(Block::new("blackstone")),
        },
        Material::AsphaltCompacted | Material::AsphaltLanes | Material::AsphaltRuts => {
            Choice::Single(Block::new("blackstone"))
        }
        Material::Azulejo => Choice::Uniform(vec![
            Block::new("blue_glazed_terracotta"),
            Block::new("cyan_glazed_terracotta"),
            Block::new("light_blue_glazed_terracotta"),
        ]),
        Material::Ballast => Choice::Weighted(vec![
            (0.7, Block::new("gravel")),
            (0.3, Block::new("cobblestone")),
        ]),
        Material::Bamboo => Choice::Single(Block::new("bamboo_block")),
        Material::BareGround => Choice::Weighted(vec![
            (0.6, Block::new("dirt")),
            (0.3, Block::new("coarse_dirt")),
            (0.1, Block::new("rooted_dirt")),
        ]),
        Material::BareRock => Choice::Uniform(vec![
            Block::new("stone"),
            Block::new("andesite"),
            Block::new("cobblestone"),
        ]),
        Material::Bark => Choice::Uniform(vec![
            Block::new("oak_wood"),
            Block::new("dark_oak_wood"),
            Block::new("spruce_wood"),
        ]),
        Material::BarkMulch => {
            Choice::Uniform(vec![Block::new("coarse_dirt"), Block::new("podzol")])
        }
        Material::Basalt | Material::Basalte => Choice::Single(Block::new("basalt")),
        Material::Bata => Choice::Single(Block::new("bricks")),
        Material::BatuBata => Choice::Single(Block::new("bricks")),
        Material::Bedrock => Choice::Single(Block::new("bedrock")),
        Material::Beton => Choice::Single(Block::new("gray_concrete")),
        Material::Bicouche => Choice::Single(Block::new("stone")),
        Material::Bing => Choice::Single(Block::new("gravel")),
        Material::Bitmac | Material::Bitumen | Material::Bituminous | Material::Blacktopped => {
            Choice::Weighted(vec![
                (0.8, Block::new("blackstone")),
                (0.2, Block::new("gray_concrete")),
            ])
        }
        Material::Blaes => Choice::Single(Block::new("gravel")),
        Material::BlockPaving => Choice::Weighted(vec![
            (0.6, Block::new("stone_bricks")),
            (0.3, Block::new("cracked_stone_bricks")),
            (0.1, Block::new("mossy_stone_bricks")),
        ]),
        Material::Blocks => Choice::Uniform(vec![
            Block::new("stone"),
            Block::new("andesite"),
            Block::new("diorite"),
        ]),
        Material::Bluestone => Choice::Single(Block::new("prismarine_bricks")),
        Material::Boardwalk => Choice::Weighted(vec![
            (0.7, Block::new("oak_planks")),
            (0.3, Block::new("spruce_planks")),
        ]),
        Material::Bois => Choice::Weighted(vec![
            (0.5, Block::new("oak_planks")),
            (0.3, Block::new("birch_planks")),
            (0.2, Block::new("spruce_planks")),
        ]),
        Material::Boulder | Material::Boulders => Choice::Uniform(vec![
            Block::new("cobblestone"),
            Block::new("mossy_cobblestone"),
            Block::new("stone"),
        ]),
        Material::Branches => {
            Choice::Uniform(vec![Block::new("oak_log"), Block::new("dark_oak_log")])
        }
        Material::Brass => Choice::Single(Block::new("gold_block")),
        Material::Brick | Material::Bricks | Material::BrickWeave => Choice::Weighted(vec![
            (0.7, Block::new("bricks")),
            (0.2, Block::new("cracked_stone_bricks")),
            (0.1, Block::new("mossy_stone_bricks")),
        ]),
        Material::Bronze => Choice::Single(Block::new("copper_block")),
        Material::Brushwood | Material::Bush | Material::Bushes => Choice::Uniform(vec![
            Block::new("oak_leaves"),
            Block::new("dark_oak_leaves"),
            Block::new("birch_leaves"),
        ]),
        Material::BushHammering => Choice::Single(Block::new("stone")),
        Material::Cable => Choice::Single(Block::new("chain")),
        Material::Calcaire => Choice::Single(Block::new("calcite")),
        Material::Canvas => Choice::Single(Block::new("white_wool")),
        Material::Cardon => Choice::Single(Block::new("cactus")),
        Material::Carpet => Choice::Uniform(vec![
            Block::new("white_carpet"),
            Block::new("light_gray_carpet"),
        ]),
        Material::CarraraMarble => Choice::Single(Block::new("calcite")),
        Material::CastIron => Choice::Single(Block::new("iron_block")),
        Material::Cement(t) => match t {
            CementType::Brick => Choice::Single(Block::new("bricks")),
            CementType::Plain => Choice::Single(Block::new("white_concrete")),
        },
        Material::CementBlock
        | Material::CementBlockPlastered
        | Material::CementBlockPlasteredPaint => Choice::Uniform(vec![
            Block::new("white_concrete"),
            Block::new("light_gray_concrete"),
        ]),
        Material::Ceramic | Material::Ceramics => Choice::Uniform(vec![
            Block::new("terracotta"),
            Block::new("white_glazed_terracotta"),
            Block::new("red_glazed_terracotta"),
        ]),
        Material::Cesped => Choice::Single(Block::new("grass_block")),
        Material::Chain | Material::ChainLink | Material::Chainlink => {
            Choice::Single(Block::new("chain"))
        }
        Material::Chalk => Choice::Single(Block::new("calcite")),
        Material::Chipseal => Choice::Weighted(vec![
            (0.7, Block::new("gravel")),
            (0.3, Block::new("blackstone")),
        ]),
        Material::Cinder | Material::Cinderblock => {
            Choice::Uniform(vec![Block::new("blackstone"), Block::new("basalt")])
        }
        Material::Clay => Choice::Single(Block::new("clay")),
        Material::ClinkerPlates => Choice::Uniform(vec![
            Block::new("stone_bricks"),
            Block::new("cracked_stone_bricks"),
        ]),
        Material::Cloth => Choice::Single(Block::new("white_wool")),
        Material::Coal => Choice::Single(Block::new("coal_block")),
        Material::Cobblestone(t) => match t {
            CobblestoneType::Plain => Choice::Weighted(vec![
                (0.6, Block::new("cobblestone")),
                (0.3, Block::new("mossy_cobblestone")),
                (0.1, Block::new("stone")),
            ]),
            CobblestoneType::Flattened => Choice::Single(Block::new("cobblestone")),
            CobblestoneType::Lanes => Choice::Uniform(vec![
                Block::new("cobblestone"),
                Block::new("mossy_cobblestone"),
            ]),
        },
        Material::Compacted(t) => match t {
            CompactedType::Plain => Choice::Weighted(vec![
                (0.7, Block::new("coarse_dirt")),
                (0.3, Block::new("gravel")),
            ]),
            CompactedType::Lanes => Choice::Single(Block::new("coarse_dirt")),
        },
        Material::Composite => Choice::Single(Block::new("stone")),
        Material::CompositeWood => {
            Choice::Uniform(vec![Block::new("oak_planks"), Block::new("birch_planks")])
        }
        Material::Concrete(t) => match t {
            ConcreteType::Plain | ConcreteType::Reinforced => Choice::Uniform(vec![
                Block::new("gray_concrete"),
                Block::new("light_gray_concrete"),
            ]),
            ConcreteType::Plates => Choice::Single(Block::new("stone_bricks")),
            ConcreteType::Slabs => Choice::Single(Block::new("smooth_stone_slab")),
            ConcreteType::Tiles => Choice::Uniform(vec![
                Block::new("stone_bricks"),
                Block::new("polished_andesite"),
            ]),
            ConcreteType::Blocks => Choice::Uniform(vec![
                Block::new("gray_concrete"),
                Block::new("light_gray_concrete"),
            ]),
            ConcreteType::Pebbles => Choice::Single(Block::new("gravel")),
            _ => todo!(),
        },
        Material::ConcreteBlock => Choice::Uniform(vec![
            Block::new("gray_concrete"),
            Block::new("light_gray_concrete"),
        ]),
        Material::ConcreteWood => Choice::Single(Block::new("oak_planks")),
        Material::Concret => Choice::Single(Block::new("gray_concrete")),
        Material::Construction => Choice::Single(Block::new("stone")),
        Material::Copper => Choice::Single(Block::new("copper_block")),
        Material::Coral => Choice::Uniform(vec![
            Block::new("brain_coral_block"),
            Block::new("tube_coral_block"),
            Block::new("bubble_coral_block"),
        ]),
        Material::CoralSand => Choice::Single(Block::new("sand")),
        Material::CorTen | Material::Corten | Material::CortenSteel => {
            Choice::Single(Block::new("iron_block"))
        }
        Material::CorrugatedIron | Material::CorrugatedSteel => {
            Choice::Single(Block::new("iron_block"))
        }
        Material::CornishGranite => Choice::Single(Block::new("granite")),
        Material::CrusherDust
        | Material::CrusherFines
        | Material::CrushedGranite
        | Material::CrushedLimestone
        | Material::CrushedShells
        | Material::CrushedStone => {
            Choice::Uniform(vec![Block::new("gravel"), Block::new("coarse_dirt")])
        }
        Material::Debris => Choice::Weighted(vec![
            (0.5, Block::new("cobblestone")),
            (0.3, Block::new("gravel")),
            (0.2, Block::new("dirt")),
        ]),
        Material::Decoturf => Choice::Single(Block::new("grass_block")),
        Material::DecomposedGranite => {
            Choice::Uniform(vec![Block::new("coarse_dirt"), Block::new("gravel")])
        }
        Material::Diabase => Choice::Single(Block::new("stone")),
        Material::Dirt | Material::DirtRock => Choice::Weighted(vec![
            (0.7, Block::new("dirt")),
            (0.3, Block::new("coarse_dirt")),
        ]),
        Material::Dolomite => Choice::Single(Block::new("calcite")),
        Material::DryStone => Choice::Uniform(vec![
            Block::new("cobblestone"),
            Block::new("mossy_cobblestone"),
        ]),
        Material::DuctileIron => Choice::Single(Block::new("iron_block")),
        Material::DuneSand => Choice::Single(Block::new("sand")),
        Material::Durapol => Choice::Single(Block::new("stone")),
        Material::Earth => Choice::Weighted(vec![
            (0.6, Block::new("dirt")),
            (0.3, Block::new("coarse_dirt")),
            (0.1, Block::new("rooted_dirt")),
        ]),
        Material::Element(el) => match el {
            Element::Ag | Element::Al => Choice::Single(Block::new("iron_block")),
            Element::Cu => Choice::Single(Block::new("copper_block")),
        },
        Material::Empedrado => Choice::Single(Block::new("cobblestone")),
        Material::EnameledLava => Choice::Single(Block::new("basalt")),
        Material::Epoxy => Choice::Single(Block::new("white_concrete")),
        Material::Eternit => Choice::Single(Block::new("gray_concrete")),
        Material::ExposedAggregateConcrete => Choice::Weighted(vec![
            (0.7, Block::new("stone")),
            (0.3, Block::new("gravel")),
        ]),
        Material::Fabric => Choice::Single(Block::new("white_wool")),
        Material::Fiberglass | Material::Fibreglass => Choice::Single(Block::new("glass")),
        Material::FibreReinforcedPolymerGrate | Material::FrpGrate => {
            Choice::Single(Block::new("iron_bars"))
        }
        Material::Fibrocemento => Choice::Single(Block::new("white_concrete")),
        Material::FineGravel => Choice::Single(Block::new("gravel")),
        Material::FineSand => Choice::Single(Block::new("sand")),
        Material::Flag | Material::Flagstone => Choice::Uniform(vec![
            Block::new("stone_bricks"),
            Block::new("cracked_stone_bricks"),
        ]),
        Material::Flint => Choice::Uniform(vec![Block::new("stone"), Block::new("gravel")]),
        Material::Flowerbed => Choice::Uniform(vec![
            Block::new("grass_block"),
            Block::new("poppy"),
            Block::new("dandelion"),
            Block::new("allium"),
            Block::new("azure_bluet"),
        ]),
        Material::Fonte => Choice::Single(Block::new("stone")),
        Material::Footway => Choice::Single(Block::new("stone")),
        Material::Frp => Choice::Single(Block::new("white_concrete")),
        Material::Gabion => Choice::Weighted(vec![
            (0.6, Block::new("cobblestone")),
            (0.4, Block::new("stone")),
        ]),
        Material::Gas => Choice::Single(Block::new("glass")),
        Material::Geofabric => Choice::Single(Block::new("white_wool")),
        Material::Glacier => Choice::Uniform(vec![Block::new("ice"), Block::new("packed_ice")]),
        Material::Glass | Material::GlassFibre => Choice::Single(Block::new("glass")),
        Material::Gneiss => Choice::Single(Block::new("stone")),
        Material::Gold => Choice::Single(Block::new("gold_block")),
        Material::Grain => Choice::Single(Block::new("hay_block")),
        Material::Grama => Choice::Single(Block::new("grass_block")),
        Material::Granite(t) => match t {
            GraniteType::Plain => Choice::Single(Block::new("granite")),
            GraniteType::Plates => Choice::Single(Block::new("polished_granite")),
        },
        Material::GraniteStone | Material::Granit => {
            Choice::Uniform(vec![Block::new("granite"), Block::new("polished_granite")])
        }
        Material::Grass | Material::Grassland | Material::GrassScrub => {
            Choice::Single(Block::new("grass_block"))
        }
        Material::GrassPaver(t) => match t {
            GrassPaverType::Solid => Choice::Weighted(vec![
                (0.6, Block::new("grass_block")),
                (0.4, Block::new("stone")),
            ]),
            GrassPaverType::Lanes => Choice::Single(Block::new("grass_block")),
        },
        Material::Gravel(t) => match t {
            GravelType::Plain | GravelType::Lanes => Choice::Weighted(vec![
                (0.8, Block::new("gravel")),
                (0.2, Block::new("cobblestone")),
            ]),
        },
        Material::Green | Material::GreenPaver | Material::Greenset => {
            Choice::Single(Block::new("grass_block"))
        }
        Material::Greywacke => Choice::Single(Block::new("stone")),
        Material::Grit => Choice::Single(Block::new("gravel")),
        Material::Ground(t) => match t {
            GroundType::Plain => Choice::Weighted(vec![
                (0.7, Block::new("dirt")),
                (0.3, Block::new("coarse_dirt")),
            ]),
            GroundType::Lanes => Choice::Single(Block::new("coarse_dirt")),
        },
        Material::Grus => Choice::Single(Block::new("gravel")),
        Material::Gypsum => Choice::Single(Block::new("white_concrete")),
        Material::Hard | Material::Hardcore => Choice::Single(Block::new("stone")),
        Material::HardCourt => Choice::Uniform(vec![
            Block::new("stone_bricks"),
            Block::new("gray_concrete"),
        ]),
        Material::Hardwood => Choice::Uniform(vec![
            Block::new("oak_planks"),
            Block::new("dark_oak_planks"),
        ]),
        Material::Hay => Choice::Single(Block::new("hay_block")),
        Material::Hotmix => Choice::Weighted(vec![
            (0.9, Block::new("blackstone")),
            (0.1, Block::new("gravel")),
        ]),
        Material::Ice => Choice::Uniform(vec![Block::new("ice"), Block::new("packed_ice")]),
        Material::IceRoad => Choice::Single(Block::new("packed_ice")),
        Material::InkOnPaper => Choice::Single(Block::new("white_wool")),
        Material::Interlock => Choice::Uniform(vec![
            Block::new("stone_bricks"),
            Block::new("cracked_stone_bricks"),
        ]),
        Material::Intermediate => Choice::Single(Block::new("stone")),
        Material::Iron => Choice::Single(Block::new("iron_block")),
        Material::IronOre => Choice::Single(Block::new("iron_ore")),
        Material::Karral => Choice::Single(Block::new("stone")),
        Material::Ladrillo => Choice::Weighted(vec![
            (0.8, Block::new("bricks")),
            (0.2, Block::new("terracotta")),
        ]),
        Material::Lahar => Choice::Single(Block::new("stone")),
        Material::Laterite => {
            Choice::Uniform(vec![Block::new("terracotta"), Block::new("red_sandstone")])
        }
        Material::Lava => Choice::Single(Block::new("lava")),
        Material::Lead => Choice::Single(Block::new("iron_block")),
        Material::LeafLitter | Material::Leaves => Choice::Uniform(vec![
            Block::new("oak_leaves"),
            Block::new("dark_oak_leaves"),
            Block::new("birch_leaves"),
        ]),
        Material::Leather => Choice::Single(Block::new("brown_wool")),
        Material::Light => Choice::Single(Block::new("glowstone")),
        Material::Limerock | Material::Limestone => Choice::Single(Block::new("calcite")),
        Material::Linoleum => Choice::Uniform(vec![
            Block::new("white_terracotta"),
            Block::new("light_gray_terracotta"),
        ]),
        Material::Log | Material::Logs => Choice::Uniform(vec![
            Block::new("oak_log"),
            Block::new("spruce_log"),
            Block::new("dark_oak_log"),
        ]),
        Material::LooseEarth => {
            Choice::Uniform(vec![Block::new("dirt"), Block::new("coarse_dirt")])
        }
        Material::LooseFineGravel | Material::LooseGravel => {
            Choice::Uniform(vec![Block::new("gravel"), Block::new("coarse_dirt")])
        }
        Material::LooseSand => Choice::Single(Block::new("sand")),
        Material::Macadam => Choice::Weighted(vec![
            (0.7, Block::new("gravel")),
            (0.3, Block::new("cobblestone")),
        ]),
        Material::Madera => Choice::Weighted(vec![
            (0.5, Block::new("oak_planks")),
            (0.3, Block::new("birch_planks")),
            (0.2, Block::new("spruce_planks")),
        ]),
        Material::Marble | Material::MarblePlate | Material::MarblePlates => {
            Choice::Single(Block::new("calcite"))
        }
        Material::MarblePowder => Choice::Single(Block::new("white_concrete_powder")),
        Material::Marsh | Material::Mud => Choice::Single(Block::new("mud")),
        Material::Masonry => Choice::Weighted(vec![
            (0.6, Block::new("stone_bricks")),
            (0.3, Block::new("cracked_stone_bricks")),
            (0.1, Block::new("mossy_stone_bricks")),
        ]),
        Material::Mat | Material::Mats => Choice::Single(Block::new("white_wool")),
        Material::MDF => Choice::Single(Block::new("oak_planks")),
        Material::Meadow => Choice::Weighted(vec![
            (0.9, Block::new("grass_block")),
            (0.1, Block::new("allium")),
        ]),
        Material::Mesh => Choice::Single(Block::new("iron_bars")),
        Material::Metal | Material::Metall => Choice::Single(Block::new("iron_block")),
        Material::MetalForm(form) => match form {
            MetalForm::Bars
            | MetalForm::Grid
            | MetalForm::Wire
            | MetalForm::Palisade
            | MetalForm::Grate => Choice::Single(Block::new("iron_bars")),
            MetalForm::Sheet | MetalForm::Corrugated => Choice::Single(Block::new("iron_block")),
        },
        Material::MetalGrid => Choice::Single(Block::new("iron_bars")),
        Material::Mirror => Choice::Single(Block::new("glass")),
        Material::Mixed => Choice::Uniform(vec![Block::new("dirt"), Block::new("coarse_dirt")]),
        Material::Monocapa => Choice::Single(Block::new("white_concrete")),
        Material::Mosaic => Choice::Uniform(vec![
            Block::new("blue_glazed_terracotta"),
            Block::new("cyan_glazed_terracotta"),
            Block::new("light_blue_glazed_terracotta"),
            Block::new("purple_glazed_terracotta"),
            Block::new("magenta_glazed_terracotta"),
        ]),
        Material::Moss => Choice::Single(Block::new("moss_block")),
        Material::Mulch => Choice::Uniform(vec![Block::new("podzol"), Block::new("coarse_dirt")]),
        Material::Multi => Choice::Single(Block::new("stone")),
        Material::Murram => Choice::Single(Block::new("gravel")),
        Material::Muschelkalk => Choice::Single(Block::new("calcite")),
        Material::Natural | Material::NaturalStone | Material::NaturalStones => {
            Choice::Uniform(vec![
                Block::new("stone"),
                Block::new("andesite"),
                Block::new("cobblestone"),
                Block::new("mossy_cobblestone"),
            ])
        }
        Material::Net => Choice::Single(Block::new("iron_bars")),
        Material::No | Material::None => Choice::Single(Block::new("barrier")),
        Material::NorthbraeRhyolite => Choice::Single(Block::new("stone")),
        Material::Oak => Choice::Uniform(vec![
            Block::new("oak_planks"),
            Block::new("dark_oak_planks"),
        ]),
        Material::Oil => Choice::Single(Block::new("black_concrete")),
        Material::Ottaseal => Choice::Single(Block::new("blackstone")),
        Material::Overgrown => Choice::Weighted(vec![
            (0.6, Block::new("grass_block")),
            (0.3, Block::new("cobblestone")),
            (0.1, Block::new("mossy_cobblestone")),
        ]),
        Material::PackedGravel => Choice::Weighted(vec![
            (0.8, Block::new("gravel")),
            (0.2, Block::new("cobblestone")),
        ]),
        Material::Paint | Material::Painted => Choice::Single(Block::new("white_concrete")),
        Material::PaintedWood => {
            Choice::Uniform(vec![Block::new("oak_planks"), Block::new("birch_planks")])
        }
        Material::PalmFrond | Material::PalmLeaves | Material::PalmLeavs => {
            Choice::Single(Block::new("jungle_leaves"))
        }
        Material::Panel => Choice::Single(Block::new("iron_block")),
        Material::Paper => Choice::Single(Block::new("white_wool")),
        Material::Path => Choice::Uniform(vec![Block::new("dirt_path"), Block::new("coarse_dirt")]),
        Material::Paver
        | Material::Pavers
        | Material::Paved
        | Material::PavedStones
        | Material::PavingBlock => Choice::Weighted(vec![
            (0.7, Block::new("stone_bricks")),
            (0.2, Block::new("cracked_stone_bricks")),
            (0.1, Block::new("mossy_stone_bricks")),
        ]),
        Material::PavingSlab | Material::PavingSlabs => {
            Choice::Single(Block::new("stone_brick_slab"))
        }
        Material::PavingStones(t) => match t {
            PavingStonesType::Plain
            | PavingStonesType::Lanes
            | PavingStonesType::V20
            | PavingStonesType::V50 => Choice::Weighted(vec![
                (0.6, Block::new("stone_bricks")),
                (0.3, Block::new("cracked_stone_bricks")),
                (0.1, Block::new("mossy_stone_bricks")),
            ]),
        },
        Material::Peat => Choice::Single(Block::new("mud")),
        Material::Pebbledash | Material::Pebblestone => {
            Choice::Uniform(vec![Block::new("gravel"), Block::new("stone")])
        }
        Material::Pierre | Material::Pietra => {
            Choice::Uniform(vec![Block::new("stone"), Block::new("andesite")])
        }
        Material::PierreSeche => Choice::Uniform(vec![
            Block::new("cobblestone"),
            Block::new("mossy_cobblestone"),
        ]),
        Material::Planks => Choice::Uniform(vec![
            Block::new("oak_planks"),
            Block::new("spruce_planks"),
            Block::new("birch_planks"),
        ]),
        Material::Plant | Material::Plants => Choice::Uniform(vec![
            Block::new("grass_block"),
            Block::new("fern"),
            Block::new("dandelion"),
        ]),
        Material::Plaster => Choice::Uniform(vec![
            Block::new("white_concrete"),
            Block::new("light_gray_concrete"),
        ]),
        Material::Plastic => Choice::Single(Block::new("white_concrete")),
        Material::Plates => Choice::Weighted(vec![
            (0.8, Block::new("stone_bricks")),
            (0.2, Block::new("cracked_stone_bricks")),
        ]),
        Material::Plexiglass => Choice::Single(Block::new("glass")),
        Material::Pole => Choice::Uniform(vec![Block::new("oak_log"), Block::new("spruce_log")]),
        Material::Polyester => Choice::Single(Block::new("white_wool")),
        Material::Polyethene | Material::Polyethylene | Material::Polyurethane => {
            Choice::Single(Block::new("white_concrete"))
        }
        Material::Porcelain => Choice::Uniform(vec![
            Block::new("white_glazed_terracotta"),
            Block::new("light_gray_glazed_terracotta"),
        ]),
        Material::Porphyrtuff => Choice::Single(Block::new("stone")),
        Material::PortlandStone => Choice::Single(Block::new("calcite")),
        Material::Prefab | Material::PrefabConcrete => Choice::Uniform(vec![
            Block::new("gray_concrete"),
            Block::new("light_gray_concrete"),
        ]),
        Material::PVC => Choice::Single(Block::new("white_concrete")),
        Material::Quartzite => Choice::Uniform(vec![
            Block::new("quartz_block"),
            Block::new("smooth_quartz"),
        ]),
        Material::RailwaySleepers => Choice::Uniform(vec![
            Block::new("oak_planks"),
            Block::new("dark_oak_planks"),
        ]),
        Material::RammedEarth => Choice::Single(Block::new("coarse_dirt")),
        Material::RecycledMaterial => {
            Choice::Uniform(vec![Block::new("stone"), Block::new("cobblestone")])
        }
        Material::RecycledPlastic => Choice::Single(Block::new("white_concrete")),
        Material::RedCedar | Material::Redwood => Choice::Uniform(vec![
            Block::new("acacia_planks"),
            Block::new("dark_oak_planks"),
        ]),
        Material::RedEarth => {
            Choice::Uniform(vec![Block::new("red_sand"), Block::new("terracotta")])
        }
        Material::RedSandstone => Choice::Single(Block::new("red_sandstone")),
        Material::Reed => Choice::Single(Block::new("sugar_cane")),
        Material::Render => Choice::Single(Block::new("white_concrete")),
        Material::Resin => Choice::Single(Block::new("honey_block")),
        Material::Rock | Material::Rocks | Material::Rocky => Choice::Uniform(vec![
            Block::new("stone"),
            Block::new("cobblestone"),
            Block::new("andesite"),
        ]),
        Material::RockDust => Choice::Single(Block::new("gravel")),
        Material::RomanPaving => Choice::Uniform(vec![
            Block::new("stone_bricks"),
            Block::new("cracked_stone_bricks"),
        ]),
        Material::RoofTiles => Choice::Weighted(vec![
            (0.6, Block::new("bricks")),
            (0.3, Block::new("terracotta")),
            (0.1, Block::new("cracked_stone_bricks")),
        ]),
        Material::Roots => Choice::Single(Block::new("mangrove_roots")),
        Material::Rope => Choice::Single(Block::new("chain")),
        Material::Rubber
        | Material::RubberCrumb
        | Material::RubberMulch
        | Material::RubberTiles
        | Material::Rubberized => Choice::Single(Block::new("blackstone")),
        Material::Rubble => Choice::Weighted(vec![
            (0.6, Block::new("cobblestone")),
            (0.3, Block::new("gravel")),
            (0.1, Block::new("dirt")),
        ]),
        Material::Saibro => Choice::Single(Block::new("gravel")),
        Material::Salt => Choice::Single(Block::new("white_concrete")),
        Material::Sand | Material::Sandbag => Choice::Single(Block::new("sand")),
        Material::Sandstone => Choice::Single(Block::new("sandstone")),
        Material::Sawdust => Choice::Single(Block::new("oak_planks")),
        Material::ScrapMetal => {
            Choice::Uniform(vec![Block::new("iron_block"), Block::new("iron_bars")])
        }
        Material::Scree => Choice::Weighted(vec![
            (0.8, Block::new("gravel")),
            (0.2, Block::new("cobblestone")),
        ]),
        Material::Screen => Choice::Single(Block::new("iron_bars")),
        Material::Scrub => Choice::Weighted(vec![
            (0.7, Block::new("grass_block")),
            (0.3, Block::new("oak_leaves")),
        ]),
        Material::SeaShells | Material::Shell | Material::Shells => Choice::Uniform(vec![
            Block::new("dead_brain_coral_block"),
            Block::new("sand"),
        ]),
        Material::Sealed => Choice::Single(Block::new("stone")),
        Material::Sett(t) => match t {
            SettType::Plain => Choice::Weighted(vec![
                (0.7, Block::new("stone_bricks")),
                (0.2, Block::new("cracked_stone_bricks")),
                (0.1, Block::new("mossy_stone_bricks")),
            ]),
            SettType::Plates => Choice::Single(Block::new("stone_brick_slab")),
        },
        Material::Shadecloth => Choice::Single(Block::new("black_wool")),
        Material::Shale => Choice::Single(Block::new("deepslate")),
        Material::SheetMetal => Choice::Single(Block::new("iron_block")),
        Material::ShellLimestone => Choice::Uniform(vec![
            Block::new("calcite"),
            Block::new("dead_brain_coral_block"),
        ]),
        Material::Shingle => Choice::Uniform(vec![Block::new("gravel"), Block::new("cobblestone")]),
        Material::ShortGrass => Choice::Single(Block::new("grass_block")),
        Material::Shrub | Material::Shrubbery => Choice::Uniform(vec![
            Block::new("oak_leaves"),
            Block::new("dark_oak_leaves"),
        ]),
        Material::SilicateBrick => Choice::Single(Block::new("bricks")),
        Material::Singletrack => Choice::Single(Block::new("dirt_path")),
        Material::Slabs => Choice::Single(Block::new("stone_brick_slab")),
        Material::Slag => Choice::Weighted(vec![
            (0.8, Block::new("blackstone")),
            (0.2, Block::new("gravel")),
        ]),
        Material::Slate => Choice::Single(Block::new("deepslate")),
        Material::Snow | Material::SnowCrust => Choice::Single(Block::new("snow_block")),
        Material::Sods => Choice::Single(Block::new("grass_block")),
        Material::Soft | Material::Soil => {
            Choice::Uniform(vec![Block::new("dirt"), Block::new("coarse_dirt")])
        }
        Material::StainlessSteel | Material::Steel => Choice::Single(Block::new("iron_block")),
        Material::SteelGrating => Choice::Single(Block::new("iron_bars")),
        Material::SteppingStones => Choice::Uniform(vec![
            Block::new("stone_bricks"),
            Block::new("cracked_stone_bricks"),
        ]),
        Material::Sterrato => {
            Choice::Weighted(vec![(0.7, Block::new("gravel")), (0.3, Block::new("dirt"))])
        }
        Material::Stone(t) => match t {
            StoneType::Plain => Choice::Uniform(vec![
                Block::new("stone"),
                Block::new("andesite"),
                Block::new("cobblestone"),
            ]),
            StoneType::Plates => Choice::Uniform(vec![
                Block::new("stone_bricks"),
                Block::new("cracked_stone_bricks"),
            ]),
            StoneType::Slabs => Choice::Single(Block::new("smooth_stone_slab")),
        },
        Material::StoneDust => Choice::Single(Block::new("gravel")),
        Material::Stones => Choice::Uniform(vec![Block::new("stone"), Block::new("cobblestone")]),
        Material::Stoneware => Choice::Uniform(vec![
            Block::new("terracotta"),
            Block::new("white_glazed_terracotta"),
        ]),
        Material::Stony => Choice::Uniform(vec![Block::new("stone"), Block::new("gravel")]),
        Material::Straw => Choice::Single(Block::new("hay_block")),
        Material::Stucco => Choice::Uniform(vec![
            Block::new("white_concrete"),
            Block::new("light_gray_concrete"),
        ]),
        Material::Surface => Choice::Single(Block::new("stone")),
        Material::Synthetic | Material::SyntheticWood => {
            Choice::Uniform(vec![Block::new("oak_planks"), Block::new("birch_planks")])
        }
        Material::TactilePaving => Choice::Uniform(vec![
            Block::new("stone_bricks"),
            Block::new("yellow_concrete"),
        ]),
        Material::Tailings => Choice::Weighted(vec![
            (0.8, Block::new("gravel")),
            (0.2, Block::new("cobblestone")),
        ]),
        Material::TanahLiat => Choice::Single(Block::new("clay")),
        Material::Tared | Material::Tarmac | Material::TarRoad => {
            Choice::Uniform(vec![Block::new("blackstone"), Block::new("gray_concrete")])
        }
        Material::Tartan => Choice::Single(Block::new("red_concrete")),
        Material::Terra | Material::Tierra => {
            Choice::Uniform(vec![Block::new("dirt"), Block::new("coarse_dirt")])
        }
        Material::Terracotta => Choice::Single(Block::new("terracotta")),
        Material::Terraway => Choice::Single(Block::new("dirt")),
        Material::TerreBattue => Choice::Single(Block::new("terracotta")),
        Material::Terrazzo => Choice::Uniform(vec![
            Block::new("polished_diorite"),
            Block::new("polished_andesite"),
            Block::new("polished_granite"),
        ]),
        Material::Thermoplastic => Choice::Single(Block::new("white_concrete")),
        Material::Tiles | Material::Tile => Choice::Uniform(vec![
            Block::new("stone_bricks"),
            Block::new("polished_andesite"),
        ]),
        Material::Timber => Choice::Uniform(vec![Block::new("oak_log"), Block::new("spruce_log")]),
        Material::Tin => Choice::Single(Block::new("iron_block")),
        Material::Tire | Material::Tyres => Choice::Single(Block::new("blackstone")),
        Material::Toprak => Choice::Single(Block::new("dirt")),
        Material::Track => Choice::Single(Block::new("dirt_path")),
        Material::TrackBallast => Choice::Weighted(vec![
            (0.9, Block::new("gravel")),
            (0.1, Block::new("cobblestone")),
        ]),
        Material::Trachyte => Choice::Single(Block::new("stone")),
        Material::Trail => Choice::Single(Block::new("dirt_path")),
        Material::Travertine => Choice::Single(Block::new("calcite")),
        Material::Tree | Material::Trees | Material::TreeTrunk => Choice::Uniform(vec![
            Block::new("oak_log"),
            Block::new("spruce_log"),
            Block::new("dark_oak_log"),
        ]),
        Material::Trylinka => Choice::Uniform(vec![
            Block::new("stone_bricks"),
            Block::new("gray_concrete"),
        ]),
        Material::Tufa | Material::Tuff => Choice::Single(Block::new("tuff")),
        Material::Turf => Choice::Single(Block::new("grass_block")),
        Material::Uneven => Choice::Uniform(vec![
            Block::new("cobblestone"),
            Block::new("stone"),
            Block::new("gravel"),
        ]),
        Material::UnhewnCobblestone => Choice::Single(Block::new("cobblestone")),
        Material::Unpaved => {
            Choice::Weighted(vec![(0.7, Block::new("dirt")), (0.3, Block::new("gravel"))])
        }
        Material::Unspecified => Choice::Single(Block::new("stone")),
        Material::Vinyl => Choice::Single(Block::new("white_concrete")),
        Material::WallStone => Choice::Weighted(vec![
            (0.6, Block::new("stone_bricks")),
            (0.3, Block::new("cracked_stone_bricks")),
            (0.1, Block::new("mossy_stone_bricks")),
        ]),
        Material::Water => Choice::Single(Block::new("water")),
        Material::WeatheringSteel => Choice::Single(Block::new("iron_block")),
        Material::Wicker => Choice::Single(Block::new("bamboo_block")),
        Material::Willow => Choice::Single(Block::new("oak_planks")),
        Material::WinterRoad => {
            Choice::Uniform(vec![Block::new("snow_block"), Block::new("packed_ice")])
        }
        Material::WireMesh => Choice::Single(Block::new("iron_bars")),
        Material::Wood => Choice::Weighted(vec![
            (0.5, Block::new("oak_planks")),
            (0.3, Block::new("spruce_planks")),
            (0.2, Block::new("birch_planks")),
        ]),
        Material::WoodChips => Choice::Uniform(vec![
            Block::new("oak_planks"),
            Block::new("dark_oak_planks"),
        ]),
        Material::WoodMetal => Choice::Single(Block::new("iron_block")),
        Material::WoodPole | Material::WoodPost => {
            Choice::Uniform(vec![Block::new("oak_log"), Block::new("spruce_log")])
        }
        Material::WroughtIron => Choice::Single(Block::new("iron_bars")),
        Material::Yes => Choice::Single(Block::new("stone")),
        Material::YorkStone => Choice::Uniform(vec![
            Block::new("stone_bricks"),
            Block::new("cracked_stone_bricks"),
        ]),
        Material::Zinc => Choice::Single(Block::new("iron_block")),
    }
}
