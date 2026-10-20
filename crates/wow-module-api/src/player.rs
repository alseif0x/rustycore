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

/// `player.creature_kill`: the player killed a creature.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct CreatureKillSnapshot {
    pub guid: ObjectGuid,
    pub creature: ObjectGuid,
    /// The creature template id.
    pub creature_entry: u32,
    pub creature_level: u8,
    pub map_id: u32,
}

impl CreatureKillSnapshot {
    #[must_use]
    pub fn new(
        guid: ObjectGuid,
        creature: ObjectGuid,
        creature_entry: u32,
        creature_level: u8,
        map_id: u32,
    ) -> Self {
        Self {
            guid,
            creature,
            creature_entry,
            creature_level,
            map_id,
        }
    }
}

/// `item.equipped`: an item was put in an equipment or bag slot.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct ItemEquippedSnapshot {
    pub guid: ObjectGuid,
    pub item_guid: ObjectGuid,
    /// The item template id.
    pub item_entry: u32,
    /// C++ `INVENTORY_SLOT_BAG_0` (255) for every equipment position.
    pub bag: u8,
    /// The equipment slot (`EQUIPMENT_SLOT_*`) or bag slot it took.
    pub slot: u8,
}

impl ItemEquippedSnapshot {
    #[must_use]
    pub fn new(
        guid: ObjectGuid,
        item_guid: ObjectGuid,
        item_entry: u32,
        bag: u8,
        slot: u8,
    ) -> Self {
        Self {
            guid,
            item_guid,
            item_entry,
            bag,
            slot,
        }
    }
}

/// `player.spell_cast`: the player cast a spell; `SMSG_SPELL_GO` was sent.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct SpellCastSnapshot {
    pub guid: ObjectGuid,
    pub spell_id: u32,
    pub cast_id: ObjectGuid,
    /// The explicit target, or `ObjectGuid::EMPTY`.
    pub target: ObjectGuid,
    /// A server cast carrying C++ `TRIGGERED_IGNORE_GCD`, which every
    /// `CastSpell(..., true)` (`TRIGGERED_FULL_MASK`) cast does. RustyCore
    /// represents no other trigger flag, so this is not TrinityCore's
    /// `skipCheck` argument.
    pub triggered: bool,
}

impl SpellCastSnapshot {
    #[must_use]
    pub fn new(
        guid: ObjectGuid,
        spell_id: u32,
        cast_id: ObjectGuid,
        target: ObjectGuid,
        triggered: bool,
    ) -> Self {
        Self {
            guid,
            spell_id,
            cast_id,
            target,
            triggered,
        }
    }
}

/// `player.loot_item`: the player looted an item into its inventory.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct LootItemSnapshot {
    pub guid: ObjectGuid,
    /// The item template id.
    pub item_id: u32,
    pub count: u32,
    /// C++ `GetLootGUID()`: the creature, game object or item looted.
    pub loot_source: ObjectGuid,
}

impl LootItemSnapshot {
    #[must_use]
    pub fn new(guid: ObjectGuid, item_id: u32, count: u32, loot_source: ObjectGuid) -> Self {
        Self {
            guid,
            item_id,
            count,
            loot_source,
        }
    }
}

/// `player.loot_money`: the player looted the money of a loot.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct LootMoneySnapshot {
    pub guid: ObjectGuid,
    /// The creature, game object or item looted.
    pub loot_source: ObjectGuid,
    /// C++ `loot->gold`: the whole amount the loot held, in copper, before
    /// any group share.
    pub money: u64,
    /// What this player's committed balance received, in copper: `money`
    /// when looting alone, its share in a group split.
    pub share: u64,
}

impl LootMoneySnapshot {
    #[must_use]
    pub fn new(guid: ObjectGuid, loot_source: ObjectGuid, money: u64, share: u64) -> Self {
        Self {
            guid,
            loot_source,
            money,
            share,
        }
    }
}

