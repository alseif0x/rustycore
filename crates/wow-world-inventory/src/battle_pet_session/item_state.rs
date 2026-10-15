// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The uncage item-state load through the Player lifecycle persistence port
//! (#1363 slice 12b1b; moved verbatim from the World
//! `session/player_items/items.rs`).

use std::sync::Arc;

use super::BattlePetSessionHostLikeCpp;

pub async fn uncage_item_state_like_cpp<H: BattlePetSessionHostLikeCpp>(
    host: &H,
    player_db_guid: u64,
    item_db_guid: u64,
) -> wow_persistence::PlayerUncageItemStateLoadOutcomeLikeCpp {
    let Some(port) = host
        .lifecycle()
        .player_lifecycle_port_like_cpp()
        .map(Arc::clone)
    else {
        return wow_persistence::PlayerUncageItemStateLoadOutcomeLikeCpp::Failed {
            reason: "Player lifecycle persistence port is unavailable".to_owned(),
        };
    };
    port.load_uncage_item_state_like_cpp(wow_persistence::PlayerUncageItemStateRequestLikeCpp {
        player_guid: player_db_guid,
        item_guid: item_db_guid,
    })
    .await
}
