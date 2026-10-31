use super::{
    SelectedCatalogError, resolve_selected_value, selected_aura_options_entry,
    selected_spell_attributes, selected_spell_info_key_difficulty, selected_spell_level,
    selected_spell_misc_entry,
};
use wow_data::{
    DifficultyEntry, DifficultyStore, SpellAuraOptionsEntry, SpellAuraOptionsStore,
    SpellLevelsEntry, SpellLevelsStore, SpellMiscEntry, SpellMiscStore, SpellStore,
};

const SPELL_ID: u32 = 41_001;
const OTHER_SPELL_ID: u32 = 41_002;

fn difficulty(id: u32, fallback_difficulty_id: u8) -> DifficultyEntry {
    DifficultyEntry {
        id,
        instance_type: 0,
        flags: 0,
        fallback_difficulty_id,
        toggle_difficulty_id: 0,
    }
}

fn options_row(
    id: u32,
    difficulty_id: u8,
    spell_id: u32,
    cumulative_aura: u32,
) -> SpellAuraOptionsEntry {
    SpellAuraOptionsEntry {
        id,
        difficulty_id,
        cumulative_aura,
        proc_category_recovery: 0,
        proc_chance: 0,
        proc_charges: 0,
        spell_procs_per_minute_id: 0,
        proc_type_mask: [0; 2],
        spell_id,
    }
}

fn levels_row(id: u32, difficulty_id: u8, spell_id: u32, spell_level: i16) -> SpellLevelsEntry {
    SpellLevelsEntry {
        id,
        difficulty_id,
        base_level: 0,
        max_level: 0,
        spell_level,
        max_passive_aura_level: 0,
        spell_id,
    }
}

fn misc_row(id: u32, difficulty_id: u8, spell_id: u32) -> SpellMiscEntry {
    SpellMiscEntry {
        id,
        difficulty_id,
        spell_id,
        spell_icon_file_data_id: id as i32,
        ..SpellMiscEntry::default()
    }
}

fn attributes_store(spell_id: u32, rows: &[(u8, [u32; 15])]) -> SpellStore {
    let mut store = SpellStore::new();
    for (difficulty_id, attributes) in rows {
        store.insert_spell_misc_attributes_for_difficulty_like_cpp(
            spell_id as i32,
            *difficulty_id,
            *attributes,
        );
    }
    store
}

#[test]
fn selected_fields_follow_4_to_3_to_0_and_choose_highest_record_ids() {
    let mut base_attributes = [0; 15];
    base_attributes[5] = 0x400;
    let spells = attributes_store(SPELL_ID, &[(0, base_attributes)]);
    let options = SpellAuraOptionsStore::from_entries([
        options_row(10, 0, SPELL_ID, 1),
        options_row(20, 0, SPELL_ID, 2),
    ]);
    let levels = SpellLevelsStore::from_entries([
        levels_row(10, 0, SPELL_ID, 12),
        levels_row(20, 0, SPELL_ID, -1),
    ]);
    let misc = SpellMiscStore::from_entries([misc_row(10, 0, SPELL_ID), misc_row(20, 0, SPELL_ID)]);
    let difficulties =
        DifficultyStore::from_entries([difficulty(4, 3), difficulty(3, 0), difficulty(0, 0)]);

    assert_eq!(
        selected_spell_attributes(&spells, SPELL_ID as i32, 4, Some(&difficulties)),
        Ok(Some(base_attributes)),
    );
    assert_eq!(
        selected_aura_options_entry(&options, SPELL_ID, 4, Some(&difficulties))
            .unwrap()
            .map(|entry| (entry.id, entry.cumulative_aura)),
        Some((20, 2)),
    );
    assert_eq!(
        selected_spell_level(&levels, SPELL_ID, 4, Some(&difficulties)),
        Ok(u8::MAX),
    );
    assert_eq!(
        selected_spell_misc_entry(&misc, SPELL_ID, 4, Some(&difficulties))
            .unwrap()
            .map(|entry| entry.id),
        Some(20),
    );
}

