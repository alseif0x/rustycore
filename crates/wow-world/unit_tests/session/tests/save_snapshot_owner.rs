//! Save projection reads one generation-checked Player without publishing state.

use super::*;

#[path = "save_snapshot_owner/sickness_support.rs"]
mod sickness_support;
use sickness_support::*;

#[test]
fn save_request_consumes_frozen_header_without_replaying_or_rereading_runtime() {
    let (mut session, _, _) = make_session();
    install_canonical_player_owner_for_test(&mut session, 571, 0);
    session.core.current_map_id = 571;
    session.fixtures.identity.player_level = 17;
    session
        .with_owned_player_mut_like_cpp(|player| {
            player.unit_mut().set_level(17);
            player.set_xp(10);
            player.set_money(20);
        })
        .unwrap();
    let snapshot = session
        .current_player_save_to_db_snapshot_like_cpp()
        .unwrap();
    session.fixtures.identity.player_level = 70;
    session
        .with_owned_player_mut_like_cpp(|player| {
            player.set_xp(30);
            player.set_money(40);
            player.unit_mut().set_level(60);
        })
        .unwrap();
    let request = session
        .current_player_character_save_request_like_cpp(&snapshot, 123)
        .unwrap();
    assert_eq!(
        (
            request.character.level,
            request.character.xp,
            request.character.money
        ),
        (17, 10, 20)
    );
    session
        .with_owned_player_like_cpp(|player| {
            assert_eq!(player.active_data().xp, 30);
            assert_eq!(player.money(), 40);
            assert_eq!(player.unit().data().level, 60);
        })
        .unwrap();
}

#[test]
fn save_snapshot_reads_active_and_detached_owner_without_changing_state() {
    let (mut session, _, send_rx) = make_session();
    let guid = install_canonical_player_owner_for_test(&mut session, 571, 7);
    session.core.current_map_id = 571;
    session.fixtures.identity.player_level = 17;
    session.set_loaded_player_powers_like_cpp([111, 222, 0, 0, 0, 0, 0, 0, 0, 0]);
    let position = Position::new(1.0, 2.0, 3.0, 0.5);
    session
        .with_owned_player_mut_like_cpp(|player| {
            player.unit_mut().world_mut().relocate(position);
            player.unit_mut().set_level(60);
            player.unit_mut().set_max_health(900);
            player.unit_mut().set_health(456);
            player.set_xp(1234);
            player.set_money(5678);
            player.clear_data_changes();
        })
        .unwrap();
    let manager = Arc::clone(session.core.canonical_map_manager.as_ref().unwrap());
    for detached in [false, true] {
        if detached {
            assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
        }
        let snapshot = session
            .current_player_save_to_db_snapshot_like_cpp()
            .unwrap();
        assert_eq!(
            snapshot,
            PlayerSaveToDbSnapshotLikeCpp {
                guid,
                map_id: 571,
                instance_id: 7, // C++ ResetMap retains the last instance.
                position,
                level: 60, // C++ SaveToDB reads Unit::GetLevel, not Session staging.
                xp: 1234,
                money: 5678,
                health: 456,
                max_health: 900,
                powers: loaded_character_power_snapshot_like_cpp([
                    111, 222, 0, 0, 0, 0, 0, 0, 0, 0
                ]),
            }
        );
        // The old fixture header uses Session level and clears detached instance;
        // it is deliberately not an oracle for these corrected owner reads.
        assert!(manager.try_lock().is_ok());
        session
            .with_owned_player_like_cpp(|player| {
                assert_eq!(player.unit().world().position(), position);
                assert_eq!(player.unit().data().level, 60);
                assert!(!player.active_player_data_changes_mask().is_any_set());
            })
            .unwrap();
        assert!(send_rx.is_empty());
    }
}

