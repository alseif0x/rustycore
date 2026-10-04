// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Inert selected inputs; mutable aura/stat owners lend read views per phase.
use super::{AuraStatsAccessBuilderLikeCpp, PlayerAuraRemovalAccessLikeCpp};
use crate::session::{SessionCore, NpcInteractionAccessLikeCpp, PlayerConditionAccessLikeCpp};

/// Stores references only, including no aura or combat-health borrow.
pub struct AuraNpcAccessBuilderLikeCpp<'a> {
    core: &'a SessionCore,
    faction_store: Option<&'a wow_data::progression_rewards::FactionStore>,
    faction_template_store: Option<&'a wow_data::progression_rewards::FactionTemplateStore>,
    friendship_store: Option<&'a wow_data::progression_rewards::FriendshipRepReactionStore>,
    #[cfg(any(test, feature = "test-fixtures"))]
    position: &'a Option<wow_core::Position>,
    #[cfg(any(test, feature = "test-fixtures"))]
    faction_template_id: &'a Option<u32>,
    #[cfg(any(test, feature = "test-fixtures"))]
    player_race: &'a u8,
    #[cfg(any(test, feature = "test-fixtures"))]
    player_class: &'a u8,
    #[cfg(any(test, feature = "test-fixtures"))]
    reputation_state: &'a wow_entities::PlayerReputationStateLikeCpp,
    #[cfg(any(test, feature = "test-fixtures"))]
    taxi_destinations: &'a Vec<u32>,
    #[cfg(any(test, feature = "test-fixtures"))]
    taxi_flight_state: &'a Option<crate::session::movement_protocol::RepresentedTaxiFlightStateLikeCpp>,
    #[cfg(any(test, feature = "test-fixtures"))]
    taxi_unit_flags: &'a wow_constants::UnitFlags,
    #[cfg(any(test, feature = "test-fixtures"))]
    taxi_mounted: &'a bool,
}

impl SessionCore {
    pub fn aura_npc_access_builder_like_cpp<'a>(
        &'a self,
        faction_store: Option<&'a wow_data::progression_rewards::FactionStore>,
        faction_template_store: Option<&'a wow_data::progression_rewards::FactionTemplateStore>,
        friendship_store: Option<&'a wow_data::progression_rewards::FriendshipRepReactionStore>,
        #[cfg(any(test, feature = "test-fixtures"))] position: &'a Option<wow_core::Position>,
        #[cfg(any(test, feature = "test-fixtures"))] faction_template_id: &'a Option<u32>,
        #[cfg(any(test, feature = "test-fixtures"))] player_race: &'a u8,
        #[cfg(any(test, feature = "test-fixtures"))] player_class: &'a u8,
        #[cfg(any(test, feature = "test-fixtures"))] reputation_state: &'a wow_entities::PlayerReputationStateLikeCpp,
        #[cfg(any(test, feature = "test-fixtures"))] taxi_destinations: &'a Vec<u32>,
        #[cfg(any(test, feature = "test-fixtures"))] taxi_flight_state: &'a Option<crate::session::movement_protocol::RepresentedTaxiFlightStateLikeCpp>,
        #[cfg(any(test, feature = "test-fixtures"))] taxi_unit_flags: &'a wow_constants::UnitFlags,
        #[cfg(any(test, feature = "test-fixtures"))] taxi_mounted: &'a bool,
    ) -> AuraNpcAccessBuilderLikeCpp<'a> {
        AuraNpcAccessBuilderLikeCpp {
            core: self,
            faction_store, faction_template_store, friendship_store,
            #[cfg(any(test, feature = "test-fixtures"))] position,
            #[cfg(any(test, feature = "test-fixtures"))] faction_template_id,
            #[cfg(any(test, feature = "test-fixtures"))] player_race,
            #[cfg(any(test, feature = "test-fixtures"))] player_class,
            #[cfg(any(test, feature = "test-fixtures"))] reputation_state,
            #[cfg(any(test, feature = "test-fixtures"))] taxi_destinations,
            #[cfg(any(test, feature = "test-fixtures"))] taxi_flight_state,
            #[cfg(any(test, feature = "test-fixtures"))] taxi_unit_flags,
            #[cfg(any(test, feature = "test-fixtures"))] taxi_mounted,
        }
    }
}

