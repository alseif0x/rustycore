//! Preserved loot response and publication scenarios.
use super::recovery_support::*;
use wow_constants::InventoryResult;
use wow_entities::INVENTORY_SLOT_BAG_0;
use wow_packet::packets::loot::*;
use wow_world::test_fixtures::loot::{
    loot_client_type_for_test, loot_fixture_response, loot_response_for_test,
    master_loot_inventory_error_for_test, notify_cached_loot_item_for_test,
    notify_committed_loot_item_for_test, open_loot_response_for_test, push_loot_item_for_test,
    send_loot_failure_for_test,
};

fn loot_response_threshold(sent: &[u8]) -> u8 {
    let mut pkt = WorldPacket::from_bytes(&sent[2..]);
    let _owner = pkt.read_packed_guid().unwrap();
    let _loot_obj = pkt.read_packed_guid().unwrap();
    let _failure = pkt.read_uint8().unwrap();
    let _acquire = pkt.read_uint8().unwrap();
    pkt.read_uint8().unwrap();
    pkt.read_uint8().unwrap()
}

#[test]
fn master_loot_inventory_result_mapping_matches_cpp_errors() {
    assert_eq!(
        master_loot_inventory_error_for_test(InventoryResult::Ok),
        None
    );
    assert_eq!(
        master_loot_inventory_error_for_test(InventoryResult::ItemMaxCount),
        Some(LOOT_ERROR_MASTER_UNIQUE_ITEM_LIKE_CPP)
    );
    assert_eq!(
        master_loot_inventory_error_for_test(InventoryResult::InvFull),
        Some(wow_packet::packets::loot::LOOT_ERROR_MASTER_INV_FULL_LIKE_CPP)
    );
    assert_eq!(
        master_loot_inventory_error_for_test(InventoryResult::CantEquipEver),
        Some(LOOT_ERROR_MASTER_OTHER_LIKE_CPP)
    );
}

#[tokio::test]
async fn represented_loot_response_acquire_reason_uses_cpp_loot_type_mapping() {
    let mut session = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let owner_guid = test_creature_guid(19_096);
    let loot_guid = represented_loot_object_guid_like_cpp(owner_guid);
    let entry = represented_loot_entry(0, 25, player_guid);
    register_test_creature_like_cpp(&mut session, test_creature(owner_guid, false));
    set_loot_for_test(
        &mut session,
        owner_guid,
        CreatureLoot {
            loot_guid,
            coins: 0,
            unlooted_count: 1,
            loot_type: LOOT_TYPE_PROSPECTING_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: vec![player_guid],
            items: vec![entry],
            looted_by_player: false,
        },
    );
    install_cached_test_creature_loot_authority_like_cpp(&mut session, owner_guid, player_guid);

    let response = loot_response_for_test(&mut session, owner_guid, player_guid, false)
        .await
        .unwrap();

    assert_eq!(response.acquire_reason, LOOT_TYPE_DISENCHANTING_LIKE_CPP);
}

