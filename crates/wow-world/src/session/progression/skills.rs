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
    pub(crate) fn skill_store(&self) -> Option<&Arc<SkillStore>> {
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
}

#[cfg(test)]
#[path = "../../../unit_tests/session/progression/skills_f3_shims.rs"]
mod f3_shims;
