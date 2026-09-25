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

use wow_constants::ItemExtendedCostFlags;
use wow_packet::packets::item::{
    ItemPurchaseContents, ItemPurchaseRefundCurrency, ItemPurchaseRefundItem,
};

fn represented_gameobject_icon_allows_interaction_like_cpp(icon_name: &str) -> bool {
    // C++ `Player::GetGameObjectIfCanInteractWith` rejects exactly this
    // template sentinel before applying the distance check.
    icon_name != "Point"
}

pub(crate) fn item_purchase_contents_from_extended_cost(
    extended_cost: &wow_data::item::extended_cost::ItemExtendedCostEntry,
    money: u64,
) -> ItemPurchaseContents {
    let mut contents = ItemPurchaseContents {
        money,
        ..Default::default()
    };

    for i in 0..5 {
        contents.items[i] = ItemPurchaseRefundItem {
            item_id: extended_cost.item_id[i] as i32,
            item_count: extended_cost.item_count[i] as i32,
        };

        let season_earned = match i {
            0 => extended_cost
                .flags
                .contains(ItemExtendedCostFlags::REQUIRE_SEASON_EARNED_1),
            1 => extended_cost
                .flags
                .contains(ItemExtendedCostFlags::REQUIRE_SEASON_EARNED_2),
            2 => extended_cost
                .flags
                .contains(ItemExtendedCostFlags::REQUIRE_SEASON_EARNED_3),
            3 => extended_cost
                .flags
                .contains(ItemExtendedCostFlags::REQUIRE_SEASON_EARNED_4),
            4 => extended_cost
                .flags
                .contains(ItemExtendedCostFlags::REQUIRE_SEASON_EARNED_5),
            _ => false,
        };
        if !season_earned {
            contents.currencies[i] = ItemPurchaseRefundCurrency {
                currency_id: extended_cost.currency_id[i] as i32,
                currency_count: extended_cost.currency_count[i] as i32,
            };
        }
    }

    contents
}

#[cfg(test)]
#[path = "entities/tests/mod.rs"]
mod tests;
