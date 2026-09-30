//! Borrowed query and redirect validation forwards; no inventory copies.

use crate::session::WorldSession;

pub fn inventory_swap_preflight_for_test(
    session: &WorldSession,
    source: u16,
    destination: u16,
) -> Option<wow_entities::SwapItemPreflightPlan> {
    session.plan_inventory_swap_preflight_like_cpp(source, destination)
}

pub fn inventory_child_redirect_for_test(
    session: &mut WorldSession,
    child_bag: u8,
    child_slot: u8,
    first_source: u16,
    first_destination: u16,
    second_source: u16,
    second_destination: u16,
) -> Result<u8, wow_constants::InventoryResult> {
    session.plan_inventory_child_redirect_like_cpp(
        child_bag,
        child_slot,
        first_source,
        first_destination,
        second_source,
        second_destination,
    )
}
