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
    taxi_flight: &'a Option<wow_world_core::session::movement_protocol::RepresentedTaxiFlightStateLikeCpp>,
    taxi_flags: &'a wow_constants::UnitFlags,
    taxi_mounted: &'a bool,
    visible_auras: &'a std::collections::HashMap<u8, wow_entities::AuraApplicationLikeCpp>,
    aura_authority: &'a bool,
    aura_tombstone: &'a bool,
    threat_auras: &'a std::collections::HashMap<u8, wow_entities::AuraThreatSnapshotLikeCpp>,
    skills: &'a std::collections::HashMap<u16, wow_world_core::session::RepresentedPlayerSkillLikeCpp>,
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
        taxi_flight: &'a Option<wow_world_core::session::movement_protocol::RepresentedTaxiFlightStateLikeCpp>,
        taxi_flags: &'a wow_constants::UnitFlags,
        taxi_mounted: &'a bool,
        visible_auras: &'a std::collections::HashMap<u8, wow_entities::AuraApplicationLikeCpp>,
        aura_authority: &'a bool,
        aura_tombstone: &'a bool,
        threat_auras: &'a std::collections::HashMap<u8, wow_entities::AuraThreatSnapshotLikeCpp>,
        skills: &'a std::collections::HashMap<u16, wow_world_core::session::RepresentedPlayerSkillLikeCpp>,
        spell_state: &'a wow_world_spell::SessionSpellState,
        instances: &'a wow_world_instances::InstanceState,
        battleground: &'a wow_world_core::session::BattlegroundState,
        battleground_status: &'a Option<u8>,
        skills_complete: &'a bool,
        in_combat: &'a bool,
    ) -> Self {
        Self {
            race, class, gender, specialization, position, zone, area, zone_authority,
            pvp_hostile, pvp_end, contested_pvp, outdoors, taxi_destinations, taxi_flight,
            taxi_flags, taxi_mounted, visible_auras, aura_authority, aura_tombstone,
            threat_auras, skills, spell_state, instances, battleground,
            battleground_status, skills_complete, in_combat,
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
        #[cfg(any(test, feature = "test-fixtures"))]
        vitals: (&u32, &u32, &bool),
        #[cfg(any(test, feature = "test-fixtures"))]
        reputation: &wow_entities::PlayerReputationStateLikeCpp,
    ) -> Option<(wow_constants::InventoryResult, Vec<wow_entities::ItemPosCount>, Option<u32>)> {
        let player = self.player.item_planning_condition_access_like_cpp(
            #[cfg(any(test, feature = "test-fixtures"))]
            wow_world_core::session::PlayerConditionFixtureRefsLikeCpp::new(
                fixtures.race, fixtures.class, &*self.player_level, fixtures.gender,
                fixtures.specialization, fixtures.position, fixtures.zone, fixtures.area,
                fixtures.zone_authority, fixtures.pvp_hostile, fixtures.pvp_end,
                fixtures.contested_pvp, fixtures.outdoors, vitals.0, vitals.1, vitals.2,
                fixtures.taxi_destinations, fixtures.taxi_flight, fixtures.taxi_flags,
                fixtures.taxi_mounted, fixtures.visible_auras, fixtures.aura_authority,
                fixtures.aura_tombstone, fixtures.threat_auras, fixtures.skills,
            ),
        );
        let conditions = crate::PlayerConditionProjectionCxLikeCpp::new(
            player,
            self.inventory,
            self.social,
            self.catalogs.chr_specialization_store().map(std::sync::Arc::as_ref),
            #[cfg(any(test, feature = "test-fixtures"))]
            fixtures.spell_state,
            #[cfg(any(test, feature = "test-fixtures"))]
            self.quest_state,
            #[cfg(any(test, feature = "test-fixtures"))]
            fixtures.instances,
            #[cfg(any(test, feature = "test-fixtures"))]
            fixtures.battleground,
            self.catalogs.inventory_valuation_catalog_view_like_cpp(),
            #[cfg(any(test, feature = "test-fixtures"))]
            reputation,
            #[cfg(any(test, feature = "test-fixtures"))]
            fixtures.battleground_status,
            #[cfg(any(test, feature = "test-fixtures"))]
            fixtures.skills_complete,
            #[cfg(any(test, feature = "test-fixtures"))]
            fixtures.in_combat,
            #[cfg(any(test, feature = "test-fixtures"))]
            self.world_test_consumer,
        );
        conditions.plan_store_direct_inventory_item_like_cpp(
            self.player.item_planning_realm_id_like_cpp(), entry_id, count,
            wow_entities::NULL_BAG, wow_entities::NULL_SLOT, None, false, &[], &[],
        )
    }
}
