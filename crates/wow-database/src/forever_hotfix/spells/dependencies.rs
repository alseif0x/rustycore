//! Exact source SQL column order; no string loss or numeric width projection.
use super::{LoadError, SpellRow};
use crate::SqlResult;
use wow_persistence::forever::spells::{
    BattlePetSpeciesRow, DifficultyRow, SpellCastTimesRow, SpellCategoryRow, SpellDurationRow,
    SpellLearnSpellRow, SpellProcsPerMinuteModRow, SpellProcsPerMinuteRow, SpellRadiusRow,
    SpellRangeRow, SpellShapeshiftFormRow, SummonPropertiesRow,
};

pub(super) fn difficulty(result: &SqlResult) -> Result<DifficultyRow, LoadError> {
    let mut r = SpellRow::new(result, 15)?;
    let row = DifficultyRow {
        id: r.read()?,
        name: r.text()?,
        instance_type: r.read()?,
        order_index: r.read()?,
        old_enum_value: r.read()?,
        fallback_difficulty_id: r.read()?,
        min_players: r.read()?,
        max_players: r.read()?,
        flags: r.read()?,
        item_context: r.read()?,
        toggle_difficulty_id: r.read()?,
        group_size_health_curve_id: r.read()?,
        group_size_dmg_curve_id: r.read()?,
        group_size_spell_points_curve_id: r.read()?,
        unknown1105: r.read()?,
    };
    r.finish()?;
    Ok(row)
}

pub(super) fn spell_cast_times(result: &SqlResult) -> Result<SpellCastTimesRow, LoadError> {
    let mut r = SpellRow::new(result, 3)?;
    let row = SpellCastTimesRow {
        id: r.read()?,
        base: r.read()?,
        minimum: r.read()?,
    };
    r.finish()?;
    Ok(row)
}

pub(super) fn spell_duration(result: &SqlResult) -> Result<SpellDurationRow, LoadError> {
    let mut r = SpellRow::new(result, 4)?;
    let row = SpellDurationRow {
        id: r.read()?,
        duration: r.read()?,
        max_duration: r.read()?,
        duration_per_resource: r.read()?,
    };
    r.finish()?;
    Ok(row)
}

pub(super) fn spell_range(result: &SqlResult) -> Result<SpellRangeRow, LoadError> {
    let mut r = SpellRow::new(result, 8)?;
    let row = SpellRangeRow {
        id: r.read()?,
        display_name: r.text()?,
        display_name_short: r.text()?,
        flags: r.read()?,
        range_min: r.array()?,
        range_max: r.array()?,
    };
    r.finish()?;
    Ok(row)
}

pub(super) fn spell_radius(result: &SqlResult) -> Result<SpellRadiusRow, LoadError> {
    let mut r = SpellRow::new(result, 5)?;
    let row = SpellRadiusRow {
        id: r.read()?,
        radius: r.read()?,
        radius_per_level: r.read()?,
        radius_min: r.read()?,
        radius_max: r.read()?,
    };
    r.finish()?;
    Ok(row)
}

pub(super) fn spell_procs_per_minute(
    result: &SqlResult,
) -> Result<SpellProcsPerMinuteRow, LoadError> {
    let mut r = SpellRow::new(result, 3)?;
    let row = SpellProcsPerMinuteRow {
        id: r.read()?,
        base_proc_rate: r.read()?,
        flags: r.read()?,
    };
    r.finish()?;
    Ok(row)
}

pub(super) fn spell_procs_per_minute_mod(
    result: &SqlResult,
) -> Result<SpellProcsPerMinuteModRow, LoadError> {
    let mut r = SpellRow::new(result, 6)?;
    let row = SpellProcsPerMinuteModRow {
        id: r.read()?,
        r#type: r.read()?,
        param: r.read()?,
        coeff: r.read()?,
        field_12_1_5_69594_003: r.read()?,
        spell_procs_per_minute_id: r.read()?,
    };
    r.finish()?;
    Ok(row)
}

pub(super) fn spell_learn_spell(result: &SqlResult) -> Result<SpellLearnSpellRow, LoadError> {
    let mut r = SpellRow::new(result, 4)?;
    let row = SpellLearnSpellRow {
        id: r.read()?,
        spell_id: r.read()?,
        learn_spell_id: r.read()?,
        overrides_spell_id: r.read()?,
    };
    r.finish()?;
    Ok(row)
}

pub(super) fn spell_shapeshift_form(
    result: &SqlResult,
) -> Result<SpellShapeshiftFormRow, LoadError> {
    let mut r = SpellRow::new(result, 18)?;
    let row = SpellShapeshiftFormRow {
        id: r.read()?,
        name: r.text()?,
        creature_display_id: r.read()?,
        creature_type: r.read()?,
        flags: r.read()?,
        attack_icon_file_id: r.read()?,
        bonus_action_bar: r.read()?,
        combat_round_time: r.read()?,
        damage_variance: r.read()?,
        mount_type_id: r.read()?,
        preset_spell_id: r.array()?,
    };
    r.finish()?;
    Ok(row)
}

pub(super) fn summon_properties(result: &SqlResult) -> Result<SummonPropertiesRow, LoadError> {
    let mut r = SpellRow::new(result, 7)?;
    let row = SummonPropertiesRow {
        id: r.read()?,
        control: r.read()?,
        faction: r.read()?,
        title: r.read()?,
        slot: r.read()?,
        flags: r.array()?,
    };
    r.finish()?;
    Ok(row)
}

pub(super) fn battle_pet_species(result: &SqlResult) -> Result<BattlePetSpeciesRow, LoadError> {
    let mut r = SpellRow::new(result, 12)?;
    let row = BattlePetSpeciesRow {
        description: r.text()?,
        source_text: r.text()?,
        id: r.read()?,
        creature_id: r.read()?,
        summon_spell_id: r.read()?,
        icon_file_data_id: r.read()?,
        pet_type_enum: r.read()?,
        flags: r.read()?,
        source_type_enum: r.read()?,
        card_ui_model_scene_id: r.read()?,
        loadout_ui_model_scene_id: r.read()?,
        covenant_id: r.read()?,
    };
    r.finish()?;
    Ok(row)
}

pub(super) fn spell_category(result: &SqlResult) -> Result<SpellCategoryRow, LoadError> {
    let mut r = SpellRow::new(result, 7)?;
    let row = SpellCategoryRow {
        id: r.read()?,
        name: r.text()?,
        flags: r.read()?,
        uses_per_week: r.read()?,
        max_charges: r.read()?,
        charge_recovery_time: r.read()?,
        type_mask: r.read()?,
    };
    r.finish()?;
    Ok(row)
}
