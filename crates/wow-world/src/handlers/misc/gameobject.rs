// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Private gameobject capability handlers extracted from the legacy misc owner.

use tracing::{debug, warn};
use wow_constants::ClientOpcodes;
use wow_core::ObjectGuid;
use wow_entities::{
    GAMEOBJECT_TYPE_BARBER_CHAIR, GAMEOBJECT_TYPE_BUTTON, GAMEOBJECT_TYPE_CAMERA,
    GAMEOBJECT_TYPE_CAPTURE_POINT, GAMEOBJECT_TYPE_CHAIR, GAMEOBJECT_TYPE_DOOR,
    GAMEOBJECT_TYPE_FISHING_HOLE, GAMEOBJECT_TYPE_FISHING_NODE, GAMEOBJECT_TYPE_FLAGDROP,
    GAMEOBJECT_TYPE_FLAGSTAND, GAMEOBJECT_TYPE_GATHERING_NODE, GAMEOBJECT_TYPE_GOOBER,
    GAMEOBJECT_TYPE_ITEM_FORGE, GAMEOBJECT_TYPE_MEETINGSTONE, GAMEOBJECT_TYPE_NEW_FLAG,
    GAMEOBJECT_TYPE_NEW_FLAG_DROP, GAMEOBJECT_TYPE_QUESTGIVER, GAMEOBJECT_TYPE_RITUAL,
    GAMEOBJECT_TYPE_SPELL_FOCUS, GAMEOBJECT_TYPE_SPELLCASTER, GAMEOBJECT_TYPE_TRAP,
    GAMEOBJECT_TYPE_UI_LINK, GameObjectTemplateData,
};
use wow_handler::{PacketProcessing, SessionStatus};

use crate::session::registry::PacketHandlerEntry;
use wow_packet::ClientPacket;
use wow_packet::packets::loot::{LOOT_TYPE_FISHING_JUNK_LIKE_CPP, LOOT_TYPE_FISHING_LIKE_CPP};
use wow_packet::packets::misc::CloseInteraction;

use super::represented_gameobject_icon_allows_interaction_like_cpp;
use crate::handlers::loot::represented_gameobject_interaction_distance_like_cpp;
use crate::session::{RepresentedGameObjectAccessLikeCpp, RepresentedGameObjectUseEffect};

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::CloseInteraction,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_close_interaction",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_close_interaction(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::GameObjUse,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_game_obj_use",
        handler: |session, catalogs, pkt| {
            Box::pin(async move {
                session
                    .handle_game_obj_use_with_catalogs_like_cpp(
                        catalogs.object_mgr.as_ref(),
                        catalogs.id_generators.item.as_ref(),
                        catalogs.item_valuation.as_ref(),
                        pkt,
                    )
                    .await
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::GameObjReportUse,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_game_obj_report_use",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_game_obj_report_use(pkt).await })
        },
    }
}

impl crate::session::WorldSession {
    // ── Game object interaction ───────────────────────────────────────────────

