//! Shared imports for application-level loot integration scenarios.

pub(crate) use std::sync::Arc;
pub(crate) use std::time::{Duration, Instant};

pub(crate) use wow_constants::{ItemFlags2, UnitDynFlags};
pub(crate) use wow_core::{ObjectGuid, Position, guid::HighGuid};
pub(crate) use wow_data::quest::{QuestObjective, QuestStore};
pub(crate) use wow_data::{ChrSpecializationEntry, ChrSpecializationStore};
pub(crate) use wow_entities::{CORPSE_DYNFLAG_LOOTABLE, CreatureLoot, CreatureOwnedLoot};
pub(crate) use wow_entities::{GAMEOBJECT_TYPE_CHEST, GAMEOBJECT_TYPE_FISHING_HOLE};
pub(crate) use wow_entities::{GameObjectLootSource, GameObjectOwnedLoot, LootState};
pub(crate) use wow_entities::{LootEntry, LootEntryFlags};
pub(crate) use wow_packet::packets::loot::{
    LOOT_ERROR_NO_LOOT_LIKE_CPP, LOOT_ERROR_TOO_FAR_LIKE_CPP, LOOT_TYPE_CHEST_LIKE_CPP,
    LOOT_TYPE_CORPSE_LIKE_CPP, LOOT_TYPE_FISHINGHOLE_LIKE_CPP, LOOT_TYPE_INSIGNIA_LIKE_CPP,
    LOOT_TYPE_ITEM_LIKE_CPP, LOOT_TYPE_SKINNING_LIKE_CPP, LootRoll,
    ROLL_ALL_TYPE_NO_DISENCHANT_LIKE_CPP, ROLL_FLAG_TYPE_NEED_LIKE_CPP, ROLL_VOTE_PASS_LIKE_CPP,
    SetLootSpecialization,
};
pub(crate) use wow_packet::{ServerPacket, WorldPacket};
pub(crate) use wow_social::group::{GroupInfo, GroupRegistry, PendingInvites};
pub(crate) use wow_world::LootDropRatesLikeCpp;
pub(crate) use wow_world::WorldSession;
pub(crate) use wow_world::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP;
pub(crate) use wow_world::handlers::quest::PlayerQuestStatus;
pub(crate) use wow_world::session::directory::PlayerRegistry;
pub(crate) use wow_world::session::mailbox::SessionCommand;
pub(crate) use wow_world::test_fixtures::loot::AccessorObjectKind;
pub(crate) use wow_world::test_fixtures::loot::{
    ENCHANTING_SKILL_FOR_TEST, active_loot_guid_for_test, active_loot_view_owners_for_test,
    add_active_loot_view_owner_for_test, apply_world_creature_corpse_loot_flags_for_test,
    attach_canonical_corpse_for_loot_test, attach_canonical_creature_for_loot_test,
    attach_canonical_gameobject_for_loot_test, attach_canonical_map_object_for_loot_test,
    basic_quest_template_for_loot_test, canonical_corpse_snapshot_for_loot_test,
    canonical_creature_snapshot_for_loot_test, canonical_gameobject_snapshot_for_loot_test,
    canonical_world_object_for_loot_test, drain_server_opcodes_for_loot_test,
    gameobject_loot_release_snapshot_for_test, handle_loot_item_for_test,
    handle_loot_money_for_test, handle_loot_roll_for_test, handle_loot_unit_for_test,
    has_loot_for_test, insert_allowed_coin_loot_for_test,
    install_basic_item_template_for_loot_test,
    install_cached_test_creature_loot_authority_for_test, install_group_loot_group_for_test,
    install_limited_item_template_for_loot_test, install_master_loot_group_for_test,
    is_active_loot_guid_for_test, loot_for_test, loot_item_packet, loot_money_packet,
    loot_release_packet, loot_specialization_for_test, loot_unit_packet,
    make_canonical_corpse_for_loot_test, make_canonical_creature_for_loot_test,
    make_canonical_gameobject_for_loot_test, make_session, make_session_with_send,
    make_session_with_send_capacity, open_test_ae_pair_for_loot, personal_loot_marker_for_test,
    player_registration_for_loot_test, reconcile_loot_cache_for_test,
    record_fishing_hole_max_opens_for_loot_test,
    record_gameobject_chest_release_metadata_for_loot_test, record_gameobject_owner_for_loot_test,
    register_test_creature_for_loot, represented_loot_entry_for_test,
    represented_loot_object_guid_for_test, set_active_loot_guid_for_test, set_loot_for_test,
    set_personal_loot_for_loot_test, set_player_map_position_for_loot_test,
    set_player_position_for_loot_test, set_world_creature_corpse_deadline_for_test,
    set_world_creature_corpse_delay_for_test, set_world_creature_expired_corpse_for_test,
    set_world_creature_personal_loot_for_test, tap_test_creature_for_loot, test_corpse_guid,
    test_creature_for_loot, test_creature_guid, test_gameobject_guid,
    world_creature_corpse_deadline_for_test, world_creature_corpse_delay_for_test,
    world_creature_corpse_despawn_at_for_test, world_creature_corpse_despawn_due_for_test,
    world_creature_has_lootable_dynamic_flag_for_test,
};
pub(crate) use wow_world::test_fixtures::record_represented_gameobject_runtime_state_for_test;
pub(crate) use wow_world::test_fixtures::{
    contains_player_quest_status_for_test, contains_rewarded_quest_for_test,
    group_guid_for_test_like_cpp, insert_client_visible_guid_for_test,
    insert_player_quest_status_for_test, set_group_guid_for_test_like_cpp,
    set_loaded_player_identity_like_cpp, set_loot_money_persistence_test_result_for_test,
    set_pass_on_group_loot_for_test_like_cpp,
};
pub(crate) use wow_world::test_fixtures::{player_gold_for_test, set_player_gold_for_test};

