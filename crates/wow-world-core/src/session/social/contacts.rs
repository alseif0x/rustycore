use std::sync::Arc;

use wow_data::progression_rewards::FriendshipRepReactionStore;

impl crate::session::state::SessionCatalogs {
    pub fn friendship_rep_reaction_store(&self) -> Option<&Arc<FriendshipRepReactionStore>> {
        self.friendship_rep_reaction_store.as_ref()
    }
}
