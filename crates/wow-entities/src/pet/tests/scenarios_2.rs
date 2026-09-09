//! Pet lifecycle, aura and persistence state regression scenarios, part 2 of 4.
//!
//! Moved out of the pet.rs root under #636; every test is unchanged.

use super::*;

#[test]
fn pet_synchronize_level_with_owner_delegates_to_give_pet_level_like_cpp() {
    let mut hunter = Pet::new(owner_guid(), PetType::Hunter);
    hunter.creature_mut().unit_mut().set_level(10);
    hunter.set_pet_experience(77);
    hunter.set_pet_next_level_experience(100);

    assert_eq!(
        hunter.synchronize_level_with_owner_like_cpp(12, |level| u32::from(level) * 1_000),
        Some(PetLevelUpdateOutcome {
            changed: true,
            reset_experience: true,
            refresh_stats: true,
            init_levelup_spells: true,
        }),
        "C++ Pet::SynchronizeLevelWithOwner calls GivePetLevel for HUNTER_PET"
    );
    assert_eq!(hunter.creature().level(), 12);
    assert_eq!(hunter.pet_experience(), 0);
    assert_eq!(hunter.pet_next_level_experience(), 600);

    let mut summon = Pet::new(owner_guid(), PetType::Summon);
    summon.creature_mut().unit_mut().set_level(4);
    assert_eq!(
        summon.synchronize_level_with_owner_like_cpp(8, |_| 999),
        Some(PetLevelUpdateOutcome {
            changed: true,
            reset_experience: false,
            refresh_stats: true,
            init_levelup_spells: true,
        }),
        "C++ also synchronizes SUMMON_PET, but GivePetLevel does not reset hunter XP fields"
    );
    assert_eq!(summon.creature().level(), 8);

    let mut max = Pet::new(owner_guid(), PetType::Max);
    max.creature_mut().unit_mut().set_level(3);
    assert_eq!(max.synchronize_level_with_owner_like_cpp(9, |_| 999), None);
    assert_eq!(
        max.creature().level(),
        3,
        "C++ default branch leaves non summon/hunter pet types unchanged"
    );
}

#[test]
fn pet_give_xp_matches_cpp_gates_and_level_rollover() {
    let mut pet = Pet::new(owner_guid(), PetType::Hunter);
    pet.creature_mut().unit_mut().set_level(10);
    pet.set_pet_experience(90);
    pet.set_pet_next_level_experience(100);

    let outcome = pet.give_pet_xp_like_cpp(700, 80, 12, |level| u32::from(level) * 1_000);

    assert_eq!(outcome.accepted, true);
    assert_eq!(outcome.levels_gained, 2);
    assert_eq!(
        outcome.level_update,
        PetLevelUpdateOutcome {
            changed: true,
            reset_experience: true,
            refresh_stats: true,
            init_levelup_spells: true
        }
    );
    assert_eq!(pet.creature().level(), 12);
    assert_eq!(
        pet.pet_experience(),
        0,
        "C++ clears pet XP when the pet reaches min(max-player-level, owner-level)"
    );
    assert_eq!(pet.pet_next_level_experience(), 600);

    let mut summon = Pet::new(owner_guid(), PetType::Summon);
    summon.creature_mut().unit_mut().set_level(10);
    assert_eq!(
        summon.give_pet_xp_like_cpp(1, 80, 80, |_| 100).accepted,
        false
    );

    let mut dead = Pet::new(owner_guid(), PetType::Hunter);
    dead.creature_mut().unit_mut().set_level(10);
    dead.creature_mut()
        .set_death_state_runtime(DeathState::JustDied, 1_000);
    assert_eq!(
        dead.give_pet_xp_like_cpp(1, 80, 80, |_| 100).accepted,
        false
    );
}

#[test]
fn pet_give_xp_handles_zero_next_level_xp_like_cpp_initialized_field_assumption() {
    let mut pet = Pet::new(owner_guid(), PetType::Hunter);
    pet.creature_mut().unit_mut().set_level(10);
    pet.set_pet_experience(0);
    pet.set_pet_next_level_experience(0);

    let outcome = pet.give_pet_xp_like_cpp(600, 80, 12, |level| u32::from(level) * 1_000);

    assert_eq!(outcome.levels_gained, 2);
    assert_eq!(pet.creature().level(), 12);
    assert_eq!(pet.pet_experience(), 0);
}

