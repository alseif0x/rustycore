//! Creature lookup, catalogs and nearby search for the represented Session.
//!
//! Moved out of the Session root under #599. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    #[allow(dead_code)]
    pub(crate) fn canonical_creature_access_like_cpp(
        &self,
        guid: ObjectGuid,
    ) -> Option<RepresentedCreatureAccessLikeCpp> {
        let (state, hub) = crate::session::split_world_entities_ref(self);
        state.canonical_creature_access_like_cpp(hub, guid)
    }
    pub fn set_creature_onkill_reputation_store(
        &mut self,
        store: Arc<CreatureOnKillReputationStoreLikeCpp>,
    ) {
        self.catalogs.creatures.onkill_reputation_store = Some(store);
    }
    pub fn set_creature_template_mount_store(
        &mut self,
        store: Arc<CreatureTemplateMountStoreLikeCpp>,
    ) {
        self.catalogs.creatures.template_mount_store = Some(store);
    }
    pub fn set_creature_template_lifecycle_store_like_cpp(
        &mut self,
        store: Arc<CreatureTemplateLifecycleStoreLikeCpp>,
    ) {
        self.catalogs.creatures.template_lifecycle_store_like_cpp = Some(store);
    }
    pub fn set_creature_display_info_store(&mut self, store: Arc<CreatureDisplayInfoStore>) {
        self.catalogs.creatures.display_info_store = Some(store);
    }
    pub fn set_creature_display_info_extra_store(
        &mut self,
        store: Arc<CreatureDisplayInfoExtraStore>,
    ) {
        self.catalogs.creatures.display_info_extra_store = Some(store);
    }
    pub fn set_creature_model_info_store(
        &mut self,
        store: Arc<wow_data::CreatureModelInfoStoreLikeCpp>,
    ) {
        self.catalogs.creatures.model_info_store = Some(store);
    }
    pub fn set_creature_model_data_store(&mut self, store: Arc<CreatureModelDataStore>) {
        self.catalogs.creatures.model_data_store = Some(store);
    }
}
