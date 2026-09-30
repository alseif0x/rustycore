//! Borrowed reputation catalog reads required by the domain rules.
//!
//! Application code adapts its existing stores to this contract. Reputation
//! rules never own or copy those catalogs.

use wow_data_model::currency::CurrencyTypesEntry;
use wow_data_model::reputation::{
    FactionEntry, FriendshipRepReactionEntry, ParagonReputationEntry,
};

pub trait ReputationCatalogReadLikeCpp {
    fn faction_like_cpp(&self, id: u32) -> Option<&FactionEntry>;

    fn factions_like_cpp(&self) -> impl Iterator<Item = &FactionEntry> + '_;

    fn faction_team_list_like_cpp(&self, id: u32) -> Vec<u32>;

    fn friendship_reactions_like_cpp(
        &self,
        id: u8,
    ) -> Option<Vec<&FriendshipRepReactionEntry>>;

    fn paragon_for_faction_like_cpp(&self, id: u32) -> Option<&ParagonReputationEntry>;

    fn currency_types_like_cpp(&self, id: u32) -> Option<&CurrencyTypesEntry>;
}
