// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use super::*;
use std::future::Future;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::task::Poll;
use wow_persistence::{
    CorpsePersistenceMutationOutcomeLikeCpp, CorpseSaveRequestLikeCpp, MapCorpseLoadOutcomeLikeCpp,
    MapCorpseLoadRequestLikeCpp, MapCorpsePersistencePortLikeCpp, PersistenceFutureLikeCpp,
};

#[tokio::test]
async fn release_publishes_exact_corpse_before_dead_cleanup_and_water_walk() {
    let (mut session, send_rx, _realm_rx, owner, port) =
        dead_player_session_with_graveyard_like_cpp(9_244);
    let own_output = session.core.send_tx().clone();
    session.install_realm_send_channel_for_test(own_output);
    let registry = Arc::new(wow_world_core::player_directory::PlayerRegistry::new());
    session.set_player_registry(Arc::clone(&registry));
    session.core.state = crate::session::SessionState::LoggedIn;
    session
        .core
        .with_owned_player_mut_like_cpp(|player| {
            player.unit_mut().world_mut().set_name("CorpseOwner");
        })
        .unwrap();
    session.register_in_player_registry();
    let canonical = Arc::clone(session.core.canonical_map_manager.as_ref().unwrap());
    let (mut peer, peer_output) = make_session();
    let peer_guid = ObjectGuid::create_player(1, 9_246);
    peer.set_player_guid(Some(peer_guid));
    peer.set_canonical_map_manager(Arc::clone(&canonical));
    peer.set_player_registry(registry);
    peer.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    let handle = {
        let mut manager = canonical.lock().unwrap();
        let mut player = Box::new(wow_entities::Player::new(Some(1), false));
        player.unit_mut().world_mut().object_mut().create(peer_guid);
        player.unit_mut().world_mut().set_name("CorpsePeer");
        player.unit_mut().set_max_health(100);
        player.unit_mut().set_health(100);
        let handle = manager.install_detached_player_like_cpp(player).unwrap();
        manager
            .attach_player_like_cpp(
                handle,
                wow_map::MapKey::new(571, 0),
                Position::new(2.0, 2.0, 3.0, 0.0),
            )
            .unwrap();
        handle
    };
    peer.core.player_handle_like_cpp = Some(handle);
    peer.core.player_identity_bootstrap_like_cpp = None;
    peer.core.state = crate::session::SessionState::LoggedIn;
    peer.core.client_visible_guids_like_cpp.insert(owner);
    peer.register_in_player_registry();
    let unrelated = ObjectGuid::create_player(1, 9_247);
    session.core.client_visible_guids_like_cpp.insert(unrelated);
    peer.core.client_visible_guids_like_cpp.insert(unrelated);
    let generators = session.id_generators_for_test_like_cpp();
    let creatures = session.creature_spawn_catalogs_for_test_like_cpp();
    let graveyards = wow_data::GraveyardStore::default();

    assert!(
        wow_world_entities::build_player_repop_like_cpp(
            &mut session,
            wow_world_entities::CorpseLifecycleCatalogsLikeCpp {
                item_guid_generator: generators.item.as_ref(),
                creature_spawns: &creatures,
                graveyards: &graveyards,
            },
        )
        .await
    );

    let corpse_guid = current_corpse_guid_like_cpp(&session, owner).unwrap();
    let create_data = {
        let manager = session
            .core
            .canonical_map_manager
            .as_ref()
            .unwrap()
            .lock()
            .unwrap();
        let corpse = manager
            .find_map(571, 0)
            .unwrap()
            .map()
            .get_typed_corpse(corpse_guid)
            .unwrap();
        wow_world_core::entity_update_bridge::corpse_create_data_from_entity_like_cpp(corpse)
    };
    let expected_create = wow_packet::packets::update::UpdateObject {
        map_id: 571,
        num_updates: 1,
        destroy_guids: Vec::new(),
        out_of_range_guids: Vec::new(),
        blocks: vec![
            wow_packet::packets::update::UpdateObject::create_corpse_birth_block(create_data),
        ],
    }
    .to_bytes();
    let packets = send_rx.try_iter().collect::<Vec<_>>();
    let create = packets
        .iter()
        .position(|bytes| bytes == &expected_create)
        .expect("the committed corpse's exact CREATE is published without another Map tick");
    assert_eq!(
        packets
            .iter()
            .filter(|bytes| *bytes == &expected_create)
            .count(),
        1
    );
    assert_eq!(
        packets[create + 1],
        wow_packet::packets::combat::CancelCombat.to_bytes()
    );
    assert_eq!(
        u16::from_le_bytes([packets[create + 2][0], packets[create + 2][1]]),
        ServerOpcodes::MoveSetWaterWalk as u16,
        "DEAD cleanup follows CREATE and precedes ghost movement"
    );
    assert!(
        session
            .core
            .client_visible_guids_like_cpp
            .contains(&corpse_guid)
    );
    assert_eq!(
        session.core.with_owned_player_like_cpp(|player| (
            player.unit().death_state(),
            player.unit().data().health,
            player.resurrection_state_like_cpp().delayed_after_teleport,
        )),
        Some((wow_constants::DeathState::Dead, 1, None))
    );
    assert_eq!(port.saves.lock().unwrap().len(), 1);
    let peer_packets = peer_output.try_iter().collect::<Vec<_>>();
    assert_eq!(peer_packets.first(), Some(&expected_create));
    assert!(
        peer_packets
            .iter()
            .skip(1)
            .any(|bytes| u16::from_le_bytes([bytes[0], bytes[1]])
                == ServerOpcodes::MoveUpdate as u16),
        "the peer learns the corpse before the owner's ghost movement"
    );
    assert!(
        peer.core
            .client_visible_guids_like_cpp
            .contains(&corpse_guid)
    );
    assert!(peer.core.client_visible_guids_like_cpp.contains(&unrelated));
    assert!(
        session
            .core
            .client_visible_guids_like_cpp
            .contains(&unrelated)
    );
}

