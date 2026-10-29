use super::*;
use wow_constants::ItemUpdateState;
use wow_entities::{EQUIPMENT_SLOT_CHEST, ItemObjectUpdateLikeCpp};

#[test]
fn native_durability_values_are_owner_only_batched_before_player_and_consumed_once() {
    for include_player_delta in [true, false] {
        let (mut owner, _, owner_output) = make_session();
        let (mut peer, _, peer_output) = make_session();
        let canonical = shared_canonical_map_manager();
        let owner_guid = ObjectGuid::create_player(1, 50_710);
        let peer_guid = ObjectGuid::create_player(1, 50_711);
        let item_guid = ObjectGuid::create_item(1, 50_712);
        for (session, guid) in [(&mut owner, owner_guid), (&mut peer, peer_guid)] {
            configure_dynamic_object_values_snapshot_session_like_cpp(
                session, &canonical, guid, 571, 0,
            );
            assert!(session.adopt_registered_canonical_player_fixture_like_cpp());
        }
        peer.core.client_visible_guids_like_cpp.insert(owner_guid);
        owner.set_player_aura_authority_complete_like_cpp(true);
        owner
            .with_owned_player_mut_like_cpp(|player| {
                player.unit_mut().set_max_health(100);
                player.unit_mut().set_health(100);
                let mut item = wow_entities::Item::new(0);
                item.object_mut().create(item_guid);
                item.object_mut().set_entry(2121);
                item.set_owner_guid(owner_guid);
                item.set_contained_in(owner_guid);
                item.set_count(7);
                item.set_slot(EQUIPMENT_SLOT_CHEST);
                item.set_max_durability(60);
                item.set_durability(40);
                item.force_state(ItemUpdateState::Unchanged);
                player
                    .store_top_level_item(EQUIPMENT_SLOT_CHEST, item_guid)
                    .unwrap();
                let runtime = player.inventory_runtime_mut_like_cpp();
                runtime.store_item_in_slot_like_cpp(
                    EQUIPMENT_SLOT_CHEST,
                    wow_entities::PlayerInventoryItem {
                        guid: item_guid,
                        entry_id: 2121,
                        db_guid: item_guid.counter() as u64,
                        inventory_type: Some(InventoryType::Chest as u8),
                    },
                );
                runtime.store_item_object_like_cpp(item);
                runtime.set_equipment_inventory_authority_complete_like_cpp(true);
                player.clear_data_changes();
            })
            .unwrap();
        peer.with_owned_player_mut_like_cpp(|player| player.clear_data_changes())
            .unwrap();

        assert_eq!(
            owner.apply_represented_durability_loss_all_like_cpp(0.1, false),
            1
        );
        assert_eq!(
            owner.apply_represented_durability_loss_all_like_cpp(0.25, false),
            1
        );
        if include_player_delta {
            owner
                .with_owned_player_mut_like_cpp(|player| player.unit_mut().set_health(77))
                .unwrap();
        }
        assert!(
            drain_server_packet_bytes(&owner_output).is_empty(),
            "no immediate Item VALUES"
        );
        canonical.lock().unwrap().update(1);
        assert_eq!(owner.send_represented_player_unit_values_updates_from_last_map_send_object_updates_like_cpp(), 1);
        let bytes = owner_output.try_recv().unwrap();
        assert!(owner_output.try_recv().is_err());
        let mut packet = wow_packet::WorldPacket::from_bytes(&bytes);
        assert_eq!(
            packet.read_uint16().unwrap(),
            ServerOpcodes::UpdateObject as u16
        );
        assert_eq!(
            packet.read_uint32().unwrap(),
            if include_player_delta { 2 } else { 1 }
        );
        assert_eq!(packet.read_uint16().unwrap(), 571);
        assert!(!packet.read_bit().unwrap());
        packet.reset_bits();
        let _blocks_size = packet.read_uint32().unwrap();
        assert_eq!(
            packet.read_uint8().unwrap(),
            0,
            "VALUES precedes the packed item GUID"
        );
        assert_eq!(packet.read_packed_guid().unwrap(), item_guid);
        assert_eq!(
            packet.read_uint32().unwrap(),
            13,
            "durability-only Item VALUES payload size"
        );
        assert_eq!(
            packet.read_uint32().unwrap(),
            1 << wow_entities::TYPEID_ITEM
        );
        assert_eq!(
            packet.read_bits(2).unwrap(),
            1,
            "only ItemData block0 is represented"
        );
        assert_eq!(
            packet.read_bits(32).unwrap(),
            0x1001,
            "exact parent0 + Durability12 mask"
        );
        packet.reset_bits();
        assert_eq!(
            packet.read_uint32().unwrap(),
            19,
            "the coalesced latest durability is published"
        );
        if include_player_delta {
            assert_eq!(packet.read_uint8().unwrap(), 0);
            assert_eq!(
                packet.read_packed_guid().unwrap(),
                owner_guid,
                "Player follows Item in the same envelope"
            );
        }

        assert_eq!(peer.send_represented_player_unit_values_updates_from_last_map_send_object_updates_like_cpp(), usize::from(include_player_delta));
        if include_player_delta {
            let peer_bytes = peer_output.try_recv().unwrap();
            let mut packet = wow_packet::WorldPacket::from_bytes(&peer_bytes);
            packet.read_uint16().unwrap();
            assert_eq!(
                packet.read_uint32().unwrap(),
                1,
                "peers receive only the filtered Player block"
            );
            packet.read_uint16().unwrap();
            assert!(!packet.read_bit().unwrap());
            packet.reset_bits();
            packet.read_uint32().unwrap();
            assert_eq!(packet.read_uint8().unwrap(), 0);
            assert_eq!(packet.read_packed_guid().unwrap(), owner_guid);
        }
        assert!(
            peer_output.try_recv().is_err(),
            "no Item block can reach the peer"
        );
        assert_eq!(owner.send_represented_player_unit_values_updates_from_last_map_send_object_updates_like_cpp(), 0);
        owner
            .with_owned_player_like_cpp(|player| {
                let item = &player.inventory_runtime_like_cpp().item_objects()[&item_guid];
                assert_eq!(item.update_state(), ItemUpdateState::Changed);
                assert!(
                    item.item_data_changes_mask()
                        .is_set(wow_entities::ITEM_DATA_STACK_COUNT_BIT)
                );
            })
            .unwrap();
        canonical.lock().unwrap().update(1);
        assert_eq!(owner.send_represented_player_unit_values_updates_from_last_map_send_object_updates_like_cpp(), 0);
    }
}