impl AuraNpcAccessBuilderLikeCpp<'_> {
    pub fn reborrow_like_cpp<'b>(
        &'b self,
        stats: &'b AuraStatsAccessBuilderLikeCpp<'_>,
    ) -> NpcInteractionAccessLikeCpp<'b> {
        #[cfg(any(test, feature = "test-fixtures"))]
        let (health, max_health, alive) = stats.health_refs_like_cpp();
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let _ = stats;
        self.core.npc_interaction_access_with_selected_refs_like_cpp(
            self.faction_store, self.faction_template_store, self.friendship_store,
            #[cfg(any(test, feature = "test-fixtures"))]
            crate::session::NpcInteractionFixtureRefsLikeCpp::new(
                self.position,
                self.faction_template_id,
                self.player_race,
                self.player_class,
                health,
                max_health,
                alive,
                self.reputation_state,
                self.taxi_destinations,
                self.taxi_flight_state,
                self.taxi_unit_flags,
                self.taxi_mounted,
            ),
        )
    }
}

/// Stores references only, including no aura or combat-health borrow.
pub struct AuraConditionAccessBuilderLikeCpp<'a> {
    core: &'a SessionCore,
    #[cfg(any(test, feature = "test-fixtures"))]
    race: &'a u8,
    #[cfg(any(test, feature = "test-fixtures"))]
    class: &'a u8,
    #[cfg(any(test, feature = "test-fixtures"))]
    level: &'a u8,
    #[cfg(any(test, feature = "test-fixtures"))]
    gender: &'a u8,
    #[cfg(any(test, feature = "test-fixtures"))]
    primary_specialization_id: &'a u32,
    #[cfg(any(test, feature = "test-fixtures"))]
    position: &'a Option<wow_core::Position>,
    #[cfg(any(test, feature = "test-fixtures"))]
    zone_id: &'a u32,
    #[cfg(any(test, feature = "test-fixtures"))]
    area_id: &'a u32,
    #[cfg(any(test, feature = "test-fixtures"))]
    zone_area_authority_complete: &'a bool,
    #[cfg(any(test, feature = "test-fixtures"))]
    pvp_hostile: &'a bool,
    #[cfg(any(test, feature = "test-fixtures"))]
    pvp_end_timer: &'a Option<i64>,
    #[cfg(any(test, feature = "test-fixtures"))]
    contested_pvp_timer: &'a u32,
    #[cfg(any(test, feature = "test-fixtures"))]
    is_outdoors: &'a Option<bool>,
    #[cfg(any(test, feature = "test-fixtures"))]
    taxi_destinations: &'a Vec<u32>,
    #[cfg(any(test, feature = "test-fixtures"))]
    taxi_flight_state: &'a Option<crate::session::movement_protocol::RepresentedTaxiFlightStateLikeCpp>,
    #[cfg(any(test, feature = "test-fixtures"))]
    taxi_unit_flags: &'a wow_constants::UnitFlags,
    #[cfg(any(test, feature = "test-fixtures"))]
    taxi_mounted: &'a bool,
    #[cfg(any(test, feature = "test-fixtures"))]
    player_skill_records: &'a std::collections::HashMap<u16,crate::session::RepresentedPlayerSkillLikeCpp>,
}

