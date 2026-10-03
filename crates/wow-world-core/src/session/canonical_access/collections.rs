// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use wow_entities::PlayerCollectionStateLikeCpp;

#[cfg(any(test, feature = "test-fixtures"))]
use crate::session::state::CollectionsState;
use crate::session::SessionCore;

/// Borrowed access to the session's canonical account-collection owner.
pub struct OwnedCollectionsAccessLikeCpp<'a> {
    core: &'a SessionCore,
    #[cfg(any(test, feature = "test-fixtures"))]
    fixture: Option<&'a CollectionsState>,
}

impl SessionCore {
    /// Build canonical-only access to this session's account collections.
    pub fn owned_collections_access_like_cpp(&self) -> OwnedCollectionsAccessLikeCpp<'_> {
        OwnedCollectionsAccessLikeCpp {
            core: self,
            #[cfg(any(test, feature = "test-fixtures"))]
            fixture: None,
        }
    }
}

impl<'a> OwnedCollectionsAccessLikeCpp<'a> {
    /// Supply represented collections only for a fixture with no canonical owner.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn with_fixture_collections(mut self, fixture: &'a CollectionsState) -> Self {
        self.fixture = Some(fixture);
        self
    }

    /// Clone the canonical account-collection state, or its fixture representation.
    pub fn player_collection_state_snapshot_like_cpp(&self) -> Option<PlayerCollectionStateLikeCpp> {
        let canonical = self
            .core
            .with_owned_player_like_cpp(|player| player.gameplay_state().collections.clone());
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            return self.fixture.map(CollectionsState::represented_player_collection_state_like_cpp);
        }
        canonical
    }
}
