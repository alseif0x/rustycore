// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Battleground adapter: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

#[cfg(test)]
use super::RepresentedWargameInviteAcceptanceLikeCpp;
use super::{BattlemasterListStore, DISABLE_TYPE_BATTLEGROUND, Duration, Instant, ObjectGuid};
use super::{RepresentedBattlegroundObjectUseRejection, RepresentedCapturePointStateLikeCpp};
use super::{RepresentedGameObjectUseEffect, RepresentedNewFlagStateRequest};
use super::{UnitFlags, WorldSession};

pub(crate) use wow_world_core::session::RepresentedBattlegroundQueueTypeIdLikeCpp;
#[cfg(test)]
pub(crate) use wow_world_core::session::{
    RepresentedBattlefieldListLikeCpp, RepresentedBattlefieldPortLikeCpp,
    RepresentedBattlemasterHelloLikeCpp, RepresentedBattlemasterJoinArenaLikeCpp,
    RepresentedBattlemasterJoinLikeCpp, RepresentedBattlemasterJoinSkirmishLikeCpp,
};

pub(crate) use wow_world_core::session::battleground_queue_type_id_from_packed_like_cpp;

pub(in crate::session) fn arena_team_type_by_slot_like_cpp(slot: u8) -> Option<u8> {
    match slot {
        0 => Some(2),
        1 => Some(3),
        2 => Some(5),
        _ => None,
    }
}

pub(in crate::session) fn arena_skirmish_type_like_cpp(bg_type_id: u32, bracket_id: u32) -> u8 {
    if bg_type_id == 3 || bg_type_id == 5 {
        bg_type_id as u8
    } else if bracket_id == 3 || bracket_id == 5 {
        bracket_id as u8
    } else {
        2
    }
}

impl WorldSession {
    #[cfg_attr(not(test), allow(unused_variables))]
    pub(crate) fn battlemaster_join_arena_like_cpp(
        &mut self,
        battlemaster_lists: &BattlemasterListStore,
        team_size_index: u8,
        roles: u8,
    ) -> bool {
        if crate::session::hub_ref(self).player_in_represented_battleground_like_cpp() {
            return false;
        }

        let Some(arena_type) = arena_team_type_by_slot_like_cpp(team_size_index) else {
            return false;
        };
        let queue_type_id = RepresentedBattlegroundQueueTypeIdLikeCpp {
            battlemaster_list_id: wow_data::BATTLEGROUND_AA_LIKE_CPP as u16,
            queue_type: 1,
            rated: true,
            team_size: arena_type,
        };
        if !crate::session::hub_ref(self)
            .is_valid_battleground_queue_type_id_like_cpp(battlemaster_lists, queue_type_id)
        {
            return false;
        }
        if self
            .catalogs
            .disable_mgr
            .as_ref()
            .map(|disable_mgr| {
                disable_mgr.is_disabled_for_like_cpp(
                    DISABLE_TYPE_BATTLEGROUND,
                    wow_data::BATTLEGROUND_AA_LIKE_CPP,
                    None,
                    0,
                    None,
                )
            })
            .unwrap_or(false)
        {
            return false;
        }

        let (Some(player_guid), Some(group_guid), Some(group_registry)) = (
            self.player_guid(),
            self.resolved_group_guid_like_cpp(),
            self.core.directory.group_registry.as_ref(),
        ) else {
            return false;
        };
        let is_group_leader = group_registry
            .get(&group_guid)
            .map(|group| {
                group.members.contains(&player_guid) && group.is_leader_like_cpp(player_guid)
            })
            .unwrap_or(false);
        if !is_group_leader {
            return false;
        }

        // C++ continues with Player::GetArenaTeamId, ArenaTeamMgr::GetArenaTeamById,
        // Group::CanJoinBattlegroundQueue, AddGroup and status packets. Rust
        // does not have the live rated-arena team/queue manager in this seam yet,
        // so the bounded port records the accepted intent after the representable
        // gates above without pretending that the queue was live.
        #[cfg(test)]
        self.fixtures
            .battleground
            .represented_battlemaster_join_arenas_like_cpp
            .push(RepresentedBattlemasterJoinArenaLikeCpp {
                team_size_index,
                roles,
                arena_type,
                group_guid,
                queue_type_id,
            });
        true
    }

