// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Login, world entry, logout and the client-state handshake.

use crate::CharacterLoginHostLikeCpp;
use tracing::info;
use wow_core::{ObjectGuid, Position};
use wow_entities::{
    BANK_SLOT_BAG_END, BANK_SLOT_BAG_START, INVENTORY_SLOT_BAG_END, INVENTORY_SLOT_BAG_START,
    REAGENT_BAG_SLOT_END, REAGENT_BAG_SLOT_START,
};
use wow_packet::packets::character::LoginVerifyWorld;
use wow_packet::packets::chat::ChatServerMessage;
use wow_packet::packets::loot::LootReleaseAll;
use wow_packet::packets::misc::{
    AccountMount, AccountMountUpdate, LogoutRequest, LogoutResponse, SetTimeZoneInformation,
};
use wow_packet::packets::update::PlayerCombatStats;
use wow_world_core::session::{SessionState, SupportFeaturePolicyLikeCpp};
use wow_world_lifecycle::{
    ALL_ACCOUNT_DATA_CACHE_MASK_LIKE_CPP, FinalizationDisposition, FinalizationMode,
    GLOBAL_CACHE_MASK_LIKE_CPP,
};

pub(super) fn is_represented_bag_slot(slot: u8) -> bool {
    (INVENTORY_SLOT_BAG_START..INVENTORY_SLOT_BAG_END).contains(&slot)
        || (BANK_SLOT_BAG_START..BANK_SLOT_BAG_END).contains(&slot)
        || (REAGENT_BAG_SLOT_START..REAGENT_BAG_SLOT_END).contains(&slot)
}

/// Fallback: skip ConnectTo and trigger direct login on the realm socket.
///
/// Used when no session manager is configured or all ConnectTo retries fail.
/// Sets a flag so that `process_pending` will call `handle_continue_player_login`.
pub fn fallback_direct_login<H: CharacterLoginHostLikeCpp>(host: &mut H) {
    // player_loading is already set — create a dummy oneshot that fires immediately
    let (tx, rx) = tokio::sync::oneshot::channel();
    let link = wow_network::session_mgr::InstanceLink {
        send_tx: host.core().send_tx().clone(),
        send_write_fence_like_cpp: None,
        pkt_rx: None, // None = keep using realm socket's packet_rx
    };
    let _ = tx.send(link);
    host.core_mut().set_instance_link_rx(Some(rx));
    info!(
        "Fallback: direct login scheduled for account {}",
        host.core().account_id
    );
}

/// Handle CMSG_SERVER_TIME_OFFSET_REQUEST — respond with current realm time.
/// Handle CMSG_TIME_SYNC_RESPONSE — client's response to our TimeSyncRequest.
///
/// We acknowledge the response to keep the client's time sync state healthy.
/// The periodic timer in `update()` handles sending the next request.
/// C++ `HandleLogoutRequestOpcode` `LogoutResult` (`MiscHandler.cpp:251-258`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum LogoutDenialLikeCpp {
    InCombat = 1,
    /// C++: "FIXME - Need the correct value".
    DuelingOrFrozen = 2,
    Falling = 3,
}

/// C++ `HandleLogoutRequestOpcode` admission (`MiscHandler.cpp:243-258`).
/// Returns the `LogoutResult` (0 = admitted) and `Instant`.
pub fn logout_admission_like_cpp(
    snapshot: &wow_world_core::session::LogoutAdmissionSnapshotLikeCpp,
    idle_logout: bool,
    has_instant_logout_permission: bool,
) -> (Option<LogoutDenialLikeCpp>, bool) {
    let mut instant_logout = snapshot.in_flight;
    if !idle_logout {
        instant_logout |=
            (snapshot.resting && !snapshot.in_combat) || has_instant_logout_permission;
    }
    // TODO in C++: "Possibly add RBAC permission to log out in combat".
    let can_logout_in_combat = snapshot.resting;
    let reason = if snapshot.in_combat && !can_logout_in_combat {
        Some(LogoutDenialLikeCpp::InCombat)
    } else if snapshot.falling {
        Some(LogoutDenialLikeCpp::Falling)
    } else if snapshot.dueling_or_frozen {
        Some(LogoutDenialLikeCpp::DuelingOrFrozen)
    } else {
        None
    };
    (reason, instant_logout)
}

