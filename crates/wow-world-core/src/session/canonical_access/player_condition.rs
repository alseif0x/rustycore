// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Selected canonical inputs for one PlayerCondition evaluation.

use crate::session::{PlayerGroupOwnerAccessLikeCpp, RepresentedPlayerSkillLikeCpp, SessionCore};
use std::collections::HashMap;
#[cfg(any(test, feature = "test-fixtures"))]
use wow_constants::UnitFlags;
use wow_constants::{TypeId, TypeMask, UnitStandStateType};
use wow_core::Position;
use wow_entities::{Player, PlayerTaxiState, PlayerWorldLocalState, WorldObject};

/// Fixture values read by PlayerCondition evaluation when its source World
/// consumer enables the corresponding test fallback.
#[cfg(any(test, feature = "test-fixtures"))]
pub struct PlayerConditionFixtureRefsLikeCpp<'a> {
    pub(crate) race: &'a u8,
    pub(crate) class: &'a u8,
    pub(crate) level: &'a u8,
    pub(crate) gender: &'a u8,
    pub(crate) primary_specialization_id: &'a u32,
    pub(crate) position: &'a Option<Position>,
    pub(crate) zone_id: &'a u32,
    pub(crate) area_id: &'a u32,
    pub(crate) zone_area_authority_complete: &'a bool,
    pub(crate) pvp_hostile: &'a bool,
    pub(crate) pvp_end_timer: &'a Option<i64>,
    pub(crate) contested_pvp_timer: &'a u32,
    pub(crate) is_outdoors: &'a Option<bool>,
    pub(crate) health: &'a u32,
    pub(crate) max_health: &'a u32,
    pub(crate) alive: &'a bool,
    pub(crate) taxi_destinations: &'a Vec<u32>,
    pub(crate) taxi_flight_state:
        &'a Option<crate::session::movement_protocol::RepresentedTaxiFlightStateLikeCpp>,
    pub(crate) taxi_unit_flags: &'a UnitFlags,
    pub(crate) taxi_mounted: &'a bool,
    pub(crate) visible_auras: &'a HashMap<u8, wow_entities::AuraApplicationLikeCpp>,
    pub(crate) aura_authority_complete: &'a bool,
    pub(crate) spell_hit_aura_authority_tombstoned: &'a bool,
    pub(crate) canonical_threat_aura_snapshots:
        &'a HashMap<u8, wow_entities::AuraThreatSnapshotLikeCpp>,
    pub(crate) player_skill_records: &'a HashMap<u16, RepresentedPlayerSkillLikeCpp>,
}

#[cfg(any(test, feature = "test-fixtures"))]
impl<'a> PlayerConditionFixtureRefsLikeCpp<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        race: &'a u8,
        class: &'a u8,
        level: &'a u8,
        gender: &'a u8,
        primary_specialization_id: &'a u32,
        position: &'a Option<Position>,
        zone_id: &'a u32,
        area_id: &'a u32,
        zone_area_authority_complete: &'a bool,
        pvp_hostile: &'a bool,
        pvp_end_timer: &'a Option<i64>,
        contested_pvp_timer: &'a u32,
        is_outdoors: &'a Option<bool>,
        health: &'a u32,
        max_health: &'a u32,
        alive: &'a bool,
        taxi_destinations: &'a Vec<u32>,
        taxi_flight_state: &'a Option<
            crate::session::movement_protocol::RepresentedTaxiFlightStateLikeCpp,
        >,
        taxi_unit_flags: &'a UnitFlags,
        taxi_mounted: &'a bool,
        visible_auras: &'a HashMap<u8, wow_entities::AuraApplicationLikeCpp>,
        aura_authority_complete: &'a bool,
        spell_hit_aura_authority_tombstoned: &'a bool,
        canonical_threat_aura_snapshots: &'a HashMap<u8, wow_entities::AuraThreatSnapshotLikeCpp>,
        player_skill_records: &'a HashMap<u16, RepresentedPlayerSkillLikeCpp>,
    ) -> Self {
        Self {
            race,
            class,
            level,
            gender,
            primary_specialization_id,
            position,
            zone_id,
            area_id,
            zone_area_authority_complete,
            pvp_hostile,
            pvp_end_timer,
            contested_pvp_timer,
            is_outdoors,
            health,
            max_health,
            alive,
            taxi_destinations,
            taxi_flight_state,
            taxi_unit_flags,
            taxi_mounted,
            visible_auras,
            aura_authority_complete,
            spell_hit_aura_authority_tombstoned,
            canonical_threat_aura_snapshots,
            player_skill_records,
        }
    }
}