    /// CMSG_GAME_OBJ_USE — player interacts with a world game object.
    /// C++ ref: `GameObject::Use` dispatches by `GameObjectTemplate::type`.
    pub(crate) async fn handle_game_obj_use_with_catalogs_like_cpp(
        &mut self,
        catalogs: &crate::session::ObjectMgrCatalogsLikeCpp,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        item_valuation: &crate::session::ItemValuationCatalogsLikeCpp,
        mut pkt: wow_packet::WorldPacket,
    ) {
        let gameobject_guid = match pkt.read_packed_guid() {
            Ok(guid) => guid,
            Err(e) => {
                warn!("GameObjUse: failed to read gameobject guid: {e}");
                return;
            }
        };

        if !gameobject_guid.is_game_object() {
            return;
        }

        let gameobject_access = if self.canonical_map_manager.is_some() {
            match self.canonical_gameobject_access_like_cpp(gameobject_guid) {
                Some(access) => access,
                None => return,
            }
        } else {
            if !self
                .client_visible_guids_like_cpp
                .contains(&gameobject_guid)
            {
                return;
            }
            RepresentedGameObjectAccessLikeCpp {
                entry: gameobject_guid.entry(),
                position: self
                    .represented_gameobject_use_states
                    .get(&gameobject_guid)
                    .and_then(|state| state.position)
                    .unwrap_or_default(),
            }
        };

        let Some(row) = catalogs.gameobject.get(gameobject_access.entry).cloned() else {
            return;
        };

        let Ok(go_type) = u32::try_from(row.go_type) else {
            return;
        };
        let data = row.data.map(|value| u32::try_from(value).unwrap_or(0));
        let template = GameObjectTemplateData::new(go_type, data);
        self.record_represented_gameobject_template_quest_source_like_cpp(
            gameobject_guid,
            &template,
        );
        let icon_allows_interaction =
            represented_gameobject_icon_allows_interaction_like_cpp(&row.icon_name);
        self.record_represented_gameobject_icon_interaction_like_cpp(
            gameobject_guid,
            icon_allows_interaction,
        );
        if !icon_allows_interaction {
            return;
        }
        let interact_distance = represented_gameobject_interaction_distance_like_cpp(
            Some(go_type as u8),
            Some(template.get_interact_radius_override_like_cpp()),
        );
        let Some(player_position) = self.player_position_like_cpp() else {
            return;
        };
        if self.canonical_map_manager.is_some() {
            let Some(verified_access) = self.represented_gameobject_can_interact_with_like_cpp(
                gameobject_guid,
                interact_distance,
            ) else {
                return;
            };
            if verified_access.entry != gameobject_access.entry {
                return;
            }
        } else if !gameobject_access
            .position
            .is_within_dist(&player_position, interact_distance)
        {
            return;
        }
        if !self
            .represented_meets_player_condition_id_like_cpp(template.get_condition_id1_like_cpp())
        {
            debug!(
                account = self.account_id,
                guid = ?gameobject_guid,
                go_type,
                condition_id = template.get_condition_id1_like_cpp(),
                "GameObjUse: represented gameobject interact condition not met"
            );
            return;
        }
        if !self.represented_gameobject_use_allowed_by_mover_like_cpp(
            template.is_usable_mounted_like_cpp(),
        ) {
            return;
        }
        let Some(player_guid) = self.player_guid() else {
            return;
        };
        if !self.apply_represented_gameobject_player_use_preamble_like_cpp(
            gameobject_guid,
            player_guid,
            template.is_usable_mounted_like_cpp(),
            template.get_no_damage_immune_like_cpp() != 0,
        ) {
            return;
        }
        if go_type != GAMEOBJECT_TYPE_TRAP
            && !self.apply_represented_gameobject_cooldown_like_cpp(
                gameobject_guid,
                template.get_cooldown_like_cpp(),
            )
        {
            return;
        }

        match go_type {
            GAMEOBJECT_TYPE_DOOR | GAMEOBJECT_TYPE_BUTTON => {
                self.use_represented_gameobject_door_or_button_like_cpp(
                    gameobject_guid,
                    player_guid,
                    template.get_auto_close_time_like_cpp(),
                );
                return;
            }
            GAMEOBJECT_TYPE_QUESTGIVER => {
                if let Some(source) = template.questgiver_use_source_like_cpp() {
                    self.use_represented_gameobject_questgiver_like_cpp(
                        gameobject_guid,
                        player_guid,
                        gameobject_access.entry,
                        source,
                    );
                }
                return;
            }
            GAMEOBJECT_TYPE_TRAP => {
                if let Some(source) = template.trap_use_source_like_cpp() {
                    self.use_represented_gameobject_trap_like_cpp(
                        gameobject_guid,
                        player_guid,
                        source,
                    );
                }
                return;
            }
            GAMEOBJECT_TYPE_FISHING_NODE => {
                let effect_start = self.represented_gameobject_use_effects.len();
                self.use_represented_gameobject_fishing_node_like_cpp(gameobject_guid, player_guid);
                let Some(area_id) = self.represented_gameobject_area_id_like_cpp(gameobject_guid)
                else {
                    return;
                };
                let loot_request = self
                    .represented_gameobject_use_effects
                    .get(effect_start..)
                    .unwrap_or(&[])
                    .iter()
                    .rev()
                    .find_map(|effect| match effect {
                        RepresentedGameObjectUseEffect::FishingLootRequested {
                            gameobject_guid: effect_guid,
                            loot_type,
                            ..
                        } if *effect_guid == gameobject_guid => Some(*loot_type),
                        _ => None,
                    });
                match loot_request {
                    Some(LOOT_TYPE_FISHING_LIKE_CPP) => {
                        self.open_represented_fishing_node_loot_with_catalogs_like_cpp(
                            item_valuation,
                            gameobject_guid,
                            area_id,
                            false,
                        )
                        .await;
                    }
                    Some(LOOT_TYPE_FISHING_JUNK_LIKE_CPP) => {
                        self.open_represented_fishing_node_loot_with_catalogs_like_cpp(
                            item_valuation,
                            gameobject_guid,
                            area_id,
                            true,
                        )
                        .await;
                    }
                    _ => {}
                }
                return;
            }
            GAMEOBJECT_TYPE_RITUAL => {
                if let Some(source) = template.ritual_use_source_like_cpp() {
                    self.use_represented_gameobject_ritual_like_cpp(
                        gameobject_guid,
                        player_guid,
                        source,
                    );
                }
                return;
            }
            GAMEOBJECT_TYPE_CHAIR => {
                if let Some(source) = template.chair_use_source_like_cpp() {
                    let gameobject_size = row.size.max(0.0);
                    self.use_represented_gameobject_chair_like_cpp(
                        gameobject_guid,
                        player_guid,
                        player_position,
                        gameobject_access.position,
                        gameobject_size,
                        source,
                    );
                }
                return;
            }
            GAMEOBJECT_TYPE_BARBER_CHAIR => {
                if let Some(source) = template.barber_chair_use_source_like_cpp() {
                    self.use_represented_gameobject_barber_chair_like_cpp(
                        gameobject_guid,
                        player_guid,
                        gameobject_access.position,
                        source,
                    );
                }
                return;
            }
            GAMEOBJECT_TYPE_UI_LINK => {
                if let Some(source) = template.ui_link_use_source_like_cpp() {
                    self.use_represented_gameobject_ui_link_like_cpp(
                        gameobject_guid,
                        player_guid,
                        source,
                    );
                }
                return;
            }
            GAMEOBJECT_TYPE_ITEM_FORGE => {
                if let Some(source) = template.item_forge_use_source_like_cpp() {
                    self.use_represented_gameobject_item_forge_like_cpp(
                        gameobject_guid,
                        player_guid,
                        source,
                    );
                }
                return;
            }
            GAMEOBJECT_TYPE_CAPTURE_POINT => {
                if let Some(source) = template.capture_point_use_source_like_cpp() {
                    self.use_represented_gameobject_capture_point_like_cpp(
                        gameobject_guid,
                        player_guid,
                        source,
                    );
                }
                return;
            }
            GAMEOBJECT_TYPE_FLAGSTAND => {
                if let Some(source) = template.flag_stand_use_source_like_cpp() {
                    self.use_represented_gameobject_flagstand_like_cpp(
                        gameobject_guid,
                        player_guid,
                        source,
                    );
                }
                return;
            }
            GAMEOBJECT_TYPE_FLAGDROP => {
                if let Some(source) = template.flag_drop_use_source_like_cpp() {
                    self.use_represented_gameobject_flagdrop_like_cpp(
                        gameobject_guid,
                        player_guid,
                        gameobject_guid.entry(),
                        source,
                    );
                }
                return;
            }
            GAMEOBJECT_TYPE_NEW_FLAG => {
                if let Some(source) = template.new_flag_use_source_like_cpp() {
                    self.use_represented_gameobject_new_flag_like_cpp(
                        gameobject_guid,
                        player_guid,
                        gameobject_access.entry,
                        source,
                    );
                }
                return;
            }
            GAMEOBJECT_TYPE_NEW_FLAG_DROP => {
                if let Some(source) = template.new_flag_drop_use_source_like_cpp() {
                    self.use_represented_gameobject_new_flag_drop_like_cpp(
                        gameobject_guid,
                        player_guid,
                        source,
                    );
                }
                return;
            }
            GAMEOBJECT_TYPE_MEETINGSTONE => {
                if let Some(mut source) = template.meeting_stone_use_source_like_cpp() {
                    source.content_tuning_id = u32::try_from(row.content_tuning_id).unwrap_or(0);
                    self.use_represented_gameobject_meeting_stone_like_cpp(
                        gameobject_guid,
                        player_guid,
                        gameobject_access.entry,
                        source,
                    );
                }
                return;
            }
            GAMEOBJECT_TYPE_SPELL_FOCUS => {
                self.use_represented_gameobject_spell_focus_like_cpp(
                    gameobject_guid,
                    player_guid,
                    template.spell_focus_linked_trap_like_cpp(),
                );
                return;
            }
            GAMEOBJECT_TYPE_SPELLCASTER => {
                if let Some(source) = template.spellcaster_use_source_like_cpp() {
                    self.use_represented_gameobject_spellcaster_like_cpp(
                        gameobject_guid,
                        player_guid,
                        gameobject_access.entry,
                        source,
                    );
                }
                return;
            }
            GAMEOBJECT_TYPE_CAMERA => {
                if let Some(source) = template.camera_use_source_like_cpp() {
                    self.use_represented_gameobject_camera_like_cpp(
                        gameobject_guid,
                        player_guid,
                        source,
                    );
                }
                return;
            }
            GAMEOBJECT_TYPE_GOOBER => {
                if let Some(source) = template.goober_use_source_like_cpp() {
                    if self
                        .use_represented_gameobject_goober_preamble_with_generator_like_cpp(
                            item_guid_generator,
                            gameobject_guid,
                            gameobject_access.entry,
                            gameobject_access.position,
                            player_guid,
                            source,
                        )
                        .await
                    {
                        self.use_represented_gameobject_goober_state_like_cpp(
                            gameobject_guid,
                            player_guid,
                            gameobject_access.entry,
                            source,
                        );
                    }
                }
                return;
            }
            _ => {}
        }

        if let Some(source) = template.chest_loot_source_like_cpp() {
            if source.is_empty() {
                return;
            }

            self.open_represented_gameobject_chest_with_template_money_like_cpp(
                item_guid_generator,
                item_valuation,
                gameobject_guid,
                source,
                (row.min_money, row.max_money),
            )
            .await;
            return;
        }

        let loot_id = template.get_loot_id_like_cpp();
        match go_type {
            GAMEOBJECT_TYPE_FISHING_HOLE if loot_id != 0 => {
                self.open_represented_fishing_hole_with_catalogs_like_cpp(
                    item_valuation,
                    gameobject_guid,
                    gameobject_access.entry,
                    loot_id,
                )
                .await;
            }
            GAMEOBJECT_TYPE_GATHERING_NODE => {
                if let Some(source) = template.gathering_node_use_source_like_cpp() {
                    self.open_represented_gathering_node_with_catalogs_like_cpp(
                        item_valuation,
                        gameobject_guid,
                        gameobject_access.entry,
                        source,
                    )
                    .await;
                }
            }
            _ => {
                debug!(
                    account = self.account_id,
                    guid = ?gameobject_guid,
                    go_type,
                    "GameObjUse: represented gameobject use type is not ported yet"
                );
            }
        }
    }

