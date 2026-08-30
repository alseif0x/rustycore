//! Composition boundary for the Hotfix DB2 rows that hydrate core `SpellInfo`.

use anyhow::{Result, anyhow};
use wow_data::{
    Db2HotfixRemovalStoreLikeCpp, EffectiveCoreSpellDb2StoresLikeCpp, SpellAuraRestrictionsEntry,
    SpellAuraRestrictionsStore, SpellCastTimesEntry, SpellCastTimesStore,
    SpellCastingRequirementsEntry, SpellCastingRequirementsStore, SpellCategoriesEntry,
    SpellCategoriesStore, SpellCategoryEntry, SpellCategoryStore, SpellCooldownsEntry,
    SpellCooldownsStore, SpellDurationEntry, SpellDurationStore, SpellEffectDb2Entry,
    SpellEffectDb2Store, SpellEquippedItemsEntry, SpellEquippedItemsStore, SpellInterruptsEntry,
    SpellInterruptsStore, SpellMiscEntry, SpellMiscStore, SpellNameEffectiveLoadReportLikeCpp,
    SpellNameEntry, SpellNameStore, SpellPowerDifficultyEntry, SpellPowerDifficultyStore,
    SpellPowerEntry, SpellPowerStore, SpellRadiusEntry, SpellRadiusStore, SpellRangeEntry,
    SpellRangeStore, SpellShapeshiftEntry, SpellShapeshiftStore, SpellStore,
    SpellTargetRestrictionsEntry, SpellTargetRestrictionsStore, SpellXSpellVisualEntry,
    SpellXSpellVisualStore,
};
use wow_persistence::{
    SpellAuraRestrictionsHotfixRowLikeCpp, SpellCastTimesHotfixRowLikeCpp,
    SpellCastingRequirementsHotfixRowLikeCpp, SpellCategoriesHotfixRowLikeCpp,
    SpellCategoryHotfixRowLikeCpp, SpellCooldownsHotfixRowLikeCpp,
    SpellCoreDb2HotfixLoadOutcomeLikeCpp, SpellCoreDb2HotfixPersistencePortLikeCpp,
    SpellDurationHotfixRowLikeCpp, SpellEffectHotfixRowLikeCpp, SpellEquippedItemsHotfixRowLikeCpp,
    SpellInterruptsHotfixRowLikeCpp, SpellMiscHotfixRowLikeCpp, SpellNameHotfixRowLikeCpp,
    SpellPowerDifficultyHotfixRowLikeCpp, SpellPowerHotfixRowLikeCpp, SpellRadiusHotfixRowLikeCpp,
    SpellRangeHotfixRowLikeCpp, SpellShapeshiftHotfixRowLikeCpp,
    SpellTargetRestrictionsHotfixRowLikeCpp, SpellXSpellVisualHotfixRowLikeCpp,
};

fn loaded_rows_like_cpp<T>(outcome: SpellCoreDb2HotfixLoadOutcomeLikeCpp<T>) -> Result<Vec<T>> {
    match outcome {
        SpellCoreDb2HotfixLoadOutcomeLikeCpp::Loaded(rows) => Ok(rows),
        SpellCoreDb2HotfixLoadOutcomeLikeCpp::Failed { reason } => Err(anyhow!(reason)),
    }
}

fn spell_name_entry_like_cpp(row: SpellNameHotfixRowLikeCpp) -> SpellNameEntry {
    SpellNameEntry {
        id: row.id,
        name: row.name,
    }
}

fn spell_categories_entry_like_cpp(row: SpellCategoriesHotfixRowLikeCpp) -> SpellCategoriesEntry {
    SpellCategoriesEntry {
        id: row.id,
        difficulty_id: row.difficulty_id,
        category: row.category,
        defense_type: row.defense_type,
        dispel_type: row.dispel_type,
        mechanic: row.mechanic,
        prevention_type: row.prevention_type,
        start_recovery_category: row.start_recovery_category,
        charge_category: row.charge_category,
        spell_id: row.spell_id,
    }
}

fn spell_misc_entry_like_cpp(row: SpellMiscHotfixRowLikeCpp) -> SpellMiscEntry {
    SpellMiscEntry {
        id: row.id,
        attributes: row.attributes,
        difficulty_id: row.difficulty_id,
        casting_time_index: row.casting_time_index,
        duration_index: row.duration_index,
        range_index: row.range_index,
        school_mask: row.school_mask,
        speed: row.speed,
        launch_delay: row.launch_delay,
        min_duration: row.min_duration,
        spell_icon_file_data_id: row.spell_icon_file_data_id,
        active_icon_file_data_id: row.active_icon_file_data_id,
        content_tuning_id: row.content_tuning_id,
        show_future_spell_player_condition_id: row.show_future_spell_player_condition_id,
        spell_id: row.spell_id,
    }
}

