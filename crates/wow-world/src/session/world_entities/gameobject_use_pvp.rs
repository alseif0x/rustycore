//! Represented use of PvP gameobjects: flags, flagstands and capture points.
//!
//! Moved out of the Session root under #599. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(crate) fn use_represented_gameobject_capture_point_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        source: wow_entities::CapturePointUseSource,
    ) -> bool {
        let state = self
            .represented_gameobject_use_states
            .get(&gameobject_guid)
            .and_then(|state| state.capture_point_state)
            .unwrap_or(RepresentedCapturePointStateLikeCpp::Neutral);
        let can_interact = match player_team_for_race_cpp(self.player_race_like_cpp()) {
            Team::Horde => matches!(
                state,
                RepresentedCapturePointStateLikeCpp::Neutral
                    | RepresentedCapturePointStateLikeCpp::ContestedAlliance
                    | RepresentedCapturePointStateLikeCpp::AllianceCaptured
            ),
            Team::Alliance => matches!(
                state,
                RepresentedCapturePointStateLikeCpp::Neutral
                    | RepresentedCapturePointStateLikeCpp::ContestedHorde
                    | RepresentedCapturePointStateLikeCpp::HordeCaptured
            ),
            Team::Other => false,
        };
        if !can_interact {
            return false;
        }

        let ai_handled = self
            .represented_gameobject_use_states
            .get(&gameobject_guid)
            .map(|state| state.capture_point_assault_ai_returns_true)
            .unwrap_or(false);
        self.represented_gameobject_use_effects.push(
            RepresentedGameObjectUseEffect::CapturePointAssaultAi {
                gameobject_guid,
                player_guid,
                handled: ai_handled,
            },
        );
        if ai_handled {
            return true;
        }

        if !self
            .player_battleground_state_snapshot_like_cpp()
            .is_some_and(|state| state.in_battleground_like_cpp())
        {
            return false;
        }

        self.represented_gameobject_use_effects.push(
            RepresentedGameObjectUseEffect::CapturePointAssaultRequested {
                gameobject_guid,
                player_guid,
                capture_time_ms: source.capture_time_ms,
                world_state_id: source.world_state_id,
                contested_event_horde: source.contested_event_horde,
                contested_event_alliance: source.contested_event_alliance,
            },
        );

        let player_team = player_team_for_race_cpp(self.player_race_like_cpp());
        let now = Instant::now();
        let (next_state, broadcast_text_id, event_id, assault_timer_ms, last_team_capture) =
            match player_team {
                Team::Horde => {
                    let last_team_capture = self
                        .represented_gameobject_use_states
                        .get(&gameobject_guid)
                        .map(|state| state.capture_point_last_team_capture)
                        .unwrap_or(Team::Other);
                    if last_team_capture == Team::Horde {
                        (
                            RepresentedCapturePointStateLikeCpp::HordeCaptured,
                            source.defended_broadcast_horde,
                            source.defended_event_horde,
                            0,
                            Team::Horde,
                        )
                    } else {
                        (
                            RepresentedCapturePointStateLikeCpp::ContestedHorde,
                            source.assault_broadcast_horde,
                            source.contested_event_horde,
                            source.capture_time_ms,
                            Team::Other,
                        )
                    }
                }
                Team::Alliance => {
                    let last_team_capture = self
                        .represented_gameobject_use_states
                        .get(&gameobject_guid)
                        .map(|state| state.capture_point_last_team_capture)
                        .unwrap_or(Team::Other);
                    if last_team_capture == Team::Alliance {
                        (
                            RepresentedCapturePointStateLikeCpp::AllianceCaptured,
                            source.defended_broadcast_alliance,
                            source.defended_event_alliance,
                            0,
                            Team::Alliance,
                        )
                    } else {
                        (
                            RepresentedCapturePointStateLikeCpp::ContestedAlliance,
                            source.assault_broadcast_alliance,
                            source.contested_event_alliance,
                            source.capture_time_ms,
                            Team::Other,
                        )
                    }
                }
                Team::Other => return false,
            };
        {
            let state = self
                .represented_gameobject_use_states
                .entry(gameobject_guid)
                .or_default();
            state.capture_point_source = Some(source);
            state.capture_point_state = Some(next_state);
            state.capture_point_assault_until = (assault_timer_ms != 0)
                .then_some(now + Duration::from_millis(u64::from(assault_timer_ms)));
            if last_team_capture != Team::Other {
                state.capture_point_last_team_capture = last_team_capture;
            }
        }
        self.record_represented_capture_point_update_like_cpp(
            gameobject_guid,
            source,
            next_state,
            broadcast_text_id,
            event_id,
            assault_timer_ms,
        );

        true
    }
    pub(crate) fn use_represented_gameobject_flagstand_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        source: wow_entities::FlagStandUseSource,
    ) -> bool {
        if !self
            .represented_player_can_use_battleground_object_like_cpp(gameobject_guid, player_guid)
        {
            return false;
        }
        if self
            .represented_player_battleground_type_id_or_reject_like_cpp(
                gameobject_guid,
                player_guid,
            )
            .is_none()
        {
            return false;
        }
        if self.represented_player_reject_battleground_object_vehicle_like_cpp(
            gameobject_guid,
            player_guid,
        ) {
            return false;
        }

        if self
            .remove_represented_stealth_or_invisibility_auras_by_type_like_cpp()
            .is_none()
        {
            return false;
        }
        self.represented_gameobject_use_effects.push(
            RepresentedGameObjectUseEffect::RemoveStealthOrInvisibilityAuras {
                gameobject_guid,
                player_guid,
            },
        );
        self.represented_gameobject_use_effects.push(
            RepresentedGameObjectUseEffect::BattlegroundFlagStandClicked {
                gameobject_guid,
                player_guid,
                pickup_spell_id: source.pickup_spell_id,
                return_aura_id: source.return_aura_id,
                return_spell_id: source.return_spell_id,
            },
        );

        true
    }
    pub(crate) fn use_represented_gameobject_flagdrop_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        gameobject_entry: u32,
        source: wow_entities::FlagDropUseSource,
    ) -> bool {
        if !self
            .represented_player_can_use_battleground_object_like_cpp(gameobject_guid, player_guid)
        {
            return false;
        }
        let Some(bg_type_id) = self.represented_player_battleground_type_id_or_reject_like_cpp(
            gameobject_guid,
            player_guid,
        ) else {
            return false;
        };
        if self.represented_player_reject_battleground_object_vehicle_like_cpp(
            gameobject_guid,
            player_guid,
        ) {
            return false;
        }

        let click_target = match gameobject_entry {
            179785 | 179786 if bg_type_id == BATTLEGROUND_WS_LIKE_CPP => {
                BattlegroundFlagDropClickTarget::WarsongGulch
            }
            184142 if bg_type_id == BATTLEGROUND_EY_LIKE_CPP => {
                BattlegroundFlagDropClickTarget::EyeOfTheStorm
            }
            _ => BattlegroundFlagDropClickTarget::None,
        };

        if self
            .remove_represented_stealth_or_invisibility_auras_by_type_like_cpp()
            .is_none()
        {
            return false;
        }
        self.represented_gameobject_use_effects.push(
            RepresentedGameObjectUseEffect::RemoveStealthOrInvisibilityAuras {
                gameobject_guid,
                player_guid,
            },
        );
        self.represented_gameobject_use_effects.push(
            RepresentedGameObjectUseEffect::BattlegroundFlagDropClicked {
                gameobject_guid,
                player_guid,
                gameobject_entry,
                click_target,
                event_id: source.event_id,
                pickup_spell_id: source.pickup_spell_id,
                expire_duration_ms: source.expire_duration_ms,
            },
        );
        if source.event_id != 0 {
            self.represented_gameobject_use_effects.push(
                RepresentedGameObjectUseEffect::TriggerGameEvent {
                    gameobject_guid,
                    player_guid,
                    event_id: source.event_id,
                },
            );
        }
        self.send_represented_gameobject_delete_packets_like_cpp(gameobject_guid);
        self.represented_gameobject_use_effects
            .push(RepresentedGameObjectUseEffect::GameObjectDeleted { gameobject_guid });

        true
    }
    pub(crate) fn use_represented_gameobject_new_flag_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        gameobject_entry: u32,
        source: wow_entities::NewFlagUseSource,
    ) -> bool {
        if !self
            .represented_player_can_use_battleground_object_like_cpp(gameobject_guid, player_guid)
        {
            return false;
        }
        let current_state = self
            .represented_gameobject_use_states
            .get(&gameobject_guid)
            .and_then(|state| state.new_flag_state)
            .unwrap_or(RepresentedNewFlagStateRequest::InBase);
        if current_state != RepresentedNewFlagStateRequest::InBase {
            return false;
        }
        {
            let state = self
                .represented_gameobject_use_states
                .entry(gameobject_guid)
                .or_default();
            state.new_flag_return_on_defender_interact = Some(source.return_on_defender_interact);
            state.new_flag_pickup_spell_id = Some(source.pickup_spell_id);
            state.new_flag_entry = Some(gameobject_entry);
        }

        self.represented_gameobject_use_effects.push(
            RepresentedGameObjectUseEffect::NewFlagPickupRequested {
                gameobject_guid,
                player_guid,
                pickup_spell_id: source.pickup_spell_id,
                expire_duration_ms: source.expire_duration_ms,
                respawn_time_ms: source.respawn_time_ms,
                flag_drop_entry: source.flag_drop_entry,
                exclusive_category: source.exclusive_category,
                world_state_id: source.world_state_id,
                return_on_defender_interact: source.return_on_defender_interact,
            },
        );

        self.apply_represented_gameobject_post_use_spell_like_cpp(
            gameobject_guid,
            player_guid,
            gameobject_entry,
            wow_entities::GAMEOBJECT_TYPE_NEW_FLAG,
            source.pickup_spell_id,
            false,
            RepresentedGameObjectSpellCaster::GameObject,
            gameobject_guid,
        );

        true
    }
    pub(crate) fn use_represented_gameobject_new_flag_drop_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        source: wow_entities::NewFlagDropUseSource,
    ) -> bool {
        if !self
            .represented_player_can_use_battleground_object_like_cpp(gameobject_guid, player_guid)
        {
            return false;
        }

        self.represented_gameobject_use_effects.push(
            RepresentedGameObjectUseEffect::NewFlagDropInteracted {
                gameobject_guid,
                player_guid,
                spawn_vignette_id: source.spawn_vignette_id,
            },
        );

        let owner_guid = self
            .represented_gameobject_use_states
            .get(&gameobject_guid)
            .and_then(|state| state.owner_guid);
        if let Some(owner_guid) = owner_guid {
            let Some(owner_state) = self.represented_gameobject_use_states.get(&owner_guid) else {
                self.send_represented_gameobject_delete_packets_like_cpp(gameobject_guid);
                self.represented_gameobject_use_effects
                    .push(RepresentedGameObjectUseEffect::GameObjectDeleted { gameobject_guid });
                return true;
            };
            if owner_state.go_type != Some(wow_entities::GAMEOBJECT_TYPE_NEW_FLAG as u8) {
                return false;
            }
            if owner_state.new_flag_state != Some(RepresentedNewFlagStateRequest::Dropped) {
                return false;
            }
            let return_on_defender_interact = owner_state
                .new_flag_return_on_defender_interact
                .unwrap_or(false);
            let owner_pickup_spell_id = owner_state.new_flag_pickup_spell_id.unwrap_or(0);
            let owner_entry = owner_state.new_flag_entry.unwrap_or(0);
            let defender_interact = self
                .represented_gameobject_is_friendly_to_player_like_cpp(owner_guid)
                .map(|friendly| !friendly)
                .unwrap_or(false);
            if defender_interact && return_on_defender_interact {
                self.send_represented_gameobject_delete_packets_like_cpp(gameobject_guid);
                self.represented_gameobject_use_effects
                    .push(RepresentedGameObjectUseEffect::GameObjectDeleted { gameobject_guid });
                self.apply_represented_new_flag_state_command_like_cpp(
                    owner_guid,
                    Some(player_guid),
                    RepresentedNewFlagStateRequest::InBase,
                    0,
                );
                return true;
            }

            if self.apply_represented_gameobject_post_use_spell_like_cpp_internal(
                owner_guid,
                player_guid,
                owner_entry,
                wow_entities::GAMEOBJECT_TYPE_NEW_FLAG,
                owner_pickup_spell_id,
                true,
                RepresentedGameObjectSpellCaster::GameObject,
                owner_guid,
                false,
            ) {
                self.send_represented_gameobject_delete_packets_like_cpp(gameobject_guid);
                self.represented_gameobject_use_effects
                    .push(RepresentedGameObjectUseEffect::GameObjectDeleted { gameobject_guid });
                self.apply_represented_new_flag_state_command_like_cpp(
                    owner_guid,
                    Some(player_guid),
                    RepresentedNewFlagStateRequest::Taken,
                    0,
                );
                return true;
            }
        }

        self.send_represented_gameobject_delete_packets_like_cpp(gameobject_guid);
        self.represented_gameobject_use_effects
            .push(RepresentedGameObjectUseEffect::GameObjectDeleted { gameobject_guid });

        true
    }
}
