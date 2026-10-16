// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Social ownership for the world-session application boundary.

mod arena_team_handlers;
mod battlemaster_skirmish;
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
mod guild_bank_handlers;
mod guild_handlers;
mod handlers;
mod lfg_lock_info;
mod reputation;
mod reputation_gain;
mod requests;
mod social_contacts_handlers;
mod social_host;
mod spell_effects;
mod state;
#[cfg(any(test, feature = "test-fixtures"))]
mod test_support;
mod text_emote;
mod trade;
mod trade_handlers;

pub use arena_team_handlers::{
    ArenaTeamHandlerCxLikeCpp, ArenaTeamHandlerHostLikeCpp, register_arena_team_handlers_like_cpp,
};
pub use battlemaster_skirmish::{
    handle_accept_wargame_invite, handle_battlemaster_join_skirmish_with_catalog_like_cpp,
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
#[cfg(any(test, feature = "test-fixtures"))]
pub use guild::mutate_player_guild_state_for_test_like_cpp;
pub use guild::{
    player_guild_state_snapshot_like_cpp, resolved_represented_guild_id_like_cpp,
    set_represented_guild_id_like_cpp,
};
pub use guild_bank_handlers::{
    GuildBankHandlerCxLikeCpp, GuildBankHandlerHostLikeCpp, register_guild_bank_handlers_like_cpp,
};
pub use guild_handlers::{
    GuildHandlerCxLikeCpp, GuildHandlerHostLikeCpp, register_guild_handlers_like_cpp,
};
pub use handlers::{
    InspectHandlerCxLikeCpp, SocialInspectHandlerHostLikeCpp,
    register_social_inspect_handlers_like_cpp,
};
pub use lfg_lock_info::{
    LFG_LOCKSTATUS_MISSING_ACHIEVEMENT_LIKE_CPP, LFG_LOCKSTATUS_MISSING_ITEM_LIKE_CPP,
    LFG_LOCKSTATUS_QUEST_NOT_COMPLETED_LIKE_CPP, can_reward_lfg_quest_like_cpp,
    handle_df_get_system_info_with_catalog_like_cpp, lfg_is_disabled_map_type_for_player_like_cpp,
    lfg_lock_status_like_cpp, lfg_player_lock_info_like_cpp,
    populate_lfg_player_dungeon_reward_like_cpp,
};
pub use reputation::{
    ReputationHandlerCxLikeCpp, ReputationHandlerHostLikeCpp, register_reputation_handlers_like_cpp,
};
pub use reputation_gain::{
    apply_recruit_a_friend_reputation_bonus_like_cpp, calculate_kill_reputation_gain_like_cpp,
    calculate_reputation_gain_like_cpp, reputation_gain_percent_before_reward_rate_like_cpp,
};
pub use social_contacts_handlers::normalize_player_name_like_cpp;
pub use social_contacts_handlers::{
    SocialContactsHandlerCxLikeCpp, SocialContactsHandlerHostLikeCpp,
    register_social_contacts_handlers_like_cpp,
};
pub use social_host::SocialSessionHostLikeCpp;
pub use spell_effects::{
    apply_duel_effect_like_cpp, apply_force_deselect_effect_like_cpp,
    apply_reputation_effect_like_cpp,
};
pub use state::SessionSocialLimits;
pub use text_emote::{
    ANIM_MOUNT_SPECIAL_LIKE_CPP, EMOTE_STATE_DANCE_LIKE_CPP, EMOTE_STATE_SIT_LIKE_CPP,
    handle_text_emote_with_catalogs_like_cpp,
};
pub use trade_handlers::{
    TradeHandlerCxLikeCpp, TradeHandlerHostLikeCpp, register_trade_handlers_like_cpp,
};
pub use trade_handlers::{
    player_trade_state_snapshot_like_cpp, resolved_represented_active_trade_partner_like_cpp,
    resolved_represented_duel_arbiter_guid_like_cpp, set_represented_duel_arbiter_guid_like_cpp,
    set_represented_duel_state_like_cpp,
};

pub const GROUP_XP_DISTANCE_LIKE_CPP: f32 = 74.0;
pub const PLAYER_FLAGS_AUTO_DECLINE_GUILD_LIKE_CPP: u32 = 0x0800_0000;
pub const SPELL_DUEL_LIKE_CPP: u32 = 7266;

/// C++ `DUEL_COUNTDOWN` before `SMSG_DUEL_COUNTDOWN`.
pub use duel::DUEL_COUNTDOWN_MS_LIKE_CPP;
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
