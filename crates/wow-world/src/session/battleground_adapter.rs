// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Battleground adapter: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::{BattlemasterListStore, DISABLE_TYPE_BATTLEGROUND, Duration, Instant, ObjectGuid};
use super::{RepresentedBattlegroundObjectUseRejection, RepresentedCapturePointStateLikeCpp};
use super::{RepresentedGameObjectUseEffect, RepresentedNewFlagStateRequest};
use super::{RepresentedWargameInviteAcceptanceLikeCpp, UnitFlags, WorldSession};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedBattlemasterHelloLikeCpp {
    pub unit: ObjectGuid,
    pub entry: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedBattlefieldListLikeCpp {
    pub list_id: u32,
}

pub(crate) type RepresentedBattlegroundQueueTypeIdLikeCpp =
    wow_entities::PlayerBattlegroundQueueTypeIdLikeCpp;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RepresentedBattlemasterJoinLikeCpp {
    pub packed_queue_id: u64,
    pub queue_type_id: RepresentedBattlegroundQueueTypeIdLikeCpp,
    pub roles: u8,
    pub blacklist_map: [i32; 2],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RepresentedBattlemasterJoinArenaLikeCpp {
    pub team_size_index: u8,
    pub roles: u8,
    pub arena_type: u8,
    pub group_guid: u64,
    pub queue_type_id: RepresentedBattlegroundQueueTypeIdLikeCpp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RepresentedBattlemasterJoinSkirmishLikeCpp {
    pub bg_type_id: u32,
    pub bracket_id: u32,
    pub as_group: bool,
    pub is_rated_packet_value: u8,
    pub arena_type: u8,
    pub group_guid: Option<u64>,
    pub queue_type_id: RepresentedBattlegroundQueueTypeIdLikeCpp,
}

#[cfg(test)]
pub(crate) type RepresentedBattlegroundQueueSlotLikeCpp =
    wow_entities::PlayerBattlegroundQueueSlotLikeCpp;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RepresentedBattlefieldPortLikeCpp {
    pub ticket: wow_packet::packets::misc::LfgRideTicket,
    pub accepted_invite: bool,
    pub queue_type_id: RepresentedBattlegroundQueueTypeIdLikeCpp,
    pub invited_instance_guid: u32,
}

pub(crate) fn battleground_queue_type_id_from_packed_like_cpp(
    packed_queue_id: u64,
) -> RepresentedBattlegroundQueueTypeIdLikeCpp {
    RepresentedBattlegroundQueueTypeIdLikeCpp {
        battlemaster_list_id: (packed_queue_id & 0xFFFF) as u16,
        queue_type: ((packed_queue_id >> 16) & 0xF) as u8,
        rated: ((packed_queue_id >> 20) & 1) != 0,
        team_size: ((packed_queue_id >> 24) & 0x3F) as u8,
    }
}

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
    pub(in crate::session) fn player_battleground_state_snapshot_like_cpp(
        &self,
    ) -> Option<wow_entities::PlayerBattlegroundState> {
        let canonical =
            self.with_owned_player_like_cpp(|player| player.battleground_state_like_cpp());
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(
                wow_entities::PlayerBattlegroundState::from_represented_parts_like_cpp(
                    self.player_battleground_type_id_like_cpp,
                    self.player_battleground_map_id_like_cpp,
                    self.represented_battleground_status_like_cpp,
                    self.represented_battleground_queue_slots_like_cpp.clone(),
                    self.represented_arena_team_id_invited_like_cpp,
                ),
            );
        }
        canonical
    }

    #[cfg(test)]
    pub(crate) fn set_player_battleground_type_id_like_cpp(&mut self, bg_type_id: u32) -> bool {
        let canonical = self
            .with_owned_player_mut_like_cpp(|player| {
                player.set_battleground_type_id_like_cpp(bg_type_id)
            })
            .is_some();
        if !canonical && self.player_handle_like_cpp.is_none() {
            return self
                .mutate_player_battleground_state_like_cpp(|state| {
                    state.set_battleground_type_id_like_cpp(bg_type_id);
                })
                .is_some();
        }
        canonical
    }

    #[cfg(test)]
    pub(crate) fn set_player_battleground_context_like_cpp(
        &mut self,
        bg_type_id: u32,
        bg_map_id: u32,
    ) -> bool {
        let canonical = self
            .with_owned_player_mut_like_cpp(|player| {
                player.set_battleground_context_like_cpp(bg_type_id, bg_map_id)
            })
            .is_some();
        if !canonical && self.player_handle_like_cpp.is_none() {
            return self
                .mutate_player_battleground_state_like_cpp(|state| {
                    state.set_battleground_context_like_cpp(bg_type_id, bg_map_id);
                })
                .is_some();
        }
        canonical
    }

    #[cfg_attr(not(test), allow(unused_variables))]
    pub(crate) fn set_represented_battleground_status_like_cpp(&mut self, status: Option<u8>) {
        let canonical = self
            .with_owned_player_mut_like_cpp(|player| {
                player.set_battleground_status_like_cpp(status)
            })
            .is_some();
        #[cfg(test)]
        if !canonical && self.player_handle_like_cpp.is_none() {
            let _ = self.mutate_player_battleground_state_like_cpp(|state| {
                state.set_battleground_status_like_cpp(status);
            });
        }
    }

    pub(crate) fn player_in_represented_battleground_like_cpp(&self) -> bool {
        self.player_battleground_state_snapshot_like_cpp()
            .is_some_and(|state| state.in_battleground_like_cpp())
    }

    pub(crate) fn represented_battleground_status_is_wait_leave_like_cpp(&self) -> bool {
        self.player_battleground_state_snapshot_like_cpp()
            .is_some_and(|state| state.battleground_status_like_cpp() == Some(4))
    }

    #[cfg_attr(not(test), allow(unused_variables))]
    pub(crate) fn battlemaster_hello_like_cpp(&mut self, unit: ObjectGuid) -> bool {
        let Some((npc_flags, entry)) =
            self.mutate_world_creature(unit, |creature| (creature.npc_flags(), creature.entry()))
        else {
            return false;
        };

        if (npc_flags & wow_constants::unit::NPCFlags1::BATTLE_MASTER.bits()) == 0 {
            return false;
        }

        #[cfg(test)]
        self.represented_battlemaster_hellos_like_cpp
            .push(RepresentedBattlemasterHelloLikeCpp { unit, entry });
        true
    }

    #[cfg_attr(not(test), allow(unused_variables))]
    pub(crate) fn battlefield_list_like_cpp(
        &mut self,
        battlemaster_lists: &BattlemasterListStore,
        list_id: i32,
    ) -> bool {
        let Ok(list_id) = u32::try_from(list_id) else {
            return false;
        };
        if battlemaster_lists.get(list_id).is_none() {
            return false;
        }

        #[cfg(test)]
        self.represented_battlefield_lists_like_cpp
            .push(RepresentedBattlefieldListLikeCpp { list_id });
        true
    }

    #[cfg_attr(not(test), allow(unused_variables))]
    pub(crate) fn battlemaster_join_like_cpp(
        &mut self,
        battlemaster_lists: &BattlemasterListStore,
        queue_ids: &[u64],
        roles: u8,
        blacklist_map: [i32; 2],
    ) -> bool {
        let Some(&packed_queue_id) = queue_ids.first() else {
            return false;
        };
        let queue_type_id = battleground_queue_type_id_from_packed_like_cpp(packed_queue_id);
        if !self.is_valid_battleground_queue_type_id_like_cpp(battlemaster_lists, queue_type_id) {
            return false;
        }
        if battlemaster_lists
            .is_internal_only_like_cpp(u32::from(queue_type_id.battlemaster_list_id))
        {
            return false;
        }
        if self
            .disable_mgr
            .as_ref()
            .map(|disable_mgr| {
                disable_mgr.is_disabled_for_like_cpp(
                    DISABLE_TYPE_BATTLEGROUND,
                    u32::from(queue_type_id.battlemaster_list_id),
                    None,
                    0,
                    None,
                )
            })
            .unwrap_or(false)
        {
            return false;
        }
        if self.player_in_represented_battleground_like_cpp() {
            return false;
        }

        #[cfg(test)]
        self.represented_battlemaster_joins_like_cpp
            .push(RepresentedBattlemasterJoinLikeCpp {
                packed_queue_id,
                queue_type_id,
                roles,
                blacklist_map,
            });
        true
    }

    #[cfg_attr(not(test), allow(unused_variables))]
    pub(crate) fn battlemaster_join_arena_like_cpp(
        &mut self,
        battlemaster_lists: &BattlemasterListStore,
        team_size_index: u8,
        roles: u8,
    ) -> bool {
        if self.player_in_represented_battleground_like_cpp() {
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
        if !self.is_valid_battleground_queue_type_id_like_cpp(battlemaster_lists, queue_type_id) {
            return false;
        }
        if self
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
            self.group_registry.as_ref(),
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
        self.represented_battlemaster_join_arenas_like_cpp.push(
            RepresentedBattlemasterJoinArenaLikeCpp {
                team_size_index,
                roles,
                arena_type,
                group_guid,
                queue_type_id,
            },
        );
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
        if self.player_in_represented_battleground_like_cpp() {
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
                self.group_registry.as_ref(),
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
        self.represented_battlemaster_join_skirmishes_like_cpp.push(
            RepresentedBattlemasterJoinSkirmishLikeCpp {
                bg_type_id,
                bracket_id,
                as_group: join_as_group,
                is_rated_packet_value: is_rated,
                arena_type,
                group_guid,
                queue_type_id,
            },
        );
        true
    }

    #[cfg(test)]
    pub(crate) fn add_represented_battleground_queue_slot_like_cpp(
        &mut self,
        slot: u32,
        queue_type_id: RepresentedBattlegroundQueueTypeIdLikeCpp,
        invited_instance_guid: u32,
    ) {
        let queue_slot = RepresentedBattlegroundQueueSlotLikeCpp {
            slot,
            queue_type_id,
            invited_instance_guid,
        };
        let canonical = self
            .with_owned_player_mut_like_cpp(|player| {
                player.install_battleground_queue_slot_like_cpp(queue_slot)
            })
            .is_some();
        if !canonical && self.player_handle_like_cpp.is_none() {
            let _ = self.mutate_player_battleground_state_like_cpp(|state| {
                state.install_queue_slot_like_cpp(queue_slot);
            });
        }
    }

    #[cfg_attr(not(test), allow(unused_variables))]
    pub(crate) fn battlefield_port_like_cpp(
        &mut self,
        ticket: wow_packet::packets::misc::LfgRideTicket,
        accepted_invite: bool,
    ) -> bool {
        let Some(state) = self.player_battleground_state_snapshot_like_cpp() else {
            return false;
        };
        if state.has_no_queue_slot_like_cpp() {
            return false;
        }
        let Some(queued) = state
            .queue_slots_like_cpp()
            .iter()
            .copied()
            .find(|queued| queued.slot == ticket.id)
        else {
            return false;
        };
        if accepted_invite && queued.invited_instance_guid == 0 {
            return false;
        }

        #[cfg(test)]
        self.represented_battlefield_ports_like_cpp
            .push(RepresentedBattlefieldPortLikeCpp {
                ticket,
                accepted_invite,
                queue_type_id: queued.queue_type_id,
                invited_instance_guid: queued.invited_instance_guid,
            });
        true
    }

    pub(in crate::session) fn is_valid_battleground_queue_type_id_like_cpp(
        &self,
        battlemaster_lists: &BattlemasterListStore,
        queue_type_id: RepresentedBattlegroundQueueTypeIdLikeCpp,
    ) -> bool {
        let Some(entry) = battlemaster_lists.get(u32::from(queue_type_id.battlemaster_list_id))
        else {
            return false;
        };

        match queue_type_id.queue_type {
            0 => {
                entry.instance_type == wow_data::MAP_BATTLEGROUND_LIKE_CPP
                    && queue_type_id.team_size == 0
            }
            1 => {
                entry.instance_type == wow_data::MAP_ARENA_LIKE_CPP
                    && queue_type_id.rated
                    && queue_type_id.team_size != 0
            }
            2 => !queue_type_id.rated,
            4 => {
                entry.instance_type == wow_data::MAP_ARENA_LIKE_CPP
                    && queue_type_id.rated
                    && queue_type_id.team_size == 3
            }
            _ => false,
        }
    }

    #[cfg(test)]
    pub(crate) fn represented_battlemaster_hellos_like_cpp(
        &self,
    ) -> &[RepresentedBattlemasterHelloLikeCpp] {
        &self.represented_battlemaster_hellos_like_cpp
    }

    #[cfg(test)]
    pub(crate) fn represented_battlefield_lists_like_cpp(
        &self,
    ) -> &[RepresentedBattlefieldListLikeCpp] {
        &self.represented_battlefield_lists_like_cpp
    }

    #[cfg(test)]
    pub(crate) fn represented_battlemaster_joins_like_cpp(
        &self,
    ) -> &[RepresentedBattlemasterJoinLikeCpp] {
        &self.represented_battlemaster_joins_like_cpp
    }

    #[cfg(test)]
    pub(crate) fn represented_battlemaster_join_arenas_like_cpp(
        &self,
    ) -> &[RepresentedBattlemasterJoinArenaLikeCpp] {
        &self.represented_battlemaster_join_arenas_like_cpp
    }

    #[cfg(test)]
    pub(crate) fn represented_battlemaster_join_skirmishes_like_cpp(
        &self,
    ) -> &[RepresentedBattlemasterJoinSkirmishLikeCpp] {
        &self.represented_battlemaster_join_skirmishes_like_cpp
    }

    #[cfg(test)]
    pub(crate) fn represented_battlefield_ports_like_cpp(
        &self,
    ) -> &[RepresentedBattlefieldPortLikeCpp] {
        &self.represented_battlefield_ports_like_cpp
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
            self.player_registry.as_ref(),
            self.group_registry.as_ref(),
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
        self.represented_wargame_invite_acceptances_like_cpp.push(
            RepresentedWargameInviteAcceptanceLikeCpp {
                inviter_name: inviter_name.to_string(),
                inviter_guid,
                player_group_guid,
                inviter_group_guid,
                group_size: player_group_size,
            },
        );
    }

    #[cfg(test)]
    pub(crate) fn represented_wargame_invite_acceptances_like_cpp(
        &self,
    ) -> &[RepresentedWargameInviteAcceptanceLikeCpp] {
        &self.represented_wargame_invite_acceptances_like_cpp
    }

    pub(in crate::session) fn has_recently_dropped_flag_debuff_like_cpp(&self) -> Option<bool> {
        const SPELL_RECENTLY_DROPPED_ALLIANCE_FLAG: i32 = 42_792;
        const SPELL_RECENTLY_DROPPED_HORDE_FLAG: i32 = 50_326;
        const SPELL_RECENTLY_DROPPED_NEUTRAL_FLAG: i32 = 50_327;

        self.resolved_player_visible_auras_like_cpp().map(|auras| {
            auras.values().any(|aura| {
                matches!(
                    aura.spell_id,
                    SPELL_RECENTLY_DROPPED_ALLIANCE_FLAG
                        | SPELL_RECENTLY_DROPPED_HORDE_FLAG
                        | SPELL_RECENTLY_DROPPED_NEUTRAL_FLAG
                )
            })
        })
    }

    pub(crate) fn represented_player_can_use_battleground_object_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
    ) -> bool {
        let gameobject_faction = self
            .represented_gameobject_use_states
            .get(&gameobject_guid)
            .and_then(|state| state.faction_template);
        if let (Some(player_faction), Some(gameobject_faction), Some(store)) = (
            self.player_faction_template_id_like_cpp(),
            gameobject_faction,
            self.factions.template_store.as_ref(),
        ) && let (Some(player_entry), Some(gameobject_entry)) =
            (store.get(player_faction), store.get(gameobject_faction))
            && !player_entry.is_friendly_to_like_cpp(gameobject_entry)
        {
            self.represented_gameobject_use_effects.push(
                RepresentedGameObjectUseEffect::BattlegroundObjectUseRejected {
                    gameobject_guid,
                    player_guid,
                    reason: RepresentedBattlegroundObjectUseRejection::UnfriendlyFaction,
                },
            );
            return false;
        }

        let Some((player_unit_flags, _, _)) = self.player_unit_presentation_snapshot_like_cpp()
        else {
            return false;
        };
        if player_unit_flags.contains(UnitFlags::IMMUNE) {
            self.represented_gameobject_use_effects.push(
                RepresentedGameObjectUseEffect::BattlegroundObjectUseRejected {
                    gameobject_guid,
                    player_guid,
                    reason: RepresentedBattlegroundObjectUseRejection::DamageImmune,
                },
            );
            return false;
        }

        let Some(has_recently_dropped_flag_debuff) =
            self.has_recently_dropped_flag_debuff_like_cpp()
        else {
            return false;
        };
        if has_recently_dropped_flag_debuff {
            self.represented_gameobject_use_effects.push(
                RepresentedGameObjectUseEffect::BattlegroundObjectUseRejected {
                    gameobject_guid,
                    player_guid,
                    reason: RepresentedBattlegroundObjectUseRejection::RecentlyDroppedFlag,
                },
            );
            return false;
        }

        let Some(player_is_alive) = self.resolved_player_is_alive_like_cpp() else {
            return false;
        };
        if !player_is_alive {
            self.represented_gameobject_use_effects.push(
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
        let state = self.player_battleground_state_snapshot_like_cpp()?;
        if let Some(bg_type_id) = state.battleground_type_id_like_cpp() {
            return Some(bg_type_id);
        }

        self.represented_gameobject_use_effects.push(
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
        self.represented_gameobject_use_effects.push(
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

        self.represented_gameobject_use_effects.push(
            RepresentedGameObjectUseEffect::NewFlagOwnerStateRequested {
                gameobject_guid,
                player_guid: player_guid.unwrap_or(ObjectGuid::EMPTY),
                state: new_state,
            },
        );
        true
    }
}