fn spell_effect_entry_like_cpp(row: SpellEffectHotfixRowLikeCpp) -> SpellEffectDb2Entry {
    SpellEffectDb2Entry {
        id: row.id,
        difficulty_id: row.difficulty_id,
        effect_index: row.effect_index,
        effect: row.effect,
        effect_amplitude: row.effect_amplitude,
        effect_attributes: row.effect_attributes,
        effect_aura: row.effect_aura,
        effect_aura_period: row.effect_aura_period,
        effect_base_points: row.effect_base_points,
        effect_bonus_coefficient: row.effect_bonus_coefficient,
        effect_chain_amplitude: row.effect_chain_amplitude,
        effect_chain_targets: row.effect_chain_targets,
        effect_die_sides: row.effect_die_sides,
        effect_item_type: row.effect_item_type,
        effect_mechanic: row.effect_mechanic,
        effect_points_per_resource: row.effect_points_per_resource,
        effect_pos_facing: row.effect_pos_facing,
        effect_real_points_per_level: row.effect_real_points_per_level,
        effect_trigger_spell: row.effect_trigger_spell,
        bonus_coefficient_from_ap: row.bonus_coefficient_from_ap,
        pvp_multiplier: row.pvp_multiplier,
        coefficient: row.coefficient,
        variance: row.variance,
        resource_coefficient: row.resource_coefficient,
        group_size_base_points_coefficient: row.group_size_base_points_coefficient,
        effect_misc_value: row.effect_misc_value,
        effect_radius_index: row.effect_radius_index,
        effect_spell_class_mask: row.effect_spell_class_mask,
        implicit_target: row.implicit_target,
        spell_id: row.spell_id,
    }
}

fn spell_shapeshift_entry_like_cpp(row: SpellShapeshiftHotfixRowLikeCpp) -> SpellShapeshiftEntry {
    SpellShapeshiftEntry {
        id: row.id,
        spell_id: row.spell_id,
        stance_bar_order: row.stance_bar_order,
        shapeshift_exclude: row.shapeshift_exclude,
        shapeshift_mask: row.shapeshift_mask,
    }
}

fn spell_interrupts_entry_like_cpp(row: SpellInterruptsHotfixRowLikeCpp) -> SpellInterruptsEntry {
    SpellInterruptsEntry {
        id: row.id,
        difficulty_id: row.difficulty_id,
        interrupt_flags: row.interrupt_flags,
        aura_interrupt_flags: row.aura_interrupt_flags,
        channel_interrupt_flags: row.channel_interrupt_flags,
        spell_id: row.spell_id,
    }
}

fn spell_cast_times_entry_like_cpp(row: SpellCastTimesHotfixRowLikeCpp) -> SpellCastTimesEntry {
    SpellCastTimesEntry {
        id: row.id,
        base: row.base,
        per_level: row.per_level,
        minimum: row.minimum,
    }
}

fn spell_cooldowns_entry_like_cpp(row: SpellCooldownsHotfixRowLikeCpp) -> SpellCooldownsEntry {
    SpellCooldownsEntry {
        id: row.id,
        difficulty_id: row.difficulty_id,
        category_recovery_time: row.category_recovery_time,
        recovery_time: row.recovery_time,
        start_recovery_time: row.start_recovery_time,
        spell_id: row.spell_id,
    }
}

fn spell_casting_requirements_entry_like_cpp(
    row: SpellCastingRequirementsHotfixRowLikeCpp,
) -> SpellCastingRequirementsEntry {
    SpellCastingRequirementsEntry {
        id: row.id,
        spell_id: row.spell_id,
        facing_caster_flags: row.facing_caster_flags,
        min_faction_id: row.min_faction_id,
        min_reputation: row.min_reputation,
        required_areas_id: row.required_areas_id,
        required_aura_vision: row.required_aura_vision,
        requires_spell_focus: row.requires_spell_focus,
    }
}

fn spell_power_entry_like_cpp(row: SpellPowerHotfixRowLikeCpp) -> SpellPowerEntry {
    SpellPowerEntry {
        id: row.id,
        order_index: row.order_index,
        mana_cost: row.mana_cost,
        mana_cost_per_level: row.mana_cost_per_level,
        mana_per_second: row.mana_per_second,
        power_display_id: row.power_display_id,
        alt_power_bar_id: row.alt_power_bar_id,
        power_cost_pct: row.power_cost_pct,
        power_cost_max_pct: row.power_cost_max_pct,
        power_pct_per_second: row.power_pct_per_second,
        power_type: row.power_type,
        required_aura_spell_id: row.required_aura_spell_id,
        optional_cost: row.optional_cost,
        spell_id: row.spell_id,
    }
}

fn spell_power_difficulty_entry_like_cpp(
    row: SpellPowerDifficultyHotfixRowLikeCpp,
) -> SpellPowerDifficultyEntry {
    SpellPowerDifficultyEntry {
        id: row.id,
        difficulty_id: row.difficulty_id,
        order_index: row.order_index,
    }
}

fn spell_aura_restrictions_entry_like_cpp(
    row: SpellAuraRestrictionsHotfixRowLikeCpp,
) -> SpellAuraRestrictionsEntry {
    SpellAuraRestrictionsEntry {
        id: row.id,
        difficulty_id: row.difficulty_id,
        caster_aura_state: row.caster_aura_state,
        target_aura_state: row.target_aura_state,
        exclude_caster_aura_state: row.exclude_caster_aura_state,
        exclude_target_aura_state: row.exclude_target_aura_state,
        caster_aura_spell: row.caster_aura_spell,
        target_aura_spell: row.target_aura_spell,
        exclude_caster_aura_spell: row.exclude_caster_aura_spell,
        exclude_target_aura_spell: row.exclude_target_aura_spell,
        spell_id: row.spell_id,
    }
}

