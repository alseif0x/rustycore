//! Source difficulty resolution for selected Aura publication facts.

use super::*;
use wow_core::{ObjectGuid, guid::HighGuid};
use wow_data::{DifficultyEntry, SpellAuraOptionsEntry, SpellAuraOptionsStore, SpellStore};

const SPELL_ID: i32 = 8_326;

fn application(difficulty_id: u8) -> AuraApplication {
    AuraApplication {
        spell_id: SPELL_ID,
        difficulty_id,
        caster_guid: ObjectGuid::create_player(1, 82_001),
        slot: 2,
        duration_total: 60_000,
        duration_remaining: 30_000,
        stack_count: 1,
        aura_flags: 0,
        effect_mask: 1,
        aura_interrupt_flags: 0,
        aura_interrupt_flags2: 0,
        represented_effect: None,
        represented_amount: 0,
        represented_effect_amounts: Vec::new(),
        represented_misc_value: None,
        represented_multiplier: 1.0,
        applied_at: std::time::Instant::now(),
    }
}

fn provenance() -> AuraCastProvenance {
    AuraCastProvenance {
        cast_id: ObjectGuid::create_world_object(HighGuid::Cast, 3, 1, 571, 0, SPELL_ID as u32, 52),
        spell_visual_id: 7,
        caster_level: Some(80),
    }
}

fn persistence() -> AuraPersistenceMetadata {
    AuraPersistenceMetadata {
        item_guid: ObjectGuid::EMPTY,
        cast_item_id: 0,
        cast_item_level: 0,
        recalculate_mask: 0,
        charges: 2,
        base_amounts: Vec::new(),
    }
}

fn options_row(id: u32, difficulty_id: u8, cumulative_aura: u32) -> SpellAuraOptionsEntry {
    SpellAuraOptionsEntry {
        id,
        difficulty_id,
        cumulative_aura,
        proc_category_recovery: 0,
        proc_chance: 0,
        proc_charges: 0,
        spell_procs_per_minute_id: 0,
        proc_type_mask: [0; 2],
        spell_id: SPELL_ID as u32,
    }
}

fn difficulty(id: u32, fallback_difficulty_id: u8) -> DifficultyEntry {
    DifficultyEntry {
        id,
        instance_type: 0,
        flags: 0,
        fallback_difficulty_id,
        toggle_difficulty_id: 0,
    }
}

#[test]
fn selected_publication_catalogs_follow_4_to_3_to_0_and_choose_highest_record_id() {
    let mut spells = SpellStore::new();
    let mut attributes = [0; 15];
    attributes[5] = 0x400;
    attributes[11] = 0x4;
    spells.insert_spell_misc_attributes_for_difficulty_like_cpp(SPELL_ID, 0, attributes);
    let options = SpellAuraOptionsStore::from_entries([options_row(3, 0, 2), options_row(9, 0, 7)]);
    let difficulties =
        DifficultyStore::from_entries([difficulty(4, 3), difficulty(3, 0), difficulty(0, 0)]);

    let facts = selected_aura_wire_facts(
        &application(4),
        &provenance(),
        Some(&persistence()),
        SelectedAuraPublicationCatalogs {
            races: None,
            spells: Some(&spells),
            aura_options: Some(&options),
            difficulties: Some(&difficulties),
        },
    )
    .expect("difficulty 4 resolves through 3 to 0");

    assert_eq!(facts.attributes5, 0x400);
    assert_eq!(facts.attributes11, 0x4);
    assert_eq!(facts.catalog_stack_amount, 7);
    assert_eq!(facts.captured_caster_level, 80);
    assert_eq!(facts.canonical_charges, 2);
}

#[test]
fn selected_publication_catalogs_stop_at_known_missing_difficulty_without_base_attributes() {
    let mut spells = SpellStore::new();
    let mut base_attributes = [0; 15];
    base_attributes[5] = 0x400;
    spells.insert_spell_misc_attributes_like_cpp(SPELL_ID, base_attributes);
    let options = SpellAuraOptionsStore::from_entries([]);
    let difficulties = DifficultyStore::from_entries([difficulty(0, 0)]);

    assert_eq!(
        selected_misc_attributes(&spells, SPELL_ID, 4, Some(&difficulties)).unwrap(),
        None,
        "a known store with no row for difficulty 4 stops before base attributes"
    );
    assert_eq!(
        selected_aura_options_entry(&options, SPELL_ID as u32, 4, Some(&difficulties)).unwrap(),
        None,
        "known options with no effective row resolve to zero stacks"
    );

    let mut spells_with_requested_row = SpellStore::new();
    spells_with_requested_row.insert_spell_misc_attributes_like_cpp(SPELL_ID, base_attributes);
    spells_with_requested_row
        .insert_spell_misc_attributes_for_difficulty_like_cpp(SPELL_ID, 4, [0; 15]);
    let facts = selected_aura_wire_facts(
        &application(4),
        &provenance(),
        Some(&persistence()),
        SelectedAuraPublicationCatalogs {
            races: None,
            spells: Some(&spells_with_requested_row),
            aura_options: Some(&options),
            difficulties: Some(&difficulties),
        },
    )
    .expect("a known empty options result contributes stack amount zero");
    assert_eq!(facts.catalog_stack_amount, 0);
}

#[test]
fn selected_publication_catalogs_hold_when_a_required_difficulty_store_is_unknown() {
    let mut spells = SpellStore::new();
    spells.insert_spell_misc_attributes_like_cpp(SPELL_ID, [0; 15]);
    let options = SpellAuraOptionsStore::from_entries([options_row(1, 3, 1)]);

    assert!(selected_misc_attributes(&spells, SPELL_ID, 4, None).is_err());
    assert!(selected_aura_options_entry(&options, SPELL_ID as u32, 4, None).is_err());

    let result = selected_aura_wire_facts(
        &application(4),
        &provenance(),
        Some(&persistence()),
        SelectedAuraPublicationCatalogs {
            races: None,
            spells: Some(&spells),
            aura_options: Some(&options),
            difficulties: None,
        },
    );
    assert!(
        result.is_err(),
        "an unknown fallback catalog holds publication"
    );
}

#[test]
fn selected_publication_catalogs_reject_difficulty_cycles() {
    let mut spells = SpellStore::new();
    spells.insert_spell_misc_attributes_for_difficulty_like_cpp(SPELL_ID, 7, [0; 15]);
    let options = SpellAuraOptionsStore::from_entries([options_row(1, 7, 1)]);
    let difficulties = DifficultyStore::from_entries([difficulty(0, 3), difficulty(3, 0)]);

    assert!(selected_misc_attributes(&spells, SPELL_ID, 0, Some(&difficulties)).is_err());
    assert!(
        selected_aura_options_entry(&options, SPELL_ID as u32, 0, Some(&difficulties)).is_err()
    );
}

#[test]
fn selected_publication_unknown_spellinfo_key_without_misc_stays_unavailable() {
    let spells = SpellStore::new();
    let options = SpellAuraOptionsStore::from_entries([]);
    let difficulties = DifficultyStore::from_ids([0]);
    let result = selected_aura_wire_facts(
        &application(0),
        &provenance(),
        Some(&persistence()),
        SelectedAuraPublicationCatalogs {
            races: None,
            spells: Some(&spells),
            aura_options: Some(&options),
            difficulties: Some(&difficulties),
        },
    );
    assert!(
        result.is_err(),
        "an unknown SpellInfo key cannot imply zero attributes"
    );
}
