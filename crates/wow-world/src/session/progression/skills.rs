//! Represented skills and their ranks at the Session boundary.
//!
//! Moved out of the Session root under #611. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    /// Set the skill store for this session.
    pub fn set_skill_store(&mut self, store: Arc<SkillStore>) {
        self.catalogs.skill_store = Some(store);
    }
    pub fn skill_store(&self) -> Option<&Arc<SkillStore>> {
        self.catalogs.skill_store()
    }
    pub fn set_skill_line_store(&mut self, store: Arc<SkillLineStore>) {
        self.catalogs.skill_line_store = Some(store);
    }
    pub fn set_skill_tiers_store(&mut self, store: Arc<SkillTiersStoreLikeCpp>) {
        self.catalogs.skill_tiers_store = Some(store);
    }
    pub fn set_fishing_base_skill_store(&mut self, store: Arc<FishingBaseSkillStoreLikeCpp>) {
        self.catalogs.fishing_base_skill_store = Some(store);
    }
    pub fn set_max_primary_trade_skills_like_cpp(&mut self, configured: u8) {
        self.config.max_primary_trade_skills_like_cpp =
            if configured <= crate::profession::MAX_PRIMARY_TRADE_SKILLS_CONFIG_LIKE_CPP {
                configured
            } else {
                crate::profession::DEFAULT_MAX_PRIMARY_TRADE_SKILLS_LIKE_CPP
            };
    }
    pub(crate) fn max_primary_trade_skills_like_cpp(&self) -> u8 {
        self.config.max_primary_trade_skills_like_cpp()
    }
    pub(crate) fn replace_player_skill_records_like_cpp(
        &mut self,
        skill_records: HashMap<u16, RepresentedPlayerSkillLikeCpp>,
        loaded: bool,
        complete: bool,
    ) -> bool {
        crate::session::hub_mut(self).replace_player_skill_records_like_cpp(
            skill_records,
            loaded,
            complete,
        )
    }
    pub(in crate::session) fn clear_player_skill_tombstones_like_cpp(&mut self) {
        #[cfg(test)]
        if self.core.player_handle_like_cpp.is_none() {
            self.fixture_clear_player_skill_tombstones_like_cpp();
            return;
        }
        let _ = self.core.with_owned_player_mut_like_cpp(
            Player::clear_skill_tombstones_for_identity_change_like_cpp,
        );
    }
    #[cfg(test)]
    pub(in crate::session) fn fixture_clear_player_skill_tombstones_like_cpp(&mut self) {
        let Some(records) = crate::session::hub_ref(self).resolved_player_skill_records_like_cpp()
        else {
            return;
        };
        let Some(loaded) = ({
            let (s, h) = crate::session::split_lifecycle_ref(self);
            s.resolved_player_skill_records_loaded_like_cpp(h)
        }) else {
            return;
        };
        let complete = crate::session::hub_ref(self)
            .complete_player_skill_records_like_cpp()
            .is_some();
        let occupied =
            crate::session::hub_ref(self).complete_player_skill_occupied_slots_like_cpp();
        let _ = self.replace_player_skill_runtime_exact_like_cpp(
            records,
            loaded,
            complete,
            occupied,
            BTreeSet::new(),
        );
    }
    #[cfg(test)]
    pub(crate) fn player_skill_values_like_cpp(&self) -> HashMap<u16, u16> {
        crate::session::hub_ref(self)
            .resolved_player_skill_values_like_cpp()
            .expect("test Player skill owner must resolve")
    }
    pub(in crate::session) fn represented_fishing_base_skill_level_like_cpp(
        &self,
        gameobject_guid: ObjectGuid,
    ) -> Option<i32> {
        let area_id = self
            .world_entities
            .represented_gameobject_use_states
            .get(&gameobject_guid)
            .and_then(|state| state.area_id)?;
        let area_store = self.catalogs.area_table_store()?;
        let fishing_store = self.catalogs.fishing_base_skill_store()?;
        Some(fishing_store.base_skill_level_like_cpp(area_store, area_id))
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/progression/skills_f3_shims.rs"]
mod f3_shims;