fn spell_category_entry_like_cpp(row: SpellCategoryHotfixRowLikeCpp) -> SpellCategoryEntry {
    SpellCategoryEntry {
        id: row.id,
        name: row.name,
        flags: row.flags,
        uses_per_week: row.uses_per_week,
        max_charges: row.max_charges,
        charge_recovery_time: row.charge_recovery_time,
        type_mask: row.type_mask,
    }
}

fn spell_duration_entry_like_cpp(row: SpellDurationHotfixRowLikeCpp) -> SpellDurationEntry {
    SpellDurationEntry {
        id: row.id,
        duration: row.duration,
        duration_per_level: row.duration_per_level,
        max_duration: row.max_duration,
    }
}

fn spell_radius_entry_like_cpp(row: SpellRadiusHotfixRowLikeCpp) -> SpellRadiusEntry {
    SpellRadiusEntry {
        id: row.id,
        radius: row.radius,
        radius_per_level: row.radius_per_level,
        radius_min: row.radius_min,
        radius_max: row.radius_max,
    }
}

fn spell_range_entry_like_cpp(row: SpellRangeHotfixRowLikeCpp) -> SpellRangeEntry {
    SpellRangeEntry {
        id: row.id,
        display_name: row.display_name,
        display_name_short: row.display_name_short,
        flags: row.flags,
        range_min: row.range_min,
        range_max: row.range_max,
    }
}

fn spell_equipped_items_entry_like_cpp(
    row: SpellEquippedItemsHotfixRowLikeCpp,
) -> SpellEquippedItemsEntry {
    SpellEquippedItemsEntry {
        id: row.id,
        spell_id: row.spell_id,
        equipped_item_class: row.equipped_item_class,
        equipped_item_inv_types: row.equipped_item_inv_types,
        equipped_item_subclass: row.equipped_item_subclass,
    }
}

fn spell_target_restrictions_entry_like_cpp(
    row: SpellTargetRestrictionsHotfixRowLikeCpp,
) -> SpellTargetRestrictionsEntry {
    SpellTargetRestrictionsEntry {
        id: row.id,
        difficulty_id: row.difficulty_id,
        cone_degrees: row.cone_degrees,
        max_targets: row.max_targets,
        max_target_level: row.max_target_level,
        target_creature_type: row.target_creature_type,
        targets: row.targets,
        width: row.width,
        spell_id: row.spell_id,
    }
}

fn spell_x_spell_visual_entry_like_cpp(
    row: SpellXSpellVisualHotfixRowLikeCpp,
) -> SpellXSpellVisualEntry {
    SpellXSpellVisualEntry {
        id: row.id,
        difficulty_id: row.difficulty_id,
        spell_visual_id: row.spell_visual_id,
        probability: row.probability,
        flags: row.flags,
        priority: row.priority,
        spell_icon_file_id: row.spell_icon_file_id,
        active_icon_file_id: row.active_icon_file_id,
        viewer_unit_condition_id: row.viewer_unit_condition_id,
        viewer_player_condition_id: row.viewer_player_condition_id,
        caster_unit_condition_id: row.caster_unit_condition_id,
        caster_player_condition_id: row.caster_player_condition_id,
        spell_id: row.spell_id,
    }
}

pub(super) async fn load_spell_name_store_like_cpp(
    data_dir: &str,
    locale: &str,
    persistence: &dyn SpellCoreDb2HotfixPersistencePortLikeCpp,
    removals: &Db2HotfixRemovalStoreLikeCpp,
) -> Result<(SpellNameStore, SpellNameEffectiveLoadReportLikeCpp)> {
    let rows = loaded_rows_like_cpp(persistence.load_spell_name_rows_like_cpp().await)?;
    SpellNameStore::load_effective_from_hotfix_rows_like_cpp(
        data_dir,
        locale,
        rows.into_iter().map(spell_name_entry_like_cpp).collect(),
        removals,
    )
}

