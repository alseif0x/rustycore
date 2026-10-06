use super::LootState;
use wow_world_core::session::{HubMut, HubRef};

impl LootState {
    pub fn resolved_pass_on_group_loot_like_cpp(&self, hub: HubRef<'_>) -> Option<bool> {
        let canonical = hub
            .core
            .with_owned_player_like_cpp(|player| player.pass_on_group_loot_like_cpp());
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && hub.core.player_handle_like_cpp.is_none() {
            return Some(self.pass_on_group_loot);
        }
        canonical
    }

    pub fn set_pass_on_group_loot_like_cpp(&mut self, hub: &mut HubMut<'_>, value: bool) -> bool {
        let canonical = hub
            .core
            .with_owned_player_mut_like_cpp(|player| player.set_pass_on_group_loot_like_cpp(value))
            .is_some();
        #[cfg(any(test, feature = "test-fixtures"))]
        if hub.core.player_handle_like_cpp.is_none() {
            self.pass_on_group_loot = value;
            return true;
        }
        canonical
    }

    #[cfg(test)]
    pub(crate) fn pass_on_group_loot_like_cpp(&self, hub: HubRef<'_>) -> bool {
        self.resolved_pass_on_group_loot_like_cpp(hub)
            .expect("test Player loot preference owner must resolve")
    }

    pub fn loot_specialization_id_like_cpp(&self, hub: HubRef<'_>) -> Option<u32> {
        let canonical = hub
            .core
            .with_owned_player_like_cpp(wow_entities::Player::loot_specialization_id_like_cpp);
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && hub.core.player_handle_like_cpp.is_none() {
            return Some(self.loot_specialization_id);
        }
        canonical
    }

    pub fn set_loot_specialization_id_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        spec_id: u32,
    ) -> bool {
        let canonical = hub
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player.set_loot_specialization_id_like_cpp(spec_id)
            })
            .is_some();
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical || hub.core.player_handle_like_cpp.is_none() {
            self.loot_specialization_id = spec_id;
            return true;
        }
        canonical
    }
}
