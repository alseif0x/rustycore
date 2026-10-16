// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! C++ `Player::IsInAreaTriggerRadius`, the tavern area-trigger transition and
//! the movement-driven area-trigger check (#1363 slice 11b; moved from the
//! World `session/world_state/operations.rs`), and the session-tick tavern
//! revalidation (#1363 slice 12b7; moved from `session/catalogs/operations.rs`).

use tracing::info;
use wow_core::Position;
use wow_data::TavernAreaTriggerStoreLikeCpp;
use wow_world_entities::TELE_TO_NONE_LIKE_CPP;

use super::zone_area::set_represented_tavern_resting_like_cpp;
use super::{
    AreaTriggerCatalogAccessLikeCpp, InstanceSessionHostLikeCpp, REST_FLAG_IN_TAVERN_LIKE_CPP,
};
use wow_data::AreaTriggerDb2Store;

/// C++ `Player::IsInAreaTriggerRadius`.
pub fn player_is_in_area_trigger_radius_like_cpp<H: InstanceSessionHostLikeCpp>(
    host: &H,
    trigger: &wow_data::AreaTriggerDb2Entry,
) -> bool {
    let Some(pos) = host.hub_ref().player_position_like_cpp() else {
        return false;
    };

    let Some(trigger_map_id) = u16::try_from(trigger.continent_id).ok() else {
        return false;
    };
    let Some(player_phase_shift) = host.represented_player_phase_shift_like_cpp() else {
        return false;
    };
    if host.core().player_map_id_like_cpp() != trigger_map_id
        && !player_phase_shift.has_visible_map_id_like_cpp(u32::from(trigger_map_id))
    {
        return false;
    }

    if trigger.phase_id != 0 || trigger.phase_group_id != 0 || trigger.phase_use_flags != 0 {
        let (trigger_phase_shift, _) = host.hub_ref().catalogs.db_spawn_phase_shift_like_cpp(
            trigger_map_id,
            trigger.phase_use_flags as u8,
            u16::try_from(trigger.phase_id).unwrap_or_default(),
            u32::try_from(trigger.phase_group_id).unwrap_or_default(),
            -1,
        );
        if !host.can_see_phase_shift_like_cpp(&trigger_phase_shift) {
            return false;
        }
    }

    let center = Position::new(trigger.pos.x, trigger.pos.y, trigger.pos.z, trigger.box_yaw);
    if trigger.radius > 0.0 {
        pos.is_within_dist(&center, trigger.radius)
    } else {
        wow_entities::position_is_within_area_trigger_box_like_cpp(
            &pos,
            &center,
            trigger.box_length / 2.0,
            trigger.box_width / 2.0,
            trigger.box_height / 2.0,
        )
    }
}
pub fn handle_represented_tavern_area_trigger_with_catalog_like_cpp<
    H: InstanceSessionHostLikeCpp,
>(
    host: &mut H,
    taverns: &TavernAreaTriggerStoreLikeCpp,
    trigger_id: u32,
    entered: bool,
) -> bool {
    if !host
        .split_instances_ref()
        .0
        .represented_is_tavern_area_trigger_like_cpp(taverns, trigger_id)
    {
        return false;
    }

    if set_represented_tavern_resting_like_cpp(host, trigger_id, entered) {
        host.send_represented_resting_player_flag_update_like_cpp();
    }
    if host.is_ffa_pvp_realm_like_cpp() {
        // C++ `MiscHandler.cpp::HandleAreaTriggerOpcode` toggles FFA
        // directly from `packet.Entered`, independently of RestMgr's
        // aggregate mask. Leaving an inn can therefore restore FFA while
        // a city/faction rest flag still keeps PLAYER_FLAGS_RESTING set.
        wow_world_entities::set_represented_ffa_pvp_flag_like_cpp(host, !entered);
    }
    true
}
/// Check for area triggers at the player's current position.
///
/// This is called after movement updates to handle:
/// - Teleportation triggers (e.g., dungeon exits)
/// - Spell effects (e.g., silencing fields)
/// - Custom trigger actions
///
/// Manages trigger state to prevent retriggering:
/// - Entry: when player enters a trigger (was not in one)
/// - Exit: when player leaves a trigger (was in one, no longer is)
pub async fn check_area_triggers_with_catalogs_like_cpp<
    H: InstanceSessionHostLikeCpp,
    C: AreaTriggerCatalogAccessLikeCpp,
