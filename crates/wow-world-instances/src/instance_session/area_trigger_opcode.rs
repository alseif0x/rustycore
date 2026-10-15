// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! CMSG_AREA_TRIGGER — player entered an area trigger.
//! C++ ref: `WorldSession::HandleAreaTriggerOpcode` (`MiscHandler.cpp:478`).
//!
//! Moved verbatim from the World `handlers/travel/travel.rs` with its client
//! condition check (#1363 slice 4d). The `ScriptMgr::OnAreaTrigger` dispatcher
//! stays a World type (it receives the mutable World session): the catalogs
//! cross as the movement host's existing `AreaTriggerCatalogsLikeCpp`
//! associated type, and the one host method below invokes the dispatcher
//! exactly as the session did.

use tracing::{debug, info, warn};
use wow_constants::{ConditionSourceType, ConditionType};
use wow_world_entities::TELE_TO_NONE_LIKE_CPP;

use super::{AreaTriggerCatalogAccessLikeCpp, InstanceSessionHostLikeCpp};

/// The World operations the area-trigger opcode adds to the instance host.
pub trait AreaTriggerHostLikeCpp:
    InstanceSessionHostLikeCpp<AreaTriggerCatalogsLikeCpp: AreaTriggerCatalogAccessLikeCpp>
{
    /// `WorldSession::dispatch_area_trigger_script_like_cpp` over `script_dispatcher`.
    fn dispatch_area_trigger_script_like_cpp(
        &mut self,
        catalogs: &Self::AreaTriggerCatalogsLikeCpp,
        script_id: wow_data::ScriptIdLikeCpp,
        trigger_id: u32,
        entered: bool,
    ) -> Option<bool>;
}

pub async fn handle_area_trigger_with_catalogs_like_cpp<H: AreaTriggerHostLikeCpp>(
    host: &mut H,
    catalogs: &H::AreaTriggerCatalogsLikeCpp,
    mut pkt: wow_packet::WorldPacket,
) {
    let Ok(trigger_id) = pkt.read_uint32() else {
        warn!(
            account = host.core().account_id,
            "AreaTrigger packet missing trigger ID"
        );
        return;
    };
    let Ok(entered) = pkt.read_bit() else {
        warn!(
            account = host.core().account_id,
            trigger_id, "AreaTrigger packet missing Entered bit"
        );
        return;
    };
    let Ok(_from_client) = pkt.read_bit() else {
        warn!(
            account = host.core().account_id,
            trigger_id, "AreaTrigger packet missing FromClient bit"
        );
        return;
    };

    info!(
        "AreaTrigger: account {} trigger_id={} entered={}",
        host.core().account_id,
        trigger_id,
        entered
    );

    if host.hub_ref().resolved_is_in_taxi_flight_like_cpp() != Some(false) {
        debug!(
            "Area trigger {} ignored because player is in taxi flight",
            trigger_id
        );
        return;
    }

    let Some(at_entry) = catalogs.db2().get(trigger_id).cloned() else {
        debug!("Unknown area trigger ID {}", trigger_id);
        return;
    };

    let player_in_area_trigger = crate::player_is_in_area_trigger_radius_like_cpp(host, &at_entry);
    // Legacy1 validates radius only for an enter notification and is the
    // selected parity behavior. Legacy2 instead requires `entered` to
    // equal the current inside/outside result, so it rejects a leave that
    // arrives while the player is still inside. Keep the disagreement
    // explicit; a 3.4.3 client capture is still needed to adjudicate it.
    if entered && !player_in_area_trigger {
        debug!(
            "Area trigger {} ignored because player is too far",
            trigger_id
        );
        return;
    }

    if !area_trigger_client_conditions_meet_like_cpp(host, trigger_id) {
        debug!("Area trigger {} rejected by C++ conditions", trigger_id);
        return;
    }

    // C++ continues unless `ScriptMgr::OnAreaTrigger` returns true. A DB
    // binding alone therefore cannot consume the event.
    let bound_script_id = catalogs
        .scripts()
        .get_script_id_like_cpp(trigger_id)
        .filter(|script_id| *script_id != wow_data::ScriptIdLikeCpp::NONE);
    if let Some(script_id) = bound_script_id {
        match host.dispatch_area_trigger_script_like_cpp(catalogs, script_id, trigger_id, entered) {
            Some(true) => return,
            Some(false) => {}
            None => warn!(
                trigger_id,
                entered,
                ?script_id,
                "Area trigger script dispatch is unrepresented; preserving prior continuation"
            ),
        }
    }

    if crate::handle_represented_tavern_area_trigger_with_catalog_like_cpp(
        host,
        catalogs.taverns().as_ref(),
        trigger_id,
        entered,
    ) {
        return;
    }

    let Some(trigger) = catalogs.destinations().get_trigger(trigger_id).cloned() else {
        return;
    };

    // Lookup in represented teleport store
    info!(
        "AreaTrigger {} detected at map {} pos ({}, {}, {})",
        trigger_id, trigger.map_id, trigger.pos.x, trigger.pos.y, trigger.pos.z
    );

    if !entered {
        return;
    }

    if let Some(ref teleport) = trigger.teleport {
        let target_map = teleport.target_map;
        let target_pos = teleport.target_position;
        info!(
            "AreaTrigger {} → teleport to map {} ({:.2}, {:.2}, {:.2})",
            trigger_id, target_map, target_pos.x, target_pos.y, target_pos.z
        );
        wow_world_entities::teleport_to_with_options(
            host,
            target_map,
            target_pos,
            TELE_TO_NONE_LIKE_CPP,
        )
        .await;
    }
}

