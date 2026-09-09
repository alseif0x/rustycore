//! Pet lifecycle, aura and persistence state regression scenarios, part 3 of 4.
//!
//! Moved out of the pet.rs root under #636; every test is unchanged.

use super::*;

#[test]
fn pet_toggle_autocast_like_cpp_does_not_mark_new_spells_changed_or_disable_absent_autospell() {
    let mut pet = Pet::new(owner_guid(), PetType::Summon);
    assert!(pet.add_spell(
        123,
        ActiveState::Disabled,
        PetSpellState::New,
        PetSpellType::Normal
    ));

    let enabled = pet.toggle_autocast_like_cpp(123, true, true);
    assert!(enabled.autospell_added);
    assert!(enabled.active_changed);
    assert!(!enabled.marked_changed);
    assert_eq!(pet.spells().get(&123).unwrap().state, PetSpellState::New);

    assert!(pet.add_spell(
        456,
        ActiveState::Enabled,
        PetSpellState::Unchanged,
        PetSpellType::Normal
    ));
    pet.autospells.retain(|spell_id| *spell_id != 456);
    let disabled = pet.toggle_autocast_like_cpp(456, false, true);
    assert_eq!(
        disabled,
        PetToggleAutocastOutcomeLikeCpp {
            spell_found: true,
            autocastable: true,
            autospell_added: false,
            autospell_removed: false,
            active_changed: false,
            marked_changed: false,
        },
        "C++ ToggleAutocast(false) is a no-op when the spell is absent from m_autospells"
    );
    assert_eq!(pet.spells().get(&456).unwrap().active, ActiveState::Enabled);
}

#[test]
fn pet_cleanup_action_bar_matches_cpp_slot_filtering_and_autocast() {
    let mut pet = Pet::new(owner_guid(), PetType::Hunter);
    assert!(pet.add_spell(
        100,
        ActiveState::Disabled,
        PetSpellState::New,
        PetSpellType::Normal
    ));
    assert!(pet.add_spell(
        200,
        ActiveState::Disabled,
        PetSpellState::New,
        PetSpellType::Normal
    ));

    let mut action_bar = [0; MAX_UNIT_ACTION_BAR_INDEX];
    action_bar[0] = make_unit_action_button_like_cpp(1, crate::ACT_COMMAND_LIKE_CPP);
    action_bar[3] = make_unit_action_button_like_cpp(100, ACT_ENABLED_LIKE_CPP);
    action_bar[4] = make_unit_action_button_like_cpp(999, ACT_DISABLED_LIKE_CPP);
    action_bar[5] = make_unit_action_button_like_cpp(200, ACT_ENABLED_LIKE_CPP);
    action_bar[6] = make_unit_action_button_like_cpp(0, ACT_ENABLED_LIKE_CPP);

    let outcome = pet.cleanup_action_bar_like_cpp(&mut action_bar, |spell_id| spell_id == 100);

    assert_eq!(
        outcome.operations,
        vec![
            PetCleanupActionBarOperationLikeCpp::EnableAutocast {
                index: 3,
                spell_id: 100,
            },
            PetCleanupActionBarOperationLikeCpp::ClearSlot { index: 4 },
        ],
        "C++ walks slots in order, clears missing pet spells and toggles ACT_ENABLED spells only when SpellMgr has SpellInfo"
    );
    assert_eq!(
        action_bar[0],
        make_unit_action_button_like_cpp(1, crate::ACT_COMMAND_LIKE_CPP),
        "UnitActionBarEntry::IsActionBarForSpell skips command slots"
    );
    assert_eq!(
        action_bar[4],
        make_unit_action_button_like_cpp(0, ACT_PASSIVE_LIKE_CPP)
    );
    assert_eq!(outcome.action_bar, action_bar);
    assert_eq!(pet.autospells(), &[100]);
    assert_eq!(pet.spells().get(&100).unwrap().active, ActiveState::Enabled);
    assert_eq!(
        pet.spells().get(&200).unwrap().active,
        ActiveState::Disabled,
        "C++ does not ToggleAutocast when sSpellMgr->GetSpellInfo returns null"
    );
}

