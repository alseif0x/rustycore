// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The Player's gossip option menu (`PlayerMenu::GetGossipMenu`) as the
//! gossip bodies read and replace it, moved from the World
//! `session/chat/operations.rs` (#1363 slice 12b5). The canonical Player owns
//! the options; the World test build keeps its detached fixture copy in the
//! interaction state.

use wow_entities::PlayerGossipOptionLikeCpp as GossipOptionInfo;

use crate::CharacterLoginHostLikeCpp;

/// The World crate's `cfg!(test)`: only the World test build's session
/// returns its fixtures from `test_fixtures_ref_like_cpp` (#1404 review).
fn world_test_consumer_like_cpp<H: CharacterLoginHostLikeCpp>(host: &H) -> bool {
    #[cfg(any(test, feature = "test-fixtures"))]
    let world_test = host.test_fixtures_ref_like_cpp().is_some();
    #[cfg(not(any(test, feature = "test-fixtures")))]
    let world_test = {
        let _ = host;
        false
    };
    world_test
}

pub fn clear_player_gossip_options_like_cpp<H: CharacterLoginHostLikeCpp>(host: &mut H) -> bool {
    let canonical = host
        .core()
        .with_owned_player_mut_like_cpp(|player| player.clear_gossip_options_like_cpp())
        .is_some();
    #[cfg(any(test, feature = "test-fixtures"))]
    if host.test_fixtures_ref_like_cpp().is_some()
        && (canonical || host.core().player_handle_like_cpp.is_none())
    {
        host.split_interaction()
            .0
            .clear_gossip_options_for_test_like_cpp();
    }
    canonical || world_test_consumer_like_cpp(host) && host.core().player_handle_like_cpp.is_none()
}
pub fn replace_player_gossip_options_like_cpp<H: CharacterLoginHostLikeCpp>(
    host: &mut H,
    options: Vec<GossipOptionInfo>,
) -> bool {
    #[cfg(any(test, feature = "test-fixtures"))]
    let fixture_options = options.clone();
    let mut options = Some(options);
    let canonical = host
        .core()
        .with_owned_player_mut_like_cpp(|player| {
            player.replace_gossip_options_like_cpp(
                options.take().expect("gossip option mutation runs once"),
            );
        })
        .is_some();
    #[cfg(any(test, feature = "test-fixtures"))]
    if host.test_fixtures_ref_like_cpp().is_some()
        && (canonical || host.core().player_handle_like_cpp.is_none())
    {
        host.split_interaction()
            .0
            .replace_gossip_options_for_test_like_cpp(fixture_options);
    }
    canonical || world_test_consumer_like_cpp(host) && host.core().player_handle_like_cpp.is_none()
}
pub fn player_gossip_option_like_cpp<H: CharacterLoginHostLikeCpp>(
    host: &H,
    gossip_option_id: i32,
) -> Option<GossipOptionInfo> {
    let canonical = host.core().with_owned_player_like_cpp(|player| {
        player
            .gossip_options_like_cpp()
            .iter()
            .find(|option| option.gossip_option_id == gossip_option_id)
            .cloned()
    });
    #[cfg(any(test, feature = "test-fixtures"))]
    if canonical.is_none()
        && host.core().player_handle_like_cpp.is_none()
        && let Some(interaction) = host.interaction_for_test_like_cpp()
    {
        return interaction
            .gossip_options_for_test_like_cpp()
            .iter()
            .find(|option| option.gossip_option_id == gossip_option_id)
            .cloned();
    }
    canonical.flatten()
}