#[tokio::test]
async fn standalone_create_corpse_preserves_native_state_without_visibility_catalogs() {
    let (mut session, send_rx, realm_rx, owner, _) =
        dead_player_session_with_graveyard_like_cpp(9_245);
    let before = session
        .core
        .with_owned_player_like_cpp(|player| {
            (player.unit().death_state(), player.unit().data().health)
        })
        .unwrap();
    let generators = session.id_generators_for_test_like_cpp();
    let corpse = wow_world_entities::create_corpse_like_cpp(&mut session, generators.item.as_ref())
        .await
        .expect("standalone corpse creation");
    assert_eq!(current_corpse_guid_like_cpp(&session, owner), Some(corpse));
    assert_eq!(
        session.core.with_owned_player_like_cpp(|player| (
            player.unit().death_state(),
            player.unit().data().health,
        )),
        Some(before)
    );
    assert!(!session.core.client_visible_guids_like_cpp.contains(&corpse));
    assert!(send_rx.is_empty());
    assert!(realm_rx.is_empty());
    assert_eq!(release_phase(&session), None);
}

#[tokio::test]
async fn corpse_equipment_uses_native_items_and_active_spec_appearance_instead_of_ui_cache() {
    use wow_constants::ItemModifier;
    use wow_entities::{EQUIPMENT_SLOT_CHEST, Item, PlayerInventoryItem, VisibleItemValues};

    for (index, (group, appearance, all_specs, spec1, spec2, stale_ui, expected)) in [
        (0, 0, 0, 0, 0, false, 1_101 | (5 << 24)),
        (0, 2, 0, 0, 0, true, 1_102 | (5 << 24)),
        (0, 0, 20, 0, 0, true, 2_202 | (20 << 24)),
        (0, 0, 20, 30, 40, true, 3_303 | (1 << 24)),
        (1, 0, 20, 30, 0, true, 2_202 | (20 << 24)),
        (1, 0, 20, 30, 40, true, 4_404 | (5 << 24)),
        (0, 2, 99_999, 0, 0, true, 1_102 | (5 << 24)),
        (0, 0, 50, 0, 0, true, 1_101 | (20 << 24)),
    ]
    .into_iter()
    .enumerate()
    {
        let (mut session, _send_rx, _realm_rx, owner, port) =
            dead_player_session_with_graveyard_like_cpp(9_250 + index as i64);
        session.set_item_store(Arc::new(wow_data::ItemStore::from_records(
            [(70_001, 5), (70_002, 20), (70_003, 1)]
                .into_iter()
                .map(|(id, inventory_type)| wow_data::ItemRecord {
                    id,
                    class_id: 4,
                    subclass_id: 0,
                    material: 0,
                    inventory_type,
                    sheathe_type: 0,
                    random_select: 0,
                    random_suffix_group_id: 0,
                    scaling_stat_distribution_id: 0,
                    scaling_stat_value: 0,
                }),
        )));
        session.set_item_stats_store(Arc::new(
            wow_data::ItemStatsStore::from_stats_sparse_and_random_property_templates(
                [],
                [(
                    70_001,
                    wow_data::ItemSparseTemplateEntry {
                        flags: [0; 4],
                        bag_family: 0,
                        start_quest_id: 0,
                        stackable: 1,
                        max_count: 0,
                        lock_id: 0,
                        required_reputation_rank: 0,
                        sell_price: 0,
                        buy_price: 0,
                        vendor_stack_count: 1,
                        price_variance: 0.0,
                        price_random_value: 0.0,
                        max_durability: 0,
                        other_faction_item_id: 0,
                        content_tuning_id: 0,
                        player_level_to_item_level_curve_id: 0,
                        limit_category: 0,
                        instance_bound: 0,
                        zone_bound: [0; 2],
                        required_reputation_faction: 0,
                        allowable_class: -1,
                        required_expansion: 0,
                        bonding: 0,
                        container_slots: 0,
                        inventory_type: 5,
                    },
                )],
                [],
            ),
        ));
        let appearances = [
            (10, 70_001, 0, 1_000, 1_101),
            (11, 70_001, 2, 1_001, 1_102),
            (20, 70_002, 3, 1_002, 2_202),
            (30, 70_003, 4, 1_003, 3_303),
            // No Item.db2 row for this visible entry: use the physical template's type.
            (40, 70_004, 5, 1_004, 4_404),
            // The selected transmog's missing appearance falls back to physical display only.
            (50, 70_002, 6, 19_999, 0),
        ];
        session.set_item_modified_appearance_store(Arc::new(
            wow_data::ItemModifiedAppearanceStore::from_entries(appearances.into_iter().map(
                |(id, item_id, item_appearance_modifier_id, item_appearance_id, _)| {
                    wow_data::ItemModifiedAppearanceEntry {
                        id,
                        item_id,
                        item_appearance_modifier_id,
                        item_appearance_id,
                        order_index: 0,
                        transmog_source_type_enum: 0,
                    }
                },
            )),
        ));
        session.set_item_appearance_store(Arc::new(wow_data::ItemAppearanceStore::from_entries(
            appearances
                .into_iter()
                .filter(|(_, _, _, id, _)| *id != 19_999)
                .map(
                    |(_, _, _, id, item_display_info_id)| wow_data::ItemAppearanceEntry {
                        id: id as u32,
                        display_type: 0,
                        item_display_info_id,
                        default_icon_file_data_id: 0,
                        ui_order: 0,
                    },
                ),
        )));
        let item_guid = ObjectGuid::create_item(owner.realm_id(), 9_300 + index as i64);
        session
            .core
            .with_owned_player_mut_like_cpp(|player| {
                let mut talents = player.talent_runtime_like_cpp().clone();
                talents.set_active_group_like_cpp(group);
                player.replace_talent_runtime_like_cpp(talents);
                let mut item = Item::default();
                item.object_mut().create(item_guid);
                item.object_mut().set_entry(70_001);
                item.set_owner_guid(owner);
                item.set_contained_in(owner);
                item.set_slot(EQUIPMENT_SLOT_CHEST);
                item.set_count(1);
                item.set_appearance_mod_id(appearance);
                item.set_modifier(ItemModifier::TransmogAppearanceAllSpecs, all_specs);
                item.set_modifier(ItemModifier::TransmogAppearanceSpec1, spec1);
                item.set_modifier(ItemModifier::TransmogAppearanceSpec2, spec2);
                player
                    .store_top_level_item(EQUIPMENT_SLOT_CHEST, item_guid)
                    .unwrap();
                let inventory = player.inventory_runtime_mut_like_cpp();
                inventory.store_item_in_slot_like_cpp(
                    EQUIPMENT_SLOT_CHEST,
                    PlayerInventoryItem {
                        guid: item_guid,
                        entry_id: 70_001,
                        db_guid: item_guid.counter() as u64,
                        inventory_type: Some(5),
                    },
                );
                inventory.store_item_object_like_cpp(item);
                player.set_visible_item_slot(
                    EQUIPMENT_SLOT_CHEST,
                    Some(VisibleItemValues {
                        item_id: if stale_ui { 70_002 } else { 0 },
                        item_appearance_mod_id: 3,
                        item_visual: 0,
                    }),
                );
                // A stale UI entry in an empty native slot must not dress the corpse.
                player.set_visible_item_slot(
                    0,
                    Some(VisibleItemValues {
                        item_id: 70_002,
                        item_appearance_mod_id: 3,
                        item_visual: 0,
                    }),
                );
            })
            .expect("canonical equipment owner");
        let generators = session.id_generators_for_test_like_cpp();
        let corpse_guid =
            wow_world_entities::create_corpse_like_cpp(&mut session, generators.item.as_ref())
                .await
                .expect("native equipped corpse creation");
        let actual = session
            .core
            .canonical_map_manager
            .as_ref()
            .unwrap()
            .lock()
            .unwrap()
            .find_map(571, 0)
            .unwrap()
            .map()
            .get_typed_corpse(corpse_guid)
            .unwrap()
            .data()
            .items;
        let mut expected_items = [0; 19];
        expected_items[usize::from(EQUIPMENT_SLOT_CHEST)] = expected;
        assert_eq!(actual, expected_items, "appearance case {index}");
        let saves = port.saves.lock().unwrap();
        assert_eq!(saves.len(), 1);
        assert_eq!(
            saves[0]
                .item_cache
                .split_ascii_whitespace()
                .map(|value| value.parse::<u32>().unwrap())
                .collect::<Vec<_>>(),
            expected_items
        );
    }
}

