// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Hydrate the selected character's represented glyphs during login.

use crate::CharacterLoginHostLikeCpp;
use std::sync::Arc;
use wow_core::ObjectGuid;
use wow_world_core::session::PlayerBootstrapCatalogsLikeCpp;

pub async fn load_character_glyphs_for_login_like_cpp<H: CharacterLoginHostLikeCpp>(
    host: &mut H,
    player_lifecycle_port: &Arc<dyn wow_persistence::PlayerLifecyclePortLikeCpp>,
    player_bootstrap: &PlayerBootstrapCatalogsLikeCpp,
    guid: ObjectGuid,
) {
    let (state, mut hub) = host.split_lifecycle_mut();
    state
        .load_character_glyphs_for_login_like_cpp(
            &mut hub,
            player_lifecycle_port,
            player_bootstrap,
            guid,
        )
        .await
}
