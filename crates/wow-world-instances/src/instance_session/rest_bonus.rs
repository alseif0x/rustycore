// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! C++ `RestMgr` rested-XP accrual: the offline bonus applied at login
//! (`Player::LoadFromDB` → `RestMgr::AddRestBonus`), the online update and the
//! session tick (`RestMgr::Update`) (#1363 slice 12c; moved from the World
//! `session/catalogs/operations.rs`). The former `#[cfg(test)]` handle-less
//! steps run only when `SessionHubHostLikeCpp::test_fixtures_ref_like_cpp` is
//! `Some`, through the two `InstanceSessionHostLikeCpp` rest-bonus fixture
//! accessors.

use wow_world_core::session::PlayerRestRatePolicyLikeCpp;

use super::InstanceSessionHostLikeCpp;
use super::rest_flags::with_owned_player_mut_for_rest_like_cpp;

pub const REST_OFFLINE_WILDERNESS_BUBBLE_LIKE_CPP: f32 = 0.031;
pub const REST_OFFLINE_TAVERN_OR_CITY_BUBBLE_LIKE_CPP: f32 = 0.125;
pub const REST_ONLINE_INGAME_BUBBLE_LIKE_CPP: f32 = 0.125;

pub fn apply_offline_xp_rest_bonus_with_policy_like_cpp<H: InstanceSessionHostLikeCpp>(
    host: &mut H,
    policy: &PlayerRestRatePolicyLikeCpp,
    logout_time_secs: u64,
    now_secs: u64,
    was_logout_resting: bool,
) -> f32 {
    #[cfg(any(test, feature = "test-fixtures"))]
    if host.test_fixtures_ref_like_cpp().is_some()
        && host
            .core()
            .with_owned_player_for_rest_like_cpp(|_| ())
            .is_none()
    {
        return host.fixture_apply_offline_xp_rest_bonus_like_cpp(
            policy,
            logout_time_secs,
            now_secs,
            was_logout_resting,
        );
    }
    let bubble = if was_logout_resting {
        REST_OFFLINE_TAVERN_OR_CITY_BUBBLE_LIKE_CPP * policy.offline_tavern_or_city
    } else {
        REST_OFFLINE_WILDERNESS_BUBBLE_LIKE_CPP * policy.offline_wilderness
    };
    let at_max = host.hub_ref().player_is_at_configured_max_level_like_cpp();
    let raf = host.represented_recruit_a_friend_xp_rest_state_applies_like_cpp();
    with_owned_player_mut_for_rest_like_cpp(host, |player| {
        player.apply_offline_xp_rest_bonus_like_cpp(logout_time_secs, now_secs, bubble, at_max, raf)
    })
    .unwrap_or(0.0)
}

pub fn update_represented_online_xp_rest_bonus_with_policy_like_cpp<
    H: InstanceSessionHostLikeCpp,
>(
    host: &mut H,
    policy: &PlayerRestRatePolicyLikeCpp,
    now_secs: u64,
) -> (f32, u8) {
    #[cfg(any(test, feature = "test-fixtures"))]
    if host.test_fixtures_ref_like_cpp().is_some() && host.core().player_handle_like_cpp.is_none() {
        return host.fixture_update_online_xp_rest_bonus_like_cpp(policy, now_secs);
    }
    let bubble = REST_ONLINE_INGAME_BUBBLE_LIKE_CPP * policy.ingame;
    let at_max = host.hub_ref().player_is_at_configured_max_level_like_cpp();
    let raf = host.represented_recruit_a_friend_xp_rest_state_applies_like_cpp();
    host.core()
        .with_owned_player_mut_like_cpp(|player| {
            player.update_online_xp_rest_bonus_like_cpp(now_secs, bubble, at_max, raf)
        })
        .unwrap_or((0.0, 0))
}

pub fn tick_represented_online_xp_rest_bonus_with_policy_like_cpp<H: InstanceSessionHostLikeCpp>(
    host: &mut H,
    policy: &PlayerRestRatePolicyLikeCpp,
    now_secs: u64,
) {
    // C++ `RestMgr::Update` freezes the elapsed-time update behind
    // `roll_chance_i(3)`. Use the session's runtime RNG so the gate is
    // probabilistic in production and seedable in focused tests.
    let update_roll_passed = host.core_mut().represented_urand_u32_like_cpp(1, 100) <= 3;
    tick_represented_online_xp_rest_bonus_with_roll_and_policy_like_cpp(
        host,
        policy,
        now_secs,
        update_roll_passed,
    );
}

pub fn tick_represented_online_xp_rest_bonus_with_roll_and_policy_like_cpp<
    H: InstanceSessionHostLikeCpp,
>(
    host: &mut H,
    policy: &PlayerRestRatePolicyLikeCpp,
    now_secs: u64,
    update_roll_passed: bool,
) {
    if !update_roll_passed {
        return;
    }
    let (_, nested_mask) =
        update_represented_online_xp_rest_bonus_with_policy_like_cpp(host, policy, now_secs);
    if nested_mask != 0 {
        host.hub_ref()
            .send_represented_rest_info_update_like_cpp(nested_mask);
    }
}
