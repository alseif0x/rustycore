//! Checked field decoders for the core spell hotfix row projections.

use anyhow::Result;
use wow_persistence::{
    SpellAuraOptionsHotfixRow, SpellAuraRestrictionsHotfixRowLikeCpp,
    SpellCastTimesHotfixRowLikeCpp, SpellCastingRequirementsHotfixRowLikeCpp,
    SpellCategoriesHotfixRowLikeCpp, SpellCategoryHotfixRowLikeCpp,
    SpellClassOptionsHotfixRow, SpellCooldownsHotfixRowLikeCpp, SpellDurationHotfixRowLikeCpp,
    SpellEffectHotfixRowLikeCpp, SpellEquippedItemsHotfixRowLikeCpp,
    SpellInterruptsHotfixRowLikeCpp, SpellLabelHotfixRow, SpellLevelsHotfixRow,
    SpellMiscHotfixRowLikeCpp, SpellNameHotfixRowLikeCpp, SpellPowerDifficultyHotfixRowLikeCpp,
    SpellPowerHotfixRowLikeCpp, SpellProcsPerMinuteHotfixRow, SpellRadiusHotfixRowLikeCpp,
    SpellRangeHotfixRowLikeCpp,
    SpellShapeshiftHotfixRowLikeCpp, SpellTargetRestrictionsHotfixRowLikeCpp,
    SpellXSpellVisualHotfixRowLikeCpp,
};

use crate::field_like_cpp::{FieldNumberLikeCpp, FieldRowLikeCpp, NativeFieldLikeCpp};

fn numeric_array<T: FieldNumberLikeCpp, const N: usize>(
    row: &[NativeFieldLikeCpp],
    first_column: usize,
    table: &str,
    field_prefix: &str,
) -> Result<[T; N]> {
    let mut values = [T::default(); N];
    for (index, value) in values.iter_mut().enumerate() {
        let field = format!("{table}.{field_prefix}{}", index + 1);
        *value = row.get_like_cpp(first_column + index, &field)?;
    }
    Ok(values)
}

pub(super) fn spell_name(row: &[NativeFieldLikeCpp]) -> Result<SpellNameHotfixRowLikeCpp> {
    Ok(SpellNameHotfixRowLikeCpp {
        id: row.get_like_cpp(0, "spell_name.ID")?,
        name: row.get_string_like_cpp(1, "spell_name.Name")?,
    })
}

pub(super) fn spell_categories(
    row: &[NativeFieldLikeCpp],
) -> Result<SpellCategoriesHotfixRowLikeCpp> {
    Ok(SpellCategoriesHotfixRowLikeCpp {
        id: row.get_like_cpp(0, "spell_categories.ID")?,
        difficulty_id: row.get_like_cpp(1, "spell_categories.DifficultyID")?,
        category: row.get_like_cpp(2, "spell_categories.Category")?,
        defense_type: row.get_like_cpp(3, "spell_categories.DefenseType")?,
        dispel_type: row.get_like_cpp(4, "spell_categories.DispelType")?,
        mechanic: row.get_like_cpp(5, "spell_categories.Mechanic")?,
        prevention_type: row.get_like_cpp(6, "spell_categories.PreventionType")?,
        start_recovery_category: row.get_like_cpp(7, "spell_categories.StartRecoveryCategory")?,
        charge_category: row.get_like_cpp(8, "spell_categories.ChargeCategory")?,
        spell_id: row.get_like_cpp(9, "spell_categories.SpellID")?,
    })
}

pub(super) fn spell_misc(row: &[NativeFieldLikeCpp]) -> Result<SpellMiscHotfixRowLikeCpp> {
    Ok(SpellMiscHotfixRowLikeCpp {
        id: row.get_like_cpp(0, "spell_misc.ID")?,
        attributes: numeric_array::<i32, 15>(row, 1, "spell_misc", "Attributes")?,
        difficulty_id: row.get_like_cpp(16, "spell_misc.DifficultyID")?,
        casting_time_index: row.get_like_cpp(17, "spell_misc.CastingTimeIndex")?,
        duration_index: row.get_like_cpp(18, "spell_misc.DurationIndex")?,
        range_index: row.get_like_cpp(19, "spell_misc.RangeIndex")?,
        school_mask: row.get_like_cpp(20, "spell_misc.SchoolMask")?,
        speed: row.get_like_cpp(21, "spell_misc.Speed")?,
        launch_delay: row.get_like_cpp(22, "spell_misc.LaunchDelay")?,
        min_duration: row.get_like_cpp(23, "spell_misc.MinDuration")?,
        spell_icon_file_data_id: row.get_like_cpp(24, "spell_misc.SpellIconFileDataID")?,
        active_icon_file_data_id: row.get_like_cpp(25, "spell_misc.ActiveIconFileDataID")?,
        content_tuning_id: row.get_like_cpp(26, "spell_misc.ContentTuningID")?,
        show_future_spell_player_condition_id: row
            .get_like_cpp(27, "spell_misc.ShowFutureSpellPlayerConditionID")?,
        spell_id: row.get_like_cpp(28, "spell_misc.SpellID")?,
    })
}

