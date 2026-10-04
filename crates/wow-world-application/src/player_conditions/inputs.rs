// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use super::PlayerConditionProjectionCxLikeCpp;

/// Inert condition inputs independent of the mutable aura/inventory owners.
pub struct PlayerConditionProjectionInputsLikeCpp<'a> {
    player: wow_world_core::session::AuraConditionAccessBuilderLikeCpp<'a>,
    social: &'a wow_world_social::SessionSocialLimits,
    chr_specializations: Option<&'a wow_data::ChrSpecializationStore>,
    #[cfg(any(test, feature = "test-fixtures"))]
    quest_state: &'a crate::SessionQuestState,
    #[cfg(any(test, feature = "test-fixtures"))]
    instances: &'a wow_world_instances::InstanceState,
    #[cfg(any(test, feature = "test-fixtures"))]
    battleground_fixture: &'a wow_world_core::session::BattlegroundState,
    valuation_catalogs: wow_world_core::session::InventoryValuationCatalogViewLikeCpp<'a>,
    #[cfg(any(test, feature = "test-fixtures"))]
    reputation_state: &'a wow_entities::PlayerReputationStateLikeCpp,
    #[cfg(any(test, feature = "test-fixtures"))]
    battleground_status: &'a Option<u8>,
    #[cfg(any(test, feature = "test-fixtures"))]
    player_skill_records_complete: &'a bool,
    #[cfg(any(test, feature = "test-fixtures"))]
    in_combat: &'a bool,
    #[cfg(any(test, feature = "test-fixtures"))]
    consumer_test: bool,
}

impl<'a> PlayerConditionProjectionInputsLikeCpp<'a> {
    pub fn new(
        player: wow_world_core::session::AuraConditionAccessBuilderLikeCpp<'a>,
        social: &'a wow_world_social::SessionSocialLimits,
        chr_specializations: Option<&'a wow_data::ChrSpecializationStore>,
        #[cfg(any(test, feature = "test-fixtures"))] quest_state: &'a crate::SessionQuestState,
        #[cfg(any(test, feature = "test-fixtures"))] instances: &'a wow_world_instances::InstanceState,
        #[cfg(any(test, feature = "test-fixtures"))] battleground_fixture: &'a wow_world_core::session::BattlegroundState,
        valuation_catalogs: wow_world_core::session::InventoryValuationCatalogViewLikeCpp<'a>,
        #[cfg(any(test, feature = "test-fixtures"))] reputation_state: &'a wow_entities::PlayerReputationStateLikeCpp,
        #[cfg(any(test, feature = "test-fixtures"))] battleground_status: &'a Option<u8>,
        #[cfg(any(test, feature = "test-fixtures"))] player_skill_records_complete: &'a bool,
        #[cfg(any(test, feature = "test-fixtures"))] in_combat: &'a bool,
        #[cfg(any(test, feature = "test-fixtures"))] consumer_test: bool,
    ) -> Self {
        Self { player,
            social,
            chr_specializations,
            #[cfg(any(test, feature = "test-fixtures"))] quest_state,
            #[cfg(any(test, feature = "test-fixtures"))] instances,
            #[cfg(any(test, feature = "test-fixtures"))] battleground_fixture,
            valuation_catalogs,
            #[cfg(any(test, feature = "test-fixtures"))] reputation_state,
            #[cfg(any(test, feature = "test-fixtures"))] battleground_status,
            #[cfg(any(test, feature = "test-fixtures"))] player_skill_records_complete,
            #[cfg(any(test, feature = "test-fixtures"))] in_combat,
            #[cfg(any(test, feature = "test-fixtures"))] consumer_test,
        }
    }

    pub(crate) fn reborrow_like_cpp<'b>(
        &'b self,
        stats: &'b wow_world_core::session::AuraStatsAccessBuilderLikeCpp<'_>,
        aura: &'b wow_world_core::session::PlayerAuraRemovalAccessLikeCpp<'_>,
        inventory: &'b wow_world_inventory::InventoryState,
        #[cfg(any(test, feature = "test-fixtures"))] spell: &'b wow_world_spell::SessionSpellState,
    ) -> PlayerConditionProjectionCxLikeCpp<'b> {
        PlayerConditionProjectionCxLikeCpp::new(
            self.player.reborrow_like_cpp(stats, aura), inventory,
            self.social, self.chr_specializations,
            #[cfg(any(test, feature = "test-fixtures"))] spell,
            #[cfg(any(test, feature = "test-fixtures"))] self.quest_state,
            #[cfg(any(test, feature = "test-fixtures"))] self.instances,
            #[cfg(any(test, feature = "test-fixtures"))] self.battleground_fixture,
            self.valuation_catalogs,
            #[cfg(any(test, feature = "test-fixtures"))] self.reputation_state,
            #[cfg(any(test, feature = "test-fixtures"))] self.battleground_status,
            #[cfg(any(test, feature = "test-fixtures"))] self.player_skill_records_complete,
            #[cfg(any(test, feature = "test-fixtures"))] self.in_combat,
            #[cfg(any(test, feature = "test-fixtures"))] self.consumer_test,
        )
    }
}