#[test]
fn pet_give_xp_uses_cpp_uint32_wrapping_sum() {
    let mut pet = Pet::new(owner_guid(), PetType::Hunter);
    pet.creature_mut().unit_mut().set_level(10);
    pet.set_pet_experience(u32::MAX);
    pet.set_pet_next_level_experience(100);

    let outcome = pet.give_pet_xp_like_cpp(2, 80, 80, |_| 1_000);

    assert!(outcome.accepted);
    assert_eq!(outcome.levels_gained, 0);
    assert_eq!(pet.creature().level(), 10);
    assert_eq!(pet.pet_experience(), 1);
}

#[test]
fn pet_specialization_spell_plans_match_cpp_filters_and_order() {
    let learned = [
        PetSpecializationSpellLikeCpp {
            spell_id: 101,
            spell_exists: true,
            spell_level: 9,
        },
        PetSpecializationSpellLikeCpp {
            spell_id: 102,
            spell_exists: false,
            spell_level: 1,
        },
        PetSpecializationSpellLikeCpp {
            spell_id: 103,
            spell_exists: true,
            spell_level: 12,
        },
    ];
    assert_eq!(
        Pet::learn_specialization_spells_plan_like_cpp(10, &learned),
        vec![101],
        "C++ skips missing SpellInfo and spells above pet level"
    );

    let normal_specs: [&[u32]; PET_MAX_SPECIALIZATIONS_LIKE_CPP] =
        [&[11, 12], &[], &[31], &[41, 42]];
    let override_specs: [&[u32]; PET_MAX_SPECIALIZATIONS_LIKE_CPP] =
        [&[111], &[221, 222], &[], &[441]];
    assert_eq!(
        Pet::remove_specialization_spells_plan_like_cpp(&normal_specs, &override_specs),
        vec![11, 12, 111, 221, 222, 31, 41, 42, 441],
        "C++ loops index 0..MAX_SPECIALIZATIONS and appends normal then override spec spells"
    );
}

#[test]
fn pet_set_specialization_like_cpp_preserves_cpp_side_effect_order() {
    let normal_specs: [&[u32]; PET_MAX_SPECIALIZATIONS_LIKE_CPP] = [&[11], &[21, 22], &[], &[41]];
    let override_specs: [&[u32]; PET_MAX_SPECIALIZATIONS_LIKE_CPP] =
        [&[111], &[], &[331], &[441, 442]];
    let learned = [
        PetSpecializationSpellLikeCpp {
            spell_id: 501,
            spell_exists: true,
            spell_level: 8,
        },
        PetSpecializationSpellLikeCpp {
            spell_id: 502,
            spell_exists: true,
            spell_level: 20,
        },
    ];

    let mut unchanged = Pet::new(owner_guid(), PetType::Hunter);
    unchanged.set_specialization(7);
    assert_eq!(
        unchanged.set_specialization_like_cpp(7, true, &learned, &normal_specs, &override_specs),
        PetSetSpecializationOutcomeLikeCpp {
            changed: false,
            removed_specialization_spells: Vec::new(),
            remove_learn_prev: false,
            remove_clear_action_bar: false,
            learned_specialization_spells: Vec::new(),
            cleanup_action_bar: false,
            pet_spell_initialize: false,
            packet_spec_id: None,
        },
        "C++ returns before removing old specialization spells when spec is unchanged"
    );

    let mut invalid = Pet::new(owner_guid(), PetType::Hunter);
    invalid.set_specialization(3);
    invalid.creature_mut().unit_mut().set_level(10);
    assert_eq!(
        invalid.set_specialization_like_cpp(999, false, &learned, &normal_specs, &override_specs),
        PetSetSpecializationOutcomeLikeCpp {
            changed: true,
            removed_specialization_spells: vec![11, 111, 21, 22, 331, 41, 441, 442],
            remove_learn_prev: true,
            remove_clear_action_bar: false,
            learned_specialization_spells: Vec::new(),
            cleanup_action_bar: false,
            pet_spell_initialize: false,
            packet_spec_id: None,
        },
        "C++ removes old spec spells before LookupEntry(spec), then sets specialization to 0 and returns"
    );
    assert_eq!(invalid.specialization(), 0);

    let mut valid = Pet::new(owner_guid(), PetType::Hunter);
    valid.set_specialization(3);
    valid.creature_mut().unit_mut().set_level(10);
    assert_eq!(
        valid.set_specialization_like_cpp(7, true, &learned, &normal_specs, &override_specs),
        PetSetSpecializationOutcomeLikeCpp {
            changed: true,
            removed_specialization_spells: vec![11, 111, 21, 22, 331, 41, 441, 442],
            remove_learn_prev: true,
            remove_clear_action_bar: false,
            learned_specialization_spells: vec![501],
            cleanup_action_bar: true,
            pet_spell_initialize: true,
            packet_spec_id: Some(7),
        }
    );
    assert_eq!(valid.specialization(), 7);
}

