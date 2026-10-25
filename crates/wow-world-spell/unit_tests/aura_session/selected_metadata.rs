use super::super::{
    SelectedSpellPolarityError,
    selected_catalog::{SelectedCatalogError, selected_spell_family, selected_spell_mechanic},
};
use super::{SelectedSpellMetadataError, selected_spell_metadata};
use wow_data::{
    DifficultyEntry, DifficultyStore, SpellClassOptionsEntry, SpellClassOptionsStore,
    SpellEffectInfo, SpellEffectPolarityMetadata, SpellHitMetadataLikeCpp, SpellStore,
};

const APPLY_AURA: u32 = 6;
const AURA_INCREASE_SPEED: i32 = 31;
const AURA_INCREASE_SWIM_SPEED: i32 = 58;
const AURA_TRANSFORM: i32 = 56;
const AURA_GHOST: i32 = 95;
const AURA_PERCENT_STAT: i32 = 80;
const AURA_DAMAGE_PERCENT_DONE: i32 = 79;
const AURA_NO_PVP_CREDIT: i32 = 159;
const EFFECT_IS_HARMFUL: u32 = 0x0000_1000;
const ATTR_PASSIVE: u32 = 0x0000_0040;
const ATTR_AURA_IS_DEBUFF: u32 = 0x0400_0000;

fn effect(index: u32, aura_type: i32) -> SpellEffectInfo {
    SpellEffectInfo {
        effect_index: index,
        effect: APPLY_AURA,
        effect_aura: aura_type,
        polarity_metadata: Some(SpellEffectPolarityMetadata::default()),
        ..SpellEffectInfo::default()
    }
}

