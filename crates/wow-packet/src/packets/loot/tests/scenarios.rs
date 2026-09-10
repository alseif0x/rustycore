//! Loot packet regressions.
//!
//! Moved out of loot.rs under #685; every test is unchanged.

use super::*;

#[test]
fn loot_money_reads_cpp_soft_interact_bit() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bit(true);
    pkt.flush_bits();
    pkt.reset_read();

    let parsed = LootMoney::read(&mut pkt).expect("loot money packet should parse");
    assert!(parsed.is_soft_interact);
}

#[test]
fn loot_roll_reads_cpp_loot_obj_list_id_and_vote() {
    let loot_obj = wow_core::ObjectGuid::create_item(1, 42);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_packed_guid(&loot_obj);
    pkt.write_uint8(7);
    pkt.write_uint8(2);
    pkt.reset_read();

    let parsed = LootRoll::read(&mut pkt).expect("loot roll packet should parse");
    assert_eq!(parsed.loot_obj, loot_obj);
    assert_eq!(parsed.loot_list_id, 7);
    assert_eq!(parsed.roll_type, 2);
}

#[test]
fn master_loot_item_reads_cpp_count_target_then_requests() {
    let target = wow_core::ObjectGuid::create_player(1, 77);
    let first = wow_core::ObjectGuid::create_item(1, 42);
    let second = wow_core::ObjectGuid::create_item(1, 43);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint32(2);
    pkt.write_packed_guid(&target);
    pkt.write_packed_guid(&first);
    pkt.write_uint8(3);
    pkt.write_packed_guid(&second);
    pkt.write_uint8(4);
    pkt.reset_read();

    let parsed = MasterLootItem::read(&mut pkt).expect("master loot packet should parse");
    assert_eq!(parsed.target, target);
    assert_eq!(parsed.loot.len(), 2);
    assert_eq!(parsed.loot[0].object, first);
    assert_eq!(parsed.loot[0].loot_list_id, 3);
    assert_eq!(parsed.loot[1].object, second);
    assert_eq!(parsed.loot[1].loot_list_id, 4);
}

#[test]
fn set_loot_specialization_reads_cpp_spec_id() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint32(65);
    pkt.reset_read();

    let parsed =
        SetLootSpecialization::read(&mut pkt).expect("set loot specialization should parse");
    assert_eq!(parsed.spec_id, 65);
}

#[test]
fn loot_money_notify_writes_cpp_money_money_mod_and_sole_looter() {
    let notify = LootMoneyNotify {
        money: 123,
        money_mod: 7,
        sole_looter: true,
    };
    let mut pkt = WorldPacket::new_empty();
    notify.write(&mut pkt);
    pkt.reset_read();

    assert_eq!(pkt.read_uint64().unwrap(), 123);
    assert_eq!(pkt.read_uint64().unwrap(), 7);
    assert!(pkt.read_bit().unwrap());
}

#[test]
fn loot_response_writes_cpp_owner_then_loot_obj() {
    let owner = wow_core::ObjectGuid::create_player(1, 7);
    let loot_obj = wow_core::ObjectGuid::create_item(1, 42);
    let response = LootResponse {
        owner,
        loot_obj,
        failure_reason: LOOT_RESPONSE_DEFAULT_FAILURE_REASON_LIKE_CPP,
        acquire_reason: 0,
        loot_method: 0,
        threshold: LOOT_RESPONSE_DEFAULT_THRESHOLD_LIKE_CPP,
        coins: 0,
        items: Vec::new(),
        currencies: Vec::new(),
        acquired: true,
        ae_looting: false,
    };
    let mut pkt = WorldPacket::new_empty();
    response.write(&mut pkt);
    pkt.reset_read();

    assert_eq!(pkt.read_packed_guid().unwrap(), owner);
    assert_eq!(pkt.read_packed_guid().unwrap(), loot_obj);
}

