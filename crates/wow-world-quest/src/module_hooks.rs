// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Hook ports of the quest and progression operations (#1444 H1).
//!
//! This crate never sees the public module API. Each hook point raises a plain
//! domain event through a port whose default does nothing; the World adapter
//! (`wow-world` `session/module_hooks.rs`) converts the event, dispatches it to
//! the trusted linked modules and applies their validated effects through the
//! session's packet publication capability.
//!
//! Two shapes, one contract:
//! - an operation that runs on the session host calls a provided method of
//!   [`crate::QuestSessionHostLikeCpp`] (`quest_accepted_hook_like_cpp`);
//! - an operation that runs inside a borrowed context ([`crate::QuestRewardCx`]
//!   and the objective drain built on it) has no host, so the World hands the
//!   context a [`QuestModuleHookPortLikeCpp`] when it builds it.
//!
//! Every port is called after the transition and its persistence boundary,
//! with no split borrow, map guard, money guard or await in scope.

use wow_core::ObjectGuid;
use wow_world_core::session::PacketPublicationAccessLikeCpp;

/// `player.level_changed`: AzerothCore `Player::GiveLevel` ->
/// `OnPlayerLevelChanged` (`Player.cpp:2585`, `PlayerScript.h:263`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlayerLevelChangedHookEventLikeCpp {
    pub guid: ObjectGuid,
    pub old_level: u8,
    pub new_level: u8,
}

/// `player.money_changed`: AzerothCore `Player::ModifyMoney` ->
/// `OnPlayerMoneyChanged` (`Player.cpp:11688`, `PlayerScript.h:282`), observed
/// here after the change committed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlayerMoneyChangedHookEventLikeCpp {
    pub guid: ObjectGuid,
    pub old_money: u64,
    pub new_money: u64,
}

/// `player.quest_accepted`: AzerothCore `Player::AddQuestAndCheckCompletion`
/// -> `OnPlayerQuestAccept` (`PlayerQuest.cpp:426`, `PlayerScript.h:750`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QuestAcceptedHookEventLikeCpp {
    pub guid: ObjectGuid,
    pub quest_id: u32,
    pub slot: u8,
}

/// `player.quest_rewarded`: the last step of AzerothCore
/// `Player::RewardQuest`, `OnPlayerCompleteQuest` (`PlayerQuest.cpp:903`,
/// `PlayerScript.h:245`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QuestRewardedHookEventLikeCpp {
    pub guid: ObjectGuid,
    pub quest_id: u32,
    pub quest_giver: ObjectGuid,
    pub choice_item_id: u32,
    /// The XP the reward offered; what `GiveXP` actually grants can be less
    /// (none at the level cap).
    pub reward_xp: u32,
    /// The committed balance before and after the reward's money.
    pub money_before: u64,
    pub money_after: u64,
}

/// The hook port a quest/progression context raises its events through.
pub trait QuestModuleHookPortLikeCpp: Sync {
    /// Whether any linked module wants the `player.*` family. A context asks
    /// before it reads anything for an event, so the zero-module path does no
    /// work (some reads take the canonical map lock).
    fn player_hooks_wanted_like_cpp(&self) -> bool {
        false
    }

    fn player_level_changed_like_cpp(
        &self,
        _publication: PacketPublicationAccessLikeCpp<'_>,
        _event: &PlayerLevelChangedHookEventLikeCpp,
    ) {
    }

    fn player_money_changed_like_cpp(
        &self,
        _publication: PacketPublicationAccessLikeCpp<'_>,
        _event: &PlayerMoneyChangedHookEventLikeCpp,
    ) {
    }

    fn quest_rewarded_like_cpp(
        &self,
        _publication: PacketPublicationAccessLikeCpp<'_>,
        _event: &QuestRewardedHookEventLikeCpp,
    ) {
    }
}
