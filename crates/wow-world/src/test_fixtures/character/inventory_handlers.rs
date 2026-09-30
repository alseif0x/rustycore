//! Fixture adapters use the same generator and catalog selection as the unit-test handlers.

use crate::session::WorldSession;
use wow_packet::packets::item::{AutoEquipItem, AutoEquipItemSlot, AutoStoreBagItem, SwapInvItem, SwapItem};

pub async fn swap_inventory_item_for_test(session: &mut WorldSession, swap: SwapInvItem) {
    let generators = session.id_generators_for_test_like_cpp();
    let catalogs = session.creature_spawn_catalogs_for_test_like_cpp();
    session.handle_swap_inv_item_with_generator_like_cpp(
        generators.item.as_ref(),
        &catalogs,
        swap,
    ).await;
}

pub async fn auto_equip_item_for_test(session: &mut WorldSession, equip: AutoEquipItem) {
    let generators = session.id_generators_for_test_like_cpp();
    let catalogs = session.creature_spawn_catalogs_for_test_like_cpp();
    session.handle_auto_equip_item_with_generator_like_cpp(
        generators.item.as_ref(),
        &catalogs,
        equip,
    ).await;
}

pub async fn auto_equip_item_slot_for_test(session: &mut WorldSession, equip: AutoEquipItemSlot) {
    let generators = session.id_generators_for_test_like_cpp();
    let catalogs = session.creature_spawn_catalogs_for_test_like_cpp();
    session.handle_auto_equip_item_slot_with_generator_like_cpp(
        generators.item.as_ref(),
        &catalogs,
        equip,
    ).await;
}

pub async fn swap_item_for_test(session: &mut WorldSession, swap: SwapItem) {
    let generators = session.id_generators_for_test_like_cpp();
    let catalogs = session.creature_spawn_catalogs_for_test_like_cpp();
    session.handle_swap_item_with_generator_like_cpp(generators.item.as_ref(), &catalogs, swap)
        .await;
}

pub async fn auto_store_bag_item_for_test(session: &mut WorldSession, store: AutoStoreBagItem) {
    let generators = session.id_generators_for_test_like_cpp();
    let catalogs = session.creature_spawn_catalogs_for_test_like_cpp();
    session.handle_auto_store_bag_item_with_generator_like_cpp(
        generators.item.as_ref(),
        &catalogs,
        store,
    ).await;
}
