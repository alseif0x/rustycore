// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Private travel capability handlers extracted from the legacy misc owner.

use tracing::{debug, info, warn};
use wow_constants::{ClientOpcodes, ConditionSourceType, ConditionType};
use wow_core::ObjectGuid;
use wow_handler::{PacketProcessing, SessionStatus};

use crate::session::registry::PacketHandlerEntry;
use wow_packet::ClientPacket;
use wow_packet::packets::misc::{
    ActivateTaxi, ActivateTaxiReply, ERR_TAXITOOFARAWAY_LIKE_CPP, SetTaxiBenchmarkMode,
    TaxiNodeStatusPkt,
};

use crate::session::RepresentedActivateTaxiLikeCpp;

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ActivateTaxi,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadSafe,
        handler_name: "handle_activate_taxi",
        handler: |session, pkt| Box::pin(async move { session.handle_activate_taxi(pkt).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::AreaTrigger,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_area_trigger",
        handler: |session, pkt| Box::pin(async move { session.handle_area_trigger(pkt).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::WorldPortResponse,
        status: SessionStatus::Transfer,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_world_port_response",
        handler: |session, pkt| {
            Box::pin(async move { session.handle_world_port_response(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SuspendTokenResponse,
        status: SessionStatus::Transfer,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_suspend_token_response",
        handler: |session, pkt| {
            Box::pin(async move { session.handle_suspend_token_response(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::TaxiNodeStatusQuery,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadSafe,
        handler_name: "handle_taxi_node_status_query",
        handler: |session, pkt| {
            Box::pin(async move { session.handle_taxi_node_status_query(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SetTaxiBenchmarkMode,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_set_taxi_benchmark_mode",
        handler: |session, pkt| {
            Box::pin(async move { session.handle_set_taxi_benchmark_mode(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::UpdateAreaTriggerVisual,
        status: SessionStatus::Authed,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_update_area_trigger_visual",
        handler: |session, pkt| {
            Box::pin(async move { session.handle_update_area_trigger_visual(pkt).await })
        },
    }
}

impl crate::session::WorldSession {
    /// C++ `Map::SendInitSelf` (Map.cpp:1877), invoked by `Map::AddPlayerToMap(initPlayer=true)`
    /// on a non-seamless far teleport (HandleMoveWorldportAck -> AddPlayerToMap, Map.cpp:470).
    /// Re-sends the player's OWN object (ActivePlayer create block) so the client finishes the
    /// loading screen and enters the destination map. Sourced from session state; combat stats
    /// are placeholders here (health from the live value, the rest defaulted) and corrected by
    /// the `send_stat_update` that follows. Inventory item objects are not yet re-sent on
    /// teleport (the client retains them from login) — a #NEXT.R8.ENTITIES.1229 follow-up.
    pub(super) async fn send_player_self_create_for_teleport_like_cpp(&mut self) {
        use wow_core::guid::HighGuid;
        use wow_packet::packets::update::{PlayerCombatStats, UpdateObject};

        let Some(guid) = self.player_guid() else {
            return;
        };
        let Some(pos) = self.player_position_like_cpp() else {
            return;
        };
        let map_id = self.player_map_id_like_cpp();
        let (zone_id, _area_id) = self.player_zone_area_like_cpp();
        let race = self.player_race_like_cpp();
        let class = self.player_class_like_cpp();
        let gender = self.player_gender_like_cpp();
        let level = self.player_level_like_cpp();
        let (Some(player_xp), Some(player_next_level_xp), Some(scaling_level_delta)) = (
            self.resolved_player_xp_like_cpp(),
            self.resolved_player_next_level_xp_like_cpp(),
            self.resolved_player_scaling_level_delta_like_cpp(),
        ) else {
            return;
        };
        let Some(player_money) = self.resolved_player_money_like_cpp() else {
            return;
        };

        // Equipped items drive the visible model; bag slots / item objects are not re-sent here.
        let mut visible_items = [(0i32, 0u16, 0u16); 19];
        let Some(inventory_items) = self.resolved_inventory_items_like_cpp() else {
            return;
        };
        for (slot, item) in inventory_items {
            if (slot as usize) < 19 {
                visible_items[slot as usize] = (item.entry_id as i32, 0, 0);
            }
        }

        let Some((health, _, _)) = self.resolved_player_vitals_like_cpp() else {
            return;
        };
        let health = health.max(1);
        let combat = PlayerCombatStats {
            health: i64::from(health),
            max_health: i64::from(health),
            ..PlayerCombatStats::default()
        };

        let quest_log = self.quest_log_create_entries_like_cpp();
        let account_toys = self.account_toy_active_player_rows_like_cpp();
        let account_heirlooms = self.account_heirloom_active_player_rows_like_cpp();
        let account_transmog = self.account_transmog_active_player_rows_like_cpp();
        let trait_configs = self.load_active_player_trait_configs_like_cpp(guid).await;
        let player_customizations = self.load_player_customizations_like_cpp(guid).await;
        let party_type = self.party_member_party_type_like_cpp();
        let display_id = crate::handlers::character::default_display_id(race, gender);

        // Rebuild the active SkillInfo rows from the canonical login skill
        // records. This preserves persisted/default values across far
        // teleports instead of re-running LearnDefaultSkills with fabricated
        // level×5 ranks.
        let skill_info: Vec<(u16, u16, u16, u16, u16, i16, u16)> =
            if let (Some(skill_store), Some(skill_line_store), Some(skill_tiers_store)) = (
                self.skill_store(),
                self.skill_line_store(),
                self.skill_tiers_store(),
            ) {
                let mut skill_records: Vec<_> =
                    self.player_skill_records_like_cpp().values().collect();
                skill_records.sort_by_key(|skill| skill.skill_id);
                skill_records
                    .into_iter()
                    .filter_map(|skill| {
                        skill_store.loaded_skill_info_like_cpp(
                            skill.skill_id,
                            race,
                            class,
                            level,
                            skill.value,
                            skill.max,
                            skill_line_store,
                            skill_tiers_store,
                        )
                    })
                    .map(|entry| {
                        (
                            entry.skill_id,
                            entry.step,
                            entry.rank,
                            entry.starting_rank,
                            entry.max_rank,
                            entry.temp_bonus,
                            entry.perm_bonus,
                        )
                    })
                    .collect()
            } else {
                Vec::new()
            };

        let mut player_pkt = UpdateObject::create_player_with_party_type(
            guid,
            race,
            class,
            gender,
            level,
            display_id,
            &pos,
            map_id,
            zone_id,
            true, // is_self -> ActivePlayer fields
            visible_items,
            [ObjectGuid::EMPTY; 141],
            combat,
            skill_info,
            player_money,
            quest_log,
            party_type,
        );
        let (player_flags, player_flags_ex) = self.represented_player_flags_for_create_like_cpp();
        player_pkt.set_player_flags_like_cpp(player_flags, player_flags_ex);
        player_pkt.set_player_xp_like_cpp(player_xp.min(i32::MAX as u32) as i32);
        player_pkt
            .set_player_next_level_xp_like_cpp(player_next_level_xp.min(i32::MAX as u32) as i32);
        player_pkt.set_player_max_level_like_cpp(self.player_active_max_level_like_cpp() as i32);
        player_pkt.set_player_scaling_level_delta_like_cpp(scaling_level_delta);
        player_pkt.set_player_rest_info_like_cpp(
            0,
            self.represented_xp_rest_threshold_like_cpp(),
            self.represented_xp_rest_state_like_cpp(),
        );
        player_pkt.set_player_account_guids_like_cpp(
            ObjectGuid::create_global(HighGuid::WowAccount, 0, self.account_id as i64),
            ObjectGuid::create_global(HighGuid::BNetAccount, 0, self.battlenet_account_id() as i64),
        );
        player_pkt.set_player_collection_dynamic_fields_like_cpp(
            account_toys,
            account_heirlooms,
            account_transmog,
            trait_configs,
        );
        let Some(action_buttons) = self.represented_action_buttons_snapshot_like_cpp() else {
            return;
        };
        player_pkt.set_player_action_buttons_like_cpp(action_buttons);
        player_pkt.set_player_customizations_like_cpp(player_customizations);
        self.send_packet(&player_pkt);
        info!(
            account = self.account_id,
            map = map_id,
            "[FAR_TELEPORT] sent SendInitSelf (player ActivePlayer create) for destination map"
        );
    }
    /// CMSG_SUSPEND_TOKEN_RESPONSE — client acknowledges SMSG_SUSPEND_TOKEN during a far
    /// teleport. C++ `WorldSession::HandleSuspendTokenResponse` (MovementHandler.cpp:239)
    /// replies with SMSG_NEW_WORLD so the client loads the destination map; only then does
    /// the client send CMSG_WORLD_PORT_RESPONSE. Without this step the client sits on the
    /// loading screen at 0% forever. #NEXT.R8.ENTITIES.1229.
    pub async fn handle_suspend_token_response(&mut self, _pkt: wow_packet::WorldPacket) {
        if !self.represented_far_teleport_pending_like_cpp() {
            return;
        }
        let Some((new_map, new_pos)) = self.pending_teleport else {
            return;
        };
        self.send_packet(&wow_packet::packets::misc::NewWorld {
            map_id: new_map,
            pos: new_pos,
            reason: 0,
        });
        info!(
            account = self.account_id,
            map = new_map,
            "[FAR_TELEPORT] SuspendTokenResponse -> sent SMSG_NEW_WORLD (client now loads destination map)"
        );
    }

    /// CMSG_WORLD_PORT_RESPONSE — client confirms it has loaded the new map.
    /// C# ref: MovementHandler.HandleMoveWorldportAck
    /// Sent after SMSG_NEW_WORLD (which is emitted from handle_suspend_token_response).
    /// We respond with SMSG_RESUME_TOKEN and replay the after-add init.

    pub async fn handle_world_port_response(&mut self, _pkt: wow_packet::WorldPacket) {
        use wow_packet::packets::misc::ResumeToken;

        if !self.represented_far_teleport_pending_like_cpp() {
            warn!(
                "WorldPortResponse from account {} but far teleport semaphore is not set",
                self.account_id
            );
            return;
        }
        self.set_represented_far_teleport_pending_like_cpp(false);

        let Some((new_map, new_pos)) = self.pending_teleport.take() else {
            warn!(
                "WorldPortResponse from account {} but no pending teleport",
                self.account_id
            );
            self.set_state(crate::session::SessionState::LoggedIn);
            return;
        };

        info!(
            account = self.account_id,
            "WorldPortResponse: completing teleport to map {} ({:.2}, {:.2}, {:.2})",
            new_map,
            new_pos.x,
            new_pos.y,
            new_pos.z
        );

        // Update internal state
        self.set_player_map_position_like_cpp(new_map as u16, new_pos);
        let _ = self.update_represented_item_level_area_based_scaling_like_cpp();
        let _ = self.ensure_canonical_world_map_for_current_player_like_cpp();
        self.update_registry_position();
        self.resummon_pet_temporary_unsummoned_if_any_like_cpp();
        self.process_represented_delayed_resurrection_after_teleport_like_cpp();

        // SMSG_NEW_WORLD was already sent from handle_suspend_token_response (C++ sends it in
        // HandleSuspendTokenResponse, BEFORE the client's worldport ack — MovementHandler.cpp:253);
        // it must NOT be resent here or the client never finishes loading. #NEXT.R8.ENTITIES.1229.

        // SMSG_RESUME_TOKEN — C++ HandleMoveWorldportAck sets SequenceIndex =
        // player->m_movementCounter (read here, before SendInitialPacketsBeforeAddToMap resets
        // it) and Reason = 1 for a non-seamless far teleport (MovementHandler.cpp:108-111).
        let resume_seq = self.movement_counter_like_cpp();
        self.send_packet(&ResumeToken {
            sequence_index: resume_seq,
            reason: 1,
        });
        info!(
            account = self.account_id,
            map = new_map,
            resume_seq,
            "[FAR_TELEPORT] worldport ack: sent ResumeToken(reason=1); NewWorld was sent at SuspendTokenResponse #NEXT.R8.ENTITIES.1229"
        );

        let Some(guid) = self.player_guid() else {
            self.set_state(crate::session::SessionState::LoggedIn);
            return;
        };
        let updateobject_trace_enabled = std::env::var_os("RUSTYCORE_UPDATEOBJECT_TRACE").is_some();

        // Before-add control packets the client needs for the new map: C++
        // SendInitialPacketsBeforeAddToMap resets m_movementCounter (Player.cpp:23483) and
        // ends with SetMovedUnit -> SMSG_MOVE_SET_ACTIVE_MOVER, plus a fresh time sync. The
        // full before-add packet SET (spells/factions/action bars/etc.) is NOT re-sent on
        // teleport: the client retains it from login and it is unchanged, and re-running the
        // DB-backed before-add helper here is a documented #NEXT.R8.ENTITIES.1229 follow-up.
        self.reset_movement_counter_like_cpp();
        self.send_packet(&wow_packet::packets::misc::MoveSetActiveMover { mover_guid: guid });
        self.send_time_sync();

        // C++ Map::AddPlayerToMap(initPlayer=true) -> SendInitSelf (Map.cpp:470): re-send the
        // player's OWN object (ActivePlayer create block) for the destination map. Without it
        // the client loads to 100% but never enters the world. #NEXT.R8.ENTITIES.1229.
        self.send_player_self_create_for_teleport_like_cpp().await;

        // AddPlayerToMap-equivalent: refresh nearby world objects at the new position.
        self.send_nearby_creatures(new_map as u16, &new_pos, 0)
            .await;
        self.send_nearby_gameobjects(new_map as u16, &new_pos, 0)
            .await;
        info!(
            account = self.account_id,
            map = new_map,
            visible = self.client_visible_guids_like_cpp.len(),
            "[FAR_TELEPORT] replayed before-add (MoveSetActiveMover + TimeSync) + refreshed \
             nearby objects; now sending after-add init"
        );

        // SendInitialPacketsAfterAddToMap: post-add phase shift, InitWorldStates resolved for
        // the destination map, the PhasingHandler::OnMapChange phase shift, CUF profiles, auras.
        self.send_initial_packets_after_add_to_map(
            guid,
            &new_pos,
            new_map as i32,
            updateobject_trace_enabled,
        )
        .await;

        let (zone_id, area_id) = self.player_zone_area_like_cpp();
        info!(
            account = self.account_id,
            map = new_map,
            zone = zone_id,
            area = area_id,
            resume_seq,
            "[FAR_TELEPORT] COMPLETE — sent after-add init (InitWorldStates for this map + \
             phase-shift x2 + CUF + auras). Client should now be live in the new map."
        );

        // Full stat VALUES update — C++ login sends this after the create; it overwrites the
        // self-create block's placeholder combat stats with the player's real values.
        self.send_stat_update();

        // Back to LoggedIn — handler dispatch resumes.
        self.set_state(crate::session::SessionState::LoggedIn);
    }

    /// CMSG_AREA_TRIGGER — player entered an area trigger.
    /// C++ ref: `WorldSession::HandleAreaTriggerOpcode`.

    pub async fn handle_area_trigger(&mut self, mut pkt: wow_packet::WorldPacket) {
        let Ok(trigger_id) = pkt.read_uint32() else {
            warn!(
                account = self.account_id,
                "AreaTrigger packet missing trigger ID"
            );
            return;
        };
        let Ok(entered) = pkt.read_bit() else {
            warn!(
                account = self.account_id,
                trigger_id, "AreaTrigger packet missing Entered bit"
            );
            return;
        };
        let Ok(_from_client) = pkt.read_bit() else {
            warn!(
                account = self.account_id,
                trigger_id, "AreaTrigger packet missing FromClient bit"
            );
            return;
        };

        info!(
            "AreaTrigger: account {} trigger_id={} entered={}",
            self.account_id, trigger_id, entered
        );

        if self.is_in_taxi_flight_like_cpp() {
            debug!(
                "Area trigger {} ignored because player is in taxi flight",
                trigger_id
            );
            return;
        }

        let Some(at_entry) = self.area_trigger_db2_entry_like_cpp(trigger_id).cloned() else {
            debug!("Unknown area trigger ID {}", trigger_id);
            return;
        };

        let player_in_area_trigger = self.player_is_in_area_trigger_radius_like_cpp(&at_entry);
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
        let bound_script_id = self
            .area_trigger_script_store()
            .and_then(|store| store.get_script_id_like_cpp(trigger_id))
            .filter(|script_id| *script_id != wow_data::ScriptIdLikeCpp::NONE);
        if let Some(script_id) = bound_script_id {
            match self.dispatch_area_trigger_script_like_cpp(script_id, trigger_id, entered) {
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

        if self.handle_represented_tavern_area_trigger_like_cpp(trigger_id, entered) {
            return;
        }

        let Some(trigger) = self
            .area_trigger_store()
            .and_then(|store| store.get_trigger(trigger_id).cloned())
        else {
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

    fn area_trigger_client_conditions_meet_like_cpp(&mut self, trigger_id: u32) -> bool {
        let Some(condition_store) = self.condition_store().cloned() else {
            return true;
        };
        let Some(player_object) = self.build_condition_player_object_like_cpp() else {
            return false;
        };

        let Some(player_unit_snapshot) = self.condition_player_unit_snapshot_like_cpp() else {
            return false;
        };
        let player_snapshot = self.condition_player_snapshot_like_cpp();
        let area_table_store = self.area_table_store().cloned();

        let mut source_info =
            crate::conditions::ConditionSourceInfo::from_targets(Some(&player_object), None, None);
        source_info.set_unit_target_snapshot(0, player_unit_snapshot);
        source_info.set_player_target_snapshot(0, player_snapshot);

        crate::conditions::is_object_meeting_not_grouped_conditions_like_cpp(
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

                match crate::conditions::condition_meets_basic_like_cpp(
                    condition,
                    source_info,
                    |current_area, required_area| {
                        area_table_store.as_ref().is_some_and(|store| {
                            store.is_in_area_like_cpp(current_area, required_area)
                        })
                    },
                ) {
                    crate::conditions::ConditionMeetResult::Evaluated(value) => value,
                    crate::conditions::ConditionMeetResult::Unsupported => {
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
    pub async fn handle_activate_taxi(&mut self, mut pkt: wow_packet::WorldPacket) {
        let activate = match ActivateTaxi::read(&mut pkt) {
            Ok(activate) => activate,
            Err(error) => {
                warn!("Bad ActivateTaxi: {error}");
                return;
            }
        };

        const NPC_FLAG_FLIGHT_MASTER: u32 = 0x2000;
        let can_interact = self
            .represented_npc_can_interact_with_like_cpp(activate.vendor, NPC_FLAG_FLIGHT_MASTER, 0)
            .is_some()
            || self
                .mutate_world_creature(activate.vendor, |creature| {
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

        self.record_represented_activate_taxi_like_cpp(RepresentedActivateTaxiLikeCpp {
            vendor: activate.vendor,
            node: activate.node,
            ground_mount_id: activate.ground_mount_id,
            flying_mount_id: activate.flying_mount_id,
            preferred_mount_display,
        });
    }

    /// CMSG_TAXI_NODE_STATUS_QUERY — client asks status of a taxi NPC.
    ///
    /// C# ref: `TaxiHandler.SendTaxiStatus`:
    ///   0 = None (no node found), 1 = Learned, 2 = Unlearned, 3 = NotEligible.
    ///
    /// Without a full taxi mask we default to:
    ///   - NPCFlags includes FlightMaster (0x2000) → `Unlearned` (2)
    ///     so the taxi icon shows as available.
    ///   - Otherwise → `None` (0).

    pub async fn handle_taxi_node_status_query(&mut self, mut pkt: wow_packet::WorldPacket) {
        let unit_guid = match pkt.read_packed_guid() {
            Ok(g) => g,
            Err(_) => {
                warn!("TaxiNodeStatusQuery: failed to read unit GUID");
                return;
            }
        };

        const NPC_FLAG_FLIGHT_MASTER: u32 = 0x2000;
        let is_flight_master = self
            .mutate_world_creature(unit_guid, |creature| {
                creature.npc_flags() & NPC_FLAG_FLIGHT_MASTER != 0
            })
            .unwrap_or(false);

        // TaxiNodeStatus: 0=None, 1=Learned, 2=Unlearned, 3=NotEligible
        let status: u8 = if is_flight_master { 2 } else { 0 };

        debug!(
            account = self.account_id,
            ?unit_guid,
            status,
            "TaxiNodeStatusQuery"
        );
        self.send_packet(&TaxiNodeStatusPkt { unit_guid, status });
    }

    pub async fn handle_set_taxi_benchmark_mode(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match SetTaxiBenchmarkMode::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "SetTaxiBenchmarkMode parse failed: {error}"
                );
                return;
            }
        };

        self.represented_set_taxi_benchmark_mode_like_cpp(packet.enable);
    }

    pub async fn handle_update_area_trigger_visual(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ registers CMSG_UPDATE_AREA_TRIGGER_VISUAL as STATUS_UNHANDLED/Handle_NULL.
    }
}