fn area_trigger_client_conditions_meet_like_cpp<H: AreaTriggerHostLikeCpp>(
    host: &mut H,
    trigger_id: u32,
) -> bool {
    let Some(condition_store) = host.hub_ref().catalogs.condition_store().cloned() else {
        return true;
    };
    let Some(player_object) = host.hub_ref().build_condition_player_object_like_cpp() else {
        return false;
    };

    let Some(player_unit_snapshot) = host.hub_ref().condition_player_unit_snapshot_like_cpp()
    else {
        return false;
    };
    let player_snapshot = host.hub_ref().condition_player_snapshot_like_cpp();
    let area_table_store = host.hub_ref().catalogs.area_table_store().cloned();

    let mut source_info =
        wow_conditions::ConditionSourceInfo::from_targets(Some(&player_object), None, None);
    source_info.set_unit_target_snapshot(0, player_unit_snapshot);
    source_info.set_player_target_snapshot(0, player_snapshot);

    wow_conditions::is_object_meeting_not_grouped_conditions_like_cpp(
        condition_store.as_ref(),
        ConditionSourceType::AreaTriggerClientTriggered,
        trigger_id,
        &mut source_info,
        |condition, source_info| {
            // C++ combines the base condition with
            // `ScriptMgr::OnConditionCheck`. Rust does not yet have a
            // ConditionScript dispatcher, so allowing a scripted row
            // through would silently bypass its only custom predicate.
            if condition.script_id != 0 {
                warn!(
                    trigger_id,
                    script_id = condition.script_id,
                    "Area trigger ConditionScript dispatch is unrepresented; failing closed"
                );
                return false;
            }

            let context_is_represented = match condition.condition_type {
                ConditionType::None
                | ConditionType::MapId
                | ConditionType::ZoneId
                | ConditionType::Class
                | ConditionType::Team
                | ConditionType::Race
                | ConditionType::Gender
                | ConditionType::Level
                | ConditionType::Alive
                | ConditionType::HpVal
                | ConditionType::HpPct
                | ConditionType::Taxi
                | ConditionType::ObjectEntryGuid
                | ConditionType::ObjectEntryGuidLegacy
                | ConditionType::TypeMask
                | ConditionType::TypeMaskLegacy => true,
                ConditionType::AreaId => area_table_store.is_some(),
                _ => false,
            };
            if !context_is_represented {
                warn!(
                    trigger_id,
                    condition_type = ?condition.condition_type,
                    "Area trigger condition context is unrepresented; failing closed"
                );
                return false;
            }

            match wow_conditions::condition_meets_basic_like_cpp(
                condition,
                source_info,
                |current_area, required_area| {
                    area_table_store
                        .as_ref()
                        .is_some_and(|store| store.is_in_area_like_cpp(current_area, required_area))
                },
            ) {
                wow_conditions::ConditionMeetResult::Evaluated(value) => value,
                wow_conditions::ConditionMeetResult::Unsupported => {
                    warn!(
                        trigger_id,
                        condition_type = ?condition.condition_type,
                        "Area trigger condition evaluation is unrepresented; failing closed"
                    );
                    false
                }
            }
        },
    )
}