#[test]
fn loot_response_success_defaults_write_cpp_failure_reason_and_threshold() {
    let owner = wow_core::ObjectGuid::create_player(1, 7);
    let loot_obj = wow_core::ObjectGuid::create_item(1, 42);
    let response = LootResponse {
        owner,
        loot_obj,
        failure_reason: LOOT_RESPONSE_DEFAULT_FAILURE_REASON_LIKE_CPP,
        acquire_reason: 0,
        loot_method: 0,
        threshold: LOOT_RESPONSE_DEFAULT_THRESHOLD_LIKE_CPP,
        coins: 0,
        items: Vec::new(),
        currencies: Vec::new(),
        acquired: true,
        ae_looting: false,
    };
    let mut pkt = WorldPacket::new_empty();
    response.write(&mut pkt);
    pkt.reset_read();

    assert_eq!(pkt.read_packed_guid().unwrap(), owner);
    assert_eq!(pkt.read_packed_guid().unwrap(), loot_obj);
    assert_eq!(
        pkt.read_uint8().unwrap(),
        LOOT_RESPONSE_DEFAULT_FAILURE_REASON_LIKE_CPP
    );
    assert_eq!(pkt.read_uint8().unwrap(), 0);
    assert_eq!(pkt.read_uint8().unwrap(), 0);
    assert_eq!(
        pkt.read_uint8().unwrap(),
        LOOT_RESPONSE_DEFAULT_THRESHOLD_LIKE_CPP
    );
}

#[test]
fn loot_currency_data_writes_cpp_shape() {
    let currency = LootCurrencyData {
        currency_id: 395,
        quantity: 7,
        loot_list_id: 3,
        ui_type: 5,
    };
    let mut pkt = WorldPacket::new_empty();
    currency.write(&mut pkt);
    pkt.reset_read();

    assert_eq!(pkt.read_uint32().unwrap(), 395);
    assert_eq!(pkt.read_uint32().unwrap(), 7);
    assert_eq!(pkt.read_uint8().unwrap(), 3);
    assert_eq!(pkt.read_bits(3).unwrap(), 5);
}

#[test]
fn loot_response_writes_cpp_currency_count_and_entries_after_items() {
    let owner = wow_core::ObjectGuid::create_player(1, 7);
    let loot_obj = wow_core::ObjectGuid::create_item(1, 42);
    let response = LootResponse {
        owner,
        loot_obj,
        failure_reason: LOOT_RESPONSE_DEFAULT_FAILURE_REASON_LIKE_CPP,
        acquire_reason: 0,
        loot_method: 0,
        threshold: LOOT_RESPONSE_DEFAULT_THRESHOLD_LIKE_CPP,
        coins: 11,
        items: Vec::new(),
        currencies: vec![LootCurrencyData {
            currency_id: 395,
            quantity: 7,
            loot_list_id: 3,
            ui_type: 5,
        }],
        acquired: true,
        ae_looting: false,
    };
    let mut pkt = WorldPacket::new_empty();
    response.write(&mut pkt);
    pkt.reset_read();

    assert_eq!(pkt.read_packed_guid().unwrap(), owner);
    assert_eq!(pkt.read_packed_guid().unwrap(), loot_obj);
    assert_eq!(
        pkt.read_uint8().unwrap(),
        LOOT_RESPONSE_DEFAULT_FAILURE_REASON_LIKE_CPP
    );
    assert_eq!(pkt.read_uint8().unwrap(), 0);
    assert_eq!(pkt.read_uint8().unwrap(), 0);
    assert_eq!(
        pkt.read_uint8().unwrap(),
        LOOT_RESPONSE_DEFAULT_THRESHOLD_LIKE_CPP
    );
    assert_eq!(pkt.read_uint32().unwrap(), 11);
    assert_eq!(pkt.read_uint32().unwrap(), 0);
    assert_eq!(pkt.read_uint32().unwrap(), 1);
    assert!(pkt.read_bit().unwrap());
    assert!(!pkt.read_bit().unwrap());
    pkt.reset_bits();
    assert_eq!(pkt.read_uint32().unwrap(), 395);
    assert_eq!(pkt.read_uint32().unwrap(), 7);
    assert_eq!(pkt.read_uint8().unwrap(), 3);
    assert_eq!(pkt.read_bits(3).unwrap(), 5);
}