pub(super) fn spell_effect(row: &[NativeFieldLikeCpp]) -> Result<SpellEffectHotfixRowLikeCpp> {
    let signed_class_mask =
        numeric_array::<i32, 4>(row, 29, "spell_effect", "EffectSpellClassMask")?;
    Ok(SpellEffectHotfixRowLikeCpp {
        id: row.get_like_cpp(0, "spell_effect.ID")?,
        difficulty_id: row.get_like_cpp(1, "spell_effect.DifficultyID")?,
        effect_index: row.get_like_cpp(2, "spell_effect.EffectIndex")?,
        effect: row.get_like_cpp(3, "spell_effect.Effect")?,
        effect_amplitude: row.get_like_cpp(4, "spell_effect.EffectAmplitude")?,
        effect_attributes: row.get_like_cpp(5, "spell_effect.EffectAttributes")?,
        effect_aura: row.get_like_cpp(6, "spell_effect.EffectAura")?,
        effect_aura_period: row.get_like_cpp(7, "spell_effect.EffectAuraPeriod")?,
        effect_base_points: row.get_like_cpp(8, "spell_effect.EffectBasePoints")?,
        effect_bonus_coefficient: row.get_like_cpp(9, "spell_effect.EffectBonusCoefficient")?,
        effect_chain_amplitude: row.get_like_cpp(10, "spell_effect.EffectChainAmplitude")?,
        effect_chain_targets: row.get_like_cpp(11, "spell_effect.EffectChainTargets")?,
        effect_die_sides: row.get_like_cpp(12, "spell_effect.EffectDieSides")?,
        effect_item_type: row.get_like_cpp(13, "spell_effect.EffectItemType")?,
        effect_mechanic: row.get_like_cpp(14, "spell_effect.EffectMechanic")?,
        effect_points_per_resource: row.get_like_cpp(15, "spell_effect.EffectPointsPerResource")?,
        effect_pos_facing: row.get_like_cpp(16, "spell_effect.EffectPosFacing")?,
        effect_real_points_per_level: row.get_like_cpp(17, "spell_effect.EffectRealPointsPerLevel")?,
        effect_trigger_spell: row.get_like_cpp(18, "spell_effect.EffectTriggerSpell")?,
        bonus_coefficient_from_ap: row.get_like_cpp(19, "spell_effect.BonusCoefficientFromAP")?,
        pvp_multiplier: row.get_like_cpp(20, "spell_effect.PvpMultiplier")?,
        coefficient: row.get_like_cpp(21, "spell_effect.Coefficient")?,
        variance: row.get_like_cpp(22, "spell_effect.Variance")?,
        resource_coefficient: row.get_like_cpp(23, "spell_effect.ResourceCoefficient")?,
        group_size_base_points_coefficient: row
            .get_like_cpp(24, "spell_effect.GroupSizeBasePointsCoefficient")?,
        effect_misc_value: numeric_array::<i32, 2>(row, 25, "spell_effect", "EffectMiscValue")?,
        effect_radius_index: numeric_array::<u32, 2>(row, 27, "spell_effect", "EffectRadiusIndex")?,
        effect_spell_class_mask: signed_class_mask.map(|mask| mask as u32),
        implicit_target: numeric_array::<i16, 2>(row, 33, "spell_effect", "ImplicitTarget")?,
        spell_id: row.get_like_cpp(35, "spell_effect.SpellID")?,
    })
}

