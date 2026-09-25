// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Social requests: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::{ObjectGuid, PowerType, WorldSession};

pub(in crate::session) fn party_member_power_kind_from_u8_like_cpp(power: u8) -> PowerType {
    match power {
        1 => PowerType::Rage,
        2 => PowerType::Focus,
        3 => PowerType::Energy,
        4 => PowerType::Happiness,
        5 => PowerType::Runes,
        6 => PowerType::RunicPower,
        7 => PowerType::SoulShards,
        8 => PowerType::LunarPower,
        9 => PowerType::HolyPower,
        10 => PowerType::AlternatePower,
        11 => PowerType::Maelstrom,
        12 => PowerType::Chi,
        13 => PowerType::Insanity,
        14 => PowerType::ComboPoints,
        15 => PowerType::DemonicFury,
        16 => PowerType::ArcaneCharges,
        17 => PowerType::Fury,
        18 => PowerType::Pain,
        19 => PowerType::Essence,
        20 => PowerType::RuneBlood,
        21 => PowerType::RuneFrost,
        22 => PowerType::RuneUnholy,
        23 => PowerType::AlternateQuest,
        24 => PowerType::AlternateEncounter,
        25 => PowerType::AlternateMount,
        _ => PowerType::Mana,
    }
}

pub(in crate::session) fn party_member_power_to_u16_like_cpp(value: i32) -> u16 {
    u16::try_from(value.max(0)).unwrap_or(u16::MAX)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RepresentedWargameInviteAcceptanceLikeCpp {
    pub inviter_name: String,
    pub inviter_guid: ObjectGuid,
    pub player_group_guid: u64,
    pub inviter_group_guid: u64,
    pub group_size: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedSignPetitionLikeCpp {
    pub petition_guid: ObjectGuid,
    pub choice: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedDeclinePetitionLikeCpp {
    pub petition_guid: ObjectGuid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedQueryPetitionLikeCpp {
    pub petition_id: u32,
    pub item_guid: ObjectGuid,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedSilencePartyTalkerLikeCpp {
    pub target: ObjectGuid,
    pub silent: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RepresentedCalendarCommunityInviteLikeCpp {
    pub guild_id: u64,
    pub min_level: u8,
    pub max_level: u8,
    pub max_rank_order: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RepresentedCalendarAddEventLikeCpp {
    pub guild_id: Option<u64>,
    pub club_id: u64,
    pub event_type: u8,
    pub texture_id: i32,
    pub time_packed: u32,
    pub flags: u32,
    pub invite_count: usize,
    pub title: String,
    pub description: String,
    pub max_size: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedCalendarRemoveEventLikeCpp {
    pub event_id: u64,
}

impl WorldSession {
    #[cfg_attr(not(test), allow(unused_variables))]
    pub(crate) fn calendar_community_invite_like_cpp(
        &mut self,
        min_level: u8,
        max_level: u8,
        max_rank_order: u8,
    ) -> bool {
        let Some(guild_id) = self.resolved_represented_guild_id_like_cpp() else {
            return false;
        };
        if guild_id == 0 {
            return false;
        }

        #[cfg(test)]
        self.calendar_test_fixture_like_cpp
            .represented_calendar_community_invites_like_cpp
            .push(RepresentedCalendarCommunityInviteLikeCpp {
                guild_id,
                min_level,
                max_level,
                max_rank_order,
            });
        true
    }

    #[cfg(test)]
    pub(crate) fn represented_calendar_community_invites_like_cpp(
        &self,
    ) -> &[RepresentedCalendarCommunityInviteLikeCpp] {
        &self
            .calendar_test_fixture_like_cpp
            .represented_calendar_community_invites_like_cpp
    }

    #[cfg_attr(not(test), allow(unused_variables))]
    pub(crate) fn calendar_add_event_like_cpp(
        &mut self,
        club_id: u64,
        event_type: u8,
        texture_id: i32,
        time_packed: u32,
        flags: u32,
        invite_count: usize,
        title: String,
        description: String,
        max_size: u32,
    ) -> bool {
        const CALENDAR_FLAG_WITHOUT_INVITES_LIKE_CPP: u32 = 0x040;
        const CALENDAR_FLAG_GUILD_EVENT_LIKE_CPP: u32 = 0x400;

        let guild_scoped = (flags
            & (CALENDAR_FLAG_GUILD_EVENT_LIKE_CPP | CALENDAR_FLAG_WITHOUT_INVITES_LIKE_CPP))
            != 0;
        let guild_id = if guild_scoped {
            let Some(resolved_guild_id) = self.resolved_represented_guild_id_like_cpp() else {
                return false;
            };
            if resolved_guild_id == 0 {
                return false;
            }
            Some(resolved_guild_id)
        } else {
            None
        };

        #[cfg(test)]
        self.calendar_test_fixture_like_cpp
            .represented_calendar_add_events_like_cpp
            .push(RepresentedCalendarAddEventLikeCpp {
                guild_id,
                club_id,
                event_type,
                texture_id,
                time_packed,
                flags,
                invite_count,
                title,
                description,
                max_size,
            });
        true
    }

    #[cfg(test)]
    pub(crate) fn represented_calendar_add_events_like_cpp(
        &self,
    ) -> &[RepresentedCalendarAddEventLikeCpp] {
        &self
            .calendar_test_fixture_like_cpp
            .represented_calendar_add_events_like_cpp
    }

    #[cfg_attr(not(test), allow(unused_variables))]
    pub(crate) fn set_represented_arena_team_id_invited_like_cpp(
        &mut self,
        arena_team_id: u32,
    ) -> bool {
        let canonical = self
            .with_owned_player_mut_like_cpp(|player| {
                player.set_arena_team_id_invited_like_cpp(arena_team_id)
            })
            .is_some();
        #[cfg(test)]
        if !canonical && self.player_handle_like_cpp.is_none() {
            return self
                .mutate_player_battleground_state_like_cpp(|state| {
                    state.set_arena_team_id_invited_like_cpp(arena_team_id);
                })
                .is_some();
        }
        canonical
    }

    #[cfg(test)]
    pub(crate) fn represented_arena_team_id_invited_like_cpp(&self) -> u32 {
        self.player_battleground_state_snapshot_like_cpp()
            .expect("test Player battleground owner must resolve")
            .arena_team_id_invited_like_cpp()
    }
}
