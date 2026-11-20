// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use super::*;
use wow_persistence::{PersistenceOutcomeLikeCpp, PlayerPetSaveTransactionLikeCpp};

const PET_NUMBER: u32 = 9_301;

fn install_quiescent_pet(session: &mut WorldSession, owner: ObjectGuid) -> ObjectGuid {
    let guid = ObjectGuid::create_world_object(
        wow_core::guid::HighGuid::Pet,
        0,
        1,
        571,
        0,
        1,
        i64::from(PET_NUMBER),
    );
    let mut pet = wow_entities::Pet::new(owner, wow_entities::PetType::Hunter);
    let unit = pet.creature_mut().unit_mut();
    unit.world_mut().object_mut().create(guid);
    unit.world_mut().object_mut().set_entry(123);
    unit.world_mut().set_map(571, 0).unwrap();
    unit.world_mut().object_mut().add_to_world();
    unit.world_mut().set_name("Misha");
    unit.set_max_health(91);
    unit.set_health(73);
    unit.subsystems_mut().control.init_charm_info().pet_number = PET_NUMBER;
    session
        .core
        .canonical_map_manager
        .as_ref()
        .unwrap()
        .lock()
        .unwrap()
        .find_map_mut(571, 0)
        .unwrap()
        .map_mut()
        .insert_map_object_record(wow_entities::MapObjectRecord::new_pet(pet).unwrap())
        .unwrap();
    session
        .core
        .with_owned_player_mut_like_cpp(|player| {
            player.set_pet_guid_like_cpp(Some(guid));
            player.unit_mut().set_minion_guid_like_cpp(Some(guid));
            let control = &mut player.unit_mut().subsystems_mut().control;
            control.controlled_guids.insert(guid);
            control.set_pet_guid(guid);
            let state = player.pet_lifecycle_state_mut_like_cpp();
            state.stable.current_pet_index = Some(0);
            state.stable.active_pets = vec![Some(wow_entities::PetStableInfo {
                pet_number: PET_NUMBER,
                creature_id: 123,
                health: 91,
                ..Default::default()
            })];
        })
        .unwrap();
    session
        .lifecycle
        .pet_load_insert_auras_for_pet_number_like_cpp(PET_NUMBER, vec![]);
    session
        .lifecycle
        .pet_load_insert_aura_effects_for_pet_number_like_cpp(PET_NUMBER, vec![]);
    session
        .lifecycle
        .pet_load_insert_spells_for_pet_number_like_cpp(PET_NUMBER, vec![]);
    session
        .lifecycle
        .pet_load_insert_spell_cooldowns_for_pet_number_like_cpp(PET_NUMBER, vec![]);
    session
        .lifecycle
        .pet_load_insert_spell_charges_for_pet_number_like_cpp(PET_NUMBER, vec![]);
    guid
}

fn install_loaded_pet_aura(session: &mut WorldSession, guid: ObjectGuid) {
    const SPELL_ID: u32 = 7_001;
    let aura_ref = wow_entities::AuraRef::new(SPELL_ID, guid);
    let applied = wow_entities::AppliedAuraRef::new(SPELL_ID, guid, 4, 1);
    let manager = session.core.canonical_map_manager.as_ref().unwrap();
    let mut manager = manager.lock().unwrap();
    let pet = manager
        .find_map_mut(571, 0)
        .unwrap()
        .map_mut()
        .get_typed_pet_mut(guid)
        .unwrap();
    let auras = &mut pet.creature_mut().unit_mut().subsystems_mut().auras;
    auras.add_owned(wow_entities::OwnedAuraRef::new(SPELL_ID, guid, None));
    auras.add_applied(applied);
    auras.set_loaded_aura_state_like_cpp(
        aura_ref,
        wow_entities::LoadedAuraStateLikeCpp::new(30_000, 12_000, 3, 2, 1)
            .with_pet_persistence_fields_like_cpp(5, vec![(0, 10)]),
    );
    auras.visible_auras.insert(4, aura_ref);
    auras.visible_aura_applications_like_cpp.insert(
        4,
        wow_entities::VisibleAuraApplicationLikeCpp::new(
            1,
            vec![wow_entities::VisibleAuraEffectAmountLikeCpp {
                effect_index: 0,
                amount: 11,
            }],
        ),
    );
    auras.applied_aura_amounts.insert(applied, 11);
    drop(manager);
    session
        .lifecycle
        .pet_load_insert_auras_for_pet_number_like_cpp(
            PET_NUMBER,
            vec![wow_world_lifecycle::CharacterPetAuraRowLikeCpp {
                caster_guid: ObjectGuid::EMPTY,
                spell_id: SPELL_ID,
                effect_mask: 1,
                recalculate_mask: 1,
                difficulty: 5,
                stack_count: 2,
                max_duration_ms: 30_000,
                remain_time_ms: 12_000,
                remain_charges: 3,
            }],
        );
    session
        .lifecycle
        .pet_load_insert_aura_effects_for_pet_number_like_cpp(
            PET_NUMBER,
            vec![wow_world_lifecycle::CharacterPetAuraEffectRowLikeCpp {
                caster_guid: ObjectGuid::EMPTY,
                spell_id: SPELL_ID,
                effect_mask: 1,
                effect_index: 0,
                amount: 11,
                base_amount: 10,
            }],
        );
}

