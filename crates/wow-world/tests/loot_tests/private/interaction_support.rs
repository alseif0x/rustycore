//! Explicit loot operation fixtures and original gameobject inputs.
pub(super) use super::support::*;
pub(super) use wow_constants::ItemContext;
pub(super) use wow_data::{AreaTableEntry, AreaTableStore};
pub(super) use wow_entities::{
    GAMEOBJECT_TYPE_FISHING_NODE, GAMEOBJECT_TYPE_GATHERING_NODE, GO_DYNFLAG_LO_NO_INTERACT,
    GatheringNodeUseSource, GoState,
};
pub(super) use wow_loot::{
    LOOT_METHOD_MASTER_LIKE_CPP, LootStore, LootStoreItem, LootStoreKind, LootStores,
    LootTemplateRow,
};
pub(super) use wow_packet::packets::loot::{
    LOOT_ERROR_MASTER_OTHER_LIKE_CPP, LOOT_ERROR_MASTER_UNIQUE_ITEM_LIKE_CPP,
    LOOT_ERROR_PLAYER_NOT_FOUND_LIKE_CPP, LOOT_TYPE_FISHING_JUNK_LIKE_CPP,
    LOOT_TYPE_FISHING_LIKE_CPP, LOOT_TYPE_GATHERING_NODE_LIKE_CPP,
};
pub(super) use wow_world::session::InventoryItem;
pub(super) use wow_world::test_fixtures::loot::*;
pub(super) use wow_world::test_fixtures::{
    record_represented_gameobject_runtime_state_for_test, set_owned_player_group_like_cpp,
};
pub(super) const LOOT_MODE_DEFAULT_LIKE_CPP: u16 = 0x01;
pub(super) const LOOT_MODE_JUNK_FISH_LIKE_CPP: u16 = 0x8000;
