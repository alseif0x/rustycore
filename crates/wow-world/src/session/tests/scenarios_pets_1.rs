//! Session scenarios exercising the represented pets responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[test]
fn canonical_player_pet_lifecycle_follows_active_detached_and_stale_ownership_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 5_581);
    let owned = wow_entities::PlayerPetLifecycleStateLikeCpp {
        temporary_unsummoned_pet_number: 42,
        old_pet_spell: 1234,
        temporary_mount_react_state: Some(2),
        ..Default::default()
    };

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(canonical_player_transfer_test_map_store_like_cpp());
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "PetLifecycleOwner".to_string(),
        Position::new(3700.0, 1500.0, 120.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    session
        .ensure_canonical_world_map_for_current_player_like_cpp()
        .expect("initial world map");
    let old_handle = session.player_handle_like_cpp.expect("canonical handle");

    assert!(session.update_player_pet_lifecycle_state_like_cpp(|state| *state = owned.clone()));
    assert_eq!(
        session.player_pet_lifecycle_state_snapshot_like_cpp(),
        Some(owned.clone())
    );

    assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .player_residence_like_cpp(old_handle),
        Some(wow_map::PlayerResidenceLikeCpp::Detached)
    );
    assert_eq!(
        session.player_pet_lifecycle_state_snapshot_like_cpp(),
        Some(owned)
    );

    let replacement_state = wow_entities::PlayerPetLifecycleStateLikeCpp {
        temporary_unsummoned_pet_number: 99,
        old_pet_spell: 5678,
        temporary_mount_react_state: Some(1),
        ..Default::default()
    };
    let mut replacement = Box::new(Player::new(Some(2), false));
    replacement
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    *replacement.pet_lifecycle_state_mut_like_cpp() = replacement_state.clone();
    let replacement_handle = canonical
        .lock()
        .unwrap()
        .install_detached_player_like_cpp(replacement)
        .expect("replacement owner");

    assert_eq!(session.player_pet_lifecycle_state_snapshot_like_cpp(), None);
    assert!(
        !session.update_player_pet_lifecycle_state_like_cpp(|state| {
            state.temporary_unsummoned_pet_number = 7;
        })
    );
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .with_player_like_cpp(replacement_handle, |player| player
                .pet_lifecycle_state_like_cpp()
                .clone()),
        Some(replacement_state)
    );
}
#[test]
fn player_registry_publishes_party_member_pet_stats_like_cpp() {
    let (mut session, _, _) = make_session();
    let guid = ObjectGuid::create_player(1, 48);
    let pet_guid = ObjectGuid::create_world_object(HighGuid::Pet, 0, 1, 571, 0, 42_000, 100);
    let registry = Arc::new(PlayerRegistry::default());
    let canonical = shared_canonical_map_manager();
    assert!(registry.bind_canonical_map_manager(Arc::clone(&canonical)));
    let position = Position::new(1.0, 2.0, 3.0, 0.0);
    session.set_player_guid(Some(guid));
    session.set_player_map_position_like_cpp(571, position);
    session.player_name = Some("PetOwnerTester".to_string());
    session.set_player_registry(Arc::clone(&registry));
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_represented_pet_mode_state_like_cpp(Some(pet_guid), 1, 0);
    add_canonical_test_player_on_map(&canonical, guid, position, 571, 0);
    add_canonical_test_pet_with_visible_aura(
        &canonical,
        pet_guid,
        guid,
        42_000,
        position,
        0,
        Some((3, 12_345, guid, 0x05)),
        None,
    );

    session.register_in_player_registry();

    let info = registry.party_member(guid).expect("registered player");
    let pet_stats = info
        .party_member_pet_stats
        .as_ref()
        .expect("pet stats are published");
    assert_eq!(pet_stats.guid, pet_guid);
    assert_eq!(pet_stats.name, "Pet");
    assert_eq!(pet_stats.model_id, 1);
    assert_eq!(pet_stats.current_health, 100);
    assert_eq!(pet_stats.max_health, 100);
    assert_eq!(pet_stats.auras.len(), 1);
    assert_eq!(pet_stats.auras[0].spell_id, 12_345);
    assert_eq!(pet_stats.auras[0].active_flags, 0x05);
    assert_eq!(pet_stats.auras[0].flags, 0);
    assert!(pet_stats.auras[0].points.is_empty());
}
#[test]
fn load_represented_pet_stable_rows_partitions_slots_like_cpp() {
    let (mut session, _, _send_rx) = make_session();

    let loaded = session.load_represented_pet_stable_rows_like_cpp(
        0,
        [
            character_pet_stable_row_like_cpp(42, PetSaveMode::active_slot(2), 1),
            character_pet_stable_row_like_cpp(43, PetSaveMode::stable_slot(3), 1),
            character_pet_stable_row_like_cpp(44, PetSaveMode::NotInSlot as i16, 0),
            character_pet_stable_row_like_cpp(45, PetSaveMode::AsDeleted as i16, 1),
        ],
    );

    assert_eq!(loaded, 3);
    assert_eq!(session.represented_pet_stable_like_cpp.active_pets.len(), 3);
    let active = session.represented_pet_stable_like_cpp.active_pets[2]
        .as_ref()
        .expect("active pet slot");
    assert_eq!(active.pet_number, 42);
    assert_eq!(active.creature_id, 542);
    assert_eq!(active.display_id, 12_042);
    assert_eq!(active.level, 70);
    assert_eq!(active.experience, 123_456);
    assert_eq!(active.react_state, ReactState::Defensive);
    assert_eq!(active.pet_type, PetType::Hunter);
    assert_eq!(active.name, "Pet42");
    assert_eq!(active.action_bar, "1 2 3");
    assert!(active.was_renamed);
    assert_eq!(active.health, 345);
    assert_eq!(active.mana, 67);
    assert_eq!(active.last_save_time, 98_765);
    assert_eq!(active.created_by_spell_id, 9_001);
    assert_eq!(active.specialization_id, 2);

    assert_eq!(
        session.represented_pet_stable_like_cpp.stabled_pets.len(),
        4
    );
    assert_eq!(
        session.represented_pet_stable_like_cpp.stabled_pets[3]
            .as_ref()
            .map(|pet| pet.pet_number),
        Some(43)
    );
    assert_eq!(
        session.represented_pet_stable_like_cpp.unslotted_pets[0].pet_number,
        44
    );
    assert_eq!(
        session.represented_temporary_unsummoned_pet_number_like_cpp(),
        0
    );
}
#[test]
fn load_represented_pet_stable_rows_sets_temporary_unsummoned_when_summoned_exists_like_cpp() {
    let (mut session, _, _send_rx) = make_session();

    let loaded = session.load_represented_pet_stable_rows_like_cpp(
        42,
        [
            character_pet_stable_row_like_cpp(42, PetSaveMode::active_slot(0), 1),
            character_pet_stable_row_like_cpp(43, PetSaveMode::stable_slot(0), 1),
        ],
    );

    assert_eq!(loaded, 2);
    assert_eq!(
        session.represented_temporary_unsummoned_pet_number_like_cpp(),
        42
    );
    assert_eq!(
        session.represented_pet_stable_like_cpp.current_pet_index,
        Some(0)
    );
}
#[test]
fn load_represented_pet_stable_rows_leaves_temporary_unsummoned_zero_when_missing_like_cpp() {
    let (mut session, _, _send_rx) = make_session();

    session.load_represented_pet_stable_rows_like_cpp(
        99,
        [character_pet_stable_row_like_cpp(
            42,
            PetSaveMode::active_slot(0),
            1,
        )],
    );

    assert_eq!(
        session.represented_temporary_unsummoned_pet_number_like_cpp(),
        0
    );
    assert_eq!(
        session.represented_pet_stable_like_cpp.current_pet_index,
        None
    );
}
#[test]
fn load_represented_pet_declined_names_replaces_and_clears_row_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    let row = CharacterPetDeclinedNamesRowLikeCpp {
        names: ["Mishy", "Mishya", "Mishu", "Mishom", "Mishe"].map(str::to_string),
    };

    assert!(session.load_represented_pet_declined_names_like_cpp(42, Some(row.clone())));
    assert_eq!(
        session
            .pet_load_query_holder_rows_like_cpp
            .declined_names
            .get(&42),
        Some(&row)
    );
    assert!(session.load_represented_pet_declined_names_like_cpp(42, None));
    assert!(
        !session
            .pet_load_query_holder_rows_like_cpp
            .declined_names
            .contains_key(&42)
    );
}
#[test]
fn beginning_character_pet_load_replaces_pet_query_holder_rows_like_cpp() {
    let (mut session, _, _send_rx) = make_session();

    session.load_represented_pet_spell_rows_like_cpp(
        42,
        [CharacterPetSpellRowLikeCpp {
            spell_id: 123,
            active: ActiveState::Enabled as u8,
        }],
    );
    session.load_represented_pet_spell_cooldown_rows_like_cpp(
        42,
        [CharacterPetSpellCooldownRowLikeCpp {
            spell_id: 123,
            cooldown_end_unix_secs: 1,
            category_id: 7,
            category_end_unix_secs: 2,
        }],
    );
    session.load_represented_pet_spell_charge_rows_like_cpp(
        42,
        [CharacterPetSpellChargeRowLikeCpp {
            category_id: 7,
            recharge_start_unix_secs: 1,
            recharge_end_unix_secs: 2,
        }],
    );
    session.load_represented_pet_aura_rows_like_cpp(
        42,
        [CharacterPetAuraRowLikeCpp {
            caster_guid: ObjectGuid::EMPTY,
            spell_id: 123,
            effect_mask: 1,
            recalculate_mask: 0,
            difficulty: 0,
            stack_count: 1,
            max_duration_ms: 10,
            remain_time_ms: 10,
            remain_charges: 0,
        }],
    );
    session.load_represented_pet_aura_effect_rows_like_cpp(
        42,
        [CharacterPetAuraEffectRowLikeCpp {
            caster_guid: ObjectGuid::EMPTY,
            spell_id: 123,
            effect_mask: 1,
            effect_index: 0,
            amount: 3,
            base_amount: 3,
        }],
    );
    session.load_represented_pet_declined_names_like_cpp(
        42,
        Some(CharacterPetDeclinedNamesRowLikeCpp {
            names: ["a", "b", "c", "d", "e"].map(str::to_string),
        }),
    );
    assert!(
        !session
            .pet_load_query_holder_rows_like_cpp
            .spells
            .is_empty()
    );

    session.begin_represented_character_pet_authority_load_like_cpp();

    let holder = &session.pet_load_query_holder_rows_like_cpp;
    assert!(holder.spells.is_empty());
    assert!(holder.spell_cooldowns.is_empty());
    assert!(holder.spell_charges.is_empty());
    assert!(holder.auras.is_empty());
    assert!(holder.aura_effects.is_empty());
    assert!(holder.declined_names.is_empty());
}
#[test]
fn resummon_pet_temporary_unsummoned_loads_represented_stable_pet_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 8_420);
    let position = Position::new(17.0, 27.0, 37.0, 1.2);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "TemporaryPetResummon".to_string(),
        position,
        571,
        1,
        3,
        80,
        0,
    ));
    add_canonical_test_player_on_map(&canonical, player_guid, position, 571, 0);
    session.represented_temporary_unsummoned_pet_number_like_cpp = 42;
    let mut xp_table = vec![0; 81];
    xp_table[80] = 10_000;
    session.set_player_xp_table(Arc::new(xp_table));
    let mut stable = represented_hunter_pet_stable_like_cpp(42, 500);
    stable.active_pets[0].as_mut().unwrap().experience = 321;
    stable.active_pets[0].as_mut().unwrap().action_bar =
        "7 2 7 1 7 0 193 12345 129 23456 1 34567 193 45678 6 2 6 1 6 0".to_string();
    session.set_represented_pet_stable_like_cpp(stable);
    session.load_represented_pet_spell_rows_like_cpp(
        42,
        [
            CharacterPetSpellRowLikeCpp {
                spell_id: 1_234,
                active: ActiveState::Enabled as u8,
            },
            CharacterPetSpellRowLikeCpp {
                spell_id: 5_678,
                active: ActiveState::Disabled as u8,
            },
        ],
    );
    session.load_represented_pet_spell_cooldown_rows_like_cpp(
        42,
        [CharacterPetSpellCooldownRowLikeCpp {
            spell_id: 1_234,
            cooldown_end_unix_secs: 1_700_000_010,
            category_id: 33,
            category_end_unix_secs: 1_700_000_020,
        }],
    );
    session.load_represented_pet_spell_charge_rows_like_cpp(
        42,
        [
            CharacterPetSpellChargeRowLikeCpp {
                category_id: 44,
                recharge_start_unix_secs: 1_700_000_001,
                recharge_end_unix_secs: 1_700_000_011,
            },
            CharacterPetSpellChargeRowLikeCpp {
                category_id: 44,
                recharge_start_unix_secs: 1_700_000_011,
                recharge_end_unix_secs: 1_700_000_021,
            },
        ],
    );
    session.load_represented_pet_aura_rows_like_cpp(
        42,
        [CharacterPetAuraRowLikeCpp {
            caster_guid: ObjectGuid::EMPTY,
            spell_id: 7_777,
            effect_mask: 0x3,
            recalculate_mask: 0,
            difficulty: 0,
            stack_count: 2,
            max_duration_ms: 30_000,
            remain_time_ms: 20_000,
            remain_charges: 1,
        }],
    );
    session.load_represented_pet_aura_effect_rows_like_cpp(
        42,
        [
            CharacterPetAuraEffectRowLikeCpp {
                caster_guid: ObjectGuid::EMPTY,
                spell_id: 7_777,
                effect_mask: 0x3,
                effect_index: 0,
                amount: 51,
                base_amount: 41,
            },
            CharacterPetAuraEffectRowLikeCpp {
                caster_guid: ObjectGuid::EMPTY,
                spell_id: 7_777,
                effect_mask: 0x3,
                effect_index: 1,
                amount: 52,
                base_amount: 42,
            },
        ],
    );
    let declined_names = ["Mishy", "Mishya", "Mishu", "Mishom", "Mishe"].map(str::to_string);
    session.load_represented_pet_declined_names_like_cpp(
        42,
        Some(CharacterPetDeclinedNamesRowLikeCpp {
            names: declined_names.clone(),
        }),
    );

    session.resummon_pet_temporary_unsummoned_if_any_like_cpp();

    assert_eq!(session.temporary_pet_resummon_requests_like_cpp(), 1);
    assert_eq!(
        session.represented_temporary_unsummoned_pet_number_like_cpp(),
        0
    );
    let pet_guid = session
        .represented_pet_guid_like_cpp()
        .expect("represented active pet guid");
    assert_eq!(session.represented_pet_created_by_spell_like_cpp, 9_001);
    let manager = canonical.lock().unwrap();
    let pet = manager
        .find_map(571, 0)
        .unwrap()
        .map()
        .get_typed_pet(pet_guid)
        .expect("canonical represented pet");
    assert_eq!(pet.owner_guid(), player_guid);
    assert_eq!(pet.creature().entry(), 500);
    assert_eq!(pet.creature().unit().data().level, 80);
    assert_eq!(pet.pet_experience(), 321);
    assert_eq!(
        pet.pet_next_level_experience(),
        500,
        "C++ InitStatsForLevel sets hunter PetNextLevelExperience from XPForLevel(petlevel) * PET_XP_FACTOR"
    );
    assert_eq!(pet.creature().unit().data().health, 345);
    assert!(pet.has_spell(1_234));
    assert!(pet.has_spell(5_678));
    assert_eq!(pet.get_pet_auto_spell_on_pos(0), 1_234);
    assert_eq!(pet.get_pet_auto_spell_size(), 1);
    let pet_history = &pet.creature().unit().subsystems().spells.history;
    let cooldown = pet_history
        .cooldown(1_234)
        .expect("represented pet spell cooldown");
    assert_eq!(cooldown.item_id, 0);
    assert_eq!(cooldown.cooldown_end_ms, 1_700_000_010_000);
    assert_eq!(cooldown.category_id, 33);
    assert_eq!(cooldown.category_end_ms, 1_700_000_020_000);
    let charges = pet_history
        .charges(44)
        .expect("represented pet spell charges");
    assert_eq!(charges.len(), 2);
    assert_eq!(charges[0].recharge_start_ms, 1_700_000_001_000);
    assert_eq!(charges[0].recharge_end_ms, 1_700_000_011_000);
    assert_eq!(charges[1].recharge_start_ms, 1_700_000_011_000);
    assert_eq!(charges[1].recharge_end_ms, 1_700_000_021_000);
    let pet_auras = &pet.creature().unit().subsystems().auras;
    let loaded_aura = wow_entities::AppliedAuraRef::new(7_777, pet_guid, 0, 0x3);
    assert!(pet_auras.has_applied(loaded_aura));
    assert!(pet_auras.has_owned(wow_entities::OwnedAuraRef::new(7_777, pet_guid, None)));
    assert_eq!(
        pet_auras.visible_auras.get(&0),
        Some(&wow_entities::AuraRef::new(7_777, pet_guid))
    );
    assert_eq!(
        pet_auras
            .visible_aura_applications_like_cpp
            .get(&0)
            .map(|application| (
                application.flags,
                application
                    .effect_amounts
                    .iter()
                    .map(|amount| (amount.effect_index, amount.amount))
                    .collect::<Vec<_>>()
            )),
        Some((0x3, vec![(0, 51), (1, 52)]))
    );
    assert_eq!(
        pet_auras
            .loaded_aura_states_like_cpp
            .get(&loaded_aura.aura_ref()),
        Some(&wow_entities::LoadedAuraStateLikeCpp::new(
            30_000, 20_000, 1, 2, 0
        ))
    );
    assert_eq!(
        pet_auras
            .applied_aura_amounts
            .get(&wow_entities::AppliedAuraRef::new(7_777, pet_guid, 0, 0x1)),
        Some(&51)
    );
    assert_eq!(
        pet.declined_names().map(|names| &names.names),
        Some(&declined_names)
    );
    assert_eq!(
        pet.creature()
            .unit()
            .subsystems()
            .control
            .charm_info
            .as_ref()
            .map(|info| info.pet_number),
        Some(42)
    );
    let charm_info = pet
        .creature()
        .unit()
        .subsystems()
        .control
        .charm_info
        .as_ref()
        .expect("pet charm info");
    assert_eq!(
        charm_info.action_bar[3],
        wow_entities::make_unit_action_button_like_cpp(12_345, wow_entities::ACT_ENABLED_LIKE_CPP)
    );
    assert_eq!(
        charm_info.action_bar[4],
        wow_entities::make_unit_action_button_like_cpp(23_456, wow_entities::ACT_DISABLED_LIKE_CPP)
    );
}
#[test]
fn resummon_pet_zero_health_hunter_pet_loads_just_died_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 8_421);
    let position = Position::new(18.0, 28.0, 38.0, 1.3);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "DeadHunterPetResummon".to_string(),
        position,
        571,
        1,
        3,
        80,
        0,
    ));
    add_canonical_test_player_on_map(&canonical, player_guid, position, 571, 0);
    session.represented_temporary_unsummoned_pet_number_like_cpp = 42;
    let mut stable = represented_hunter_pet_stable_like_cpp(42, 500);
    stable.active_pets[0].as_mut().unwrap().health = 0;
    session.set_represented_pet_stable_like_cpp(stable);

    session.resummon_pet_temporary_unsummoned_if_any_like_cpp();

    let pet_guid = session
        .represented_pet_guid_like_cpp()
        .expect("represented active pet guid");
    let manager = canonical.lock().unwrap();
    let pet = manager
        .find_map(571, 0)
        .unwrap()
        .map()
        .get_typed_pet(pet_guid)
        .expect("canonical represented pet");
    assert_eq!(
        pet.creature().unit().death_state(),
        wow_constants::DeathState::JustDied,
        "C++ Pet::LoadPetFromDB calls setDeathState(JUST_DIED) for HUNTER_PET rows with zero saved health"
    );
    assert_eq!(pet.creature().unit().data().health, 0);
}
#[test]
fn resummon_pet_temporary_unsummoned_skips_declined_names_for_non_hunter_pet_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 8_421);
    let position = Position::new(17.0, 27.0, 37.0, 1.2);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "TemporarySummonPetNoDeclinedNames".to_string(),
        position,
        571,
        1,
        3,
        80,
        0,
    ));
    add_canonical_test_player_on_map(&canonical, player_guid, position, 571, 0);
    session.represented_temporary_unsummoned_pet_number_like_cpp = 42;
    let mut stable = represented_hunter_pet_stable_like_cpp(42, 500);
    stable.active_pets[0].as_mut().unwrap().pet_type = PetType::Summon;
    session.set_represented_pet_stable_like_cpp(stable);
    session.load_represented_pet_declined_names_like_cpp(
        42,
        Some(CharacterPetDeclinedNamesRowLikeCpp {
            names: ["Mishy", "Mishya", "Mishu", "Mishom", "Mishe"].map(str::to_string),
        }),
    );

    session.resummon_pet_temporary_unsummoned_if_any_like_cpp();

    let pet_guid = session
        .represented_pet_guid_like_cpp()
        .expect("represented active pet guid");
    let manager = canonical.lock().unwrap();
    let pet = manager
        .find_map(571, 0)
        .unwrap()
        .map()
        .get_typed_pet(pet_guid)
        .expect("canonical represented pet");
    assert_eq!(pet.pet_type(), PetType::Summon);
    assert!(pet.declined_names().is_none());
}
#[test]
fn resummon_pet_temporary_unsummoned_waits_while_player_dead_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    session.represented_temporary_unsummoned_pet_number_like_cpp = 42;
    session.set_represented_pet_stable_like_cpp(represented_hunter_pet_stable_like_cpp(42, 500));
    session.set_player_alive_like_cpp(false);

    session.resummon_pet_temporary_unsummoned_if_any_like_cpp();

    assert_eq!(session.temporary_pet_resummon_requests_like_cpp(), 1);
    assert_eq!(
        session.represented_temporary_unsummoned_pet_number_like_cpp(),
        42
    );
    assert_eq!(session.represented_pet_guid_like_cpp(), None);
}
#[test]
fn resummon_pet_temporary_unsummoned_waits_while_player_flying_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    session.represented_temporary_unsummoned_pet_number_like_cpp = 42;
    session.set_represented_pet_stable_like_cpp(represented_hunter_pet_stable_like_cpp(42, 500));
    session.set_player_movement_flags_like_cpp(MovementFlag::FLYING);

    session.resummon_pet_temporary_unsummoned_if_any_like_cpp();

    assert_eq!(session.temporary_pet_resummon_requests_like_cpp(), 1);
    assert_eq!(
        session.represented_temporary_unsummoned_pet_number_like_cpp(),
        42
    );
    assert_eq!(session.represented_pet_guid_like_cpp(), None);
}
#[test]
fn resummon_pet_temporary_unsummoned_failed_load_clears_number_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 8_422);
    let position = Position::new(17.0, 27.0, 37.0, 1.2);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "TemporaryPetMissingStable".to_string(),
        position,
        571,
        1,
        3,
        80,
        0,
    ));
    add_canonical_test_player_on_map(&canonical, player_guid, position, 571, 0);
    session.represented_temporary_unsummoned_pet_number_like_cpp = 42;
    session.set_represented_pet_stable_like_cpp(represented_hunter_pet_stable_like_cpp(99, 500));

    session.resummon_pet_temporary_unsummoned_if_any_like_cpp();

    assert_eq!(session.temporary_pet_resummon_requests_like_cpp(), 1);
    assert_eq!(
        session.represented_temporary_unsummoned_pet_number_like_cpp(),
        0,
        "C++ clears m_temporaryUnsummonedPetNumber after a failed LoadPetFromDB attempt"
    );
    assert_eq!(session.represented_pet_guid_like_cpp(), None);
}
#[test]
fn resummon_pet_temporary_unsummoned_skips_when_pet_already_active_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    let pet_guid = ObjectGuid::create_world_object(HighGuid::Pet, 0, 1, 571, 0, 500, 8421);
    session.represented_temporary_unsummoned_pet_number_like_cpp = 42;
    session.set_represented_pet_mode_state_like_cpp(
        Some(pet_guid),
        wow_packet::packets::pet::REACT_DEFENSIVE_LIKE_CPP,
        wow_packet::packets::pet::COMMAND_FOLLOW_LIKE_CPP,
    );

    session.resummon_pet_temporary_unsummoned_if_any_like_cpp();

    assert_eq!(session.temporary_pet_resummon_requests_like_cpp(), 1);
    assert_eq!(
        session.represented_temporary_unsummoned_pet_number_like_cpp(),
        42
    );
    assert_eq!(session.represented_pet_guid_like_cpp(), Some(pet_guid));
}
#[test]
fn battle_pet_clear_fanfare_clears_known_pet_only_like_cpp() {
    let (mut session, _, _) = make_session();
    let pet_guid = ObjectGuid::new(0, 0x123);
    let new_pet_guid = ObjectGuid::new(0, 0x124);
    let unknown_guid = ObjectGuid::new(0, 0x125);

    session.add_represented_battle_pet_like_cpp(
        pet_guid,
        BATTLE_PET_FLAG_FANFARE_NEEDED_LIKE_CPP | 0x10,
        RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
    );
    session.add_represented_battle_pet_like_cpp(
        new_pet_guid,
        BATTLE_PET_FLAG_FANFARE_NEEDED_LIKE_CPP,
        RepresentedBattlePetSaveInfoLikeCpp::New,
    );

    assert!(session.battle_pet_clear_fanfare_like_cpp(pet_guid));
    assert!(session.battle_pet_clear_fanfare_like_cpp(new_pet_guid));
    assert!(!session.battle_pet_clear_fanfare_like_cpp(unknown_guid));

    assert_eq!(
        session.represented_battle_pet_like_cpp(pet_guid),
        Some(RepresentedBattlePetDataLikeCpp::minimal_like_cpp(
            0x10,
            RepresentedBattlePetSaveInfoLikeCpp::Changed,
        ))
    );
    assert_eq!(
        session.represented_battle_pet_like_cpp(new_pet_guid),
        Some(RepresentedBattlePetDataLikeCpp::minimal_like_cpp(
            0,
            RepresentedBattlePetSaveInfoLikeCpp::New,
        ))
    );
}
#[tokio::test]
async fn battle_pet_cage_battle_pet_applies_cpp_gates_without_side_effects() {
    let (mut session, _, send_rx) = make_session();
    let pet_guid = ObjectGuid::new(0, 0x180);
    let slotted_guid = ObjectGuid::new(0, 0x181);
    let damaged_guid = ObjectGuid::new(0, 0x182);
    let unknown_guid = ObjectGuid::new(0, 0x183);

    session.add_represented_battle_pet_packet_info_like_cpp(
        pet_guid,
        RepresentedBattlePetDataLikeCpp {
            species: 11,
            creature_id: 22,
            display_id: 33,
            breed: 44,
            level: 17,
            exp: 0,
            flags: 0,
            power: 0,
            health: 100,
            max_health: 100,
            speed: 0,
            quality: 3,
            owner_info: None,
            name: String::new(),
            name_timestamp: 0,
            declined_names: None,
            save_info: RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
        },
    );
    session.add_represented_battle_pet_packet_info_like_cpp(
        slotted_guid,
        RepresentedBattlePetDataLikeCpp {
            species: 12,
            health: 100,
            max_health: 100,
            ..RepresentedBattlePetDataLikeCpp::minimal_like_cpp(
                0,
                RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
            )
        },
    );
    session.add_represented_battle_pet_packet_info_like_cpp(
        damaged_guid,
        RepresentedBattlePetDataLikeCpp {
            species: 13,
            health: 99,
            max_health: 100,
            ..RepresentedBattlePetDataLikeCpp::minimal_like_cpp(
                0,
                RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
            )
        },
    );
    assert!(session.battle_pet_set_battle_slot_like_cpp(slotted_guid, 1));

    assert_eq!(
        session.battle_pet_cage_battle_pet_represented_like_cpp(pet_guid, true, true),
        RepresentedBattlePetCageOutcomeLikeCpp::NoJournalLock
    );
    session.send_battle_pet_journal_lock_status_like_cpp().await;
    let _ = drain_server_packet_bytes(&send_rx);

    assert_eq!(
        session.battle_pet_cage_battle_pet_represented_like_cpp(unknown_guid, true, true),
        RepresentedBattlePetCageOutcomeLikeCpp::UnknownPet
    );
    install_represented_battle_pet_species_flags_like_cpp(
        &mut session,
        11,
        wow_data::BATTLE_PET_SPECIES_FLAG_NOT_TRADABLE_LIKE_CPP,
    );
    assert_eq!(
        session.battle_pet_cage_battle_pet_represented_like_cpp(pet_guid, true, true),
        RepresentedBattlePetCageOutcomeLikeCpp::NotTradable
    );
    install_represented_battle_pet_species_flags_like_cpp(&mut session, 11, 0);
    assert_eq!(
        session.battle_pet_cage_battle_pet_represented_like_cpp(slotted_guid, true, true),
        RepresentedBattlePetCageOutcomeLikeCpp::InBattleSlot
    );
    assert_eq!(
        session.battle_pet_cage_battle_pet_represented_like_cpp(damaged_guid, true, true),
        RepresentedBattlePetCageOutcomeLikeCpp::Damaged
    );
    assert_eq!(
        session.battle_pet_cage_battle_pet_represented_like_cpp(pet_guid, false, true),
        RepresentedBattlePetCageOutcomeLikeCpp::InventoryUnavailable
    );
    assert_eq!(
        session.battle_pet_cage_battle_pet_represented_like_cpp(pet_guid, true, false),
        RepresentedBattlePetCageOutcomeLikeCpp::StoreFailed
    );

    assert!(
        session
            .represented_battle_pet_cage_items_like_cpp()
            .is_empty()
    );
    assert_eq!(
        session
            .represented_battle_pet_like_cpp(pet_guid)
            .expect("pet still present")
            .save_info,
        RepresentedBattlePetSaveInfoLikeCpp::Unchanged
    );
    assert!(drain_server_packet_bytes(&send_rx).is_empty());
}