pub(super) fn spell_shapeshift(
    row: &[NativeFieldLikeCpp],
) -> Result<SpellShapeshiftHotfixRowLikeCpp> {
    Ok(SpellShapeshiftHotfixRowLikeCpp {
        id: row.get_like_cpp(0, "spell_shapeshift.ID")?,
        spell_id: row.get_like_cpp(1, "spell_shapeshift.SpellID")?,
        stance_bar_order: row.get_like_cpp(2, "spell_shapeshift.StanceBarOrder")?,
        shapeshift_exclude: numeric_array::<i32, 2>(
            row,
            3,
            "spell_shapeshift",
            "ShapeshiftExclude",
        )?,
        shapeshift_mask: numeric_array::<i32, 2>(
            row,
            5,
            "spell_shapeshift",
            "ShapeshiftMask",
        )?,
    })
}

pub(super) fn spell_interrupts(
    row: &[NativeFieldLikeCpp],
) -> Result<SpellInterruptsHotfixRowLikeCpp> {
    Ok(SpellInterruptsHotfixRowLikeCpp {
        id: row.get_like_cpp(0, "spell_interrupts.ID")?,
        difficulty_id: row.get_like_cpp(1, "spell_interrupts.DifficultyID")?,
        interrupt_flags: row.get_like_cpp(2, "spell_interrupts.InterruptFlags")?,
        aura_interrupt_flags: numeric_array::<i32, 2>(
            row,
            3,
            "spell_interrupts",
            "AuraInterruptFlags",
        )?,
        channel_interrupt_flags: numeric_array::<i32, 2>(
            row,
            5,
            "spell_interrupts",
            "ChannelInterruptFlags",
        )?,
        spell_id: row.get_like_cpp(7, "spell_interrupts.SpellID")?,
    })
}

pub(super) fn spell_cast_times(
    row: &[NativeFieldLikeCpp],
) -> Result<SpellCastTimesHotfixRowLikeCpp> {
    Ok(SpellCastTimesHotfixRowLikeCpp {
        id: row.get_like_cpp(0, "spell_cast_times.ID")?,
        base: row.get_like_cpp(1, "spell_cast_times.Base")?,
        per_level: row.get_like_cpp(2, "spell_cast_times.PerLevel")?,
        minimum: row.get_like_cpp(3, "spell_cast_times.Minimum")?,
    })
}

pub(super) fn spell_cooldowns(
    row: &[NativeFieldLikeCpp],
) -> Result<SpellCooldownsHotfixRowLikeCpp> {
    Ok(SpellCooldownsHotfixRowLikeCpp {
        id: row.get_like_cpp(0, "spell_cooldowns.ID")?,
        difficulty_id: row.get_like_cpp(1, "spell_cooldowns.DifficultyID")?,
        category_recovery_time: row.get_like_cpp(2, "spell_cooldowns.CategoryRecoveryTime")?,
        recovery_time: row.get_like_cpp(3, "spell_cooldowns.RecoveryTime")?,
        start_recovery_time: row.get_like_cpp(4, "spell_cooldowns.StartRecoveryTime")?,
        spell_id: row.get_like_cpp(5, "spell_cooldowns.SpellID")?,
    })
}

pub(super) fn spell_casting_requirements(
    row: &[NativeFieldLikeCpp],
) -> Result<SpellCastingRequirementsHotfixRowLikeCpp> {
    Ok(SpellCastingRequirementsHotfixRowLikeCpp {
        id: row.get_like_cpp(0, "spell_casting_requirements.ID")?,
        spell_id: row.get_like_cpp(1, "spell_casting_requirements.SpellID")?,
        facing_caster_flags: row.get_like_cpp(2, "spell_casting_requirements.FacingCasterFlags")?,
        min_faction_id: row.get_like_cpp(3, "spell_casting_requirements.MinFactionID")?,
        min_reputation: row.get_like_cpp(4, "spell_casting_requirements.MinReputation")?,
        required_areas_id: row.get_like_cpp(5, "spell_casting_requirements.RequiredAreasID")?,
        required_aura_vision: row.get_like_cpp(6, "spell_casting_requirements.RequiredAuraVision")?,
        requires_spell_focus: row.get_like_cpp(7, "spell_casting_requirements.RequiresSpellFocus")?,
    })
}

