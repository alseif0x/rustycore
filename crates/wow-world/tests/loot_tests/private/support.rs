//! Existing feature facades shared by the migrated application families.
pub(super) use super::super::support::*;
pub(super) use wow_packet::packets::loot::MasterLootItem;
pub(super) use wow_world::session::SessionState;
pub(super) use wow_world::test_fixtures::record_represented_gameobject_runtime_state_for_test;
pub(super) use wow_world::test_fixtures::loot::{
    gameobject_loot_release_snapshot_for_test, handle_master_loot_item_for_test,
    mark_chest_restock_expired_for_loot_test, process_pending_for_loot_test,
    record_gameobject_chest_release_metadata_for_loot_test,
};
pub(super) use super::super::support::player_registration_for_loot_test as broadcast_info;
pub(super) use super::super::support::represented_loot_object_guid_for_test
    as represented_loot_object_guid_like_cpp;
