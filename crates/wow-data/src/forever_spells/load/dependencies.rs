use super::super::*;
use crate::wdc4::creation::CreationDb2;
use anyhow::Result;

pub(super) fn difficulty(table: &CreationDb2, id: u32) -> Result<DifficultyRecord> {
    Ok(DifficultyRecord {
        id: id,
        name: SpellText::from_locale(6, table.spell_text(id, 0)?)?,
        instance_type: table.bits(id, 1, 0)? as u8,
        order_index: table.bits(id, 2, 0)? as u8,
        old_enum_value: table.bits(id, 3, 0)? as i8,
        fallback_difficulty_id: table.bits(id, 4, 0)? as i16,
        min_players: table.bits(id, 5, 0)? as u8,
        max_players: table.bits(id, 6, 0)? as u8,
        flags: table.bits(id, 7, 0)? as i32,
        item_context: table.bits(id, 8, 0)? as u8,
        toggle_difficulty_id: table.bits(id, 9, 0)? as i16,
        group_size_health_curve_id: table.bits(id, 10, 0)?,
        group_size_dmg_curve_id: table.bits(id, 11, 0)?,
        group_size_spell_points_curve_id: table.bits(id, 12, 0)?,
        unknown1105: table.bits(id, 13, 0)? as i32,
    })
}

pub(super) fn spell_cast_times(table: &CreationDb2, id: u32) -> Result<SpellCastTimesRecord> {
    Ok(SpellCastTimesRecord {
        id: id,
        base: table.bits(id, 0, 0)? as i32,
        minimum: table.bits(id, 1, 0)? as i32,
    })
}

pub(super) fn spell_duration(table: &CreationDb2, id: u32) -> Result<SpellDurationRecord> {
    Ok(SpellDurationRecord {
        id: id,
        duration: table.bits(id, 0, 0)? as i32,
        max_duration: table.bits(id, 1, 0)? as i32,
        duration_per_resource: table.bits(id, 2, 0)? as i32,
    })
}

pub(super) fn spell_range(table: &CreationDb2, id: u32) -> Result<SpellRangeRecord> {
    Ok(SpellRangeRecord {
        id: id,
        display_name: SpellText::from_locale(6, table.spell_text(id, 0)?)?,
        display_name_short: SpellText::from_locale(6, table.spell_text(id, 1)?)?,
        flags: table.bits(id, 2, 0)? as i32,
        range_min: [
            f32::from_bits(table.bits(id, 3, 0)?),
            f32::from_bits(table.bits(id, 3, 1)?),
        ],
        range_max: [
            f32::from_bits(table.bits(id, 4, 0)?),
            f32::from_bits(table.bits(id, 4, 1)?),
        ],
    })
}

pub(super) fn spell_radius(table: &CreationDb2, id: u32) -> Result<SpellRadiusRecord> {
    Ok(SpellRadiusRecord {
        id: id,
        radius: f32::from_bits(table.bits(id, 0, 0)?),
        radius_per_level: f32::from_bits(table.bits(id, 1, 0)?),
        radius_min: f32::from_bits(table.bits(id, 2, 0)?),
        radius_max: f32::from_bits(table.bits(id, 3, 0)?),
    })
}

pub(super) fn spell_procs_per_minute(
    table: &CreationDb2,
    id: u32,
) -> Result<SpellProcsPerMinuteRecord> {
    Ok(SpellProcsPerMinuteRecord {
        id: id,
        base_proc_rate: f32::from_bits(table.bits(id, 0, 0)?),
        flags: table.bits(id, 1, 0)? as i32,
    })
}

pub(super) fn spell_procs_per_minute_mod(
    table: &CreationDb2,
    id: u32,
) -> Result<SpellProcsPerMinuteModRecord> {
    Ok(SpellProcsPerMinuteModRecord {
        id: id,
        r#type: table.bits(id, 0, 0)? as i32,
        param: table.bits(id, 1, 0)? as i32,
        coeff: f32::from_bits(table.bits(id, 2, 0)?),
        field_12_1_5_69594_003: table.bits(id, 3, 0)? as i32,
        spell_procs_per_minute_id: table.bits(id, 4, 0)?,
    })
}

