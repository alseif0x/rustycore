use super::super::super::*;

pub(super) fn spell_name(id: u32, locale: u8, bytes: &[u8]) -> SpellNameRecord {
    SpellNameRecord {
        id: id,
        name: SpellText::from_locale(locale, bytes.to_vec()).unwrap(),
    }
}

pub(super) fn difficulty(id: u32, locale: u8, bytes: &[u8]) -> DifficultyRecord {
    DifficultyRecord {
        id: id,
        name: SpellText::from_locale(locale, bytes.to_vec()).unwrap(),
        instance_type: 0,
        order_index: 0,
        old_enum_value: 0,
        fallback_difficulty_id: 0,
        min_players: 0,
        max_players: 0,
        flags: 0,
        item_context: 0,
        toggle_difficulty_id: 0,
        group_size_health_curve_id: 0,
        group_size_dmg_curve_id: 0,
        group_size_spell_points_curve_id: 0,
        unknown1105: 0,
    }
}

pub(super) fn spell_range(id: u32, locale: u8, bytes: &[u8]) -> SpellRangeRecord {
    SpellRangeRecord {
        id: id,
        display_name: SpellText::from_locale(locale, bytes.to_vec()).unwrap(),
        display_name_short: SpellText::from_locale(locale, bytes.to_vec()).unwrap(),
        flags: 0,
        range_min: [0; 2],
        range_max: [0; 2],
    }
}

pub(super) fn spell_shapeshift_form(
    id: u32,
    locale: u8,
    bytes: &[u8],
) -> SpellShapeshiftFormRecord {
    SpellShapeshiftFormRecord {
        id: id,
        name: SpellText::from_locale(locale, bytes.to_vec()).unwrap(),
        creature_display_id: 0,
        creature_type: 0,
        flags: 0,
        attack_icon_file_id: 0,
        bonus_action_bar: 0,
        combat_round_time: 0,
        damage_variance: 0.0,
        mount_type_id: 0,
        preset_spell_id: [0; 8],
    }
}

pub(super) fn battle_pet_species(id: u32, locale: u8, bytes: &[u8]) -> BattlePetSpeciesRecord {
    BattlePetSpeciesRecord {
        description: SpellText::from_locale(locale, bytes.to_vec()).unwrap(),
        source_text: SpellText::from_locale(locale, bytes.to_vec()).unwrap(),
        id: id,
        creature_id: 0,
        summon_spell_id: 0,
        icon_file_data_id: 0,
        pet_type_enum: 0,
        flags: 0,
        source_type_enum: 0,
        card_ui_model_scene_id: 0,
        loadout_ui_model_scene_id: 0,
        covenant_id: 0,
    }
}

pub(super) fn spell_category(id: u32, locale: u8, bytes: &[u8]) -> SpellCategoryRecord {
    SpellCategoryRecord {
        id: id,
        name: SpellText::from_locale(locale, bytes.to_vec()).unwrap(),
        flags: 0,
        uses_per_week: 0,
        max_charges: 0,
        charge_recovery_time: 0,
        type_mask: 0,
    }
}
