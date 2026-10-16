// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The Player's alive and ghost state writes (C++ `Unit::setDeathState` with
//! the health reset, and `PLAYER_FLAGS_GHOST`), each followed by the registry
//! sync (#1363 slice 12d; moved from the World
//! `session/player_vitals_adapter.rs`). The former `#[cfg(test)]` fixture
//! step runs only when `test_fixtures_ref_like_cpp` is `Some` (the World lib
//! test build).

use wow_world_core::session::PLAYER_FLAGS_GHOST_LIKE_CPP;

use crate::PlayerMovementHostLikeCpp;

pub fn set_player_alive_like_cpp<H: PlayerMovementHostLikeCpp + ?Sized>(host: &mut H, alive: bool) {
    #[cfg(any(test, feature = "test-fixtures"))]
    let world_test = host.test_fixtures_ref_like_cpp().is_some();
    #[cfg(not(any(test, feature = "test-fixtures")))]
    let world_test = false;
    let resolved = host.core().with_owned_player_mut_like_cpp(|player| {
        if !alive {
            player
                .unit_mut()
                .set_death_state(wow_constants::DeathState::Corpse);
            player.unit_mut().set_health(0);
        } else {
            let max_health = player.unit().data().max_health.max(1);
            player
                .unit_mut()
                .set_death_state(wow_constants::DeathState::Alive);
            if player.unit().data().health == 0 {
                player.unit_mut().set_health(max_health);
            }
        }
        (
            player.unit().data().health.min(u64::from(u32::MAX)) as u32,
            player
                .unit()
                .data()
                .max_health
                .clamp(1, u64::from(u32::MAX)) as u32,
            player.unit().is_alive(),
        )
    });
    #[cfg(any(test, feature = "test-fixtures"))]
    if world_test {
        let (health, max_health, is_alive) = resolved.unwrap_or_else(|| {
            let max_health = host
                .hub_mut()
                .fixtures
                .combat
                .player_max_health_like_cpp
                .max(1);
            let health = if alive {
                host.hub_mut()
                    .fixtures
                    .combat
                    .player_health_like_cpp
                    .max(1)
                    .min(max_health)
            } else {
                0
            };
            (health, max_health, alive)
        });
        host.hub_mut().fixtures.combat.player_health_like_cpp = health;
        host.hub_mut().fixtures.combat.player_max_health_like_cpp = max_health;
        host.hub_mut().fixtures.combat.player_alive_like_cpp = is_alive;
    }
    if resolved.is_some() || world_test && host.core().player_handle_like_cpp.is_none() {
        host.sync_player_registry_state_like_cpp();
    }
}

pub fn set_player_ghost_flag_like_cpp<H: PlayerMovementHostLikeCpp + ?Sized>(
    host: &mut H,
    ghost: bool,
) {
    if host.player_guid().is_some() {
        let _ = host.core().mutate_canonical_player_like_cpp(|player| {
            if ghost {
                player.set_player_flag(PLAYER_FLAGS_GHOST_LIKE_CPP);
            } else {
                player.remove_player_flag(PLAYER_FLAGS_GHOST_LIKE_CPP);
            }
        });
    }
    host.sync_player_registry_state_like_cpp();
}