#[test]
fn save_snapshot_keeps_pending_destination_precedence_without_relocating_player() {
    let (mut session, _, _) = make_session();
    install_canonical_player_owner_for_test(&mut session, 571, 7);
    session.core.current_map_id = 571;
    let original = session.player_position_like_cpp().unwrap();
    let near = Position::new(11.0, 22.0, 33.0, 0.5);
    let far = Position::new(44.0, 55.0, 66.0, 1.5);
    session
        .with_owned_player_mut_like_cpp(|player| {
            let teleport = &mut player.gameplay_state_mut().teleport;
            teleport.near_pending = true;
            teleport.near_destination = Some((1, near));
        })
        .unwrap();
    for detached in [false, true] {
        if detached {
            assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
        }
        for pending in [None, Some((u32::MAX, far))] {
            assert!(session.set_pending_teleport_like_cpp(pending));
            let snapshot = session
                .current_player_save_to_db_snapshot_like_cpp()
                .unwrap();
            assert_eq!(
                (snapshot.map_id, snapshot.instance_id, snapshot.position),
                if pending.is_some() {
                    (u16::MAX, 0, far)
                } else {
                    (1, 0, near)
                }
            );
            let legacy = session
                .fixture_player_save_to_db_snapshot_like_cpp()
                .unwrap();
            assert_eq!(
                (snapshot.map_id, snapshot.instance_id, snapshot.position),
                (legacy.map_id, legacy.instance_id, legacy.position)
            );
            assert_eq!(session.player_position_like_cpp(), Some(original));
        }
    }
}

#[test]
fn save_snapshot_preserves_canonical_dead_health_for_active_and_detached_players() {
    let (mut session, _, _) = make_session();
    install_canonical_player_owner_for_test(&mut session, 571, 0);
    session
        .with_owned_player_mut_like_cpp(|player| {
            player.unit_mut().set_max_health(900);
            player.unit_mut().set_health(456);
            player
                .unit_mut()
                .set_death_state(wow_constants::DeathState::Corpse);
        })
        .unwrap();
    for (detached, expected) in [(false, 456), (true, 456)] {
        if detached {
            assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
        }
        let snapshot = session
            .current_player_save_to_db_snapshot_like_cpp()
            .unwrap();
        assert_eq!(snapshot.health, expected);
        assert_eq!(
            snapshot.health,
            session
                .fixture_player_save_to_db_snapshot_like_cpp()
                .unwrap()
                .health
        );
    }
}

#[test]
fn save_snapshot_keeps_ghost_health_and_captures_player_death_metadata() {
    let (mut session, _, _) = make_session();
    install_canonical_player_owner_for_test(&mut session, 571, 0);
    session
        .with_owned_player_mut_like_cpp(|player| {
            player.unit_mut().set_max_health(100);
            player.unit_mut().set_health(1);
            player
                .unit_mut()
                .set_death_state(wow_constants::DeathState::Dead);
            player.set_at_login_flags_like_cpp(0x104);
            player.resurrection_state_mut_like_cpp().death_expire_time = 1_700_000_123;
        })
        .unwrap();

    let snapshot = session
        .current_player_save_to_db_snapshot_like_cpp()
        .unwrap();
    assert_eq!(snapshot.health, 1, "a C++ ghost saves GetHealth() == 1");
    let request = session.prepare_player_save_like_cpp(123).unwrap().request;
    assert_eq!(request.character.at_login_flags, 0x104);
    assert_eq!(request.character.death_expire_time, 1_700_000_123);

    session
        .with_owned_player_mut_like_cpp(|player| {
            player
                .unit_mut()
                .set_death_state(wow_constants::DeathState::Corpse);
            player.unit_mut().set_health(0);
        })
        .unwrap();
    assert_eq!(
        session
            .current_player_save_to_db_snapshot_like_cpp()
            .unwrap()
            .health,
        0,
        "a corpse whose canonical health is zero remains zero"
    );
}