/// Short-lived access to canonical PlayerCondition inputs. The fixture view is
/// selected by the World caller; this capability never retains a Hub or session.
pub struct PlayerConditionAccessLikeCpp<'a> {
    core: &'a SessionCore,
    #[cfg(any(test, feature = "test-fixtures"))]
    fixtures: PlayerConditionFixtureRefsLikeCpp<'a>,
}

impl SessionCore {
    pub fn player_condition_access_with_selected_fixture_refs_like_cpp<'a>(
        &'a self,
        #[cfg(any(test, feature = "test-fixtures"))] fixtures: PlayerConditionFixtureRefsLikeCpp<
            'a,
        >,
    ) -> PlayerConditionAccessLikeCpp<'a> {
        PlayerConditionAccessLikeCpp {
            core: self,
            #[cfg(any(test, feature = "test-fixtures"))]
            fixtures,
        }
    }
}

impl PlayerConditionAccessLikeCpp<'_> {
    pub fn resolved_player_skill_records_like_cpp(
        &self,
    ) -> Option<HashMap<u16, RepresentedPlayerSkillLikeCpp>> {
        self.core
            .resolved_player_skill_records_for_publication_like_cpp(
                #[cfg(any(test, feature = "test-fixtures"))]
                self.fixtures.player_skill_records,
            )
    }

    pub fn resolved_player_aura_authority_complete_like_cpp(&self) -> Option<bool> {
        self.core
            .player_aura_subsystem_snapshot_with_fixture_refs_like_cpp(
                #[cfg(any(test, feature = "test-fixtures"))]
                self.fixtures.aura_authority_complete,
                #[cfg(any(test, feature = "test-fixtures"))]
                self.fixtures.spell_hit_aura_authority_tombstoned,
                #[cfg(any(test, feature = "test-fixtures"))]
                self.fixtures.visible_auras,
                #[cfg(any(test, feature = "test-fixtures"))]
                self.fixtures.canonical_threat_aura_snapshots,
            )
            .map(|auras| auras.persisted_player_aura_authority_complete_like_cpp())
    }

    pub fn player_handle_absent_like_cpp(&self) -> bool {
        self.core.player_handle_like_cpp.is_none()
    }

    /// Resolve the complete skill-record snapshot used by trainer admission.
    /// The test-only completeness bit is supplied by the selected caller; the
    /// fixture rows remain borrowed from this access's existing fixture view.
    pub fn complete_player_skill_records_like_cpp(
        &self,
        #[cfg(any(test, feature = "test-fixtures"))] fixture_complete: bool,
    ) -> Option<HashMap<u16, RepresentedPlayerSkillLikeCpp>> {
        #[cfg(any(test, feature = "test-fixtures"))]
        let records = self
            .core
            .resolved_player_skill_records_for_publication_like_cpp(
                self.fixtures.player_skill_records,
            )?;
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let records = self
            .core
            .resolved_player_skill_records_for_publication_like_cpp()?;

        let complete = self
            .core
            .with_owned_player_like_cpp(Player::skill_records_complete_like_cpp);
        #[cfg(any(test, feature = "test-fixtures"))]
        let complete = complete.or_else(|| {
            self.core
                .player_handle_like_cpp
                .is_none()
                .then_some(fixture_complete)
        });
        complete.unwrap_or(false).then_some(records)
    }

    pub fn known_spells_snapshot_like_cpp(&self) -> Option<Vec<i32>> {
        self.core.with_owned_player_like_cpp(|player| {
            player
                .spell_runtime_like_cpp()
                .known_spells_like_cpp()
                .to_vec()
        })
    }

    pub fn owned_inventory_access_like_cpp(
        &self,
    ) -> crate::session::OwnedInventoryAccessLikeCpp<'_> {
        self.core.owned_inventory_access_like_cpp()
    }

    pub fn inventory_valuation_access_like_cpp(
        &self,
    ) -> crate::session::InventoryValuationAccessLikeCpp<'_> {
        self.core.inventory_valuation_access_like_cpp()
    }

    pub fn owned_item_modifiers_access_like_cpp(
        &self,
    ) -> crate::session::OwnedItemModifiersAccessLikeCpp<'_> {
        self.core.owned_item_modifiers_access_like_cpp()
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_player_level_like_cpp(&self) -> &u8 {
        self.fixtures.level
    }

    pub fn owned_player_currency_access_like_cpp(
        &self,
    ) -> crate::session::OwnedPlayerCurrencyAccessLikeCpp<'_> {
        self.core.owned_player_currency_access_like_cpp()
    }

    pub fn player_registry_hydration_access_like_cpp(
        &self,
    ) -> crate::session::PlayerRegistryHydrationAccessLikeCpp<'_> {
        self.core.player_registry_hydration_access_like_cpp()
    }

    pub fn resolved_visible_auras_like_cpp(
        &self,
    ) -> Option<HashMap<u8, wow_entities::AuraApplicationLikeCpp>> {
        self.core
            .player_aura_subsystem_snapshot_with_fixture_refs_like_cpp(
                #[cfg(any(test, feature = "test-fixtures"))]
                self.fixtures.aura_authority_complete,
                #[cfg(any(test, feature = "test-fixtures"))]
                self.fixtures.spell_hit_aura_authority_tombstoned,
                #[cfg(any(test, feature = "test-fixtures"))]
                self.fixtures.visible_auras,
                #[cfg(any(test, feature = "test-fixtures"))]
                self.fixtures.canonical_threat_aura_snapshots,
            )
            .map(|auras| auras.runtime_applications_like_cpp().clone())
    }

    pub fn resolved_skill_values_like_cpp(&self) -> Option<HashMap<u16, u16>> {
        #[cfg(any(test, feature = "test-fixtures"))]
        let records = self
            .core
            .resolved_player_skill_records_for_publication_like_cpp(
                self.fixtures.player_skill_records,
            )?;
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let records = self
            .core
            .resolved_player_skill_records_for_publication_like_cpp()?;
        Some(crate::session::represented_skill_values_from_records_like_cpp(&records))
    }

    pub fn owned_player_quest_gameplay_snapshot_like_cpp(
        &self,
    ) -> Option<wow_entities::PlayerQuestGameplayState> {
        self.core
            .with_owned_player_like_cpp(|player| player.gameplay_state().quests.clone())
    }

    pub fn explored_zones_snapshot_like_cpp(
        &self,
        #[cfg(any(test, feature = "test-fixtures"))] consumer_test: bool,
        #[cfg(any(test, feature = "test-fixtures"))]
        fixture_blocks: &[u64; wow_entities::PLAYER_EXPLORED_ZONES_SIZE_LIKE_CPP],
    ) -> Option<[u64; wow_entities::PLAYER_EXPLORED_ZONES_SIZE_LIKE_CPP]> {
        let canonical = self
            .core
            .with_owned_player_like_cpp(|player| *player.explored_zones_blocks_like_cpp());
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && consumer_test && self.core.player_handle_like_cpp.is_none() {
            return Some(*fixture_blocks);
        }
        canonical
    }

    pub fn player_race_like_cpp(&self) -> u8 {
        #[cfg(any(test, feature = "test-fixtures"))]
        {
            self.core
                .player_race_with_fixture_like_cpp(self.fixtures.race)
        }
        #[cfg(not(any(test, feature = "test-fixtures")))]
        {
            self.core.player_race_with_fixture_like_cpp()
        }
    }

    pub fn player_class_like_cpp(&self) -> u8 {
        #[cfg(any(test, feature = "test-fixtures"))]
        {
            self.core
                .player_class_with_fixture_like_cpp(self.fixtures.class)
        }
        #[cfg(not(any(test, feature = "test-fixtures")))]
        {
            self.core.player_class_with_fixture_like_cpp()
        }
    }

    pub fn player_level_like_cpp(&self) -> u8 {
        #[cfg(any(test, feature = "test-fixtures"))]
        {
            self.core
                .player_level_with_fixture_like_cpp(self.fixtures.level)
        }
        #[cfg(not(any(test, feature = "test-fixtures")))]
        {
            self.core.player_level_with_fixture_like_cpp()
        }
    }

    pub fn player_gender_like_cpp(&self) -> u8 {
        #[cfg(any(test, feature = "test-fixtures"))]
        {
            self.core
                .player_gender_with_fixture_like_cpp(self.fixtures.gender)
        }
        #[cfg(not(any(test, feature = "test-fixtures")))]
        {
            self.core.player_gender_with_fixture_like_cpp()
        }
    }

    pub fn player_zone_area_like_cpp(&self) -> Option<(u32, u32)> {
        let canonical = self
            .core
            .with_owned_player_like_cpp(|player| player.gameplay_state().world_local);
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            return Some(
                PlayerWorldLocalState::from_represented_parts_like_cpp(
                    *self.fixtures.zone_id,
                    *self.fixtures.area_id,
                    *self.fixtures.zone_area_authority_complete,
                    *self.fixtures.pvp_hostile,
                    *self.fixtures.pvp_end_timer,
                    *self.fixtures.contested_pvp_timer,
                    *self.fixtures.is_outdoors,
                )
                .zone_area_like_cpp(),
            );
        }
        canonical.map(|state| state.zone_area_like_cpp())
    }

    pub fn primary_specialization_id_like_cpp(&self, consumer_test: bool) -> Option<u32> {
        let canonical = self
            .core
            .with_owned_player_like_cpp(wow_entities::Player::primary_specialization_id_like_cpp);
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && consumer_test && self.core.player_handle_like_cpp.is_none() {
            return Some(*self.fixtures.primary_specialization_id);
        }
        canonical
    }

    pub fn player_group_owner_access_like_cpp(&self) -> PlayerGroupOwnerAccessLikeCpp<'_> {
        self.core.player_group_owner_access_like_cpp()
    }

    pub fn expansion_like_cpp(&self) -> u8 {
        self.core.expansion
    }

    pub fn account_expansion_like_cpp(&self) -> u8 {
        self.core.account_expansion
    }

    pub fn is_game_master_like_cpp(&self) -> bool {
        self.core.security > 0
    }

    pub fn build_condition_player_object_like_cpp(&self) -> Option<WorldObject> {
        let mut player = WorldObject::new(
            false,
            TypeId::Player,
            TypeMask::OBJECT | TypeMask::UNIT | TypeMask::PLAYER,
        );
        player.object_mut().create(self.core.player_guid()?);
        let instance_id = self
            .core
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        let _ = player.set_map(u32::from(self.core.player_map_id_like_cpp()), instance_id);
        let (zone_id, area_id) = self.player_zone_area_like_cpp()?;
        player.set_zone_and_area(zone_id, area_id);
        if let Some(position) = self.core.player_position_with_fixture_like_cpp(
            #[cfg(any(test, feature = "test-fixtures"))]
            self.fixtures.position,
        ) {
            player.relocate(position);
        }
        Some(player)
    }

    pub fn condition_player_unit_snapshot_like_cpp(
        &self,
    ) -> Option<wow_conditions::ConditionUnitSnapshot> {
        let (health, max_health, is_alive) =
            self.core.resolved_player_vitals_with_fixture_like_cpp(
                #[cfg(any(test, feature = "test-fixtures"))]
                self.fixtures.health,
                #[cfg(any(test, feature = "test-fixtures"))]
                self.fixtures.max_health,
                #[cfg(any(test, feature = "test-fixtures"))]
                self.fixtures.alive,
            )?;
        Some(wow_conditions::ConditionUnitSnapshot {
            level: u32::from(self.player_level_like_cpp()),
            health: u64::from(health),
            max_health: u64::from(max_health),
            class_mask: crate::session::state::hub_support::player_class_mask(
                self.player_class_like_cpp(),
            ),
            race: self.player_race_like_cpp(),
            creature_type: None,
            is_alive,
            is_charmed: false,
            in_water: false,
            unit_state: 0,
            stand_state: UnitStandStateType::Stand as u32,
        })
    }

    pub fn condition_player_snapshot_like_cpp(&self) -> wow_conditions::ConditionPlayerSnapshot {
        wow_conditions::ConditionPlayerSnapshot {
            team: crate::session::state::hub_support::player_team_for_race_cpp(
                self.player_race_like_cpp(),
            ) as u32,
            native_gender: u32::from(self.player_gender_like_cpp()),
            drunken_state: 0,
            can_be_game_master: false,
            is_game_master: false,
            pet_type: None,
            // An unresolved generation-checked Player cannot prove the negative.
            is_in_flight: self.resolved_is_in_taxi_flight_like_cpp().unwrap_or(true),
        }
    }

    fn resolved_is_in_taxi_flight_like_cpp(&self) -> Option<bool> {
        let canonical = self
            .core
            .with_owned_player_like_cpp(|player| player.taxi_state_like_cpp().clone());
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            return Some(
                PlayerTaxiState::from_represented_parts_like_cpp(
                    self.fixtures.taxi_destinations.clone(),
                    self.fixtures.taxi_flight_state.map(
                        crate::session::movement_protocol::canonical_taxi_flight_state_like_cpp,
                    ),
                    self.fixtures.taxi_unit_flags.bits(),
                    *self.fixtures.taxi_mounted,
                )
                .is_in_flight_like_cpp(),
            );
        }
        canonical.map(|taxi| taxi.is_in_flight_like_cpp())
    }
}
