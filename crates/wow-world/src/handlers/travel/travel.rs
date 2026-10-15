// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Private travel capability handlers extracted from the legacy misc owner.
//!
//! `#1263 F5 remaining families`: the three registrations that lived here
//! (`CMSG_ACTIVATE_TAXI`, `CMSG_AREA_TRIGGER`, `CMSG_WORLD_PORT_RESPONSE`)
//! moved unchanged to the application crate's `crate::travel_handlers` area
//! registrar. `crates/wow-world/src/session/travel_handler_contexts.rs` lends
//! that registrar the same `WorldSession` operations and catalog destructuring
//! the legacy closures used.

use tracing::{debug, info, warn};
use wow_constants::{ConditionSourceType, ConditionType};

use wow_packet::ClientPacket;
use wow_packet::packets::misc::{ActivateTaxi, ActivateTaxiReply, ERR_TAXITOOFARAWAY_LIKE_CPP};

use crate::session::{AreaTriggerCatalogsLikeCpp, RepresentedActivateTaxiLikeCpp};

#[cfg(test)]
mod test_shims;

impl crate::session::WorldSession {
    /// CMSG_WORLD_PORT_RESPONSE — client confirms it has loaded the new map.
    /// The body (C++ `WorldSession::HandleMoveWorldportAck`) moved to
    /// `wow_world_character` (#1363 slice 13a); the integration tests call it
    /// as a session method.
    pub async fn handle_world_port_response_with_catalogs_like_cpp(
        &mut self,
        creature_spawn_catalogs: &crate::session::CreatureSpawnCatalogsLikeCpp,
        trait_node_entries: &wow_data::trait_tree::TraitNodeEntryStore,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        pkt: wow_packet::WorldPacket,
    ) {
        wow_world_character::handle_world_port_response_with_catalogs_like_cpp(
            self,
            creature_spawn_catalogs,
            trait_node_entries,
            item_guid_generator,
            pkt,
        )
        .await;
    }

    #[cfg(test)]
    pub(crate) async fn handle_world_port_response(&mut self, pkt: wow_packet::WorldPacket) {
        let catalogs = self.creature_spawn_catalogs_for_test_like_cpp();
        let generators = self.id_generators_for_test_like_cpp();
        self.handle_world_port_response_with_catalogs_like_cpp(
            &catalogs,
            &wow_data::trait_tree::TraitNodeEntryStore::from_entries([]),
            generators.item.as_ref(),
            pkt,
        )
        .await;
    }

    /// CMSG_AREA_TRIGGER — player entered an area trigger.
    /// C++ ref: `WorldSession::HandleAreaTriggerOpcode`.

