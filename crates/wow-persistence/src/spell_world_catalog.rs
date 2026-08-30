//! SQLx-free startup source contract for foundational C++ `SpellMgr` World catalogs.

use crate::PersistenceFutureLikeCpp;

#[derive(Debug, Clone, PartialEq)]
pub enum SpellWorldCatalogLoadOutcomeLikeCpp<T> {
    Loaded(Vec<T>),
    Failed { reason: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellRequiredPersistenceRowLikeCpp {
    pub spell_id: u32,
    pub req_spell: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpellThreatPersistenceRowLikeCpp {
    pub spell_id: u32,
    pub flat_mod: i32,
    pub pct_mod: f32,
    pub ap_pct_mod: f32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellLinkedPersistenceRowLikeCpp {
    pub spell_trigger: i32,
    pub spell_effect: i32,
    pub link_type: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellTotemModelPersistenceRowLikeCpp {
    pub spell_id: u32,
    pub race_id: u8,
    pub display_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellPetAuraPersistenceRowLikeCpp {
    pub spell_id: u32,
    pub effect_index: u8,
    pub pet_entry: u32,
    pub aura_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellAreaPersistenceRowLikeCpp {
    pub spell_id: u32,
    pub area_id: u32,
    pub quest_start: u32,
    pub quest_start_status: u32,
    pub quest_end_status: u32,
    pub quest_end: u32,
    pub aura_spell: i32,
    pub race_mask: u64,
    pub gender: u8,
    pub flags: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellGroupPersistenceRowLikeCpp {
    pub group_id: u32,
    pub spell_id: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellGroupStackRulePersistenceRowLikeCpp {
    pub group_id: u32,
    pub stack_rule: u8,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpellProcPersistenceRowLikeCpp {
    pub spell_id: i32,
    pub school_mask: u8,
    pub spell_family_name: u16,
    pub spell_family_mask: [u32; 4],
    pub proc_flags: [u32; 2],
    pub spell_type_mask: u32,
    pub spell_phase_mask: u32,
    pub hit_mask: u32,
    pub attributes_mask: u32,
    pub disable_effects_mask: u32,
    pub procs_per_minute: f32,
    pub chance: f32,
    pub cooldown_ms: u32,
    pub charges: u8,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpellTargetPositionPersistenceRowLikeCpp {
    pub spell_id: u32,
    pub effect_index: u32,
    pub target_map_id: u16,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub orientation: Option<f32>,
}

/// Independent World-table reads used by foundational C++ `SpellMgr`
/// catalogs. The concrete adapter owns statement identity and tolerant row
/// decoding; `wow-data` retains validation and immutable catalog ownership.
pub trait SpellWorldCatalogPersistencePortLikeCpp: Send + Sync {
    fn load_spell_area_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SpellWorldCatalogLoadOutcomeLikeCpp<SpellAreaPersistenceRowLikeCpp>,
    >;

    fn load_spell_target_position_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SpellWorldCatalogLoadOutcomeLikeCpp<SpellTargetPositionPersistenceRowLikeCpp>,
    >;

    fn load_spell_proc_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SpellWorldCatalogLoadOutcomeLikeCpp<SpellProcPersistenceRowLikeCpp>,
    >;

    fn load_spell_required_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SpellWorldCatalogLoadOutcomeLikeCpp<SpellRequiredPersistenceRowLikeCpp>,
    >;

    fn load_spell_group_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SpellWorldCatalogLoadOutcomeLikeCpp<SpellGroupPersistenceRowLikeCpp>,
    >;

    fn load_spell_group_stack_rule_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SpellWorldCatalogLoadOutcomeLikeCpp<SpellGroupStackRulePersistenceRowLikeCpp>,
    >;

    fn load_spell_threat_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SpellWorldCatalogLoadOutcomeLikeCpp<SpellThreatPersistenceRowLikeCpp>,
    >;

    fn load_spell_linked_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SpellWorldCatalogLoadOutcomeLikeCpp<SpellLinkedPersistenceRowLikeCpp>,
    >;

    fn load_spell_totem_model_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SpellWorldCatalogLoadOutcomeLikeCpp<SpellTotemModelPersistenceRowLikeCpp>,
    >;

    fn load_spell_pet_aura_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SpellWorldCatalogLoadOutcomeLikeCpp<SpellPetAuraPersistenceRowLikeCpp>,
    >;
}