    #[cfg_attr(not(test), allow(unused_variables))]
    pub(crate) fn battlemaster_join_skirmish_like_cpp(
        &mut self,
        battlemaster_lists: &BattlemasterListStore,
        bg_type_id: u32,
        bracket_id: u32,
        as_group: u8,
        is_rated: u8,
    ) -> bool {
        if crate::session::hub_ref(self).player_in_represented_battleground_like_cpp() {
            return false;
        }

        let arena_type = arena_skirmish_type_like_cpp(bg_type_id, bracket_id);
        let Some(entry) = battlemaster_lists.get(wow_data::BATTLEGROUND_AA_LIKE_CPP) else {
            return false;
        };
        if entry.instance_type != wow_data::MAP_ARENA_LIKE_CPP {
            return false;
        }
        if self
            .catalogs
            .disable_mgr
            .as_ref()
            .map(|disable_mgr| {
                disable_mgr.is_disabled_for_like_cpp(
                    DISABLE_TYPE_BATTLEGROUND,
                    wow_data::BATTLEGROUND_AA_LIKE_CPP,
                    None,
                    0,
                    None,
                )
            })
            .unwrap_or(false)
        {
            return false;
        }

        let join_as_group = as_group != 0;
        let group_guid = if join_as_group {
            let (Some(player_guid), Some(group_guid), Some(group_registry)) = (
                self.player_guid(),
                self.resolved_group_guid_like_cpp(),
                self.core.directory.group_registry.as_ref(),
            ) else {
                return false;
            };
            let is_group_leader = group_registry
                .get(&group_guid)
                .map(|group| {
                    group.members.contains(&player_guid) && group.is_leader_like_cpp(player_guid)
                })
                .unwrap_or(false);
            if !is_group_leader {
                return false;
            }
            Some(group_guid)
        } else {
            None
        };

        let queue_type_id = RepresentedBattlegroundQueueTypeIdLikeCpp {
            battlemaster_list_id: wow_data::BATTLEGROUND_AA_LIKE_CPP as u16,
            queue_type: 4,
            rated: false,
            team_size: arena_type,
        };

        // C++ continues with PVPDifficulty lookup, BattlegroundQueue::AddGroup,
        // solo queue-slot checks, Group::CanJoinBattlegroundQueue, status packet
        // fanout and ScheduleQueueUpdate. Rust records the bounded intent after
        // the currently represented gates without pretending that live queueing exists.
        #[cfg(test)]
        self.fixtures
            .battleground
            .represented_battlemaster_join_skirmishes_like_cpp
            .push(RepresentedBattlemasterJoinSkirmishLikeCpp {
                bg_type_id,
                bracket_id,
                as_group: join_as_group,
                is_rated_packet_value: is_rated,
                arena_type,
                group_guid,
                queue_type_id,
            });
        true
    }

    #[cfg_attr(not(test), allow(unused_variables))]
    pub(crate) fn accept_represented_wargame_invite_like_cpp(&mut self, inviter_name: &str) {
        let (
            Some(player_guid),
            Some(player_group_guid),
            Some(player_registry),
            Some(group_registry),
        ) = (
            self.player_guid(),
            self.resolved_group_guid_like_cpp(),
            self.core.player_registry.as_ref(),
            self.core.directory.group_registry.as_ref(),
        )
        else {
            return;
        };

        let Some(inviter) = player_registry.social_recipient_by_name(inviter_name) else {
            return;
        };
        let inviter_guid = inviter.guid;

        let Some(player_group) = group_registry.get(&player_group_guid) else {
            return;
        };
        if !player_group.members.contains(&player_guid) {
            return;
        }
        let player_group_size = player_group.members.len();
        drop(player_group);

        let Some(inviter_group) = group_registry
            .snapshots()
            .into_iter()
            .find(|group| group.members.contains(&inviter_guid))
        else {
            return;
        };
        let inviter_group_guid = inviter_group.group_guid;
        let inviter_group_size = inviter_group.members.len();

        if player_group_size != inviter_group_size {
            return;
        }

        #[cfg(test)]
        self.fixtures
            .battleground
            .represented_wargame_invite_acceptances_like_cpp
            .push(RepresentedWargameInviteAcceptanceLikeCpp {
                inviter_name: inviter_name.to_string(),
                inviter_guid,
                player_group_guid,
                inviter_group_guid,
                group_size: player_group_size,
            });
    }

