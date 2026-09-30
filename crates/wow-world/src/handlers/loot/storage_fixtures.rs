//! Real inventory storage ports and application observations for loot fixtures.
use super::*;

pub fn install_loot_wire_channel_for_test(session: &mut WorldSession, sender: flume::Sender<Vec<u8>>) {
    session.install_realm_send_channel_for_test(sender);
}

pub fn attach_loot_inventory_port_for_test(session: &mut WorldSession, port: Arc<dyn wow_persistence::PlayerInventoryPersistencePortLikeCpp>) {
    session.set_player_inventory_persistence_port_like_cpp(port);
}

pub fn applied_loot_item_quantity_for_test(session: &WorldSession, item: u32) -> u32 {
    session.direct_inventory_item_count_like_cpp(item).expect("resident loot inventory owner")
}

pub async fn store_loot_materials_for_test(session: &mut WorldSession, materials: &[LootEntry], encounter: u32, claim: Option<&LootClaimLease>, owner: ObjectGuid, loot: ObjectGuid, list: u8, player: ObjectGuid, free_for_all: bool) -> bool {
    let generators = session.id_generators_for_test_like_cpp();
    session.store_direct_disenchant_batch_with_generator_like_cpp(generators.item.as_ref(), materials, encounter, claim, Some(LootItemClaimCommitContextLikeCpp { owner_guid: owner, loot_obj: loot, loot_list_id: list, player_guid: player, free_for_all })).await
}

pub async fn request_master_loot_store_for_test(session: &WorldSession, target: ObjectGuid, owner: ObjectGuid, loot: ObjectGuid, list: u8, encounter: u32, entry: LootEntry, claim: Option<LootClaimLease>) -> MasterLootGiveResult {
    session.request_represented_remote_master_loot_give_like_cpp(target, owner, loot, list, encounter, entry, claim).await
}

pub async fn request_roll_loot_store_for_test(session: &WorldSession, target: ObjectGuid, owner: ObjectGuid, loot: ObjectGuid, list: u8, encounter: u32, entries: Vec<LootEntry>, disenchant: bool, claim: Option<LootClaimLease>) -> MasterLootGiveResult {
    session.request_represented_remote_loot_roll_winner_store_like_cpp(target, owner, loot, list, encounter, entries, disenchant, claim).await
}