#[test]
fn pet_cleanup_action_bar_treats_removed_pet_spell_as_missing_like_cpp() {
    let mut pet = Pet::new(owner_guid(), PetType::Hunter);
    assert!(pet.add_spell(
        300,
        ActiveState::Enabled,
        PetSpellState::New,
        PetSpellType::Normal
    ));
    assert!(pet.remove_spell(300));

    let mut action_bar = [0; MAX_UNIT_ACTION_BAR_INDEX];
    action_bar[2] = make_unit_action_button_like_cpp(300, ACT_ENABLED_LIKE_CPP);

    let outcome = pet.cleanup_action_bar_like_cpp(&mut action_bar, |_| true);

    assert_eq!(
        outcome.operations,
        vec![PetCleanupActionBarOperationLikeCpp::ClearSlot { index: 2 }]
    );
    assert_eq!(
        action_bar[2],
        make_unit_action_button_like_cpp(0, ACT_PASSIVE_LIKE_CPP)
    );
    assert!(pet.autospells().is_empty());
}

#[test]
fn pet_learn_spell_high_rank_walks_next_spell_chain_like_cpp() {
    let mut pet = Pet::new(owner_guid(), PetType::Hunter);

    let outcome = pet.learn_spell_high_rank_like_cpp(100, |spell_id| match spell_id {
        100 => 200,
        200 => 300,
        300 => 0,
        _ => 0,
    });

    assert_eq!(outcome.attempted_spell_ids, vec![100, 200, 300]);
    assert_eq!(outcome.learned_spell_ids, vec![100, 200, 300]);
    assert!(pet.has_spell(100));
    assert!(pet.has_spell(200));
    assert!(pet.has_spell(300));
    assert_eq!(
        pet.spells().get(&300).unwrap().active,
        ActiveState::Disabled,
        "the current represented addSpell seam maps ACT_DECIDE to ACT_DISABLED until SpellInfo autocast/passive metadata is live"
    );
}

#[test]
fn pet_learn_spell_high_rank_continues_after_duplicate_like_cpp() {
    let mut pet = Pet::new(owner_guid(), PetType::Hunter);
    assert!(pet.add_spell(
        100,
        ActiveState::Disabled,
        PetSpellState::New,
        PetSpellType::Normal
    ));

    let outcome = pet.learn_spell_high_rank_like_cpp(100, |spell_id| match spell_id {
        100 => 200,
        200 => 0,
        _ => 0,
    });

    assert_eq!(
        outcome.attempted_spell_ids,
        vec![100, 200],
        "C++ calls GetNextSpellInChain after learnSpell(spellid), even if learnSpell was a no-op"
    );
    assert_eq!(outcome.learned_spell_ids, vec![200]);
    assert!(pet.has_spell(100));
    assert!(pet.has_spell(200));
}

#[test]
fn pet_add_spell_like_cpp_rejects_existing_spell_and_restores_removed_as_changed() {
    let mut pet = Pet::new(owner_guid(), PetType::Hunter);
    assert!(pet.add_spell(
        100,
        ActiveState::Disabled,
        PetSpellState::New,
        PetSpellType::Normal
    ));
    assert!(
        !pet.add_spell(
            100,
            ActiveState::Enabled,
            PetSpellState::New,
            PetSpellType::Family
        ),
        "C++ Pet::addSpell returns false for an existing non-removed spell"
    );
    assert_eq!(pet.spells().get(&100).unwrap().state, PetSpellState::New);
    assert_eq!(
        pet.spells().get(&100).unwrap().active,
        ActiveState::Disabled
    );
    assert_eq!(
        pet.spells().get(&100).unwrap().spell_type,
        PetSpellType::Normal
    );

    assert!(pet.remove_spell(100));
    assert!(pet.add_spell(
        100,
        ActiveState::Enabled,
        PetSpellState::New,
        PetSpellType::Normal
    ));
    assert_eq!(
        pet.spells().get(&100).unwrap().state,
        PetSpellState::Changed,
        "C++ promotes re-added PETSPELL_REMOVED entries to PETSPELL_CHANGED"
    );
    assert_eq!(pet.autospells(), &[100]);
}

