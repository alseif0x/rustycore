use super::*;
fn species(id: u32, creature: i32) -> BattlePetSpeciesRecord {
    BattlePetSpeciesRecord {
        description: SpellText::default(),
        source_text: SpellText::default(),
        id,
        creature_id: creature,
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

#[test]
fn language_registrations_keep_duplicates_even_overwritten_and_unnamed_effects() {
    let mut rows = named(&[1]);
    let mut first = effect(1, 1, 0, 0);
    first.effect = 39;
    first.effect_misc_value[0] = -1;
    let mut second = first;
    second.id = 2;
    let mut unnamed = first;
    unnamed.id = 3;
    unnamed.spell_id = 99;
    let mut skipped = first;
    skipped.id = 4;
    skipped.effect_aura = -1;
    rows.spell_effects = vec![skipped, unnamed, second, first];
    let result = plan(rows);
    assert_eq!(
        result
            .language_spell_registrations(u32::MAX)
            .collect::<Vec<_>>(),
        [1, 1, 99]
    );
    assert_eq!(result.language_spell_registrations(0).count(), 0);
    assert_eq!(result.counts().language_registrations, 3);
    assert_eq!(result.counts().unnamed_helpers, 1);
    assert_eq!(result.get(1, 0).unwrap().effect_slots()[0].unwrap().id, 2);
}

#[test]
fn summon_species_uses_last_creature_and_spell_association_and_required_flags() {
    let mut rows = named(&[1]);
    rows.battle_pet_species = vec![
        species(2, 10),
        species(1, 10),
        species(3, -1),
        species(4, 0),
    ];
    rows.summon_properties = vec![
        SummonPropertiesRecord {
            id: u32::MAX,
            control: 0,
            faction: 0,
            title: 0,
            slot: 5,
            flags: [0x0020_0000, i32::MIN],
        },
        SummonPropertiesRecord {
            id: 1,
            control: 0,
            faction: 0,
            title: 0,
            slot: 5,
            flags: [0, 0],
        },
        SummonPropertiesRecord {
            id: 2,
            control: 0,
            faction: 0,
            title: 0,
            slot: 0,
            flags: [0x0020_0000, 0],
        },
    ];
    let mut first = effect(1, 1, 0, 0);
    first.effect = 28;
    first.effect_misc_value = [10, -1];
    let mut second = first;
    second.id = 2;
    second.effect_misc_value[0] = -1;
    let mut missing_flag = first;
    missing_flag.id = 3;
    missing_flag.effect_misc_value[1] = 1;
    let mut wrong_slot = first;
    wrong_slot.id = 4;
    wrong_slot.effect_misc_value[1] = 2;
    let mut zero_creature = first;
    zero_creature.id = 5;
    zero_creature.effect_misc_value[0] = 0;
    let mut unnamed = first;
    unnamed.id = 6;
    unnamed.spell_id = 99;
    let mut skipped = first;
    skipped.id = 7;
    skipped.effect_aura = -1;
    rows.spell_effects = vec![
        skipped,
        unnamed,
        zero_creature,
        wrong_slot,
        missing_flag,
        second,
        first,
    ];
    let raw = catalog(rows);
    let result = SpellLoadPlan::build(raw.clone()).unwrap();
    assert_eq!(result.battle_pet_species_for_spell(1).unwrap().id, 3);
    assert_eq!(result.battle_pet_species_for_spell(99).unwrap().id, 2);
    assert!(std::ptr::eq(
        result.battle_pet_species_for_spell(1).unwrap(),
        raw.battle_pet_species(3).unwrap()
    ));
    assert!(result.battle_pet_species_for_spell(2).is_none());
    assert_eq!(result.counts().battle_pet_spell_associations, 2);
}
