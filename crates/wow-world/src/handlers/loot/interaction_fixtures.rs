//! Explicit operation-local fixture rails over the shared application cores.
use super::*;

pub async fn open_gameobject_loot_cycle_for_test(session: &mut WorldSession, owner: ObjectGuid, source: GameObjectLootSource) {
    let money = session.world_query_catalogs_like_cpp().and_then(|catalogs| catalogs.gameobject.get(owner.entry())).map(|row| (row.min_money, row.max_money)).unwrap_or((0, 0));
    let generators = session.id_generators_for_test_like_cpp();
    let valuations = session.item_valuation_catalogs_for_test_like_cpp();
    session.open_gameobject_chest_operation(generators.item.as_ref(), &valuations, owner, source, money, LootOperationPolicy::GameObjectFixture).await;
}
pub async fn open_fishing_loot_cycle_for_test(session: &mut WorldSession, owner: ObjectGuid, area: u32, junk: bool) {
    let valuations = session.item_valuation_catalogs_for_test_like_cpp();
    session.open_fishing_node_operation(&valuations, owner, area, junk, LootOperationPolicy::GameObjectFixture).await;
}
pub async fn open_fishing_hole_cycle_for_test(session: &mut WorldSession, owner: ObjectGuid, entry: u32, loot: u32) {
    let valuations = session.item_valuation_catalogs_for_test_like_cpp();
    session.open_fishing_hole_operation(&valuations, owner, entry, loot, LootOperationPolicy::GameObjectFixture).await;
}
pub async fn open_gathering_loot_cycle_for_test(session: &mut WorldSession, owner: ObjectGuid, entry: u32, source: GatheringNodeUseSource) {
    let valuations = session.item_valuation_catalogs_for_test_like_cpp();
    session.open_gathering_node_operation(&valuations, owner, entry, source, LootOperationPolicy::GameObjectFixture).await;
}
pub async fn take_local_loot_item_for_test(session: &mut WorldSession, packet: WorldPacket) {
    let generators = session.id_generators_for_test_like_cpp();
    session.handle_loot_item_operation(generators.item.as_ref(), packet, LootOperationPolicy::LocalRequestFixture).await;
}
pub async fn give_local_master_loot_for_test(session: &mut WorldSession, request: MasterLootItem) {
    let generators = session.id_generators_for_test_like_cpp();
    session.handle_master_loot_operation(generators.item.as_ref(), request, LootOperationPolicy::LocalRequestFixture).await;
}
pub async fn release_local_loot_for_test(session: &mut WorldSession, owner: ObjectGuid, player: ObjectGuid) -> bool {
    session.release_loot_owner_operation(owner, player, LootOperationPolicy::LocalRequestFixture).await
}
pub fn remove_master_loot_slot_for_test(session: &mut WorldSession, owner: ObjectGuid, loot: ObjectGuid, list: u8, player: ObjectGuid) {
    session.mark_represented_master_loot_item_removed_like_cpp(owner, loot, list, player);
}