pub(crate) use wow_world::test_fixtures::loot::player_registration_for_loot_test as broadcast_info;
pub(crate) use wow_world::test_fixtures::loot::{
    attach_canonical_creature_for_loot_test as attach_canonical_creature,
    attach_canonical_gameobject_for_loot_test as attach_canonical_gameobject,
    attach_canonical_map_object_for_loot_test as attach_canonical_map_object,
    canonical_creature_snapshot_for_loot_test as canonical_creature_snapshot,
    canonical_gameobject_snapshot_for_loot_test as canonical_gameobject_snapshot,
    canonical_world_object_for_loot_test as canonical_world_object,
    drain_server_opcodes_for_loot_test as drain_server_opcodes_like_cpp,
    gameobject_loot_release_snapshot_for_test as represented_gameobject_loot_release_snapshot_like_cpp,
    insert_allowed_coin_loot_for_test as insert_allowed_coin_loot_like_cpp,
    install_basic_item_template_for_loot_test as install_limited_test_item_template,
    install_cached_test_creature_loot_authority_for_test as install_cached_test_creature_loot_authority_like_cpp,
    install_group_loot_group_for_test as install_group_loot_group,
    install_limited_item_template_for_loot_test as install_limited_test_item_template_with_flags2,
    install_master_loot_group_for_test as install_master_loot_group,
    is_active_loot_guid_for_test as is_active_loot_guid_like_cpp,
    loot_item_packet as loot_item_packet_for_test,
    make_canonical_creature_for_loot_test as make_canonical_creature_for_session,
    make_canonical_gameobject_for_loot_test as make_canonical_gameobject_for_session,
    open_test_ae_pair_for_loot as open_test_ae_pair_like_cpp,
    personal_loot_marker_for_test as personal_loot_marker_like_cpp,
    reconcile_loot_cache_for_test as reconcile_represented_loot_cache_for_test,
    reconcile_loot_cache_for_test as reconcile_represented_loot_cache_like_cpp,
    record_gameobject_chest_release_metadata_for_loot_test as record_gameobject_chest_release_metadata_like_cpp,
    record_gameobject_owner_for_loot_test as record_gameobject_owner_like_cpp,
    register_test_creature_for_loot as register_test_creature_like_cpp,
    represented_loot_object_guid_for_test as represented_loot_object_guid_like_cpp,
    set_active_loot_guid_for_test as set_active_loot_guid_like_cpp,
    set_loot_for_test as set_loot_like_cpp,
    set_personal_loot_for_loot_test as set_personal_loot_like_cpp,
    test_creature_for_loot as test_creature,
    world_creature_corpse_deadline_for_test as corpse_despawn_deadline_for_test,
    world_creature_corpse_delay_for_test as corpse_delay_for_test,
    world_creature_corpse_despawn_at_for_test as corpse_despawn_at_for_test,
};

pub(crate) fn loot_response_failure_reason(sent: &[u8]) -> u8 {
    let mut packet = WorldPacket::from_bytes(&sent[2..]);
    let _owner = packet.read_packed_guid().unwrap();
    let _loot_object = packet.read_packed_guid().unwrap();
    packet.read_uint8().unwrap()
}