#[tokio::test]
async fn release_clears_absent_pet_stable_slot_without_erasing_stale_guid() {
    let (mut session, _send_rx, _realm_rx, _owner, port) =
        dead_player_session_with_graveyard_like_cpp(9_240);
    let stale =
        ObjectGuid::create_world_object(wow_core::guid::HighGuid::Pet, 0, 1, 571, 0, 1, 9_241);
    session
        .core
        .with_owned_player_mut_like_cpp(|player| {
            player.set_pet_guid_like_cpp(Some(stale));
            player
                .pet_lifecycle_state_mut_like_cpp()
                .stable
                .current_pet_index = Some(0);
        })
        .unwrap();

    handle_repop_request(&mut session, repop_request_packet(true)).await;

    assert_eq!(
        port.saves.lock().unwrap().len(),
        1,
        "release must reach corpse persistence"
    );
    assert_eq!(
        session.core.with_owned_player_like_cpp(|player| (
            player.gameplay_state().pet_guid,
            player
                .pet_lifecycle_state_like_cpp()
                .stable
                .current_pet_index,
        )),
        Some((Some(stale), None))
    );
}

#[tokio::test]
async fn release_preserves_active_pet_when_removal_cannot_complete() {
    let (mut session, send_rx, _realm_rx, owner, port) =
        dead_player_session_with_graveyard_like_cpp(9_242);
    let pet_guid =
        ObjectGuid::create_world_object(wow_core::guid::HighGuid::Pet, 0, 1, 571, 0, 1, 9_243);
    let mut pet = wow_entities::Pet::new(owner, wow_entities::PetType::Hunter);
    pet.creature_mut()
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(pet_guid);
    pet.creature_mut()
        .unit_mut()
        .world_mut()
        .set_map(571, 0)
        .unwrap();
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
            player.set_pet_guid_like_cpp(Some(pet_guid));
            let pets = player.pet_lifecycle_state_mut_like_cpp();
            pets.temporary_unsummoned_pet_number = 42;
            pets.stable.current_pet_index = Some(0);
        })
        .unwrap();

    handle_repop_request(&mut session, repop_request_packet(true)).await;

    assert!(port.saves.lock().unwrap().is_empty());
    assert!(current_corpse_guid_like_cpp(&session, owner).is_none());
    assert_ne!(session.state(), crate::session::SessionState::Disconnecting);
    assert!(!sent_opcodes(&send_rx).contains(&(ServerOpcodes::MoveTeleport as u16)));
    assert_eq!(
        session.core.with_owned_player_like_cpp(|player| {
            let pets = player.pet_lifecycle_state_like_cpp();
            (
                player.gameplay_state().pet_guid,
                pets.temporary_unsummoned_pet_number,
                pets.stable.current_pet_index,
            )
        }),
        Some((Some(pet_guid), 42, Some(0)))
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
            .get_typed_pet(pet_guid)
            .is_some_and(|pet| !pet.is_removed())
    );
}

