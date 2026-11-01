// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The RestMgr flag operations over the canonical Player (C++
//! `RestMgr::SetRestFlag`/`RemoveRestFlag` and the deferred rest-flag
//! publication of `Player::UpdateZone`) and the rest-state owner access
//! (#1363 slice 12c; moved from the World `session/canonical_access/operations.rs`).
//! The former `#[cfg(test)]` handle-less steps run only when
//! `SessionHubHostLikeCpp::test_fixtures_ref_like_cpp` is `Some`, through the
//! `InstanceSessionHostLikeCpp::mutate_player_rest_state_like_cpp` fixture accessor.
//! The login rest-bonus load and its handle-less fixture step moved here from
//! the World `session/persistence/load.rs` (#1363 final cleanup).

use wow_constants::rest::REST_STATE_NORMAL_LIKE_CPP;
use wow_entities::Player;
#[cfg(any(test, feature = "test-fixtures"))]
use wow_world_core::session::PLAYER_FLAGS_RESTING_LIKE_CPP;
use wow_world_core::session::SessionHubHostLikeCpp;

use super::InstanceSessionHostLikeCpp;

pub fn with_owned_player_mut_for_rest_like_cpp<H: SessionHubHostLikeCpp + ?Sized, R>(
    host: &H,
    f: impl FnOnce(&mut Player) -> R,
) -> Option<R> {
    #[cfg(any(test, feature = "test-fixtures"))]
    if host.test_fixtures_ref_like_cpp().is_some() && host.core().player_handle_like_cpp.is_none() {
        return host.core().mutate_canonical_player_like_cpp(f);
    }
    host.core().with_owned_player_mut_like_cpp(f)
}

pub fn set_player_rest_flag_like_cpp<H: InstanceSessionHostLikeCpp>(
    host: &mut H,
    rest_flag: u32,
    trigger_id: u32,
) -> bool {
    let canonical = host.core().with_owned_player_mut_like_cpp(|player| {
        player.set_rest_flag_like_cpp(rest_flag, trigger_id, || {
            wow_core::GameTime::now().as_secs()
        })
    });
    if let Some(changed) = canonical {
        return changed;
    }
    #[cfg(any(test, feature = "test-fixtures"))]
    if host.test_fixtures_ref_like_cpp().is_some() && host.core().player_handle_like_cpp.is_none() {
        return host
            .mutate_player_rest_state_like_cpp(|state| {
                state.set_flag_like_cpp(rest_flag, trigger_id, || {
                    wow_core::GameTime::now().as_secs()
                })
            })
            .unwrap_or(false);
    }
    false
}

pub fn remove_player_rest_flag_like_cpp<H: InstanceSessionHostLikeCpp>(
    host: &mut H,
    rest_flag: u32,
) -> bool {
    let canonical = host
        .core()
        .with_owned_player_mut_like_cpp(|player| player.remove_rest_flag_like_cpp(rest_flag));
    if let Some(changed) = canonical {
        return changed;
    }
    #[cfg(any(test, feature = "test-fixtures"))]
    if host.test_fixtures_ref_like_cpp().is_some() && host.core().player_handle_like_cpp.is_none() {
        return host
            .mutate_player_rest_state_like_cpp(|state| state.remove_flag_like_cpp(rest_flag))
            .unwrap_or(false);
    }
    false
}

pub fn defer_player_rest_flag_sync_like_cpp<H: InstanceSessionHostLikeCpp>(host: &mut H) -> bool {
    if host
        .core()
        .with_owned_player_mut_like_cpp(|player| player.defer_rest_flag_sync_like_cpp())
        .is_some()
    {
        return true;
    }
    #[cfg(any(test, feature = "test-fixtures"))]
    if host.test_fixtures_ref_like_cpp().is_some() && host.core().player_handle_like_cpp.is_none() {
        return host
            .mutate_player_rest_state_like_cpp(|state| state.defer_flag_sync_like_cpp())
            .is_some();
    }
    false
}

pub fn end_player_rest_flag_sync_like_cpp<H: InstanceSessionHostLikeCpp>(host: &mut H) -> bool {
    let canonical = host
        .core()
        .with_owned_player_mut_like_cpp(|player| player.end_deferred_rest_flag_sync_like_cpp());
    if let Some(dirty) = canonical {
        return dirty;
    }
    #[cfg(any(test, feature = "test-fixtures"))]
    if host.test_fixtures_ref_like_cpp().is_some() && host.core().player_handle_like_cpp.is_none() {
        return host
            .mutate_player_rest_state_like_cpp(|state| state.end_deferred_flag_sync_like_cpp())
            .unwrap_or(false);
    }
    false
}

