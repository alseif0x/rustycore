// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    #[cfg(test)]
    pub fn set_area_trigger_db2_store(&mut self, store: Arc<AreaTriggerDb2Store>) {
        self.catalogs.set_area_trigger_db2_store(store)
    }
    #[cfg(test)]
    pub fn set_area_trigger_script_store(&mut self, store: Arc<AreaTriggerScriptStoreLikeCpp>) {
        self.catalogs.set_area_trigger_script_store(store)
    }
    #[cfg(test)]
    pub fn set_area_trigger_script_dispatcher_like_cpp(
        &mut self,
        dispatcher: AreaTriggerScriptDispatcherLikeCpp,
    ) {
        self.view
            .set_area_trigger_script_dispatcher_like_cpp(dispatcher)
    }
    #[cfg(test)]
    pub fn set_tavern_area_trigger_store(&mut self, store: Arc<TavernAreaTriggerStoreLikeCpp>) {
        self.catalogs.set_tavern_area_trigger_store(store)
    }
    #[cfg(test)]
    pub(crate) fn represented_area_zone_criteria_like_cpp(
        &self,
    ) -> &[RepresentedAreaZoneCriteriaLikeCpp] {
        self.instances.represented_area_zone_criteria_like_cpp()
    }
}

impl WorldSession {
    pub(crate) fn represented_explored_zones_db_string_like_cpp(&self) -> Option<String> {
        Some(explored_zones_db_string_from_blocks_like_cpp(
            &self.player_explored_zones_snapshot_like_cpp()?,
        ))
    }
    /// Represented C++ `Player::UpdateArea` criteria branch.
    ///
    /// C++ records `EnterArea`/`LeaveArea` after updating area-dependent state when
    /// `oldArea != newArea`; this represented slice records those criteria and
    /// the C++ area rest flag side effects. PvP flags, phasing, aura checks, quest push, mount
    /// capability refresh, and chat-channel updates remain runtime gaps.
    pub(crate) fn update_area_represented_like_cpp(&mut self, new_area: u32) -> bool {
        self.update_area_represented_with_rest_update_like_cpp(new_area, true)
    }
    pub(crate) fn set_area_spirit_healer_guid_like_cpp(&mut self, healer_guid: ObjectGuid) -> bool {
        let (state, mut hub) = crate::session::split_instances_mut(self);
        state.set_area_spirit_healer_guid_like_cpp(&mut hub, healer_guid)
    }
    pub(crate) fn area_spirit_healer_guid_like_cpp(&self) -> Option<ObjectGuid> {
        let canonical = self.core.with_owned_player_like_cpp(|player| {
            player.resurrection_state_like_cpp().area_spirit_healer_guid
        });
        #[cfg(test)]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            return Some(self.fixtures.combat.area_spirit_healer_guid_like_cpp);
        }
        canonical
    }
}
