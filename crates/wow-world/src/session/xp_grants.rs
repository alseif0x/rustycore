// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World-owned construction seam for the App-owned Player XP operation.

use super::WorldSession;

#[cfg(any(test, feature = "test-fixtures"))]
use wow_world_core::session::{
    CoreXPGainFixtureRefsLikeCpp, StatsAuraFixtureRefs, StatsCombatFixtureRefs, StatsFixtureRefs,
};

impl WorldSession {
    #[cfg(any(test, feature = "test-fixtures"))]
    fn quest_xp_gain_context_like_cpp(
        &mut self,
    ) -> (
        wow_world_application::QuestRewardCx<'_>,
        wow_world_application::QuestXpGainFixtureRefsLikeCpp<'_>,
    ) {
        let wow_world_core::session::state::SessionFixtures {
            identity,
            progression,
            combat,
            movement,
            auras,
            battleground,
            ..
        } = &mut self.fixtures;

        let core_fixtures = CoreXPGainFixtureRefsLikeCpp::new_like_cpp(
            &mut progression.player_xp,
            &mut progression.player_next_level_xp,
            &movement.player_position,
            &battleground.player_battleground_type_id_like_cpp,
            &battleground.player_battleground_map_id_like_cpp,
            &battleground.represented_battleground_status_like_cpp,
            &battleground.represented_battleground_queue_slots_like_cpp,
            &battleground.represented_arena_team_id_invited_like_cpp,
            &mut progression.rest_mgr_test_fixture_like_cpp,
            &auras.player_aura_authority_complete_like_cpp,
            &auras.player_spell_hit_aura_authority_tombstoned_like_cpp,
            &auras.visible_auras,
            &auras.canonical_threat_aura_snapshots_like_cpp,
        );
        let stats_fixtures = StatsFixtureRefs::new_like_cpp(
            StatsCombatFixtureRefs::new_like_cpp(
                &mut combat.player_health_like_cpp,
                &mut combat.player_max_health_like_cpp,
                &mut combat.player_alive_like_cpp,
                &mut combat.represented_player_powers_like_cpp[0],
                &mut combat.represented_player_max_powers_like_cpp[0],
                &mut combat.represented_player_base_mana_like_cpp,
            ),
            StatsAuraFixtureRefs::new_like_cpp(
                &auras.represented_shapeshift_form_like_cpp,
                &auras.player_aura_authority_complete_like_cpp,
                &auras.player_spell_hit_aura_authority_tombstoned_like_cpp,
                &auras.visible_auras,
                &auras.canonical_threat_aura_snapshots_like_cpp,
            ),
        );
        let fixtures = wow_world_application::QuestXpGainFixtureRefsLikeCpp::new_like_cpp(
            core_fixtures,
            &mut progression.player_character_points_like_cpp,
            &progression.represented_gray_level_script_overrides_like_cpp,
            &progression.represented_talents_like_cpp,
            &progression.represented_active_talent_group_like_cpp,
            stats_fixtures,
        );

        let player = self
            .core
            .quest_reward_player_access_like_cpp(&identity.player_race, &identity.player_class);
        let operation = wow_world_application::QuestRewardCx::new(
            &mut self.inventory,
            &mut self.lifecycle,
            &mut self.quest_state,
            player,
            &mut identity.player_level,
            &self.catalogs,
            &self.config,
            &self.social,
            self.catalogs.currency_types_store.as_deref(),
            self.catalogs.quests.xp_store.as_deref(),
            cfg!(test),
        );
        (operation, fixtures)
    }

    #[cfg(not(any(test, feature = "test-fixtures")))]
    fn quest_xp_gain_context_like_cpp(&mut self) -> wow_world_application::QuestRewardCx<'_> {
        let player = self.core.quest_reward_player_access_like_cpp();
        wow_world_application::QuestRewardCx::new(
            &mut self.inventory,
            &mut self.lifecycle,
            &mut self.quest_state,
            player,
            &self.catalogs,
            &self.config,
            &self.social,
            self.catalogs.currency_types_store.as_deref(),
            self.catalogs.quests.xp_store.as_deref(),
            cfg!(test),
        )
    }

    /// Apply the visible XP grant phases. The handler's async wrapper persists
    /// the resulting projection after this operation returns.
    pub(crate) fn give_xp_runtime_like_cpp(
        &mut self,
        xp: u32,
        victim: wow_core::ObjectGuid,
        group_rate: f32,
    ) -> bool {
        #[cfg(any(test, feature = "test-fixtures"))]
        {
            let (mut operation, mut fixtures) = self.quest_xp_gain_context_like_cpp();
            operation.give_xp_runtime_like_cpp(xp, victim, group_rate, &mut fixtures)
        }
        #[cfg(not(any(test, feature = "test-fixtures")))]
        self.quest_xp_gain_context_like_cpp()
            .give_xp_runtime_like_cpp(xp, victim, group_rate)
    }

    /// Give XP and retain the existing persistence boundary after all runtime
    /// publication phases complete.
    pub(crate) async fn give_xp(&mut self, xp: u32, victim: wow_core::ObjectGuid, group_rate: f32) {
        #[cfg(any(test, feature = "test-fixtures"))]
        {
            let (mut operation, mut fixtures) = self.quest_xp_gain_context_like_cpp();
            operation
                .give_xp_like_cpp(xp, victim, group_rate, &mut fixtures)
                .await;
        }
        #[cfg(not(any(test, feature = "test-fixtures")))]
        self.quest_xp_gain_context_like_cpp()
            .give_xp_like_cpp(xp, victim, group_rate)
            .await;
    }
}
