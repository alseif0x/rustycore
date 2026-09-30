//! Opaque access to Character's existing inventory decisions.
use super::super::*;
use super::super::inventory_plan::{
    DirectInventoryPositionUpdateLikeCpp, plan_direct_inventory_swap_persistence_like_cpp,
};

#[derive(Debug, PartialEq, Eq)]
pub struct InventoryPositionUpdatesForTest(Vec<DirectInventoryPositionUpdateLikeCpp>);

pub fn inventory_position_updates_for_test(updates: &[(u8, u64)]) -> InventoryPositionUpdatesForTest {
    InventoryPositionUpdatesForTest(updates.iter().map(|&(slot, item_db_guid)| {
        DirectInventoryPositionUpdateLikeCpp::from_fixture_fields(slot, item_db_guid)
    }).collect())
}

pub fn direct_inventory_swap_persistence_for_test(
    src: u8, dst: u8, src_item: Option<&InventoryItem>, dst_item: Option<&InventoryItem>,
) -> InventoryPositionUpdatesForTest {
    InventoryPositionUpdatesForTest(plan_direct_inventory_swap_persistence_like_cpp(src, dst, src_item, dst_item))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InventoryStorageTargetForTest(InventoryStorageTargetLikeCpp);
impl InventoryStorageTargetForTest {
    pub fn inventory() -> Self { Self(InventoryStorageTargetLikeCpp::Inventory) }
    pub fn bank() -> Self { Self(InventoryStorageTargetLikeCpp::Bank) }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InventoryQuestChecksForTest(InventoryStorageQuestChecksLikeCpp);
impl InventoryQuestChecksForTest {
    pub fn none() -> Self { Self(InventoryStorageQuestChecksLikeCpp::None) }
    pub fn autostore_added() -> Self { Self(InventoryStorageQuestChecksLikeCpp::AutoStoreBankItemAdded) }
}

pub fn autostore_bank_target_for_test(bag: u8, slot: u8) -> InventoryStorageTargetForTest {
    InventoryStorageTargetForTest(autostore_bank_target_like_cpp(bag, slot))
}

pub fn autostore_bank_quest_checks_for_test(target: InventoryStorageTargetForTest) -> InventoryQuestChecksForTest {
    InventoryQuestChecksForTest(autostore_bank_quest_checks_like_cpp(target.0))
}

pub fn inventory_move_quest_directions_for_test(bag: u8, slot: u8, target: InventoryStorageTargetForTest) -> (bool, bool) {
    inventory_storage_move_quest_directions_like_cpp(bag, slot, target.0)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InventoryChildPlanForTest(InventoryEquipChildPlanLikeCpp);
impl InventoryChildPlanForTest {
    pub fn new(child_guid: ObjectGuid, destination_slot: u8, displaced_storage: Option<(u8, u8, InventoryStorageTargetForTest)>) -> Self {
        Self(InventoryEquipChildPlanLikeCpp {
            child_guid, destination_slot,
            displaced_storage: displaced_storage.map(|(bag, slot, target)| (bag, slot, target.0)),
        })
    }
    pub fn displaced_storage(&self) -> Option<(u8, u8, InventoryStorageTargetForTest)> {
        self.0.displaced_storage.map(|(bag, slot, target)| (bag, slot, InventoryStorageTargetForTest(target)))
    }
}

pub fn inventory_equip_child_for_test(session: &WorldSession, bag: u8, slot: u8, guid: ObjectGuid) -> Result<Option<InventoryChildPlanForTest>, InventoryResult> {
    session.plan_inventory_equip_child_like_cpp(bag, slot, guid).map(|plan| plan.map(InventoryChildPlanForTest))
}

pub fn inventory_real_swap_children_for_test(
    session: &WorldSession, source_bag: u8, source_slot: u8, source: ObjectGuid,
    destination_bag: u8, destination_slot: u8, destination: ObjectGuid,
) -> Result<Vec<InventoryChildPlanForTest>, InventoryResult> {
    session.plan_inventory_real_swap_children_like_cpp(source_bag, source_slot, source, destination_bag, destination_slot, destination)
        .map(|plans| plans.into_iter().map(InventoryChildPlanForTest).collect())
}

pub fn inventory_storage_move_for_test(session: &WorldSession, source_bag: u8, source_slot: u8, destination_bag: u8, destination_slot: u8, target: InventoryStorageTargetForTest) -> Option<Result<InventoryStorageMovePlanLikeCpp, InventoryResult>> {
    session.plan_inventory_storage_move_like_cpp(source_bag, source_slot, destination_bag, destination_slot, target.0)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DestroyItemCountActionForTest(DestroyItemCountAction);
impl DestroyItemCountActionForTest {
    pub fn full_stack() -> Self { Self(DestroyItemCountAction::FullStack) }
    pub fn partial_stack(new_count: u32) -> Self { Self(DestroyItemCountAction::PartialStack { new_count }) }
}
pub fn destroy_item_count_action_for_test(current: u32, requested: u32) -> DestroyItemCountActionForTest {
    DestroyItemCountActionForTest(destroy_item_count_action(current, requested))
}

#[derive(Debug, PartialEq, Eq)]
pub struct InventoryTurninPlanForTest(Vec<ExtendedCostItemTurninChange>);
impl InventoryTurninPlanForTest {
    pub fn new(changes: Vec<InventoryTurninChangeForTest>) -> Self {
        Self(changes.into_iter().map(|change| change.0).collect())
    }
}
pub struct InventoryTurninChangeForTest(ExtendedCostItemTurninChange);
impl InventoryTurninChangeForTest {
    pub fn delete(slot: u8, item_guid: ObjectGuid, db_guid: u64) -> Self {
        Self(ExtendedCostItemTurninChange::Delete { slot, item_guid, db_guid })
    }
    pub fn update(slot: u8, item_guid: ObjectGuid, db_guid: u64, new_count: u32) -> Self {
        Self(ExtendedCostItemTurninChange::Update { slot, item_guid, db_guid, new_count })
    }
}
pub fn inventory_has_item_count_for_test(session: &WorldSession, entry: u32, count: u32) -> bool {
    session.has_item_count_direct_inventory(entry, count)
}
pub fn inventory_turnin_plan_for_test(session: &WorldSession, entry: u32, count: u32) -> Option<InventoryTurninPlanForTest> {
    session.plan_destroy_item_count_direct_inventory(entry, count).map(InventoryTurninPlanForTest)
}
pub fn inventory_destroy_quest_plan_for_test(session: &WorldSession, items: &[(u8, u8, u32, u32)]) -> Option<Vec<crate::handlers::quest::PlayerQuestStatus>> {
    let items: Vec<_> = items.iter().map(|&(bag, slot, entry_id, count)| DestroyQuestItemLikeCpp { bag, slot, entry_id, count }).collect();
    session.plan_destroyed_inventory_quest_persistence_like_cpp(&items)
}
