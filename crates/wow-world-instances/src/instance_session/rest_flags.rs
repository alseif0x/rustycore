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

use wow_entities::Player;
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