#[test]
fn pet_add_spell_like_cpp_load_case_normalizes_existing_state_and_autocast() {
    let mut pet = Pet::new(owner_guid(), PetType::Hunter);
    assert!(pet.add_spell(
        100,
        ActiveState::Disabled,
        PetSpellState::New,
        PetSpellType::Normal
    ));

    assert!(
        !pet.add_spell(
            100,
            ActiveState::Enabled,
            PetSpellState::Unchanged,
            PetSpellType::Normal
        ),
        "C++ load case normalizes an existing learned spell but still returns false"
    );
    assert_eq!(
        pet.spells().get(&100).unwrap().state,
        PetSpellState::Unchanged
    );
    assert_eq!(pet.spells().get(&100).unwrap().active, ActiveState::Enabled);
    assert_eq!(pet.autospells(), &[100]);

    assert!(!pet.add_spell(
        100,
        ActiveState::Disabled,
        PetSpellState::Unchanged,
        PetSpellType::Normal
    ));
    assert_eq!(
        pet.spells().get(&100).unwrap().active,
        ActiveState::Enabled,
        "C++ only applies the load autocast correction while old state is not PETSPELL_UNCHANGED"
    );
    assert_eq!(pet.autospells(), &[100]);
}

#[test]
fn pet_learn_pet_passives_applies_family_spell_set_like_cpp() {
    let mut pet = Pet::new(owner_guid(), PetType::Hunter);

    let outcome = pet.learn_pet_passives_like_cpp(
        Some(7),
        |family| family == 7,
        |family| {
            assert_eq!(family, 7);
            vec![300, 100, 200, 100]
        },
    );

    assert_eq!(outcome.attempted_spell_ids, vec![100, 200, 300]);
    assert_eq!(outcome.learned_spell_ids, vec![100, 200, 300]);
    assert!(!outcome.creature_template_missing);
    assert!(!outcome.creature_family_missing);
    assert_eq!(
        pet.spells().get(&100).unwrap().spell_type,
        PetSpellType::Family
    );
    assert_eq!(
        pet.spells().get(&200).unwrap().spell_type,
        PetSpellType::Family
    );
    assert_eq!(
        pet.spells().get(&300).unwrap().spell_type,
        PetSpellType::Family
    );
}

#[test]
fn pet_learn_pet_passives_keeps_cpp_template_and_family_guards() {
    let mut pet = Pet::new(owner_guid(), PetType::Hunter);

    let missing_template = pet.learn_pet_passives_like_cpp(
        None,
        |_| panic!("C++ returns before CreatureFamily lookup when template is missing"),
        |_| panic!("C++ returns before PetFamilySpellsStore lookup when template is missing"),
    );
    assert!(missing_template.creature_template_missing);
    assert!(missing_template.attempted_spell_ids.is_empty());

    let missing_family = pet.learn_pet_passives_like_cpp(
        Some(99),
        |family| {
            assert_eq!(family, 99);
            false
        },
        |_| panic!("C++ returns before PetFamilySpellsStore lookup when family is missing"),
    );
    assert!(!missing_family.creature_template_missing);
    assert!(missing_family.creature_family_missing);
    assert!(pet.spells().is_empty());
}

#[test]
fn pet_learn_pet_passives_records_duplicate_noop_like_cpp_add_spell() {
    let mut pet = Pet::new(owner_guid(), PetType::Hunter);
    assert!(pet.add_spell(
        100,
        ActiveState::Disabled,
        PetSpellState::New,
        PetSpellType::Family
    ));

    let outcome =
        pet.learn_pet_passives_like_cpp(Some(7), |family| family == 7, |_| vec![100, 200]);

    assert_eq!(outcome.attempted_spell_ids, vec![100, 200]);
    assert_eq!(outcome.learned_spell_ids, vec![200]);
    assert_eq!(
        pet.spells().get(&100).unwrap().spell_type,
        PetSpellType::Family
    );
    assert_eq!(
        pet.spells().get(&200).unwrap().spell_type,
        PetSpellType::Family
    );
}