    pub(crate) async fn handle_area_trigger_with_catalogs_like_cpp(
        &mut self,
        catalogs: &AreaTriggerCatalogsLikeCpp,
        mut pkt: wow_packet::WorldPacket,
    ) {
        let Ok(trigger_id) = pkt.read_uint32() else {
            warn!(
                account = self.core.account_id,
                "AreaTrigger packet missing trigger ID"
            );
            return;
        };
        let Ok(entered) = pkt.read_bit() else {
            warn!(
                account = self.core.account_id,
                trigger_id, "AreaTrigger packet missing Entered bit"
            );
            return;
        };
        let Ok(_from_client) = pkt.read_bit() else {
            warn!(
                account = self.core.account_id,
                trigger_id, "AreaTrigger packet missing FromClient bit"
            );
            return;
        };

        info!(
            "AreaTrigger: account {} trigger_id={} entered={}",
            self.core.account_id, trigger_id, entered
        );

        if crate::session::hub_ref(self).resolved_is_in_taxi_flight_like_cpp() != Some(false) {
            debug!(
                "Area trigger {} ignored because player is in taxi flight",
                trigger_id
            );
            return;
        }

        let Some(at_entry) = catalogs.db2.get(trigger_id).cloned() else {
            debug!("Unknown area trigger ID {}", trigger_id);
            return;
        };

        let player_in_area_trigger =
            wow_world_instances::player_is_in_area_trigger_radius_like_cpp(self, &at_entry);
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

        if !self.area_trigger_client_conditions_meet_like_cpp(trigger_id) {
            debug!("Area trigger {} rejected by C++ conditions", trigger_id);
            return;
        }

        // C++ continues unless `ScriptMgr::OnAreaTrigger` returns true. A DB
        // binding alone therefore cannot consume the event.
        let bound_script_id = catalogs
            .scripts
            .get_script_id_like_cpp(trigger_id)
            .filter(|script_id| *script_id != wow_data::ScriptIdLikeCpp::NONE);
        if let Some(script_id) = bound_script_id {
            match self.dispatch_area_trigger_script_like_cpp(
                catalogs.script_dispatcher.as_ref(),
                script_id,
                trigger_id,
                entered,
            ) {
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

        if wow_world_instances::handle_represented_tavern_area_trigger_with_catalog_like_cpp(
            self,
            catalogs.taverns.as_ref(),
            trigger_id,
            entered,
        ) {
            return;
        }

        let Some(trigger) = catalogs.destinations.get_trigger(trigger_id).cloned() else {
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
            self.teleport_to(target_map, target_pos).await;
        }
    }

    #[cfg(test)]
    pub(crate) async fn handle_area_trigger(&mut self, pkt: wow_packet::WorldPacket) {
        let catalogs = self.area_trigger_catalogs_for_test_like_cpp();
        self.handle_area_trigger_with_catalogs_like_cpp(&catalogs, pkt)
            .await;
    }

    fn area_trigger_client_conditions_meet_like_cpp(&mut self, trigger_id: u32) -> bool {
        let Some(condition_store) = self.condition_store().cloned() else {
            return true;
        };
        let Some(player_object) =
            crate::session::hub_ref(self).build_condition_player_object_like_cpp()
        else {
            return false;
        };

        let Some(player_unit_snapshot) =
            crate::session::hub_ref(self).condition_player_unit_snapshot_like_cpp()
        else {
            return false;
        };
        let player_snapshot = crate::session::hub_ref(self).condition_player_snapshot_like_cpp();
        let area_table_store = self.catalogs.area_table_store().cloned();

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
                        area_table_store.as_ref().is_some_and(|store| {
                            store.is_in_area_like_cpp(current_area, required_area)
                        })
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

    /// CMSG_ACTIVATE_TAXI.
    ///
    /// C++ resolves `GetNPCIfCanInteractWith(Vendor, UNIT_NPC_FLAG_FLIGHTMASTER)`,
    /// sends `ERR_TAXITOOFARAWAY` when that fails, then checks nearest taxi
    /// node, known taximask nodes, preferred mount display, `TaxiPathGraph`,
    /// and `Player::ActivateTaxiPathTo`.
    ///
    /// Rust currently has represented NPC interaction and mount display filters,
    /// but not `TaxiNodes.db2`, `TaxiPathGraph`, or live MotionMaster taxi
    /// flight. This handler preserves packet/dispatch and the first C++ failure
    /// reply, then records the accepted request for the future taxi runtime.
    pub(crate) async fn handle_activate_taxi(&mut self, mut pkt: wow_packet::WorldPacket) {
        let activate = match ActivateTaxi::read(&mut pkt) {
            Ok(activate) => activate,
            Err(error) => {
                warn!("Bad ActivateTaxi: {error}");
                return;
            }
        };

        const NPC_FLAG_FLIGHT_MASTER: u32 = 0x2000;
        let can_interact = crate::session::hub_ref(self)
            .represented_npc_can_interact_with_like_cpp(activate.vendor, NPC_FLAG_FLIGHT_MASTER, 0)
            .is_some()
            || self
                .core
                .read_world_creature_like_cpp(activate.vendor, |creature| {
                    creature.npc_flags() & NPC_FLAG_FLIGHT_MASTER != 0
                })
                .unwrap_or(false);

        if !can_interact {
            self.send_packet(&ActivateTaxiReply {
                reply: ERR_TAXITOOFARAWAY_LIKE_CPP,
            });
            return;
        }

        let preferred_mount_display = self
            .represented_taxi_usable_mount_displays_like_cpp(activate.flying_mount_id)
            .into_iter()
            .find_map(|display| u32::try_from(display).ok())
            .unwrap_or_default();

        crate::session::hub_mut(self).record_represented_activate_taxi_like_cpp(
            RepresentedActivateTaxiLikeCpp {
                vendor: activate.vendor,
                node: activate.node,
                ground_mount_id: activate.ground_mount_id,
                flying_mount_id: activate.flying_mount_id,
                preferred_mount_display,
            },
        );
    }
}