#[test]
fn pet_generate_action_bar_data_matches_cpp_type_action_format() {
    let mut action_bar = [0u32; 10];
    action_bar[0] = crate::make_unit_action_button_like_cpp(
        crate::COMMAND_ATTACK_LIKE_CPP,
        crate::ACT_COMMAND_LIKE_CPP,
    );
    action_bar[3] = crate::make_unit_action_button_like_cpp(12_345, crate::ACT_ENABLED_LIKE_CPP);
    action_bar[4] = crate::make_unit_action_button_like_cpp(23_456, crate::ACT_DISABLED_LIKE_CPP);
    action_bar[9] = crate::make_unit_action_button_like_cpp(
        crate::COMMAND_STAY_LIKE_CPP,
        crate::ACT_REACTION_LIKE_CPP,
    );

    let expected = action_bar
        .iter()
        .map(|packed| {
            format!(
                "{} {} ",
                unit_action_button_type_like_cpp(*packed),
                unit_action_button_action_like_cpp(*packed)
            )
        })
        .collect::<String>();

    assert_eq!(
        Pet::generate_action_bar_data_like_cpp(&action_bar),
        expected
    );
    assert!(
        expected.ends_with(' '),
        "C++ ostream appends a trailing space after every action-bar entry"
    );
}

#[test]
fn pet_fill_pet_info_matches_cpp_field_copy() {
    let mut pet = Pet::new(owner_guid(), PetType::Hunter);
    pet.creature_mut()
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(pet_guid(21));
    pet.creature_mut()
        .unit_mut()
        .world_mut()
        .object_mut()
        .set_entry(500);
    pet.creature_mut().unit_mut().world_mut().set_name("Misha");
    pet.creature_mut().set_display_id(12_345, true, None);
    pet.creature_mut().unit_mut().set_level(43);
    pet.creature_mut().unit_mut().set_max_health(1_000);
    pet.creature_mut().unit_mut().set_health(888);
    pet.creature_mut().set_power_type(PowerType::Mana);
    pet.creature_mut()
        .unit_mut()
        .set_max_power(PowerType::Mana, 500);
    pet.creature_mut()
        .unit_mut()
        .set_power(PowerType::Mana, 222);
    pet.creature_mut().set_react_state(ReactState::Defensive);
    pet.set_pet_experience(777);
    pet.set_specialization(12);

    let mut action_bar = [0u32; 10];
    action_bar[0] = crate::make_unit_action_button_like_cpp(
        crate::COMMAND_ATTACK_LIKE_CPP,
        crate::ACT_COMMAND_LIKE_CPP,
    );
    action_bar[3] = crate::make_unit_action_button_like_cpp(11_111, crate::ACT_ENABLED_LIKE_CPP);

    let forced = pet.fill_pet_info_like_cpp(
        42,
        &action_bar,
        Some(ReactState::Passive),
        false,
        1_700_000_000,
        9_001,
    );

    assert_eq!(forced.name, "Misha");
    assert_eq!(
        forced.action_bar,
        Pet::generate_action_bar_data_like_cpp(&action_bar)
    );
    assert_eq!(forced.pet_number, 42);
    assert_eq!(forced.creature_id, 500);
    assert_eq!(forced.display_id, 12_345);
    assert_eq!(forced.experience, 777);
    assert_eq!(forced.health, 888);
    assert_eq!(forced.mana, 222);
    assert_eq!(forced.last_save_time, 1_700_000_000);
    assert_eq!(forced.created_by_spell_id, 9_001);
    assert_eq!(forced.specialization_id, 12);
    assert_eq!(forced.level, 43);
    assert_eq!(forced.react_state, ReactState::Passive);
    assert_eq!(forced.pet_type, PetType::Hunter);
    assert!(forced.was_renamed);

    let unforced = pet.fill_pet_info_like_cpp(43, &action_bar, None, true, 7, 8);
    assert_eq!(unforced.react_state, ReactState::Defensive);
    assert!(!unforced.was_renamed);
}