/// Handle CMSG_LOGOUT_REQUEST — C++ `WorldSession::HandleLogoutRequestOpcode`
/// (`MiscHandler.cpp:238-287`): release loot, decide denial/instant, answer
/// `SMSG_LOGOUT_RESPONSE`, then either `LogoutPlayer(true)` at once or start
/// the 20 s countdown that `WorldSession::Update` completes
/// (`WorldSession.cpp:505-511`).
pub async fn handle_logout_request_with_generator_like_cpp<H: CharacterLoginHostLikeCpp>(
    host: &mut H,
    item_guid_generator: &wow_core::ObjectGuidGenerator,
    req: LogoutRequest,
) {
    if host.state() == SessionState::Disconnecting {
        return;
    }
    info!(
        "LogoutRequest (idle={}) from account {}",
        req.idle_logout,
        host.core().account_id
    );

    if !host.loot().active_loot_guid_like_cpp().is_empty() {
        host.send_packet(&LootReleaseAll);
    }

    // C++ dereferences `GetPlayer()` here; a LoggedIn session always has one.
    // Without a canonical Player nothing can be admitted or rooted, so the
    // represented session keeps its former instant finalization.
    let (reason, instant_logout) = match host.hub_ref().logout_admission_snapshot_like_cpp() {
        Some(snapshot) => logout_admission_like_cpp(
            &snapshot,
            req.idle_logout,
            host.hub_ref().has_instant_logout_permission_like_cpp(),
        ),
        None => (None, true),
    };

    host.send_packet(&LogoutResponse {
        logout_result: reason.map_or(0, |reason| reason as i32),
        instant: instant_logout,
    });

    if reason.is_some() {
        host.lifecycle_mut().clear_logout_time();
        if !host.lifecycle_mut().player_logout_like_cpp() {
            host.core().mark_player_logging_out_like_cpp(false);
        }
        return;
    }

    // Instant logout in taverns/cities, on taxi, or with the permission.
    if instant_logout {
        let report = host
            .finalize_session_with_generator_like_cpp(
                FinalizationMode::CharacterSelection,
                item_guid_generator,
            )
            .await;
        if report.disposition == FinalizationDisposition::Complete {
            info!("Player logged out for account {}", host.core().account_id);
        }
        return;
    }

    host.begin_logout_wait_movement_like_cpp();
    host.lifecycle_mut()
        .set_logout_start_time_like_cpp(wow_entities::game_time_secs_like_cpp());
    // `isLogingOut()` is now true for other sessions too (trade admission).
    host.core().mark_player_logging_out_like_cpp(true);
}

/// Handle CMSG_LOGOUT_CANCEL — player cancels a pending logout.
/// Build the self CreateObject combat snapshot after C++ login has loaded
/// persisted auras and applied all equipped-item modifiers.
pub fn player_login_combat_stats_like_cpp<H: CharacterLoginHostLikeCpp>(
    host: &mut H,
    race: u8,
    class: u8,
    level: u8,
    saved_health: Option<u32>,
    saved_power0: i32,
) -> Option<(PlayerCombatStats, i32, i32)> {
    host.stats_application_cx_like_cpp()
        .player_login_combat_stats_like_cpp(race, class, level, saved_health, saved_power0)
}