#[test]
fn pet_learn_pet_talent_like_cpp_is_currently_debug_log_only() {
    let mut pet = Pet::new(owner_guid(), PetType::Hunter);
    assert!(pet.add_spell(
        100,
        ActiveState::Enabled,
        PetSpellState::Unchanged,
        PetSpellType::Normal
    ));
    let autospells_before = pet.autospells().to_vec();
    let spells_before = pet.spells().clone();

    let outcome = pet.learn_pet_talent_like_cpp(42);

    assert_eq!(
        outcome,
        PetLearnPetTalentOutcomeLikeCpp {
            talent_id: 42,
            debug_log_only: true,
        },
        "local C++ Pet::LearnPetTalent only logs name/talent id and performs no mutation"
    );
    assert_eq!(pet.spells(), &spells_before);
    assert_eq!(pet.autospells(), autospells_before.as_slice());
}

#[test]
fn pet_cast_pet_auras_like_cpp_skips_non_permanent_pets() {
    let pet = Pet::new(owner_guid(), PetType::Summon);
    let outcome = pet.cast_pet_auras_like_cpp(
        false,
        Class::Warrior,
        CreatureType::Demon,
        &[PetAuraLikeCpp::new(0, 77, true, 0)],
        10.0,
        10.0,
    );

    assert!(outcome.skipped_not_permanent);
    assert!(outcome.removed_owner_pet_aura_indices.is_empty());
    assert!(outcome.removed_pet_aura_spell_ids.is_empty());
    assert!(outcome.cast_auras.is_empty());
}

#[test]
fn pet_cast_pet_auras_like_cpp_removes_on_pet_change_or_casts_in_owner_order() {
    let mut pet = Pet::new(owner_guid(), PetType::Summon);
    pet.creature_mut()
        .unit_mut()
        .world_mut()
        .object_mut()
        .set_entry(500);
    let owner_pet_auras = vec![
        PetAuraLikeCpp::new(500, 10, true, 0),
        PetAuraLikeCpp::new(0, 20, false, 0),
        PetAuraLikeCpp::new(700, 30, false, 0),
    ];

    let current_false = pet.cast_pet_auras_like_cpp(
        false,
        Class::Warlock,
        CreatureType::Demon,
        &owner_pet_auras,
        0.0,
        0.0,
    );
    assert!(!current_false.skipped_not_permanent);
    assert_eq!(current_false.removed_owner_pet_aura_indices, vec![0]);
    assert_eq!(current_false.removed_pet_aura_spell_ids, vec![10]);
    assert_eq!(
        current_false.cast_auras,
        vec![PetCastPetAuraPlanLikeCpp {
            owner_pet_aura_index: 1,
            aura_id: 20,
            spell_value_base_point0: None,
        }],
        "C++ falls back to petEntry 0 and ignores PetAura rows without exact or wildcard aura"
    );

    let current_true = pet.cast_pet_auras_like_cpp(
        true,
        Class::Warlock,
        CreatureType::Demon,
        &owner_pet_auras,
        0.0,
        0.0,
    );
    assert!(current_true.removed_owner_pet_aura_indices.is_empty());
    assert_eq!(
        current_true.cast_auras,
        vec![
            PetCastPetAuraPlanLikeCpp {
                owner_pet_aura_index: 0,
                aura_id: 10,
                spell_value_base_point0: None,
            },
            PetCastPetAuraPlanLikeCpp {
                owner_pet_aura_index: 1,
                aura_id: 20,
                spell_value_base_point0: None,
            },
        ]
    );
}

