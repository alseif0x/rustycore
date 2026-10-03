//! One-time source-width SQL DTO conversion; no retained raw mirror.
use anyhow::Result;
use wow_data::forever_spells::{
    BattlePetSpeciesRecord, DifficultyRecord, SpellCastTimesRecord, SpellCategoryRecord,
    SpellDurationRecord, SpellLearnSpellRecord, SpellProcsPerMinuteModRecord,
    SpellProcsPerMinuteRecord, SpellRadiusRecord, SpellRangeRecord, SpellShapeshiftFormRecord,
    SpellText, SummonPropertiesRecord,
};
use wow_persistence::forever::spells::{
    BattlePetSpeciesRow, DifficultyRow, SpellCastTimesRow, SpellCategoryRow, SpellDurationRow,
    SpellLearnSpellRow, SpellProcsPerMinuteModRow, SpellProcsPerMinuteRow, SpellRadiusRow,
    SpellRangeRow, SpellShapeshiftFormRow, SummonPropertiesRow,
};

pub(super) fn difficulty(row: DifficultyRow) -> Result<DifficultyRecord> {
    Ok(DifficultyRecord {
        id: row.id,
        name: SpellText::from_locale(0, row.name)?,
        instance_type: row.instance_type,
        order_index: row.order_index,
        old_enum_value: row.old_enum_value,
        fallback_difficulty_id: row.fallback_difficulty_id,
        min_players: row.min_players,
        max_players: row.max_players,
        flags: row.flags,
        item_context: row.item_context,
        toggle_difficulty_id: row.toggle_difficulty_id,
        group_size_health_curve_id: row.group_size_health_curve_id,
        group_size_dmg_curve_id: row.group_size_dmg_curve_id,
        group_size_spell_points_curve_id: row.group_size_spell_points_curve_id,
        unknown1105: row.unknown1105,
    })
}

pub(super) fn spell_cast_times(row: SpellCastTimesRow) -> Result<SpellCastTimesRecord> {
    Ok(SpellCastTimesRecord {
        id: row.id,
        base: row.base,
        minimum: row.minimum,
    })
}

pub(super) fn spell_duration(row: SpellDurationRow) -> Result<SpellDurationRecord> {
    Ok(SpellDurationRecord {
        id: row.id,
        duration: row.duration,
        max_duration: row.max_duration,
        duration_per_resource: row.duration_per_resource,
    })
}

pub(super) fn spell_range(row: SpellRangeRow) -> Result<SpellRangeRecord> {
    Ok(SpellRangeRecord {
        id: row.id,
        display_name: SpellText::from_locale(0, row.display_name)?,
        display_name_short: SpellText::from_locale(0, row.display_name_short)?,
        flags: row.flags,
        range_min: row.range_min,
        range_max: row.range_max,
    })
}

pub(super) fn spell_radius(row: SpellRadiusRow) -> Result<SpellRadiusRecord> {
    Ok(SpellRadiusRecord {
        id: row.id,
        radius: row.radius,
        radius_per_level: row.radius_per_level,
        radius_min: row.radius_min,
        radius_max: row.radius_max,
    })
}

pub(super) fn spell_procs_per_minute(
    row: SpellProcsPerMinuteRow,
) -> Result<SpellProcsPerMinuteRecord> {
    Ok(SpellProcsPerMinuteRecord {
        id: row.id,
        base_proc_rate: row.base_proc_rate,
        flags: row.flags,
    })
}

pub(super) fn spell_procs_per_minute_mod(
    row: SpellProcsPerMinuteModRow,
) -> Result<SpellProcsPerMinuteModRecord> {
    Ok(SpellProcsPerMinuteModRecord {
        id: row.id,
        r#type: row.r#type,
        param: row.param,
        coeff: row.coeff,
        field_12_1_5_69594_003: row.field_12_1_5_69594_003,
        spell_procs_per_minute_id: row.spell_procs_per_minute_id,
    })
}

pub(super) fn spell_learn_spell(row: SpellLearnSpellRow) -> Result<SpellLearnSpellRecord> {
    Ok(SpellLearnSpellRecord {
        id: row.id,
        spell_id: row.spell_id,
        learn_spell_id: row.learn_spell_id,
        overrides_spell_id: row.overrides_spell_id,
    })
}

pub(super) fn spell_shapeshift_form(
    row: SpellShapeshiftFormRow,
) -> Result<SpellShapeshiftFormRecord> {
    Ok(SpellShapeshiftFormRecord {
        id: row.id,
        name: SpellText::from_locale(0, row.name)?,
        creature_display_id: row.creature_display_id,
        creature_type: row.creature_type,
        flags: row.flags,
        attack_icon_file_id: row.attack_icon_file_id,
        bonus_action_bar: row.bonus_action_bar,
        combat_round_time: row.combat_round_time,
        damage_variance: row.damage_variance,
        mount_type_id: row.mount_type_id,
        preset_spell_id: row.preset_spell_id,
    })
}

pub(super) fn summon_properties(row: SummonPropertiesRow) -> Result<SummonPropertiesRecord> {
    Ok(SummonPropertiesRecord {
        id: row.id,
        control: row.control,
        faction: row.faction,
        title: row.title,
        slot: row.slot,
        flags: row.flags,
    })
}

pub(super) fn battle_pet_species(row: BattlePetSpeciesRow) -> Result<BattlePetSpeciesRecord> {
    Ok(BattlePetSpeciesRecord {
        description: SpellText::from_locale(0, row.description)?,
        source_text: SpellText::from_locale(0, row.source_text)?,
        id: row.id,
        creature_id: row.creature_id,
        summon_spell_id: row.summon_spell_id,
        icon_file_data_id: row.icon_file_data_id,
        pet_type_enum: row.pet_type_enum,
        flags: row.flags,
        source_type_enum: row.source_type_enum,
        card_ui_model_scene_id: row.card_ui_model_scene_id,
        loadout_ui_model_scene_id: row.loadout_ui_model_scene_id,
        covenant_id: row.covenant_id,
    })
}

pub(super) fn spell_category(row: SpellCategoryRow) -> Result<SpellCategoryRecord> {
    Ok(SpellCategoryRecord {
        id: row.id,
        name: SpellText::from_locale(0, row.name)?,
        flags: row.flags,
        uses_per_week: row.uses_per_week,
        max_charges: row.max_charges,
        charge_recovery_time: row.charge_recovery_time,
        type_mask: row.type_mask,
    })
}
