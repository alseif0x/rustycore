// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use super::*;

#[test]
fn inventory_position_publication_deduplicates_and_sends_bag_then_player_then_stats() {
    let (mut session, _, send_rx) = make_session();
    let owner = ObjectGuid::create_player(1, 30_210);
    session.set_player_guid(Some(owner));
    session.set_loaded_player_identity_like_cpp(571, 1, 5, 80, 0);
    session.set_player_stats(Arc::new(wow_data::PlayerStatsStore::from_entries([(
        (1, 5, 80),
        wow_data::PlayerLevelStats {
            strength: 10,
            agility: 10,
            stamina: 10,
            intellect: 40,
            spirit: 30,
            base_mana: 1_000,
        },
    )])));
    session.set_chr_classes_store(Arc::new(
        wow_data::character_progression::ChrClassesStore::from_entries([{
            let mut entry = wow_data::character_progression::ChrClassesEntry::default();
            entry.id = 5;
            entry
        }]),
    ));
    crate::canonical_player_access::install_canonical_player_owner_for_test(&mut session, 571, 0);
    session.set_loaded_player_identity_like_cpp(571, 1, 5, 80, 0);
    session.set_spell_store(Arc::new(wow_data::SpellStore::new()));
    install_remove_spell_offhand_templates_like_cpp(
        &mut session,
        &[
            (30_210, InventoryType::Bag, 0, ItemClass::Container, 0),
            (30_211, InventoryType::Chest, 0, ItemClass::Armor, 0),
        ],
    );
    let bag = ObjectGuid::create_item(1, 30_210);
    let chest = ObjectGuid::create_item(1, 30_211);
    let child = ObjectGuid::create_item(1, 30_212);
    equip_represented_test_item_like_cpp(
        &mut session,
        INVENTORY_SLOT_BAG_START,
        bag,
        30_210,
        InventoryType::Bag,
    );
    equip_represented_test_item_like_cpp(
        &mut session,
        EQUIPMENT_SLOT_CHEST,
        chest,
        30_211,
        InventoryType::Chest,
    );
    let _ = session.mutate_canonical_player_like_cpp(|player| {
        player
            .register_bag_storage(INVENTORY_SLOT_BAG_START, bag, 4)
            .unwrap();
        player
            .store_bag_item(INVENTORY_SLOT_BAG_START, 1, child)
            .unwrap();
    });
    let _ = drain_server_packet_bytes(&send_rx);
    session.publish_inventory_position_changes_like_cpp(&[
        (INVENTORY_SLOT_BAG_START, 1),
        (INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_CHEST),
        (INVENTORY_SLOT_BAG_START, 1),
        (INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_CHEST),
    ]);
    let packets = drain_server_packet_bytes(&send_rx);
    assert_eq!(packets.len(), 3);
    for bytes in &packets {
        assert_eq!(
            WorldPacket::from_bytes(bytes).server_opcode(),
            Some(ServerOpcodes::UpdateObject)
        );
        assert_eq!(u16::from_le_bytes([bytes[6], bytes[7]]), 571);
    }
    let mut mask = wow_entities::UpdateMask::new(wow_entities::CONTAINER_DATA_BITS);
    mask.set(wow_entities::CONTAINER_DATA_SLOTS_PARENT_BIT);
    mask.set(wow_entities::CONTAINER_DATA_SLOTS_FIRST_BIT + 1);
    let mut slots = [ObjectGuid::EMPTY; wow_entities::MAX_BAG_SIZE];
    slots[1] = child;
    let update = wow_entities::BagValuesUpdate {
        changed_object_type_mask: 1 << wow_entities::TYPEID_CONTAINER,
        object_data: None,
        item_data: None,
        container_data: Some(wow_entities::ContainerDataUpdate {
            mask,
            values: wow_entities::ContainerDataValues {
                num_slots: 4,
                slots,
            },
        }),
    };
    assert_eq!(
        packets[0],
        wow_world_core::entity_update_bridge::bag_values_update_to_update_object(bag, 571, &update)
            .unwrap()
            .to_bytes()
    );
    assert!(session.send_stat_update());
    assert_eq!(
        drain_server_packet_bytes(&send_rx),
        vec![packets[2].clone()]
    );
    assert!(session.send_player_values_update_from_entity_bridge(
        &[(EQUIPMENT_SLOT_CHEST, chest)],
        &[(EQUIPMENT_SLOT_CHEST, 30_211, 0, 0)],
        &[],
        &[],
        None,
    ));
    assert_eq!(
        drain_server_packet_bytes(&send_rx),
        vec![packets[1].clone()]
    );
}

#[test]
fn inventory_position_publication_without_gear_uses_current_map_and_no_stats() {
    let (mut session, _, send_rx) = make_session();
    session.set_player_guid(Some(ObjectGuid::create_player(1, 30_213)));
    let item = ObjectGuid::create_item(1, 30_213);
    equip_represented_test_item_like_cpp(
        &mut session,
        INVENTORY_SLOT_ITEM_START,
        item,
        30_213,
        InventoryType::NonEquip,
    );
    session.set_player_map_position_like_cpp(489, Position::ZERO);
    let _ = drain_server_packet_bytes(&send_rx);
    session.publish_inventory_position_changes_like_cpp(&[
        (INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START),
        (INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START),
    ]);
    let packets = drain_server_packet_bytes(&send_rx);
    assert_eq!(packets.len(), 1);
    assert_eq!(u16::from_le_bytes([packets[0][6], packets[0][7]]), 489);
    session.set_player_map_position_like_cpp(571, Position::ZERO);
    session.publish_inventory_position_changes_like_cpp(&[(
        INVENTORY_SLOT_BAG_0,
        INVENTORY_SLOT_ITEM_START,
    )]);
    let packets = drain_server_packet_bytes(&send_rx);
    assert_eq!(packets.len(), 1);
    assert_eq!(u16::from_le_bytes([packets[0][6], packets[0][7]]), 571);
    session.publish_inventory_position_changes_like_cpp(&[]);
    assert!(drain_server_packet_bytes(&send_rx).is_empty());
}

#[test]
fn inventory_position_publication_bag_slot_does_not_reconstruct_no_handle_fixture() {
    let (mut session, _, send_rx) = make_session();
    let owner = ObjectGuid::create_player(1, 30_214);
    let bag = ObjectGuid::create_item(1, 30_214);
    session.set_player_guid(Some(owner));
    install_remove_spell_offhand_templates_like_cpp(
        &mut session,
        &[(30_214, InventoryType::Bag, 0, ItemClass::Container, 0)],
    );
    equip_represented_test_item_like_cpp(
        &mut session,
        INVENTORY_SLOT_BAG_START,
        bag,
        30_214,
        InventoryType::Bag,
    );
    assert!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_BAG_START)
            .is_some()
    );
    let _ = drain_server_packet_bytes(&send_rx);
    session.publish_inventory_position_changes_like_cpp(&[
        (INVENTORY_SLOT_BAG_START, 1),
        (INVENTORY_SLOT_BAG_START, 1),
    ]);
    assert!(drain_server_packet_bytes(&send_rx).is_empty());
}