#[test]
fn canonical_full_save_capture_waits_for_resurrection_ready_phase() {
    let (mut session, _, _) = make_session();
    install_canonical_player_owner_for_test(&mut session, 571, 0);
    let request = wow_entities::PlayerResurrectionRequestLikeCpp {
        resurrecter: wow_core::ObjectGuid::create_player(1, 77),
        map_id: 571,
        position: Position::default(),
        health: 450,
        mana: 120,
        aura: 1234,
    };
    session
        .with_owned_player_mut_like_cpp(|player| {
            player
                .resurrection_state_mut_like_cpp()
                .delayed_after_teleport = Some(wow_entities::PlayerResurrectionContinuation {
                operation: wow_entities::PlayerResurrectionOperation::RequestData(request),
                phase: wow_entities::PlayerResurrectionPhase::BonesInFlight,
            });
        })
        .unwrap();

    assert!(
        session.prepare_player_save_like_cpp(123).is_none(),
        "the canonical owner cannot capture while corpse conversion is unresolved"
    );

    session
        .with_owned_player_mut_like_cpp(|player| {
            player
                .resurrection_state_mut_like_cpp()
                .delayed_after_teleport
                .as_mut()
                .unwrap()
                .phase = wow_entities::PlayerResurrectionPhase::ReadyToSavePlayer;
            assert_eq!(
                player.defer_save_if_transfer_pending_like_cpp(),
                Some(false),
                "canonical Player admission claims SavingPlayer after the bones step"
            );
        })
        .unwrap();

    assert!(
        session.prepare_player_save_like_cpp(124).is_some(),
        "the existing canonical capture owner permits only the claimed SavingPlayer phase"
    );
}

fn dirty_item(
    item_id: i64,
    owner: wow_core::ObjectGuid,
    max_durability: u32,
    durability: u32,
    state: wow_constants::ItemUpdateState,
) -> wow_entities::Item {
    let mut item = wow_entities::Item::new(0);
    item.object_mut()
        .create(wow_core::ObjectGuid::create_item(1, item_id));
    item.set_owner_guid(owner);
    item.set_max_durability(max_durability);
    item.set_durability(durability);
    item.force_state(state);
    item
}

fn inventory_item(guid: wow_core::ObjectGuid, db_guid: u64) -> wow_entities::PlayerInventoryItem {
    wow_entities::PlayerInventoryItem {
        guid,
        entry_id: 1,
        db_guid,
        inventory_type: None,
    }
}