impl SessionCore {
    pub fn aura_condition_access_builder_like_cpp<'a>(
        &'a self,
        #[cfg(any(test, feature = "test-fixtures"))] race: &'a u8,
        #[cfg(any(test, feature = "test-fixtures"))] class: &'a u8,
        #[cfg(any(test, feature = "test-fixtures"))] level: &'a u8,
        #[cfg(any(test, feature = "test-fixtures"))] gender: &'a u8,
        #[cfg(any(test, feature = "test-fixtures"))] primary_specialization_id: &'a u32,
        #[cfg(any(test, feature = "test-fixtures"))] position: &'a Option<wow_core::Position>,
        #[cfg(any(test, feature = "test-fixtures"))] zone_id: &'a u32,
        #[cfg(any(test, feature = "test-fixtures"))] area_id: &'a u32,
        #[cfg(any(test, feature = "test-fixtures"))] zone_area_authority_complete: &'a bool,
        #[cfg(any(test, feature = "test-fixtures"))] pvp_hostile: &'a bool,
        #[cfg(any(test, feature = "test-fixtures"))] pvp_end_timer: &'a Option<i64>,
        #[cfg(any(test, feature = "test-fixtures"))] contested_pvp_timer: &'a u32,
        #[cfg(any(test, feature = "test-fixtures"))] is_outdoors: &'a Option<bool>,
        #[cfg(any(test, feature = "test-fixtures"))] taxi_destinations: &'a Vec<u32>,
        #[cfg(any(test, feature = "test-fixtures"))] taxi_flight_state: &'a Option<crate::session::movement_protocol::RepresentedTaxiFlightStateLikeCpp>,
        #[cfg(any(test, feature = "test-fixtures"))] taxi_unit_flags: &'a wow_constants::UnitFlags,
        #[cfg(any(test, feature = "test-fixtures"))] taxi_mounted: &'a bool,
        #[cfg(any(test, feature = "test-fixtures"))] player_skill_records: &'a std::collections::HashMap<u16,crate::session::RepresentedPlayerSkillLikeCpp>,
    ) -> AuraConditionAccessBuilderLikeCpp<'a> {
        AuraConditionAccessBuilderLikeCpp {
            core: self,
            #[cfg(any(test, feature = "test-fixtures"))] race,
            #[cfg(any(test, feature = "test-fixtures"))] class,
            #[cfg(any(test, feature = "test-fixtures"))] level,
            #[cfg(any(test, feature = "test-fixtures"))] gender,
            #[cfg(any(test, feature = "test-fixtures"))] primary_specialization_id,
            #[cfg(any(test, feature = "test-fixtures"))] position,
            #[cfg(any(test, feature = "test-fixtures"))] zone_id,
            #[cfg(any(test, feature = "test-fixtures"))] area_id,
            #[cfg(any(test, feature = "test-fixtures"))] zone_area_authority_complete,
            #[cfg(any(test, feature = "test-fixtures"))] pvp_hostile,
            #[cfg(any(test, feature = "test-fixtures"))] pvp_end_timer,
            #[cfg(any(test, feature = "test-fixtures"))] contested_pvp_timer,
            #[cfg(any(test, feature = "test-fixtures"))] is_outdoors,
            #[cfg(any(test, feature = "test-fixtures"))] taxi_destinations,
            #[cfg(any(test, feature = "test-fixtures"))] taxi_flight_state,
            #[cfg(any(test, feature = "test-fixtures"))] taxi_unit_flags,
            #[cfg(any(test, feature = "test-fixtures"))] taxi_mounted,
            #[cfg(any(test, feature = "test-fixtures"))] player_skill_records,
        }
    }
}

impl AuraConditionAccessBuilderLikeCpp<'_> {
    pub fn reborrow_like_cpp<'b>(
        &'b self,
        stats: &'b AuraStatsAccessBuilderLikeCpp<'_>,
        aura: &'b PlayerAuraRemovalAccessLikeCpp<'_>,
    ) -> PlayerConditionAccessLikeCpp<'b> {
        #[cfg(any(test, feature = "test-fixtures"))]
        let (health, max_health, alive) = stats.health_refs_like_cpp();
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let _ = (stats, aura);
        self.core.player_condition_access_with_selected_fixture_refs_like_cpp(
            #[cfg(any(test, feature = "test-fixtures"))]
            crate::session::PlayerConditionFixtureRefsLikeCpp::new(
                self.race,
                self.class,
                self.level,
                self.gender,
                self.primary_specialization_id,
                self.position,
                self.zone_id,
                self.area_id,
                self.zone_area_authority_complete,
                self.pvp_hostile,
                self.pvp_end_timer,
                self.contested_pvp_timer,
                self.is_outdoors,
                health,
                max_health,
                alive,
                self.taxi_destinations,
                self.taxi_flight_state,
                self.taxi_unit_flags,
                self.taxi_mounted,
                &*aura.fixtures.visible,
                &*aura.fixtures.complete,
                &*aura.fixtures.tombstoned,
                &*aura.fixtures.threat,
                self.player_skill_records,
            ),
        )
    }
}