pub fn clear_player_deferred_rest_flag_update_like_cpp<H: InstanceSessionHostLikeCpp>(
    host: &mut H,
) -> bool {
    if host
        .core()
        .with_owned_player_mut_like_cpp(|player| player.clear_deferred_rest_flag_update_like_cpp())
        .is_some()
    {
        return true;
    }
    #[cfg(any(test, feature = "test-fixtures"))]
    if host.test_fixtures_ref_like_cpp().is_some() && host.core().player_handle_like_cpp.is_none() {
        return host
            .mutate_player_rest_state_like_cpp(|state| state.clear_deferred_flag_update_like_cpp())
            .is_some();
    }
    false
}

pub fn take_player_deferred_rest_flag_update_dirty_like_cpp<H: InstanceSessionHostLikeCpp>(
    host: &mut H,
) -> bool {
    let canonical = host.core().with_owned_player_mut_like_cpp(|player| {
        player.take_deferred_rest_flag_update_dirty_like_cpp()
    });
    if let Some(dirty) = canonical {
        return dirty;
    }
    #[cfg(any(test, feature = "test-fixtures"))]
    if host.test_fixtures_ref_like_cpp().is_some() && host.core().player_handle_like_cpp.is_none() {
        return host
            .mutate_player_rest_state_like_cpp(|state| state.take_deferred_flag_update_like_cpp())
            .unwrap_or(false);
    }
    false
}

pub fn load_represented_xp_rest_bonus_like_cpp<H: InstanceSessionHostLikeCpp>(
    host: &mut H,
    rest_state: u8,
    rest_bonus: f32,
) {
    // C++ `RestMgr::LoadRestBonus` restores both DB values verbatim. It does
    // not clamp or recompute the state until a later `AddRestBonus` reaches
    // `SetRestBonus` (for example when offline time is applied).
    // C++-created rows only contain the declared PlayerRestState values.
    // Normalize legacy Rust rows that persisted the old invalid value 0,
    // while preserving every valid DB state verbatim like LoadRestBonus.
    let rest_state = if wow_entities::valid_player_rest_state_like_cpp(rest_state) {
        rest_state
    } else {
        REST_STATE_NORMAL_LIKE_CPP
    };
    #[cfg(any(test, feature = "test-fixtures"))]
    if host.test_fixtures_ref_like_cpp().is_some() && host.core().player_handle_like_cpp.is_none() {
        clear_represented_rest_flags_for_character_load_like_cpp(host);
        let _ = host.mutate_player_rest_state_like_cpp(|state| {
            state.install_loaded_rest_like_cpp(rest_state, rest_bonus);
        });
        return;
    }
    let _ = host.core().with_owned_player_mut_like_cpp(|player| {
        player.load_xp_rest_bonus_like_cpp(rest_state, rest_bonus);
    });
}
#[cfg(any(test, feature = "test-fixtures"))]
fn clear_represented_rest_flags_for_character_load_like_cpp<H: InstanceSessionHostLikeCpp>(
    host: &mut H,
) {
    let loaded_resting = host
        .core()
        .canonical_player_snapshot_like_cpp(|player| player.data().player_flags)
        .is_some_and(|flags| (flags & PLAYER_FLAGS_RESTING_LIKE_CPP) != 0);
    let _canonical = with_owned_player_mut_for_rest_like_cpp(host, |player| {
        let mut state = player.rest_state_like_cpp().clone();
        state.reset_location_tracking_like_cpp();
        player.replace_rest_state_like_cpp(state);
        if loaded_resting {
            player.set_player_flag(PLAYER_FLAGS_RESTING_LIKE_CPP);
        } else {
            player.remove_player_flag(PLAYER_FLAGS_RESTING_LIKE_CPP);
        }
    });
    #[cfg(any(test, feature = "test-fixtures"))]
    if host.test_fixtures_ref_like_cpp().is_some()
        && _canonical.is_none()
        && host.core().player_handle_like_cpp.is_none()
    {
        host.hub_mut()
            .fixtures
            .progression
            .rest_mgr_test_fixture_like_cpp
            .represented_rest_flag_mask_like_cpp = 0;
        host.hub_mut()
            .fixtures
            .progression
            .rest_mgr_test_fixture_like_cpp
            .represented_rest_location_initialized_like_cpp = false;
        host.hub_mut()
            .fixtures
            .progression
            .rest_mgr_test_fixture_like_cpp
            .represented_defer_rest_flag_sync_like_cpp = false;
        host.hub_mut()
            .fixtures
            .progression
            .rest_mgr_test_fixture_like_cpp
            .represented_deferred_rest_flag_update_dirty_like_cpp = false;
        host.hub_mut()
            .fixtures
            .progression
            .rest_mgr_test_fixture_like_cpp
            .represented_inn_area_trigger_id_like_cpp = 0;
        host.hub_mut()
            .fixtures
            .progression
            .rest_mgr_test_fixture_like_cpp
            .represented_rest_time_secs_like_cpp = 0;
    }
}