pub(super) async fn load_spell_store_like_cpp(
    data_dir: &str,
    locale: &str,
    seed: SpellStore,
    persistence: &dyn SpellCoreDb2HotfixPersistencePortLikeCpp,
    removals: &Db2HotfixRemovalStoreLikeCpp,
) -> Result<SpellStore> {
    let categories = loaded_rows_like_cpp(persistence.load_spell_categories_rows_like_cpp().await)?;
    let categories = SpellCategoriesStore::load_effective_from_hotfix_rows_like_cpp(
        data_dir,
        locale,
        categories.into_iter().map(spell_categories_entry_like_cpp),
        removals,
    )?;
    let misc = loaded_rows_like_cpp(persistence.load_spell_misc_rows_like_cpp().await)?;
    let misc = SpellMiscStore::load_effective_from_hotfix_rows_like_cpp(
        data_dir,
        locale,
        misc.into_iter().map(spell_misc_entry_like_cpp),
        removals,
    )?;
    let effect = loaded_rows_like_cpp(persistence.load_spell_effect_rows_like_cpp().await)?;
    let effect = SpellEffectDb2Store::load_effective_from_hotfix_rows_like_cpp(
        data_dir,
        locale,
        effect.into_iter().map(spell_effect_entry_like_cpp),
        removals,
    )?;
    let shapeshift = loaded_rows_like_cpp(persistence.load_spell_shapeshift_rows_like_cpp().await)?;
    let shapeshift = SpellShapeshiftStore::load_effective_from_hotfix_rows_like_cpp(
        data_dir,
        locale,
        shapeshift.into_iter().map(spell_shapeshift_entry_like_cpp),
        removals,
    )?;
    let interrupts = loaded_rows_like_cpp(persistence.load_spell_interrupts_rows_like_cpp().await)?;
    let interrupts = SpellInterruptsStore::load_effective_from_hotfix_rows_like_cpp(
        data_dir,
        locale,
        interrupts.into_iter().map(spell_interrupts_entry_like_cpp),
        removals,
    )?;
    let cast_times = loaded_rows_like_cpp(persistence.load_spell_cast_times_rows_like_cpp().await)?;
    let cast_times = SpellCastTimesStore::load_effective_from_hotfix_rows_like_cpp(
        data_dir,
        locale,
        cast_times.into_iter().map(spell_cast_times_entry_like_cpp),
        removals,
    )?;
    let cooldowns = loaded_rows_like_cpp(persistence.load_spell_cooldowns_rows_like_cpp().await)?;
    let cooldowns = SpellCooldownsStore::load_effective_from_hotfix_rows_like_cpp(
        data_dir,
        locale,
        cooldowns.into_iter().map(spell_cooldowns_entry_like_cpp),
        removals,
    )?;
    let casting_requirements = loaded_rows_like_cpp(
        persistence
            .load_spell_casting_requirements_rows_like_cpp()
            .await,
    )?;
    let casting_requirements =
        SpellCastingRequirementsStore::load_effective_from_hotfix_rows_like_cpp(
            data_dir,
            locale,
            casting_requirements
                .into_iter()
                .map(spell_casting_requirements_entry_like_cpp),
            removals,
        )?;
    let power = loaded_rows_like_cpp(persistence.load_spell_power_rows_like_cpp().await)?;
    let power = SpellPowerStore::load_effective_from_hotfix_rows_like_cpp(
        data_dir,
        locale,
        power.into_iter().map(spell_power_entry_like_cpp),
        removals,
    )?;
    let power_difficulty = loaded_rows_like_cpp(
        persistence
            .load_spell_power_difficulty_rows_like_cpp()
            .await,
    )?;
    let power_difficulty = SpellPowerDifficultyStore::load_effective_from_hotfix_rows_like_cpp(
        data_dir,
        locale,
        power_difficulty
            .into_iter()
            .map(spell_power_difficulty_entry_like_cpp),
        removals,
    )?;

    Ok(
        seed.hydrate_effective_core_db2_like_cpp(EffectiveCoreSpellDb2StoresLikeCpp::new(
            categories,
            misc,
            effect,
            shapeshift,
            interrupts,
            cast_times,
            cooldowns,
            casting_requirements,
            power,
            power_difficulty,
        )),
    )
}

pub(super) async fn load_spell_casting_requirements_store_like_cpp(
    data_dir: &str,
    locale: &str,
    persistence: &dyn SpellCoreDb2HotfixPersistencePortLikeCpp,
    removals: &Db2HotfixRemovalStoreLikeCpp,
) -> Result<SpellCastingRequirementsStore> {
    let rows = loaded_rows_like_cpp(
        persistence
            .load_spell_casting_requirements_rows_like_cpp()
            .await,
    )?;
    SpellCastingRequirementsStore::load_effective_from_hotfix_rows_like_cpp(
        data_dir,
        locale,
        rows.into_iter()
            .map(spell_casting_requirements_entry_like_cpp),
        removals,
    )
}

pub(super) async fn load_spell_misc_store_like_cpp(
    data_dir: &str,
    locale: &str,
    persistence: &dyn SpellCoreDb2HotfixPersistencePortLikeCpp,
    removals: &Db2HotfixRemovalStoreLikeCpp,
) -> Result<SpellMiscStore> {
    let rows = loaded_rows_like_cpp(persistence.load_spell_misc_rows_like_cpp().await)?;
    SpellMiscStore::load_effective_from_hotfix_rows_like_cpp(
        data_dir,
        locale,
        rows.into_iter().map(spell_misc_entry_like_cpp),
        removals,
    )
}

pub(super) async fn load_spell_cooldowns_store_like_cpp(
    data_dir: &str,
    locale: &str,
    persistence: &dyn SpellCoreDb2HotfixPersistencePortLikeCpp,
    removals: &Db2HotfixRemovalStoreLikeCpp,
) -> Result<SpellCooldownsStore> {
    let rows = loaded_rows_like_cpp(persistence.load_spell_cooldowns_rows_like_cpp().await)?;
    SpellCooldownsStore::load_effective_from_hotfix_rows_like_cpp(
        data_dir,
        locale,
        rows.into_iter().map(spell_cooldowns_entry_like_cpp),
        removals,
    )
}

pub(super) async fn load_spell_aura_restrictions_store_like_cpp(
    data_dir: &str,
    locale: &str,
    persistence: &dyn SpellCoreDb2HotfixPersistencePortLikeCpp,
    removals: &Db2HotfixRemovalStoreLikeCpp,
) -> Result<SpellAuraRestrictionsStore> {
    let rows = loaded_rows_like_cpp(
        persistence
            .load_spell_aura_restrictions_rows_like_cpp()
            .await,
    )?;
    SpellAuraRestrictionsStore::load_effective_from_hotfix_rows_like_cpp(
        data_dir,
        locale,
        rows.into_iter().map(spell_aura_restrictions_entry_like_cpp),
        removals,
    )
}