#[derive(Default)]
struct GatedCorpseSavePort {
    receiver: Mutex<Option<flume::Receiver<CorpsePersistenceMutationOutcomeLikeCpp>>>,
    started: AtomicBool,
    saves: AtomicUsize,
}

impl MapCorpsePersistencePortLikeCpp for GatedCorpseSavePort {
    fn load_map_corpses_like_cpp<'a>(
        &'a self,
        _request: MapCorpseLoadRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, MapCorpseLoadOutcomeLikeCpp> {
        Box::pin(async {
            MapCorpseLoadOutcomeLikeCpp::Failed {
                reason: "not used".into(),
            }
        })
    }

    fn save_corpse_like_cpp<'a>(
        &'a self,
        _request: CorpseSaveRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, CorpsePersistenceMutationOutcomeLikeCpp> {
        self.saves.fetch_add(1, Ordering::SeqCst);
        self.started.store(true, Ordering::SeqCst);
        let receiver = self.receiver.lock().unwrap().take().unwrap();
        Box::pin(async move {
            receiver.recv_async().await.unwrap_or_else(|_| {
                CorpsePersistenceMutationOutcomeLikeCpp::Failed {
                    reason: "controlled save result channel closed".into(),
                }
            })
        })
    }

    fn delete_corpse_like_cpp<'a>(
        &'a self,
        _owner_guid: u64,
    ) -> PersistenceFutureLikeCpp<'a, CorpsePersistenceMutationOutcomeLikeCpp> {
        Box::pin(async { CorpsePersistenceMutationOutcomeLikeCpp::Committed })
    }
}