#[test]
fn pet_prepare_save_to_db_matches_cpp_initial_guards() {
    let pet = Pet::new(owner_guid(), PetType::Hunter);
    assert_eq!(
        pet.prepare_save_pet_to_db_like_cpp(PetSaveMode::AsCurrent as i16, 42, None, Some(0)),
        Err(PetSaveToDbSkipReason::ZeroEntry)
    );

    let mut uncontrolled = Pet::new(owner_guid(), PetType::Max);
    uncontrolled
        .creature_mut()
        .unit_mut()
        .world_mut()
        .object_mut()
        .set_entry(500);
    assert_eq!(
        uncontrolled.prepare_save_pet_to_db_like_cpp(
            PetSaveMode::AsCurrent as i16,
            42,
            None,
            Some(0)
        ),
        Err(PetSaveToDbSkipReason::NotControlled)
    );

    let mut non_player_owner = Pet::new(pet_guid(99), PetType::Hunter);
    non_player_owner
        .creature_mut()
        .unit_mut()
        .world_mut()
        .object_mut()
        .set_entry(500);
    assert_eq!(
        non_player_owner.prepare_save_pet_to_db_like_cpp(
            PetSaveMode::AsCurrent as i16,
            42,
            None,
            Some(0)
        ),
        Err(PetSaveToDbSkipReason::OwnerNotPlayer)
    );
}

#[test]
fn pet_prepare_save_to_db_remaps_current_and_preserves_cpp_insert_slot_quirk() {
    let mut pet = Pet::new(owner_guid(), PetType::Hunter);
    pet.creature_mut()
        .unit_mut()
        .world_mut()
        .object_mut()
        .set_entry(500);

    let current = pet
        .prepare_save_pet_to_db_like_cpp(PetSaveMode::AsCurrent as i16, 42, None, Some(2))
        .unwrap();

    assert_eq!(current.pet_number, 42);
    assert_eq!(current.effective_mode, PetSaveMode::active_slot(2));
    assert!(current.save_auras_before_cleanup);
    assert!(!current.remove_all_auras_before_spell_save);
    assert!(current.save_spells);
    assert!(current.save_spell_history);
    assert!(current.delete_existing_pet_row);
    assert!(current.fill_pet_info);
    assert!(current.insert_pet_row);
    assert_eq!(current.insert_slot, Some(PetSaveMode::active_slot(2)));
    assert!(!current.remove_all_auras_before_delete);
    assert_eq!(current.delete_from_db_pet_number, None);

    let stable_slot = pet
        .prepare_save_pet_to_db_like_cpp(PetSaveMode::stable_slot(3), 42, None, Some(1))
        .unwrap();
    assert_eq!(stable_slot.effective_mode, PetSaveMode::stable_slot(3));
    assert!(
        stable_slot.remove_all_auras_before_spell_save,
        "C++ removes all auras before saving spells for stable/not-in-slot saves"
    );
    assert_eq!(
        stable_slot.insert_slot,
        Some(PetSaveMode::active_slot(1)),
        "C++ CHAR_INS_PET stores current active slot, not the already-computed mode"
    );

    let without_current_slot = pet
        .prepare_save_pet_to_db_like_cpp(PetSaveMode::stable_slot(4), 42, None, None)
        .unwrap();
    assert_eq!(
        without_current_slot.insert_slot,
        Some(PetSaveMode::NotInSlot as i16)
    );
}

