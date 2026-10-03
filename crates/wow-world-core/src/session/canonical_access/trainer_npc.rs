// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Selected canonical inputs for NPC interaction checks.

use crate::session::{HubRef, SessionCore};
use wow_core::{ObjectGuid, Position};
use wow_data::progression_rewards::{
    FactionStore, FactionTemplateStore, FriendshipRepReactionStore,
};

#[cfg(any(test, feature = "test-fixtures"))]
use wow_constants::UnitFlags;

/// Only fixture fields read by NPC interaction and its shared reputation
/// reaction kernel. No session fixture aggregate is retained.
#[cfg(any(test, feature = "test-fixtures"))]
pub struct NpcInteractionFixtureRefsLikeCpp<'a> {
    pub(crate) position: &'a Option<Position>,
    pub(crate) faction_template_id: &'a Option<u32>,
    pub(crate) player_race: &'a u8,
    pub(crate) player_class: &'a u8,
    pub(crate) player_health: &'a u32,
    pub(crate) player_max_health: &'a u32,
    pub(crate) player_alive: &'a bool,
    pub(crate) reputation_state: &'a wow_entities::PlayerReputationStateLikeCpp,
    pub(crate) taxi_destinations: &'a Vec<u32>,
    pub(crate) taxi_flight_state:
        &'a Option<crate::session::movement_protocol::RepresentedTaxiFlightStateLikeCpp>,
    pub(crate) taxi_unit_flags: &'a UnitFlags,
    pub(crate) taxi_mounted: &'a bool,
}

#[cfg(any(test, feature = "test-fixtures"))]
impl<'a> NpcInteractionFixtureRefsLikeCpp<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        position: &'a Option<Position>,
        faction_template_id: &'a Option<u32>,
        player_race: &'a u8,
        player_class: &'a u8,
        player_health: &'a u32,
        player_max_health: &'a u32,
        player_alive: &'a bool,
        reputation_state: &'a wow_entities::PlayerReputationStateLikeCpp,
        taxi_destinations: &'a Vec<u32>,
        taxi_flight_state: &'a Option<crate::session::movement_protocol::RepresentedTaxiFlightStateLikeCpp>,
        taxi_unit_flags: &'a UnitFlags,
        taxi_mounted: &'a bool,
    ) -> Self {
        Self {
            position,
            faction_template_id,
            player_race,
            player_class,
            player_health,
            player_max_health,
            player_alive,
            reputation_state,
            taxi_destinations,
            taxi_flight_state,
            taxi_unit_flags,
            taxi_mounted,
        }
    }
}

/// A short-lived Core capability for the existing NPC-interaction operation.
/// It owns only selected canonical and catalog references, never HubRef.
pub struct NpcInteractionAccessLikeCpp<'a> {
    pub(crate) core: &'a SessionCore,
    pub(crate) faction_store: Option<&'a FactionStore>,
    pub(crate) faction_template_store: Option<&'a FactionTemplateStore>,
    pub(crate) friendship_rep_reaction_store: Option<&'a FriendshipRepReactionStore>,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) fixtures: NpcInteractionFixtureRefsLikeCpp<'a>,
}

impl SessionCore {
    pub fn npc_interaction_access_with_selected_refs_like_cpp<'a>(
        &'a self,
        faction_store: Option<&'a FactionStore>,
        faction_template_store: Option<&'a FactionTemplateStore>,
        friendship_rep_reaction_store: Option<&'a FriendshipRepReactionStore>,
        #[cfg(any(test, feature = "test-fixtures"))]
        fixtures: NpcInteractionFixtureRefsLikeCpp<'a>,
    ) -> NpcInteractionAccessLikeCpp<'a> {
        NpcInteractionAccessLikeCpp {
            core: self,
            faction_store,
            faction_template_store,
            friendship_rep_reaction_store,
            #[cfg(any(test, feature = "test-fixtures"))]
            fixtures,
        }
    }
}