>(
    host: &mut H,
    catalogs: &C,
) {
    let Some(pos) = host.hub_ref().player_position_like_cpp() else {
        return;
    };
    let store = catalogs.destinations().as_ref();

    let (exited_trigger_id, entered_trigger) = {
        // Get all triggers at the current position on the player's current map.
        let triggers = store.get_triggers_at_position(host.core().player_map_id_like_cpp(), &pos);
        let exited_trigger_id = host
            .active_area_trigger_mut_like_cpp()
            .filter(|prev_trigger_id| {
                !triggers
                    .iter()
                    .any(|trigger| trigger.trigger_id == *prev_trigger_id)
            });
        let entered_trigger = triggers
            .first()
            .map(|trigger| (trigger.trigger_id, trigger.teleport.clone()));
        (exited_trigger_id, entered_trigger)
    };

    // Check if we've exited the previous trigger
    if let Some(prev_trigger_id) = exited_trigger_id {
        info!(
            account = host.core().account_id,
            trigger_id = prev_trigger_id,
            "Exited area trigger"
        );
        handle_represented_tavern_area_trigger_with_catalog_like_cpp(
            host,
            catalogs.taverns().as_ref(),
            prev_trigger_id,
            false,
        );
        *host.active_area_trigger_mut_like_cpp() = None;
    }

    // Check if we've entered a new trigger
    if let Some((trigger_id, teleport)) = entered_trigger {
        // Only trigger if this is a NEW trigger (wasn't active before)
        if *host.active_area_trigger_mut_like_cpp() != Some(trigger_id) {
            info!(
                account = host.core().account_id,
                trigger_id, "Entered area trigger"
            );
            *host.active_area_trigger_mut_like_cpp() = Some(trigger_id);

            if handle_represented_tavern_area_trigger_with_catalog_like_cpp(
                host,
                catalogs.taverns().as_ref(),
                trigger_id,
                true,
            ) {
                return;
            }

            // Handle teleportation if present
            if let Some(ref teleport) = teleport {
                info!(
                    account = host.core().account_id,
                    trigger_id,
                    target_map = teleport.target_map,
                    target_x = teleport.target_position.x,
                    target_y = teleport.target_position.y,
                    target_z = teleport.target_position.z,
                    "Teleporting player via area trigger"
                );
                wow_world_entities::teleport_to_with_options(
                    host,
                    teleport.target_map,
                    teleport.target_position,
                    TELE_TO_NONE_LIKE_CPP,
                )
                .await;
            }
        }
    }
}

pub fn revalidate_represented_tavern_resting_with_catalog_like_cpp<
    H: InstanceSessionHostLikeCpp,
>(
    host: &mut H,
    area_trigger_db2_store: &AreaTriggerDb2Store,
) {
    let Some(rest) = host.hub_ref().player_rest_state_snapshot_like_cpp() else {
        return;
    };
    if !rest.has_rest_flag_like_cpp(REST_FLAG_IN_TAVERN_LIKE_CPP) {
        return;
    }

    let Some(at_entry) = area_trigger_db2_store
        .get(rest.inn_trigger_id_like_cpp())
        .cloned()
    else {
        if host.remove_represented_rest_flag_like_cpp(REST_FLAG_IN_TAVERN_LIKE_CPP) {
            host.send_represented_resting_player_flag_update_like_cpp();
        }
        return;
    };

    if !player_is_in_area_trigger_radius_like_cpp(host, &at_entry)
        && host.remove_represented_rest_flag_like_cpp(REST_FLAG_IN_TAVERN_LIKE_CPP)
    {
        host.send_represented_resting_player_flag_update_like_cpp();
    }
}