    #[cfg(test)]
    pub async fn handle_game_obj_use(&mut self, pkt: wow_packet::WorldPacket) {
        let catalogs = self
            .world_query_catalogs_like_cpp()
            .cloned()
            .unwrap_or_default();
        let Some(generator) = self.item_guid_generator_like_cpp_for_bridge() else {
            return;
        };
        let item_valuation = self.item_valuation_catalogs_for_test_like_cpp();
        self.handle_game_obj_use_with_catalogs_like_cpp(
            &catalogs,
            generator.as_ref(),
            &item_valuation,
            pkt,
        )
        .await;
    }

    /// CMSG_GAME_OBJ_REPORT_USE — client reports a game object use event.
    /// C++ ref: `WorldSession::HandleGameobjectReportUse`.

    pub async fn handle_game_obj_report_use(&mut self, mut pkt: wow_packet::WorldPacket) {
        let gameobject_guid = match pkt.read_packed_guid() {
            Ok(guid) => guid,
            Err(e) => {
                warn!("GameObjReportUse: failed to read gameobject guid: {e}");
                return;
            }
        };

        if !gameobject_guid.is_game_object() {
            return;
        }

        let Some(player_guid) = self.player_guid() else {
            return;
        };
        if self.player_moved_unit_guid_like_cpp() != Some(player_guid) {
            return;
        }

        let state = self.represented_gameobject_use_states.get(&gameobject_guid);
        let interaction_distance = represented_gameobject_interaction_distance_like_cpp(
            state.and_then(|state| state.go_type),
            state.and_then(|state| state.interact_radius_override),
        );

        let gameobject_access = if self.canonical_map_manager.is_some() {
            match self.represented_gameobject_can_interact_with_like_cpp(
                gameobject_guid,
                interaction_distance,
            ) {
                Some(access) => access,
                None => return,
            }
        } else {
            if !self
                .client_visible_guids_like_cpp
                .contains(&gameobject_guid)
            {
                return;
            }
            let Some(position) = state.and_then(|state| state.position) else {
                return;
            };
            let Some(player_position) = self.player_position_like_cpp() else {
                return;
            };
            if !position.is_within_dist(&player_position, interaction_distance) {
                return;
            }
            RepresentedGameObjectAccessLikeCpp {
                entry: gameobject_guid.entry(),
                position,
            }
        };
        #[cfg(not(test))]
        let _ = gameobject_access;

        if self.record_represented_gameobject_report_use_ai_like_cpp(gameobject_guid, player_guid) {
            return;
        }

        #[cfg(test)]
        {
            self.represented_gameobject_criteria_events.push(
                crate::session::RepresentedGameObjectCriteriaEvent::UseGameobject {
                    player_guid,
                    gameobject_entry: gameobject_access.entry,
                },
            );
        }
    }