#[test]
fn selected_intermediate_rows_win_over_base_and_choose_highest_record_ids() {
    let mut intermediate_attributes = [0; 15];
    intermediate_attributes[11] = 0x80;
    let mut base_attributes = [0; 15];
    base_attributes[11] = 0x40;
    let spells = attributes_store(
        SPELL_ID,
        &[(0, base_attributes), (3, intermediate_attributes)],
    );
    let options = SpellAuraOptionsStore::from_entries([
        options_row(30, 3, SPELL_ID, 3),
        options_row(31, 3, SPELL_ID, 4),
        options_row(40, 0, SPELL_ID, 5),
    ]);
    let levels = SpellLevelsStore::from_entries([
        levels_row(30, 3, SPELL_ID, 33),
        levels_row(31, 3, SPELL_ID, 34),
        levels_row(40, 0, SPELL_ID, 40),
    ]);
    let misc = SpellMiscStore::from_entries([
        misc_row(30, 3, SPELL_ID),
        misc_row(31, 3, SPELL_ID),
        misc_row(40, 0, SPELL_ID),
    ]);
    let difficulties =
        DifficultyStore::from_entries([difficulty(4, 3), difficulty(3, 0), difficulty(0, 0)]);

    assert_eq!(
        selected_spell_attributes(&spells, SPELL_ID as i32, 4, Some(&difficulties)),
        Ok(Some(intermediate_attributes)),
    );
    assert_eq!(
        selected_aura_options_entry(&options, SPELL_ID, 4, Some(&difficulties))
            .unwrap()
            .map(|entry| (entry.id, entry.cumulative_aura)),
        Some((31, 4)),
    );
    assert_eq!(
        selected_spell_level(&levels, SPELL_ID, 4, Some(&difficulties)),
        Ok(34),
    );
    assert_eq!(
        selected_spell_misc_entry(&misc, SPELL_ID, 4, Some(&difficulties))
            .unwrap()
            .map(|entry| entry.id),
        Some(31),
    );
}

#[test]
fn selected_zero_difficulty_follows_its_known_fallback_even_without_row_three() {
    let mut attributes = [0; 15];
    attributes[5] = 0x20;
    let spells = attributes_store(SPELL_ID, &[(3, attributes)]);
    let options = SpellAuraOptionsStore::from_entries([options_row(3, 3, SPELL_ID, 7)]);
    let levels = SpellLevelsStore::from_entries([levels_row(3, 3, SPELL_ID, 71)]);
    let misc = SpellMiscStore::from_entries([misc_row(3, 3, SPELL_ID)]);
    let difficulties = DifficultyStore::from_entries([difficulty(0, 3)]);

    assert_eq!(
        selected_spell_attributes(&spells, SPELL_ID as i32, 0, Some(&difficulties)),
        Ok(Some(attributes)),
    );
    assert_eq!(
        selected_aura_options_entry(&options, SPELL_ID, 0, Some(&difficulties))
            .unwrap()
            .map(|entry| entry.id),
        Some(3),
    );
    assert_eq!(
        selected_spell_level(&levels, SPELL_ID, 0, Some(&difficulties)),
        Ok(71),
    );
    assert_eq!(
        selected_spell_misc_entry(&misc, SPELL_ID, 0, Some(&difficulties))
            .unwrap()
            .map(|entry| entry.id),
        Some(3),
    );
}

