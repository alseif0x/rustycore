use crate::contracts::{
    RepresentedCalendarAddEventLikeCpp, RepresentedCalendarCommunityInviteLikeCpp,
    RepresentedCalendarRemoveEventLikeCpp, RepresentedCanDuelSpellCastLikeCpp,
    RepresentedDuelAcceptedLikeCpp, RepresentedDuelCancelledLikeCpp,
    RepresentedDuelRequestedLikeCpp, RepresentedForceDeselectLikeCpp,
};
use wow_core::ObjectGuid;

/// Handle-less fixture for Player duel state and its test evidence.
pub(crate) struct DuelTestFixtureLikeCpp {
    pub(crate) represented_can_duel_spell_casts_like_cpp: Vec<RepresentedCanDuelSpellCastLikeCpp>,
    pub(crate) represented_duel_arbiter_guid_like_cpp: Option<ObjectGuid>,
    pub(crate) represented_duel_requests_like_cpp: Vec<RepresentedDuelRequestedLikeCpp>,
    pub(crate) represented_force_deselects_like_cpp: Vec<RepresentedForceDeselectLikeCpp>,
    pub(crate) represented_duel_accepts_like_cpp: Vec<RepresentedDuelAcceptedLikeCpp>,
    pub(crate) represented_duel_cancels_like_cpp: Vec<RepresentedDuelCancelledLikeCpp>,
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
    pub(crate) represented_calendar_community_invites_like_cpp:
        Vec<RepresentedCalendarCommunityInviteLikeCpp>,
    pub(crate) represented_calendar_add_events_like_cpp: Vec<RepresentedCalendarAddEventLikeCpp>,
    pub(crate) represented_calendar_remove_events_like_cpp:
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
    pub(crate) represented_guild_id_like_cpp: u64,
    pub(crate) represented_guild_id_authority_complete_like_cpp: bool,
    pub(crate) represented_guild_id_invited_like_cpp: u64,
    pub(crate) represented_guild_accept_invites_like_cpp: Vec<u64>,
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
    pub(crate) represented_active_trade_partner_like_cpp: Option<ObjectGuid>,
    pub(crate) represented_trade_accepted_like_cpp: bool,
    pub(crate) represented_partner_trade_server_state_index_like_cpp: u32,
    pub(crate) represented_trade_client_state_index_like_cpp: u32,
    pub(crate) represented_trade_server_state_index_like_cpp: u32,
    pub(crate) represented_trade_items_like_cpp:
        [Option<ObjectGuid>; wow_packet::packets::misc::TRADE_SLOT_COUNT_LIKE_CPP as usize],
    pub(crate) represented_trade_money_like_cpp: u64,
    pub(crate) represented_trade_spell_like_cpp: u32,
    pub(crate) represented_trade_spell_cast_item_like_cpp: Option<ObjectGuid>,
    pub(crate) represented_trade_cancel_statuses_like_cpp: Vec<u8>,
}

impl Default for TradeTestFixtureLikeCpp {
    fn default() -> Self {
        Self {
            represented_active_trade_partner_like_cpp: None,
            represented_trade_accepted_like_cpp: false,
            represented_partner_trade_server_state_index_like_cpp: 0,
            represented_trade_client_state_index_like_cpp: 1,
            represented_trade_server_state_index_like_cpp: 1,
            represented_trade_items_like_cpp: [None;
                wow_packet::packets::misc::TRADE_SLOT_COUNT_LIKE_CPP as usize],
            represented_trade_money_like_cpp: 0,
            represented_trade_spell_like_cpp: 0,
            represented_trade_spell_cast_item_like_cpp: None,
            represented_trade_cancel_statuses_like_cpp: Vec::new(),
        }
    }
}
