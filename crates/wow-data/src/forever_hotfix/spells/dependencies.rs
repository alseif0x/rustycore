//! 02245dcd DB2StorageBase::WriteRecord, source dependencies spell metadata.
use super::text;
use crate::forever_spells::{
    BattlePetSpeciesRecord, DifficultyRecord, SpellCastTimesRecord, SpellCategoryRecord,
    SpellDurationRecord, SpellLearnSpellRecord, SpellProcsPerMinuteModRecord,
    SpellProcsPerMinuteRecord, SpellRadiusRecord, SpellRangeRecord, SpellShapeshiftFormRecord,
    SummonPropertiesRecord,
};

pub(super) fn difficulty(row: &DifficultyRecord, locale: u8) -> Vec<u8> {
    let mut bytes = Vec::new();
    text(&mut bytes, &row.name, locale);
    bytes.extend_from_slice(&row.instance_type.to_le_bytes());
    bytes.extend_from_slice(&row.order_index.to_le_bytes());
    bytes.extend_from_slice(&row.old_enum_value.to_le_bytes());
    bytes.extend_from_slice(&row.fallback_difficulty_id.to_le_bytes());
    bytes.extend_from_slice(&row.min_players.to_le_bytes());
    bytes.extend_from_slice(&row.max_players.to_le_bytes());
    bytes.extend_from_slice(&row.flags.to_le_bytes());
    bytes.extend_from_slice(&row.item_context.to_le_bytes());
    bytes.extend_from_slice(&row.toggle_difficulty_id.to_le_bytes());
    bytes.extend_from_slice(&row.group_size_health_curve_id.to_le_bytes());
    bytes.extend_from_slice(&row.group_size_dmg_curve_id.to_le_bytes());
    bytes.extend_from_slice(&row.group_size_spell_points_curve_id.to_le_bytes());
    bytes.extend_from_slice(&row.unknown1105.to_le_bytes());
    bytes
}

pub(super) fn spell_cast_times(row: &SpellCastTimesRecord, _locale: u8) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&row.base.to_le_bytes());
    bytes.extend_from_slice(&row.minimum.to_le_bytes());
    bytes
}

pub(super) fn spell_duration(row: &SpellDurationRecord, _locale: u8) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&row.duration.to_le_bytes());
    bytes.extend_from_slice(&row.max_duration.to_le_bytes());
    bytes.extend_from_slice(&row.duration_per_resource.to_le_bytes());
    bytes
}

pub(super) fn spell_range(row: &SpellRangeRecord, locale: u8) -> Vec<u8> {
    let mut bytes = Vec::new();
    text(&mut bytes, &row.display_name, locale);
    text(&mut bytes, &row.display_name_short, locale);
    bytes.extend_from_slice(&row.flags.to_le_bytes());
    for value in row.range_min {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    for value in row.range_max {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes
}

pub(super) fn spell_radius(row: &SpellRadiusRecord, _locale: u8) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&row.radius.to_le_bytes());
    bytes.extend_from_slice(&row.radius_per_level.to_le_bytes());
    bytes.extend_from_slice(&row.radius_min.to_le_bytes());
    bytes.extend_from_slice(&row.radius_max.to_le_bytes());
    bytes
}

pub(super) fn spell_procs_per_minute(row: &SpellProcsPerMinuteRecord, _locale: u8) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&row.base_proc_rate.to_le_bytes());
    bytes.extend_from_slice(&row.flags.to_le_bytes());
    bytes
}

pub(super) fn spell_procs_per_minute_mod(
    row: &SpellProcsPerMinuteModRecord,
    _locale: u8,
) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&row.r#type.to_le_bytes());
    bytes.extend_from_slice(&row.param.to_le_bytes());
    bytes.extend_from_slice(&row.coeff.to_le_bytes());
    bytes.extend_from_slice(&row.field_12_1_5_69594_003.to_le_bytes());
    bytes.extend_from_slice(&row.spell_procs_per_minute_id.to_le_bytes());
    bytes
}

pub(super) fn spell_learn_spell(row: &SpellLearnSpellRecord, _locale: u8) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&row.spell_id.to_le_bytes());
    bytes.extend_from_slice(&row.learn_spell_id.to_le_bytes());
    bytes.extend_from_slice(&row.overrides_spell_id.to_le_bytes());
    bytes
}

pub(super) fn spell_shapeshift_form(row: &SpellShapeshiftFormRecord, locale: u8) -> Vec<u8> {
    let mut bytes = Vec::new();
    text(&mut bytes, &row.name, locale);
    bytes.extend_from_slice(&row.creature_display_id.to_le_bytes());
    bytes.extend_from_slice(&row.creature_type.to_le_bytes());
    bytes.extend_from_slice(&row.flags.to_le_bytes());
    bytes.extend_from_slice(&row.attack_icon_file_id.to_le_bytes());
    bytes.extend_from_slice(&row.bonus_action_bar.to_le_bytes());
    bytes.extend_from_slice(&row.combat_round_time.to_le_bytes());
    bytes.extend_from_slice(&row.damage_variance.to_le_bytes());
    bytes.extend_from_slice(&row.mount_type_id.to_le_bytes());
    for value in row.preset_spell_id {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes
}

pub(super) fn summon_properties(row: &SummonPropertiesRecord, _locale: u8) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&row.control.to_le_bytes());
    bytes.extend_from_slice(&row.faction.to_le_bytes());
    bytes.extend_from_slice(&row.title.to_le_bytes());
    bytes.extend_from_slice(&row.slot.to_le_bytes());
    for value in row.flags {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes
}

pub(super) fn battle_pet_species(row: &BattlePetSpeciesRecord, locale: u8) -> Vec<u8> {
    let mut bytes = Vec::new();
    text(&mut bytes, &row.description, locale);
    text(&mut bytes, &row.source_text, locale);
    bytes.extend_from_slice(&row.id.to_le_bytes());
    bytes.extend_from_slice(&row.creature_id.to_le_bytes());
    bytes.extend_from_slice(&row.summon_spell_id.to_le_bytes());
    bytes.extend_from_slice(&row.icon_file_data_id.to_le_bytes());
    bytes.extend_from_slice(&row.pet_type_enum.to_le_bytes());
    bytes.extend_from_slice(&row.flags.to_le_bytes());
    bytes.extend_from_slice(&row.source_type_enum.to_le_bytes());
    bytes.extend_from_slice(&row.card_ui_model_scene_id.to_le_bytes());
    bytes.extend_from_slice(&row.loadout_ui_model_scene_id.to_le_bytes());
    bytes.extend_from_slice(&row.covenant_id.to_le_bytes());
    bytes
}

pub(super) fn spell_category(row: &SpellCategoryRecord, locale: u8) -> Vec<u8> {
    let mut bytes = Vec::new();
    text(&mut bytes, &row.name, locale);
    bytes.extend_from_slice(&row.flags.to_le_bytes());
    bytes.extend_from_slice(&row.uses_per_week.to_le_bytes());
    bytes.extend_from_slice(&row.max_charges.to_le_bytes());
    bytes.extend_from_slice(&row.charge_recovery_time.to_le_bytes());
    bytes.extend_from_slice(&row.type_mask.to_le_bytes());
    bytes
}