fn assert_pet_retained(session: &WorldSession, guid: ObjectGuid) {
    assert_eq!(
        session.core.with_owned_player_like_cpp(|player| (
            player.gameplay_state().pet_guid,
            player
                .pet_lifecycle_state_like_cpp()
                .stable
                .current_pet_index,
        )),
        Some((Some(guid), Some(0)))
    );
    assert!(
        !session
            .core
            .canonical_map_manager
            .as_ref()
            .unwrap()
            .lock()
            .unwrap()
            .find_map(571, 0)
            .unwrap()
            .map()
            .get_typed_pet(guid)
            .unwrap()
            .is_removed()
    );
}

#[tokio::test]
async fn unloaded_move_pet_command_saves_and_detaches_current_pet_like_cpp() {
    let (mut session, send_rx, _realm_rx, owner, _corpses) =
        dead_player_session_with_graveyard_like_cpp(9_307);
    let guid = install_quiescent_pet(&mut session, owner);
    let port =
        crate::session::RecordingPlayerLifecyclePort::new(PersistenceOutcomeLikeCpp::Applied {
            rows: 1,
        });
    session.set_player_lifecycle_port_like_cpp(port.clone());
    let catalogs = session.session_handler_catalogs_for_test_like_cpp();

    session
        .apply_session_command_with_catalogs_like_cpp(
            &catalogs,
            crate::session::mailbox::SessionCommand::RemovePetAfterUnloadedMoveLikeCpp(
                crate::session::mailbox::RemovePetAfterUnloadedMoveLikeCppCommand {
                    pet_guid: guid,
                    owner_guid: owner,
                    map_id: 571,
                    instance_id: 0,
                },
            ),
        )
        .await;

    assert_eq!(port.pet_save_requests().len(), 2);
    assert_eq!(
        session.core.with_owned_player_like_cpp(|player| (
            player.gameplay_state().pet_guid,
            player
                .pet_lifecycle_state_like_cpp()
                .stable
                .current_pet_index,
            player.unit().minion_guid_like_cpp(),
        )),
        Some((None, None, None))
    );
    assert!(
        session
            .core
            .canonical_map_manager
            .as_ref()
            .unwrap()
            .lock()
            .unwrap()
            .find_map(571, 0)
            .unwrap()
            .map()
            .get_typed_pet(guid)
            .unwrap()
            .is_removed()
    );
    assert_eq!(sent_opcodes(&send_rx), vec![0x2c22]);
    assert_ne!(session.state(), crate::session::SessionState::Disconnecting);
}