    pub(crate) fn represented_gameobject_gossip_can_interact_with_like_cpp(
        &self,
        gameobject_guid: ObjectGuid,
    ) -> Option<RepresentedGameObjectAccessLikeCpp> {
        // The caller separately requires the current server-owned
        // InteractionData source and menu item. This helper revalidates the
        // represented GameObject half; constructing full scripted GO gossip
        // menus remains an explicit runtime boundary.
        if !gameobject_guid.is_game_object()
            || self.resolved_is_in_taxi_flight_like_cpp() != Some(false)
            || !self.player_is_strictly_in_world_like_cpp()
        {
            return None;
        }

        let state = self
            .represented_gameobject_use_states
            .get(&gameobject_guid)?;
        let go_type = state.go_type?;

        // C++ checks the immutable template icon on every lookup. Rust records
        // that fact while reading the same template in HandleGameObjectUse;
        // missing evidence fails closed rather than trusting historical
        // effects or canonical existence alone.
        if state.icon_name_allows_interaction_like_cpp != Some(true) {
            return None;
        }

        let map_key = self.current_canonical_player_map_key_like_cpp()?;
        {
            let manager = self.canonical_map_manager.as_ref()?.lock().ok()?;
            let map = manager.find_map(map_key.map_id, map_key.instance_id)?;
            let gameobject = map.map().get_typed_game_object(gameobject_guid)?;
            if !gameobject.world().object().is_in_world() {
                return None;
            }
            let gameobject_phase_shift = self
                .represented_gameobject_phase_shifts
                .get(&gameobject_guid)
                .unwrap_or_else(|| gameobject.world().phase_shift());
            if !self.can_see_phase_shift_like_cpp(gameobject_phase_shift) {
                return None;
            }
        }

        let interaction_distance = represented_gameobject_interaction_distance_like_cpp(
            Some(go_type),
            state.interact_radius_override,
        );
        self.represented_gameobject_can_interact_with_like_cpp(
            gameobject_guid,
            interaction_distance,
        )
    }

    /// CMSG_CLOSE_INTERACTION — player closed an NPC interaction window.
    /// C++ ref: `WorldSession::HandleCloseInteraction`.

    pub async fn handle_close_interaction(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match CloseInteraction::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "CloseInteraction parse failed: {error}"
                );
                return;
            }
        };

        self.reset_player_interaction_if_source_like_cpp(request.source_guid);

        // C++ also clears Player::StableMaster when it matches SourceGuid. Rust
        // does not expose represented stable-master state yet.
    }
}
