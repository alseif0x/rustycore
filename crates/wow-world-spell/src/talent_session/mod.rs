// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Represented talents at the session boundary (#1363 slice 9b).
//!
//! C++ `Player::LearnTalent`, the `Player::ResetTalents` runtime reset (with
//! the `AT_LOGIN_RESET_TALENTS` login reset) and
//! `WorldSession::HandleLearnTalentOpcode` (`SkillHandler.cpp:29`). Their
//! Rust bodies now live in this crate as free functions over
//! [`TalentHostLikeCpp`], next to the spellbook learn/remove closure they
//! call; each host method is the same-named `WorldSession` operation, called
//! at the exact point the body called it before the move.

use crate::SpellbookHostLikeCpp;

mod handlers;
mod talents;

pub use handlers::*;
pub use talents::*;

/// The World session as the talents' host. The supertrait lends the
/// spellbook host and, through it, the hub.
pub trait TalentHostLikeCpp: SpellbookHostLikeCpp {
    /// The World talent-point refresh. It stays on the session because it
    /// builds `wow_world_quest::QuestRewardCx`, and quest depends on spell.
    fn refresh_represented_talent_points_like_cpp(&mut self);
    fn resolved_represented_at_login_flags_like_cpp(&self) -> Option<u16>;
    fn remove_represented_at_login_flag_like_cpp(&mut self, flags: u16, persist: bool) -> bool;
    fn represented_talents_loaded_like_cpp(&self) -> bool;
    fn trinity_string_like_cpp(&self, entry: u32) -> &str;
}