#[tokio::test]
async fn unloaded_move_pet_command_rejects_stale_pet_identity_like_cpp() {
    let (mut session, _send_rx, _realm_rx, owner, _corpses) =
        dead_player_session_with_graveyard_like_cpp(9_308);
    let guid = install_quiescent_pet(&mut session, owner);
    let stale_guid = ObjectGuid::create_world_object(
        wow_core::guid::HighGuid::Pet,
        0,
        1,
        571,
        0,
        1,
        i64::from(PET_NUMBER + 1),
    );
    let port =
        crate::session::RecordingPlayerLifecyclePort::new(PersistenceOutcomeLikeCpp::Applied {
            rows: 1,
        });
    session.set_player_lifecycle_port_like_cpp(port.clone());
    let catalogs = session.session_handler_catalogs_for_test_like_cpp();

    session
        .apply_session_command_with_catalogs_like_cpp(
            &catalogs,
            crate::session::mailbox::SessionCommand::RemovePetAfterUnloadedMoveLikeCpp(
                crate::session::mailbox::RemovePetAfterUnloadedMoveLikeCppCommand {
                    pet_guid: stale_guid,
                    owner_guid: owner,
                    map_id: 571,
                    instance_id: 0,
                },
            ),
        )
        .await;

    assert!(port.pet_save_requests().is_empty());
    assert_pet_retained(&session, guid);
    assert_ne!(session.state(), crate::session::SessionState::Disconnecting);
}

#[tokio::test]
async fn unloaded_move_pet_command_disconnects_when_save_fails_like_cpp() {
    let (mut session, _send_rx, _realm_rx, owner, _corpses) =
        dead_player_session_with_graveyard_like_cpp(9_309);
    let guid = install_quiescent_pet(&mut session, owner);
    let port =
        crate::session::RecordingPlayerLifecyclePort::new(PersistenceOutcomeLikeCpp::Applied {
            rows: 1,
        });
    port.set_pet_save_outcomes([PersistenceOutcomeLikeCpp::Failed {
        reason: "pet save failed".into(),
    }]);
    session.set_player_lifecycle_port_like_cpp(port.clone());
    let catalogs = session.session_handler_catalogs_for_test_like_cpp();

    session
        .apply_session_command_with_catalogs_like_cpp(
            &catalogs,
            crate::session::mailbox::SessionCommand::RemovePetAfterUnloadedMoveLikeCpp(
                crate::session::mailbox::RemovePetAfterUnloadedMoveLikeCppCommand {
                    pet_guid: guid,
                    owner_guid: owner,
                    map_id: 571,
                    instance_id: 0,
                },
            ),
        )
        .await;

    assert_eq!(port.pet_save_requests().len(), 1);
    assert_pet_retained(&session, guid);
    assert_eq!(session.state(), crate::session::SessionState::Disconnecting);
}

#[tokio::test]
async fn live_pet_release_saves_both_transactions_before_detaching_and_creating_corpse() {
    let (mut session, send_rx, _realm_rx, owner, corpses) =
        dead_player_session_with_graveyard_like_cpp(9_302);
    let guid = install_quiescent_pet(&mut session, owner);
    let port =
        crate::session::RecordingPlayerLifecyclePort::new(PersistenceOutcomeLikeCpp::Applied {
            rows: 1,
        });
    session.set_player_lifecycle_port_like_cpp(port.clone());

    handle_repop_request(&mut session, repop_request_packet(true)).await;

    let saves = port.pet_save_requests();
    assert_eq!(saves.len(), 2);
    assert!(
        matches!(&saves[0], PlayerPetSaveTransactionLikeCpp::AurasSpellsAndHistory { pet_number, auras, aura_effects, .. }
        if *pet_number == PET_NUMBER && auras.is_empty() && aura_effects.is_empty())
    );
    let PlayerPetSaveTransactionLikeCpp::PetRow { owner_guid, row } = &saves[1] else {
        panic!("pet row must be second")
    };
    assert_eq!(*owner_guid, owner.counter() as u64);
    assert_eq!(
        (row.pet_number, row.slot, row.health, row.name.as_str()),
        (PET_NUMBER, 0, 73, "Misha")
    );
    assert_eq!(corpses.saves.lock().unwrap().len(), 1);
    assert_eq!(
        session.core.with_owned_player_like_cpp(|player| (
            player.gameplay_state().pet_guid,
            player
                .pet_lifecycle_state_like_cpp()
                .stable
                .current_pet_index,
            player.unit().minion_guid_like_cpp(),
        )),
        Some((None, None, None))
    );
    assert!(
        session
            .core
            .with_owned_player_like_cpp(|player| player
                .unit()
                .unit_data_changes_mask()
                .is_set(wow_entities::UNIT_DATA_SUMMON_BIT))
            .expect("canonical owner remains attached")
    );
    assert_eq!(
        session
            .core
            .with_owned_player_like_cpp(|player| player.unit().data().summon),
        Some(ObjectGuid::EMPTY)
    );
    assert!(
        session
            .core
            .canonical_map_manager
            .as_ref()
            .unwrap()
            .lock()
            .unwrap()
            .find_map(571, 0)
            .unwrap()
            .map()
            .get_typed_pet(guid)
            .unwrap()
            .is_removed()
    );
    assert_eq!(
        sent_opcodes(&send_rx)
            .iter()
            .filter(|opcode| **opcode == 0x2c22)
            .count(),
        1
    );
    assert_eq!(
        session
            .core
            .with_owned_player_like_cpp(|player| player.group_update_flag_like_cpp()),
        Some(0)
    );
    assert!(
        !session
            .lifecycle
            .durable_loot_money_persistence_tracker_like_cpp()
            .is_indeterminate_like_cpp()
    );
}