#[test]
fn pet_prepare_save_to_db_handles_temporary_unsummoned_current_like_cpp() {
    let mut hunter = Pet::new(owner_guid(), PetType::Hunter);
    hunter
        .creature_mut()
        .unit_mut()
        .world_mut()
        .object_mut()
        .set_entry(500);
    assert_eq!(
        hunter.prepare_save_pet_to_db_like_cpp(PetSaveMode::AsCurrent as i16, 42, Some(7), Some(0)),
        Err(PetSaveToDbSkipReason::TemporaryUnsummonedHunterCurrent)
    );

    let mut summon = Pet::new(owner_guid(), PetType::Summon);
    summon
        .creature_mut()
        .unit_mut()
        .world_mut()
        .object_mut()
        .set_entry(600);
    let plan = summon
        .prepare_save_pet_to_db_like_cpp(PetSaveMode::AsCurrent as i16, 42, Some(7), Some(0))
        .unwrap();
    assert_eq!(plan.effective_mode, PetSaveMode::NotInSlot as i16);
    assert!(plan.remove_all_auras_before_spell_save);
    assert!(plan.insert_pet_row);
    assert_eq!(plan.delete_from_db_pet_number, None);
}

#[test]
fn pet_prepare_save_to_db_delete_path_matches_cpp_delete_from_db_shape() {
    let mut pet = Pet::new(owner_guid(), PetType::Hunter);
    pet.creature_mut()
        .unit_mut()
        .world_mut()
        .object_mut()
        .set_entry(500);

    let plan = pet
        .prepare_save_pet_to_db_like_cpp(PetSaveMode::AsDeleted as i16, 42, None, Some(3))
        .unwrap();

    assert_eq!(plan.effective_mode, PetSaveMode::AsDeleted as i16);
    assert!(plan.save_auras_before_cleanup);
    assert!(plan.remove_all_auras_before_spell_save);
    assert!(plan.save_spells);
    assert!(plan.save_spell_history);
    assert!(!plan.delete_existing_pet_row);
    assert!(!plan.fill_pet_info);
    assert!(!plan.insert_pet_row);
    assert_eq!(plan.insert_slot, None);
    assert!(plan.remove_all_auras_before_delete);
    assert_eq!(plan.delete_from_db_pet_number, Some(42));
}