#[test]
fn represented_loot_type_for_client_matches_cpp_aliases() {
    assert_eq!(
        loot_client_type_for_test(LOOT_TYPE_NONE_LIKE_CPP),
        LOOT_TYPE_NONE_LIKE_CPP
    );
    assert_eq!(
        loot_client_type_for_test(LOOT_TYPE_CORPSE_LIKE_CPP),
        LOOT_TYPE_CORPSE_LIKE_CPP
    );
    assert_eq!(
        loot_client_type_for_test(LOOT_TYPE_ITEM_LIKE_CPP),
        LOOT_TYPE_ITEM_LIKE_CPP
    );
    assert_eq!(
        loot_client_type_for_test(LOOT_TYPE_GATHERING_NODE_LIKE_CPP),
        LOOT_TYPE_GATHERING_NODE_LIKE_CPP
    );
    assert_eq!(
        loot_client_type_for_test(LOOT_TYPE_CHEST_LIKE_CPP),
        LOOT_TYPE_CHEST_LIKE_CPP
    );
    assert_eq!(
        loot_client_type_for_test(LOOT_TYPE_CORPSE_PERSONAL_LIKE_CPP),
        LOOT_TYPE_CORPSE_PERSONAL_LIKE_CPP
    );
    assert_eq!(
        loot_client_type_for_test(LOOT_TYPE_PROSPECTING_LIKE_CPP),
        LOOT_TYPE_DISENCHANTING_LIKE_CPP
    );
    assert_eq!(
        loot_client_type_for_test(LOOT_TYPE_MILLING_LIKE_CPP),
        LOOT_TYPE_DISENCHANTING_LIKE_CPP
    );
    assert_eq!(
        loot_client_type_for_test(LOOT_TYPE_INSIGNIA_LIKE_CPP),
        LOOT_TYPE_SKINNING_LIKE_CPP
    );
    assert_eq!(
        loot_client_type_for_test(LOOT_TYPE_FISHINGHOLE_LIKE_CPP),
        LOOT_TYPE_FISHING_LIKE_CPP
    );
    assert_eq!(
        loot_client_type_for_test(LOOT_TYPE_FISHING_JUNK_LIKE_CPP),
        LOOT_TYPE_FISHING_LIKE_CPP
    );
}

#[tokio::test]
async fn loot_error_response_keeps_cpp_threshold_default_like_cpp() {
    let (session, send_rx) = make_session_with_send();
    let owner = test_creature_guid(19_119);
    let loot_obj = represented_loot_object_guid_like_cpp(owner);

    send_loot_failure_for_test(&session, loot_obj, owner, LOOT_ERROR_TOO_FAR_LIKE_CPP);

    let sent = send_rx.try_recv().unwrap();
    assert_eq!(
        loot_response_failure_reason(&sent),
        LOOT_ERROR_TOO_FAR_LIKE_CPP
    );
    assert_eq!(
        loot_response_threshold(&sent),
        LOOT_RESPONSE_DEFAULT_THRESHOLD_LIKE_CPP
    );
}

#[tokio::test]
async fn loot_response_success_keeps_cpp_failure_and_threshold_defaults() {
    let mut session = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let creature_guid = test_creature_guid(19_118);
    session.set_player_guid(Some(player_guid));
    register_test_creature_like_cpp(&mut session, test_creature(creature_guid, false));

    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(player_guid);
    group.loot_method = LOOT_METHOD_GROUP_LIKE_CPP;
    group.loot_threshold = 4;
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    set_group_guid_for_test_like_cpp(&mut session, Some(group_guid));
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));

    set_loot_for_test(
        &mut session,
        creature_guid,
        CreatureLoot {
            loot_guid: represented_loot_object_guid_like_cpp(creature_guid),
            coins: 1,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: LOOT_METHOD_GROUP_LIKE_CPP,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: player_guid,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: vec![player_guid],
            items: Vec::new(),
            looted_by_player: false,
        },
    );
    install_cached_test_creature_loot_authority_like_cpp(&mut session, creature_guid, player_guid);

    let response = loot_response_for_test(&mut session, creature_guid, player_guid, false)
        .await
        .unwrap();

    assert_eq!(response.loot_method, LOOT_METHOD_GROUP_LIKE_CPP);
    assert_eq!(
        response.failure_reason,
        LOOT_RESPONSE_DEFAULT_FAILURE_REASON_LIKE_CPP
    );
    assert_eq!(response.threshold, LOOT_RESPONSE_DEFAULT_THRESHOLD_LIKE_CPP);
}

#[test]
fn represented_start_loot_roll_carries_cpp_dungeon_encounter_id() {
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_obj = ObjectGuid::create_world_object(HighGuid::LootObject, 0, 1, 0, 0, 1, 900);
    let entry = represented_loot_entry(0, 25, player_guid);

    let packet = wow_world::test_fixtures::loot::start_loot_roll_for_test(
        loot_obj,
        571,
        LOOT_METHOD_GROUP_LIKE_CPP,
        &entry,
        ROLL_ALL_TYPE_NO_DISENCHANT_LIKE_CPP,
        615,
    );

    assert_eq!(packet.dungeon_encounter_id, 615);
}
