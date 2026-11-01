// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Late storage planning through the shared selected PlayerCondition provider.

use super::QuestRewardCx;

#[cfg(any(test, feature = "test-fixtures"))]
pub struct QuestRewardItemPlanningFixtureRefsLikeCpp<'a> {
    race: &'a u8,
    class: &'a u8,
    gender: &'a u8,
    specialization: &'a u32,
    position: &'a Option<wow_core::Position>,
    zone: &'a u32,
    area: &'a u32,
    zone_authority: &'a bool,
    pvp_hostile: &'a bool,
    pvp_end: &'a Option<i64>,
    contested_pvp: &'a u32,
    outdoors: &'a Option<bool>,
    taxi_destinations: &'a Vec<u32>,
    taxi_flight:
        &'a Option<wow_world_core::session::movement_protocol::RepresentedTaxiFlightStateLikeCpp>,
    taxi_flags: &'a wow_constants::UnitFlags,
    taxi_mounted: &'a bool,
    visible_auras: &'a std::collections::HashMap<u8, wow_entities::AuraApplicationLikeCpp>,
    aura_authority: &'a bool,
    aura_tombstone: &'a bool,
    threat_auras: &'a std::collections::HashMap<u8, wow_entities::AuraThreatSnapshotLikeCpp>,
    skills:
        &'a std::collections::HashMap<u16, wow_world_core::session::RepresentedPlayerSkillLikeCpp>,
    spell_state: &'a wow_world_spell::SessionSpellState,
    instances: &'a wow_world_instances::InstanceState,
    battleground: &'a wow_world_core::session::BattlegroundState,
    battleground_status: &'a Option<u8>,
    skills_complete: &'a bool,
    in_combat: &'a bool,
}

#[cfg(any(test, feature = "test-fixtures"))]
impl<'a> QuestRewardItemPlanningFixtureRefsLikeCpp<'a> {
    pub(in crate::quest) fn player_skill_records_like_cpp(
        &self,
    ) -> &std::collections::HashMap<u16, wow_world_core::session::RepresentedPlayerSkillLikeCpp>
    {
        self.skills
    }

    pub(super) fn registry_position_like_cpp(&self) -> &Option<wow_core::Position> {
        self.position
    }

    pub(super) fn registry_spell_state_like_cpp(&self) -> &wow_world_spell::SessionSpellState {
        self.spell_state
    }

    pub(in crate::quest) fn race_class_refs_like_cpp(&self) -> (&u8, &u8) {
        (self.race, self.class)
    }

    /// The selected Player-condition fixture inputs, shared by item planning
    /// and the completion visibility refresh.
    pub(in crate::quest) fn player_condition_fixture_refs_like_cpp<'b>(
        &'b self,
        level: &'b u8,
        vitals: (&'b u32, &'b u32, &'b bool),
    ) -> wow_world_core::session::PlayerConditionFixtureRefsLikeCpp<'b> {
        wow_world_core::session::PlayerConditionFixtureRefsLikeCpp::new(
            self.race,
            self.class,
            level,
            self.gender,
            self.specialization,
            self.position,
            self.zone,
            self.area,
            self.zone_authority,
            self.pvp_hostile,
            self.pvp_end,
            self.contested_pvp,
            self.outdoors,
            vitals.0,
            vitals.1,
            vitals.2,
            self.taxi_destinations,
            self.taxi_flight,
            self.taxi_flags,
            self.taxi_mounted,
            self.visible_auras,
            self.aura_authority,
            self.aura_tombstone,
            self.threat_auras,
            self.skills,
        )
    }

    /// The selected NPC-interaction fixture inputs of the spell-click filter.
    pub(in crate::quest) fn npc_interaction_fixture_refs_like_cpp<'b>(
        &'b self,
        faction_template: &'b Option<u32>,
        vitals: (&'b u32, &'b u32, &'b bool),
        reputation: &'b wow_entities::PlayerReputationStateLikeCpp,
    ) -> wow_world_core::session::NpcInteractionFixtureRefsLikeCpp<'b> {
        wow_world_core::session::NpcInteractionFixtureRefsLikeCpp::new(
            self.position,
            faction_template,
            self.race,
            self.class,
            vitals.0,
            vitals.1,
            vitals.2,
            reputation,
            self.taxi_destinations,
            self.taxi_flight,
            self.taxi_flags,
            self.taxi_mounted,
        )
    }

    pub(in crate::quest) fn condition_projection_fixtures_like_cpp(
        &self,
    ) -> (
        &wow_world_spell::SessionSpellState,
        &wow_world_instances::InstanceState,
        &wow_world_core::session::BattlegroundState,
        &Option<u8>,
        &bool,
        &bool,
    ) {
        (
            self.spell_state,
            self.instances,
            self.battleground,
            self.battleground_status,
            self.skills_complete,
            self.in_combat,
        )
    }

    pub fn new_like_cpp(
        race: &'a u8,
        class: &'a u8,
        gender: &'a u8,
        specialization: &'a u32,
        position: &'a Option<wow_core::Position>,
        zone: &'a u32,
        area: &'a u32,
        zone_authority: &'a bool,
        pvp_hostile: &'a bool,
        pvp_end: &'a Option<i64>,
        contested_pvp: &'a u32,
        outdoors: &'a Option<bool>,
        taxi_destinations: &'a Vec<u32>,
        taxi_flight: &'a Option<
            wow_world_core::session::movement_protocol::RepresentedTaxiFlightStateLikeCpp,
        >,
        taxi_flags: &'a wow_constants::UnitFlags,
        taxi_mounted: &'a bool,
        visible_auras: &'a std::collections::HashMap<u8, wow_entities::AuraApplicationLikeCpp>,
        aura_authority: &'a bool,
        aura_tombstone: &'a bool,
        threat_auras: &'a std::collections::HashMap<u8, wow_entities::AuraThreatSnapshotLikeCpp>,
        skills: &'a std::collections::HashMap<
            u16,
            wow_world_core::session::RepresentedPlayerSkillLikeCpp,
        >,
        spell_state: &'a wow_world_spell::SessionSpellState,
        instances: &'a wow_world_instances::InstanceState,
        battleground: &'a wow_world_core::session::BattlegroundState,
        battleground_status: &'a Option<u8>,
        skills_complete: &'a bool,
        in_combat: &'a bool,
    ) -> Self {
        Self {
            race,
            class,
            gender,
            specialization,
            position,
            zone,
            area,
            zone_authority,
            pvp_hostile,
            pvp_end,
            contested_pvp,
            outdoors,
            taxi_destinations,
            taxi_flight,
            taxi_flags,
            taxi_mounted,
            visible_auras,
            aura_authority,
            aura_tombstone,
            threat_auras,
            skills,
            spell_state,
            instances,
            battleground,
            battleground_status,
            skills_complete,
            in_combat,
        }
    }
}