#[test]
fn save_snapshot_captures_only_dirty_owned_non_buyback_durability_in_guid_order() {
    let (mut session, _, _) = make_session();
    let player_guid = install_canonical_player_owner_for_test(&mut session, 571, 0);
    let foreign_owner = wow_core::ObjectGuid::create_player(1, 999);
    session
        .with_owned_player_mut_like_cpp(|player| {
            let runtime = player.inventory_runtime_mut_like_cpp();
            let carried = dirty_item(
                30,
                player_guid,
                100,
                70,
                wow_constants::ItemUpdateState::Changed,
            );
            let carried_guid = carried.object().guid();
            runtime.store_item_object_like_cpp(carried);
            runtime.store_item_in_slot_like_cpp(0, inventory_item(carried_guid, 30));

            let bag = dirty_item(
                40,
                player_guid,
                0,
                0,
                wow_constants::ItemUpdateState::Unchanged,
            );
            let bag_guid = bag.object().guid();
            runtime.store_item_object_like_cpp(bag);
            runtime.store_item_in_slot_like_cpp(1, inventory_item(bag_guid, 40));
            let mut bag_child = dirty_item(
                50,
                player_guid,
                80,
                45,
                wow_constants::ItemUpdateState::Changed,
            );
            bag_child.set_container_guid(bag_guid);
            runtime.store_item_object_like_cpp(bag_child);

            let new_item = dirty_item(20, player_guid, 50, 37, wow_constants::ItemUpdateState::New);
            let new_item_guid = new_item.object().guid();
            runtime.store_item_object_like_cpp(new_item);
            runtime.store_item_in_slot_like_cpp(2, inventory_item(new_item_guid, 20));

            let bank = dirty_item(
                60,
                player_guid,
                70,
                31,
                wow_constants::ItemUpdateState::Changed,
            );
            let bank_guid = bank.object().guid();
            runtime.store_item_object_like_cpp(bank);
            runtime.store_item_in_slot_like_cpp(100, inventory_item(bank_guid, 60));

            let buyback = dirty_item(
                10,
                player_guid,
                100,
                8,
                wow_constants::ItemUpdateState::Changed,
            );
            let buyback_guid = buyback.object().guid();
            runtime.store_item_object_like_cpp(buyback);
            runtime.store_buyback_item_in_slot_like_cpp(94, inventory_item(buyback_guid, 10));

            runtime.store_item_object_like_cpp(dirty_item(
                70,
                foreign_owner,
                100,
                7,
                wow_constants::ItemUpdateState::Changed,
            ));
            runtime.store_item_object_like_cpp(dirty_item(
                80,
                player_guid,
                100,
                6,
                wow_constants::ItemUpdateState::Removed,
            ));
            runtime.store_item_object_like_cpp(dirty_item(
                90,
                player_guid,
                0,
                0,
                wow_constants::ItemUpdateState::Changed,
            ));
        })
        .unwrap();

    let incomplete = session.prepare_player_save_like_cpp(123).unwrap().request;
    assert_eq!(incomplete.item_durability, None);

    session
        .with_owned_player_mut_like_cpp(|player| {
            player
                .inventory_runtime_mut_like_cpp()
                .set_equipment_inventory_authority_complete_like_cpp(true);
        })
        .unwrap();
    let request = session.prepare_player_save_like_cpp(123).unwrap().request;
    assert_eq!(
        request.item_durability,
        Some(vec![
            wow_persistence::PlayerItemDurabilitySave {
                item_guid: 20,
                durability: 37,
            },
            wow_persistence::PlayerItemDurabilitySave {
                item_guid: 30,
                durability: 70,
            },
            wow_persistence::PlayerItemDurabilitySave {
                item_guid: 50,
                durability: 45,
            },
            wow_persistence::PlayerItemDurabilitySave {
                item_guid: 60,
                durability: 31,
            },
        ])
    );
    session
        .with_owned_player_like_cpp(|player| {
            assert_eq!(
                player.inventory_runtime_like_cpp().item_objects()
                    [&wow_core::ObjectGuid::create_item(1, 20)]
                    .update_state(),
                wow_constants::ItemUpdateState::New,
                "full-save capture must not acknowledge item update state"
            );
        })
        .unwrap();
}

#[test]
fn save_snapshot_cannot_read_replacement_or_fallback_after_owner_loss() {
    let (mut session, _, _) = make_session();
    let guid = install_canonical_player_owner_for_test(&mut session, 571, 0);
    let manager = Arc::clone(session.core.canonical_map_manager.as_ref().unwrap());
    assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
    let mut replacement = Box::new(Player::new(Some(1), false));
    replacement.unit_mut().world_mut().object_mut().create(guid);
    replacement.set_money(999);
    let handle = manager
        .lock()
        .unwrap()
        .install_detached_player_like_cpp(replacement)
        .unwrap();
    assert_eq!(session.current_player_save_to_db_snapshot_like_cpp(), None);
    session.core.canonical_map_manager = None;
    assert_eq!(session.current_player_save_to_db_snapshot_like_cpp(), None);
    assert_eq!(
        manager
            .lock()
            .unwrap()
            .with_player_like_cpp(handle, Player::money),
        Some(999)
    );
}