fn release_phase(session: &WorldSession) -> Option<wow_entities::PlayerResurrectionPhase> {
    session
        .core
        .with_owned_player_like_cpp(|player| {
            player
                .resurrection_state_like_cpp()
                .delayed_after_teleport
                .map(|continuation| continuation.phase)
        })
        .flatten()
}

fn release_phase_at(
    manager: &wow_map::MapManager,
    handle: wow_map::PlayerHandle,
) -> Option<wow_entities::PlayerResurrectionPhase> {
    manager.with_player_like_cpp(handle, |player| {
        player
            .resurrection_state_like_cpp()
            .delayed_after_teleport
            .map(|continuation| continuation.phase)
    })?
}

fn corpse_guid_at(manager: &wow_map::MapManager, player_guid: ObjectGuid) -> Option<ObjectGuid> {
    manager
        .find_map(571, 0)?
        .map()
        .corpse_guid_by_player_like_cpp(player_guid)
}

fn assert_release_failed_before_graveyard(
    session: &WorldSession,
    player_guid: ObjectGuid,
    send_rx: &flume::Receiver<Vec<u8>>,
) {
    assert_eq!(session.state(), crate::session::SessionState::Disconnecting);
    assert!(current_corpse_guid_like_cpp(session, player_guid).is_none());
    assert_eq!(
        wow_world_entities::player_corpse_location_like_cpp(session),
        None
    );
    assert_eq!(
        session.fixtures.combat.represented_repop_at_graveyard_count,
        0
    );
    assert!(!crate::session::hub_ref(session).near_teleport_pending_like_cpp());
    assert!(!sent_opcodes(send_rx).contains(&(ServerOpcodes::MoveTeleport as u16)));
    assert_eq!(
        release_phase(session),
        Some(wow_entities::PlayerResurrectionPhase::CorpseSaveInFlight)
    );
}

fn assert_release_phase_rejects_late_ack_and_full_save(session: &mut WorldSession) {
    let before = release_phase(session);
    assert!(!wow_world_entities::mark_player_resurrection_teleport_completed(session));
    assert_eq!(
        release_phase(session),
        before,
        "late ACK must retain the fence"
    );
    assert!(
        session.player_save_snapshot_blocked_for_test(1),
        "a full Player snapshot must remain blocked during release"
    );
}

#[tokio::test]
async fn release_failure_corpse_save_retains_fence_and_never_repopulates() {
    let (mut session, send_rx, _realm_rx, player_guid, port) =
        dead_player_session_with_graveyard_like_cpp(9_201);
    port.fail_next_save();

    handle_repop_request(&mut session, repop_request_packet(true)).await;

    assert_eq!(port.saves.lock().unwrap().len(), 1);
    assert_release_failed_before_graveyard(&session, player_guid, &send_rx);
    assert_release_phase_rejects_late_ack_and_full_save(&mut session);

    handle_repop_request(&mut session, repop_request_packet(true)).await;
    assert_eq!(
        port.saves.lock().unwrap().len(),
        1,
        "duplicate must not replay SQL"
    );
    assert_release_failed_before_graveyard(&session, player_guid, &send_rx);
    assert_release_phase_rejects_late_ack_and_full_save(&mut session);
}

