//! Money fixture exports and the original packet/cache setup adapters.

pub(super) use super::recovery_support::*;
pub(super) use super::money_port::prepare_personal_money_fixture;
pub(super) use std::collections::HashMap;
pub(super) use std::sync::{Mutex, atomic::AtomicU64};
pub(super) use wow_entities::MAX_MONEY_AMOUNT;
pub(super) use wow_loot::{LOOT_METHOD_GROUP_LIKE_CPP, LootClaimPayload, OwnedLootAuthority};
pub(super) use wow_packet::packets::loot::LOOT_TYPE_PICKPOCKETING_LIKE_CPP;
pub(super) use wow_world::test_fixtures::loot::{
    LootMoneyApplication,
    consume_personal_money_loot_for_test,
    open_personal_money_loot_for_test,
    open_money_loot_normally_for_test,
    LootMoneyDelivery,
    LootMoneyFanout,
    LootMoneyTracker,
    LootMoneyWorker,
    LootMoneyWorkerError,
    accepted_money_delta_for_test,
    active_money_generation_for_test,
    apply_money_command_for_test,
    attach_money_player_controller_for_test,
    clear_money_persistence_outcome_for_test,
    ensure_money_player_map_for_test,
    generate_chest_money_loot_for_test,
    generate_creature_money_loot_for_test,
    money_instance_for_test,
    money_map_for_test,
    money_recipients_for_test,
    money_tracker_for_test,
    notify_money_removed_for_test,
    set_group_money_port_for_test,
    source_money_delivery_for_test,
    spawn_group_money_worker_for_test,
    sync_creature_loot_fixture_for_test,
    prepare_money_player_residence_for_test,
    allow_money_looter_for_test,
};
pub(super) use wow_world::test_fixtures::loot::two_sessions_with_money_loot_for_test
    as two_sessions_with_authoritative_creature_loot_like_cpp;

pub(super) fn attach_loot_guid_allocator_for_owner(
    session: &mut WorldSession,
    owner_guid: ObjectGuid,
) {
    let kind = if owner_guid.is_game_object() {
        AccessorObjectKind::GameObject
    } else {
        AccessorObjectKind::Creature
    };
    attach_canonical_map_object(
        session,
        kind,
        canonical_world_object(owner_guid, u32::from(owner_guid.map_id()), Position::ZERO),
    );
}

pub(super) fn recv_packet_with_opcode(
    rx: &flume::Receiver<Vec<u8>>,
    opcode: wow_constants::ServerOpcodes,
) -> WorldPacket {
    for _ in 0..8 {
        let sent = rx.try_recv().unwrap();
        let mut packet = WorldPacket::from_bytes(&sent);
        if packet.read_uint16().unwrap() == opcode as u16 {
            return packet;
        }
    }
    panic!("expected packet opcode {:?}", opcode);
}
