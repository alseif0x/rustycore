// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use wow_core::ObjectGuid;
use wow_data::{BattlemasterListStore, DISABLE_TYPE_BATTLEGROUND};

#[cfg(any(test, feature = "test-fixtures"))]
use crate::session::RepresentedWargameInviteAcceptanceLikeCpp;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepresentedBattlemasterHelloLikeCpp {
    pub unit: ObjectGuid,
    pub entry: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepresentedBattlefieldListLikeCpp {
    pub list_id: u32,
}

pub type RepresentedBattlegroundQueueTypeIdLikeCpp =
    wow_entities::PlayerBattlegroundQueueTypeIdLikeCpp;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepresentedBattlemasterJoinLikeCpp {
    pub packed_queue_id: u64,
    pub queue_type_id: RepresentedBattlegroundQueueTypeIdLikeCpp,
    pub roles: u8,
    pub blacklist_map: [i32; 2],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepresentedBattlemasterJoinArenaLikeCpp {
    pub team_size_index: u8,
    pub roles: u8,
    pub arena_type: u8,
    pub group_guid: u64,
    pub queue_type_id: RepresentedBattlegroundQueueTypeIdLikeCpp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepresentedBattlemasterJoinSkirmishLikeCpp {
    pub bg_type_id: u32,
    pub bracket_id: u32,
    pub as_group: bool,
    pub is_rated_packet_value: u8,
    pub arena_type: u8,
    pub group_guid: Option<u64>,
    pub queue_type_id: RepresentedBattlegroundQueueTypeIdLikeCpp,
}

#[cfg(any(test, feature = "test-fixtures"))]
pub type RepresentedBattlegroundQueueSlotLikeCpp =
    wow_entities::PlayerBattlegroundQueueSlotLikeCpp;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepresentedBattlefieldPortLikeCpp {
    pub ticket: wow_packet::packets::misc::LfgRideTicket,
    pub accepted_invite: bool,
    pub queue_type_id: RepresentedBattlegroundQueueTypeIdLikeCpp,
    pub invited_instance_guid: u32,
}

pub fn battleground_queue_type_id_from_packed_like_cpp(
    packed_queue_id: u64,
) -> RepresentedBattlegroundQueueTypeIdLikeCpp {
    RepresentedBattlegroundQueueTypeIdLikeCpp {
        battlemaster_list_id: (packed_queue_id & 0xFFFF) as u16,
        queue_type: ((packed_queue_id >> 16) & 0xF) as u8,
        rated: ((packed_queue_id >> 20) & 1) != 0,
        team_size: ((packed_queue_id >> 24) & 0x3F) as u8,
    }
}

#[cfg(any(test, feature = "test-fixtures"))]
impl crate::session::state::BattlegroundState {
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_battlemaster_hellos_like_cpp(
        &self,
    ) -> &[RepresentedBattlemasterHelloLikeCpp] {
        &self.represented_battlemaster_hellos_like_cpp
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_battlefield_lists_like_cpp(
        &self,
    ) -> &[RepresentedBattlefieldListLikeCpp] {
        &self.represented_battlefield_lists_like_cpp
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_battlemaster_joins_like_cpp(
        &self,
    ) -> &[RepresentedBattlemasterJoinLikeCpp] {
        &self.represented_battlemaster_joins_like_cpp
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_battlemaster_join_arenas_like_cpp(
        &self,
    ) -> &[RepresentedBattlemasterJoinArenaLikeCpp] {
        &self.represented_battlemaster_join_arenas_like_cpp
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_battlemaster_join_skirmishes_like_cpp(
        &self,
    ) -> &[RepresentedBattlemasterJoinSkirmishLikeCpp] {
        &self.represented_battlemaster_join_skirmishes_like_cpp
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_battlefield_ports_like_cpp(
        &self,
    ) -> &[RepresentedBattlefieldPortLikeCpp] {
        &self.represented_battlefield_ports_like_cpp
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_wargame_invite_acceptances_like_cpp(
        &self,
    ) -> &[RepresentedWargameInviteAcceptanceLikeCpp] {
        &self.represented_wargame_invite_acceptances_like_cpp
    }
}

impl crate::session::HubRef<'_> {
    pub fn represented_battleground_status_is_wait_leave_like_cpp(&self) -> bool {
        self.player_battleground_state_snapshot_like_cpp()
            .is_some_and(|state| state.battleground_status_like_cpp() == Some(4))
    }

    pub fn is_valid_battleground_queue_type_id_like_cpp(
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

    pub fn has_recently_dropped_flag_debuff_like_cpp(&self) -> Option<bool> {
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
}

impl crate::session::HubMut<'_> {
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_player_battleground_type_id_like_cpp(&mut self, bg_type_id: u32) -> bool {
        let canonical = self
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player.set_battleground_type_id_like_cpp(bg_type_id)
            })
            .is_some();
        if !canonical && self.core.player_handle_like_cpp.is_none() {
            return self
                .mutate_player_battleground_state_like_cpp(|state| {
                    state.set_battleground_type_id_like_cpp(bg_type_id);
                })
                .is_some();
        }
        canonical
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_player_battleground_context_like_cpp(
        &mut self,
        bg_type_id: u32,
        bg_map_id: u32,
    ) -> bool {
        let canonical = self
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player.set_battleground_context_like_cpp(bg_type_id, bg_map_id)
            })
            .is_some();
        if !canonical && self.core.player_handle_like_cpp.is_none() {
            return self
                .mutate_player_battleground_state_like_cpp(|state| {
                    state.set_battleground_context_like_cpp(bg_type_id, bg_map_id);
                })
                .is_some();
        }
        canonical
    }

    #[cfg_attr(not(any(test, feature = "test-fixtures")), allow(unused_variables))]
    pub fn set_represented_battleground_status_like_cpp(&mut self, status: Option<u8>) {
        let canonical = self
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player.set_battleground_status_like_cpp(status)
            })
            .is_some();
        #[cfg(any(test, feature = "test-fixtures"))]
        if !canonical && self.core.player_handle_like_cpp.is_none() {
            let _ = self.mutate_player_battleground_state_like_cpp(|state| {
                state.set_battleground_status_like_cpp(status);
            });
        }
    }

    #[cfg_attr(not(any(test, feature = "test-fixtures")), allow(unused_variables))]
    pub fn battlemaster_hello_like_cpp(&mut self, unit: ObjectGuid) -> bool {
        let Some((npc_flags, entry)) = self
            .core
            .mutate_world_creature(unit, |creature| (creature.npc_flags(), creature.entry()))
        else {
            return false;
        };

        if (npc_flags & wow_constants::unit::NPCFlags1::BATTLE_MASTER.bits()) == 0 {
            return false;
        }

        #[cfg(any(test, feature = "test-fixtures"))]
        self.fixtures
            .battleground
            .represented_battlemaster_hellos_like_cpp
            .push(RepresentedBattlemasterHelloLikeCpp { unit, entry });
        true
    }

    #[cfg_attr(not(any(test, feature = "test-fixtures")), allow(unused_variables))]
    pub fn battlefield_list_like_cpp(
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

        #[cfg(any(test, feature = "test-fixtures"))]
        self.fixtures
            .battleground
            .represented_battlefield_lists_like_cpp
            .push(RepresentedBattlefieldListLikeCpp { list_id });
        true
    }

    #[cfg_attr(not(any(test, feature = "test-fixtures")), allow(unused_variables))]
    pub fn battlemaster_join_like_cpp(
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
        if !self
            .shared()
            .is_valid_battleground_queue_type_id_like_cpp(battlemaster_lists, queue_type_id)
        {
            return false;
        }
        if battlemaster_lists
            .is_internal_only_like_cpp(u32::from(queue_type_id.battlemaster_list_id))
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
        if self.shared().player_in_represented_battleground_like_cpp() {
            return false;
        }

        #[cfg(any(test, feature = "test-fixtures"))]
        self.fixtures
            .battleground
            .represented_battlemaster_joins_like_cpp
            .push(RepresentedBattlemasterJoinLikeCpp {
                packed_queue_id,
                queue_type_id,
                roles,
                blacklist_map,
            });
        true
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn add_represented_battleground_queue_slot_like_cpp(
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
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player.install_battleground_queue_slot_like_cpp(queue_slot)
            })
            .is_some();
        if !canonical && self.core.player_handle_like_cpp.is_none() {
            let _ = self.mutate_player_battleground_state_like_cpp(|state| {
                state.install_queue_slot_like_cpp(queue_slot);
            });
        }
    }

    #[cfg_attr(not(any(test, feature = "test-fixtures")), allow(unused_variables))]
    pub fn battlefield_port_like_cpp(
        &mut self,
        ticket: wow_packet::packets::misc::LfgRideTicket,
        accepted_invite: bool,
    ) -> bool {
        let Some(state) = self.shared().player_battleground_state_snapshot_like_cpp() else {
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

        #[cfg(any(test, feature = "test-fixtures"))]
        self.fixtures
            .battleground
            .represented_battlefield_ports_like_cpp
            .push(RepresentedBattlefieldPortLikeCpp {
                ticket,
                accepted_invite,
                queue_type_id: queued.queue_type_id,
                invited_instance_guid: queued.invited_instance_guid,
            });
        true
    }
}

impl crate::session::HubRef<'_> {
    pub fn player_battleground_state_snapshot_like_cpp(
        &self,
    ) -> Option<wow_entities::PlayerBattlegroundState> {
        let canonical = self
            .core
            .with_owned_player_like_cpp(|player| player.battleground_state_like_cpp());
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            return Some(
                wow_entities::PlayerBattlegroundState::from_represented_parts_like_cpp(
                    self.fixtures
                        .battleground
                        .player_battleground_type_id_like_cpp,
                    self.fixtures
                        .battleground
                        .player_battleground_map_id_like_cpp,
                    self.fixtures
                        .battleground
                        .represented_battleground_status_like_cpp,
                    self.fixtures
                        .battleground
                        .represented_battleground_queue_slots_like_cpp
                        .clone(),
                    self.fixtures
                        .battleground
                        .represented_arena_team_id_invited_like_cpp,
                ),
            );
        }
        canonical
    }

    pub fn player_in_represented_battleground_like_cpp(&self) -> bool {
        self.player_battleground_state_snapshot_like_cpp()
            .is_some_and(|state| state.in_battleground_like_cpp())
    }
}
