// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Represented player-item operations moved from the World
//! `session/player_items/**` (#1363 slice 12b1): durability loss and repair,
//! item mods and the item-bonus stat publication, the item-set item records,
//! the automatic offhand unequip and the item appearance collection gates.
//!
//! C++ anchors stay on each body. The bodies run on the Player's session
//! through [`crate::VendorSessionHostLikeCpp`] (and
//! [`crate::ItemSessionHostLikeCpp`] for the offhand unequip); each host call
//! is the same-named `WorldSession` operation at the exact point the body
//! called it before the move. A World wrapper that was one
//! `split_inventory_{ref,mut}` (or `split_lifecycle_mut`) plus one state call,
//! or one catalog read, is that same split and call here. A former
//! `#[cfg(test)]` step runs only when the host reports the World test
//! fixtures, so it keeps meaning the World test build.
//!
//! Slice 12b1b adds the area-based item-level scaling application and the
//! equipment-set use context, which take the World session's disjoint fields
//! through [`PlayerItemsSessionHostLikeCpp::split_player_items_mut`].

mod appearance;
mod durability;
mod equipment_sets;
mod item_mods;
mod offhand;
mod repair;
mod scaling;

use wow_world_core::session::{HubMut, SessionHubHostLikeCpp};
#[cfg(any(test, feature = "test-fixtures"))]
use wow_world_core::session::{
    KnownSpellsFixtureSourceLikeCpp, PlayerQuestGameplayFixtureSourceLikeCpp,
};
use wow_world_loot::LootState;

use crate::InventoryState;

#[cfg(any(test, feature = "test-fixtures"))]
pub use appearance::can_add_item_appearance_represented_like_cpp;
pub use appearance::{
    add_item_appearance_for_item_like_cpp, add_item_appearance_for_runtime_item_like_cpp,
    on_item_added_to_collection_like_cpp,
};
pub use durability::{
    apply_durability_damage_pct_effect_like_cpp, apply_represented_durability_loss_all_like_cpp,
    apply_represented_durability_points_loss_all_like_cpp,
    apply_represented_durability_points_loss_at_slot_like_cpp,
    publish_creature_melee_death_durability_loss_like_cpp,
};
pub use equipment_sets::build_equipment_set_use_context_like_cpp;
#[cfg(any(test, feature = "test-fixtures"))]
pub use item_mods::record_represented_items_set_item_like_cpp;
pub use item_mods::{
    record_represented_item_mods_like_cpp, record_represented_items_set_item_events_like_cpp,
    send_represented_item_bonus_player_stat_update_like_cpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
pub use offhand::record_direct_inventory_item_set_remove_like_cpp;
pub use offhand::represented_auto_unequip_offhand_if_need_like_cpp;
#[cfg(any(test, feature = "test-fixtures"))]
pub use repair::apply_inventory_item_durability_repair_runtime_like_cpp;
#[cfg(not(any(test, feature = "test-fixtures")))]
pub(crate) use repair::{
    repair_all_inventory_item_durability_with_guild_bank_and_generator_like_cpp,
    repair_all_inventory_item_durability_with_player_money_and_generator_like_cpp,
    repair_inventory_item_durability_with_generator_like_cpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
pub use repair::{
    repair_all_inventory_item_durability_with_guild_bank_and_generator_like_cpp,
    repair_all_inventory_item_durability_with_player_money_and_generator_like_cpp,
    repair_inventory_item_durability_with_generator_like_cpp,
};
pub use scaling::update_represented_item_level_area_based_scaling_with_publication_like_cpp;

/// The World session fields the area-based scaling application and the
/// equipment-set use context borrow, split once (#1363 slice 12b1b): the same
/// disjoint borrows the World bodies took.
pub struct PlayerItemsSessionFieldsLikeCpp<'a> {
    pub hub: HubMut<'a>,
    pub inventory: &'a mut InventoryState,
    pub loot: &'a LootState,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub spell_state: &'a dyn KnownSpellsFixtureSourceLikeCpp,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub quest_state: &'a dyn PlayerQuestGameplayFixtureSourceLikeCpp,
}

pub trait PlayerItemsSessionHostLikeCpp: SessionHubHostLikeCpp {
    /// The disjoint field borrows of the World session.
    fn split_player_items_mut(&mut self) -> PlayerItemsSessionFieldsLikeCpp<'_>;
}

/// The World crate's `cfg!(test)`: only the World test build's session
/// returns its fixtures from `test_fixtures_ref_like_cpp` (#1404 review).
fn world_test_consumer_like_cpp<H: SessionHubHostLikeCpp + ?Sized>(host: &H) -> bool {
    #[cfg(any(test, feature = "test-fixtures"))]
    let world_test = host.test_fixtures_ref_like_cpp().is_some();
    #[cfg(not(any(test, feature = "test-fixtures")))]
    let world_test = {
        let _ = host;
        false
    };
    world_test
}