#[test]
fn pet_cast_pet_aura_like_cpp_applies_demonic_knowledge_basepoint_and_is_pet_aura() {
    let mut pet = Pet::new(owner_guid(), PetType::Hunter);
    pet.creature_mut()
        .unit_mut()
        .world_mut()
        .object_mut()
        .set_entry(500);
    let demonic_knowledge =
        PetAuraLikeCpp::new(0, 999, false, 15).with_aura(500, DEMONIC_KNOWLEDGE_AURA_LIKE_CPP);
    let other = PetAuraLikeCpp::new(0, 123, false, 0);

    let plan = pet
        .cast_pet_aura_like_cpp(4, &demonic_knowledge, 101.0, 50.0)
        .unwrap();
    assert_eq!(
        plan,
        PetCastPetAuraPlanLikeCpp {
            owner_pet_aura_index: 4,
            aura_id: DEMONIC_KNOWLEDGE_AURA_LIKE_CPP,
            spell_value_base_point0: Some(22),
        },
        "C++ CalculatePct(int32 damage, stamina + intellect) truncates toward zero"
    );
    assert!(pet.is_pet_aura_like_cpp(
        &[demonic_knowledge.clone(), other.clone()],
        DEMONIC_KNOWLEDGE_AURA_LIKE_CPP
    ));
    assert!(pet.is_pet_aura_like_cpp(&[demonic_knowledge, other], 123));
    assert!(!pet.is_pet_aura_like_cpp(&[], DEMONIC_KNOWLEDGE_AURA_LIKE_CPP));
}

#[test]
fn pet_init_create_spells_like_cpp_resets_action_bar_spellbook_and_runs_create_phases() {
    let mut pet = Pet::new(owner_guid(), PetType::Hunter);
    assert!(pet.add_spell(
        10,
        ActiveState::Enabled,
        PetSpellState::Unchanged,
        PetSpellType::Normal
    ));
    assert!(pet.add_spell(
        20,
        ActiveState::Disabled,
        PetSpellState::Unchanged,
        PetSpellType::Normal
    ));
    let mut charm_info = CharmInfoState::default();
    charm_info.action_bar[0] = make_unit_action_button_like_cpp(999, ACT_PASSIVE_LIKE_CPP);

    let outcome = pet.init_pet_create_spells_like_cpp(
        &mut charm_info,
        Some(7),
        |family| family == 7,
        |family| {
            assert_eq!(family, 7);
            vec![300, 100, 200]
        },
    );

    let mut expected_charm = CharmInfoState::default();
    expected_charm.init_pet_action_bar_like_cpp();
    assert_eq!(charm_info.action_bar, expected_charm.action_bar);
    assert_eq!(outcome.cleared_spell_count, 2);
    assert_eq!(outcome.cleared_autospell_count, 1);
    assert_eq!(
        outcome.learn_pet_passives.attempted_spell_ids,
        vec![100, 200, 300]
    );
    assert_eq!(
        outcome.learn_pet_passives.learned_spell_ids,
        vec![100, 200, 300]
    );
    assert!(outcome.init_pet_action_bar);
    assert!(outcome.init_levelup_spells_for_level);
    assert!(
        !outcome.cast_pet_auras_current,
        "C++ Pet::InitPetCreateSpells ends with CastPetAuras(false)"
    );
    assert_eq!(pet.autospells(), &[]);
    assert_eq!(pet.spells().len(), 3);
    assert!(
        pet.spells()
            .values()
            .all(|spell| spell.spell_type == PetSpellType::Family)
    );
}

#[test]
fn pet_init_create_spells_like_cpp_still_runs_levelup_and_auras_when_passives_skip() {
    let mut pet = Pet::new(owner_guid(), PetType::Summon);
    let mut charm_info = CharmInfoState::default();

    let outcome = pet.init_pet_create_spells_like_cpp(
        &mut charm_info,
        None,
        |_| panic!("C++ LearnPetPassives returns before family lookup without template family"),
        |_| panic!("C++ LearnPetPassives returns before spell set lookup without template family"),
    );

    assert!(outcome.learn_pet_passives.creature_template_missing);
    assert!(outcome.init_levelup_spells_for_level);
    assert!(!outcome.cast_pet_auras_current);
    assert!(pet.spells().is_empty());
}

