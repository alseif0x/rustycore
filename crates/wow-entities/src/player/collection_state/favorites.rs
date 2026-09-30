//! Favorite appearance transitions and the full-update projection.

use super::*;

impl PlayerCollectionStateLikeCpp {
    /// C++ CollectionMgr::SetAppearanceIsFavorite (:828), including repeated removals.
    pub fn set_appearance_favorite(
        &mut self,
        item_modified_appearance_id: u32,
        apply: bool,
    ) -> bool {
        use PlayerFavoriteAppearanceStateLikeCpp::{New, Removed, Unchanged};
        use std::collections::hash_map::Entry;

        if apply {
            match self
                .favorite_item_appearance_entry_like_cpp(item_modified_appearance_id)
            {
                Entry::Vacant(entry) => {
                    entry.insert(New);
                    true
                }
                Entry::Occupied(mut entry) if *entry.get() == Removed => {
                    entry.insert(Unchanged);
                    true
                }
                Entry::Occupied(_) => false,
            }
        } else {
            match self
                .favorite_item_appearance_entry_like_cpp(item_modified_appearance_id)
            {
                Entry::Occupied(entry) if *entry.get() == New => {
                    entry.remove();
                    true
                }
                Entry::Occupied(mut entry) => {
                    entry.insert(Removed);
                    true
                }
                Entry::Vacant(_) => false,
            }
        }
    }

    pub fn favorite_appearance_ids(&self) -> Vec<u32> {
        self.favorite_item_appearances_like_cpp()
            .into_iter()
            .filter_map(|(appearance, state)| {
                (*state != PlayerFavoriteAppearanceStateLikeCpp::Removed).then_some(*appearance)
            })
            .collect::<Vec<_>>()
    }
}