    pub(crate) fn represented_player_can_use_battleground_object_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
    ) -> bool {
        let gameobject_faction = self
            .world_entities
            .represented_gameobject_use_states
            .get(&gameobject_guid)
            .and_then(|state| state.faction_template);
        if let (Some(player_faction), Some(gameobject_faction), Some(store)) = (
            crate::session::hub_ref(self).player_faction_template_id_like_cpp(),
            gameobject_faction,
            self.catalogs.factions.template_store.as_ref(),
        ) && let (Some(player_entry), Some(gameobject_entry)) =
            (store.get(player_faction), store.get(gameobject_faction))
            && !player_entry.is_friendly_to_like_cpp(gameobject_entry)
        {
            self.world_entities.represented_gameobject_use_effects.push(
                RepresentedGameObjectUseEffect::BattlegroundObjectUseRejected {
                    gameobject_guid,
                    player_guid,
                    reason: RepresentedBattlegroundObjectUseRejection::UnfriendlyFaction,
                },
            );
            return false;
        }

        let Some((player_unit_flags, _, _)) =
            crate::session::hub_ref(self).player_unit_presentation_snapshot_like_cpp()
        else {
            return false;
        };
        if player_unit_flags.contains(UnitFlags::IMMUNE) {
            self.world_entities.represented_gameobject_use_effects.push(
                RepresentedGameObjectUseEffect::BattlegroundObjectUseRejected {
                    gameobject_guid,
                    player_guid,
                    reason: RepresentedBattlegroundObjectUseRejection::DamageImmune,
                },
            );
            return false;
        }

        let Some(has_recently_dropped_flag_debuff) =
            crate::session::hub_ref(self).has_recently_dropped_flag_debuff_like_cpp()
        else {
            return false;
        };
        if has_recently_dropped_flag_debuff {
            self.world_entities.represented_gameobject_use_effects.push(
                RepresentedGameObjectUseEffect::BattlegroundObjectUseRejected {
                    gameobject_guid,
                    player_guid,
                    reason: RepresentedBattlegroundObjectUseRejection::RecentlyDroppedFlag,
                },
            );
            return false;
        }

        let Some(player_is_alive) =
            crate::session::hub_ref(self).resolved_player_is_alive_like_cpp()
        else {
            return false;
        };
        if !player_is_alive {
            self.world_entities.represented_gameobject_use_effects.push(
                RepresentedGameObjectUseEffect::BattlegroundObjectUseRejected {
                    gameobject_guid,
                    player_guid,
                    reason: RepresentedBattlegroundObjectUseRejection::Dead,
                },
            );
            return false;
        }

        true
    }

    pub(in crate::session) fn represented_player_battleground_type_id_or_reject_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
    ) -> Option<u32> {
        let state = crate::session::hub_ref(self).player_battleground_state_snapshot_like_cpp()?;
        if let Some(bg_type_id) = state.battleground_type_id_like_cpp() {
            return Some(bg_type_id);
        }

        self.world_entities.represented_gameobject_use_effects.push(
            RepresentedGameObjectUseEffect::BattlegroundObjectUseRejected {
                gameobject_guid,
                player_guid,
                reason: RepresentedBattlegroundObjectUseRejection::NotInBattleground,
            },
        );
        None
    }

    pub(in crate::session) fn record_represented_capture_point_update_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        source: wow_entities::CapturePointUseSource,
        state: RepresentedCapturePointStateLikeCpp,
        broadcast_text_id: u32,
        event_id: u32,
        assault_timer_ms: u32,
    ) {
        let (custom_anim, spell_visual_id) = state.custom_anim_and_spell_visual_like_cpp(source);
        if let Some(position) = self
            .world_entities
            .represented_gameobject_use_states
            .get(&gameobject_guid)
            .and_then(|state| state.position)
        {
            self.send_packet(&wow_packet::packets::misc::UpdateCapturePoint {
                guid: gameobject_guid,
                position,
                state: state.packet_state_like_cpp(),
                capture_time_ms: assault_timer_ms,
                capture_total_duration_ms: source.capture_time_ms,
            });
        }
        self.world_entities.represented_gameobject_use_effects.push(
            RepresentedGameObjectUseEffect::CapturePointUpdated {
                gameobject_guid,
                state,
                broadcast_text_id,
                event_id,
                world_state_id: source.world_state_id,
                spell_visual_id,
                custom_anim,
                assault_timer_ms,
            },
        );
    }

    pub(crate) fn apply_represented_new_flag_state_command_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        player_guid: Option<ObjectGuid>,
        new_state: RepresentedNewFlagStateRequest,
        respawn_time_ms: u32,
    ) -> bool {
        let state = self
            .world_entities
            .represented_gameobject_use_states
            .entry(gameobject_guid)
            .or_default();
        let old_state = state
            .new_flag_state
            .unwrap_or(RepresentedNewFlagStateRequest::InBase);
        if old_state == new_state {
            return false;
        }

        state.new_flag_state = Some(new_state);
        state.new_flag_carrier_guid = if new_state == RepresentedNewFlagStateRequest::Taken {
            player_guid
        } else {
            None
        };

        if new_state == RepresentedNewFlagStateRequest::Taken
            && old_state == RepresentedNewFlagStateRequest::InBase
        {
            state.new_flag_taken_from_base_game_time_ms =
                Some(crate::session::game_time_ms_like_cpp());
        } else if matches!(
            new_state,
            RepresentedNewFlagStateRequest::InBase | RepresentedNewFlagStateRequest::Respawning
        ) {
            state.new_flag_taken_from_base_game_time_ms = None;
        }

        state.new_flag_respawn_until = (new_state == RepresentedNewFlagStateRequest::Respawning)
            .then(|| Instant::now() + Duration::from_millis(u64::from(respawn_time_ms)));

        self.world_entities.represented_gameobject_use_effects.push(
            RepresentedGameObjectUseEffect::NewFlagOwnerStateRequested {
                gameobject_guid,
                player_guid: player_guid.unwrap_or(ObjectGuid::EMPTY),
                state: new_state,
            },
        );
        true
    }
}

#[cfg(test)]
#[path = "../../unit_tests/session/battleground_adapter/f3_shims.rs"]
mod f3_shims;
