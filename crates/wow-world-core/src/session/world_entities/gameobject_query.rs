use std::sync::Arc;

use wow_data::{GameObjectDisplayInfoStore, GameObjectTemplateLifecycleStoreLikeCpp};

impl crate::session::state::SessionCatalogs {
    pub fn gameobject_display_info_store(&self) -> Option<&Arc<GameObjectDisplayInfoStore>> {
        self.gameobjects.display_info_store.as_ref()
    }

    pub fn gameobject_template_lifecycle_store(
        &self,
    ) -> Option<&Arc<GameObjectTemplateLifecycleStoreLikeCpp>> {
        self.gameobject_template_lifecycle_store_like_cpp.as_ref()
    }
}
