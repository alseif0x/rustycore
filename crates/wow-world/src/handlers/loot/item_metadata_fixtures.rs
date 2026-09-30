//! Fixtures for the real loot item metadata and binding operations.
use super::*;
use wow_entities::Item;

#[derive(Debug, PartialEq, Eq)]
pub struct LootRandomProperties(LootStoreRandomProperties);

impl LootRandomProperties {
    pub fn new(id: i32, seed: i32) -> Self { Self(LootStoreRandomProperties { id, seed }) }
    pub fn id(&self) -> i32 { self.0.id }
    pub fn seed(&self) -> i32 { self.0.seed }
}

pub fn generate_loot_item_properties_for_test<R: Rng + ?Sized>(session: &WorldSession, item: u32, rng: &mut R) -> LootRandomProperties {
    LootRandomProperties(session.generate_loot_store_random_properties_with_rng_like_cpp(item, rng))
}

pub fn new_loot_item_flags_for_test(session: &WorldSession, item: u32, slot: u8) -> u32 {
    session.stored_new_item_dynamic_flags_like_cpp(item, slot)
}

pub fn existing_loot_item_flags_for_test(session: &WorldSession, item: u32, slot: u8, existing: &Item) -> u32 {
    session.stored_existing_item_dynamic_flags_like_cpp(item, slot, existing)
}

#[allow(clippy::too_many_arguments)]
pub fn install_loot_inventory_item_for_test(session: &mut WorldSession, slot: u8, inventory: InventoryItem, player: ObjectGuid, count: u32, durability: u32, context: ItemContext) {
    let guid = inventory.guid;
    let entry = inventory.entry_id;
    session.insert_inventory_item_like_cpp(slot, inventory);
    let item = session.make_inventory_item_object(guid, entry, player, count, durability, context, slot);
    session.insert_inventory_item_object(item);
}

pub fn loot_inventory_item_for_test(session: &WorldSession, guid: ObjectGuid) -> Option<&Item> {
    session.inventory_item_objects_like_cpp().get(&guid)
}

pub fn install_loot_inventory_port_for_test(session: &mut WorldSession, outcome: PersistenceOutcomeLikeCpp) -> Arc<std::sync::Mutex<Vec<wow_persistence::PlayerInventoryPersistenceRequestLikeCpp>>> {
    let (port, requests) = crate::player::inventory_persistence_test_fixture::PlayerInventoryPersistencePortFixtureLikeCpp::new_like_cpp(outcome);
    session.set_player_inventory_persistence_port_like_cpp(port);
    requests
}

pub async fn store_loot_item_for_test(session: &mut WorldSession, entry: &LootEntry, encounter: u32) -> bool {
    let generators = session.id_generators_for_test_like_cpp();
    session.store_direct_loot_item_with_generator_like_cpp(generators.item.as_ref(), entry, encounter).await
}