#[test]
fn loot_item_data_writes_cpp_shape() {
    let item = LootItemData {
        item_type: 0,
        ui_type: 4,
        can_trade_to_tap_list: false,
        loot: ItemInstance {
            item_id: 25,
            ..ItemInstance::default()
        },
        loot_list_id: 7,
        quantity: 2,
        loot_item_type: 0,
    };
    let mut pkt = WorldPacket::new_empty();
    item.write(&mut pkt);
    pkt.reset_read();

    assert_eq!(pkt.read_bits(2).unwrap(), 0);
    assert_eq!(pkt.read_bits(3).unwrap(), 4);
    assert!(!pkt.read_bit().unwrap());
    assert_eq!(pkt.read_int32().unwrap(), 25);
    assert_eq!(pkt.read_int32().unwrap(), 0);
    assert_eq!(pkt.read_int32().unwrap(), 0);
    assert!(!pkt.read_bit().unwrap());
    pkt.reset_bits();
    assert_eq!(pkt.read_bits(6).unwrap(), 0);
    assert_eq!(pkt.read_uint32().unwrap(), 2);
    assert_eq!(pkt.read_uint8().unwrap(), 0);
    assert_eq!(pkt.read_uint8().unwrap(), 7);
}

#[test]
fn loot_removed_writes_cpp_owner_then_loot_obj() {
    let owner = wow_core::ObjectGuid::create_player(1, 7);
    let loot_obj = wow_core::ObjectGuid::create_item(1, 42);
    let removed = LootRemoved {
        owner,
        loot_obj,
        loot_list_id: 3,
    };
    let mut pkt = WorldPacket::new_empty();
    removed.write(&mut pkt);
    pkt.reset_read();

    assert_eq!(pkt.read_packed_guid().unwrap(), owner);
    assert_eq!(pkt.read_packed_guid().unwrap(), loot_obj);
    assert_eq!(pkt.read_uint8().unwrap(), 3);
}

#[test]
fn loot_list_writes_cpp_owner_loot_obj_bits_and_optional_guids() {
    let owner = wow_core::ObjectGuid::create_player(1, 7);
    let loot_obj = wow_core::ObjectGuid::create_item(1, 42);
    let master = wow_core::ObjectGuid::create_player(1, 77);
    let round_robin_winner = wow_core::ObjectGuid::create_player(1, 78);
    let list = LootList {
        owner,
        loot_obj,
        master: Some(master),
        round_robin_winner: Some(round_robin_winner),
    };
    let mut pkt = WorldPacket::new_empty();
    list.write(&mut pkt);
    pkt.reset_read();

    assert_eq!(pkt.read_packed_guid().unwrap(), owner);
    assert_eq!(pkt.read_packed_guid().unwrap(), loot_obj);
    assert!(pkt.read_bit().unwrap());
    assert!(pkt.read_bit().unwrap());
    pkt.reset_bits();
    assert_eq!(pkt.read_packed_guid().unwrap(), master);
    assert_eq!(pkt.read_packed_guid().unwrap(), round_robin_winner);
}

