//! Represented visibility and phase state at the Session boundary.
//!
//! Moved out of the Session root under #632. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub fn set_phase_store(&mut self, store: Arc<PhaseStore>) {
        self.catalogs.phase_store = Some(store);
    }
    pub(crate) fn sync_represented_farsight_clear_from_canonical_like_cpp(&mut self) -> bool {
        let (state, mut hub) = crate::session::split_visibility_mut(self);
        state.sync_represented_farsight_clear_from_canonical_like_cpp(&mut hub)
    }
    #[cfg(test)]
    pub(crate) async fn force_update_visibility_like_cpp(&mut self) {
        let catalogs = self.creature_spawn_catalogs_for_test_like_cpp();
        wow_world_entities::force_update_visibility_with_catalogs_like_cpp(self, &catalogs).await;
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/visibility/operations/f3_shims.rs"]
mod f3_shims;
