//! WorldSession entry point and facade for loot random item properties
//! (#1363 slice 5a; moved from the World `handlers/loot/random_properties.rs`).

use rand::Rng;

use super::{LootSessionHostLikeCpp, LootStoreRandomProperties};

pub fn generate_loot_store_random_properties_with_rng_like_cpp<
    H: LootSessionHostLikeCpp,
    R: Rng + ?Sized,
>(
    host: &H,
    item_id: u32,
    rng: &mut R,
) -> LootStoreRandomProperties {
    let (state, hub) = host.split_loot_ref();
    state.generate_loot_store_random_properties_with_rng_like_cpp(hub, item_id, rng)
}