#[test]
fn pet_save_to_db_operations_preserve_cpp_transaction_order() {
    let mut pet = Pet::new(owner_guid(), PetType::Hunter);
    pet.creature_mut()
        .unit_mut()
        .world_mut()
        .object_mut()
        .set_entry(500);

    let current = pet
        .prepare_save_pet_to_db_like_cpp(PetSaveMode::AsCurrent as i16, 42, None, Some(2))
        .unwrap();
    assert_eq!(
        current.operations_like_cpp(),
        vec![
            PetSaveToDbOperationLikeCpp::BeginAuraSpellHistoryTransaction,
            PetSaveToDbOperationLikeCpp::SaveAuras,
            PetSaveToDbOperationLikeCpp::SaveSpells,
            PetSaveToDbOperationLikeCpp::SaveSpellHistory,
            PetSaveToDbOperationLikeCpp::CommitAuraSpellHistoryTransaction,
            PetSaveToDbOperationLikeCpp::BeginPetRowTransaction,
            PetSaveToDbOperationLikeCpp::DeleteCharacterPetById { pet_number: 42 },
            PetSaveToDbOperationLikeCpp::FillPetInfo,
            PetSaveToDbOperationLikeCpp::InsertPetRow {
                pet_number: 42,
                insert_slot: PetSaveMode::active_slot(2),
            },
            PetSaveToDbOperationLikeCpp::CommitPetRowTransaction,
        ]
    );

    let stable_slot = pet
        .prepare_save_pet_to_db_like_cpp(PetSaveMode::stable_slot(3), 42, None, Some(1))
        .unwrap();
    assert_eq!(
        stable_slot.operations_like_cpp(),
        vec![
            PetSaveToDbOperationLikeCpp::BeginAuraSpellHistoryTransaction,
            PetSaveToDbOperationLikeCpp::SaveAuras,
            PetSaveToDbOperationLikeCpp::RemoveAllAurasBeforeSpellSave,
            PetSaveToDbOperationLikeCpp::SaveSpells,
            PetSaveToDbOperationLikeCpp::SaveSpellHistory,
            PetSaveToDbOperationLikeCpp::CommitAuraSpellHistoryTransaction,
            PetSaveToDbOperationLikeCpp::BeginPetRowTransaction,
            PetSaveToDbOperationLikeCpp::DeleteCharacterPetById { pet_number: 42 },
            PetSaveToDbOperationLikeCpp::FillPetInfo,
            PetSaveToDbOperationLikeCpp::InsertPetRow {
                pet_number: 42,
                insert_slot: PetSaveMode::active_slot(1),
            },
            PetSaveToDbOperationLikeCpp::CommitPetRowTransaction,
        ]
    );

    let delete = pet
        .prepare_save_pet_to_db_like_cpp(PetSaveMode::AsDeleted as i16, 42, None, Some(1))
        .unwrap();
    assert_eq!(
        delete.operations_like_cpp(),
        vec![
            PetSaveToDbOperationLikeCpp::BeginAuraSpellHistoryTransaction,
            PetSaveToDbOperationLikeCpp::SaveAuras,
            PetSaveToDbOperationLikeCpp::RemoveAllAurasBeforeSpellSave,
            PetSaveToDbOperationLikeCpp::SaveSpells,
            PetSaveToDbOperationLikeCpp::SaveSpellHistory,
            PetSaveToDbOperationLikeCpp::CommitAuraSpellHistoryTransaction,
            PetSaveToDbOperationLikeCpp::RemoveAllAurasBeforeDelete,
            PetSaveToDbOperationLikeCpp::DeleteFromDb { pet_number: 42 },
        ]
    );
    assert_eq!(
        Pet::delete_from_db_plan_like_cpp(42),
        vec![
            PetDeleteFromDbOperationLikeCpp::BeginTransaction,
            PetDeleteFromDbOperationLikeCpp::DeleteCharacterPetById { pet_number: 42 },
            PetDeleteFromDbOperationLikeCpp::DeleteCharacterPetDeclinedName { pet_number: 42 },
            PetDeleteFromDbOperationLikeCpp::DeletePetAuraEffects { pet_number: 42 },
            PetDeleteFromDbOperationLikeCpp::DeletePetAuras { pet_number: 42 },
            PetDeleteFromDbOperationLikeCpp::DeletePetSpells { pet_number: 42 },
            PetDeleteFromDbOperationLikeCpp::DeletePetSpellCooldowns { pet_number: 42 },
            PetDeleteFromDbOperationLikeCpp::DeletePetSpellCharges { pet_number: 42 },
            PetDeleteFromDbOperationLikeCpp::CommitTransaction,
        ]
    );
}

#[test]
fn pet_save_spells_plan_matches_cpp_state_machine_and_family_skip() {
    let mut pet = Pet::new(owner_guid(), PetType::Hunter);
    assert!(pet.add_spell(
        100,
        ActiveState::Enabled,
        PetSpellState::New,
        PetSpellType::Normal
    ));
    assert!(pet.add_spell(
        200,
        ActiveState::Disabled,
        PetSpellState::Changed,
        PetSpellType::Normal
    ));
    assert!(pet.add_spell(
        300,
        ActiveState::Passive,
        PetSpellState::Unchanged,
        PetSpellType::Normal
    ));
    assert!(pet.add_spell(
        400,
        ActiveState::Enabled,
        PetSpellState::New,
        PetSpellType::Family
    ));
    assert!(pet.add_spell(
        500,
        ActiveState::Enabled,
        PetSpellState::New,
        PetSpellType::Normal
    ));
    assert!(pet.remove_spell(500));

    let operations = pet.save_spells_plan_like_cpp(42);

    assert_eq!(
        operations,
        vec![
            PetSpellSaveOperationLikeCpp::Insert {
                pet_number: 42,
                spell_id: 100,
                active: ActiveState::Enabled
            },
            PetSpellSaveOperationLikeCpp::DeleteBySpell {
                pet_number: 42,
                spell_id: 200
            },
            PetSpellSaveOperationLikeCpp::Insert {
                pet_number: 42,
                spell_id: 200,
                active: ActiveState::Disabled
            },
            PetSpellSaveOperationLikeCpp::DeleteBySpell {
                pet_number: 42,
                spell_id: 500
            },
        ],
        "C++ iterates the spell map in key order and appends delete/insert statements per state"
    );
    assert_eq!(
        pet.spells().get(&100).unwrap().state,
        PetSpellState::Unchanged
    );
    assert_eq!(
        pet.spells().get(&200).unwrap().state,
        PetSpellState::Unchanged
    );
    assert_eq!(
        pet.spells().get(&300).unwrap().state,
        PetSpellState::Unchanged
    );
    assert_eq!(
        pet.spells().get(&400).unwrap().state,
        PetSpellState::New,
        "C++ skips PETSPELL_FAMILY before handling state, so even NEW family passives stay dirty"
    );
    assert!(!pet.spells().contains_key(&500));
}

