// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The durable half of one quest-reward operation, accumulated while the
//! operation runs and committed once at its end.
//!
//! C++ `Player::RewardQuest` (Player.cpp:14625) mutates the player in memory
//! throughout and only reaches the database in its closing `SaveToDB(false)`
//! (Player.cpp:14867). This type is that deferral made explicit: every grant
//! and removal records what it will need durable, and nothing is written until
//! the operation has finished deciding.

use wow_persistence::{
    PlayerCurrencySaveRequestLikeCpp, PlayerInventoryPersistenceRequestLikeCpp,
    PlayerQuestLockoutPersistenceRequestLikeCpp, PlayerQuestRewardDurableRequestLikeCpp,
    PlayerQuestRewardMoneyLikeCpp, PlayerQuestStatusPersistenceRequestLikeCpp,
};

/// Durable participants of one in-flight quest-reward operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct QuestRewardDurablePlanLikeCpp {
    owner_guid: u64,
    quest_id: u32,
    inventory_mutations: Vec<PlayerInventoryPersistenceRequestLikeCpp>,
    currencies: Option<PlayerCurrencySaveRequestLikeCpp>,
    money: Option<PlayerQuestRewardMoneyLikeCpp>,
    quest_status: Option<PlayerQuestStatusPersistenceRequestLikeCpp>,
    lockouts: Vec<PlayerQuestLockoutPersistenceRequestLikeCpp>,
}

impl QuestRewardDurablePlanLikeCpp {
    pub(crate) fn new(owner_guid: u64, quest_id: u32) -> Self {
        Self {
            owner_guid,
            quest_id,
            inventory_mutations: Vec::new(),
            currencies: None,
            money: None,
            quest_status: None,
            lockouts: Vec::new(),
        }
    }

    pub(crate) fn owner_guid(&self) -> u64 {
        self.owner_guid
    }

    /// Record one inventory removal or grant, keeping the operation's order.
    pub(crate) fn push_inventory_mutation(
        &mut self,
        mutation: PlayerInventoryPersistenceRequestLikeCpp,
    ) {
        self.inventory_mutations.push(mutation);
    }

    /// Record the currency state after a change.
    ///
    /// `_SaveCurrency` writes the player's complete currency state, so a later
    /// change supersedes an earlier one rather than adding to it.
    pub(crate) fn set_currencies(&mut self, currencies: PlayerCurrencySaveRequestLikeCpp) {
        self.currencies = Some(currencies);
    }

    /// Record the money the operation produced.
    ///
    /// `money_before` is kept from the first recorded change so an ambiguous
    /// COMMIT is reconciled against the value the operation actually started
    /// from, not an intermediate one.
    pub(crate) fn set_money(&mut self, money_before: u64, money_after: u64) {
        let money_before = self
            .money
            .map_or(money_before, |existing| existing.money_before);
        self.money = Some(PlayerQuestRewardMoneyLikeCpp {
            money_before,
            money_after,
        });
    }

    pub(crate) fn money(&self) -> Option<PlayerQuestRewardMoneyLikeCpp> {
        self.money
    }

    pub(crate) fn set_quest_status(&mut self, status: PlayerQuestStatusPersistenceRequestLikeCpp) {
        self.quest_status = Some(status);
    }

    pub(crate) fn push_lockout(&mut self, lockout: PlayerQuestLockoutPersistenceRequestLikeCpp) {
        self.lockouts.push(lockout);
    }

    /// Close the plan into the request the port commits.
    ///
    /// Returns `None` when the operation never decided the quest's own durable
    /// status: without it there is no reward to make durable, and committing
    /// the grants alone would be exactly the partial write this contract
    /// exists to prevent.
    pub(crate) fn into_request(self) -> Option<PlayerQuestRewardDurableRequestLikeCpp> {
        Some(PlayerQuestRewardDurableRequestLikeCpp {
            owner_guid: self.owner_guid,
            quest_id: self.quest_id,
            inventory_mutations: self.inventory_mutations,
            currencies: self.currencies,
            money: self.money,
            quest_status: self.quest_status?,
            lockouts: self.lockouts,
        })
    }
}