#[tokio::test]
async fn release_failure_missing_corpse_port_retains_fence_and_never_repopulates() {
    let (mut session, send_rx, _realm_rx, player_guid) =
        dead_player_session_without_corpse_port(9_202);

    handle_repop_request(&mut session, repop_request_packet(true)).await;

    assert_release_failed_before_graveyard(&session, player_guid, &send_rx);
}

#[tokio::test]
async fn release_failure_cancelled_corpse_save_keeps_request_unpublished() {
    let (mut session, _send_rx, _realm_rx, player_guid, _recording_port) =
        dead_player_session_with_graveyard_like_cpp(9_203);
    let (save_tx, save_rx) = flume::bounded(1);
    let port = Arc::new(GatedCorpseSavePort {
        receiver: Mutex::new(Some(save_rx)),
        ..Default::default()
    });
    session.set_map_corpse_persistence_port_like_cpp(port.clone());
    let handle = session.core.player_handle_like_cpp.unwrap();
    let manager = Arc::clone(session.core.canonical_map_manager.as_ref().unwrap());

    let mut release = Box::pin(handle_repop_request(
        &mut session,
        repop_request_packet(true),
    ));
    let reached_pending_save = std::future::poll_fn(|cx| {
        let polled = release.as_mut().poll(cx);
        if port.started.load(Ordering::SeqCst) || polled.is_ready() {
            Poll::Ready(port.started.load(Ordering::SeqCst) && polled.is_pending())
        } else {
            Poll::Pending
        }
    })
    .await;
    assert!(reached_pending_save);
    assert!(port.started.load(Ordering::SeqCst));
    assert_eq!(port.saves.load(Ordering::SeqCst), 1);
    {
        let manager = manager.lock().unwrap();
        assert!(corpse_guid_at(&manager, player_guid).is_none());
        assert_eq!(
            release_phase_at(&manager, handle),
            Some(wow_entities::PlayerResurrectionPhase::CorpseSaveInFlight)
        );
    }

    drop(release);
    handle_repop_request(&mut session, repop_request_packet(true)).await;
    assert_eq!(
        port.saves.load(Ordering::SeqCst),
        1,
        "cancelled SQL must not replay"
    );
    assert!(current_corpse_guid_like_cpp(&session, player_guid).is_none());
    assert_eq!(
        release_phase(&session),
        Some(wow_entities::PlayerResurrectionPhase::CorpseSaveInFlight)
    );
    assert_release_phase_rejects_late_ack_and_full_save(&mut session);
    assert_eq!(
        session.fixtures.combat.represented_repop_at_graveyard_count,
        0
    );
    drop(save_tx);
}

#[derive(Clone, Copy)]
enum StaleAdmissionChange {
    ResidenceRevision,
    Incarnation,
}

