//! Raw world inputs for target `ObjectMgr::LoadPlayerInfo` (02245dcd).
//! This batch is not an admitted PlayerInfo or a character-save request.
//! DB2/map/spell/item validation and Player initialization remain mandatory.

use super::LoadError;
use crate::PersistenceFutureLikeCpp;
pub mod templates;

#[derive(Clone, Copy)]
pub struct StartDefinition {
    pub race: u8,
    pub class: u8,
    pub map: u16,
    pub position: [f32; 4],
    /// Preserve each SQL NULL independently; source enables NPE only when
    /// all five map/position columns are present (transport is independent).
    pub npe_map: Option<u32>,
    pub npe_position: [Option<f32>; 4],
    pub npe_transport: Option<u64>,
    pub intro_movie: Option<u32>,
    pub intro_scene: Option<u32>,
    pub npe_intro_scene: Option<u32>,
}

#[derive(Clone, Copy)]
pub struct RaceStats {
    pub race: u8,
    pub modifiers: [i16; 5],
}

#[derive(Clone, Copy)]
pub struct ClassLevelStats {
    pub class: u8,
    pub level: u8,
    /// Target PlayerLevelInfo uses int32, NOT the legacy Rust uint16 table.
    pub stats: [i32; 5],
}

#[derive(Clone, Copy)]
pub struct StartAction {
    pub race: u8,
    pub class: u8,
    /// Source SQL reads uint16 before PlayerCreateInfoAction's narrowing.
    pub button: u16,
    pub action: u32,
    pub kind: u16,
}

#[derive(Clone, Copy)]
pub struct ItemOverride {
    pub race: u8,
    pub class: u8,
    pub item: u32,
    pub amount: i8,
}

#[derive(Clone, Copy)]
pub struct CustomSpell {
    pub race_mask: u64,
    pub class_mask: u32,
    pub spell: u32,
}

#[derive(Clone, Copy)]
pub struct CastSpell {
    pub spell: CustomSpell,
    pub create_mode: i8,
}

#[derive(Clone, Copy)]
pub struct XpOverride {
    pub level: u8,
    pub experience: u32,
}

/// ObjectMgr::LoadSkillTiers reads all sixteen uint32 values, not uint16.
#[derive(Clone, Copy)]
pub struct SkillTierRow {
    pub id: u32,
    pub values: [u32; 16],
}

/// All queries succeed before any batch is returned. Empty optional tables
/// are real empty state; they never substitute for query/decode failures.
#[derive(Default)]
pub struct CreationWorldRows {
    pub definitions: Vec<StartDefinition>,
    pub item_overrides: Vec<ItemOverride>,
    pub custom_spells: Vec<CustomSpell>,
    pub cast_spells: Vec<CastSpell>,
    pub actions: Vec<StartAction>,
    pub race_stats: Vec<RaceStats>,
    pub class_stats: Vec<ClassLevelStats>,
    pub xp_overrides: Vec<XpOverride>,
    pub skill_tiers: Vec<SkillTierRow>,
}

/// One startup operation, not a query capability per table. SQL stays in the
/// adapter; the immutable source catalog stays in the application/data owner.
pub trait CreationWorldRepository: Send + Sync {
    fn load_item_addons(
        &self,
    ) -> PersistenceFutureLikeCpp<'_, Result<Vec<super::item_specs::ItemAddonRow>, LoadError>>;
    fn load_character_templates(
        &self,
    ) -> PersistenceFutureLikeCpp<'_, Result<templates::TemplateRows, LoadError>>;
    fn load_creation_world(
        &self,
    ) -> PersistenceFutureLikeCpp<'_, Result<CreationWorldRows, LoadError>>;
}
