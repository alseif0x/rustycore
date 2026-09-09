//! Spell scenarios for [`super`].
//!
//! Split out of spell_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn primary_profession_spell_classifier_matches_cpp_root_and_rank_rules() {
    let skill_lines = crate::skill_talent::SkillLineStore::from_entries([
        test_skill_line_like_cpp(100, 11, 0),
        test_skill_line_like_cpp(101, 11, 100),
        test_skill_line_like_cpp(200, 9, 0),
        test_skill_line_like_cpp(300, 11, 0),
    ]);
    let mut spell = SpellStore::empty_spell_info_like_cpp(1_000);
    spell.effects = vec![
        test_skill_effect_like_cpp(2, 100),
        test_skill_effect_like_cpp(1, 300),
        test_skill_effect_like_cpp(2, 101),
        test_skill_effect_like_cpp(3, 200),
        test_skill_effect_like_cpp(4, 300),
    ];

    assert_eq!(
        spell
            .primary_profession_skill_effect_ids_like_cpp(&skill_lines)
            .unwrap(),
        vec![300, 100],
        "primary lines follow C++ effect-index order and deduplicate at first appearance"
    );
    assert!(
        spell
            .is_primary_profession_first_rank_like_cpp(
                &skill_lines,
                &SpellChainStoreLikeCpp::default(),
            )
            .unwrap(),
        "C++ SpellInfo::GetRank returns one without a ChainEntry"
    );

    let rank_two = SpellChainStoreLikeCpp {
        chains_by_spell_id: BTreeMap::from([(
            1_000,
            SpellChainNodeLikeCpp {
                prev_spell_id: Some(999),
                next_spell_id: None,
                first_spell_id: 999,
                last_spell_id: 1_000,
                rank: 2,
            },
        )]),
        ..SpellChainStoreLikeCpp::default()
    };
    assert!(
        !spell
            .is_primary_profession_first_rank_like_cpp(&skill_lines, &rank_two)
            .unwrap()
    );

    let mut unhydrated_rank_two = SpellStore::empty_spell_info_like_cpp(1_000);
    unhydrated_rank_two.effects = vec![test_skill_effect_like_cpp(0, 999)];
    let partial_skill_lines =
        crate::skill_talent::SkillLineStore::from_hydrated_entries_and_effective_ids_like_cpp(
            [test_skill_line_like_cpp(100, 11, 0)],
            [100, 999],
        );
    assert_eq!(
        unhydrated_rank_two
            .is_primary_profession_first_rank_like_cpp(&partial_skill_lines, &rank_two,),
        Ok(false),
        "rank two is decidably false without requiring unrelated partial payload"
    );

    let mut partly_hydrated_rank_one = SpellStore::empty_spell_info_like_cpp(1_001);
    partly_hydrated_rank_one.effects = vec![
        test_skill_effect_like_cpp(0, 999),
        test_skill_effect_like_cpp(1, 100),
    ];
    assert_eq!(
        partly_hydrated_rank_one.is_primary_profession_first_rank_like_cpp(
            &partial_skill_lines,
            &SpellChainStoreLikeCpp::default(),
        ),
        Ok(true),
        "one hydrated primary effect proves C++'s boolean result"
    );

    let mut only_unhydrated_rank_one = SpellStore::empty_spell_info_like_cpp(1_002);
    only_unhydrated_rank_one.effects = vec![test_skill_effect_like_cpp(0, 999)];
    assert_eq!(
        only_unhydrated_rank_one.is_primary_profession_first_rank_like_cpp(
            &partial_skill_lines,
            &SpellChainStoreLikeCpp::default(),
        ),
        Err(
            PrimaryProfessionSpellClassificationErrorLikeCpp::MissingSkillLinePayload {
                spell_id: 1_002,
                skill_id: 999,
            }
        )
    );
}
#[test]
fn primary_profession_spell_classifier_distinguishes_absent_unhydrated_and_invalid_skill() {
    let skill_lines =
        crate::skill_talent::SkillLineStore::from_hydrated_entries_and_effective_ids_like_cpp(
            [test_skill_line_like_cpp(100, 11, 0)],
            [100, 999],
        );
    let mut unhydrated = SpellStore::empty_spell_info_like_cpp(1_000);
    unhydrated.effects = vec![test_skill_effect_like_cpp(0, 999)];
    assert_eq!(
        unhydrated.primary_profession_skill_effect_ids_like_cpp(&skill_lines),
        Err(
            PrimaryProfessionSpellClassificationErrorLikeCpp::MissingSkillLinePayload {
                spell_id: 1_000,
                skill_id: 999,
            }
        )
    );

    let mut absent = SpellStore::empty_spell_info_like_cpp(1_001);
    absent.effects = vec![test_skill_effect_like_cpp(0, 998)];
    assert_eq!(
        absent.primary_profession_skill_effect_ids_like_cpp(&skill_lines),
        Ok(Vec::new()),
        "a failed C++ LookupEntry is non-primary"
    );

    let mut invalid = SpellStore::empty_spell_info_like_cpp(1_002);
    invalid.effects = vec![test_skill_effect_like_cpp(2, -1)];
    assert_eq!(
        invalid.primary_profession_skill_effect_ids_like_cpp(&skill_lines),
        Err(
            PrimaryProfessionSpellClassificationErrorLikeCpp::InvalidSkillId {
                spell_id: 1_002,
                effect_index: 2,
                skill_id: -1,
            }
        )
    );

    let mut invalid_spell = SpellStore::empty_spell_info_like_cpp(1_003);
    invalid_spell.spell_id = -1;
    invalid_spell.effects = vec![test_skill_effect_like_cpp(0, 100)];
    assert_eq!(
        invalid_spell.is_primary_profession_first_rank_like_cpp(
            &skill_lines,
            &SpellChainStoreLikeCpp::default(),
        ),
        Err(PrimaryProfessionSpellClassificationErrorLikeCpp::InvalidSpellId { spell_id: -1 })
    );
}
#[test]
fn casting_requirements_hydrate_requires_spell_focus_from_effective_store_like_cpp() {
    let requirements =
        |id, spell_id: i32, requires_spell_focus| crate::spell_db2::SpellCastingRequirementsEntry {
            id,
            spell_id,
            facing_caster_flags: 0,
            min_faction_id: 0,
            min_reputation: 0,
            required_areas_id: 0,
            required_aura_vision: 0,
            requires_spell_focus,
        };
    let build_spell_store = || {
        SpellStore::from_spell_db2_stores_like_cpp(
            &crate::spell_db2::SpellCategoriesStore::from_entries([]),
            &crate::spell_db2::SpellMiscStore::from_entries([
                test_spell_misc_entry_like_cpp(1, 100, 0, 0),
                test_spell_misc_entry_like_cpp(2, 200, 0, 0),
            ]),
            &crate::spell_db2::SpellEffectDb2Store::from_entries([]),
            &crate::spell_db2::SpellShapeshiftStore::from_entries([]),
        )
    };

    let mut store = build_spell_store();
    store.apply_db2_casting_requirements_like_cpp(
        &crate::spell_db2::SpellCastingRequirementsStore::from_entries([
            requirements(1, 100, 181),
            // A malformed duplicate resolves to the highest record ID, the
            // slot C++'s record-ID ordered DB2 iteration assigns last.
            requirements(2, 100, 23),
        ]),
    );
    assert_eq!(
        store.get(100).map(|spell| spell.requires_spell_focus),
        Some(23)
    );
    assert!(
        store
            .get(100)
            .expect("spell 100")
            .requires_spell_focus_like_cpp()
    );
    assert_eq!(
        store.get(200).map(|spell| spell.requires_spell_focus),
        Some(0),
        "a spell without a requirements row keeps the C++ default"
    );

    // A final tombstone removes the only row, so the spell must fall back to
    // zero instead of keeping a resurrected focus object.
    let mut tombstoned = build_spell_store();
    tombstoned.apply_db2_casting_requirements_like_cpp(
        &crate::spell_db2::SpellCastingRequirementsStore::from_entries([]),
    );
    assert_eq!(
        tombstoned.get(100).map(|spell| spell.requires_spell_focus),
        Some(0)
    );
}
#[test]
fn real_spell_15691_hit_metadata_matches_db2_when_data_exists() {
    let data_dir = std::env::var("RUSTYCORE_REAL_DATA_DIR")
        .unwrap_or_else(|_| "/home/server/woltk-server-core/Data".to_string());
    let locale = std::env::var("RUSTYCORE_REAL_LOCALE").unwrap_or_else(|_| "enUS".to_string());
    let dbc_dir = std::path::Path::new(&data_dir).join("dbc").join(&locale);
    if ["SpellCategories.db2", "SpellMisc.db2", "SpellEffect.db2"]
        .into_iter()
        .any(|file| !dbc_dir.join(file).is_file())
    {
        eprintln!(
            "Skipping real spell hit-metadata fixture: DB2 files not found at {}",
            dbc_dir.display()
        );
        return;
    }

    let category_store = crate::spell_db2::SpellCategoriesStore::load(&data_dir, &locale)
        .expect("load real SpellCategories.db2");
    let misc_store = crate::spell_db2::SpellMiscStore::load(&data_dir, &locale)
        .expect("load real SpellMisc.db2");
    let effect_store = crate::spell_db2::SpellEffectDb2Store::load(&data_dir, &locale)
        .expect("load real SpellEffect.db2");
    let store = SpellStore::from_spell_db2_stores_like_cpp(
        &category_store,
        &misc_store,
        &effect_store,
        &crate::spell_db2::SpellShapeshiftStore::from_entries([]),
    );

    assert_eq!(
        store.hit_metadata_for_difficulty_like_cpp(15_691, 0, None),
        Some(SpellHitMetadataLikeCpp {
            category_id: 0,
            charge_category_id: 0,
            defense_type: 2,
            spell_mechanic: 0,
            school_mask: 1,
            effect_mechanics: BTreeMap::from([(0, 0)]),
        })
    );
}
#[test]
fn test_spell_store_creation() {
    let store = SpellStore::new();
    assert!(store.is_empty(), "new store should be empty");
}
#[test]
fn exact_spell_info_key_does_not_fabricate_hydrated_payload() {
    let mut store = SpellStore::new();
    store.spell_info_keys_like_cpp =
        crate::spell_info_keys::SpellInfoKeyStoreLikeCpp::from_candidate_keys_like_cpp(
            [(200, 2), (100, 0), (200, 1)],
            &HashSet::from([100, 200]),
        );

    assert!(store.contains_spell_info_exact_like_cpp(100, 0));
    assert!(store.get(100).is_none());
    assert_eq!(
        store.spell_info_keys_in_order_like_cpp(),
        [(100, 0), (200, 1), (200, 2)]
    );
}
#[test]
fn spell_store_db2_loader_keeps_mount_aura_spells_like_cpp() {
    let spell_id = 32_243;
    let mut misc = test_spell_misc_entry_like_cpp(1, spell_id, 0, 0);
    misc.attributes[0] = attributes::SPELL_ATTR0_NO_AURA_CANCEL as i32;
    let misc_store = crate::spell_db2::SpellMiscStore::from_entries([misc]);
    let effect_store = crate::spell_db2::SpellEffectDb2Store::from_entries([
        crate::spell_db2::SpellEffectDb2Entry {
            id: 1,
            difficulty_id: 0,
            effect_index: 0,
            effect: spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            effect_amplitude: 0.0,
            effect_attributes: 0,
            effect_aura: aura_types::SPELL_AURA_MOUNTED as i16,
            effect_aura_period: 0,
            effect_base_points: 77,
            effect_bonus_coefficient: 0.0,
            effect_chain_amplitude: 0.0,
            effect_chain_targets: 0,
            effect_die_sides: 0,
            effect_item_type: 0,
            effect_mechanic: 0,
            effect_points_per_resource: 0.0,
            effect_pos_facing: 0.0,
            effect_real_points_per_level: 0.0,
            effect_trigger_spell: 0,
            bonus_coefficient_from_ap: 0.0,
            pvp_multiplier: 0.0,
            coefficient: 0.0,
            variance: 0.0,
            resource_coefficient: 0.0,
            group_size_base_points_coefficient: 0.0,
            effect_misc_value: [23966, 0],
            effect_radius_index: [0, 0],
            effect_spell_class_mask: [0, 0, 0, 0],
            implicit_target: [0, 0],
            spell_id,
        },
    ]);

    let shapeshift_store = crate::spell_db2::SpellShapeshiftStore::from_entries([]);
    let store = SpellStore::from_spell_db2_stores_like_cpp(
        &crate::spell_db2::SpellCategoriesStore::from_entries([]),
        &misc_store,
        &effect_store,
        &shapeshift_store,
    );
    let spell = store.get(spell_id as i32).expect("mount spell loaded");

    assert_eq!(
        spell.effect_type,
        spell_effect_types::SPELL_EFFECT_APPLY_AURA
    );
    assert_eq!(spell.aura_type, Some(aura_types::SPELL_AURA_MOUNTED));
    assert!(
        spell
            .effects
            .iter()
            .any(SpellEffectInfo::is_mounted_aura_like_cpp)
    );
    assert!(store.has_attribute0_like_cpp(spell_id as i32, attributes::SPELL_ATTR0_NO_AURA_CANCEL));
}
#[test]
fn spell_store_db2_loader_keeps_channeled_spell_attr1_like_cpp() {
    let spell_id = 51_588;
    let mut misc = test_spell_misc_entry_like_cpp(1, spell_id, 0, 0);
    misc.attributes[1] = attributes::SPELL_ATTR1_IS_CHANNELLED as i32;
    let misc_store = crate::spell_db2::SpellMiscStore::from_entries([misc]);
    let effect_store = crate::spell_db2::SpellEffectDb2Store::from_entries([]);

    let shapeshift_store = crate::spell_db2::SpellShapeshiftStore::from_entries([]);
    let store = SpellStore::from_spell_db2_stores_like_cpp(
        &crate::spell_db2::SpellCategoriesStore::from_entries([]),
        &misc_store,
        &effect_store,
        &shapeshift_store,
    );

    assert!(store.has_attribute1_like_cpp(spell_id as i32, attributes::SPELL_ATTR1_IS_CHANNELLED));
    assert!(store.is_channeled_like_cpp(spell_id as i32));
    assert!(!store.is_channeled_like_cpp(99_999));
}
#[test]
fn spell_store_resolves_interrupt_masks_by_difficulty_and_fallback_like_cpp() {
    let spell_id = 70_101;
    let exact_without_difficulty_entry_spell_id = 70_102;
    let interrupts = crate::spell_db2::SpellInterruptsStore::from_entries([
        crate::spell_db2::SpellInterruptsEntry {
            id: 1,
            difficulty_id: 0,
            interrupt_flags: 0,
            aura_interrupt_flags: [0x0004_0000, 0],
            channel_interrupt_flags: [0, 0],
            spell_id,
        },
        crate::spell_db2::SpellInterruptsEntry {
            id: 2,
            difficulty_id: 2,
            interrupt_flags: 0,
            aura_interrupt_flags: [0, 0],
            channel_interrupt_flags: [0x0004_0000, 0],
            spell_id,
        },
        crate::spell_db2::SpellInterruptsEntry {
            id: 3,
            difficulty_id: 9,
            interrupt_flags: 0,
            aura_interrupt_flags: [0, 0x40],
            channel_interrupt_flags: [0, 0x80],
            spell_id: exact_without_difficulty_entry_spell_id,
        },
    ]);
    let mut store = SpellStore::new();
    let difficulties = crate::difficulty::DifficultyStore::from_entries([
        crate::difficulty::DifficultyEntry {
            id: 1,
            instance_type: 0,
            flags: 0,
            fallback_difficulty_id: 0,
            toggle_difficulty_id: 0,
        },
        crate::difficulty::DifficultyEntry {
            id: 2,
            instance_type: 0,
            flags: 0,
            fallback_difficulty_id: 1,
            toggle_difficulty_id: 0,
        },
        crate::difficulty::DifficultyEntry {
            id: 3,
            instance_type: 0,
            flags: 0,
            fallback_difficulty_id: 1,
            toggle_difficulty_id: 0,
        },
    ]);

    store.apply_db2_interrupts_like_cpp(&interrupts);

    assert_eq!(
        store.interrupt_flags_for_difficulty_like_cpp(spell_id as i32, 2, Some(&difficulties),),
        Some(([0, 0], [0x0004_0000, 0])),
        "the exact row overrides its base row without merging words"
    );
    assert_eq!(
        store.interrupt_flags_for_difficulty_like_cpp(spell_id as i32, 3, Some(&difficulties),),
        Some(([0x0004_0000, 0], [0, 0])),
        "difficulty 3 walks 3 -> 1 -> 0"
    );
    assert_eq!(
        store.interrupt_flags_for_difficulty_like_cpp(
            exact_without_difficulty_entry_spell_id as i32,
            9,
            Some(&difficulties),
        ),
        Some(([0, 0x40], [0, 0x80])),
        "an exact SpellInterrupts row wins before Difficulty lookup"
    );
    assert_eq!(
        store.interrupt_flags_for_difficulty_like_cpp(99_999, 3, Some(&difficulties)),
        None,
        "a fully missing fallback chain stays unknown"
    );
    assert!(store.has_aura_interrupt_flag_like_cpp(spell_id as i32, 0x0004_0000, 0));
    assert!(!store.has_channel_interrupt_flag_like_cpp(spell_id as i32, 0x0004_0000, 0));
}
#[test]
fn spell_store_effective_interrupt_masks_follow_cpp_load_order() {
    let regular_spell_id = 24_314;
    let serverside_spell_id = 70_001;
    let interrupts = crate::spell_db2::SpellInterruptsStore::from_entries([
        crate::spell_db2::SpellInterruptsEntry {
            id: 1,
            difficulty_id: 2,
            interrupt_flags: 0,
            aura_interrupt_flags: [0x100, 0x200],
            channel_interrupt_flags: [0x300, 0x400],
            spell_id: regular_spell_id,
        },
    ]);
    let mut store = SpellStore::new();
    store.apply_db2_interrupts_like_cpp(&interrupts);

    assert_eq!(
        store.interrupt_flags_for_difficulty_like_cpp(regular_spell_id as i32, 2, None),
        Some(([0x100, 0x200], [0x300, 0x400]))
    );

    assert!(store.store_signed_interrupt_row_by_id_like_cpp(
        1,
        regular_spell_id,
        2,
        [0x10, -1],
        [i32::MIN, 0x40],
    ));
    store.rebuild_interrupt_flags_from_rows_like_cpp();
    assert_eq!(
        store.interrupt_flags_for_difficulty_like_cpp(regular_spell_id as i32, 2, None),
        Some(([0x10, u32::MAX], [0x8000_0000, 0x40])),
        "the later row for the same DB2 record ID replaces its masks and preserves signed bit patterns"
    );

    let serverside = ServersideSpellStoreLikeCpp::from_rows_like_cpp(
        [serverside_spell_row(serverside_spell_id, 2)],
        &ServersideSpellEffectStoreLikeCpp::default(),
        |_| false,
    );
    assert!(serverside.errors.is_empty());
    store.apply_serverside_spell_interrupts_like_cpp(&serverside.store);

    assert_eq!(
        store.interrupt_flags_for_difficulty_like_cpp(regular_spell_id as i32, 2, None),
        Some(([0x3c, u32::MAX], [0x8000_0000, 0x40])),
        "the interrupt correction runs after the file/hotfix composition"
    );
    assert_eq!(
        store.interrupt_flags_for_difficulty_like_cpp(serverside_spell_id as i32, 2, None),
        Some(([43, 44], [45, 46])),
        "server-side masks enter the same effective table before corrections"
    );
}
#[test]
fn spell_store_hotfix_overlay_rekeys_by_db2_record_id_like_cpp() {
    let original_spell_id = 70_201;
    let rekeyed_spell_id = 70_202;
    let interrupts = crate::spell_db2::SpellInterruptsStore::from_entries([
        crate::spell_db2::SpellInterruptsEntry {
            id: 10,
            difficulty_id: 2,
            interrupt_flags: 0,
            aura_interrupt_flags: [0x10, 0],
            channel_interrupt_flags: [0x20, 0],
            spell_id: original_spell_id,
        },
        crate::spell_db2::SpellInterruptsEntry {
            id: 20,
            difficulty_id: 2,
            interrupt_flags: 0,
            aura_interrupt_flags: [0x30, 0],
            channel_interrupt_flags: [0x40, 0],
            spell_id: original_spell_id,
        },
    ]);
    let mut store = SpellStore::new();
    store.apply_db2_interrupts_like_cpp(&interrupts);

    assert_eq!(
        store.interrupt_flags_for_difficulty_like_cpp(original_spell_id as i32, 2, None),
        Some(([0x30, 0], [0x40, 0])),
        "the highest DB2 record ID wins when two rows have the same relational key"
    );

    assert!(store.store_signed_interrupt_row_by_id_like_cpp(
        20,
        rekeyed_spell_id,
        3,
        [0x50, 0],
        [0x60, 0],
    ));
    store.rebuild_interrupt_flags_from_rows_like_cpp();
    assert_eq!(
        store.interrupt_flags_for_difficulty_like_cpp(original_spell_id as i32, 2, None),
        Some(([0x10, 0], [0x20, 0])),
        "replacing record ID 20 uncovers record ID 10 at its former key"
    );
    assert_eq!(
        store.interrupt_flags_for_difficulty_like_cpp(rekeyed_spell_id as i32, 3, None),
        Some(([0x50, 0], [0x60, 0])),
        "the replacement row is indexed by its new spell/difficulty relationship"
    );
}
#[test]
fn spell_store_interrupt_corrections_cover_every_stored_difficulty() {
    let mut store = SpellStore::new();
    for difficulty_id in [0, 2] {
        store.insert_spell_interrupt_flags_for_difficulty_like_cpp(
            29_726,
            difficulty_id,
            [0, 0],
            [0xffff_ffff, 0x20],
        );
        store.insert_spell_interrupt_flags_for_difficulty_like_cpp(
            24_314,
            difficulty_id,
            [0x10, 0x40],
            [0x80, 0x100],
        );
        store.insert_spell_interrupt_flags_for_difficulty_like_cpp(
            99_252,
            difficulty_id,
            [0x200, 0x400],
            [0x800, 0x1000],
        );
    }
    store.insert_spell_interrupt_flags_like_cpp(63_414, [0x10, 0x20], [0xffff_ffff, 0xffff_ffff]);
    store
        .spells
        .insert(61_719, SpellStore::empty_spell_info_like_cpp(61_719));

    store.apply_interrupt_flag_corrections_like_cpp();

    for difficulty_id in [0, 2] {
        assert_eq!(
            store.interrupt_flags_for_difficulty_like_cpp(29_726, difficulty_id, None),
            Some(([0, 0], [0xffff_fffb, 0x20]))
        );
        assert_eq!(
            store.interrupt_flags_for_difficulty_like_cpp(24_314, difficulty_id, None),
            Some(([0x3c, 0x40], [0x80, 0x100]))
        );
        assert_eq!(
            store.interrupt_flags_for_difficulty_like_cpp(99_252, difficulty_id, None),
            Some(([0x8_0200, 0x400], [0x800, 0x1000]))
        );
    }
    assert_eq!(
        store.interrupt_flags_for_difficulty_like_cpp(63_414, 0, None),
        Some(([0x10, 0x20], [0, 0]))
    );
    assert_eq!(
        store.interrupt_flags_for_difficulty_like_cpp(61_719, 0, None),
        Some(([0x3, 0], [0, 0])),
        "a corrected regular spell without a SpellInterrupts row receives a base mask"
    );
}
#[test]
fn db2_cast_times_set_max_base_minimum_like_cpp() {
    use crate::spell_db2::{
        SpellCastTimesEntry, SpellCastTimesStore, SpellMiscEntry, SpellMiscStore,
    };
    let mut store = SpellStore::new();
    store
        .spells
        .insert(100, SpellStore::empty_spell_info_like_cpp(100));
    store
        .spells
        .insert(200, SpellStore::empty_spell_info_like_cpp(200));

    let misc = SpellMiscStore::from_entries([
        SpellMiscEntry {
            id: 1,
            spell_id: 100,
            casting_time_index: 5,
            difficulty_id: 0,
            ..Default::default()
        },
        // casting_time_index 0 → no cast-time row, stays instant.
        SpellMiscEntry {
            id: 2,
            spell_id: 200,
            casting_time_index: 0,
            difficulty_id: 0,
            ..Default::default()
        },
    ]);
    let cast_times = SpellCastTimesStore::from_entries([SpellCastTimesEntry {
        id: 5,
        base: 1500,
        minimum: 1000,
        ..Default::default()
    }]);

    store.apply_db2_cast_times_like_cpp(&misc, &cast_times);

    // C++ CalcCastTime = max(Base, Minimum) = max(1500, 1000) = 1500.
    assert_eq!(store.spells.get(&100).unwrap().cast_time_ms, 1500);
    assert!(store.spells.get(&100).unwrap().has_cast_time());
    // No CastingTimeIndex → untouched (instant).
    assert_eq!(store.spells.get(&200).unwrap().cast_time_ms, 0);
    assert!(!store.spells.get(&200).unwrap().has_cast_time());
}
#[test]
fn db2_spell_power_sets_power_costs_like_cpp() {
    use crate::spell_db2::{
        SpellPowerDifficultyEntry, SpellPowerDifficultyStore, SpellPowerEntry, SpellPowerStore,
    };

    let mut store = SpellStore::new();
    store
        .spells
        .insert(400, SpellStore::empty_spell_info_like_cpp(400));

    let spell_power = SpellPowerStore::from_entries([
        SpellPowerEntry {
            id: 10,
            order_index: 1,
            mana_cost: 40,
            mana_cost_per_level: 4,
            mana_per_second: 5,
            power_display_id: 0,
            alt_power_bar_id: 0,
            power_cost_pct: 10.0,
            power_cost_max_pct: 0.0,
            power_pct_per_second: 6.5,
            power_type: PowerType::Mana as i8,
            required_aura_spell_id: 0,
            optional_cost: 0,
            spell_id: 400,
        },
        SpellPowerEntry {
            id: 11,
            order_index: 2,
            mana_cost: 999,
            mana_cost_per_level: 0,
            mana_per_second: 0,
            power_display_id: 0,
            alt_power_bar_id: 0,
            power_cost_pct: 0.0,
            power_cost_max_pct: 0.0,
            power_pct_per_second: 0.0,
            power_type: PowerType::Mana as i8,
            required_aura_spell_id: 0,
            optional_cost: 0,
            spell_id: 400,
        },
    ]);
    let spell_power_difficulty =
        SpellPowerDifficultyStore::from_entries([SpellPowerDifficultyEntry {
            id: 11,
            difficulty_id: 1,
            order_index: 2,
        }]);

    store.apply_db2_power_costs_like_cpp(&spell_power, &spell_power_difficulty);

    let costs = &store.spells.get(&400).unwrap().power_costs;
    assert_eq!(costs.len(), 1, "non-default difficulty rows are skipped");
    assert_eq!(costs[0].order_index, 1);
    assert_eq!(costs[0].mana_cost, 40);
    assert_eq!(costs[0].mana_cost_per_level, 4);
    assert_eq!(costs[0].mana_per_second, 5);
    assert_eq!(costs[0].power_cost_pct, 10.0);
    assert_eq!(costs[0].power_pct_per_second, 6.5);
    assert_eq!(costs[0].power_type, PowerType::Mana as i8);
}
#[test]
fn spell_info_calc_power_costs_flat_plus_mana_pct_like_cpp() {
    let mut spell = SpellStore::empty_spell_info_like_cpp(500);
    spell.power_costs.push(SpellPowerCostInfoLikeCpp {
        order_index: 0,
        power_type: PowerType::Mana as i8,
        mana_cost: 50,
        mana_cost_per_level: 0,
        mana_per_second: 0,
        power_cost_pct: 12.5,
        power_cost_max_pct: 0.0,
        power_pct_per_second: 0.0,
        required_aura_spell_id: 0,
        optional_cost: 0,
    });

    let costs = spell.calc_power_costs_like_cpp(1000);

    assert_eq!(
        costs,
        vec![SpellPowerCostLikeCpp {
            power_type: PowerType::Mana as i8,
            amount: 175,
        }]
    );
}
#[test]
fn spell_info_calc_power_costs_ignores_mana_max_pct_like_cpp() {
    let mut spell = SpellStore::empty_spell_info_like_cpp(501);
    spell.power_costs.push(SpellPowerCostInfoLikeCpp {
        order_index: 0,
        power_type: PowerType::Mana as i8,
        mana_cost: 0,
        mana_cost_per_level: 0,
        mana_per_second: 0,
        power_cost_pct: 0.0,
        power_cost_max_pct: 18.0,
        power_pct_per_second: 0.0,
        required_aura_spell_id: 0,
        optional_cost: 0,
    });

    let costs = spell.calc_power_costs_like_cpp(1000);

    assert!(costs.is_empty());
}
#[test]
fn spell_store_db2_loader_composes_shapeshift_masks_like_cpp() {
    let spell_id = 70_001;
    let misc_store =
        crate::spell_db2::SpellMiscStore::from_entries([test_spell_misc_entry_like_cpp(
            1, spell_id, 0, 0,
        )]);
    let effect_store = crate::spell_db2::SpellEffectDb2Store::from_entries([]);
    let shapeshift_store = crate::spell_db2::SpellShapeshiftStore::from_entries([
        crate::spell_db2::SpellShapeshiftEntry {
            id: 1,
            spell_id: spell_id as i32,
            stance_bar_order: 0,
            shapeshift_exclude: [1 << 2, 0],
            shapeshift_mask: [1 << 4, 0],
        },
    ]);
    let form = shapeshift_form(shapeshift_form_flags::STANCE);
    let store = SpellStore::from_spell_db2_stores_like_cpp(
        &crate::spell_db2::SpellCategoriesStore::from_entries([]),
        &misc_store,
        &effect_store,
        &shapeshift_store,
    );

    assert_eq!(
        store.check_shapeshift_like_cpp(spell_id as i32, 3, |_| Some(&form)),
        Some(SpellCastResult::NotShapeshift)
    );
    assert_eq!(
        store.check_shapeshift_like_cpp(spell_id as i32, 5, |_| Some(&form)),
        Some(SpellCastResult::Success)
    );
    assert_eq!(
        store.check_shapeshift_like_cpp(spell_id as i32, 0, |_| None),
        Some(SpellCastResult::OnlyShapeshift)
    );
}
#[test]
fn spell_store_check_shapeshift_uses_spell_misc_attr2_like_cpp() {
    let spell_id = 70_002;
    let mut misc = test_spell_misc_entry_like_cpp(1, spell_id, 0, 0);
    misc.attributes[2] = attributes::SPELL_ATTR2_ALLOW_WHILE_NOT_SHAPESHIFTED_CASTER_FORM as i32;
    let misc_store = crate::spell_db2::SpellMiscStore::from_entries([misc]);
    let effect_store = crate::spell_db2::SpellEffectDb2Store::from_entries([]);
    let shapeshift_store = crate::spell_db2::SpellShapeshiftStore::from_entries([
        crate::spell_db2::SpellShapeshiftEntry {
            id: 2,
            spell_id: spell_id as i32,
            stance_bar_order: 0,
            shapeshift_exclude: [0, 0],
            shapeshift_mask: [1 << 4, 0],
        },
    ]);
    let store = SpellStore::from_spell_db2_stores_like_cpp(
        &crate::spell_db2::SpellCategoriesStore::from_entries([]),
        &misc_store,
        &effect_store,
        &shapeshift_store,
    );

    assert_eq!(
        store.check_shapeshift_like_cpp(spell_id as i32, 0, |_| None),
        Some(SpellCastResult::Success)
    );
}
#[test]
fn test_spell_info_effective_cooldown() {
    let spell = SpellInfo {
        spell_id: 100,
        cast_time_ms: 0,
        cooldown_ms: 1500,
        recovery_time_ms: 8000,
        effect_type: 2,
        effect_base_points: 50,
        effect_bonus_coefficient: 0.5,
        aura_type: None,
        display_flags: 0,
        requires_spell_focus: 0,
        power_costs: Vec::new(),
        effects: Vec::new(),
    };

    // recovery_time_ms is larger
    assert_eq!(spell.effective_cooldown_ms(), 8000);

    let instant = SpellInfo {
        spell_id: 100,
        cast_time_ms: 0,
        cooldown_ms: 1500,
        recovery_time_ms: 0,
        effect_type: 2,
        effect_base_points: 50,
        effect_bonus_coefficient: 0.5,
        aura_type: None,
        display_flags: 0,
        requires_spell_focus: 0,
        power_costs: Vec::new(),
        effects: Vec::new(),
    };

    // GCD is the limit
    assert_eq!(instant.effective_cooldown_ms(), 1500);
}
#[test]
fn spell_info_requires_spell_focus_matches_cpp_field() {
    let mut spell = SpellInfo {
        spell_id: 100,
        cast_time_ms: 0,
        cooldown_ms: 0,
        recovery_time_ms: 0,
        effect_type: 0,
        effect_base_points: 0,
        effect_bonus_coefficient: 0.0,
        aura_type: None,
        display_flags: 0,
        requires_spell_focus: 0,
        power_costs: Vec::new(),
        effects: Vec::new(),
    };

    assert!(!spell.requires_spell_focus_like_cpp());
    spell.requires_spell_focus = 181;
    assert!(spell.requires_spell_focus_like_cpp());
}
