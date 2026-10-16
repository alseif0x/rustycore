// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The session init packets (#1363 slice 12b7), moved from the World
//! `session/catalogs/operations.rs`. C++ `WorldSession::InitializeSessionCallback`;
//! the packet order is unchanged. The World session keeps the method of the
//! same name as the `world-server` entry point, naming this function.

use tracing::info;
use wow_core::ObjectGuid;
use wow_data::HotfixBlobCache;
use wow_world_core::session::SupportFeaturePolicyLikeCpp;
use wow_world_core::session::character_availability::default_available_classes;
use wow_world_core::session::connection_identity::unix_now;
use wow_world_entities::VisibilitySessionHostLikeCpp;
use wow_world_lifecycle::GLOBAL_CACHE_MASK_LIKE_CPP;

/// Send session initialization packets (first encrypted packets after
/// EnterEncryptedModeAck). Matches C++ `WorldSession::InitializeSessionCallback`.
///
/// These packets are sent immediately when the session starts, before any
/// client packets are processed. They tell the client that auth succeeded
/// and provide the initial glue screen data (character select).
///
/// Exact C++ order:
/// 1. AuthResponse
/// 2. SetTimeZoneInformation
/// 3. FeatureSystemStatusGlueScreen (NOT the in-game FeatureSystemStatus!)
/// 4. ClientCacheVersion
/// 5. AvailableHotfixes
/// 6. AccountDataTimes (global)
/// 7. TutorialFlags
/// 8. ConnectionStatus (State=1)
pub fn send_session_init_packets_with_policy_like_cpp<H: VisibilitySessionHostLikeCpp>(
    host: &H,
    policy: &SupportFeaturePolicyLikeCpp,
    hotfixes: &HotfixBlobCache,
) {
    use wow_packet::packets::auth::*;
    use wow_packet::packets::misc::*;

    let vra = host.core().virtual_realm_address();
    let (realm_name_actual, realm_name_normalized) = host
        .core()
        .realm_names_for_address_like_cpp(vra)
        .unwrap_or(("RustyCore", "RustyCore"));

    // 1. AuthResponse (OK) — tells the client authentication succeeded
    let auth_response = AuthResponse {
        result: 0, // OK
        success_info: Some(AuthSuccessInfo {
            virtual_realm_address: vra,
            virtual_realms: vec![VirtualRealmInfo {
                realm_address: vra,
                is_local: true,
                is_internal_realm: false,
                realm_name_actual: realm_name_actual.to_string(),
                realm_name_normalized: realm_name_normalized.to_string(),
            }],
            time_rested: 0,
            active_expansion_level: host.core().expansion,
            account_expansion_level: host.core().account_expansion,
            time_seconds_until_pc_kick: 0,
            available_classes: default_available_classes(),
            templates: vec![],
            currency_id: 0,
            time: unix_now(),
            game_time_info: GameTimeInfo {
                billing_plan: 0,
                time_remain: 0,
                unknown735: 0,
                in_game_room: false,
            },
            is_expansion_trial: false,
            force_character_template: false,
            num_players_horde: None,
            num_players_alliance: None,
            expansion_trial_expiration: None,
        }),
        wait_info: None,
    };
    host.send_packet(&auth_response);

    // 2. SetTimeZoneInformation
    host.send_packet(&SetTimeZoneInformation::utc());

    // 3. FeatureSystemStatusGlueScreen (character select version, NOT in-game)
    host.send_packet(
        &host
            .core()
            .feature_system_status_glue_screen_with_policy_like_cpp(policy),
    );

    // 4. ClientCacheVersion (from world DB version.cache_id = 24081)
    host.send_packet(&ClientCacheVersion {
        cache_version: 24081,
    });

    let hotfixes = hotfixes
        .available_hotfix_ids(&host.core().locale)
        .into_iter()
        .map(|id| HotfixId {
            push_id: id.push_id,
            unique_id: id.unique_id,
        })
        .collect();

    // 5. AvailableHotfixes
    host.send_packet(&AvailableHotfixes {
        virtual_realm_address: vra,
        hotfixes,
    });

    // 6. AccountDataTimes (global)
    host.send_packet(
        &host
            .lifecycle()
            .account_data_times_like_cpp(ObjectGuid::EMPTY, GLOBAL_CACHE_MASK_LIKE_CPP),
    );

    // 7. TutorialFlags
    host.send_packet(&host.lifecycle().tutorial_flags_packet_like_cpp());

    // 8. ConnectionStatus (State=1, SuppressNotification=true)
    // This compatibility packet has no ConnectionType override,
    // so it's sent on the realm socket. State uses 2 bits, SuppressNotification
    // defaults to true.
    host.send_packet(&ConnectionStatus {
        state: 1,
        suppress_notification: true,
    });

    info!(
        "Session init packets sent for account {} (8 packets: AuthResponse → ConnectionStatus)",
        host.core().account_id
    );
}
