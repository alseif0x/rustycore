use std::collections::BTreeMap;
use std::collections::HashMap;
use std::sync::Arc;

use wow_constants::{CurrencyTypes, Team};
use wow_entities::{PlayerCurrency, PlayerCurrencyState};
use wow_loot::LootClaimCommitError;
use wow_world_core::session::{
    HubMut, HubRef, MAX_SPECIALIZATIONS_LIKE_CPP, PlayerCurrencyDelta,
    currency_max_quantity_cpp,
};
use wow_world_core::session::state::hub_support::player_team_for_race_cpp;

use super::SessionLifecycleState;

#[derive(Debug)]
pub enum LootMoneyPersistenceErrorLikeCpp {
    MissingPlayer,
    MissingCharacterDatabase,
    WorkerTerminated,
    Claim(LootClaimCommitError),
    Persistence(String),
    CommitOutcomeUnknownPersistence(String),
}

impl std::fmt::Display for LootMoneyPersistenceErrorLikeCpp {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingPlayer => formatter.write_str("loot-money player is missing"),
            Self::MissingCharacterDatabase => {
                formatter.write_str("loot-money character database is missing")
            }
            Self::WorkerTerminated => formatter.write_str("loot-money persistence worker stopped"),
            Self::Claim(error) => write!(formatter, "loot-money claim failure: {error:?}"),
            Self::Persistence(reason) => {
                write!(formatter, "loot-money persistence failure: {reason}")
            }
            Self::CommitOutcomeUnknownPersistence(reason) => {
                write!(formatter, "loot-money COMMIT outcome is unknown: {reason}")
            }
        }
    }
}

impl std::error::Error for LootMoneyPersistenceErrorLikeCpp {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::MissingPlayer
            | Self::MissingCharacterDatabase
            | Self::WorkerTerminated
            | Self::Claim(_)
            | Self::Persistence(_)
            | Self::CommitOutcomeUnknownPersistence(_) => None,
        }
    }
}

/// Pure post-reset snapshot used to make the durable talent reset and its
/// runtime publication describe the same state.
///
/// C++ `Player::ResetTalents` calls `RemoveTalent` for the active group, then
/// `_SaveTalents` rewrites every group. `RemoveTalent` calls
/// `RemoveSpell(..., disabled=true)`, and the complete C++ `_SaveSpells` path
/// rewrites those rows with their exact active/disabled/favorite state. Rust
/// does not retain the full `PlayerSpellMap` active/disabled/temporary state,
/// so this deliberately leaves `character_spell` and
/// `character_spell_favorite` untouched. A known non-dependent spell proves a
/// normal persisted row exists; it does not prove that the talent is its only
/// source. Deleting that row would lose normal ownership where C++ instead
/// preserves a disabled row. Recursive `RemoveSpell` persistence and exact
/// disabled/favorite preservation therefore remain a represented boundary,
/// while active `character_talent` rows can no longer survive a committed fee.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepresentedTalentResetStatePlanLikeCpp {
    pub active_group: u8,
    pub active_talents: BTreeMap<u32, u8>,
    pub post_talents: [BTreeMap<u32, u8>; MAX_SPECIALIZATIONS_LIKE_CPP],
}