impl QuestRewardCx<'_> {
    pub(super) fn plan_store_new_direct_inventory_item_like_cpp(
        &self,
        entry_id: u32,
        count: u32,
        #[cfg(any(test, feature = "test-fixtures"))]
        fixtures: &QuestRewardItemPlanningFixtureRefsLikeCpp<'_>,
        #[cfg(any(test, feature = "test-fixtures"))] vitals: (&u32, &u32, &bool),
        #[cfg(any(test, feature = "test-fixtures"))]
        reputation: &wow_entities::PlayerReputationStateLikeCpp,
    ) -> Option<(
        wow_constants::InventoryResult,
        Vec<wow_entities::ItemPosCount>,
        Option<u32>,
    )> {
        let player = self.player.item_planning_condition_access_like_cpp(
            #[cfg(any(test, feature = "test-fixtures"))]
            fixtures.player_condition_fixture_refs_like_cpp(&*self.player_level, vitals),
        );
        let conditions = self.condition_projection_like_cpp(
            player,
            #[cfg(any(test, feature = "test-fixtures"))]
            fixtures,
            #[cfg(any(test, feature = "test-fixtures"))]
            reputation,
        );
        conditions.plan_store_direct_inventory_item_like_cpp(
            self.player.item_planning_realm_id_like_cpp(),
            entry_id,
            count,
            wow_entities::NULL_BAG,
            wow_entities::NULL_SLOT,
            None,
            false,
            &[],
            &[],
        )
    }

    /// The selected Player-condition projection over this operation's split
    /// borrows; item planning and the completion visibility refresh share it.
    pub(in crate::quest) fn condition_projection_like_cpp<'b>(
        &'b self,
        player: wow_world_core::session::PlayerConditionAccessLikeCpp<'b>,
        #[cfg(any(test, feature = "test-fixtures"))]
        fixtures: &'b QuestRewardItemPlanningFixtureRefsLikeCpp<'_>,
        #[cfg(any(test, feature = "test-fixtures"))]
        reputation: &'b wow_entities::PlayerReputationStateLikeCpp,
    ) -> wow_world_player_conditions::PlayerConditionProjectionCxLikeCpp<'b> {
        #[cfg(any(test, feature = "test-fixtures"))]
        let (spell_state, instances, battleground, battleground_status, skills_complete, in_combat) =
            fixtures.condition_projection_fixtures_like_cpp();
        wow_world_player_conditions::PlayerConditionProjectionCxLikeCpp::new(
            player,
            &*self.inventory,
            self.social,
            self.catalogs
                .chr_specialization_store()
                .map(std::sync::Arc::as_ref),
            #[cfg(any(test, feature = "test-fixtures"))]
            spell_state,
            #[cfg(any(test, feature = "test-fixtures"))]
            &*self.quest_state,
            #[cfg(any(test, feature = "test-fixtures"))]
            instances,
            #[cfg(any(test, feature = "test-fixtures"))]
            battleground,
            self.catalogs.inventory_valuation_catalog_view_like_cpp(),
            #[cfg(any(test, feature = "test-fixtures"))]
            reputation,
            #[cfg(any(test, feature = "test-fixtures"))]
            battleground_status,
            #[cfg(any(test, feature = "test-fixtures"))]
            skills_complete,
            #[cfg(any(test, feature = "test-fixtures"))]
            in_combat,
            #[cfg(any(test, feature = "test-fixtures"))]
            self.world_test_consumer,
        )
    }
}
