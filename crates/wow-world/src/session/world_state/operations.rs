//! Represented area, zone, trigger and weather state.
//!
//! Moved out of the Session root under #632. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(crate) fn dispatch_area_trigger_script_like_cpp(
        &mut self,
        dispatcher: Option<&AreaTriggerScriptDispatcherLikeCpp>,
        script_id: ScriptIdLikeCpp,
        trigger_id: u32,
        entered: bool,
    ) -> Option<bool> {
        let dispatcher = Arc::clone(dispatcher?);
        Some(dispatcher(self, script_id, trigger_id, entered))
    }
    pub fn set_area_table_store(&mut self, store: Arc<AreaTableStore>) {
        self.core
            .invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        self.catalogs.area_table_store = Some(store);
    }
    #[cfg(test)]
    pub(crate) fn handle_represented_tavern_area_trigger_like_cpp(
        &mut self,
        trigger_id: u32,
        entered: bool,
    ) -> bool {
        let taverns = self
            .catalogs
            .tavern_area_trigger_store
            .clone()
            .unwrap_or_else(|| Arc::new(TavernAreaTriggerStoreLikeCpp::default()));
        wow_world_instances::handle_represented_tavern_area_trigger_with_catalog_like_cpp(
            self,
            taverns.as_ref(),
            trigger_id,
            entered,
        )
    }
    #[cfg(test)]
    pub(crate) async fn check_area_explore_and_outdoor_represented_like_cpp(
        &mut self,
        area_id: u32,
    ) -> bool {
        let progression = self.progression_catalogs_for_test_like_cpp();
        wow_world_instances::check_area_explore_and_outdoor_represented_with_catalogs_like_cpp(
            self,
            &progression,
            area_id,
        )
        .await
    }
    #[cfg(test)]
    pub(crate) fn apply_represented_first_login_explored_zones_like_cpp(&mut self) -> usize {
        let player_bootstrap = self.player_bootstrap_catalogs_for_test_like_cpp();
        {
            let (s, mut h) = crate::session::split_instances_mut(self);
            s.apply_represented_first_login_explored_zones_with_catalogs_like_cpp(
                &mut h,
                &player_bootstrap,
            )
        }
    }
    pub(crate) fn set_player_zone_area_authority_complete_like_cpp(&mut self, complete: bool) {
        let (state, mut hub) = crate::session::split_instances_mut(self);
        state.set_player_zone_area_authority_complete_like_cpp(&mut hub, complete)
    }
}

impl crate::session::state::SessionWorldView {
    #[cfg(test)]
    pub(crate) fn set_area_trigger_script_dispatcher_like_cpp(
        &mut self,
        dispatcher: AreaTriggerScriptDispatcherLikeCpp,
    ) {
        self.area_trigger_script_dispatcher_like_cpp = Some(dispatcher);
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/world_state/operations/f3_shims.rs"]
mod f3_shims;
