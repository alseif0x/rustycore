//! Load-authority sequencing: beginning a hydration, completing it and the
//! guard that proves the represented state is authoritative.
//!
//! Moved out of the Session root under #609. Behaviour is preserved; the
//! #585 identity-hydration ordering is unchanged.

use super::*;

impl WorldSession {
    pub async fn load_completed_achievements_like_cpp(&mut self) {
        let (state, mut hub) = crate::session::split_lifecycle_mut(self);
        state.load_completed_achievements_like_cpp(&mut hub).await
    }
    pub(crate) fn begin_represented_trait_config_authority_load_like_cpp(&mut self) {
        let _ = self.begin_represented_trait_config_load_like_cpp();
        self.core
            .invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
    }
    #[cfg(test)]
    pub(in crate::session) fn fixture_begin_trait_config_authority_load_like_cpp(&mut self) {
        let _ = self.begin_represented_trait_authority_load_like_cpp();
        self.core
            .invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
    }
    pub(crate) fn complete_represented_trait_config_authority_load_like_cpp(
        &mut self,
        configs: impl IntoIterator<Item = (i32, i32, i32, i32)>,
        entries_empty: bool,
    ) -> bool {
        self.core
            .invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        let configs = configs.into_iter().collect();
        let result = self.complete_represented_trait_config_rows_like_cpp(configs, entries_empty);
        if result == Some(false) {
            // Invalid input has reset the source proof. Keep the previous
            // post-reset invalidation outside the exclusive Player access.
            self.core
                .invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        }
        result.unwrap_or(false)
    }
    #[cfg(test)]
    pub(in crate::session) fn fixture_complete_trait_config_authority_load_like_cpp(
        &mut self,
        configs: impl IntoIterator<Item = (i32, i32, i32, i32)>,
        entries_empty: bool,
    ) -> bool {
        self.core
            .invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        let mut exact_configs = BTreeMap::new();
        for (config_id, config_type, specialization_id, combat_flags) in configs {
            if config_id <= 0
                || exact_configs
                    .insert(
                        config_id,
                        (config_type, specialization_id, combat_flags).into(),
                    )
                    .is_some()
            {
                self.fixture_begin_trait_config_authority_load_like_cpp();
                return false;
            }
        }

        self.install_represented_trait_authority_rows_like_cpp(exact_configs, entries_empty)
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/persistence/load_authority/f3_shims.rs"]
mod f3_shims;
