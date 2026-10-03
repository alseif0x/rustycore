//! Synthetic startup metadata and failure tests, not live Unit application.
use super::*;
use crate::forever::spells::SpellLoadPlan;
use std::sync::Arc;
use wow_data::{Db2HotfixRemovalStoreLikeCpp, forever_spells::*};

fn effect(aura: u32, misc: i32) -> SpellEffectValues {
    SpellEffectValues {
        aura,
        misc_values: [misc, 0],
        ..Default::default()
    }
}
fn row(id: i32) -> CreatureImmunityRow {
    CreatureImmunityRow {
        id,
        school: 0,
        dispel: 0,
        mechanics: 0,
        effects: vec![],
        auras: vec![],
        immune_aoe: false,
        immune_chain: false,
    }
}
fn phase(id: u32, effects: Vec<SpellEffectValues>, records: SpellRecords) -> SpellDefinitionSeeds {
    let catalog = Arc::new(
        records
            .finish(
                Default::default(),
                Default::default(),
                6,
                Default::default(),
                Default::default(),
                &Db2HotfixRemovalStoreLikeCpp::default(),
            )
            .unwrap(),
    );
    let mut seeds = SpellLoadPlan::build(catalog)
        .unwrap()
        .with_server_spells(Default::default())
        .unwrap();
    let mut definition = Definition::empty_server(vec![]);
    definition.effects = effects;
    seeds.definitions.insert((id, 0), definition);
    // Synthetic phase setup only; production-linked native test traverses real
    // constructor/custom/diminishing operations before this operation.
    seeds.source_order = Some(super::super::DefinitionOrder {
        primary: vec![(id, 0)],
        by_spell: BTreeMap::from([(id, vec![(id, 0)])]),
    });
    seeds.diminishing = Some(Default::default());
    seeds
}

#[test]
fn creatures_keep_signed_bitsets_repeated_row_semantics_and_strict_tokens() {
    let mut first = row(-7);
    first.school = -1;
    first.dispel = -1;
    first.mechanics = -1;
    first.immune_aoe = true;
    first.effects = b",0,360,361,01, 2,+2,-1,2x,4294967296,\xff,,".to_vec();
    first.auras = b"664,665,0002".to_vec();
    let mut counts = ImmunityCounts::default();
    let catalog = creatures::load(vec![first], &mut counts);
    let info = &catalog[&-7];
    assert_eq!(
        (
            info.school_mask,
            info.dispel_mask,
            info.mechanic_mask,
            info.other_mask
        ),
        (0x7f, 0xfff, (1 << 37) - 1, 1)
    );
    assert_eq!(info.effect_types, [0, 360, 1]);
    assert_eq!(info.aura_types, [664, 2]);
    assert_eq!(
        (
            counts.truncated_masks,
            counts.invalid_effect_tokens,
            counts.invalid_aura_tokens
        ),
        (3, 7, 1)
    );
    let mut a = row(3);
    a.effects = b"1,1".to_vec();
    a.immune_aoe = true;
    a.school = 7;
    let mut b = row(3);
    b.effects = b"2".to_vec();
    b.immune_chain = true;
    b.school = 2;
    let catalog = creatures::load(vec![a, b], &mut ImmunityCounts::default());
    let info = &catalog[&3];
    assert_eq!((info.school_mask, info.other_mask), (2, 3));
    assert_eq!(info.effect_types, [1, 1, 2]);
}

#[test]
fn each_aura_rule_keeps_native_widths_and_does_not_require_an_active_effect() {
    let empty = BTreeMap::new();
    for (aura, misc) in [
        (37, -1),
        (38, -1),
        (39, -1),
        (267, -1),
        (40, -1),
        (41, 31),
        (77, 63),
    ] {
        let info = effect_info(900001, &effect(aura, misc), 0, &empty).unwrap();
        match aura {
            37 => assert_eq!(info.effect_types, BTreeSet::from([u32::MAX])),
            38 => assert_eq!(info.aura_types, BTreeSet::from([u32::MAX])),
            39 => assert_eq!(info.school_mask, u32::MAX),
            267 => assert_eq!(info.harmful_aura_school_mask, u32::MAX),
            40 => assert_eq!(info.damage_school_mask, u32::MAX),
            41 => assert_eq!(info.dispel_mask, 1 << 31),
            77 => assert_eq!(info.mechanic_mask, 1 << 63),
            _ => unreachable!(),
        }
        assert!(has_info(&info));
    }
    for aura in [0, 999] {
        assert!(!has_info(
            &effect_info(900001, &effect(aura, -1), 0, &empty).unwrap()
        ));
    }
}