pub(super) async fn load_spell_category_store_like_cpp(
    data_dir: &str,
    locale: &str,
    persistence: &dyn SpellCoreDb2HotfixPersistencePortLikeCpp,
    removals: &Db2HotfixRemovalStoreLikeCpp,
) -> Result<SpellCategoryStore> {
    let rows = loaded_rows_like_cpp(persistence.load_spell_category_rows_like_cpp().await)?;
    SpellCategoryStore::load_effective_from_hotfix_rows_like_cpp(
        data_dir,
        locale,
        rows.into_iter().map(spell_category_entry_like_cpp),
        removals,
    )
}

pub(super) async fn load_spell_duration_store_like_cpp(
    data_dir: &str,
    locale: &str,
    persistence: &dyn SpellCoreDb2HotfixPersistencePortLikeCpp,
    removals: &Db2HotfixRemovalStoreLikeCpp,
) -> Result<SpellDurationStore> {
    let rows = loaded_rows_like_cpp(persistence.load_spell_duration_rows_like_cpp().await)?;
    SpellDurationStore::load_effective_from_hotfix_rows_like_cpp(
        data_dir,
        locale,
        rows.into_iter().map(spell_duration_entry_like_cpp),
        removals,
    )
}

pub(super) async fn load_spell_radius_store_like_cpp(
    data_dir: &str,
    locale: &str,
    persistence: &dyn SpellCoreDb2HotfixPersistencePortLikeCpp,
    removals: &Db2HotfixRemovalStoreLikeCpp,
) -> Result<SpellRadiusStore> {
    let rows = loaded_rows_like_cpp(persistence.load_spell_radius_rows_like_cpp().await)?;
    SpellRadiusStore::load_effective_from_hotfix_rows_like_cpp(
        data_dir,
        locale,
        rows.into_iter().map(spell_radius_entry_like_cpp),
        removals,
    )
}

pub(super) async fn load_spell_range_store_like_cpp(
    data_dir: &str,
    locale: &str,
    persistence: &dyn SpellCoreDb2HotfixPersistencePortLikeCpp,
    removals: &Db2HotfixRemovalStoreLikeCpp,
) -> Result<SpellRangeStore> {
    let rows = loaded_rows_like_cpp(persistence.load_spell_range_rows_like_cpp().await)?;
    SpellRangeStore::load_effective_from_hotfix_rows_like_cpp(
        data_dir,
        locale,
        rows.into_iter().map(spell_range_entry_like_cpp),
        removals,
    )
}

pub(super) async fn load_spell_equipped_items_store_like_cpp(
    data_dir: &str,
    locale: &str,
    persistence: &dyn SpellCoreDb2HotfixPersistencePortLikeCpp,
    removals: &Db2HotfixRemovalStoreLikeCpp,
) -> Result<SpellEquippedItemsStore> {
    let rows = loaded_rows_like_cpp(persistence.load_spell_equipped_items_rows_like_cpp().await)?;
    SpellEquippedItemsStore::load_effective_from_hotfix_rows_like_cpp(
        data_dir,
        locale,
        rows.into_iter().map(spell_equipped_items_entry_like_cpp),
        removals,
    )
}

pub(super) async fn load_spell_target_restrictions_store_like_cpp(
    data_dir: &str,
    locale: &str,
    persistence: &dyn SpellCoreDb2HotfixPersistencePortLikeCpp,
    removals: &Db2HotfixRemovalStoreLikeCpp,
) -> Result<SpellTargetRestrictionsStore> {
    let rows = loaded_rows_like_cpp(
        persistence
            .load_spell_target_restrictions_rows_like_cpp()
            .await,
    )?;
    SpellTargetRestrictionsStore::load_effective_from_hotfix_rows_like_cpp(
        data_dir,
        locale,
        rows.into_iter()
            .map(spell_target_restrictions_entry_like_cpp),
        removals,
    )
}