/// One hook of the `player.*` family, for the per-hook interest query.
///
/// AzerothCore registers each `PlayerScript` with the hooks it implements
/// (its `enabledHooks` constructor argument) and `CALL_ENABLED_HOOKS` only
/// visits the scripts that enabled a hook; [`PlayerModule::enabled_hooks`]
/// is that declaration, so an owner skips the snapshot of a hook no module
/// enabled even when another `player.*` hook is in use.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub enum PlayerHook {
    Logout,
    LevelChanged,
    MoneyChanged,
    QuestAccepted,
    QuestRewarded,
    CreatureKill,
    SpellCast,
    ItemEquipped,
    LootItem,
    LootMoney,
}

impl PlayerHook {
    /// Every `player.*` hook: the default declaration.
    pub const ALL: &'static [PlayerHook] = &[
        PlayerHook::Logout,
        PlayerHook::LevelChanged,
        PlayerHook::MoneyChanged,
        PlayerHook::QuestAccepted,
        PlayerHook::QuestRewarded,
        PlayerHook::CreatureKill,
        PlayerHook::SpellCast,
        PlayerHook::ItemEquipped,
        PlayerHook::LootItem,
        PlayerHook::LootMoney,
    ];

    pub(crate) const fn bit(self) -> u32 {
        1 << self as u32
    }

    pub(crate) fn mask(hooks: &[PlayerHook]) -> u32 {
        hooks.iter().fold(0, |mask, hook| mask | hook.bit())
    }
}

/// The `player.*` family. Every method has an empty default, so a module
/// overrides only the hooks it wants.
pub trait PlayerModule: Send + Sync {
    /// The hooks this module implements (AzerothCore's `enabledHooks`). Read
    /// once, when the module is registered; only these hooks are dispatched to
    /// it, and only they make an owner build a snapshot. The default enables
    /// every hook, so a module that does not declare keeps working, at the
    /// cost of every `player.*` snapshot being built.
    fn enabled_hooks(&self) -> &'static [PlayerHook] {
        PlayerHook::ALL
    }

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

    /// AzerothCore `PlayerScript::OnPlayerCreatureKill` (`PlayerScript.h:254`,
    /// `Unit.cpp:14298`); TrinityCore `OnCreatureKill` (`Unit.cpp:10755`), at
    /// the end of `Unit::Kill`: once per kill, after `setDeathState(JUST_DIED)`.
    fn on_creature_kill(&self, _snapshot: &CreatureKillSnapshot, _effects: &mut ScopedEffects<'_>) {
    }

    /// AzerothCore `PlayerScript::OnPlayerSpellCast` (`PlayerScript.h:326`,
    /// `Spell.cpp:3825`); TrinityCore `OnPlayerSpellCast` (`Spell.cpp:3671`).
    /// Observation only: no effect can change spells or auras (#754).
    /// Raised after `SMSG_SPELL_GO`, so unlike AzerothCore (which calls it
    /// before `CheckCast`) a cast that fails raises nothing. A non-persistent
    /// observation: nothing about the cast is durable yet.
    fn on_spell_cast(&self, _snapshot: &SpellCastSnapshot, _effects: &mut ScopedEffects<'_>) {}

    /// AzerothCore `PlayerScript::OnPlayerEquip` (`PlayerScript.h:416`), from
    /// `Player::EquipItem` (`PlayerStorage.cpp:2940`): once per item that
    /// reaches an equipment or bag slot, after the move committed.
    fn on_equip(&self, _snapshot: &ItemEquippedSnapshot, _effects: &mut ScopedEffects<'_>) {}

    /// AzerothCore `PlayerScript::OnPlayerLootItem` (`PlayerScript.h:431`),
    /// the last step of `Player::StoreLootItem` (`Player.cpp:13918`): a
    /// directly looted item, once it is stored. Roll winners and master-loot
    /// recipients are not reported, as in AzerothCore.
    fn on_loot_item(&self, _snapshot: &LootItemSnapshot, _effects: &mut ScopedEffects<'_>) {}

    /// AzerothCore `LootScript::OnLootMoney` (`LootScript.h:44`), from
    /// `WorldSession::HandleLootMoneyOpcode` (`LootHandler.cpp:273`): once, for
    /// the looter, after every share was paid.
    fn on_loot_money(&self, _snapshot: &LootMoneySnapshot, _effects: &mut ScopedEffects<'_>) {}
}