pub(super) fn spell_power(row: &[NativeFieldLikeCpp]) -> Result<SpellPowerHotfixRowLikeCpp> {
    Ok(SpellPowerHotfixRowLikeCpp {
        id: row.get_like_cpp(0, "spell_power.ID")?,
        order_index: row.get_like_cpp(1, "spell_power.OrderIndex")?,
        mana_cost: row.get_like_cpp(2, "spell_power.ManaCost")?,
        mana_cost_per_level: row.get_like_cpp(3, "spell_power.ManaCostPerLevel")?,
        mana_per_second: row.get_like_cpp(4, "spell_power.ManaPerSecond")?,
        power_display_id: row.get_like_cpp(5, "spell_power.PowerDisplayID")?,
        alt_power_bar_id: row.get_like_cpp(6, "spell_power.AltPowerBarID")?,
        power_cost_pct: row.get_like_cpp(7, "spell_power.PowerCostPct")?,
        power_cost_max_pct: row.get_like_cpp(8, "spell_power.PowerCostMaxPct")?,
        power_pct_per_second: row.get_like_cpp(9, "spell_power.PowerPctPerSecond")?,
        power_type: row.get_like_cpp(10, "spell_power.PowerType")?,
        required_aura_spell_id: row.get_like_cpp(11, "spell_power.RequiredAuraSpellID")?,
        optional_cost: row.get_like_cpp(12, "spell_power.OptionalCost")?,
        spell_id: row.get_like_cpp(13, "spell_power.SpellID")?,
    })
}

pub(super) fn spell_power_difficulty(
    row: &[NativeFieldLikeCpp],
) -> Result<SpellPowerDifficultyHotfixRowLikeCpp> {
    Ok(SpellPowerDifficultyHotfixRowLikeCpp {
        id: row.get_like_cpp(0, "spell_power_difficulty.ID")?,
        difficulty_id: row.get_like_cpp(1, "spell_power_difficulty.DifficultyID")?,
        order_index: row.get_like_cpp(2, "spell_power_difficulty.OrderIndex")?,
    })
}

pub(super) fn spell_aura_restrictions(
    row: &[NativeFieldLikeCpp],
) -> Result<SpellAuraRestrictionsHotfixRowLikeCpp> {
    Ok(SpellAuraRestrictionsHotfixRowLikeCpp {
        id: row.get_like_cpp(0, "spell_aura_restrictions.ID")?,
        difficulty_id: row.get_like_cpp(1, "spell_aura_restrictions.DifficultyID")?,
        caster_aura_state: row.get_like_cpp(2, "spell_aura_restrictions.CasterAuraState")?,
        target_aura_state: row.get_like_cpp(3, "spell_aura_restrictions.TargetAuraState")?,
        exclude_caster_aura_state: row.get_like_cpp(4, "spell_aura_restrictions.ExcludeCasterAuraState")?,
        exclude_target_aura_state: row.get_like_cpp(5, "spell_aura_restrictions.ExcludeTargetAuraState")?,
        caster_aura_spell: row.get_like_cpp(6, "spell_aura_restrictions.CasterAuraSpell")?,
        target_aura_spell: row.get_like_cpp(7, "spell_aura_restrictions.TargetAuraSpell")?,
        exclude_caster_aura_spell: row.get_like_cpp(8, "spell_aura_restrictions.ExcludeCasterAuraSpell")?,
        exclude_target_aura_spell: row.get_like_cpp(9, "spell_aura_restrictions.ExcludeTargetAuraSpell")?,
        spell_id: row.get_like_cpp(10, "spell_aura_restrictions.SpellID")?,
    })
}

pub(super) fn spell_category(
    row: &[NativeFieldLikeCpp],
) -> Result<SpellCategoryHotfixRowLikeCpp> {
    Ok(SpellCategoryHotfixRowLikeCpp {
        id: row.get_like_cpp(0, "spell_category.ID")?,
        name: row.get_string_like_cpp(1, "spell_category.Name")?,
        flags: row.get_like_cpp(2, "spell_category.Flags")?,
        uses_per_week: row.get_like_cpp(3, "spell_category.UsesPerWeek")?,
        max_charges: row.get_like_cpp(4, "spell_category.MaxCharges")?,
        charge_recovery_time: row.get_like_cpp(5, "spell_category.ChargeRecoveryTime")?,
        type_mask: row.get_like_cpp(6, "spell_category.TypeMask")?,
    })
}

pub(super) fn spell_duration(
    row: &[NativeFieldLikeCpp],
) -> Result<SpellDurationHotfixRowLikeCpp> {
    Ok(SpellDurationHotfixRowLikeCpp {
        id: row.get_like_cpp(0, "spell_duration.ID")?,
        duration: row.get_like_cpp(1, "spell_duration.Duration")?,
        duration_per_level: row.get_like_cpp(2, "spell_duration.DurationPerLevel")?,
        max_duration: row.get_like_cpp(3, "spell_duration.MaxDuration")?,
    })
}