#[test]
fn source_trinket_overrides_use_expression_not_obsolete_hex_comment() {
    let empty = BTreeMap::new();
    assert_eq!(LOSS_CONTROL, 0x49967eae);
    for id in [
        42292, 59752, 34471, 19574, 46227, 53490, 65547, 134946, 134956, 195710, 208683,
    ] {
        let info = effect_info(id, &effect(77, 99), 0, &empty).unwrap();
        assert_eq!(info.mechanic_mask, LOSS_CONTROL);
        assert!(info.remove_effects_with_mechanic);
        assert_eq!(
            info.aura_types,
            if matches!(id, 42292 | 59752) {
                BTreeSet::from([191])
            } else {
                BTreeSet::new()
            }
        );
    }
    let info = effect_info(54508, &effect(77, 99), 0, &empty).unwrap();
    assert_eq!(info.mechanic_mask, (1 << 11) | (1 << 7) | (1 << 12));
    assert!(!info.remove_effects_with_mechanic);
    assert!(
        effect_info(900001, &effect(77, 0), 100, &empty)
            .unwrap()
            .remove_effects_with_mechanic
    );
    // Flag-only work buffer is not published to its effect.
    assert!(!has_info(
        &effect_info(900001, &effect(77, 0), 100, &empty).unwrap()
    ));
}

#[test]
fn invalid_source_shifts_are_errors_never_modulo_or_synthetic_zero() {
    let empty = BTreeMap::new();
    for misc in [64, i32::MAX] {
        assert_eq!(
            effect_info(900001, &effect(77, misc), 0, &empty),
            Err(SpellImmunityError::UndefinedMechanicShift)
        );
    }
    for misc in [-1, 32, i32::MAX] {
        assert_eq!(
            effect_info(900001, &effect(41, misc), 0, &empty),
            Err(SpellImmunityError::UndefinedDispelShift)
        );
    }
    for misc in [0, -1] {
        assert_eq!(
            effect_info(900001, &effect(77, misc), 0, &empty)
                .unwrap()
                .mechanic_mask,
            0
        );
    }
}

#[test]
fn allowed_attribute_mechanics_preserve_barkskin_dispersion_lichborne_exceptions() {
    for id in [900001, 22812, 47585, 49039] {
        for flags in [0, 0x8, 0x40000, 0x20000, 0x60008] {
            let mut definition = Definition::empty_server(vec![]);
            definition.fields.attributes[5] = flags;
            let mut expected = 0;
            if flags & 0x8 != 0 {
                expected |= match id {
                    22812 | 47585 => (1 << 12) | (1 << 13) | (1 << 14) | (1 << 10),
                    49039 => 0,
                    _ => 1 << 12,
                };
            }
            if flags & 0x40000 != 0 {
                expected |= 1 << 2;
            }
            if flags & 0x20000 != 0 {
                expected |= if matches!(id, 22812 | 47585) {
                    (1 << 5) | (1 << 24)
                } else {
                    1 << 5
                };
            }
            assert_eq!(attribute_mechanics(id, &definition), expected);
        }
    }
}