impl SessionLifecycleState {
    /// Quarantine this session after an unreconcilable battle-pet purchase
    /// COMMIT, mirroring the #159 money-persistence indeterminate boundary:
    /// normal payout admission stays closed and the client must relog.
    pub fn quarantine_player_money_persistence_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        reason: &'static str,
    ) {
        self.durable_loot_money_persistence_like_cpp
            .mark_indeterminate_like_cpp();
        hub.core.kick(reason);
    }

    /// C++ `Player::AddCurrency(..., CurrencyGainSource::Vendor)` without aura gain bonuses.
    pub fn plan_add_currency_vendor_like_cpp(
        &self,
        hub: HubRef<'_>,
        currencies: &mut HashMap<u32, PlayerCurrency>,
        currency_id: u32,
        amount: u32,
    ) -> Result<Option<PlayerCurrencyDelta>, ()> {
        if amount == 0 {
            return Ok(None);
        }

        let Some(entry) = hub
            .catalogs
            .currency_types_store
            .as_ref()
            .and_then(|store| store.get(currency_id))
            .copied()
        else {
            return Err(());
        };

        let player_team = player_team_for_race_cpp(hub.player_race_like_cpp());
        if (entry.is_alliance() && player_team != Team::Alliance)
            || (entry.is_horde() && player_team != Team::Horde)
        {
            return Err(());
        }

        if entry.award_condition_id != 0
            || entry.faction_id != 0
            || currency_id == CurrencyTypes::Azerite as u32
        {
            return Err(());
        }

        let currency = currencies.entry(currency_id).or_insert(PlayerCurrency {
            state: PlayerCurrencyState::New,
            quantity: 0,
            weekly_quantity: 0,
            tracked_quantity: 0,
            increased_cap_quantity: 0,
            earned_quantity: 0,
            flags: 0,
        });

        let weekly_cap = entry.max_earnable_per_week;
        let mut applied = amount;
        if weekly_cap != 0 && currency.weekly_quantity.saturating_add(applied) > weekly_cap {
            applied = weekly_cap.saturating_sub(currency.weekly_quantity);
        }

        let max_quantity = currency_max_quantity_cpp(&entry, currency);
        if max_quantity != 0 && currency.quantity.saturating_add(applied) > max_quantity {
            applied = max_quantity.saturating_sub(currency.quantity);
        }

        if applied == 0 {
            return Ok(None);
        }

        if currency.state != PlayerCurrencyState::New {
            currency.state = PlayerCurrencyState::Changed;
        }
        currency.quantity = currency.quantity.saturating_add(applied);
        if weekly_cap != 0 {
            currency.weekly_quantity = currency.weekly_quantity.saturating_add(applied);
        }
        if entry.is_tracking_quantity() {
            currency.tracked_quantity = currency.tracked_quantity.saturating_add(applied);
        }
        if entry.has_total_earned() {
            currency.earned_quantity = currency.earned_quantity.saturating_add(applied);
        }

        let scaler = entry.scaler().max(1) as u32;
        let delta = PlayerCurrencyDelta {
            currency_id,
            quantity: currency.quantity,
            amount: applied,
            weekly_quantity: ((currency.weekly_quantity / scaler) > 0)
                .then_some(currency.weekly_quantity),
            max_quantity: (max_quantity != 0).then_some(max_quantity),
            total_earned: entry.has_total_earned().then_some(currency.earned_quantity),
            suppress_chat_log: entry.is_suppressing_chat_log(false),
        };
        Ok(Some(delta))
    }

    /// Persist an explicit player-money value and surface database failures to
    /// callers that must not expose a loot payout before it is durable.
    ///
    /// Unlike [`Self::save_player_gold`], this helper fails closed when there is
    /// no selected player or lifecycle port. Focused tests must opt into an
    /// explicit persistence result through the test seam below.
    pub async fn persist_player_gold_checked_like_cpp(
        &self,
        hub: HubRef<'_>,
        money: u64,
    ) -> Result<(), LootMoneyPersistenceErrorLikeCpp> {
        #[cfg(any(test, feature = "test-fixtures"))]
        if let Some(success) = self.loot_money_persistence_test_result_like_cpp {
            return success
                .then_some(())
                .ok_or(LootMoneyPersistenceErrorLikeCpp::MissingCharacterDatabase);
        }

        let guid = hub
            .core
            .player_guid()
            .ok_or(LootMoneyPersistenceErrorLikeCpp::MissingPlayer)?;
        let port = self
            .player_lifecycle_port_like_cpp()
            .map(Arc::clone)
            .ok_or(LootMoneyPersistenceErrorLikeCpp::MissingCharacterDatabase)?;
        match port
            .persist_money_write_like_cpp(wow_persistence::PlayerMoneyWriteRequestLikeCpp {
                player_guid: guid.counter() as u64,
                money,
            })
            .await
        {
            wow_persistence::PersistenceOutcomeLikeCpp::Applied { .. } => Ok(()),
            wow_persistence::PersistenceOutcomeLikeCpp::Failed { reason }
            | wow_persistence::PersistenceOutcomeLikeCpp::Unknown { reason } => {
                Err(LootMoneyPersistenceErrorLikeCpp::Persistence(reason))
            }
        }
    }

    fn represented_talent_reset_state_plan_like_cpp(
        &self,
        hub: HubRef<'_>,
    ) -> Option<RepresentedTalentResetStatePlanLikeCpp> {
        let runtime = hub.player_talent_runtime_snapshot_like_cpp()?;
        if !runtime.talents_loaded_like_cpp() {
            return None;
        }

        let active_group = runtime.active_group_like_cpp();
        let active_group_index = usize::from(active_group);
        let active_talents = runtime.talent_group_like_cpp(active_group)?.clone();
        let mut post_talents = runtime.talent_groups_snapshot_like_cpp();
        post_talents[active_group_index].clear();

        Some(RepresentedTalentResetStatePlanLikeCpp {
            active_group,
            active_talents,
            post_talents,
        })
    }

    /// Build the represented durable talent-reset request without mutating the
    /// session. Statement identity, transaction construction and ambiguous
    /// COMMIT reconciliation belong to the lifecycle adapter.
    pub fn represented_talent_reset_persistence_plan_like_cpp(
        &self,
        hub: HubRef<'_>,
        guid_counter: u64,
        old_money: u64,
        new_money: u64,
        cost: u32,
        reset_time_secs: u64,
    ) -> Option<(
        RepresentedTalentResetStatePlanLikeCpp,
        wow_persistence::PlayerTalentResetPersistenceRequestLikeCpp,
    )> {
        let state_plan = self.represented_talent_reset_state_plan_like_cpp(hub)?;
        let mut retained_talents = Vec::new();

        for (talent_group, talents) in state_plan.post_talents.iter().enumerate() {
            for (talent_id, rank) in talents {
                if hub
                    .represented_talent_info_like_cpp(*talent_id, *rank)
                    .is_none()
                {
                    continue;
                }
                retained_talents.push(wow_persistence::PlayerTalentResetSaveRowLikeCpp {
                    talent_id: *talent_id,
                    rank: *rank,
                    talent_group: talent_group as u8,
                });
            }
        }

        debug_assert_eq!(old_money.saturating_sub(new_money), u64::from(cost));
        Some((
            state_plan,
            wow_persistence::PlayerTalentResetPersistenceRequestLikeCpp {
                player_guid: guid_counter,
                money_before: old_money,
                money_after: new_money,
                reset_cost: cost,
                reset_time_secs,
                retained_talents,
            },
        ))
    }
}