#[test]
fn sickness_capture_uses_race_spell_all_keys_effect_amounts_and_exact_durations() {
    let (mut session, _, _) = make_session();
    let player_guid = install_canonical_player_owner_for_test(&mut session, 571, 0);
    let spell_id = 54_321;
    install_sickness_catalogs(&mut session, spell_id, &[(0, 80, 50), (2, 159, -60)]);
    session
        .with_owned_player_mut_like_cpp(|player| {
            player.unit_mut().set_race(2);
        })
        .unwrap();

    insert_sickness_application(
        &mut session,
        sickness_application(
            player_guid,
            spell_id,
            7,
            0b101,
            60_000,
            45_000,
            std::time::Instant::now() - std::time::Duration::from_secs(5),
        ),
        Some(vec![(1, 80, 777, 0), (4, 159, -123, 0)]),
    );
    insert_sickness_application(
        &mut session,
        sickness_application(
            wow_core::ObjectGuid::create_player(1, 9_999),
            spell_id,
            8,
            1,
            60_000,
            45_000,
            std::time::Instant::now(),
        ),
        Some(vec![(1, 80, 9, 0)]),
    );
    insert_sickness_application(
        &mut session,
        sickness_application(
            player_guid,
            spell_id,
            9,
            1,
            60_000,
            45_000,
            std::time::Instant::now(),
        ),
        Some(vec![(1, 80, 11, 0)]),
    );
    insert_sickness_application(
        &mut session,
        sickness_application(
            player_guid,
            15_007,
            10,
            1,
            60_000,
            45_000,
            std::time::Instant::now(),
        ),
        Some(vec![(1, 80, 11, 0)]),
    );
    session
        .with_owned_player_mut_like_cpp(|player| {
            let auras = &mut player.unit_mut().subsystems_mut().auras;
            assert!(auras.set_aura_persistence_metadata(
                9,
                wow_entities::AuraPersistenceMetadata {
                    item_guid: wow_core::ObjectGuid::create_item(1, 88),
                    cast_item_id: 123,
                    cast_item_level: 60,
                    recalculate_mask: 1,
                    charges: 2,
                    base_amounts: vec![(0, 999)],
                },
            ));
            player.set_player_aura_authority_complete_like_cpp(true);
        })
        .unwrap();

    let saved = capture_sickness(&mut session).expect("complete aura authority saves the spell");
    assert_eq!(saved.spell_id, spell_id as u32);
    assert_eq!(saved.families.len(), 3);
    let aura = &saved
        .families
        .iter()
        .find(|family| {
            family.caster_guid_binary == player_guid.to_raw_bytes()
                && family.item_guid_binary == ObjectGuid::EMPTY.to_raw_bytes()
        })
        .expect("self-cast empty-item family")
        .auras[0];
    assert_eq!(aura.difficulty, 0);
    assert_eq!(aura.stack_count, 1);
    assert_eq!(aura.max_duration, 60_000);
    assert_eq!(aura.remaining_duration, 45_000);
    assert_eq!(aura.charges, 0);
    assert_eq!(aura.recalculate_mask, 0b101);
    assert_eq!((aura.cast_item_id, aura.cast_item_level), (0, -1));
    assert_eq!(
        aura.effects,
        vec![
            wow_persistence::PlayerLifecycleAuraEffectSave {
                index: 0,
                amount: 777,
                base_amount: 50,
            },
            wow_persistence::PlayerLifecycleAuraEffectSave {
                index: 2,
                amount: -123,
                base_amount: -60,
            },
        ]
    );
    let foreign = saved
        .families
        .iter()
        .find(|family| {
            family.caster_guid_binary
                == wow_core::ObjectGuid::create_player(1, 9_999).to_raw_bytes()
                && family.item_guid_binary == ObjectGuid::EMPTY.to_raw_bytes()
        })
        .expect("foreign caster remains part of the selected spell keyset");
    assert_eq!(foreign.auras[0].effects[0].amount, 9);
    let item_key = ObjectGuid::create_item(1, 88).to_raw_bytes();
    let item_family = saved
        .families
        .iter()
        .find(|family| {
            family.caster_guid_binary == player_guid.to_raw_bytes()
                && family.item_guid_binary == item_key
        })
        .expect("finite item-triggered aura keeps its full database key");
    assert_eq!(
        (
            item_family.auras[0].cast_item_id,
            item_family.auras[0].cast_item_level,
            item_family.auras[0].charges,
            item_family.auras[0].recalculate_mask,
            item_family.auras[0].effects[0].base_amount,
        ),
        (123, 60, 2, 1, 999)
    );
}