#[test]
fn known_missing_difficulty_stops_before_base_rows_and_unknown_graph_errors() {
    let spells = attributes_store(SPELL_ID, &[(0, [0; 15])]);
    let options = SpellAuraOptionsStore::from_entries([options_row(1, 0, SPELL_ID, 8)]);
    let levels = SpellLevelsStore::from_entries([levels_row(1, 0, SPELL_ID, 88)]);
    let misc = SpellMiscStore::from_entries([misc_row(1, 0, SPELL_ID)]);
    let difficulties = DifficultyStore::from_entries([difficulty(0, 0)]);

    assert_eq!(
        selected_spell_attributes(&spells, SPELL_ID as i32, 4, Some(&difficulties)),
        Ok(None),
    );
    assert_eq!(
        selected_aura_options_entry(&options, SPELL_ID, 4, Some(&difficulties)).unwrap(),
        None,
    );
    assert_eq!(
        selected_spell_level(&levels, SPELL_ID, 4, Some(&difficulties)),
        Ok(0),
    );
    assert_eq!(
        selected_spell_misc_entry(&misc, SPELL_ID, 4, Some(&difficulties)).unwrap(),
        None,
    );

    let attributes_elsewhere = attributes_store(SPELL_ID, &[(3, [0; 15])]);
    let options_elsewhere = SpellAuraOptionsStore::from_entries([options_row(3, 3, SPELL_ID, 8)]);
    let levels_elsewhere = SpellLevelsStore::from_entries([levels_row(3, 3, SPELL_ID, 88)]);
    let misc_elsewhere = SpellMiscStore::from_entries([misc_row(3, 3, SPELL_ID)]);
    assert_eq!(
        selected_spell_attributes(&spells, SPELL_ID as i32, 4, None),
        Err(SelectedCatalogError::DifficultyUnavailable),
    );
    assert_eq!(
        selected_aura_options_entry(&options, SPELL_ID, 4, None),
        Err(SelectedCatalogError::DifficultyUnavailable),
    );
    assert_eq!(
        selected_spell_level(&levels, SPELL_ID, 4, None),
        Err(SelectedCatalogError::DifficultyUnavailable),
    );
    assert_eq!(
        selected_spell_misc_entry(&misc, SPELL_ID, 4, None),
        Err(SelectedCatalogError::DifficultyUnavailable),
    );
    assert_eq!(
        selected_spell_attributes(&attributes_elsewhere, SPELL_ID as i32, 0, None),
        Err(SelectedCatalogError::DifficultyUnavailable),
    );
    assert_eq!(
        selected_aura_options_entry(&options_elsewhere, SPELL_ID, 0, None),
        Err(SelectedCatalogError::DifficultyUnavailable),
    );
    assert_eq!(
        selected_spell_level(&levels_elsewhere, SPELL_ID, 0, None),
        Err(SelectedCatalogError::DifficultyUnavailable),
    );
    assert_eq!(
        selected_spell_misc_entry(&misc_elsewhere, SPELL_ID, 0, None),
        Err(SelectedCatalogError::DifficultyUnavailable),
    );
}

#[test]
fn absent_field_for_this_spell_is_known_without_a_difficulty_store() {
    let spells = attributes_store(SPELL_ID, &[(3, [0; 15])]);
    let options = SpellAuraOptionsStore::from_entries([options_row(1, 3, SPELL_ID, 8)]);
    let levels = SpellLevelsStore::from_entries([levels_row(1, 3, SPELL_ID, 88)]);
    let misc = SpellMiscStore::from_entries([misc_row(1, 3, SPELL_ID)]);
    let empty_spells = SpellStore::new();
    let empty_options = SpellAuraOptionsStore::from_entries([]);
    let empty_levels = SpellLevelsStore::from_entries([]);
    let empty_misc = SpellMiscStore::from_entries([]);

    assert_eq!(
        selected_spell_attributes(&empty_spells, OTHER_SPELL_ID as i32, 0, None),
        Ok(None),
    );
    assert_eq!(
        selected_aura_options_entry(&empty_options, OTHER_SPELL_ID, 0, None).unwrap(),
        None,
    );
    assert_eq!(
        selected_spell_level(&empty_levels, OTHER_SPELL_ID, 0, None),
        Ok(0),
    );
    assert_eq!(
        selected_spell_misc_entry(&empty_misc, OTHER_SPELL_ID, 0, None).unwrap(),
        None,
    );

    let difficulties = DifficultyStore::from_entries([difficulty(0, 3), difficulty(3, 0)]);
    assert_eq!(
        selected_spell_attributes(&spells, OTHER_SPELL_ID as i32, 0, Some(&difficulties)),
        Ok(None),
    );
    assert_eq!(
        selected_aura_options_entry(&options, OTHER_SPELL_ID, 0, Some(&difficulties)).unwrap(),
        None,
    );
    assert_eq!(
        selected_spell_level(&levels, OTHER_SPELL_ID, 0, Some(&difficulties)),
        Ok(0),
    );
    assert_eq!(
        selected_spell_misc_entry(&misc, OTHER_SPELL_ID, 0, Some(&difficulties)).unwrap(),
        None,
    );
}