pub(super) fn spell_radius(row: &[NativeFieldLikeCpp]) -> Result<SpellRadiusHotfixRowLikeCpp> {
    Ok(SpellRadiusHotfixRowLikeCpp {
        id: row.get_like_cpp(0, "spell_radius.ID")?,
        radius: row.get_like_cpp(1, "spell_radius.Radius")?,
        radius_per_level: row.get_like_cpp(2, "spell_radius.RadiusPerLevel")?,
        radius_min: row.get_like_cpp(3, "spell_radius.RadiusMin")?,
        radius_max: row.get_like_cpp(4, "spell_radius.RadiusMax")?,
    })
}

pub(super) fn spell_range(row: &[NativeFieldLikeCpp]) -> Result<SpellRangeHotfixRowLikeCpp> {
    Ok(SpellRangeHotfixRowLikeCpp {
        id: row.get_like_cpp(0, "spell_range.ID")?,
        display_name: row.get_string_like_cpp(1, "spell_range.DisplayName")?,
        display_name_short: row.get_string_like_cpp(2, "spell_range.DisplayNameShort")?,
        flags: row.get_like_cpp(3, "spell_range.Flags")?,
        range_min: numeric_array::<f32, 2>(row, 4, "spell_range", "RangeMin")?,
        range_max: numeric_array::<f32, 2>(row, 6, "spell_range", "RangeMax")?,
    })
}

pub(super) fn spell_equipped_items(
    row: &[NativeFieldLikeCpp],
) -> Result<SpellEquippedItemsHotfixRowLikeCpp> {
    Ok(SpellEquippedItemsHotfixRowLikeCpp {
        id: row.get_like_cpp(0, "spell_equipped_items.ID")?,
        spell_id: row.get_like_cpp(1, "spell_equipped_items.SpellID")?,
        equipped_item_class: row.get_like_cpp(2, "spell_equipped_items.EquippedItemClass")?,
        equipped_item_inv_types: row.get_like_cpp(3, "spell_equipped_items.EquippedItemInvTypes")?,
        equipped_item_subclass: row.get_like_cpp(4, "spell_equipped_items.EquippedItemSubclass")?,
    })
}

pub(super) fn spell_target_restrictions(
    row: &[NativeFieldLikeCpp],
) -> Result<SpellTargetRestrictionsHotfixRowLikeCpp> {
    Ok(SpellTargetRestrictionsHotfixRowLikeCpp {
        id: row.get_like_cpp(0, "spell_target_restrictions.ID")?,
        difficulty_id: row.get_like_cpp(1, "spell_target_restrictions.DifficultyID")?,
        cone_degrees: row.get_like_cpp(2, "spell_target_restrictions.ConeDegrees")?,
        max_targets: row.get_like_cpp(3, "spell_target_restrictions.MaxTargets")?,
        max_target_level: row.get_like_cpp(4, "spell_target_restrictions.MaxTargetLevel")?,
        target_creature_type: row.get_like_cpp(5, "spell_target_restrictions.TargetCreatureType")?,
        targets: row.get_like_cpp(6, "spell_target_restrictions.Targets")?,
        width: row.get_like_cpp(7, "spell_target_restrictions.Width")?,
        spell_id: row.get_like_cpp(8, "spell_target_restrictions.SpellID")?,
    })
}

pub(super) fn spell_x_spell_visual(
    row: &[NativeFieldLikeCpp],
) -> Result<SpellXSpellVisualHotfixRowLikeCpp> {
    Ok(SpellXSpellVisualHotfixRowLikeCpp {
        id: row.get_like_cpp(0, "spell_x_spell_visual.ID")?,
        difficulty_id: row.get_like_cpp(1, "spell_x_spell_visual.DifficultyID")?,
        spell_visual_id: row.get_like_cpp(2, "spell_x_spell_visual.SpellVisualID")?,
        probability: row.get_like_cpp(3, "spell_x_spell_visual.Probability")?,
        flags: row.get_like_cpp(4, "spell_x_spell_visual.Flags")?,
        priority: row.get_like_cpp(5, "spell_x_spell_visual.Priority")?,
        spell_icon_file_id: row.get_like_cpp(6, "spell_x_spell_visual.SpellIconFileID")?,
        active_icon_file_id: row.get_like_cpp(7, "spell_x_spell_visual.ActiveIconFileID")?,
        viewer_unit_condition_id: row.get_like_cpp(8, "spell_x_spell_visual.ViewerUnitConditionID")?,
        viewer_player_condition_id: row.get_like_cpp(9, "spell_x_spell_visual.ViewerPlayerConditionID")?,
        caster_unit_condition_id: row.get_like_cpp(10, "spell_x_spell_visual.CasterUnitConditionID")?,
        caster_player_condition_id: row.get_like_cpp(11, "spell_x_spell_visual.CasterPlayerConditionID")?,
        spell_id: row.get_like_cpp(12, "spell_x_spell_visual.SpellID")?,
    })
}

