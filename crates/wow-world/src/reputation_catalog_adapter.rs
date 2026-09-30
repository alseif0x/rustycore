//! Borrowed application adapter for the existing reputation stores.

use wow_data::CurrencyTypesStore;
use wow_data::progression_rewards::{
    FactionStore, FriendshipRepReactionStore, ParagonReputationStore,
};
use wow_data_model::currency::CurrencyTypesEntry;
use wow_data_model::reputation::{
    FactionEntry, FriendshipRepReactionEntry, ParagonReputationEntry,
};
use wow_progression::mgr::ReputationCatalogReadLikeCpp;

#[derive(Clone, Copy)]
pub(crate) struct ReputationCatalogViewLikeCpp<'a> {
    faction_store: Option<&'a FactionStore>,
    friendship_rep_reaction_store: Option<&'a FriendshipRepReactionStore>,
    paragon_reputation_store: Option<&'a ParagonReputationStore>,
    currency_types_store: Option<&'a CurrencyTypesStore>,
}

impl<'a> ReputationCatalogViewLikeCpp<'a> {
    pub(crate) const fn new(
        faction_store: Option<&'a FactionStore>,
        friendship_rep_reaction_store: Option<&'a FriendshipRepReactionStore>,
        paragon_reputation_store: Option<&'a ParagonReputationStore>,
        currency_types_store: Option<&'a CurrencyTypesStore>,
    ) -> Self {
        Self {
            faction_store,
            friendship_rep_reaction_store,
            paragon_reputation_store,
            currency_types_store,
        }
    }
}

impl ReputationCatalogReadLikeCpp for ReputationCatalogViewLikeCpp<'_> {
    fn faction_like_cpp(&self, id: u32) -> Option<&FactionEntry> {
        self.faction_store.and_then(|store| store.get(id))
    }

    fn factions_like_cpp(&self) -> impl Iterator<Item = &FactionEntry> + '_ {
        self.faction_store.into_iter().flat_map(|store| store.iter())
    }

    fn faction_team_list_like_cpp(&self, id: u32) -> Vec<u32> {
        self.faction_store
            .map_or_else(Vec::new, |store| store.faction_team_list_like_cpp(id))
    }

    fn friendship_reactions_like_cpp(
        &self,
        id: u8,
    ) -> Option<Vec<&FriendshipRepReactionEntry>> {
        self.friendship_rep_reaction_store
            .map(|store| store.reactions_for_friendship_rep_like_cpp(id))
    }

    fn paragon_for_faction_like_cpp(&self, id: u32) -> Option<&ParagonReputationEntry> {
        self.paragon_reputation_store
            .and_then(|store| store.get_by_faction_id_like_cpp(id))
    }

    fn currency_types_like_cpp(&self, id: u32) -> Option<&CurrencyTypesEntry> {
        self.currency_types_store
            .and_then(|store| store.get(id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wow_constants::{CurrencyTypesFlags, CurrencyTypesFlagsB};
    use wow_data::progression_rewards::{FactionStore, FriendshipRepReactionStore};

    #[test]
    fn friendship_catalog_adapter_preserves_absent_and_empty_store_distinction() {
        let absent = ReputationCatalogViewLikeCpp::new(None, None, None, None);
        assert_eq!(absent.friendship_reactions_like_cpp(4), None);

        let empty_store = FriendshipRepReactionStore::from_entries([]);
        let present_empty =
            ReputationCatalogViewLikeCpp::new(None, Some(&empty_store), None, None);
        assert_eq!(present_empty.friendship_reactions_like_cpp(4), Some(Vec::new()));
    }

    #[test]
    fn reputation_catalog_adapter_delegates_store_reads_and_ordering() {
        let parent = FactionEntry::for_test_like_cpp(50, 1);
        let mut child_high = FactionEntry::for_test_like_cpp(9, 2);
        child_high.parent_faction_id = 50;
        let mut child_low = FactionEntry::for_test_like_cpp(3, 3);
        child_low.parent_faction_id = 50;
        let factions = FactionStore::from_entries([parent, child_high, child_low]);
        let friendships = FriendshipRepReactionStore::from_entries([
            FriendshipRepReactionEntry {
                id: 2,
                reaction: String::new(),
                friendship_rep_id: 7,
                reaction_threshold: 200,
            },
            FriendshipRepReactionEntry {
                id: 1,
                reaction: String::new(),
                friendship_rep_id: 7,
                reaction_threshold: 100,
            },
        ]);
        let paragons = ParagonReputationStore::from_entries([ParagonReputationEntry {
            id: 11,
            faction_id: 9,
            level_threshold: 1_000,
            quest_id: 12,
        }]);
        let currency = CurrencyTypesStore::from_entries([CurrencyTypesEntry {
            id: 25,
            category_id: 0,
            inventory_icon_file_id: 0,
            spell_weight: 0,
            spell_category: 0,
            max_qty: 5,
            max_earnable_per_week: 0,
            quality: 0,
            faction_id: 0,
            award_condition_id: 0,
            flags: CurrencyTypesFlags::empty(),
            flags_b: CurrencyTypesFlagsB::empty(),
        }]);
        let view = ReputationCatalogViewLikeCpp::new(
            Some(&factions),
            Some(&friendships),
            Some(&paragons),
            Some(&currency),
        );

        assert_eq!(view.faction_like_cpp(9).map(|entry| entry.id), Some(9));
        assert_eq!(
            view.factions_like_cpp().map(|entry| entry.id).collect::<Vec<_>>(),
            factions.iter().map(|entry| entry.id).collect::<Vec<_>>(),
        );
        assert_eq!(view.faction_team_list_like_cpp(50), vec![3, 9]);
        assert_eq!(
            view.friendship_reactions_like_cpp(7)
                .unwrap()
                .into_iter()
                .map(|entry| entry.id)
                .collect::<Vec<_>>(),
            vec![1, 2]
        );
        assert_eq!(view.paragon_for_faction_like_cpp(9).map(|entry| entry.id), Some(11));
        assert_eq!(view.currency_types_like_cpp(25).map(|entry| entry.id), Some(25));
    }
}