/// C++ `WorldSession::HandlePlayerLogin` packet prelude through
/// `BattlePetMgr::SendJournalLockStatus`.
pub async fn send_handle_player_login_packets_like_cpp<H: CharacterLoginHostLikeCpp>(
    host: &mut H,
    item_guid_generator: &wow_core::ObjectGuidGenerator,
    feature_policy: &SupportFeaturePolicyLikeCpp,
    guid: ObjectGuid,
    position: &Position,
    map_id: i32,
    account_mounts: &[AccountMount],
    motd: &str,
) -> bool {
    // C++ `Player::LoadFromDB -> CollectionMgr::LoadMounts -> AddMount`
    // publishes one partial update for every usable account mount before
    // `HandlePlayerLogin` begins its explicit packet burst.
    for mount in account_mounts {
        host.send_packet(&AccountMountUpdate::partial(vec![*mount]));
    }
    // `LoadFromDB` may already have published proficiency/aura packets on
    // the instance writer even when this account has no partial mounts.
    // Drain that complete prefix before starting the realm-routed login
    // burst so the two physical sockets retain C++ call order.
    if !host
        .core()
        .wait_for_instance_send_before_realm_send_like_cpp()
        .await
    {
        return false;
    }

    // C++ resends both account-scoped datasets here even though they were
    // already sent by `InitializeSessionCallback` on the glue screen.
    host.core().send_packet_realm(
        &host
            .lifecycle()
            .account_data_times_like_cpp(ObjectGuid::EMPTY, GLOBAL_CACHE_MASK_LIKE_CPP),
    );
    host.core()
        .send_packet_realm(&host.lifecycle().tutorial_flags_packet_like_cpp());

    let Some(dungeon_difficulty) = host.represented_dungeon_difficulty_packet_like_cpp() else {
        return false;
    };
    host.core().send_packet_realm(&dungeon_difficulty);
    if !host
        .core()
        .wait_for_realm_send_before_instance_update_like_cpp()
        .await
    {
        return false;
    }
    host.send_packet(&LoginVerifyWorld {
        map_id,
        position: *position,
        reason: 0,
    });
    if !host
        .core()
        .wait_for_instance_send_before_realm_send_like_cpp()
        .await
    {
        return false;
    }
    host.core().send_packet_realm(
        &host
            .lifecycle()
            .account_data_times_like_cpp(guid, ALL_ACCOUNT_DATA_CACHE_MASK_LIKE_CPP),
    );
    host.core().send_packet_realm(
        &host
            .core()
            .feature_system_status_with_policy_like_cpp(feature_policy),
    );

    for motd_line in motd_lines_like_cpp(motd) {
        host.core().send_packet_realm(&ChatServerMessage {
            message_id: 3,
            string_param: motd_line,
        });
    }

    host.core()
        .send_packet_realm(&SetTimeZoneInformation::utc());

    // Issue #161: converge interrupted battle-pet trainer purchases
    // before the journal lock and before the client can interact; any
    // recovery publication lands inside this login burst. The recovery
    // writes instance-socket packets between Realm-socket neighbours, so
    // it is bracketed by the same cross-socket ordering fences the rest
    // of the burst already uses.
    if !host
        .core()
        .wait_for_realm_send_before_instance_update_like_cpp()
        .await
    {
        return false;
    }
    wow_world_inventory::recover_battle_pet_trainer_purchases_with_generator_like_cpp(
        host,
        item_guid_generator,
    )
    .await;
    if !host
        .core()
        .wait_for_instance_send_before_realm_send_like_cpp()
        .await
    {
        return false;
    }

    // C++ sends the journal lock before
    // `Player::SendInitialPacketsBeforeAddToMap`.
    host.send_battle_pet_journal_lock_status_like_cpp().await;
    host.core()
        .wait_for_realm_send_before_instance_update_like_cpp()
        .await
}

pub fn motd_lines_like_cpp(motd: &str) -> Vec<String> {
    // C++ `World::SetMotd` uses `boost::split` on `@` with token compression
    // disabled, so empty and trailing lines remain part of the login burst.
    motd.split('@').map(ToOwned::to_owned).collect()
}
