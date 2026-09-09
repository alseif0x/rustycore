//! Load-authority sequencing: beginning a hydration, completing it and the
//! guard that proves the represented state is authoritative.
//!
//! Moved out of the Session root under #609. Behaviour is preserved; the
//! #585 identity-hydration ordering is unchanged.

use super::*;

impl WorldSession {
    #[cfg(test)]
    pub(in crate::session) fn load_completed_achievement_rows_like_cpp(
        &mut self,
        rows: impl IntoIterator<Item = u32>,
    ) {
        let _ = self.replace_completed_achievement_ids_like_cpp(rows);
    }
    pub async fn load_completed_achievements_like_cpp(&mut self) {
        let _ = self.replace_completed_achievement_ids_like_cpp([]);

        let Some(player_guid) = self.player_guid() else {
            warn!(
                account = self.account_id,
                "LoadCompletedAchievements skipped: player guid unavailable"
            );
            return;
        };
        let Some(port) = self.player_lifecycle_port_like_cpp().map(Arc::clone) else {
            warn!(
                account = self.account_id,
                guid = player_guid.counter(),
                "LoadCompletedAchievements skipped: Player lifecycle port unavailable"
            );
            return;
        };

        let rows = match port
            .load_login_auxiliary_like_cpp(
                wow_persistence::PlayerLoginAuxiliaryLoadRequestLikeCpp::CompletedAchievements {
                    player_guid: player_guid.counter() as u64,
                },
            )
            .await
        {
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
                wow_persistence::PlayerLoginAuxiliaryLoadedLikeCpp::CompletedAchievements(rows),
            ) => rows,
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed { reason } => {
                warn!(
                    account = self.account_id,
                    guid = player_guid.counter(),
                    "LoadCompletedAchievements query failed: {reason}"
                );
                return;
            }
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(_) => {
                warn!(
                    account = self.account_id,
                    guid = player_guid.counter(),
                    "Player lifecycle port returned the wrong auxiliary login data for completed achievements"
                );
                return;
            }
        };

        let _ = self.replace_completed_achievement_ids_like_cpp(rows);
    }
    pub(crate) fn begin_represented_trait_config_authority_load_like_cpp(&mut self) {
        let _ = self.mutate_player_spell_runtime_like_cpp(
            wow_entities::PlayerSpellRuntimeState::begin_trait_config_load_like_cpp,
        );
        self.invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
    }
    #[cfg(test)]
    pub(in crate::session) fn fixture_begin_trait_config_authority_load_like_cpp(&mut self) {
        let _ = self.mutate_player_spell_runtime_like_cpp(|runtime| {
            runtime.trait_definition_ids.clear();
            runtime.trait_definition_ids_complete = false;
            runtime.trait_config_rows.clear();
            runtime.trait_config_rows_complete = false;
            runtime.trait_entry_rows_complete = false;
            runtime.trait_entry_rows_empty = false;
        });
        self.invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
    }
    pub(crate) fn complete_represented_trait_config_authority_load_like_cpp(
        &mut self,
        configs: impl IntoIterator<Item = (i32, i32, i32, i32)>,
        entries_empty: bool,
    ) -> bool {
        self.invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        let configs = configs.into_iter().collect();
        let result = self.mutate_player_spell_runtime_like_cpp(|runtime| {
            runtime.complete_trait_config_load_like_cpp(configs, entries_empty)
        });
        if result == Some(false) {
            // Invalid input has reset the source proof. Keep the previous
            // post-reset invalidation outside the exclusive Player access.
            self.invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        }
        result.unwrap_or(false)
    }
    #[cfg(test)]
    pub(in crate::session) fn fixture_complete_trait_config_authority_load_like_cpp(
        &mut self,
        configs: impl IntoIterator<Item = (i32, i32, i32, i32)>,
        entries_empty: bool,
    ) -> bool {
        self.invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
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

        self.mutate_player_spell_runtime_like_cpp(|runtime| {
            runtime.trait_config_rows = exact_configs;
            runtime.trait_config_rows_complete = true;
            runtime.trait_entry_rows_complete = true;
            runtime.trait_entry_rows_empty = entries_empty;
        })
        .is_some()
    }
}
