//! Gameobject lookup, catalogs and nearby search for the represented Session.
//!
//! Moved out of the Session root under #599. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub fn set_gameobject_display_info_store(&mut self, store: Arc<GameObjectDisplayInfoStore>) {
        self.catalogs.gameobjects.display_info_store = Some(store);
    }
    pub fn set_gameobject_template_lifecycle_store(
        &mut self,
        store: Arc<GameObjectTemplateLifecycleStoreLikeCpp>,
    ) {
        self.catalogs.gameobject_template_lifecycle_store_like_cpp = Some(store);
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/world_entities/gameobject_query/f3_shims.rs"]
mod f3_shims;