#[test]
fn pet_learn_spell_like_cpp_sends_and_initializes_only_after_new_spell() {
    let mut pet = Pet::new(owner_guid(), PetType::Hunter);

    let learned = pet.learn_spell_like_cpp(100);
    assert_eq!(
        learned,
        PetLearnSpellOutcomeLikeCpp {
            learned: true,
            packet_spell_ids: vec![100],
            send_direct_message: true,
            pet_spell_initialize: true,
        }
    );

    let duplicate = pet.learn_spell_like_cpp(100);
    assert_eq!(
        duplicate,
        PetLearnSpellOutcomeLikeCpp {
            learned: false,
            packet_spell_ids: Vec::new(),
            send_direct_message: false,
            pet_spell_initialize: false,
        },
        "C++ Pet::learnSpell returns before sending when addSpell rejects a duplicate"
    );

    let mut loading_pet = Pet::new(owner_guid(), PetType::Hunter);
    loading_pet.set_loading(true);
    let loading = loading_pet.learn_spell_like_cpp(200);
    assert_eq!(loading.packet_spell_ids, vec![200]);
    assert!(!loading.send_direct_message);
    assert!(!loading.pet_spell_initialize);
}

#[test]
fn pet_learn_spells_like_cpp_batches_and_sends_even_empty_when_loaded() {
    let mut pet = Pet::new(owner_guid(), PetType::Hunter);
    assert!(pet.add_spell(
        100,
        ActiveState::Disabled,
        PetSpellState::Unchanged,
        PetSpellType::Normal
    ));

    let outcome = pet.learn_spells_like_cpp([100, 200, 300, 200]);
    assert_eq!(outcome.learned_spell_ids, vec![200, 300]);
    assert_eq!(outcome.packet_spell_ids, vec![200, 300]);
    assert!(outcome.send_session_packet);

    let empty = pet.learn_spells_like_cpp([100, 200, 300]);
    assert!(empty.learned_spell_ids.is_empty());
    assert!(empty.packet_spell_ids.is_empty());
    assert!(
        empty.send_session_packet,
        "C++ Pet::learnSpells sends the packet after the loop whenever !m_loading"
    );
}

#[test]
fn pet_learn_spells_like_cpp_loading_batches_without_sending() {
    let mut pet = Pet::new(owner_guid(), PetType::Hunter);
    pet.set_loading(true);

    let outcome = pet.learn_spells_like_cpp([100, 200]);
    assert_eq!(outcome.learned_spell_ids, vec![100, 200]);
    assert_eq!(outcome.packet_spell_ids, vec![100, 200]);
    assert!(!outcome.send_session_packet);
}

#[test]
fn pet_remove_spell_like_cpp_erases_new_and_marks_persisted() {
    let mut pet = Pet::new(owner_guid(), PetType::Hunter);
    assert!(pet.add_spell(
        100,
        ActiveState::Disabled,
        PetSpellState::New,
        PetSpellType::Normal
    ));
    assert!(pet.add_spell(
        200,
        ActiveState::Enabled,
        PetSpellState::Unchanged,
        PetSpellType::Normal
    ));
    let mut action_bar = [0; MAX_UNIT_ACTION_BAR_INDEX];

    let new_outcome = pet.remove_spell_like_cpp(
        100,
        false,
        false,
        &mut action_bar,
        |_| 0,
        |spell_id| spell_id,
    );
    assert_eq!(
        new_outcome,
        PetRemoveSpellOutcomeLikeCpp {
            removed: true,
            erased_new_spell: true,
            marked_removed: false,
            remove_auras_due_to_spell: true,
            learned_prev_spell_id: None,
            cleared_action_bar_slot: None,
            pet_spell_initialize: false,
        }
    );
    assert!(!pet.spells().contains_key(&100));

    let persisted_outcome = pet.remove_spell_like_cpp(
        200,
        false,
        false,
        &mut action_bar,
        |_| 0,
        |spell_id| spell_id,
    );
    assert!(persisted_outcome.marked_removed);
    assert!(!persisted_outcome.erased_new_spell);
    assert_eq!(
        pet.spells().get(&200).unwrap().state,
        PetSpellState::Removed
    );
    assert!(
        pet.autospells().is_empty(),
        "C++ ToggleAutocast state must not survive removing a learned spell"
    );
}