#[test]
fn loaded_sickness_metadata_keeps_database_base_amounts_and_cast_fields() {
    let (mut session, _, _) = make_session();
    let player_guid = install_canonical_player_owner_for_test(&mut session, 571, 0);
    let spell_id = 54_321;
    install_sickness_catalogs(&mut session, spell_id, &[(0, 80, 10)]);
    session
        .with_owned_player_mut_like_cpp(|player| player.unit_mut().set_race(2))
        .unwrap();
    insert_sickness_application(
        &mut session,
        sickness_application(
            player_guid,
            spell_id,
            3,
            1,
            30_000,
            20_000,
            std::time::Instant::now(),
        ),
        Some(vec![(1, 80, 400, 0)]),
    );
    session
        .with_owned_player_mut_like_cpp(|player| {
            let auras = &mut player.unit_mut().subsystems_mut().auras;
            assert!(auras.set_aura_persistence_metadata(
                3,
                wow_entities::AuraPersistenceMetadata {
                    item_guid: wow_core::ObjectGuid::EMPTY,
                    cast_item_id: 7,
                    cast_item_level: 42,
                    recalculate_mask: 1,
                    charges: 5,
                    base_amounts: vec![(0, 777)],
                },
            ));
            player.set_player_aura_authority_complete_like_cpp(true);
        })
        .unwrap();

    let saved = capture_sickness(&mut session).unwrap();
    let aura = &saved.families[0].auras[0];
    assert_eq!(
        (aura.cast_item_id, aura.cast_item_level, aura.charges),
        (7, 42, 5)
    );
    assert_eq!(aura.recalculate_mask, 1);
    assert_eq!(aura.effects[0].amount, 400);
    assert_eq!(aura.effects[0].base_amount, 777);
}

#[test]
fn incomplete_aura_authority_preserves_rows_with_or_without_a_runtime_aura() {
    let (mut session, _, _) = make_session();
    let player_guid = install_canonical_player_owner_for_test(&mut session, 571, 0);
    let spell_id = 54_321;
    install_sickness_catalogs(&mut session, spell_id, &[(0, 80, 10)]);
    session
        .with_owned_player_mut_like_cpp(|player| player.unit_mut().set_race(2))
        .unwrap();
    assert_eq!(capture_sickness(&mut session), None);

    insert_sickness_application(
        &mut session,
        sickness_application(
            player_guid,
            spell_id,
            1,
            1,
            30_000,
            20_000,
            std::time::Instant::now(),
        ),
        Some(vec![(1, 80, 10, 0)]),
    );
    assert_eq!(capture_sickness(&mut session), None);
}

