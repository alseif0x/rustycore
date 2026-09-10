//! SQLx-free source contracts for effective spell-acquisition startup.

use crate::PersistenceFutureLikeCpp;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellAcquisitionHotfixTablePersistenceLikeCpp {
    SpellEffect,
    SpellLearnSpell,
    SpellMisc,
    SpellLevels,
    Talent,
    SummonProperties,
    BattlePetSpecies,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellEffectHotfixPersistenceRowLikeCpp {
    pub record_id: Option<i64>,
    pub difficulty_id: Option<i64>,
    pub effect_index: Option<i64>,
    pub effect: Option<i64>,
    pub effect_base_points: Option<i64>,
    pub effect_die_sides: Option<i64>,
    pub effect_trigger_spell: Option<i64>,
    pub effect_misc_value: [Option<i64>; 2],
    pub implicit_target: [Option<i64>; 2],
    pub coefficient_bits: Option<u32>,
    pub variance_bits: Option<u32>,
    pub spell_id: Option<i64>,
    pub effect_chain_targets: Option<i64>,
    pub effect_points_per_resource_bits: Option<u32>,
    pub effect_real_points_per_level_bits: Option<u32>,
    pub effect_item_type: Option<i64>,
    pub effect_aura: Option<i64>,
    pub effect_mechanic: Option<i64>,
    pub effect_attributes: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellLearnSpellHotfixPersistenceRowLikeCpp {
    pub record_id: Option<i64>,
    pub spell_id: Option<i64>,
    pub learn_spell_id: Option<i64>,
    pub overrides_spell_id: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellMiscHotfixPersistenceRowLikeCpp {
    pub record_id: Option<i64>,
    pub attributes: [Option<i64>; 2],
    pub difficulty_id: Option<i64>,
    pub show_future_spell_player_condition_id: Option<i64>,
    pub spell_id: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellLevelsHotfixPersistenceRowLikeCpp {
    pub record_id: Option<i64>,
    pub difficulty_id: Option<i64>,
    pub base_level: Option<i64>,
    pub spell_level: Option<i64>,
    pub spell_id: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TalentHotfixPersistenceRowLikeCpp {
    pub record_id: Option<i64>,
    pub spell_rank: [Option<i64>; 9],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SummonPropertiesHotfixPersistenceRowLikeCpp {
    pub record_id: Option<i64>,
    pub slot: Option<i64>,
    pub flags_1: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BattlePetSpeciesHotfixPersistenceRowLikeCpp {
    pub record_id: Option<i64>,
    pub creature_id: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpellAcquisitionHotfixPersistenceRowLikeCpp {
    SpellEffect(SpellEffectHotfixPersistenceRowLikeCpp),
    SpellLearnSpell(SpellLearnSpellHotfixPersistenceRowLikeCpp),
    SpellMisc(SpellMiscHotfixPersistenceRowLikeCpp),
    SpellLevels(SpellLevelsHotfixPersistenceRowLikeCpp),
    Talent(TalentHotfixPersistenceRowLikeCpp),
    SummonProperties(SummonPropertiesHotfixPersistenceRowLikeCpp),
    BattlePetSpecies(BattlePetSpeciesHotfixPersistenceRowLikeCpp),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ServersideSpellEffectPersistenceRowLikeCpp {
    pub spell_id: u32,
    pub effect_index: i32,
    pub difficulty_id: u32,
    pub effect: i32,
    pub effect_aura: i32,
    pub effect_amplitude: f32,
    pub effect_attributes: i32,
    pub effect_aura_period: i32,
    pub effect_bonus_coefficient: f32,
    pub effect_chain_amplitude: f32,
    pub effect_chain_targets: i32,
    pub effect_item_type: i32,
    pub effect_mechanic: i32,
    pub effect_points_per_resource: f32,
    pub effect_pos_facing: f32,
    pub effect_real_points_per_level: f32,
    pub effect_trigger_spell: i32,
    pub bonus_coefficient_from_ap: f32,
    pub pvp_multiplier: f32,
    pub coefficient: f32,
    pub variance: f32,
    pub resource_coefficient: f32,
    pub group_size_base_points_coefficient: f32,
    pub effect_base_points: f32,
    pub effect_misc_value: [i32; 2],
    pub effect_radius_index: [u32; 2],
    pub effect_spell_class_mask: [i32; 4],
    pub implicit_target: [i32; 2],
}

#[derive(Debug, Clone, PartialEq)]
pub struct ServersideSpellPersistenceRowLikeCpp {
    pub spell_id: u32,
    pub difficulty_id: u32,
    pub category_id: u32,
    pub dispel: u32,
    pub mechanic: u32,
    pub attributes: u32,
    pub attributes_ex: [u32; 14],
    pub stances: u64,
    pub stances_not: u64,
    pub targets: u32,
    pub target_creature_type: u32,
    pub requires_spell_focus: u32,
    pub facing_caster_flags: u32,
    pub caster_aura_state: u32,
    pub target_aura_state: u32,
    pub exclude_caster_aura_state: u32,
    pub exclude_target_aura_state: u32,
    pub caster_aura_spell: u32,
    pub target_aura_spell: u32,
    pub exclude_caster_aura_spell: u32,
    pub exclude_target_aura_spell: u32,
    pub caster_aura_type: i32,
    pub target_aura_type: i32,
    pub exclude_caster_aura_type: i32,
    pub exclude_target_aura_type: i32,
    pub casting_time_index: u32,
    pub recovery_time: u32,
    pub category_recovery_time: u32,
    pub start_recovery_category: u32,
    pub start_recovery_time: u32,
    pub interrupt_flags: u32,
    pub aura_interrupt_flags: [u32; 2],
    pub channel_interrupt_flags: [u32; 2],
    pub proc_flags: [u32; 2],
    pub proc_chance: u32,
    pub proc_charges: u32,
    pub proc_cooldown: u32,
    pub proc_base_ppm: f32,
    pub max_level: u32,
    pub base_level: u32,
    pub spell_level: u32,
    pub duration_index: u32,
    pub range_index: u32,
    pub speed: f32,
    pub launch_delay: f32,
    pub stack_amount: u32,
    pub equipped_item_class: i32,
    pub equipped_item_sub_class_mask: i32,
    pub equipped_item_inventory_type_mask: i32,
    pub content_tuning_id: u32,
    pub spell_name: String,
    pub cone_angle: f32,
    pub cone_width: f32,
    pub max_target_level: u32,
    pub max_affected_targets: u32,
    pub spell_family_name: u32,
    pub spell_family_flags: [u32; 4],
    pub dmg_class: u32,
    pub prevention_type: u32,
    pub area_group_id: i32,
    pub school_mask: u32,
    pub charge_category_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellCustomAttributePersistenceRowLikeCpp {
    pub spell_id: u32,
    pub attributes: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellLearnSpellWorldPersistenceRowLikeCpp {
    pub entry: u32,
    pub spell_id: u32,
    pub active: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellReagentsPersistenceRowLikeCpp {
    pub id: u32,
    pub spell_id: i32,
    pub reagent: [i32; 8],
    pub reagent_count: [i16; 8],
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TrainerSpellAuditPersistenceCatalogLikeCpp {
    pub script_binding_ids: Vec<i32>,
    pub legacy_script_ids: Vec<u32>,
    pub condition_spell_ids: Vec<i32>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SpellAcquisitionStartupLoadOutcomeLikeCpp<T> {
    Loaded(T),
    Failed { reason: String },
}

/// Independent Hotfix and World startup reads. Each method returns one complete
/// batch; the caller retains C++ application and publication order.
pub trait SpellAcquisitionStartupPersistencePortLikeCpp: Send + Sync {
    fn load_hotfix_overlay_like_cpp(
        &self,
        table: SpellAcquisitionHotfixTablePersistenceLikeCpp,
        official: bool,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SpellAcquisitionStartupLoadOutcomeLikeCpp<Vec<SpellAcquisitionHotfixPersistenceRowLikeCpp>>,
    >;

    fn load_serverside_spell_effects_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SpellAcquisitionStartupLoadOutcomeLikeCpp<Vec<ServersideSpellEffectPersistenceRowLikeCpp>>,
    >;

    fn load_serverside_spells_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SpellAcquisitionStartupLoadOutcomeLikeCpp<Vec<ServersideSpellPersistenceRowLikeCpp>>,
    >;

    fn load_spell_custom_attributes_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SpellAcquisitionStartupLoadOutcomeLikeCpp<Vec<SpellCustomAttributePersistenceRowLikeCpp>>,
    >;

    fn load_spell_learn_spells_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SpellAcquisitionStartupLoadOutcomeLikeCpp<Vec<SpellLearnSpellWorldPersistenceRowLikeCpp>>,
    >;

    fn load_trainer_spell_audit_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SpellAcquisitionStartupLoadOutcomeLikeCpp<TrainerSpellAuditPersistenceCatalogLikeCpp>,
    >;

    fn load_spell_reagents_overlay_like_cpp(
        &self,
        official: bool,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SpellAcquisitionStartupLoadOutcomeLikeCpp<Vec<SpellReagentsPersistenceRowLikeCpp>>,
    >;
}