#[test]
fn pet_remove_spell_like_cpp_learns_previous_rank_without_clearing_action_bar() {
    let mut pet = Pet::new(owner_guid(), PetType::Hunter);
    assert!(pet.add_spell(
        300,
        ActiveState::Disabled,
        PetSpellState::Unchanged,
        PetSpellType::Normal
    ));
    let mut action_bar = [0; MAX_UNIT_ACTION_BAR_INDEX];
    action_bar[3] = make_unit_action_button_like_cpp(300, ACT_ENABLED_LIKE_CPP);

    let outcome = pet.remove_spell_like_cpp(
        300,
        true,
        true,
        &mut action_bar,
        |spell_id| (spell_id == 300).then_some(200).unwrap_or_default(),
        |spell_id| match spell_id {
            200 | 300 => 100,
            _ => spell_id,
        },
    );

    assert_eq!(outcome.learned_prev_spell_id, Some(200));
    assert_eq!(outcome.cleared_action_bar_slot, None);
    assert_eq!(
        action_bar[3],
        make_unit_action_button_like_cpp(300, ACT_ENABLED_LIKE_CPP),
        "C++ skips RemoveSpellFromActionBar when learn_prev remains true"
    );
    assert!(pet.has_spell(200));
}

#[test]
fn pet_remove_spell_like_cpp_clears_action_bar_when_no_previous_rank() {
    let mut pet = Pet::new(owner_guid(), PetType::Hunter);
    assert!(pet.add_spell(
        300,
        ActiveState::Disabled,
        PetSpellState::Unchanged,
        PetSpellType::Normal
    ));
    let mut action_bar = [0; MAX_UNIT_ACTION_BAR_INDEX];
    action_bar[1] = make_unit_action_button_like_cpp(1, crate::ACT_COMMAND_LIKE_CPP);
    action_bar[3] = make_unit_action_button_like_cpp(301, ACT_ENABLED_LIKE_CPP);
    action_bar[4] = make_unit_action_button_like_cpp(300, ACT_DISABLED_LIKE_CPP);

    let outcome = pet.remove_spell_like_cpp(
        300,
        true,
        true,
        &mut action_bar,
        |_| 0,
        |spell_id| match spell_id {
            300 | 301 => 100,
            _ => spell_id,
        },
    );

    assert_eq!(outcome.learned_prev_spell_id, None);
    assert_eq!(outcome.cleared_action_bar_slot, Some(3));
    assert!(outcome.pet_spell_initialize);
    assert_eq!(
        action_bar[3],
        make_unit_action_button_like_cpp(0, ACT_PASSIVE_LIKE_CPP)
    );
    assert_eq!(
        action_bar[4],
        make_unit_action_button_like_cpp(300, ACT_DISABLED_LIKE_CPP),
        "C++ stops after clearing the first action bar slot in the same chain"
    );
}

#[test]
fn pet_remove_spell_like_cpp_missing_or_already_removed_is_noop() {
    let mut pet = Pet::new(owner_guid(), PetType::Hunter);
    assert!(pet.add_spell(
        100,
        ActiveState::Disabled,
        PetSpellState::Removed,
        PetSpellType::Normal
    ));
    let mut action_bar = [0; MAX_UNIT_ACTION_BAR_INDEX];

    assert_eq!(
        pet.remove_spell_like_cpp(999, true, true, &mut action_bar, |_| 1, |spell_id| spell_id),
        PetRemoveSpellOutcomeLikeCpp::not_removed()
    );
    assert_eq!(
        pet.remove_spell_like_cpp(100, true, true, &mut action_bar, |_| 1, |spell_id| spell_id),
        PetRemoveSpellOutcomeLikeCpp::not_removed()
    );
}