#[test]
fn canonical_phase_keeps_creature_and_effect_owners_gap_slots_and_raw_identity() {
    let mut rows = row(-7);
    rows.school = 2;
    rows.dispel = 4;
    rows.mechanics = 1 << 32;
    rows.effects = b"2,2,1".to_vec();
    rows.auras = b"77,77".to_vec();
    rows.immune_chain = true;
    let mut seeds = phase(
        900001,
        vec![effect(147, -7), effect(147, 999), effect(38, 0)],
        Default::default(),
    );
    seeds
        .definitions
        .get_mut(&(900001, 0))
        .unwrap()
        .fields
        .attributes[5] = 0x40000;
    let before = seeds.raw_catalog();
    let loaded = seeds.with_immunity_info(vec![rows]).unwrap();
    assert!(Arc::ptr_eq(&before, &loaded.raw_catalog()));
    let definition = loaded.get_exact(900001, 0).unwrap();
    let first = definition.effect(0).unwrap();
    assert!(!first.is_effect()); // Source still initializes immunity metadata.
    let info = first.immunity_info().unwrap();
    assert_eq!(
        (
            info.school_mask,
            info.dispel_mask,
            info.mechanic_mask,
            info.other_mask
        ),
        (2, 4, 1 << 32, 2)
    );
    assert_eq!(info.effect_types, BTreeSet::from([1, 2]));
    assert_eq!(info.aura_types, BTreeSet::from([77]));
    assert!(definition.effect(1).unwrap().immunity_info().is_none());
    assert_eq!(
        definition
            .effect(2)
            .unwrap()
            .immunity_info()
            .unwrap()
            .aura_types,
        BTreeSet::from([0])
    );
    assert_eq!(definition.allowed_mechanic_mask(), (1 << 32) | (1 << 2));
    let iterated = definition.effects().next().unwrap();
    assert!(std::ptr::eq(info, iterated.immunity_info().unwrap()));
    assert_eq!(
        loaded.creature_immunity(-7).unwrap().effect_types,
        [2, 2, 1]
    );
    assert_eq!(
        loaded.immunity_counts(),
        Some(ImmunityCounts {
            input_rows: 1,
            creatures: 1,
            definitions: 1,
            effect_slots: 3,
            effects_with_info: 2,
            ..Default::default()
        })
    );
    assert!(matches!(
        loaded.with_immunity_info(vec![]),
        Err(SpellImmunityError::AlreadyApplied)
    ));
}

#[test]
fn admission_and_duration_errors_do_not_return_a_partial_completed_phase() {
    let mut seeds = phase(900001, vec![], Default::default());
    seeds.diminishing = None;
    assert!(matches!(
        seeds.with_immunity_info(vec![]),
        Err(SpellImmunityError::RequiresDiminishing)
    ));
    for (maximum, expected) in [(100, true), (-100, true), (-1, false), (99, false)] {
        let mut seeds = phase(
            900001,
            vec![effect(77, 7)],
            SpellRecords {
                spell_durations: vec![SpellDurationRecord {
                    id: 3,
                    duration: 0,
                    max_duration: maximum,
                    duration_per_resource: 0,
                }],
                ..Default::default()
            },
        );
        seeds.definitions.get_mut(&(900001, 0)).unwrap().duration = Some(3);
        let seeds = seeds.with_immunity_info(vec![]).unwrap();
        assert_eq!(
            seeds
                .get_exact(900001, 0)
                .unwrap()
                .effect(0)
                .unwrap()
                .immunity_info()
                .unwrap()
                .remove_effects_with_mechanic,
            expected
        );
    }
    let mut seeds = phase(
        900001,
        vec![effect(77, 7)],
        SpellRecords {
            spell_durations: vec![SpellDurationRecord {
                id: 3,
                duration: 0,
                max_duration: i32::MIN,
                duration_per_resource: 0,
            }],
            ..Default::default()
        },
    );
    seeds.definitions.get_mut(&(900001, 0)).unwrap().duration = Some(3);
    assert!(matches!(
        seeds.with_immunity_info(vec![]),
        Err(SpellImmunityError::UndefinedDurationAbs)
    ));
    assert!(matches!(
        phase(
            900001,
            vec![effect(39, 1), effect(41, 32)],
            Default::default()
        )
        .with_immunity_info(vec![]),
        Err(SpellImmunityError::UndefinedDispelShift)
    ));
}
