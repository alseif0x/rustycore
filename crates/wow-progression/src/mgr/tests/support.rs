//! Pure in-memory catalog fixtures for reputation rule tests.

use std::collections::HashMap;

use wow_data_model::currency::CurrencyTypesEntry;
use wow_data_model::reputation::{
    FactionEntry, FriendshipRepReactionEntry, ParagonReputationEntry,
};

use crate::mgr::ReputationCatalogReadLikeCpp;

#[derive(Default)]
pub(super) struct TestReputationCatalogLikeCpp {
    factions: HashMap<u32, FactionEntry>,
    friendship_reactions: Option<HashMap<u8, Vec<FriendshipRepReactionEntry>>>,
    paragons_by_faction: HashMap<u32, ParagonReputationEntry>,
    currencies: HashMap<u32, CurrencyTypesEntry>,
}

impl TestReputationCatalogLikeCpp {
    pub(super) fn from_factions(entries: impl IntoIterator<Item = FactionEntry>) -> Self {
        Self {
            factions: entries.into_iter().map(|entry| (entry.id, entry)).collect(),
            ..Self::default()
        }
    }

    pub(super) fn with_friendship_reactions(
        mut self,
        entries: impl IntoIterator<Item = FriendshipRepReactionEntry>,
    ) -> Self {
        let mut reactions_by_friendship = HashMap::<u8, Vec<_>>::new();
        for entry in entries {
            reactions_by_friendship
                .entry(entry.friendship_rep_id)
                .or_default()
                .push(entry);
        }
        for reactions in reactions_by_friendship.values_mut() {
            reactions.sort_by_key(|entry| entry.reaction_threshold);
        }
        self.friendship_reactions = Some(reactions_by_friendship);
        self
    }

    pub(super) fn with_paragon_entries(
        mut self,
        entries: impl IntoIterator<Item = ParagonReputationEntry>,
    ) -> Self {
        self.paragons_by_faction = entries
            .into_iter()
            .map(|entry| (entry.faction_id as u32, entry))
            .collect();
        self
    }

    pub(super) fn with_currency_entries(
        mut self,
        entries: impl IntoIterator<Item = CurrencyTypesEntry>,
    ) -> Self {
        self.currencies = entries.into_iter().map(|entry| (entry.id, entry)).collect();
        self
    }
}

impl ReputationCatalogReadLikeCpp for TestReputationCatalogLikeCpp {
    fn faction_like_cpp(&self, id: u32) -> Option<&FactionEntry> {
        self.factions.get(&id)
    }

    fn factions_like_cpp(&self) -> impl Iterator<Item = &FactionEntry> + '_ {
        self.factions.values()
    }

    fn faction_team_list_like_cpp(&self, id: u32) -> Vec<u32> {
        let mut faction_ids = self
            .factions
            .values()
            .filter(|entry| u32::from(entry.parent_faction_id) == id)
            .map(|entry| entry.id)
            .collect::<Vec<_>>();
        faction_ids.sort_unstable();
        faction_ids
    }

    fn friendship_reactions_like_cpp(
        &self,
        id: u8,
    ) -> Option<Vec<&FriendshipRepReactionEntry>> {
        self.friendship_reactions.as_ref().map(|by_friendship| {
            by_friendship
                .get(&id)
                .into_iter()
                .flatten()
                .collect()
        })
    }

    fn paragon_for_faction_like_cpp(&self, id: u32) -> Option<&ParagonReputationEntry> {
        self.paragons_by_faction.get(&id)
    }

    fn currency_types_like_cpp(&self, id: u32) -> Option<&CurrencyTypesEntry> {
        self.currencies.get(&id)
    }
}