impl HubRef<'_> {
    /// Select the exact catalog and fixture inputs for one NPC check.
    pub fn trainer_npc_interaction_access_like_cpp(
        &self,
    ) -> NpcInteractionAccessLikeCpp<'_> {
        self.core.npc_interaction_access_with_selected_refs_like_cpp(
            self.catalogs.factions.store.as_deref(),
            self.catalogs.factions.template_store.as_deref(),
            self.catalogs.friendship_rep_reaction_store.as_deref(),
            #[cfg(any(test, feature = "test-fixtures"))]
            NpcInteractionFixtureRefsLikeCpp::new(
                &self.fixtures.movement.player_position,
                &self.fixtures.identity.player_faction_template_like_cpp,
                &self.fixtures.identity.player_race,
                &self.fixtures.identity.player_class,
                &self.fixtures.combat.player_health_like_cpp,
                &self.fixtures.combat.player_max_health_like_cpp,
                &self.fixtures.combat.player_alive_like_cpp,
                &self.fixtures.progression.reputation_state_like_cpp,
                &self.fixtures.vehicles.taxi_destinations_like_cpp,
                &self.fixtures.vehicles.taxi_flight_state_like_cpp,
                &self.fixtures.vehicles.taxi_unit_flags_like_cpp,
                &self.fixtures.vehicles.taxi_mounted_like_cpp,
            ),
        )
    }
}

impl NpcInteractionAccessLikeCpp<'_> {
    pub(crate) fn player_race_like_cpp(&self) -> u8 {
        #[cfg(any(test, feature = "test-fixtures"))]
        {
            self.core
                .player_race_with_fixture_like_cpp(self.fixtures.player_race)
        }
        #[cfg(not(any(test, feature = "test-fixtures")))]
        {
            self.core.player_race_with_fixture_like_cpp()
        }
    }

    pub(crate) fn player_class_like_cpp(&self) -> u8 {
        #[cfg(any(test, feature = "test-fixtures"))]
        {
            self.core
                .player_class_with_fixture_like_cpp(self.fixtures.player_class)
        }
        #[cfg(not(any(test, feature = "test-fixtures")))]
        {
            self.core.player_class_with_fixture_like_cpp()
        }
    }

    pub(crate) fn player_position_like_cpp(&self) -> Option<Position> {
        self.core.player_position_with_fixture_like_cpp(
            #[cfg(any(test, feature = "test-fixtures"))]
            self.fixtures.position,
        )
    }

    pub(crate) fn player_faction_template_id_like_cpp(&self) -> Option<u32> {
        let canonical = self.core.with_owned_player_like_cpp(|player| {
            u32::try_from(player.unit().data().faction_template)
                .ok()
                .filter(|faction| *faction != 0)
        });
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            return *self.fixtures.faction_template_id;
        }
        canonical.flatten()
    }

    pub(crate) fn resolved_is_in_taxi_flight_like_cpp(&self) -> Option<bool> {
        let canonical = self
            .core
            .with_owned_player_like_cpp(|player| player.taxi_state_like_cpp().clone());
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            return Some(
                wow_entities::PlayerTaxiState::from_represented_parts_like_cpp(
                    self.fixtures.taxi_destinations.clone(),
                    self.fixtures
                        .taxi_flight_state
                        .map(crate::session::movement_protocol::canonical_taxi_flight_state_like_cpp),
                    self.fixtures.taxi_unit_flags.bits(),
                    *self.fixtures.taxi_mounted,
                )
                .is_in_flight_like_cpp(),
            );
        }
        canonical.map(|taxi| taxi.is_in_flight_like_cpp())
    }

    pub(crate) fn resolved_player_is_alive_like_cpp(&self) -> Option<bool> {
        self.core
            .resolved_player_vitals_with_fixture_like_cpp(
                #[cfg(any(test, feature = "test-fixtures"))]
                self.fixtures.player_health,
                #[cfg(any(test, feature = "test-fixtures"))]
                self.fixtures.player_max_health,
                #[cfg(any(test, feature = "test-fixtures"))]
                self.fixtures.player_alive,
            )
            .map(|(_, _, alive)| alive)
    }
}
