//! Represented cast-state helpers outside the #589 request lifecycle.
//!
//! Moved out of the Session root under #601. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    /// Represented C++ first-login `PlayerInfo::castSpells[GetCreateMode()]`.
    ///
    /// C++ executes these casts after clearing `AT_LOGIN_FIRST` and before
    /// `CONFIG_START_ALL_EXPLORED` / `CONFIG_START_ALL_REP`. This slice uses the
    /// represented spell executor; full triggered-cast semantics still depend on
    /// the remaining Spell runtime port.
    pub(crate) async fn apply_represented_first_login_cast_spells_with_catalogs_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        creature_spawn_catalogs: &CreatureSpawnCatalogsLikeCpp,
        player_bootstrap: &PlayerBootstrapCatalogsLikeCpp,
    ) -> usize {
        let Some(player_guid) = self.player_guid() else {
            return 0;
        };
        let Some(create_mode) = crate::session::hub_ref(self).player_create_mode_like_cpp() else {
            return 0;
        };
        let spells = player_bootstrap
            .cast_spells
            .cast_spells_like_cpp(
                crate::session::hub_ref(self).player_race_like_cpp(),
                crate::session::hub_ref(self).player_class_like_cpp(),
                create_mode,
            )
            .to_vec();

        let mut cast_count = 0usize;
        for spell_id in spells {
            // C++ `CharacterHandler.cpp` casts the create-mode spells with
            // `CastSpell(pCurrChar, spellId, true)`, i.e. TRIGGERED_FULL_MASK:
            // the cast is triggered and ignores the global cooldown.
            if wow_world_spell::execute_server_triggered_spell_like_cpp(
                self,
                item_guid_generator,
                creature_spawn_catalogs,
                spell_id as i32,
                player_guid,
                SpellCastMetadata {
                    cast_flags: CAST_FLAG_PENDING_LIKE_CPP,
                    triggered_ignores_global_cooldown_like_cpp: true,
                    ..SpellCastMetadata::default()
                },
            )
            .await
            .is_ok()
            {
                cast_count += 1;
            }
        }

        cast_count
    }
    #[cfg(test)]
    pub(crate) async fn apply_represented_first_login_cast_spells_like_cpp(&mut self) -> usize {
        let generators = self.id_generators_for_test_like_cpp();
        let creature_spawn_catalogs = self.creature_spawn_catalogs_for_test_like_cpp();
        let player_bootstrap = self.player_bootstrap_catalogs_for_test_like_cpp();
        self.apply_represented_first_login_cast_spells_with_catalogs_like_cpp(
            generators.item.as_ref(),
            &creature_spawn_catalogs,
            &player_bootstrap,
        )
        .await
    }
    pub(in crate::session) fn represented_login_passive_spell_cast_gate_like_cpp(
        &self,
        spell_id: i32,
    ) -> bool {
        let (state, hub) = crate::session::split_spell_state_ref(self);
        state.represented_login_passive_spell_cast_gate_like_cpp(hub, spell_id)
    }
    #[cfg(test)]
    pub(crate) fn represented_can_duel_spell_casts_like_cpp(
        &self,
    ) -> &[RepresentedCanDuelSpellCastLikeCpp] {
        self.social
            .represented_can_duel_spell_casts_for_test_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn represented_talent_respec_visual_spell_casts_like_cpp(
        &self,
    ) -> &[RepresentedTalentRespecVisualSpellCastLikeCpp] {
        &self
            .fixtures
            .progression
            .represented_talent_respec_visual_spell_casts_like_cpp
    }
}
#[cfg(test)]
#[path = "../../../unit_tests/session/spell_state/cast/f3_shims.rs"]
mod f3_shims;