async fn assert_stale_release_completion_is_rejected(
    guid_counter: i64,
    change: StaleAdmissionChange,
) {
    let (mut session, send_rx, _realm_rx, player_guid, _recording_port) =
        dead_player_session_with_graveyard_like_cpp(guid_counter);
    let (save_tx, save_rx) = flume::bounded(1);
    let port = Arc::new(GatedCorpseSavePort {
        receiver: Mutex::new(Some(save_rx)),
        ..Default::default()
    });
    session.set_map_corpse_persistence_port_like_cpp(port.clone());
    let handle = session.core.player_handle_like_cpp.unwrap();
    let manager = Arc::clone(session.core.canonical_map_manager.as_ref().unwrap());
    let initial_admission = manager
        .lock()
        .unwrap()
        .current_player_admission_like_cpp(player_guid)
        .expect("initial canonical admission");
    assert_eq!(initial_admission.0, handle);
    let original_position = session
        .player_position_like_cpp()
        .expect("active Player fixture position");
    let mut release = Box::pin(handle_repop_request(
        &mut session,
        repop_request_packet(true),
    ));
    let pending_at_save = std::future::poll_fn(|cx| {
        let polled = release.as_mut().poll(cx);
        if port.started.load(Ordering::SeqCst) || polled.is_ready() {
            Poll::Ready(port.started.load(Ordering::SeqCst) && polled.is_pending())
        } else {
            Poll::Pending
        }
    })
    .await;
    assert!(pending_at_save);
    assert!(port.started.load(Ordering::SeqCst));
    let expected_current_handle = match change {
        StaleAdmissionChange::ResidenceRevision => {
            let mut manager = manager.lock().unwrap();
            manager
                .detach_player_like_cpp(handle)
                .expect("detach original residence");
            manager
                .attach_player_like_cpp(handle, wow_map::MapKey::new(571, 0), original_position)
                .expect("reattach original incarnation");
            handle
        }
        StaleAdmissionChange::Incarnation => {
            let mut replacement = Box::new(wow_entities::Player::new(Some(2), false));
            replacement
                .unit_mut()
                .world_mut()
                .object_mut()
                .create(player_guid);
            let replacement_handle = manager
                .lock()
                .unwrap()
                .install_detached_player_like_cpp(replacement)
                .unwrap();
            assert_ne!(replacement_handle, handle);
            manager
                .lock()
                .unwrap()
                .with_player_mut_like_cpp(replacement_handle, |player| {
                    player.unit_mut().set_max_health(200);
                    player.unit_mut().set_health(123);
                    player
                        .unit_mut()
                        .set_unit_flags_like_cpp(wow_constants::UnitFlags::SKINNABLE);
                })
                .expect("replacement canonical incarnation");
            replacement_handle
        }
    };
    let expected_residence_revision = match change {
        StaleAdmissionChange::ResidenceRevision => initial_admission.1 + 1,
        StaleAdmissionChange::Incarnation => 0,
    };
    {
        let manager = manager.lock().unwrap();
        assert_eq!(
            manager.current_player_admission_like_cpp(player_guid),
            Some((expected_current_handle, expected_residence_revision))
        );
        if matches!(change, StaleAdmissionChange::Incarnation) {
            assert_eq!(
                manager.with_player_like_cpp(expected_current_handle, |player| {
                    (
                        player.unit().death_state(),
                        player.unit().data().max_health,
                        player.unit().data().health,
                        player.unit().unit_flags_like_cpp(),
                        player.resurrection_state_like_cpp().corpse_location,
                        player.resurrection_state_like_cpp().delayed_after_teleport,
                    )
                }),
                Some((
                    wow_constants::DeathState::Alive,
                    200,
                    123,
                    wow_constants::UnitFlags::SKINNABLE,
                    None,
                    None,
                )),
                "replacement baseline must be canonical before the save resumes"
            );
        }
    }
    save_tx
        .send(CorpsePersistenceMutationOutcomeLikeCpp::Committed)
        .unwrap();
    release.await;

    assert_eq!(port.saves.load(Ordering::SeqCst), 1);
    assert_eq!(session.state(), crate::session::SessionState::Disconnecting);
    assert!(current_corpse_guid_like_cpp(&session, player_guid).is_none());
    assert_eq!(
        session.fixtures.combat.represented_repop_at_graveyard_count,
        0
    );
    assert!(!crate::session::hub_ref(&session).near_teleport_pending_like_cpp());
    assert!(!sent_opcodes(&send_rx).contains(&(ServerOpcodes::MoveTeleport as u16)));
    assert_eq!(session.core.player_handle_like_cpp, Some(handle));
    let manager = manager.lock().unwrap();
    match change {
        StaleAdmissionChange::ResidenceRevision => assert_eq!(
            manager.with_player_like_cpp(handle, |player| {
                (
                    player.resurrection_state_like_cpp().corpse_location,
                    player
                        .resurrection_state_like_cpp()
                        .delayed_after_teleport
                        .map(|continuation| continuation.phase),
                )
            }),
            Some((
                None,
                Some(wow_entities::PlayerResurrectionPhase::CorpseSaveInFlight),
            )),
            "stale residence completion must retain the original in-flight fence"
        ),
        StaleAdmissionChange::Incarnation => assert_eq!(
            manager.with_player_like_cpp(expected_current_handle, |player| {
                (
                    player.unit().death_state(),
                    player.unit().data().max_health,
                    player.unit().data().health,
                    player.unit().unit_flags_like_cpp(),
                    player.resurrection_state_like_cpp().corpse_location,
                    player.resurrection_state_like_cpp().delayed_after_teleport,
                )
            }),
            Some((
                wow_constants::DeathState::Alive,
                200,
                123,
                wow_constants::UnitFlags::SKINNABLE,
                None,
                None,
            )),
            "stale completion must not mutate the replacement incarnation"
        ),
    }
}