#[tokio::test]
async fn live_pet_release_marks_owner_group_update_and_preserves_other_flags() {
    let (mut session, send_rx, _realm_rx, owner, corpses) =
        dead_player_session_with_graveyard_like_cpp(9_312);
    let guid = install_quiescent_pet(&mut session, owner);
    session
        .core
        .with_owned_player_mut_like_cpp(|player| {
            player.set_group_like_cpp(Some(wow_entities::PlayerGroupState {
                group_guid: ObjectGuid::create_group(9_312),
                leader_guid: owner,
                role_mask: 0,
                subgroup: 0,
            }));
            player.set_group_update_flag_like_cpp(0x40);
        })
        .unwrap();
    let port =
        crate::session::RecordingPlayerLifecyclePort::new(PersistenceOutcomeLikeCpp::Applied {
            rows: 1,
        });
    session.set_player_lifecycle_port_like_cpp(port.clone());

    handle_repop_request(&mut session, repop_request_packet(true)).await;

    assert_eq!(port.pet_save_requests().len(), 2);
    assert_eq!(corpses.saves.lock().unwrap().len(), 1);
    assert_eq!(
        session.core.with_owned_player_like_cpp(|player| (
            player.gameplay_state().pet_guid,
            player.group_update_flag_like_cpp(),
        )),
        Some((None, 0x40 | wow_entities::GROUP_UPDATE_FLAG_PET_LIKE_CPP,))
    );
    assert_eq!(
        sent_opcodes(&send_rx)
            .iter()
            .filter(|opcode| **opcode == 0x2c22)
            .count(),
        1
    );
}

