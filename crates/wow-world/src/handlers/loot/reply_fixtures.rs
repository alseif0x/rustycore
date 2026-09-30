//! Fixtures for the real loot response and publication paths.
use super::*;

pub fn loot_item_fanout_at_commit_for_test(before: &[ObjectGuid], committed: &[ObjectGuid]) -> HashSet<ObjectGuid> {
    durable_loot_item_fanout_viewers_like_cpp(before, committed)
}

pub fn loot_release_values_for_test(session: &WorldSession, owner: ObjectGuid, viewer: ObjectGuid, pending_bind: bool, authority: Option<&OwnedLootAuthority>, update: wow_packet::packets::update::UnitDataValuesDeltaUpdate) -> wow_packet::packets::update::UnitDataValuesDeltaUpdate {
    session.creature_loot_release_values_for_viewer_like_cpp(owner, viewer, pending_bind, authority, update)
}
pub fn loot_client_type_for_test(kind: u8) -> u8 { loot_type_for_client_like_cpp(kind) }
pub fn master_loot_inventory_error_for_test(result: InventoryResult) -> Option<u8> { master_loot_error_for_inventory_result_like_cpp(result) }
pub async fn loot_response_for_test(session: &mut WorldSession, owner: ObjectGuid, player: ObjectGuid, ae: bool) -> Option<LootResponse> { session.represented_loot_response_for_owner_like_cpp(owner, player, ae).await }
pub fn loot_fixture_response(owner: ObjectGuid, loot: &CreatureLoot, player: ObjectGuid) -> LootResponse {
    LootResponse {
        owner,
        loot_obj: loot.loot_guid,
        failure_reason: LOOT_RESPONSE_DEFAULT_FAILURE_REASON_LIKE_CPP,
        acquire_reason: loot_type_for_client_like_cpp(loot.loot_type),
        loot_method: loot.loot_method,
        threshold: LOOT_RESPONSE_DEFAULT_THRESHOLD_LIKE_CPP,
        coins: loot.coins,
        items: represented_loot_response_items_like_cpp(loot, player),
        currencies: Vec::new(),
        acquired: true,
        ae_looting: false,
    }
}
pub fn open_loot_response_for_test(session: &mut WorldSession, owner: ObjectGuid, player: ObjectGuid, response: LootResponse) {
    let item_valuation = session.item_valuation_catalogs_for_test_like_cpp();
    session.represented_on_loot_opened_with_catalogs_like_cpp(&item_valuation, owner, player, response);
}
pub fn send_loot_failure_for_test(session: &WorldSession, loot: ObjectGuid, owner: ObjectGuid, error: u8) { session.send_loot_error_like_cpp(loot, owner, error); }
pub fn notify_cached_loot_item_for_test(session: &mut WorldSession, owner: ObjectGuid, index: u8) { session.represented_notify_loot_item_removed_like_cpp(owner, index); }
pub fn notify_committed_loot_item_for_test(session: &mut WorldSession, owner: ObjectGuid, authority: Option<&OwnedLootAuthority>, snapshot: &wow_loot::OwnedLootSnapshot, index: u8) { session.represented_notify_loot_item_removed_from_snapshot_like_cpp(owner, authority, snapshot, index); }
#[allow(clippy::too_many_arguments)]
pub fn push_loot_item_for_test(session: &WorldSession, player: ObjectGuid, item: ObjectGuid, entry: &LootEntry, properties: i32, seed: i32, slot: u8, quantity: u32, inventory_quantity: u32, created: bool, encounter: u32) {
    session.send_loot_item_push_result(player, item, entry, properties, seed, slot, quantity, inventory_quantity, created, encounter);
}

pub fn start_loot_roll_for_test(loot: ObjectGuid, map: u16, method: u8, entry: &LootEntry, valid: u8, encounter: i32) -> StartLootRoll {
    start_loot_roll_packet_like_cpp(loot, map, method, entry, valid, encounter)
}