#[tokio::test]
async fn release_failure_residence_revision_change_after_commit_does_not_publish_corpse() {
    assert_stale_release_completion_is_rejected(9_204, StaleAdmissionChange::ResidenceRevision)
        .await;
}

#[tokio::test]
async fn release_failure_incarnation_change_after_commit_does_not_mutate_replacement() {
    assert_stale_release_completion_is_rejected(9_207, StaleAdmissionChange::Incarnation).await;
}

#[tokio::test]
async fn release_failure_timer_save_does_not_send_graveyard_teleport() {
    let (mut session, send_rx, _realm_rx, player_guid, port) =
        dead_player_session_with_graveyard_like_cpp(9_205);
    let _packet_tx = keep_driver_input_open(&mut session);
    session.core.state = crate::session::SessionState::LoggedIn;
    let (graveyards, conditions) =
        graveyard_store_with_links(0, [wow_data::ALLIANCE_GRAVEYARD_SAFE_LOC_ID_LIKE_CPP], []);
    session.set_graveyard_store(graveyards);
    session.set_condition_store(conditions);
    set_death_timer_for_driver(&mut session, 1);
    port.fail_next_save();

    session.update(1).await;

    assert_eq!(death_timer_from_driver(&session), 0);
    assert_eq!(port.saves.lock().unwrap().len(), 1);
    assert_release_failed_before_graveyard(&session, player_guid, &send_rx);
    assert_release_phase_rejects_late_ack_and_full_save(&mut session);
}

#[tokio::test]
async fn release_failure_old_bones_delete_stops_standalone_replacement() {
    let (mut session, _send_rx, _realm_rx, player_guid, port) =
        dead_player_session_with_graveyard_like_cpp(9_206);
    let generators = session.id_generators_for_test_like_cpp();
    assert!(
        wow_world_entities::create_corpse_like_cpp(&mut session, generators.item.as_ref(),)
            .await
            .is_some()
    );
    assert!(current_corpse_guid_like_cpp(&session, player_guid).is_some());

    port.fail_next_delete();
    assert!(
        wow_world_entities::create_corpse_like_cpp(&mut session, generators.item.as_ref(),)
            .await
            .is_none()
    );

    assert_eq!(port.delete_requests(), vec![9_206]);
    assert_eq!(
        port.saves.lock().unwrap().len(),
        1,
        "no replacement save before oldbones succeeds"
    );
    assert!(current_corpse_guid_like_cpp(&session, player_guid).is_none());
    assert_eq!(
        release_phase(&session),
        Some(wow_entities::PlayerResurrectionPhase::CorpseBonesInFlight)
    );
}

#[tokio::test]
async fn release_failure_same_map_corpse_guard_precedes_oldbones_conversion() {
    let (mut session, send_rx, _realm_rx, player_guid, port) =
        dead_player_session_with_graveyard_like_cpp(9_207);
    session
        .core
        .with_owned_player_mut_like_cpp(|player| {
            player.resurrection_state_mut_like_cpp().corpse_location =
                Some((571, Position::new(8.0, 9.0, 10.0, 0.0)));
        })
        .expect("canonical Player owner");

    handle_repop_request(&mut session, repop_request_packet(true)).await;

    assert_eq!(port.delete_requests(), Vec::<u64>::new());
    assert!(port.saves.lock().unwrap().is_empty());
    assert!(current_corpse_guid_like_cpp(&session, player_guid).is_none());
    assert_eq!(
        wow_world_entities::player_corpse_location_like_cpp(&session),
        Some((571, Position::new(8.0, 9.0, 10.0, 0.0)))
    );
    assert_eq!(
        session.fixtures.combat.represented_repop_at_graveyard_count,
        0
    );
    assert!(!crate::session::hub_ref(&session).near_teleport_pending_like_cpp());
    assert!(!sent_opcodes(&send_rx).contains(&(ServerOpcodes::MoveTeleport as u16)));
    assert_eq!(
        release_phase(&session),
        Some(wow_entities::PlayerResurrectionPhase::CorpseBonesInFlight)
    );
}