pub(super) fn spell_learn_spell(table: &CreationDb2, id: u32) -> Result<SpellLearnSpellRecord> {
    Ok(SpellLearnSpellRecord {
        id: id,
        spell_id: table.bits(id, 0, 0)?,
        learn_spell_id: table.bits(id, 1, 0)? as i32,
        overrides_spell_id: table.bits(id, 2, 0)? as i32,
    })
}

pub(super) fn spell_shapeshift_form(
    table: &CreationDb2,
    id: u32,
) -> Result<SpellShapeshiftFormRecord> {
    Ok(SpellShapeshiftFormRecord {
        id: id,
        name: SpellText::from_locale(6, table.spell_text(id, 0)?)?,
        creature_display_id: table.bits(id, 1, 0)?,
        creature_type: table.bits(id, 2, 0)? as u8,
        flags: table.bits(id, 3, 0)? as i32,
        attack_icon_file_id: table.bits(id, 4, 0)? as i32,
        bonus_action_bar: table.bits(id, 5, 0)? as i8,
        combat_round_time: table.bits(id, 6, 0)? as i16,
        damage_variance: f32::from_bits(table.bits(id, 7, 0)?),
        mount_type_id: table.bits(id, 8, 0)? as u16,
        preset_spell_id: [
            table.bits(id, 9, 0)?,
            table.bits(id, 9, 1)?,
            table.bits(id, 9, 2)?,
            table.bits(id, 9, 3)?,
            table.bits(id, 9, 4)?,
            table.bits(id, 9, 5)?,
            table.bits(id, 9, 6)?,
            table.bits(id, 9, 7)?,
        ],
    })
}

pub(super) fn summon_properties(table: &CreationDb2, id: u32) -> Result<SummonPropertiesRecord> {
    Ok(SummonPropertiesRecord {
        id: id,
        control: table.bits(id, 0, 0)? as i32,
        faction: table.bits(id, 1, 0)? as i32,
        title: table.bits(id, 2, 0)? as i32,
        slot: table.bits(id, 3, 0)? as i32,
        flags: [table.bits(id, 4, 0)? as i32, table.bits(id, 4, 1)? as i32],
    })
}

pub(super) fn battle_pet_species(table: &CreationDb2, id: u32) -> Result<BattlePetSpeciesRecord> {
    Ok(BattlePetSpeciesRecord {
        description: SpellText::from_locale(6, table.spell_text(id, 0)?)?,
        source_text: SpellText::from_locale(6, table.spell_text(id, 1)?)?,
        id: id,
        creature_id: table.bits(id, 3, 0)? as i32,
        summon_spell_id: table.bits(id, 4, 0)? as i32,
        icon_file_data_id: table.bits(id, 5, 0)? as i32,
        pet_type_enum: table.bits(id, 6, 0)? as i8,
        flags: table.bits(id, 7, 0)? as i32,
        source_type_enum: table.bits(id, 8, 0)? as i8,
        card_ui_model_scene_id: table.bits(id, 9, 0)? as i32,
        loadout_ui_model_scene_id: table.bits(id, 10, 0)? as i32,
        covenant_id: table.bits(id, 11, 0)? as i32,
    })
}

pub(super) fn spell_category(table: &CreationDb2, id: u32) -> Result<SpellCategoryRecord> {
    Ok(SpellCategoryRecord {
        id: id,
        name: SpellText::from_locale(6, table.spell_text(id, 0)?)?,
        flags: table.bits(id, 1, 0)? as i32,
        uses_per_week: table.bits(id, 2, 0)? as i32,
        max_charges: table.bits(id, 3, 0)? as i32,
        charge_recovery_time: table.bits(id, 4, 0)? as i32,
        type_mask: table.bits(id, 5, 0)? as i32,
    })
}
