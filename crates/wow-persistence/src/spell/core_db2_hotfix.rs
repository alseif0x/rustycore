//! SQLx-free Hotfix DB contract for the DB2 rows that hydrate core `SpellInfo`.

use crate::PersistenceFutureLikeCpp;

#[derive(Debug, Clone, PartialEq)]
pub enum SpellCoreDb2HotfixLoadOutcomeLikeCpp<T> {
    Loaded(Vec<T>),
    Failed { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellNameHotfixRowLikeCpp {
    pub id: u32,
    pub name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellCategoriesHotfixRowLikeCpp {
    pub id: u32,
    pub difficulty_id: u8,
    pub category: i16,
    pub defense_type: i8,
    pub dispel_type: i8,
    pub mechanic: i8,
    pub prevention_type: i8,
    pub start_recovery_category: i16,
    pub charge_category: i16,
    pub spell_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpellMiscHotfixRowLikeCpp {
    pub id: u32,
    pub attributes: [i32; 15],
    pub difficulty_id: u8,
    pub casting_time_index: u16,
    pub duration_index: u16,
    pub range_index: u16,
    pub school_mask: u8,
    pub speed: f32,
    pub launch_delay: f32,
    pub min_duration: f32,
    pub spell_icon_file_data_id: i32,
    pub active_icon_file_data_id: i32,
    pub content_tuning_id: i32,
    pub show_future_spell_player_condition_id: i32,
    pub spell_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpellEffectHotfixRowLikeCpp {
    pub id: u32,
    pub difficulty_id: i32,
    pub effect_index: i32,
    pub effect: u32,
    pub effect_amplitude: f32,
    pub effect_attributes: i32,
    pub effect_aura: i16,
    pub effect_aura_period: i32,
    pub effect_base_points: i32,
    pub effect_bonus_coefficient: f32,
    pub effect_chain_amplitude: f32,
    pub effect_chain_targets: i32,
    pub effect_die_sides: i32,
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
    pub effect_misc_value: [i32; 2],
    pub effect_radius_index: [u32; 2],
    pub effect_spell_class_mask: [u32; 4],
    pub implicit_target: [i16; 2],
    pub spell_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellShapeshiftHotfixRowLikeCpp {
    pub id: u32,
    pub spell_id: i32,
    pub stance_bar_order: i8,
    pub shapeshift_exclude: [i32; 2],
    pub shapeshift_mask: [i32; 2],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellInterruptsHotfixRowLikeCpp {
    pub id: u32,
    pub difficulty_id: u8,
    pub interrupt_flags: i16,
    pub aura_interrupt_flags: [i32; 2],
    pub channel_interrupt_flags: [i32; 2],
    pub spell_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellCastTimesHotfixRowLikeCpp {
    pub id: u32,
    pub base: i32,
    pub per_level: i16,
    pub minimum: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellCooldownsHotfixRowLikeCpp {
    pub id: u32,
    pub difficulty_id: u8,
    pub category_recovery_time: i32,
    pub recovery_time: i32,
    pub start_recovery_time: i32,
    pub spell_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellCastingRequirementsHotfixRowLikeCpp {
    pub id: u32,
    pub spell_id: i32,
    pub facing_caster_flags: u8,
    pub min_faction_id: u16,
    pub min_reputation: i32,
    pub required_areas_id: u16,
    pub required_aura_vision: u8,
    pub requires_spell_focus: u16,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpellPowerHotfixRowLikeCpp {
    pub id: u32,
    pub order_index: u8,
    pub mana_cost: i32,
    pub mana_cost_per_level: i32,
    pub mana_per_second: i32,
    pub power_display_id: u32,
    pub alt_power_bar_id: i32,
    pub power_cost_pct: f32,
    pub power_cost_max_pct: f32,
    pub power_pct_per_second: f32,
    pub power_type: i8,
    pub required_aura_spell_id: i32,
    pub optional_cost: u32,
    pub spell_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellPowerDifficultyHotfixRowLikeCpp {
    pub id: u32,
    pub difficulty_id: u8,
    pub order_index: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellAuraRestrictionsHotfixRowLikeCpp {
    pub id: u32,
    pub difficulty_id: u8,
    pub caster_aura_state: u8,
    pub target_aura_state: u8,
    pub exclude_caster_aura_state: u8,
    pub exclude_target_aura_state: u8,
    pub caster_aura_spell: i32,
    pub target_aura_spell: i32,
    pub exclude_caster_aura_spell: i32,
    pub exclude_target_aura_spell: i32,
    pub spell_id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellCategoryHotfixRowLikeCpp {
    pub id: u32,
    pub name: String,
    pub flags: i32,
    pub uses_per_week: u8,
    pub max_charges: i8,
    pub charge_recovery_time: i32,
    pub type_mask: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellDurationHotfixRowLikeCpp {
    pub id: u32,
    pub duration: i32,
    pub duration_per_level: u32,
    pub max_duration: i32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpellRadiusHotfixRowLikeCpp {
    pub id: u32,
    pub radius: f32,
    pub radius_per_level: f32,
    pub radius_min: f32,
    pub radius_max: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpellRangeHotfixRowLikeCpp {
    pub id: u32,
    pub display_name: String,
    pub display_name_short: String,
    pub flags: u8,
    pub range_min: [f32; 2],
    pub range_max: [f32; 2],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellEquippedItemsHotfixRowLikeCpp {
    pub id: u32,
    pub spell_id: i32,
    pub equipped_item_class: i8,
    pub equipped_item_inv_types: i32,
    pub equipped_item_subclass: i32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpellTargetRestrictionsHotfixRowLikeCpp {
    pub id: u32,
    pub difficulty_id: u8,
    pub cone_degrees: f32,
    pub max_targets: u8,
    pub max_target_level: u32,
    pub target_creature_type: i16,
    pub targets: i32,
    pub width: f32,
    pub spell_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpellXSpellVisualHotfixRowLikeCpp {
    pub id: u32,
    pub difficulty_id: u8,
    pub spell_visual_id: u32,
    pub probability: f32,
    pub flags: u8,
    pub priority: i32,
    pub spell_icon_file_id: i32,
    pub active_icon_file_id: i32,
    pub viewer_unit_condition_id: u16,
    pub viewer_player_condition_id: u32,
    pub caster_unit_condition_id: u16,
    pub caster_player_condition_id: u32,
    pub spell_id: u32,
}

/// Hotfix DB capability for the core DB2 contributors used to build the
/// represented `SpellInfo` authority. Every method returns official rows
/// followed by custom rows, matching C++ `DB2StorageBase::LoadFromDB`.
pub trait SpellCoreDb2HotfixPersistencePortLikeCpp: Send + Sync {
    fn load_spell_name_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<'_, SpellCoreDb2HotfixLoadOutcomeLikeCpp<SpellNameHotfixRowLikeCpp>>;

    fn load_spell_categories_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SpellCoreDb2HotfixLoadOutcomeLikeCpp<SpellCategoriesHotfixRowLikeCpp>,
    >;

    fn load_spell_misc_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<'_, SpellCoreDb2HotfixLoadOutcomeLikeCpp<SpellMiscHotfixRowLikeCpp>>;

    fn load_spell_effect_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SpellCoreDb2HotfixLoadOutcomeLikeCpp<SpellEffectHotfixRowLikeCpp>,
    >;

    fn load_spell_shapeshift_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SpellCoreDb2HotfixLoadOutcomeLikeCpp<SpellShapeshiftHotfixRowLikeCpp>,
    >;

    fn load_spell_interrupts_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SpellCoreDb2HotfixLoadOutcomeLikeCpp<SpellInterruptsHotfixRowLikeCpp>,
    >;

    fn load_spell_cast_times_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SpellCoreDb2HotfixLoadOutcomeLikeCpp<SpellCastTimesHotfixRowLikeCpp>,
    >;

    fn load_spell_cooldowns_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SpellCoreDb2HotfixLoadOutcomeLikeCpp<SpellCooldownsHotfixRowLikeCpp>,
    >;

    fn load_spell_casting_requirements_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SpellCoreDb2HotfixLoadOutcomeLikeCpp<SpellCastingRequirementsHotfixRowLikeCpp>,
    >;

    fn load_spell_power_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SpellCoreDb2HotfixLoadOutcomeLikeCpp<SpellPowerHotfixRowLikeCpp>,
    >;

    fn load_spell_power_difficulty_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SpellCoreDb2HotfixLoadOutcomeLikeCpp<SpellPowerDifficultyHotfixRowLikeCpp>,
    >;

    fn load_spell_aura_restrictions_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SpellCoreDb2HotfixLoadOutcomeLikeCpp<SpellAuraRestrictionsHotfixRowLikeCpp>,
    >;

    fn load_spell_category_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SpellCoreDb2HotfixLoadOutcomeLikeCpp<SpellCategoryHotfixRowLikeCpp>,
    >;

    fn load_spell_duration_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SpellCoreDb2HotfixLoadOutcomeLikeCpp<SpellDurationHotfixRowLikeCpp>,
    >;

    fn load_spell_radius_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SpellCoreDb2HotfixLoadOutcomeLikeCpp<SpellRadiusHotfixRowLikeCpp>,
    >;

    fn load_spell_range_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SpellCoreDb2HotfixLoadOutcomeLikeCpp<SpellRangeHotfixRowLikeCpp>,
    >;

    fn load_spell_equipped_items_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SpellCoreDb2HotfixLoadOutcomeLikeCpp<SpellEquippedItemsHotfixRowLikeCpp>,
    >;

    fn load_spell_target_restrictions_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SpellCoreDb2HotfixLoadOutcomeLikeCpp<SpellTargetRestrictionsHotfixRowLikeCpp>,
    >;

    fn load_spell_x_spell_visual_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SpellCoreDb2HotfixLoadOutcomeLikeCpp<SpellXSpellVisualHotfixRowLikeCpp>,
    >;
}