#[test]
fn partial_mask_and_unknown_effect_fail_closed_while_zero_duration_is_saved_exactly() {
    let (mut partial, _, _) = make_session();
    let player_guid = install_canonical_player_owner_for_test(&mut partial, 571, 0);
    let spell_id = 54_321;
    install_sickness_catalogs(&mut partial, spell_id, &[(0, 80, 10), (2, 159, 20)]);
    partial
        .with_owned_player_mut_like_cpp(|player| {
            player.unit_mut().set_race(2);
            player.set_player_aura_authority_complete_like_cpp(true);
        })
        .unwrap();
    insert_sickness_application(
        &mut partial,
        sickness_application(
            player_guid,
            spell_id,
            2,
            0b101,
            30_000,
            20_000,
            std::time::Instant::now(),
        ),
        Some(vec![(1, 80, 10, 0)]),
    );
    assert_eq!(capture_sickness(&mut partial), None);

    let (mut unknown, _, _) = make_session();
    let player_guid = install_canonical_player_owner_for_test(&mut unknown, 571, 0);
    install_sickness_catalogs(&mut unknown, spell_id, &[(0, 17, 10)]);
    unknown
        .with_owned_player_mut_like_cpp(|player| {
            player.unit_mut().set_race(2);
            player.set_player_aura_authority_complete_like_cpp(true);
        })
        .unwrap();
    insert_sickness_application(
        &mut unknown,
        sickness_application(
            player_guid,
            spell_id,
            2,
            1,
            30_000,
            20_000,
            std::time::Instant::now(),
        ),
        Some(vec![(1, 17, 10, 0)]),
    );
    assert_eq!(capture_sickness(&mut unknown), None);

    let (mut no_duration, _, _) = make_session();
    let player_guid = install_canonical_player_owner_for_test(&mut no_duration, 571, 0);
    install_sickness_catalogs(&mut no_duration, spell_id, &[(0, 80, 10)]);
    no_duration
        .with_owned_player_mut_like_cpp(|player| {
            player.unit_mut().set_race(2);
            player.set_player_aura_authority_complete_like_cpp(true);
        })
        .unwrap();
    insert_sickness_application(
        &mut no_duration,
        sickness_application(player_guid, spell_id, 2, 1, 0, 0, std::time::Instant::now()),
        Some(vec![(1, 80, 10, 0)]),
    );
    let saved = capture_sickness(&mut no_duration)
        .expect("a present zero-maximum sickness application remains a valid snapshot");
    assert_eq!(saved.families.len(), 1);
    assert_eq!(
        (
            saved.families[0].auras[0].max_duration,
            saved.families[0].auras[0].remaining_duration,
        ),
        (0, 0)
    );
}

#[test]
fn stale_timestamp_keeps_the_present_finite_sickness_snapshot() {
    let (mut session, _, _) = make_session();
    let player_guid = install_canonical_player_owner_for_test(&mut session, 571, 0);
    let spell_id = 54_321;
    install_sickness_catalogs(&mut session, spell_id, &[(0, 80, 10)]);
    session
        .with_owned_player_mut_like_cpp(|player| {
            player.unit_mut().set_race(2);
            player.set_player_aura_authority_complete_like_cpp(true);
        })
        .unwrap();
    insert_sickness_application(
        &mut session,
        sickness_application(
            player_guid,
            spell_id,
            2,
            1,
            30_000,
            0,
            std::time::Instant::now() - std::time::Duration::from_secs(5),
        ),
        Some(vec![(1, 80, 10, 0)]),
    );
    let saved = capture_sickness(&mut session).unwrap();
    assert_eq!(saved.families.len(), 1);
    assert_eq!(
        (
            saved.families[0].auras[0].max_duration,
            saved.families[0].auras[0].remaining_duration,
        ),
        (30_000, 0),
        "a stale wall-clock timestamp does not rewrite a present application"
    );
}

#[test]
fn selected_spell_missing_from_catalog_cannot_authorize_an_empty_delete() {
    let (mut session, _, _) = make_session();
    install_canonical_player_owner_for_test(&mut session, 571, 0);
    let spell_id = 54_321;
    session
        .with_owned_player_mut_like_cpp(|player| {
            player.unit_mut().set_race(2);
            player.set_player_aura_authority_complete_like_cpp(true);
        })
        .unwrap();
    assert_eq!(
        capture_sickness(&mut session),
        None,
        "missing catalogs preserve rows"
    );

    session.set_chr_races_store(std::sync::Arc::new(
        wow_data::character_progression::ChrRacesStore::from_entries([sickness_race_row(spell_id)]),
    ));
    let mut selected_spell_store = wow_data::SpellStore::new();
    selected_spell_store.insert(spell_id, sickness_spell_info(spell_id, &[(0, 80, 10)]));
    session.set_spell_store(std::sync::Arc::new(selected_spell_store));
    assert_eq!(
        capture_sickness(&mut session),
        None,
        "missing difficulty store preserves rows"
    );

    session.set_spell_store(std::sync::Arc::new(wow_data::SpellStore::new()));
    session.set_difficulty_store(std::sync::Arc::new(wow_data::DifficultyStore::from_ids([
        0,
    ])));
    assert_eq!(
        capture_sickness(&mut session),
        None,
        "missing selected SpellInfo preserves rows"
    );
}

