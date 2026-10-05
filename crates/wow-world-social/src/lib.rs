// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Social ownership for the world-session application boundary.

mod arena_team_handlers;
mod calendar_handlers;
mod catalogs;
mod chat;
mod chat_handlers;
mod contracts;
mod duel;
mod duel_publication;
mod group;
pub mod group_fanout;
mod group_handlers;
mod group_owner;
mod guild;
mod handlers;
mod requests;
mod social_contacts_handlers;
mod state;
#[cfg(any(test, feature = "test-fixtures"))]
mod test_support;
mod trade;

pub use arena_team_handlers::{
    ArenaTeamHandlerCxLikeCpp, ArenaTeamHandlerHostLikeCpp, register_arena_team_handlers_like_cpp,
};
pub use calendar_handlers::{
    CalendarHandlerCxLikeCpp, CalendarHandlerHostLikeCpp, register_calendar_handlers_like_cpp,
};
pub use chat_handlers::{
    ChatHandlerCxLikeCpp, ChatHandlerHostLikeCpp, register_chat_handlers_like_cpp,
};
pub use chat_handlers::{
    GM_SILENCE_AURA_LIKE_CPP, LANG_ADDON_LIKE_CPP, LANG_ADDON_LOGGED_LIKE_CPP,
    LANG_UNIVERSAL_LIKE_CPP, secs_to_full_time_string_like_cpp,
};
pub use chat_handlers::{
    JoinChannelPrecheckLikeCpp, chat_msg_from_i32_like_cpp, is_known_language_like_cpp,
    join_channel_custom_precheck_like_cpp,
};
pub use chat_handlers::{
    player_name_and_guid_like_cpp, send_wait_before_speaking_notification_if_muted_like_cpp,
};
pub use contracts::{
    ChatFloodThrottleIndexLikeCpp, PlayerAwayModeLikeCpp, RepresentedCalendarAddEventLikeCpp,
    RepresentedCalendarCommunityInviteLikeCpp, RepresentedCalendarRemoveEventLikeCpp,
    RepresentedDeclinePetitionLikeCpp, RepresentedQueryPetitionLikeCpp,
    RepresentedSignPetitionLikeCpp,
};
pub use group_handlers::{
    SocialGroupHandlerCxLikeCpp, SocialGroupHandlerHostLikeCpp,
    register_social_group_handlers_like_cpp,
};
pub use guild::{player_guild_state_snapshot_like_cpp, resolved_represented_guild_id_like_cpp};
pub use handlers::{
    InspectHandlerCxLikeCpp, SocialInspectHandlerHostLikeCpp,
    register_social_inspect_handlers_like_cpp,
};
pub use social_contacts_handlers::normalize_player_name_like_cpp;
pub use social_contacts_handlers::{
    SocialContactsHandlerCxLikeCpp, SocialContactsHandlerHostLikeCpp,
    register_social_contacts_handlers_like_cpp,
};
pub use state::SessionSocialLimits;

pub const GROUP_XP_DISTANCE_LIKE_CPP: f32 = 74.0;
pub const PLAYER_FLAGS_AUTO_DECLINE_GUILD_LIKE_CPP: u32 = 0x0800_0000;
pub const SPELL_DUEL_LIKE_CPP: u32 = 7266;
pub const SPELL_MOUNTED_DUEL_LIKE_CPP: u32 = 62875;
#[cfg(any(test, feature = "test-fixtures"))]
pub const SPELL_DUEL_BEG_LIKE_CPP: u32 = 7267;
#[cfg(any(test, feature = "test-fixtures"))]
pub use contracts::{
    RepresentedCanDuelSpellCastLikeCpp, RepresentedDuelAcceptedLikeCpp,
    RepresentedDuelCancelOutcomeLikeCpp, RepresentedDuelCancelledLikeCpp,
    RepresentedDuelRequestedLikeCpp, RepresentedForceDeselectLikeCpp,
    RepresentedSilencePartyTalkerLikeCpp,
};