#[test]
fn loot_list_writes_cpp_absent_optional_bits_without_guids() {
    let owner = wow_core::ObjectGuid::create_player(1, 7);
    let loot_obj = wow_core::ObjectGuid::create_item(1, 42);
    let list = LootList {
        owner,
        loot_obj,
        master: None,
        round_robin_winner: None,
    };
    let mut pkt = WorldPacket::new_empty();
    list.write(&mut pkt);
    pkt.reset_read();

    assert_eq!(pkt.read_packed_guid().unwrap(), owner);
    assert_eq!(pkt.read_packed_guid().unwrap(), loot_obj);
    assert!(!pkt.read_bit().unwrap());
    assert!(!pkt.read_bit().unwrap());
    pkt.reset_bits();
    assert!(pkt.read_uint8().is_err());
}

#[test]
fn coin_removed_writes_cpp_loot_obj() {
    let loot_obj = wow_core::ObjectGuid::create_item(1, 42);
    let notify = CoinRemoved { loot_obj };
    let mut pkt = WorldPacket::new_empty();
    notify.write(&mut pkt);
    pkt.reset_read();

    assert_eq!(pkt.read_packed_guid().unwrap(), loot_obj);
}

#[test]
fn ae_loot_targets_writes_cpp_count_only() {
    let targets = AELootTargets { count: 3 };
    let mut pkt = WorldPacket::new_empty();
    targets.write(&mut pkt);
    pkt.reset_read();

    assert_eq!(pkt.read_uint32().unwrap(), 3);
    assert!(pkt.read_uint8().is_err());
}

#[test]
fn ae_loot_targets_ack_writes_cpp_empty_payload() {
    let ack = AELootTargetsAck;
    let mut pkt = WorldPacket::new_empty();
    ack.write(&mut pkt);
    pkt.reset_read();

    assert!(pkt.read_uint8().is_err());
}

#[test]
fn start_loot_roll_writes_cpp_order() {
    let loot_obj = wow_core::ObjectGuid::create_item(1, 42);
    let roll = StartLootRoll {
        loot_obj,
        map_id: 571,
        roll_time_ms: 60_000,
        method: 3,
        valid_rolls: 0x07,
        loot_roll_ineligible_reason: [1, 2, 3, 4],
        item: roll_test_item(),
        dungeon_encounter_id: 99,
    };
    let mut pkt = WorldPacket::new_empty();
    roll.write(&mut pkt);
    pkt.reset_read();

    assert_eq!(pkt.read_packed_guid().unwrap(), loot_obj);
    assert_eq!(pkt.read_int32().unwrap(), 571);
    assert_eq!(pkt.read_uint32().unwrap(), 60_000);
    assert_eq!(pkt.read_uint8().unwrap(), 0x07);
    assert_eq!(pkt.read_uint32().unwrap(), 1);
    assert_eq!(pkt.read_uint32().unwrap(), 2);
    assert_eq!(pkt.read_uint32().unwrap(), 3);
    assert_eq!(pkt.read_uint32().unwrap(), 4);
    assert_eq!(pkt.read_uint8().unwrap(), 3);
    assert_eq!(pkt.read_int32().unwrap(), 99);
    assert_eq!(pkt.read_bits(2).unwrap(), 0);
    assert_eq!(pkt.read_bits(3).unwrap(), 5);
}

#[test]
fn loot_roll_broadcast_writes_cpp_order_and_bits() {
    let loot_obj = wow_core::ObjectGuid::create_item(1, 42);
    let player = wow_core::ObjectGuid::create_player(1, 77);
    let broadcast = LootRollBroadcast {
        loot_obj,
        player,
        roll: -42,
        roll_type: 2,
        item: roll_test_item(),
        autopassed: true,
        off_spec: false,
        dungeon_encounter_id: 99,
    };
    let mut pkt = WorldPacket::new_empty();
    broadcast.write(&mut pkt);
    pkt.reset_read();

    assert_eq!(pkt.read_packed_guid().unwrap(), loot_obj);
    assert_eq!(pkt.read_packed_guid().unwrap(), player);
    assert_eq!(pkt.read_int32().unwrap(), -42);
    assert_eq!(pkt.read_uint8().unwrap(), 2);
    assert_eq!(pkt.read_int32().unwrap(), 99);
}

