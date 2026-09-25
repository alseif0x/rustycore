//! Test-only detached social state owned by the Session fixture.

use super::super::{ObjectGuid, TRADE_SLOT_COUNT_LIKE_CPP};
use super::super::{
    RepresentedCalendarAddEventLikeCpp, RepresentedCalendarCommunityInviteLikeCpp,
    RepresentedCalendarRemoveEventLikeCpp,
};
use super::super::{
    RepresentedCanDuelSpellCastLikeCpp, RepresentedDuelAcceptedLikeCpp,
    RepresentedDuelCancelledLikeCpp, RepresentedDuelRequestedLikeCpp,
    RepresentedForceDeselectLikeCpp,
};

/// Handle-less fixture for Player duel state and its test evidence.
pub(crate) struct DuelTestFixtureLikeCpp {
    pub(in crate::session) represented_can_duel_spell_casts_like_cpp:
        Vec<RepresentedCanDuelSpellCastLikeCpp>,
    pub(in crate::session) represented_duel_arbiter_guid_like_cpp: Option<ObjectGuid>,
    pub(in crate::session) represented_duel_requests_like_cpp: Vec<RepresentedDuelRequestedLikeCpp>,
    pub(in crate::session) represented_force_deselects_like_cpp:
        Vec<RepresentedForceDeselectLikeCpp>,
    pub(in crate::session) represented_duel_accepts_like_cpp: Vec<RepresentedDuelAcceptedLikeCpp>,
    pub(in crate::session) represented_duel_cancels_like_cpp: Vec<RepresentedDuelCancelledLikeCpp>,
}

impl Default for DuelTestFixtureLikeCpp {
    fn default() -> Self {
        Self {
            represented_can_duel_spell_casts_like_cpp: Vec::new(),
            represented_duel_arbiter_guid_like_cpp: None,
            represented_duel_requests_like_cpp: Vec::new(),
            represented_force_deselects_like_cpp: Vec::new(),
            represented_duel_accepts_like_cpp: Vec::new(),
            represented_duel_cancels_like_cpp: Vec::new(),
        }
    }
}

/// Test evidence for represented Calendar request handlers.
pub(crate) struct CalendarTestFixtureLikeCpp {
    pub(in crate::session) represented_calendar_community_invites_like_cpp:
        Vec<RepresentedCalendarCommunityInviteLikeCpp>,
    pub(in crate::session) represented_calendar_add_events_like_cpp:
        Vec<RepresentedCalendarAddEventLikeCpp>,
    pub(in crate::session) represented_calendar_remove_events_like_cpp:
        Vec<RepresentedCalendarRemoveEventLikeCpp>,
}

impl Default for CalendarTestFixtureLikeCpp {
    fn default() -> Self {
        Self {
            represented_calendar_community_invites_like_cpp: Vec::new(),
            represented_calendar_add_events_like_cpp: Vec::new(),
            represented_calendar_remove_events_like_cpp: Vec::new(),
        }
    }
}

/// Handle-less fixture for Player-owned guild membership and invitations.
pub(crate) struct GuildTestFixtureLikeCpp {
    pub(in crate::session) represented_guild_id_like_cpp: u64,
    pub(in crate::session) represented_guild_id_authority_complete_like_cpp: bool,
    pub(in crate::session) represented_guild_id_invited_like_cpp: u64,
    pub(in crate::session) represented_guild_accept_invites_like_cpp: Vec<u64>,
}

impl Default for GuildTestFixtureLikeCpp {
    fn default() -> Self {
        Self {
            represented_guild_id_like_cpp: 0,
            represented_guild_id_authority_complete_like_cpp: false,
            represented_guild_id_invited_like_cpp: 0,
            represented_guild_accept_invites_like_cpp: Vec::new(),
        }
    }
}

/// Handle-less fixture for state canonically owned by C++ `Player::TradeData`.
pub(crate) struct TradeTestFixtureLikeCpp {
    pub(in crate::session) represented_active_trade_partner_like_cpp: Option<ObjectGuid>,
    pub(in crate::session) represented_trade_accepted_like_cpp: bool,
    pub(in crate::session) represented_partner_trade_server_state_index_like_cpp: u32,
    pub(in crate::session) represented_trade_client_state_index_like_cpp: u32,
    pub(in crate::session) represented_trade_server_state_index_like_cpp: u32,
    pub(in crate::session) represented_trade_items_like_cpp:
        [Option<ObjectGuid>; TRADE_SLOT_COUNT_LIKE_CPP as usize],
    pub(in crate::session) represented_trade_money_like_cpp: u64,
    pub(in crate::session) represented_trade_spell_like_cpp: u32,
    pub(in crate::session) represented_trade_spell_cast_item_like_cpp: Option<ObjectGuid>,
    pub(in crate::session) represented_trade_cancel_statuses_like_cpp: Vec<u8>,
}

impl Default for TradeTestFixtureLikeCpp {
    fn default() -> Self {
        Self {
            represented_active_trade_partner_like_cpp: None,
            represented_trade_accepted_like_cpp: false,
            represented_partner_trade_server_state_index_like_cpp: 0,
            represented_trade_client_state_index_like_cpp: 1,
            represented_trade_server_state_index_like_cpp: 1,
            represented_trade_items_like_cpp: [None; TRADE_SLOT_COUNT_LIKE_CPP as usize],
            represented_trade_money_like_cpp: 0,
            represented_trade_spell_like_cpp: 0,
            represented_trade_spell_cast_item_like_cpp: None,
            represented_trade_cancel_statuses_like_cpp: Vec::new(),
        }
    }
}