fn class_row(id: u32, spell_id: i32, family: u8, flags: [u32; 4]) -> SpellClassOptionsEntry {
    SpellClassOptionsEntry {
        id,
        spell_id,
        modal_next_spell: 0,
        spell_class_set: family,
        spell_class_mask: flags,
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

fn set_attributes(store: &mut SpellStore, spell_id: i32, difficulty_id: u8, attributes: [u32; 15]) {
    store.insert_spell_misc_attributes_for_difficulty_like_cpp(spell_id, difficulty_id, attributes);
}

fn set_mechanic(store: &mut SpellStore, spell_id: i32, difficulty_id: u8, mechanic: i8) {
    let metadata = SpellHitMetadataLikeCpp {
        spell_mechanic: mechanic,
        ..SpellHitMetadataLikeCpp::default()
    };
    store.insert_spell_hit_metadata_for_difficulty_like_cpp(spell_id, difficulty_id, metadata);
}

#[test]
fn selected_metadata_debuff_attributes_decide_ghost_wisp_and_sickness_effects() {
    let cycle = DifficultyStore::from_entries([difficulty(4, 3), difficulty(3, 4)]);
    let cyclic_classes =
        SpellClassOptionsStore::from_entries([class_row(50_002, 50_002, 4, [1; 4])]);
    let cases = [
        (
            50_001,
            [AURA_GHOST, AURA_INCREASE_SPEED, AURA_INCREASE_SWIM_SPEED],
        ),
        (
            50_002,
            [
                AURA_INCREASE_SPEED,
                AURA_INCREASE_SWIM_SPEED,
                AURA_TRANSFORM,
            ],
        ),
        (
            50_003,
            [
                AURA_PERCENT_STAT,
                AURA_DAMAGE_PERCENT_DONE,
                AURA_NO_PVP_CREDIT,
            ],
        ),
    ];

    for (spell_id, aura_types) in cases {
        let mut spells = SpellStore::new();
        let mut attributes = [0; 15];
        attributes[0] = ATTR_AURA_IS_DEBUFF;
        set_attributes(&mut spells, spell_id, 4, attributes);
        set_mechanic(&mut spells, spell_id, 2, 29);
        let effects = aura_types
            .into_iter()
            .enumerate()
            .map(|(index, aura_type)| effect(index as u32, aura_type))
            .collect::<Vec<_>>();
        let (class_options, difficulties) = if spell_id == 50_002 {
            (Some(&cyclic_classes), Some(&cycle))
        } else {
            (None, None)
        };

        let selected = selected_spell_metadata(
            &spells,
            spell_id,
            4,
            &effects,
            class_options,
            difficulties,
        )
        .expect("debuff attributes decide each effect before unavailable family or mechanic facts");
        assert_eq!(selected.attributes, attributes);
        assert_eq!(selected.polarity.available_effect_mask, 0b111);
        assert_eq!(selected.polarity.negative_effect_mask, 0b111);
        assert_eq!(selected.polarity.positive_effect_mask, 0);
        assert!(!selected.polarity.whole_spell_positive);
    }
}

#[test]
fn selected_metadata_mixes_harmful_and_immune_effects_and_keeps_family_precedence() {
    let spell_id = 50_002;
    let mut spells = SpellStore::new();
    set_attributes(&mut spells, spell_id, 0, [0; 15]);
    set_mechanic(&mut spells, spell_id, 0, 29);
    let class_options =
        SpellClassOptionsStore::from_entries([class_row(spell_id as u32, spell_id, 0, [0; 4])]);
    let mut harmful = effect(1, AURA_NO_PVP_CREDIT);
    harmful.polarity_metadata.as_mut().unwrap().attributes = EFFECT_IS_HARMFUL;
    let mixed = selected_spell_metadata(
        &spells,
        spell_id,
        0,
        &[effect(0, AURA_NO_PVP_CREDIT), harmful],
        Some(&class_options),
        None,
    )
    .unwrap();
    assert_eq!(mixed.polarity.negative_effect_mask, 0b10);
    assert_eq!(mixed.polarity.positive_effect_mask, 0b01);
    assert!(!mixed.polarity.whole_spell_positive);

    let family_spell_id = 40_268;
    let mut family_spells = SpellStore::new();
    set_attributes(&mut family_spells, family_spell_id, 0, [0; 15]);
    set_mechanic(&mut family_spells, family_spell_id, 0, 29);
    let generic_family = SpellClassOptionsStore::from_entries([class_row(
        family_spell_id as u32,
        family_spell_id,
        0,
        [0; 4],
    )]);
    let family_result = selected_spell_metadata(
        &family_spells,
        family_spell_id,
        0,
        &[effect(0, AURA_NO_PVP_CREDIT)],
        Some(&generic_family),
        None,
    )
    .unwrap();
    assert_eq!(
        family_result.polarity.negative_effect_mask, 1,
        "the source family decision precedes the immune-shield mechanic",
    );
}

#[test]
fn selected_metadata_family_uses_highest_record_id_and_reachable_zero_row() {
    let spell_id = 50_003;
    let class_options = SpellClassOptionsStore::from_entries([
        class_row(10, spell_id, 4, [0; 4]),
        class_row(20, spell_id, 8, [1, 2, 3, 4]),
    ]);
    let chain =
        DifficultyStore::from_entries([difficulty(4, 3), difficulty(3, 0), difficulty(0, 0)]);
    assert_eq!(
        selected_spell_family(&class_options, spell_id, 4, Some(&chain)),
        Ok((8, [1, 2, 3, 4])),
    );

    let stopped = DifficultyStore::from_entries([difficulty(4, 3)]);
    assert_eq!(
        selected_spell_family(&class_options, spell_id, 4, Some(&stopped)),
        Ok((0, [0; 4])),
        "a known chain ending at a missing difficulty does not invent a base row",
    );

    let other_spell_only = SpellClassOptionsStore::from_entries([class_row(30, 50_004, 4, [9; 4])]);
    let zero_to_three = DifficultyStore::from_entries([difficulty(0, 3), difficulty(3, 0)]);
    assert_eq!(
        selected_spell_family(&other_spell_only, spell_id, 0, Some(&zero_to_three)),
        Ok((0, [0; 4])),
        "another spell's difficulty-zero row does not supply this spell's family",
    );
}

#[test]
fn selected_metadata_distinguishes_known_empty_catalogs_from_unknown_sources() {
    let spell_id = 50_005;
    let mut spells = SpellStore::new();
    set_attributes(&mut spells, spell_id, 4, [0; 15]);
    let empty_classes = SpellClassOptionsStore::from_entries([]);
    assert_eq!(selected_spell_mechanic(&spells, spell_id, 4, None), Ok(0));
    let known_empty = selected_spell_metadata(
        &spells,
        spell_id,
        4,
        &[effect(0, AURA_NO_PVP_CREDIT)],
        Some(&empty_classes),
        None,
    )
    .unwrap();
    assert_eq!(known_empty.polarity.positive_effect_mask, 1);

    assert_eq!(
        selected_spell_metadata(
            &spells,
            spell_id,
            4,
            &[effect(0, AURA_NO_PVP_CREDIT)],
            None,
            None
        ),
        Err(SelectedSpellMetadataError::Polarity(
            SelectedSpellPolarityError::MissingSpellFamilyName { index: 0 }
        )),
        "an unavailable class catalog is not treated as a known generic family",
    );

    let mut category_unknown = SpellStore::new();
    set_attributes(&mut category_unknown, spell_id, 0, [0; 15]);
    set_mechanic(&mut category_unknown, spell_id, 3, 7);
    let class_at_zero =
        SpellClassOptionsStore::from_entries([class_row(spell_id as u32, spell_id, 0, [0; 4])]);
    assert_eq!(
        selected_spell_metadata(
            &category_unknown,
            spell_id,
            0,
            &[effect(0, AURA_NO_PVP_CREDIT)],
            Some(&class_at_zero),
            None,
        ),
        Err(SelectedSpellMetadataError::Catalog {
            spell_id,
            requested_difficulty: 0,
            source: SelectedCatalogError::DifficultyUnavailable,
        }),
        "an existing category row with an unknown source graph stays unknown",
    );
}

#[test]
fn selected_metadata_exact_zero_mechanic_suppresses_base_and_zero_can_fall_back_to_three() {
    let spell_id = 50_006;
    let mut spells = SpellStore::new();
    set_mechanic(&mut spells, spell_id, 0, 29);
    set_mechanic(&mut spells, spell_id, 4, 0);
    set_mechanic(&mut spells, 50_007, 3, -7);
    let difficulties =
        DifficultyStore::from_entries([difficulty(4, 0), difficulty(0, 3), difficulty(3, 0)]);

    assert_eq!(
        selected_spell_mechanic(&spells, spell_id, 4, Some(&difficulties)),
        Ok(0)
    );
    assert_eq!(
        selected_spell_mechanic(&spells, 50_007, 0, Some(&difficulties)),
        Ok(-7)
    );
}

#[test]
fn selected_metadata_needed_unknown_and_cyclic_graphs_fail_but_irrelevant_graphs_do_not() {
    let spell_id = 50_008;
    let mut unknown = SpellStore::new();
    set_attributes(&mut unknown, spell_id, 0, [0; 15]);
    set_mechanic(&mut unknown, spell_id, 3, 7);
    let class_at_zero =
        SpellClassOptionsStore::from_entries([class_row(spell_id as u32, spell_id, 0, [0; 4])]);
    assert_eq!(
        selected_spell_metadata(
            &unknown,
            spell_id,
            0,
            &[effect(0, AURA_NO_PVP_CREDIT)],
            Some(&class_at_zero),
            None,
        ),
        Err(SelectedSpellMetadataError::Catalog {
            spell_id,
            requested_difficulty: 0,
            source: SelectedCatalogError::DifficultyUnavailable,
        }),
    );

    let mut cyclic = SpellStore::new();
    set_attributes(&mut cyclic, spell_id, 0, [0; 15]);
    set_mechanic(&mut cyclic, spell_id, 7, 7);
    let cycle = DifficultyStore::from_entries([difficulty(0, 3), difficulty(3, 0)]);
    assert_eq!(
        selected_spell_metadata(
            &cyclic,
            spell_id,
            0,
            &[effect(0, AURA_NO_PVP_CREDIT)],
            Some(&class_at_zero),
            Some(&cycle),
        ),
        Err(SelectedSpellMetadataError::Catalog {
            spell_id,
            requested_difficulty: 0,
            source: SelectedCatalogError::DifficultyCycle,
        }),
    );
}

#[test]
fn selected_metadata_keeps_the_earthquake_negative_effect_preset() {
    let spell_id = 61_882;
    let mut spells = SpellStore::new();
    let mut attributes = [0; 15];
    attributes[0] = ATTR_PASSIVE;
    set_attributes(&mut spells, spell_id, 0, attributes);

    let selected = selected_spell_metadata(
        &spells,
        spell_id,
        0,
        &[effect(2, AURA_NO_PVP_CREDIT)],
        None,
        None,
    )
    .unwrap();
    assert_eq!(selected.polarity.available_effect_mask, 1 << 2);
    assert_eq!(selected.polarity.negative_effect_mask, 1 << 2);
    assert_eq!(selected.polarity.positive_effect_mask, 0);
    assert!(!selected.polarity.whole_spell_positive);
}

#[test]
fn selected_metadata_without_attributes_requires_a_reachable_regular_key() {
    let spells = SpellStore::new();
    assert_eq!(
        selected_spell_metadata(
            &spells,
            50_009,
            0,
            &[effect(0, AURA_NO_PVP_CREDIT)],
            None,
            None,
        ),
        Err(SelectedSpellMetadataError::SpellInfoUnavailable {
            spell_id: 50_009,
            requested_difficulty: 0,
        }),
    );
}
