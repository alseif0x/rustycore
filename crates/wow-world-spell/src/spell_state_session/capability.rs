// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The Player's persistent capability state (C++ `Player::m_atLoginFlags`
//! with the weapon and armor proficiency masks): the snapshot and the
//! at-login flag set, remove and read (#1363 slice 12d; moved from the World
//! `session/persistence/plans.rs`, `session/lifecycle_ops/operations.rs` and
//! `session/movement/state.rs`). Each former `#[cfg(test)]` fixture step runs
//! only when `test_fixtures_ref_like_cpp` is `Some` (the World lib test build).

#[cfg(any(test, feature = "test-fixtures"))]
use wow_world_lifecycle::RepresentedAtLoginFlagRemovalLikeCpp;

use crate::SpellExecutionHostLikeCpp;

pub fn player_persistent_capability_state_snapshot_like_cpp<
    H: SpellExecutionHostLikeCpp + ?Sized,
>(
    host: &H,
) -> Option<wow_entities::PlayerPersistentCapabilityStateLikeCpp> {
    let canonical = host
        .core()
        .with_owned_player_like_cpp(|player| player.persistent_capability_state_like_cpp());
    #[cfg(any(test, feature = "test-fixtures"))]
    if host.test_fixtures_ref_like_cpp().is_some()
        && canonical.is_none()
        && host.core().player_handle_like_cpp.is_none()
    {
        return Some(wow_entities::PlayerPersistentCapabilityStateLikeCpp {
            at_login_flags: host
                .lifecycle()
                .represented_at_login_flags_for_test_like_cpp(),
            weapon_proficiency: host
                .hub_ref()
                .fixtures
                .progression
                .represented_weapon_proficiency_like_cpp,
            armor_proficiency: host
                .hub_ref()
                .fixtures
                .progression
                .represented_armor_proficiency_like_cpp,
        });
    }
    canonical
}

#[cfg_attr(not(any(test, feature = "test-fixtures")), allow(unused_variables))]
pub fn set_represented_at_login_flags_like_cpp<H: SpellExecutionHostLikeCpp + ?Sized>(
    host: &mut H,
    flags: u16,
) -> bool {
    let canonical = host
        .core()
        .with_owned_player_mut_like_cpp(|player| player.set_at_login_flags_like_cpp(flags))
        .is_some();
    #[cfg(any(test, feature = "test-fixtures"))]
    if host.test_fixtures_ref_like_cpp().is_some()
        && !canonical
        && host.core().player_handle_like_cpp.is_none()
    {
        return host
            .mutate_player_persistent_capability_state_like_cpp(|state| {
                state.at_login_flags = flags;
            })
            .is_some();
    }
    canonical
}

pub fn remove_represented_at_login_flag_like_cpp<H: SpellExecutionHostLikeCpp + ?Sized>(
    host: &mut H,
    flags: u16,
    persist: bool,
) -> bool {
    #[cfg(any(test, feature = "test-fixtures"))]
    let world_test = host.test_fixtures_ref_like_cpp().is_some();
    let canonical = host
        .core()
        .with_owned_player_mut_like_cpp(|player| player.remove_at_login_flags_like_cpp(flags));
    #[cfg(any(test, feature = "test-fixtures"))]
    let removed = canonical.or_else(|| {
        (world_test && host.core().player_handle_like_cpp.is_none()).then(|| {
            host.mutate_player_persistent_capability_state_like_cpp(|state| {
                let removed = (state.at_login_flags & flags) != 0;
                state.at_login_flags &= !flags;
                removed
            })
            .expect("handle-less fixture capability owner")
        })
    });
    #[cfg(not(any(test, feature = "test-fixtures")))]
    let removed = canonical;
    let Some(removed) = removed else {
        return false;
    };
    if !removed {
        return false;
    }
    if persist {
        #[cfg(any(test, feature = "test-fixtures"))]
        if world_test {
            host.split_lifecycle_mut()
                .0
                .record_at_login_flag_removal_for_test_like_cpp(
                    RepresentedAtLoginFlagRemovalLikeCpp {
                        flags,
                        persist,
                        db_statement_unrepresented: true,
                    },
                );
        }
    }
    true
}

pub fn resolved_represented_at_login_flags_like_cpp<H: SpellExecutionHostLikeCpp + ?Sized>(
    host: &H,
) -> Option<u16> {
    player_persistent_capability_state_snapshot_like_cpp(host).map(|state| state.at_login_flags)
}
