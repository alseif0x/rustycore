// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The `player.*` hook family (source API 2, #1444 H1).
//!
//! Confirmed-result notifications, modelled on AzerothCore `PlayerScript`
//! (`src/server/game/Scripting/ScriptDefines/PlayerScript.h`). Each hook runs
//! after its transition and that transition's persistence boundary, so a
//! module observes what is already durable and cannot veto it. Every snapshot
//! is an owned copy; none can mutate the player, money, items or spells.

use wow_core::ObjectGuid;

use crate::effect::ScopedEffects;

/// Why the player left the world. C++ `WorldSession::LogoutPlayer` serves all
/// three, and so does this hook.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum LogoutReason {
    /// Back to the character list.
    CharacterSelection,
    /// The client's logout timer completed.
    TimedLogout,
    /// The connection closed.
    Disconnect,
}

/// `player.logout`: the character left the world after its final save.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct PlayerLogoutSnapshot {
    pub guid: ObjectGuid,
    pub account_id: u32,
    pub reason: LogoutReason,
}

impl PlayerLogoutSnapshot {
    #[must_use]
    pub fn new(guid: ObjectGuid, account_id: u32, reason: LogoutReason) -> Self {
        Self {
            guid,
            account_id,
            reason,
        }
    }
}

/// `player.level_changed`: the player's level changed through XP.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct PlayerLevelChangedSnapshot {
    pub guid: ObjectGuid,
    pub old_level: u8,
    pub new_level: u8,
}

impl PlayerLevelChangedSnapshot {
    #[must_use]
    pub fn new(guid: ObjectGuid, old_level: u8, new_level: u8) -> Self {
        Self {
            guid,
            old_level,
            new_level,
        }
    }
}

/// `player.money_changed`: the player's committed balance changed.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct PlayerMoneyChangedSnapshot {
    pub guid: ObjectGuid,
    /// Copper before the change.
    pub old_money: u64,
    /// Copper after the change.
    pub new_money: u64,
}

impl PlayerMoneyChangedSnapshot {
    #[must_use]
    pub fn new(guid: ObjectGuid, old_money: u64, new_money: u64) -> Self {
        Self {
            guid,
            old_money,
            new_money,
        }
    }
}

/// `player.quest_accepted`: a quest entered the player's quest log.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct QuestAcceptedSnapshot {
    pub guid: ObjectGuid,
    pub quest_id: u32,
    /// The quest-log slot the quest took.
    pub slot: u8,
}

impl QuestAcceptedSnapshot {
    #[must_use]
    pub fn new(guid: ObjectGuid, quest_id: u32, slot: u8) -> Self {
        Self {
            guid,
            quest_id,
            slot,
        }
    }
}

/// `player.quest_rewarded`: a quest was turned in and its reward committed.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct QuestRewardedSnapshot {
    pub guid: ObjectGuid,
    pub quest_id: u32,
    /// Who the quest was turned in to; the player itself for an auto reward.
    pub quest_giver: ObjectGuid,
    /// The chosen reward item, or 0.
    pub choice_item_id: u32,
    /// The XP the reward offered (`Quest::XPValue`). `GiveXP` may grant less,
    /// nothing at the level cap; `player.level_changed` reports what changed.
    pub reward_xp: u32,
    /// The committed balance, in copper, before the reward's money.
    pub money_before: u64,
    /// The committed balance, in copper, after it; equal to `money_before`
    /// when the reward grants no money.
    pub money_after: u64,
}

impl QuestRewardedSnapshot {
    #[must_use]
    pub fn new(
        guid: ObjectGuid,
        quest_id: u32,
        quest_giver: ObjectGuid,
        choice_item_id: u32,
        reward_xp: u32,
        money_before: u64,
        money_after: u64,
    ) -> Self {
        Self {
            guid,
            quest_id,
            quest_giver,
            choice_item_id,
            reward_xp,
            money_before,
            money_after,
        }
    }
}

/// The `player.*` family. Every method has an empty default, so a module
/// overrides only the hooks it wants.
pub trait PlayerModule: Send + Sync {
    /// AzerothCore `PlayerScript::OnPlayerLogout` (`PlayerScript.h:338`), from
    /// `WorldSession::LogoutPlayer` after the final save. The session is
    /// closing, so this hook observes and has no effects.
    fn on_logout(&self, _snapshot: &PlayerLogoutSnapshot) {}

    /// AzerothCore `PlayerScript::OnPlayerLevelChanged` (`PlayerScript.h:263`),
    /// from `Player::GiveLevel`: once per level gained, in order, only after
    /// the XP save applied.
    fn on_level_changed(
        &self,
        _snapshot: &PlayerLevelChangedSnapshot,
        _effects: &mut ScopedEffects<'_>,
    ) {
    }

    /// AzerothCore `PlayerScript::OnPlayerMoneyChanged` (`PlayerScript.h:282`).
    /// Observation only: AzerothCore's `int32&` may change the amount, which
    /// needs a pre-decision on every money path (#583). Reported once the
    /// change committed.
    fn on_money_changed(
        &self,
        _snapshot: &PlayerMoneyChangedSnapshot,
        _effects: &mut ScopedEffects<'_>,
    ) {
    }

    /// AzerothCore `PlayerScript::OnPlayerQuestAccept` (`PlayerScript.h:750`),
    /// from `Player::AddQuestAndCheckCompletion`.
    fn on_quest_accepted(
        &self,
        _snapshot: &QuestAcceptedSnapshot,
        _effects: &mut ScopedEffects<'_>,
    ) {
    }

    /// AzerothCore `PlayerScript::OnPlayerCompleteQuest` (`PlayerScript.h:245`),
    /// the last step of `Player::RewardQuest`. The reward's level and money
    /// changes are reported first, in AzerothCore's order.
    fn on_quest_rewarded(
        &self,
        _snapshot: &QuestRewardedSnapshot,
        _effects: &mut ScopedEffects<'_>,
    ) {
    }
}