#[tokio::test]
async fn live_pet_release_saves_loaded_aura_then_cleans_it_before_detaching() {
    let (mut session, _send_rx, _realm_rx, owner, corpses) =
        dead_player_session_with_graveyard_like_cpp(9_310);
    let guid = install_quiescent_pet(&mut session, owner);
    install_loaded_pet_aura(&mut session, guid);
    let port =
        crate::session::RecordingPlayerLifecyclePort::new(PersistenceOutcomeLikeCpp::Applied {
            rows: 1,
        });
    session.set_player_lifecycle_port_like_cpp(port.clone());

    handle_repop_request(&mut session, repop_request_packet(true)).await;

    let saves = port.pet_save_requests();
    assert_eq!(saves.len(), 2);
    let PlayerPetSaveTransactionLikeCpp::AurasSpellsAndHistory {
        auras,
        aura_effects,
        ..
    } = &saves[0]
    else {
        panic!("pet aura/spell/history transaction must be first")
    };
    assert_eq!(
        auras,
        &vec![wow_persistence::PlayerPetAuraLoadRowLikeCpp {
            caster_guid_binary: ObjectGuid::EMPTY.to_raw_bytes().to_vec(),
            spell_id: 7_001,
            effect_mask: 1,
            recalculate_mask: 1,
            difficulty: 5,
            stack_count: 2,
            max_duration_ms: 30_000,
            remain_time_ms: 12_000,
            remain_charges: 3,
        }]
    );
    assert_eq!(
        aura_effects,
        &vec![wow_persistence::PlayerPetAuraEffectLoadRowLikeCpp {
            caster_guid_binary: ObjectGuid::EMPTY.to_raw_bytes().to_vec(),
            spell_id: 7_001,
            effect_mask: 1,
            effect_index: 0,
            amount: 11,
            base_amount: 10,
        }]
    );
    let manager = session.core.canonical_map_manager.as_ref().unwrap();
    let manager = manager.lock().unwrap();
    let pet = manager
        .find_map(571, 0)
        .unwrap()
        .map()
        .get_typed_pet(guid)
        .unwrap();
    assert!(pet.is_removed());
    assert!(
        pet.creature()
            .unit()
            .subsystems()
            .auras
            .has_no_local_aura_state_like_cpp()
    );
    assert!(
        pet.creature()
            .unit()
            .subsystems()
            .auras
            .runtime_applications_like_cpp()
            .is_empty()
    );
    drop(manager);
    assert_eq!(corpses.saves.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn live_pet_release_rejects_divergent_loaded_aura_before_mutation() {
    let (mut session, _send_rx, _realm_rx, owner, corpses) =
        dead_player_session_with_graveyard_like_cpp(9_311);
    let guid = install_quiescent_pet(&mut session, owner);
    install_loaded_pet_aura(&mut session, guid);
    let mut divergent = session
        .lifecycle
        .pet_load_auras_for_pet_number_like_cpp(PET_NUMBER)
        .cloned()
        .unwrap();
    divergent[0].remain_time_ms += 1;
    session
        .lifecycle
        .pet_load_remove_auras_for_pet_number_like_cpp(PET_NUMBER);
    session
        .lifecycle
        .pet_load_insert_auras_for_pet_number_like_cpp(PET_NUMBER, divergent);
    let port =
        crate::session::RecordingPlayerLifecyclePort::new(PersistenceOutcomeLikeCpp::Applied {
            rows: 1,
        });
    session.set_player_lifecycle_port_like_cpp(port.clone());

    handle_repop_request(&mut session, repop_request_packet(true)).await;

    assert!(port.pet_save_requests().is_empty());
    assert!(corpses.saves.lock().unwrap().is_empty());
    assert_pet_retained(&session, guid);
    let manager = session.core.canonical_map_manager.as_ref().unwrap();
    let manager = manager.lock().unwrap();
    assert_eq!(
        manager
            .find_map(571, 0)
            .unwrap()
            .map()
            .get_typed_pet(guid)
            .unwrap()
            .creature()
            .unit()
            .subsystems()
            .auras
            .owned_auras
            .len(),
        1
    );
}

#[tokio::test]
async fn live_pet_release_save_failure_or_unknown_result_retains_pet_and_blocks_corpse() {
    for (outcomes, expected_calls) in [
        (
            vec![PersistenceOutcomeLikeCpp::Failed {
                reason: "first transaction failed".into(),
            }],
            1,
        ),
        (
            vec![
                PersistenceOutcomeLikeCpp::Applied { rows: 1 },
                PersistenceOutcomeLikeCpp::Unknown {
                    reason: "second commit reply lost".into(),
                },
            ],
            2,
        ),
    ] {
        let (mut session, send_rx, _realm_rx, owner, corpses) =
            dead_player_session_with_graveyard_like_cpp(9_303);
        let guid = install_quiescent_pet(&mut session, owner);
        let port =
            crate::session::RecordingPlayerLifecyclePort::new(PersistenceOutcomeLikeCpp::Applied {
                rows: 1,
            });
        port.set_pet_save_outcomes(outcomes);
        session.set_player_lifecycle_port_like_cpp(port.clone());
        handle_repop_request(&mut session, repop_request_packet(true)).await;
        assert_eq!(port.pet_save_requests().len(), expected_calls);
        assert!(corpses.saves.lock().unwrap().is_empty());
        assert_pet_retained(&session, guid);
        assert!(!sent_opcodes(&send_rx).contains(&0x2c22));
        assert!(
            session
                .lifecycle
                .durable_loot_money_persistence_tracker_like_cpp()
                .is_indeterminate_like_cpp()
        );
        assert_eq!(session.state(), crate::session::SessionState::Disconnecting);
    }
}

#[tokio::test]
async fn live_pet_release_cancellation_during_save_retains_pet_and_fences_logout() {
    use std::future::Future;
    let (mut session, send_rx, _realm_rx, owner, corpses) =
        dead_player_session_with_graveyard_like_cpp(9_304);
    let guid = install_quiescent_pet(&mut session, owner);
    let port =
        crate::session::RecordingPlayerLifecyclePort::new(PersistenceOutcomeLikeCpp::Applied {
            rows: 1,
        });
    let (_release, gate) = flume::bounded(1);
    port.hold_next_pet_save(gate);
    session.set_player_lifecycle_port_like_cpp(port.clone());
    let mut operation = Box::pin(handle_repop_request(
        &mut session,
        repop_request_packet(true),
    ));
    std::future::poll_fn(|cx| {
        assert!(operation.as_mut().poll(cx).is_pending());
        std::task::Poll::Ready(())
    })
    .await;
    assert_eq!(port.pet_save_requests().len(), 1);
    drop(operation);
    assert_pet_retained(&session, guid);
    assert!(corpses.saves.lock().unwrap().is_empty());
    assert!(!sent_opcodes(&send_rx).contains(&0x2c22));
    assert!(
        session
            .lifecycle
            .durable_loot_money_persistence_tracker_like_cpp()
            .is_indeterminate_like_cpp()
    );
    assert!(session.player_save_snapshot_blocked_for_test(1));
}

#[tokio::test]
async fn live_pet_release_rejects_unknown_aura_load_before_writing() {
    let (mut session, _send_rx, _realm_rx, owner, corpses) =
        dead_player_session_with_graveyard_like_cpp(9_305);
    let guid = install_quiescent_pet(&mut session, owner);
    session
        .lifecycle
        .pet_load_remove_auras_for_pet_number_like_cpp(PET_NUMBER);
    let port =
        crate::session::RecordingPlayerLifecyclePort::new(PersistenceOutcomeLikeCpp::Applied {
            rows: 1,
        });
    session.set_player_lifecycle_port_like_cpp(port.clone());
    handle_repop_request(&mut session, repop_request_packet(true)).await;
    assert!(port.pet_save_requests().is_empty());
    assert!(corpses.saves.lock().unwrap().is_empty());
    assert_pet_retained(&session, guid);
    assert!(
        !session
            .lifecycle
            .durable_loot_money_persistence_tracker_like_cpp()
            .is_indeterminate_like_cpp()
    );
}

#[tokio::test]
async fn live_pet_release_stops_between_transactions_when_pet_changes_during_save() {
    use std::future::Future;
    let (mut session, send_rx, _realm_rx, owner, corpses) =
        dead_player_session_with_graveyard_like_cpp(9_306);
    let guid = install_quiescent_pet(&mut session, owner);
    let manager = session.core.canonical_map_manager.as_ref().unwrap().clone();
    let port =
        crate::session::RecordingPlayerLifecyclePort::new(PersistenceOutcomeLikeCpp::Applied {
            rows: 1,
        });
    let (release, gate) = flume::bounded(1);
    port.hold_next_pet_save(gate);
    session.set_player_lifecycle_port_like_cpp(port.clone());
    let mut operation = Box::pin(handle_repop_request(
        &mut session,
        repop_request_packet(true),
    ));
    std::future::poll_fn(|cx| {
        assert!(operation.as_mut().poll(cx).is_pending());
        std::task::Poll::Ready(())
    })
    .await;
    manager
        .lock()
        .unwrap()
        .find_map_mut(571, 0)
        .unwrap()
        .map_mut()
        .get_typed_pet_mut(guid)
        .unwrap()
        .creature_mut()
        .unit_mut()
        .set_health(10);
    release.send(()).unwrap();
    operation.await;
    assert_eq!(
        port.pet_save_requests().len(),
        1,
        "changed Pet must not reach the row transaction"
    );
    assert_pet_retained(&session, guid);
    assert!(corpses.saves.lock().unwrap().is_empty());
    assert!(!sent_opcodes(&send_rx).contains(&0x2c22));
    assert!(
        session
            .lifecycle
            .durable_loot_money_persistence_tracker_like_cpp()
            .is_indeterminate_like_cpp()
    );
}
