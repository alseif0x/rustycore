//! SQLx-free persistence contract for Player-owned inventory mutations.

use crate::{PersistenceFutureLikeCpp, PersistenceOutcomeLikeCpp};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InventoryItemMutablePersistenceLikeCpp {
    pub item_guid: u64,
    pub count: u32,
    pub expiration: u32,
    pub charges: String,
    pub flags: u32,
    pub enchantments: String,
    pub durability: u32,
    pub played_time: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InventoryLinkPersistenceLikeCpp {
    pub owner_guid: u64,
    pub bag_guid: u64,
    pub slot: u8,
    pub item_guid: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuestObjectiveCountPersistenceLikeCpp {
    pub objective_index: u8,
    pub count: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestStatusPersistenceLikeCpp {
    pub quest_id: u32,
    pub status: u8,
    pub explored: bool,
    pub accept_time_secs: i64,
    pub end_time_secs: i64,
    pub objectives: Vec<QuestObjectiveCountPersistenceLikeCpp>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InventoryStorageMovePersistenceLikeCpp {
    pub owner_guid: u64,
    pub mutable_items: Vec<InventoryItemMutablePersistenceLikeCpp>,
    pub delete_source_link_item_guid: Option<u64>,
    pub destination_link: Option<InventoryLinkPersistenceLikeCpp>,
    pub fully_merged_source_item_guid: Option<u64>,
    pub quest_statuses: Vec<QuestStatusPersistenceLikeCpp>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InventoryEquipPersistenceLikeCpp {
    pub mutable_item: InventoryItemMutablePersistenceLikeCpp,
    pub delete_source_link_owner_guid: u64,
    pub delete_source_link_item_guid: u64,
    pub destination_link: InventoryLinkPersistenceLikeCpp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InventoryStackMergeSourcePersistenceLikeCpp {
    Retained(InventoryItemMutablePersistenceLikeCpp),
    FullyMerged { item_guid: u64 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InventoryStackMergePersistenceLikeCpp {
    pub owner_guid: u64,
    pub destination_item: InventoryItemMutablePersistenceLikeCpp,
    pub source: InventoryStackMergeSourcePersistenceLikeCpp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InventorySwapPersistenceLikeCpp {
    pub source_item: InventoryItemMutablePersistenceLikeCpp,
    pub destination_item: InventoryItemMutablePersistenceLikeCpp,
    /// C++ bag-content relinks occur before the two parent item relinks.
    pub child_links: Vec<InventoryLinkPersistenceLikeCpp>,
    pub source_link: InventoryLinkPersistenceLikeCpp,
    pub destination_link: InventoryLinkPersistenceLikeCpp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InventoryPartialDestroyPersistenceLikeCpp {
    pub owner_guid: u64,
    pub item_guid: u64,
    pub new_count: u32,
    pub quest_statuses: Vec<QuestStatusPersistenceLikeCpp>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InventoryDestroyNodePersistenceLikeCpp {
    pub item_guid: u64,
    pub expected_owner_db_guid: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InventoryGraphDestroyPersistenceLikeCpp {
    pub owner_guid: u64,
    /// Descendants in postorder followed by the root, matching C++ recursive
    /// `Player::DestroyItem` persistence order.
    pub nodes: Vec<InventoryDestroyNodePersistenceLikeCpp>,
    pub quest_statuses: Vec<QuestStatusPersistenceLikeCpp>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlayerInventoryPersistenceRequestLikeCpp {
    StorageMove(InventoryStorageMovePersistenceLikeCpp),
    Equip(InventoryEquipPersistenceLikeCpp),
    StackMerge(InventoryStackMergePersistenceLikeCpp),
    Swap(InventorySwapPersistenceLikeCpp),
    PartialDestroy(InventoryPartialDestroyPersistenceLikeCpp),
    GraphDestroy(InventoryGraphDestroyPersistenceLikeCpp),
}

pub trait PlayerInventoryPersistencePortLikeCpp: Send + Sync {
    fn persist_inventory_mutation_like_cpp(
        &self,
        request: PlayerInventoryPersistenceRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'_, PersistenceOutcomeLikeCpp>;
}