#[test]
fn pet_save_auras_plan_matches_cpp_delete_filter_and_insert_order() {
    let pet_guid = pet_guid(42);
    let other_caster = owner_guid();
    let saved_self_cast = PetAuraSaveRefLikeCpp {
        caster_guid: pet_guid,
        spell_id: 7_001,
        effect_mask: 0x3,
        recalculate_mask: 0x2,
        difficulty: 1,
        stack_count: 2,
        max_duration_ms: 30_000,
        duration_ms: 12_000,
        charges: 3,
        can_be_saved: true,
        is_pet_aura: false,
        effects: vec![
            PetAuraSaveEffectLikeCpp {
                effect_index: 0,
                amount: 11,
                base_amount: 10,
            },
            PetAuraSaveEffectLikeCpp {
                effect_index: 1,
                amount: 22,
                base_amount: 20,
            },
        ],
    };
    let saved_external_cast = PetAuraSaveRefLikeCpp {
        caster_guid: other_caster,
        spell_id: 7_002,
        effect_mask: 0x4,
        recalculate_mask: 0,
        difficulty: 0,
        stack_count: 1,
        max_duration_ms: -1,
        duration_ms: -1,
        charges: 0,
        can_be_saved: true,
        is_pet_aura: false,
        effects: vec![],
    };
    let not_saveable = PetAuraSaveRefLikeCpp {
        spell_id: 7_003,
        can_be_saved: false,
        ..saved_external_cast.clone()
    };
    let pet_aura = PetAuraSaveRefLikeCpp {
        spell_id: 7_004,
        can_be_saved: true,
        is_pet_aura: true,
        ..saved_external_cast.clone()
    };

    let operations = Pet::save_auras_plan_like_cpp(
        77,
        pet_guid,
        &[saved_self_cast, saved_external_cast, not_saveable, pet_aura],
    );

    assert_eq!(
        operations,
        vec![
            PetAuraSaveOperationLikeCpp::DeleteAuraEffects { pet_number: 77 },
            PetAuraSaveOperationLikeCpp::DeleteAuras { pet_number: 77 },
            PetAuraSaveOperationLikeCpp::InsertAura {
                pet_number: 77,
                caster_guid: ObjectGuid::EMPTY,
                spell_id: 7_001,
                effect_mask: 0x3,
                recalculate_mask: 0x2,
                difficulty: 1,
                stack_count: 2,
                max_duration_ms: 30_000,
                duration_ms: 12_000,
                charges: 3,
            },
            PetAuraSaveOperationLikeCpp::InsertAuraEffect {
                pet_number: 77,
                caster_guid: ObjectGuid::EMPTY,
                spell_id: 7_001,
                effect_mask: 0x3,
                effect_index: 0,
                amount: 11,
                base_amount: 10,
            },
            PetAuraSaveOperationLikeCpp::InsertAuraEffect {
                pet_number: 77,
                caster_guid: ObjectGuid::EMPTY,
                spell_id: 7_001,
                effect_mask: 0x3,
                effect_index: 1,
                amount: 22,
                base_amount: 20,
            },
            PetAuraSaveOperationLikeCpp::InsertAura {
                pet_number: 77,
                caster_guid: other_caster,
                spell_id: 7_002,
                effect_mask: 0x4,
                recalculate_mask: 0,
                difficulty: 0,
                stack_count: 1,
                max_duration_ms: -1,
                duration_ms: -1,
                charges: 0,
            },
        ],
        "C++ deletes existing pet aura rows first, clears self-caster GUIDs, then appends aura/effect inserts"
    );
}