#[test]
fn native_durability_delivery_rechecks_current_item_and_owner_handle() {
    for invalidation in 0..4 {
        let (mut session, _, output) = make_session();
        let canonical = shared_canonical_map_manager();
        let owner_guid = ObjectGuid::create_player(1, 50_713);
        let item_guid = ObjectGuid::create_item(1, 50_714);
        configure_dynamic_object_values_snapshot_session_like_cpp(
            &mut session,
            &canonical,
            owner_guid,
            571,
            0,
        );
        assert!(session.adopt_registered_canonical_player_fixture_like_cpp());
        session
            .with_owned_player_mut_like_cpp(|player| {
                let mut item = wow_entities::Item::default();
                item.object_mut().create(item_guid);
                item.set_owner_guid(owner_guid);
                item.set_max_durability(60);
                item.set_durability(34);
                player
                    .inventory_runtime_mut_like_cpp()
                    .store_item_object_like_cpp(item);
                player.clear_data_changes();
            })
            .unwrap();
        session
            .core
            .owned_inventory_access_like_cpp()
            .with_inventory_runtime_mut_like_cpp(|runtime| {
                assert!(runtime.apply_item_object_updates_like_cpp(
                    item_guid,
                    &[ItemObjectUpdateLikeCpp::QueueDurabilityUpdate]
                ));
            })
            .unwrap();
        if invalidation == 3 {
            session
                .core
                .owned_inventory_access_like_cpp()
                .with_inventory_runtime_mut_like_cpp(|runtime| {
                    let replacement = runtime.item_objects()[&item_guid].clone();
                    runtime.store_item_object_like_cpp(replacement);
                })
                .unwrap();
        }
        canonical.lock().unwrap().update(1);
        if invalidation == 0 {
            session
                .core
                .owned_inventory_access_like_cpp()
                .with_inventory_runtime_mut_like_cpp(|runtime| {
                    runtime.remove_item_object_like_cpp(item_guid);
                })
                .unwrap();
        } else if invalidation == 1 {
            session
                .core
                .owned_inventory_access_like_cpp()
                .with_inventory_runtime_mut_like_cpp(|runtime| {
                    runtime
                        .item_objects_mut()
                        .get_mut(&item_guid)
                        .unwrap()
                        .set_owner_guid(ObjectGuid::create_player(1, 50_715));
                })
                .unwrap();
        } else if invalidation == 2 {
            // A source handle cannot authorize a different current Player GUID.
            session.core.player_guid = Some(ObjectGuid::create_player(1, 50_715));
        }
        assert_eq!(session.send_represented_player_unit_values_updates_from_last_map_send_object_updates_like_cpp(), 0);
        assert!(output.try_recv().is_err());
    }
}