#[test]
fn required_zero_to_three_to_zero_cycle_is_reported_for_fields_with_other_rows() {
    let spells = attributes_store(SPELL_ID, &[(7, [0; 15])]);
    let options = SpellAuraOptionsStore::from_entries([options_row(1, 7, SPELL_ID, 8)]);
    let levels = SpellLevelsStore::from_entries([levels_row(1, 7, SPELL_ID, 88)]);
    let misc = SpellMiscStore::from_entries([misc_row(1, 7, SPELL_ID)]);
    let difficulties = DifficultyStore::from_entries([difficulty(0, 3), difficulty(3, 0)]);

    assert_eq!(
        selected_spell_attributes(&spells, SPELL_ID as i32, 0, Some(&difficulties)),
        Err(SelectedCatalogError::DifficultyCycle),
    );
    assert_eq!(
        selected_aura_options_entry(&options, SPELL_ID, 0, Some(&difficulties)),
        Err(SelectedCatalogError::DifficultyCycle),
    );
    assert_eq!(
        selected_spell_level(&levels, SPELL_ID, 0, Some(&difficulties)),
        Err(SelectedCatalogError::DifficultyCycle),
    );
    assert_eq!(
        selected_spell_misc_entry(&misc, SPELL_ID, 0, Some(&difficulties)),
        Err(SelectedCatalogError::DifficultyCycle),
    );

    assert_eq!(
        selected_spell_attributes(&spells, OTHER_SPELL_ID as i32, 0, Some(&difficulties)),
        Ok(None),
    );
    assert_eq!(
        selected_aura_options_entry(&options, OTHER_SPELL_ID, 0, Some(&difficulties)).unwrap(),
        None,
    );
    assert_eq!(
        selected_spell_level(&levels, OTHER_SPELL_ID, 0, Some(&difficulties)),
        Ok(0),
    );
    assert_eq!(
        selected_spell_misc_entry(&misc, OTHER_SPELL_ID, 0, Some(&difficulties)).unwrap(),
        None,
    );
}

#[test]
fn exact_field_rows_do_not_need_an_irrelevant_difficulty_graph() {
    let spells = attributes_store(SPELL_ID, &[(4, [0; 15])]);
    let options = SpellAuraOptionsStore::from_entries([options_row(1, 4, SPELL_ID, 8)]);
    let levels = SpellLevelsStore::from_entries([levels_row(1, 4, SPELL_ID, 88)]);
    let misc = SpellMiscStore::from_entries([misc_row(1, 4, SPELL_ID)]);

    assert_eq!(
        selected_spell_attributes(&spells, SPELL_ID as i32, 4, None),
        Ok(Some([0; 15])),
    );
    assert_eq!(
        selected_aura_options_entry(&options, SPELL_ID, 4, None)
            .unwrap()
            .map(|entry| entry.id),
        Some(1),
    );
    assert_eq!(selected_spell_level(&levels, SPELL_ID, 4, None), Ok(88));
    assert_eq!(
        selected_spell_misc_entry(&misc, SPELL_ID, 4, None)
            .unwrap()
            .map(|entry| entry.id),
        Some(1),
    );
}

#[test]
fn regular_spell_info_key_walk_follows_fallback_and_keeps_exact_reads_independent() {
    let difficulties =
        DifficultyStore::from_entries([difficulty(4, 3), difficulty(3, 0), difficulty(0, 0)]);
    let key_difficulties = [0_u8];
    let spells_without_keys = SpellStore::new();

    assert_eq!(
        selected_spell_info_key_difficulty(&spells_without_keys, SPELL_ID, 0, None),
        Ok(None),
    );

    assert_eq!(
        resolve_selected_value(4, Some(&difficulties), true, |difficulty_id| {
            key_difficulties
                .contains(&difficulty_id)
                .then_some(difficulty_id)
        }),
        Ok(Some(0)),
    );
    let zero_fallbacks_to_unlisted_three = DifficultyStore::from_entries([difficulty(0, 3)]);
    assert_eq!(
        resolve_selected_value(
            0,
            Some(&zero_fallbacks_to_unlisted_three),
            true,
            |difficulty_id| (difficulty_id == 3).then_some(difficulty_id),
        ),
        Ok(Some(3)),
    );
    assert_eq!(
        resolve_selected_value(4, None, true, |difficulty_id| {
            (difficulty_id == 4).then_some(difficulty_id)
        }),
        Ok(Some(4)),
    );

    let known_missing_requested_difficulty = DifficultyStore::from_entries([difficulty(0, 0)]);
    assert_eq!(
        resolve_selected_value(
            4,
            Some(&known_missing_requested_difficulty),
            true,
            |difficulty_id| { (difficulty_id == 0).then_some(difficulty_id) }
        ),
        Ok(None),
    );
    assert_eq!(
        resolve_selected_value(0, None, true, |difficulty_id| {
            (difficulty_id == 3).then_some(difficulty_id)
        }),
        Err(SelectedCatalogError::DifficultyUnavailable),
    );
}
