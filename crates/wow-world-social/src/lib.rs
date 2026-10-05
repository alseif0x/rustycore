// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Social ownership for the world-session application boundary.

mod contracts;
mod catalogs;
mod handlers;
mod chat;
mod duel_publication;
mod duel;
mod group;
pub mod group_fanout;
mod state;
mod requests;
mod trade;
mod guild;
mod group_owner;
#[cfg(any(test, feature = "test-fixtures"))]
mod test_support;

pub use handlers::{
    InspectHandlerCxLikeCpp, SocialInspectHandlerHostLikeCpp,
    register_social_inspect_handlers_like_cpp,
};
pub use contracts::{
    ChatFloodThrottleIndexLikeCpp, PlayerAwayModeLikeCpp,
    RepresentedCalendarAddEventLikeCpp, RepresentedCalendarCommunityInviteLikeCpp,
    RepresentedCalendarRemoveEventLikeCpp, RepresentedDeclinePetitionLikeCpp,
    RepresentedQueryPetitionLikeCpp, RepresentedSignPetitionLikeCpp,
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
