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

#[cfg(test)]
pub(crate) use wow_world_core::session::RepresentedWargameInviteAcceptanceLikeCpp;

pub(crate) use wow_world_social::{
    RepresentedCalendarAddEventLikeCpp, RepresentedCalendarCommunityInviteLikeCpp,
    RepresentedCalendarRemoveEventLikeCpp, RepresentedDeclinePetitionLikeCpp,
    RepresentedQueryPetitionLikeCpp, RepresentedSignPetitionLikeCpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
pub(crate) use wow_world_social::RepresentedSilencePartyTalkerLikeCpp;

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
        self.social.record_calendar_community_invite_for_test_like_cpp(
            RepresentedCalendarCommunityInviteLikeCpp {
                guild_id,
                min_level,
                max_level,
                max_rank_order,
            },
        );
        true
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
        self.social.record_calendar_add_event_for_test_like_cpp(
            RepresentedCalendarAddEventLikeCpp {
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
            },
        );
        true
    }

    #[cfg_attr(not(test), allow(unused_variables))]
    pub(crate) fn set_represented_arena_team_id_invited_like_cpp(
        &mut self,
        arena_team_id: u32,
    ) -> bool {
        let (state, mut hub) = crate::session::split_social_mut(self);
        state.set_represented_arena_team_id_invited_like_cpp(&mut hub, arena_team_id)
    }
}

#[cfg(test)]
#[path = "../../unit_tests/session/social_requests/f3_shims.rs"]
mod f3_shims;
