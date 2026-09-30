//! Narrow forwards to the existing inventory runtime operations.

use crate::session::{InventoryItem, WorldSession};
use wow_core::ObjectGuid;

pub fn enable_ownerless_inventory_snapshots_for_test(session: &mut WorldSession) {
    session.enable_ownerless_inventory_snapshots_for_test();
}

pub fn inventory_descendants_for_test(
    session: &WorldSession,
    container_guid: ObjectGuid,
) -> Option<Vec<(u8, u8, InventoryItem)>> {
    session.represented_inventory_descendants_postorder_like_cpp(container_guid)
}

pub fn apply_inventory_swap_for_test(
    session: &mut WorldSession,
    source_bag: u8,
    source_slot: u8,
    destination_bag: u8,
    destination_slot: u8,
) -> bool {
    session.apply_committed_inventory_item_swap_like_cpp(
        source_bag,
        source_slot,
        destination_bag,
        destination_slot,
    )
}

pub async fn destroy_inventory_stack_for_test(
    session: &mut WorldSession,
    bag: u8,
    slot: u8,
    item: InventoryItem,
    runtime_item: Option<wow_entities::Item>,
    context: &str,
) -> bool {
    session.destroy_inventory_full_stack_by_pos_like_cpp(
        bag,
        slot,
        item,
        runtime_item,
        context,
    ).await
}