pub(super) async fn load_spell_x_spell_visual_store_like_cpp(
    data_dir: &str,
    locale: &str,
    persistence: &dyn SpellCoreDb2HotfixPersistencePortLikeCpp,
    removals: &Db2HotfixRemovalStoreLikeCpp,
) -> Result<SpellXSpellVisualStore> {
    let rows = loaded_rows_like_cpp(persistence.load_spell_x_spell_visual_rows_like_cpp().await)?;
    SpellXSpellVisualStore::load_effective_from_hotfix_rows_like_cpp(
        data_dir,
        locale,
        rows.into_iter().map(spell_x_spell_visual_entry_like_cpp),
        removals,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_and_failure_remain_distinct_before_publication() {
        assert!(
            loaded_rows_like_cpp::<u32>(SpellCoreDb2HotfixLoadOutcomeLikeCpp::Loaded(Vec::new()))
                .unwrap()
                .is_empty()
        );
        let error = loaded_rows_like_cpp::<u32>(SpellCoreDb2HotfixLoadOutcomeLikeCpp::Failed {
            reason: "hotfix unavailable".to_owned(),
        })
        .unwrap_err();
        assert_eq!(error.to_string(), "hotfix unavailable");
    }

    #[test]
    fn large_row_conversion_preserves_every_field() {
        let misc_row = SpellMiscHotfixRowLikeCpp {
            id: 1,
            attributes: std::array::from_fn(|index| index as i32 + 10),
            difficulty_id: 2,
            casting_time_index: 3,
            duration_index: 4,
            range_index: 5,
            school_mask: 6,
            speed: 7.5,
            launch_delay: 8.5,
            min_duration: 9.5,
            spell_icon_file_data_id: -10,
            active_icon_file_data_id: -11,
            content_tuning_id: -12,
            show_future_spell_player_condition_id: -13,
            spell_id: 14,
        };
        assert_eq!(
            spell_misc_entry_like_cpp(misc_row),
            SpellMiscEntry {
                id: misc_row.id,
                attributes: misc_row.attributes,
                difficulty_id: misc_row.difficulty_id,
                casting_time_index: misc_row.casting_time_index,
                duration_index: misc_row.duration_index,
                range_index: misc_row.range_index,
                school_mask: misc_row.school_mask,
                speed: misc_row.speed,
                launch_delay: misc_row.launch_delay,
                min_duration: misc_row.min_duration,
                spell_icon_file_data_id: misc_row.spell_icon_file_data_id,
                active_icon_file_data_id: misc_row.active_icon_file_data_id,
                content_tuning_id: misc_row.content_tuning_id,
                show_future_spell_player_condition_id: misc_row
                    .show_future_spell_player_condition_id,
                spell_id: misc_row.spell_id,
            }
        );

        let effect_row = SpellEffectHotfixRowLikeCpp {
            id: 20,
            difficulty_id: -21,
            effect_index: -22,
            effect: 23,
            effect_amplitude: 24.5,
            effect_attributes: -25,
            effect_aura: -26,
            effect_aura_period: -27,
            effect_base_points: -28,
            effect_bonus_coefficient: 29.5,
            effect_chain_amplitude: 30.5,
            effect_chain_targets: -31,
            effect_die_sides: -32,
            effect_item_type: -33,
            effect_mechanic: -34,
            effect_points_per_resource: 35.5,
            effect_pos_facing: 36.5,
            effect_real_points_per_level: 37.5,
            effect_trigger_spell: -38,
            bonus_coefficient_from_ap: 39.5,
            pvp_multiplier: 40.5,
            coefficient: 41.5,
            variance: 42.5,
            resource_coefficient: 43.5,
            group_size_base_points_coefficient: 44.5,
            effect_misc_value: [-45, -46],
            effect_radius_index: [47, 48],
            effect_spell_class_mask: [49, 50, 51, 52],
            implicit_target: [-53, -54],
            spell_id: 55,
        };
        let converted = spell_effect_entry_like_cpp(effect_row);
        assert_eq!(converted.id, effect_row.id);
        assert_eq!(converted.difficulty_id, effect_row.difficulty_id);
        assert_eq!(converted.effect_index, effect_row.effect_index);
        assert_eq!(converted.effect, effect_row.effect);
        assert_eq!(converted.effect_amplitude, effect_row.effect_amplitude);
        assert_eq!(converted.effect_attributes, effect_row.effect_attributes);
        assert_eq!(converted.effect_aura, effect_row.effect_aura);
        assert_eq!(converted.effect_aura_period, effect_row.effect_aura_period);
        assert_eq!(converted.effect_base_points, effect_row.effect_base_points);
        assert_eq!(
            converted.effect_bonus_coefficient,
            effect_row.effect_bonus_coefficient
        );
        assert_eq!(
            converted.effect_chain_amplitude,
            effect_row.effect_chain_amplitude
        );
        assert_eq!(
            converted.effect_chain_targets,
            effect_row.effect_chain_targets
        );
        assert_eq!(converted.effect_die_sides, effect_row.effect_die_sides);
        assert_eq!(converted.effect_item_type, effect_row.effect_item_type);
        assert_eq!(converted.effect_mechanic, effect_row.effect_mechanic);
        assert_eq!(
            converted.effect_points_per_resource,
            effect_row.effect_points_per_resource
        );
        assert_eq!(converted.effect_pos_facing, effect_row.effect_pos_facing);
        assert_eq!(
            converted.effect_real_points_per_level,
            effect_row.effect_real_points_per_level
        );
        assert_eq!(
            converted.effect_trigger_spell,
            effect_row.effect_trigger_spell
        );
        assert_eq!(
            converted.bonus_coefficient_from_ap,
            effect_row.bonus_coefficient_from_ap
        );
        assert_eq!(converted.pvp_multiplier, effect_row.pvp_multiplier);
        assert_eq!(converted.coefficient, effect_row.coefficient);
        assert_eq!(converted.variance, effect_row.variance);
        assert_eq!(
            converted.resource_coefficient,
            effect_row.resource_coefficient
        );
        assert_eq!(
            converted.group_size_base_points_coefficient,
            effect_row.group_size_base_points_coefficient
        );
        assert_eq!(converted.effect_misc_value, effect_row.effect_misc_value);
        assert_eq!(
            converted.effect_radius_index,
            effect_row.effect_radius_index
        );
        assert_eq!(
            converted.effect_spell_class_mask,
            effect_row.effect_spell_class_mask
        );
        assert_eq!(converted.implicit_target, effect_row.implicit_target);
        assert_eq!(converted.spell_id, effect_row.spell_id);

        let power_row = SpellPowerHotfixRowLikeCpp {
            id: 60,
            order_index: 61,
            mana_cost: -62,
            mana_cost_per_level: -63,
            mana_per_second: -64,
            power_display_id: 65,
            alt_power_bar_id: -66,
            power_cost_pct: 67.5,
            power_cost_max_pct: 68.5,
            power_pct_per_second: 69.5,
            power_type: -70,
            required_aura_spell_id: -71,
            optional_cost: 72,
            spell_id: 73,
        };
        assert_eq!(
            spell_power_entry_like_cpp(power_row),
            SpellPowerEntry {
                id: power_row.id,
                order_index: power_row.order_index,
                mana_cost: power_row.mana_cost,
                mana_cost_per_level: power_row.mana_cost_per_level,
                mana_per_second: power_row.mana_per_second,
                power_display_id: power_row.power_display_id,
                alt_power_bar_id: power_row.alt_power_bar_id,
                power_cost_pct: power_row.power_cost_pct,
                power_cost_max_pct: power_row.power_cost_max_pct,
                power_pct_per_second: power_row.power_pct_per_second,
                power_type: power_row.power_type,
                required_aura_spell_id: power_row.required_aura_spell_id,
                optional_cost: power_row.optional_cost,
                spell_id: power_row.spell_id,
            }
        );
    }

    #[test]
    fn small_row_conversion_preserves_every_field() {
        assert_eq!(
            spell_name_entry_like_cpp(SpellNameHotfixRowLikeCpp {
                id: 1,
                name: "spell".to_owned(),
            }),
            SpellNameEntry {
                id: 1,
                name: "spell".to_owned(),
            }
        );
        assert_eq!(
            spell_categories_entry_like_cpp(SpellCategoriesHotfixRowLikeCpp {
                id: 2,
                difficulty_id: 3,
                category: -4,
                defense_type: -5,
                dispel_type: -6,
                mechanic: -7,
                prevention_type: -8,
                start_recovery_category: -9,
                charge_category: -10,
                spell_id: 11,
            }),
            SpellCategoriesEntry {
                id: 2,
                difficulty_id: 3,
                category: -4,
                defense_type: -5,
                dispel_type: -6,
                mechanic: -7,
                prevention_type: -8,
                start_recovery_category: -9,
                charge_category: -10,
                spell_id: 11,
            }
        );
        assert_eq!(
            spell_shapeshift_entry_like_cpp(SpellShapeshiftHotfixRowLikeCpp {
                id: 12,
                spell_id: -13,
                stance_bar_order: -14,
                shapeshift_exclude: [-15, -16],
                shapeshift_mask: [-17, -18],
            }),
            SpellShapeshiftEntry {
                id: 12,
                spell_id: -13,
                stance_bar_order: -14,
                shapeshift_exclude: [-15, -16],
                shapeshift_mask: [-17, -18],
            }
        );
        assert_eq!(
            spell_interrupts_entry_like_cpp(SpellInterruptsHotfixRowLikeCpp {
                id: 19,
                difficulty_id: 20,
                interrupt_flags: -21,
                aura_interrupt_flags: [-22, -23],
                channel_interrupt_flags: [-24, -25],
                spell_id: 26,
            }),
            SpellInterruptsEntry {
                id: 19,
                difficulty_id: 20,
                interrupt_flags: -21,
                aura_interrupt_flags: [-22, -23],
                channel_interrupt_flags: [-24, -25],
                spell_id: 26,
            }
        );
        assert_eq!(
            spell_cast_times_entry_like_cpp(SpellCastTimesHotfixRowLikeCpp {
                id: 27,
                base: -28,
                per_level: -29,
                minimum: -30,
            }),
            SpellCastTimesEntry {
                id: 27,
                base: -28,
                per_level: -29,
                minimum: -30,
            }
        );
        assert_eq!(
            spell_cooldowns_entry_like_cpp(SpellCooldownsHotfixRowLikeCpp {
                id: 31,
                difficulty_id: 32,
                category_recovery_time: -33,
                recovery_time: -34,
                start_recovery_time: -35,
                spell_id: 36,
            }),
            SpellCooldownsEntry {
                id: 31,
                difficulty_id: 32,
                category_recovery_time: -33,
                recovery_time: -34,
                start_recovery_time: -35,
                spell_id: 36,
            }
        );
        assert_eq!(
            spell_casting_requirements_entry_like_cpp(SpellCastingRequirementsHotfixRowLikeCpp {
                id: 37,
                spell_id: -38,
                facing_caster_flags: 39,
                min_faction_id: 40,
                min_reputation: -41,
                required_areas_id: 42,
                required_aura_vision: 43,
                requires_spell_focus: 44,
            }),
            SpellCastingRequirementsEntry {
                id: 37,
                spell_id: -38,
                facing_caster_flags: 39,
                min_faction_id: 40,
                min_reputation: -41,
                required_areas_id: 42,
                required_aura_vision: 43,
                requires_spell_focus: 44,
            }
        );
        assert_eq!(
            spell_power_difficulty_entry_like_cpp(SpellPowerDifficultyHotfixRowLikeCpp {
                id: 45,
                difficulty_id: 46,
                order_index: 47,
            }),
            SpellPowerDifficultyEntry {
                id: 45,
                difficulty_id: 46,
                order_index: 47,
            }
        );
    }

    #[test]
    fn standalone_spell_db2_row_conversion_preserves_every_field() {
        assert_eq!(
            spell_aura_restrictions_entry_like_cpp(SpellAuraRestrictionsHotfixRowLikeCpp {
                id: 1,
                difficulty_id: 2,
                caster_aura_state: 3,
                target_aura_state: 4,
                exclude_caster_aura_state: 5,
                exclude_target_aura_state: 6,
                caster_aura_spell: -7,
                target_aura_spell: -8,
                exclude_caster_aura_spell: -9,
                exclude_target_aura_spell: -10,
                spell_id: 11,
            }),
            SpellAuraRestrictionsEntry {
                id: 1,
                difficulty_id: 2,
                caster_aura_state: 3,
                target_aura_state: 4,
                exclude_caster_aura_state: 5,
                exclude_target_aura_state: 6,
                caster_aura_spell: -7,
                target_aura_spell: -8,
                exclude_caster_aura_spell: -9,
                exclude_target_aura_spell: -10,
                spell_id: 11,
            }
        );
        assert_eq!(
            spell_category_entry_like_cpp(SpellCategoryHotfixRowLikeCpp {
                id: 12,
                name: "category".to_owned(),
                flags: -13,
                uses_per_week: 14,
                max_charges: -15,
                charge_recovery_time: -16,
                type_mask: -17,
            }),
            SpellCategoryEntry {
                id: 12,
                name: "category".to_owned(),
                flags: -13,
                uses_per_week: 14,
                max_charges: -15,
                charge_recovery_time: -16,
                type_mask: -17,
            }
        );
        assert_eq!(
            spell_duration_entry_like_cpp(SpellDurationHotfixRowLikeCpp {
                id: 18,
                duration: -19,
                duration_per_level: 20,
                max_duration: -21,
            }),
            SpellDurationEntry {
                id: 18,
                duration: -19,
                duration_per_level: 20,
                max_duration: -21
            }
        );
        assert_eq!(
            spell_radius_entry_like_cpp(SpellRadiusHotfixRowLikeCpp {
                id: 22,
                radius: 23.5,
                radius_per_level: 24.5,
                radius_min: 25.5,
                radius_max: 26.5,
            }),
            SpellRadiusEntry {
                id: 22,
                radius: 23.5,
                radius_per_level: 24.5,
                radius_min: 25.5,
                radius_max: 26.5,
            }
        );
        assert_eq!(
            spell_range_entry_like_cpp(SpellRangeHotfixRowLikeCpp {
                id: 27,
                display_name: "range".to_owned(),
                display_name_short: "r".to_owned(),
                flags: 28,
                range_min: [29.5, 30.5],
                range_max: [31.5, 32.5],
            }),
            SpellRangeEntry {
                id: 27,
                display_name: "range".to_owned(),
                display_name_short: "r".to_owned(),
                flags: 28,
                range_min: [29.5, 30.5],
                range_max: [31.5, 32.5],
            }
        );
        assert_eq!(
            spell_equipped_items_entry_like_cpp(SpellEquippedItemsHotfixRowLikeCpp {
                id: 33,
                spell_id: -34,
                equipped_item_class: -35,
                equipped_item_inv_types: -36,
                equipped_item_subclass: -37,
            }),
            SpellEquippedItemsEntry {
                id: 33,
                spell_id: -34,
                equipped_item_class: -35,
                equipped_item_inv_types: -36,
                equipped_item_subclass: -37,
            }
        );
        assert_eq!(
            spell_target_restrictions_entry_like_cpp(SpellTargetRestrictionsHotfixRowLikeCpp {
                id: 38,
                difficulty_id: 39,
                cone_degrees: 40.5,
                max_targets: 41,
                max_target_level: 42,
                target_creature_type: -43,
                targets: -44,
                width: 45.5,
                spell_id: 46,
            }),
            SpellTargetRestrictionsEntry {
                id: 38,
                difficulty_id: 39,
                cone_degrees: 40.5,
                max_targets: 41,
                max_target_level: 42,
                target_creature_type: -43,
                targets: -44,
                width: 45.5,
                spell_id: 46,
            }
        );
        assert_eq!(
            spell_x_spell_visual_entry_like_cpp(SpellXSpellVisualHotfixRowLikeCpp {
                id: 47,
                difficulty_id: 48,
                spell_visual_id: 49,
                probability: 50.5,
                flags: 51,
                priority: -52,
                spell_icon_file_id: -53,
                active_icon_file_id: -54,
                viewer_unit_condition_id: 55,
                viewer_player_condition_id: 56,
                caster_unit_condition_id: 57,
                caster_player_condition_id: 58,
                spell_id: 59,
            }),
            SpellXSpellVisualEntry {
                id: 47,
                difficulty_id: 48,
                spell_visual_id: 49,
                probability: 50.5,
                flags: 51,
                priority: -52,
                spell_icon_file_id: -53,
                active_icon_file_id: -54,
                viewer_unit_condition_id: 55,
                viewer_player_condition_id: 56,
                caster_unit_condition_id: 57,
                caster_player_condition_id: 58,
                spell_id: 59,
            }
        );
    }
}