#[test]
fn pet_unlearn_spell_like_cpp_sends_single_packet_only_when_removed_and_not_loading() {
    let mut pet = Pet::new(owner_guid(), PetType::Hunter);
    assert!(pet.add_spell(
        100,
        ActiveState::Disabled,
        PetSpellState::Unchanged,
        PetSpellType::Normal
    ));
    let mut action_bar = [0; MAX_UNIT_ACTION_BAR_INDEX];

    let outcome = pet.unlearn_spell_like_cpp(
        100,
        false,
        false,
        &mut action_bar,
        |_| 0,
        |spell_id| spell_id,
    );

    assert!(outcome.remove_spell.removed);
    assert_eq!(outcome.packet_spell_ids, vec![100]);
    assert!(
        outcome.send_direct_message,
        "C++ Pet::unlearnSpell sends PetUnlearnedSpells only after removeSpell succeeds"
    );

    let missing = pet.unlearn_spell_like_cpp(
        999,
        false,
        false,
        &mut action_bar,
        |_| 0,
        |spell_id| spell_id,
    );
    assert!(!missing.remove_spell.removed);
    assert!(missing.packet_spell_ids.is_empty());
    assert!(!missing.send_direct_message);

    let mut loading_pet = Pet::new(owner_guid(), PetType::Hunter);
    loading_pet.set_loading(true);
    assert!(loading_pet.add_spell(
        200,
        ActiveState::Disabled,
        PetSpellState::Unchanged,
        PetSpellType::Normal
    ));
    let loading = loading_pet.unlearn_spell_like_cpp(
        200,
        false,
        false,
        &mut action_bar,
        |_| 0,
        |spell_id| spell_id,
    );
    assert_eq!(loading.packet_spell_ids, vec![200]);
    assert!(
        !loading.send_direct_message,
        "C++ removes while loading but suppresses the direct message"
    );
}

#[test]
fn pet_unlearn_spells_like_cpp_batches_removed_spells_and_sends_even_empty_when_loaded() {
    let mut pet = Pet::new(owner_guid(), PetType::Hunter);
    assert!(pet.add_spell(
        100,
        ActiveState::Disabled,
        PetSpellState::Unchanged,
        PetSpellType::Normal
    ));
    assert!(pet.add_spell(
        200,
        ActiveState::Disabled,
        PetSpellState::Removed,
        PetSpellType::Normal
    ));
    assert!(pet.add_spell(
        300,
        ActiveState::Disabled,
        PetSpellState::New,
        PetSpellType::Normal
    ));
    let mut action_bar = [0; MAX_UNIT_ACTION_BAR_INDEX];

    let outcome = pet.unlearn_spells_like_cpp(
        [100, 200, 999, 300],
        false,
        false,
        &mut action_bar,
        |_| 0,
        |spell_id| spell_id,
    );

    assert_eq!(outcome.packet_spell_ids, vec![100, 300]);
    assert!(outcome.send_session_packet);
    assert_eq!(outcome.remove_spells.len(), 4);
    assert!(
        !outcome.remove_spells[1].1.removed,
        "C++ skips already PETSPELL_REMOVED entries inside the batch"
    );
    assert!(
        !outcome.remove_spells[2].1.removed,
        "C++ skips missing entries inside the batch"
    );

    let empty = pet.unlearn_spells_like_cpp(
        [200, 999],
        false,
        false,
        &mut action_bar,
        |_| 0,
        |spell_id| spell_id,
    );
    assert!(empty.packet_spell_ids.is_empty());
    assert!(
        empty.send_session_packet,
        "C++ Pet::unlearnSpells sends the packet after the loop whenever !m_loading"
    );
}
