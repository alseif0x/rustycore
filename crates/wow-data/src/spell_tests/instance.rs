//! Instance scenarios for [`super`].
//!
//! Split out of spell_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn effective_effect_store_is_the_only_difficulty_zero_effect_authority_like_cpp() {
    let spell_id = 100u32;
    let build = |effects: Vec<crate::spell_db2::SpellEffectDb2Entry>| {
        SpellStore::from_spell_db2_stores_like_cpp(
            &crate::spell_db2::SpellCategoriesStore::from_entries([]),
            &crate::spell_db2::SpellMiscStore::from_entries([test_spell_misc_entry_like_cpp(
                1, spell_id, 0, 0,
            )]),
            &crate::spell_db2::SpellEffectDb2Store::from_entries(effects),
            &crate::spell_db2::SpellShapeshiftStore::from_entries([]),
        )
    };
    let effect = |id, effect_index, effect_aura: i32| {
        let mut entry = test_spell_effect_db2_entry_like_cpp(
            id,
            spell_id,
            0,
            effect_index,
            spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            0,
        );
        entry.effect_aura = effect_aura as i16;
        entry
    };

    let before_tombstone = build(vec![
        effect(1, 0, aura_types::SPELL_AURA_MOD_THREAT),
        effect(2, 1, aura_types::SPELL_AURA_MOD_TAUNT),
    ]);
    assert_eq!(
        before_tombstone
            .effects_for_difficulty_like_cpp(spell_id as i32, 0, None)
            .expect("both effective rows")
            .len(),
        2
    );

    // A final `hotfix_data` tombstone leaves the effective store without
    // record 2. No other authority may reintroduce that effect index.
    let after_tombstone = build(vec![effect(1, 0, aura_types::SPELL_AURA_MOD_THREAT)]);
    let effects = after_tombstone
        .effects_for_difficulty_like_cpp(spell_id as i32, 0, None)
        .expect("surviving effective row");
    assert_eq!(effects.len(), 1);
    assert_eq!(effects[0].effect_index, 0);
    assert_eq!(effects[0].effect_aura, aura_types::SPELL_AURA_MOD_THREAT);

    let spell = after_tombstone
        .get(spell_id as i32)
        .expect("spell from effective SpellMisc");
    assert_eq!(spell.effects.len(), 1);
    assert_eq!(spell.aura_type, Some(aura_types::SPELL_AURA_MOD_THREAT));
}
#[test]
fn misc_attributes_resolve_exact_difficulty_then_base_like_cpp() {
    let mut store = SpellStore::new();
    let mut base = [0; 15];
    base[1] = attributes::SPELL_ATTR1_NO_THREAT;
    store.insert_spell_misc_attributes_like_cpp(100, base);
    let mut heroic = [0; 15];
    heroic[4] = attributes::SPELL_ATTR4_NO_HARMFUL_THREAT;
    store.insert_spell_misc_attributes_for_difficulty_like_cpp(100, 2, heroic);

    assert!(store.has_attribute_for_difficulty_like_cpp(
        100,
        2,
        None,
        4,
        attributes::SPELL_ATTR4_NO_HARMFUL_THREAT,
    ));
    assert!(!store.has_attribute_for_difficulty_like_cpp(
        100,
        2,
        None,
        1,
        attributes::SPELL_ATTR1_NO_THREAT,
    ));
    assert!(store.has_attribute_for_difficulty_like_cpp(
        100,
        3,
        None,
        1,
        attributes::SPELL_ATTR1_NO_THREAT,
    ));
}
#[test]
fn difficulty_none_existence_composes_exact_regular_and_serverside_keys_like_cpp() {
    let mut regular = SpellStore::new();
    regular.spell_info_keys_like_cpp =
        crate::spell_info_keys::SpellInfoKeyStoreLikeCpp::from_candidate_keys_like_cpp(
            [(100, 0), (101, 2), (300, 2)],
            &HashSet::from([100, 101, 300]),
        );
    let serverside = ServersideSpellStoreLikeCpp::from_rows_like_cpp(
        [
            serverside_spell_row(200, 0),
            serverside_spell_row(201, 2),
            serverside_spell_row(400, 3),
        ],
        &ServersideSpellEffectStoreLikeCpp::default(),
        |_| false,
    );
    assert!(serverside.errors.is_empty());
    let no_fallback = crate::DifficultyStore::from_entries([]);

    assert!(
        regular.contains_spell_info_difficulty_none_like_cpp(&serverside.store, &no_fallback, 100),
        "an exact regular difficulty-zero key is visible even without hydrated payload"
    );
    assert!(
        !regular.contains_spell_info_difficulty_none_like_cpp(&serverside.store, &no_fallback, 101),
        "a regular key that exists only at another difficulty is not a trainer spell"
    );
    assert!(
        regular.contains_spell_info_difficulty_none_like_cpp(&serverside.store, &no_fallback, 200),
        "an exact server-side difficulty-zero key shares C++ GetSpellInfo authority"
    );
    assert!(
        !regular.contains_spell_info_difficulty_none_like_cpp(&serverside.store, &no_fallback, 201),
        "a server-side key that exists only at another difficulty is not a trainer spell"
    );

    let trainer = crate::trainer::TrainerStoreLikeCpp::from_rows_like_cpp(
        [crate::trainer::TrainerRowLikeCpp {
            id: 10,
            trainer_type: crate::trainer::TRAINER_TYPE_TRADESKILL_LIKE_CPP,
            greeting: String::new(),
        }],
        [100, 101, 200, 201].map(|spell_id| crate::trainer::TrainerSpellRowLikeCpp {
            trainer_id: 10,
            spell: crate::trainer::TrainerSpellLikeCpp {
                spell_id,
                money_cost: 0,
                req_skill_line: 0,
                req_skill_rank: 0,
                req_ability: [0; 3],
                req_level: 0,
            },
        }),
        [],
        [],
        |spell_id| {
            regular.contains_spell_info_difficulty_none_like_cpp(
                &serverside.store,
                &no_fallback,
                spell_id,
            )
        },
        |_| true,
        |_| true,
        |_, _| true,
    );
    let loaded = trainer.store.get_trainer_like_cpp(10).unwrap();
    assert!(loaded.get_spell_like_cpp(100).is_some());
    assert!(loaded.get_spell_like_cpp(200).is_some());
    assert!(loaded.get_spell_like_cpp(101).is_none());
    assert!(loaded.get_spell_like_cpp(201).is_none());
    assert_eq!(
        trainer.report.skipped_spells_missing_spell,
        vec![(10, 101), (10, 201)]
    );

    let difficulty_fallbacks = crate::DifficultyStore::from_entries([
        crate::DifficultyEntry {
            id: 0,
            instance_type: 0,
            flags: 0,
            fallback_difficulty_id: 2,
            toggle_difficulty_id: 0,
        },
        crate::DifficultyEntry {
            id: 2,
            instance_type: 0,
            flags: 0,
            fallback_difficulty_id: 3,
            toggle_difficulty_id: 0,
        },
        crate::DifficultyEntry {
            id: 3,
            instance_type: 0,
            flags: 0,
            fallback_difficulty_id: 0,
            toggle_difficulty_id: 0,
        },
    ]);
    assert!(
        regular.contains_spell_info_difficulty_none_like_cpp(
            &serverside.store,
            &difficulty_fallbacks,
            300
        ),
        "a custom Difficulty(0) fallback reaches a regular spell like C++"
    );
    assert!(
        regular.contains_spell_info_difficulty_none_like_cpp(
            &serverside.store,
            &difficulty_fallbacks,
            400
        ),
        "the fallback chain reaches a server-side spell like C++"
    );
    assert!(
        !regular.contains_spell_info_difficulty_none_like_cpp(
            &serverside.store,
            &difficulty_fallbacks,
            999
        ),
        "invalid custom fallback cycles terminate instead of hanging startup"
    );
}
