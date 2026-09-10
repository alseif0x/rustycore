//! Quest, reward, criteria, faction, curve and scaling DB2 readers.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use tracing::info;

use crate::wdc4::Wdc4Reader;

mod criteria;
mod curves;
mod faction;
mod loading;
mod quest;
mod reward_pack;
mod scaling;

pub use criteria::*;
pub use curves::*;
pub use faction::*;
pub use quest::*;
pub use reward_pack::*;
pub use scaling::*;

use loading::{FromEntries, f32_array, f32_field, load_store};

macro_rules! db2_store {
    ($store:ident, $entry:ty) => {
        pub struct $store {
            entries: HashMap<u32, $entry>,
        }
        impl $store {
            pub fn from_entries(entries: impl IntoIterator<Item = $entry>) -> Self {
                Self {
                    entries: entries.into_iter().map(|entry| (entry.id, entry)).collect(),
                }
            }
            pub fn get(&self, id: u32) -> Option<&$entry> {
                self.entries.get(&id)
            }
            pub fn len(&self) -> usize {
                self.entries.len()
            }
            pub fn is_empty(&self) -> bool {
                self.entries.is_empty()
            }
        }
    };
}

db2_store!(AchievementCategoryStore, AchievementCategoryEntry);

db2_store!(ContentTuningStore, ContentTuningEntry);

db2_store!(CriteriaTreeStore, CriteriaTreeEntry);

db2_store!(CurveStore, CurveEntry);

db2_store!(CurvePointStore, CurvePointEntry);

db2_store!(FactionStore, FactionEntry);

db2_store!(FactionTemplateStore, FactionTemplateEntry);

db2_store!(FriendshipRepReactionStore, FriendshipRepReactionEntry);

db2_store!(FriendshipReputationStore, FriendshipReputationEntry);

db2_store!(ModifierTreeStore, ModifierTreeEntry);

db2_store!(NumTalentsAtLevelStore, NumTalentsAtLevelEntry);

db2_store!(ParagonReputationStore, ParagonReputationEntry);

db2_store!(QuestFactionRewardStore, QuestFactionRewardEntry);

db2_store!(QuestInfoStore, QuestInfoEntry);

db2_store!(QuestLineXQuestStore, QuestLineXQuestEntry);

db2_store!(QuestMoneyRewardStore, QuestMoneyRewardEntry);

db2_store!(QuestPackageItemStore, QuestPackageItemEntry);

db2_store!(QuestSortStore, QuestSortEntry);

db2_store!(QuestV2Store, QuestV2Entry);

db2_store!(RewardPackStore, RewardPackEntry);

db2_store!(RewardPackXCurrencyTypeStore, RewardPackXCurrencyTypeEntry);

db2_store!(RewardPackXItemStore, RewardPackXItemEntry);

db2_store!(ScalingStatDistributionStore, ScalingStatDistributionEntry);

db2_store!(ScalingStatValuesStore, ScalingStatValuesEntry);

macro_rules! impl_from_entries {
    ($store:ident, $entry:ty) => {
        impl FromEntries<$entry> for $store {
            fn from_entries(entries: impl IntoIterator<Item = $entry>) -> Self {
                Self::from_entries(entries)
            }
            fn len(&self) -> usize {
                self.len()
            }
        }
    };
}

impl_from_entries!(AchievementCategoryStore, AchievementCategoryEntry);

impl_from_entries!(ContentTuningStore, ContentTuningEntry);

impl_from_entries!(CriteriaTreeStore, CriteriaTreeEntry);

impl_from_entries!(CurveStore, CurveEntry);

impl_from_entries!(CurvePointStore, CurvePointEntry);

impl_from_entries!(FactionStore, FactionEntry);

impl_from_entries!(FactionTemplateStore, FactionTemplateEntry);

impl_from_entries!(FriendshipRepReactionStore, FriendshipRepReactionEntry);

impl_from_entries!(FriendshipReputationStore, FriendshipReputationEntry);

impl_from_entries!(ModifierTreeStore, ModifierTreeEntry);

impl_from_entries!(NumTalentsAtLevelStore, NumTalentsAtLevelEntry);

impl_from_entries!(ParagonReputationStore, ParagonReputationEntry);

impl_from_entries!(QuestFactionRewardStore, QuestFactionRewardEntry);

impl_from_entries!(QuestInfoStore, QuestInfoEntry);

impl_from_entries!(QuestLineXQuestStore, QuestLineXQuestEntry);

impl_from_entries!(QuestMoneyRewardStore, QuestMoneyRewardEntry);

impl_from_entries!(QuestPackageItemStore, QuestPackageItemEntry);

impl_from_entries!(QuestSortStore, QuestSortEntry);

impl_from_entries!(QuestV2Store, QuestV2Entry);

impl_from_entries!(RewardPackStore, RewardPackEntry);

impl_from_entries!(RewardPackXCurrencyTypeStore, RewardPackXCurrencyTypeEntry);

impl_from_entries!(RewardPackXItemStore, RewardPackXItemEntry);

impl_from_entries!(ScalingStatDistributionStore, ScalingStatDistributionEntry);

impl_from_entries!(ScalingStatValuesStore, ScalingStatValuesEntry);

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
