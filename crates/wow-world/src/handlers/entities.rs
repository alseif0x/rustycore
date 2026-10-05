// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Entity capability packet handlers.
//!
//! Corpse, gameobject and player interaction handlers moved here from the
//! former `handlers::misc` owner. The shared vocabulary below is the part of
//! the old `misc/mod.rs` prelude that these children (and `character/vendor.rs`)
//! actually consume.

mod corpse;
mod gameobject;
mod player;

fn represented_gameobject_icon_allows_interaction_like_cpp(icon_name: &str) -> bool {
    // C++ `Player::GetGameObjectIfCanInteractWith` rejects exactly this
    // template sentinel before applying the distance check.
    icon_name != "Point"
}

#[cfg(test)]
use wow_constants::ItemExtendedCostFlags;
pub(crate) use wow_world_application::item_purchase_contents_from_extended_cost;

#[cfg(test)]
#[path = "../../unit_tests/handlers/entities/tests/mod.rs"]
mod tests;