#[test]
fn zero_effect_mask_and_loaded_recalculate_bits_outside_the_mask_fail_closed() {
    let (mut zero_mask, _, _) = make_session();
    let player_guid = install_canonical_player_owner_for_test(&mut zero_mask, 571, 0);
    let spell_id = 54_321;
    install_sickness_catalogs(&mut zero_mask, spell_id, &[(0, 80, 10)]);
    zero_mask
        .with_owned_player_mut_like_cpp(|player| {
            player.unit_mut().set_race(2);
            player.set_player_aura_authority_complete_like_cpp(true);
        })
        .unwrap();
    insert_sickness_application(
        &mut zero_mask,
        sickness_application(
            player_guid,
            spell_id,
            4,
            0,
            30_000,
            20_000,
            std::time::Instant::now(),
        ),
        Some(Vec::new()),
    );
    assert_eq!(capture_sickness(&mut zero_mask), None);

    let (mut invalid_recalc, _, _) = make_session();
    let player_guid = install_canonical_player_owner_for_test(&mut invalid_recalc, 571, 0);
    install_sickness_catalogs(&mut invalid_recalc, spell_id, &[(0, 80, 10)]);
    invalid_recalc
        .with_owned_player_mut_like_cpp(|player| {
            player.unit_mut().set_race(2);
        })
        .unwrap();
    insert_sickness_application(
        &mut invalid_recalc,
        sickness_application(
            player_guid,
            spell_id,
            5,
            1,
            30_000,
            20_000,
            std::time::Instant::now(),
        ),
        Some(vec![(1, 80, 10, 0)]),
    );
    invalid_recalc
        .with_owned_player_mut_like_cpp(|player| {
            let auras = &mut player.unit_mut().subsystems_mut().auras;
            assert!(auras.set_aura_persistence_metadata(
                5,
                wow_entities::AuraPersistenceMetadata {
                    item_guid: wow_core::ObjectGuid::EMPTY,
                    cast_item_id: 0,
                    cast_item_level: -1,
                    recalculate_mask: 2,
                    charges: 0,
                    base_amounts: vec![(0, 10)],
                },
            ));
            player.set_player_aura_authority_complete_like_cpp(true);
        })
        .unwrap();
    assert_eq!(capture_sickness(&mut invalid_recalc), None);
}

#[test]
fn duplicate_sickness_effect_masks_do_not_create_duplicate_database_keys() {
    let (mut session, _, _) = make_session();
    let player_guid = install_canonical_player_owner_for_test(&mut session, 571, 0);
    let spell_id = 54_321;
    install_sickness_catalogs(&mut session, spell_id, &[(0, 80, 10)]);
    session
        .with_owned_player_mut_like_cpp(|player| {
            player.unit_mut().set_race(2);
        })
        .unwrap();
    for slot in [1, 2] {
        insert_sickness_application(
            &mut session,
            sickness_application(
                player_guid,
                spell_id,
                slot,
                1,
                30_000,
                20_000,
                std::time::Instant::now(),
            ),
            Some(vec![(1, 80, i32::from(slot), 0)]),
        );
    }
    session
        .with_owned_player_mut_like_cpp(|player| {
            player.set_player_aura_authority_complete_like_cpp(true);
        })
        .unwrap();
    assert_eq!(capture_sickness(&mut session), None);
}