#[test]
fn pet_spell_map_and_autospells_match_cpp_field_shape() {
    let mut pet = Pet::new(owner_guid(), PetType::Summon);

    assert!(pet.add_spell(
        123,
        ActiveState::Enabled,
        PetSpellState::New,
        PetSpellType::Normal
    ));
    assert!(pet.has_spell(123));
    assert_eq!(pet.get_pet_auto_spell_size(), 1);
    assert_eq!(pet.get_pet_auto_spell_on_pos(0), 123);

    assert!(pet.toggle_autocast(123, false));
    assert_eq!(pet.get_pet_auto_spell_size(), 0);
    assert_eq!(pet.get_pet_auto_spell_on_pos(1), 0);

    assert!(pet.remove_spell(123));
    assert!(!pet.has_spell(123));
}

#[test]
fn pet_toggle_autocast_like_cpp_requires_autocastable_spell_and_known_spell() {
    let mut pet = Pet::new(owner_guid(), PetType::Summon);
    assert!(pet.add_spell(
        123,
        ActiveState::Disabled,
        PetSpellState::Unchanged,
        PetSpellType::Normal
    ));

    let not_autocastable = pet.toggle_autocast_like_cpp(123, true, false);
    assert_eq!(
        not_autocastable,
        PetToggleAutocastOutcomeLikeCpp {
            spell_found: false,
            autocastable: false,
            autospell_added: false,
            autospell_removed: false,
            active_changed: false,
            marked_changed: false,
        }
    );
    assert!(pet.autospells().is_empty());
    assert_eq!(
        pet.spells().get(&123).unwrap().active,
        ActiveState::Disabled
    );

    let missing = pet.toggle_autocast_like_cpp(999, true, true);
    assert_eq!(
        missing,
        PetToggleAutocastOutcomeLikeCpp {
            spell_found: false,
            autocastable: true,
            autospell_added: false,
            autospell_removed: false,
            active_changed: false,
            marked_changed: false,
        }
    );
}

#[test]
fn pet_toggle_autocast_like_cpp_adds_removes_and_marks_persisted_changed() {
    let mut pet = Pet::new(owner_guid(), PetType::Summon);
    assert!(pet.add_spell(
        123,
        ActiveState::Disabled,
        PetSpellState::Unchanged,
        PetSpellType::Normal
    ));

    let enabled = pet.toggle_autocast_like_cpp(123, true, true);
    assert!(enabled.autospell_added);
    assert!(enabled.active_changed);
    assert!(enabled.marked_changed);
    assert_eq!(pet.autospells(), &[123]);
    assert_eq!(pet.spells().get(&123).unwrap().active, ActiveState::Enabled);
    assert_eq!(
        pet.spells().get(&123).unwrap().state,
        PetSpellState::Changed
    );

    let duplicate_enable = pet.toggle_autocast_like_cpp(123, true, true);
    assert_eq!(
        duplicate_enable,
        PetToggleAutocastOutcomeLikeCpp {
            spell_found: true,
            autocastable: true,
            autospell_added: false,
            autospell_removed: false,
            active_changed: false,
            marked_changed: false,
        },
        "C++ ToggleAutocast(true) is a no-op when the spell is already in m_autospells"
    );

    let disabled = pet.toggle_autocast_like_cpp(123, false, true);
    assert!(disabled.autospell_removed);
    assert!(disabled.active_changed);
    assert!(disabled.marked_changed);
    assert!(pet.autospells().is_empty());
    assert_eq!(
        pet.spells().get(&123).unwrap().active,
        ActiveState::Disabled
    );
    assert_eq!(
        pet.spells().get(&123).unwrap().state,
        PetSpellState::Changed
    );
}
