// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Completion visibility refresh run inside the objective-drain operation.
//!
//! C++ anchor (`a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`):
//! `Player::CompleteQuest` (`Player.cpp:14496`) calls `SetQuestStatus`
//! (`:15557`), whose `SendQuestUpdate` (`:15624`) ends with
//! `UpdateVisibleGameobjectsOrSpellClicks` (`:15672`, body `:24433`) before the
//! tracking-event reward. The scans are the existing application refresh; this
//! context lends them the participants it already borrows and the two
//! viewer-dependent projections, built from the same selected owners the World
//! adapter uses.

use wow_core::ObjectGuid;
use wow_data::spell_click::UNIT_NPC_FLAG_SPELLCLICK_LIKE_CPP;
use wow_packet::packets::update::{UnitDataValuesDeltaUpdate, UpdateObject};
use wow_world_core::session::PlayerConditionAccessLikeCpp;
use wow_world_entities::RepresentedGameObjectUseState;

use super::QuestObjectiveProgressCx;
use crate::quest::{
    QuestEligibilityCx, VisibilityRefreshCxLikeCpp, VisibilityRefreshHostLikeCpp,
    represented_can_see_spell_click_on_like_cpp,
    represented_viewer_dependent_creature_npc_flags_like_cpp,
    update_visible_gameobjects_or_spell_clicks_like_cpp,
};
use wow_world_player_conditions::PlayerConditionProjectionCxLikeCpp;

impl QuestObjectiveProgressCx<'_, '_> {
    /// C++ `Player::UpdateVisibleGameobjectsOrSpellClicks` at quest completion.
    pub(super) fn update_visible_gameobjects_or_spell_clicks_like_cpp(&self) -> usize {
        let cx = VisibilityRefreshCxLikeCpp {
            core: self.reward.player.core_ref_like_cpp(),
            catalogs: self.reward.catalogs,
            quest_state: &*self.reward.quest_state,
            world_entities: self.world_entities,
            consumer_test: self.reward.world_test_consumer,
        };
        update_visible_gameobjects_or_spell_clicks_like_cpp(self, &cx)
    }

    fn completion_player_condition_access_like_cpp(&self) -> PlayerConditionAccessLikeCpp<'_> {
        self.reward
            .player
            .core_ref_like_cpp()
            .player_condition_access_with_selected_fixture_refs_like_cpp(
                #[cfg(any(test, feature = "test-fixtures"))]
                self.item_planning_fixtures
                    .player_condition_fixture_refs_like_cpp(
                        &*self.reward.player_level,
                        self.xp_fixtures.vitals_fixture_refs_like_cpp(),
                    ),
            )
    }

    fn completion_condition_projection_like_cpp(&self) -> PlayerConditionProjectionCxLikeCpp<'_> {
        self.reward.condition_projection_like_cpp(
            self.completion_player_condition_access_like_cpp(),
            #[cfg(any(test, feature = "test-fixtures"))]
            self.item_planning_fixtures,
            #[cfg(any(test, feature = "test-fixtures"))]
            self.reputation_fixture.state_like_cpp(),
        )
    }
}

/// The two viewer-dependent projections, evaluated against the post-completion
/// state exactly as the World adapter evaluates them.
impl VisibilityRefreshHostLikeCpp for QuestObjectiveProgressCx<'_, '_> {
    fn visibility_refresh_gameobject_dynamic_flags_like_cpp(
        &self,
        gameobject_entry: u32,
        state: &RepresentedGameObjectUseState,
    ) -> u32 {
        let core = self.reward.player.core_ref_like_cpp();
        let conditions = self.completion_condition_projection_like_cpp();
        #[cfg(any(test, feature = "test-fixtures"))]
        let (race, class) = self.item_planning_fixtures.race_class_refs_like_cpp();
        let eligibility = QuestEligibilityCx::new(
            core.quest_eligibility_access_like_cpp(
                #[cfg(any(test, feature = "test-fixtures"))]
                race,
                #[cfg(any(test, feature = "test-fixtures"))]
                class,
                #[cfg(any(test, feature = "test-fixtures"))]
                &*self.reward.player_level,
                #[cfg(any(test, feature = "test-fixtures"))]
                self.item_planning_fixtures.player_skill_records_like_cpp(),
                #[cfg(any(test, feature = "test-fixtures"))]
                self.reputation_fixture.state_like_cpp(),
            ),
            &*self.reward.quest_state,
            self.reward.catalogs,
            &conditions,
            self.reward.world_test_consumer,
        );
        self.represented_gameobject_dynamic_flags_for_player_like_cpp(
            &eligibility,
            gameobject_entry,
            state,
        )
    }

    fn visibility_refresh_unit_npc_flags_update_like_cpp(
        &self,
        guid: ObjectGuid,
        map_id: u16,
        mut packet_update: UnitDataValuesDeltaUpdate,
    ) -> UpdateObject {
        if guid.is_any_type_creature()
            && (packet_update.npc_flags[0] & UNIT_NPC_FLAG_SPELLCLICK_LIKE_CPP as u32) != 0
        {
            let npc_flags = u64::from(packet_update.npc_flags[0])
                | (u64::from(packet_update.npc_flags[1]) << 32);
            let filtered = represented_viewer_dependent_creature_npc_flags_like_cpp(
                npc_flags,
                self.completion_can_see_spell_click_like_cpp(guid),
            );
            packet_update.npc_flags[0] = filtered as u32;
            packet_update.npc_flags[1] = (filtered >> 32) as u32;
        }
        UpdateObject::unit_values_update(guid, map_id, packet_update)
    }
}

impl QuestObjectiveProgressCx<'_, '_> {
    fn completion_can_see_spell_click_like_cpp(
        &self,
        creature_guid: ObjectGuid,
    ) -> wow_world_entities::spell_click_values::RepresentedCanSeeSpellClickOutcomeLikeCpp {
        let core = self.reward.player.core_ref_like_cpp();
        let catalogs = self.reward.catalogs;
        let owner = core.quest_objective_access_like_cpp();
        let npc_access = core.npc_interaction_access_with_selected_refs_like_cpp(
            catalogs.factions.store.as_deref(),
            catalogs.factions.template_store.as_deref(),
            catalogs.friendship_rep_reaction_store.as_deref(),
            #[cfg(any(test, feature = "test-fixtures"))]
            self.item_planning_fixtures
                .npc_interaction_fixture_refs_like_cpp(
                    self.registry_fixtures.faction_template,
                    self.xp_fixtures.vitals_fixture_refs_like_cpp(),
                    self.reputation_fixture.state_like_cpp(),
                ),
        );
        let player_access = self.completion_player_condition_access_like_cpp();
        let condition_projection = self.completion_condition_projection_like_cpp();
        represented_can_see_spell_click_on_like_cpp(
            self.world_entities,
            &owner,
            &npc_access,
            &player_access,
            &condition_projection,
            self.reward.social,
            catalogs.chr.specialization_store.as_deref(),
            creature_guid,
            catalogs.spell_catalogs.npc_spell_click_store.as_deref(),
            catalogs.condition_store.as_deref(),
            catalogs.player_condition_store.as_ref(),
            catalogs.area_table_store.as_ref(),
            self.reward.world_test_consumer,
        )
    }
}
