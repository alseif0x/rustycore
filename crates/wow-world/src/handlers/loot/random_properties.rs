//! WorldSession entry point and facade for loot random item properties.

use rand::Rng;

use crate::session::WorldSession;

pub(super) use wow_world_loot::{LootStoreRandomProperties, loot_store_data_can_stack_with_item};

impl WorldSession {
    pub(super) fn generate_loot_store_random_properties_with_rng_like_cpp<R: Rng + ?Sized>(
        &self,
        item_id: u32,
        rng: &mut R,
    ) -> LootStoreRandomProperties {
        let (state, hub) = crate::session::split_loot_ref(self);
        state.generate_loot_store_random_properties_with_rng_like_cpp(hub, item_id, rng)
    }
}