pub(super) fn spell_aura_options(
    row: &[NativeFieldLikeCpp],
) -> Result<SpellAuraOptionsHotfixRow> {
    Ok(SpellAuraOptionsHotfixRow {
        id: row.get_like_cpp(0, "spell_aura_options.ID")?,
        difficulty_id: row.get_like_cpp(1, "spell_aura_options.DifficultyID")?,
        cumulative_aura: row.get_like_cpp(2, "spell_aura_options.CumulativeAura")?,
        proc_category_recovery: row.get_like_cpp(3, "spell_aura_options.ProcCategoryRecovery")?,
        proc_chance: row.get_like_cpp(4, "spell_aura_options.ProcChance")?,
        proc_charges: row.get_like_cpp(5, "spell_aura_options.ProcCharges")?,
        spell_procs_per_minute_id: row.get_like_cpp(6, "spell_aura_options.SpellProcsPerMinuteID")?,
        proc_type_mask: numeric_array::<i32, 2>(row, 7, "spell_aura_options", "ProcTypeMask")?,
        spell_id: row.get_like_cpp(9, "spell_aura_options.SpellID")?,
    })
}

pub(super) fn spell_class_options(
    row: &[NativeFieldLikeCpp],
) -> Result<SpellClassOptionsHotfixRow> {
    Ok(SpellClassOptionsHotfixRow {
        id: row.get_like_cpp(0, "spell_class_options.ID")?,
        spell_id: row.get_like_cpp(1, "spell_class_options.SpellID")?,
        modal_next_spell: row.get_like_cpp(2, "spell_class_options.ModalNextSpell")?,
        spell_class_set: row.get_like_cpp(3, "spell_class_options.SpellClassSet")?,
        spell_class_mask: numeric_array::<i32, 4>(
            row,
            4,
            "spell_class_options",
            "SpellClassMask",
        )?,
    })
}

pub(super) fn spell_label(row: &[NativeFieldLikeCpp]) -> Result<SpellLabelHotfixRow> {
    Ok(SpellLabelHotfixRow {
        id: row.get_like_cpp(0, "spell_label.ID")?,
        label_id: row.get_like_cpp(1, "spell_label.LabelID")?,
        spell_id: row.get_like_cpp(2, "spell_label.SpellID")?,
    })
}

pub(super) fn spell_levels(row: &[NativeFieldLikeCpp]) -> Result<SpellLevelsHotfixRow> {
    Ok(SpellLevelsHotfixRow {
        id: row.get_like_cpp(0, "spell_levels.ID")?,
        difficulty_id: row.get_like_cpp(1, "spell_levels.DifficultyID")?,
        base_level: row.get_like_cpp(2, "spell_levels.BaseLevel")?,
        max_level: row.get_like_cpp(3, "spell_levels.MaxLevel")?,
        spell_level: row.get_like_cpp(4, "spell_levels.SpellLevel")?,
        max_passive_aura_level: row.get_like_cpp(5, "spell_levels.MaxPassiveAuraLevel")?,
        spell_id: row.get_like_cpp(6, "spell_levels.SpellID")?,
    })
}

pub(super) fn spell_procs_per_minute(
    row: &[NativeFieldLikeCpp],
) -> Result<SpellProcsPerMinuteHotfixRow> {
    Ok(SpellProcsPerMinuteHotfixRow {
        id: row.get_like_cpp(0, "spell_procs_per_minute.ID")?,
        base_proc_rate: row.get_like_cpp(1, "spell_procs_per_minute.BaseProcRate")?,
        flags: row.get_like_cpp(2, "spell_procs_per_minute.Flags")?,
    })
}

#[cfg(test)]
#[path = "decode_tests.rs"]
mod tests;