#[test]
fn loot_roll_won_writes_cpp_order() {
    let loot_obj = wow_core::ObjectGuid::create_item(1, 42);
    let winner = wow_core::ObjectGuid::create_player(1, 77);
    let won = LootRollWon {
        loot_obj,
        winner,
        roll: 98,
        roll_type: 2,
        item: roll_test_item(),
        main_spec: true,
        dungeon_encounter_id: 99,
    };
    let mut pkt = WorldPacket::new_empty();
    won.write(&mut pkt);
    pkt.reset_read();

    assert_eq!(pkt.read_packed_guid().unwrap(), loot_obj);
    assert_eq!(pkt.read_packed_guid().unwrap(), winner);
    assert_eq!(pkt.read_int32().unwrap(), 98);
    assert_eq!(pkt.read_uint8().unwrap(), 2);
    assert_eq!(pkt.read_int32().unwrap(), 99);
}

#[test]
fn loot_all_passed_writes_cpp_order() {
    let loot_obj = wow_core::ObjectGuid::create_item(1, 42);
    let passed = LootAllPassed {
        loot_obj,
        item: roll_test_item(),
        dungeon_encounter_id: 99,
    };
    let mut pkt = WorldPacket::new_empty();
    passed.write(&mut pkt);
    pkt.reset_read();

    assert_eq!(pkt.read_packed_guid().unwrap(), loot_obj);
    assert_eq!(pkt.read_int32().unwrap(), 99);
    assert_eq!(pkt.read_bits(2).unwrap(), 0);
    assert_eq!(pkt.read_bits(3).unwrap(), 5);
}

#[test]
fn loot_rolls_complete_writes_cpp_order() {
    let loot_obj = wow_core::ObjectGuid::create_item(1, 42);
    let complete = LootRollsComplete {
        loot_obj,
        loot_list_id: 7,
        dungeon_encounter_id: 99,
    };
    let mut pkt = WorldPacket::new_empty();
    complete.write(&mut pkt);
    pkt.reset_read();

    assert_eq!(pkt.read_packed_guid().unwrap(), loot_obj);
    assert_eq!(pkt.read_uint8().unwrap(), 7);
    assert_eq!(pkt.read_int32().unwrap(), 99);
}

#[test]
fn master_loot_candidate_list_writes_cpp_order() {
    let loot_obj = wow_core::ObjectGuid::create_item(1, 42);
    let first = wow_core::ObjectGuid::create_player(1, 77);
    let second = wow_core::ObjectGuid::create_player(1, 78);
    let list = MasterLootCandidateList {
        loot_obj,
        players: vec![first, second],
    };
    let mut pkt = WorldPacket::new_empty();
    list.write(&mut pkt);
    pkt.reset_read();

    assert_eq!(pkt.read_packed_guid().unwrap(), loot_obj);
    assert_eq!(pkt.read_uint32().unwrap(), 2);
    assert_eq!(pkt.read_packed_guid().unwrap(), first);
    assert_eq!(pkt.read_packed_guid().unwrap(), second);
}

#[test]
fn loot_release_writes_cpp_loot_obj_then_owner() {
    let loot_obj = wow_core::ObjectGuid::create_item(1, 42);
    let owner = wow_core::ObjectGuid::create_player(1, 7);
    let release = SLootRelease { loot_obj, owner };
    let mut pkt = WorldPacket::new_empty();
    release.write(&mut pkt);
    pkt.reset_read();

    assert_eq!(pkt.read_packed_guid().unwrap(), loot_obj);
    assert_eq!(pkt.read_packed_guid().unwrap(), owner);
}

#[test]
fn loot_release_all_writes_cpp_empty_payload() {
    let release = LootReleaseAll;
    let mut pkt = WorldPacket::new_empty();
    release.write(&mut pkt);
    pkt.reset_read();

    assert!(pkt.read_uint8().is_err());
}
