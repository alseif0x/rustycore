// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use super::{PlayerAuraRemovalAccessLikeCpp, SessionCore};
use wow_entities::Vehicle;
mod speed;

// The Rust sender consumes a sequence before its fresh scale-duration lookup.
// Failure of that later lookup leaves the consumed sequence unchanged.
fn collision_sequence_then_scale_duration_like_cpp<T>(
    state: &mut T,
    sequence: impl FnOnce(&mut T) -> Option<u32>,
    scale_duration: impl FnOnce(&T) -> Option<i32>,
) -> Option<(u32, i32)> {
    let sequence = sequence(state)?;
    let scale_duration = scale_duration(state)?;
    Some((sequence, scale_duration))
}

#[cfg(test)]
mod ordering_tests {
    use super::collision_sequence_then_scale_duration_like_cpp;
    use std::cell::RefCell;

    #[test]
    fn collision_sequence_is_consumed_before_missing_scale_duration() {
        struct State { counter: u32, reads: RefCell<Vec<&'static str>> }
        let mut state = State { counter: 17, reads: RefCell::new(Vec::new()) };
        let result = collision_sequence_then_scale_duration_like_cpp(
            &mut state,
            |state| {
                state.reads.borrow_mut().push("sequence");
                let previous = state.counter;
                state.counter = state.counter.wrapping_add(1);
                Some(previous)
            },
            |state| {
                state.reads.borrow_mut().push("scale_duration");
                None
            },
        );
        assert_eq!(result, None);
        assert_eq!(state.counter, 18);
        assert_eq!(*state.reads.borrow(), ["sequence", "scale_duration"]);
    }

    #[test]
    fn missing_sequence_owner_does_not_query_scale_duration() {
        let mut queried_scale = false;
        let result = collision_sequence_then_scale_duration_like_cpp(
            &mut (), |_| None,
            |_| { queried_scale = true; Some(0) },
        );
        assert_eq!(result, None);
        assert!(!queried_scale);
    }
}

#[cfg(any(test, feature = "test-fixtures"))]
pub struct AuraMountControlFixtureRefsLikeCpp<'a> {
    kit: &'a mut Option<Vehicle>,
    counter: &'a mut u32,
    collision_height: &'a mut f32,
    position: &'a Option<wow_core::Position>,
    flags: &'a mut wow_constants::movement::MovementFlag,
    movement_time: &'a u32,
    can_swim_to_fly: &'a mut bool,
    speed_rates: &'a mut [f32; crate::session::UnitMoveTypeLikeCpp::COUNT],
    forced_changes: &'a mut [u8; crate::session::UnitMoveTypeLikeCpp::COUNT],
    pet_guid: &'a Option<wow_core::ObjectGuid>,
    pet_speed_rates: &'a mut [f32; crate::session::UnitMoveTypeLikeCpp::COUNT],
    pet_speed_propagations: &'a mut u32,
    in_combat: &'a bool,
    fall_time: &'a mut u32,
    fall_z: &'a mut f32,
    scale_duration: &'a i32,
    race: &'a u8,
    gender: &'a u8,
    pet: super::pet_control::AuraDismountPetFixtureRefsLikeCpp<'a>,
}

#[cfg(any(test, feature = "test-fixtures"))]
impl<'a> AuraMountControlFixtureRefsLikeCpp<'a> {
    pub fn new(
        kit: &'a mut Option<Vehicle>, counter: &'a mut u32, collision_height: &'a mut f32,
        position: &'a Option<wow_core::Position>, flags: &'a mut wow_constants::movement::MovementFlag,
        movement_time: &'a u32, can_swim_to_fly: &'a mut bool, scale_duration: &'a i32,
        race: &'a u8, gender: &'a u8,
        pet: super::pet_control::AuraDismountPetFixtureRefsLikeCpp<'a>,
        speed_rates: &'a mut [f32; crate::session::UnitMoveTypeLikeCpp::COUNT],
        forced_changes: &'a mut [u8; crate::session::UnitMoveTypeLikeCpp::COUNT],
        pet_guid: &'a Option<wow_core::ObjectGuid>,
        pet_speed_rates: &'a mut [f32; crate::session::UnitMoveTypeLikeCpp::COUNT],
        pet_speed_propagations: &'a mut u32, in_combat: &'a bool,
        fall_time: &'a mut u32, fall_z: &'a mut f32,
    ) -> Self {
        Self { kit, counter, collision_height, position, flags, movement_time,
            can_swim_to_fly, scale_duration, race, gender, pet,
            speed_rates, forced_changes, pet_guid, pet_speed_rates,
            pet_speed_propagations, in_combat, fall_time, fall_z }
    }
}

/// Mount/control readers retain only selected immutable catalogs and borrowed
/// fixture values. Presentation is reborrowed from the aura role per phase.
pub struct AuraMountControlAccessLikeCpp<'a> {
    core: &'a SessionCore,
    display_store: Option<&'a wow_data::CreatureDisplayInfoStore>,
    model_store: Option<&'a wow_data::CreatureModelDataStore>,
    #[cfg(any(test, feature = "test-fixtures"))]
    fixtures: AuraMountControlFixtureRefsLikeCpp<'a>,
}

impl SessionCore {
    pub fn aura_mount_control_access_like_cpp<'a>(
        &'a self,
        display_store: Option<&'a wow_data::CreatureDisplayInfoStore>,
        model_store: Option<&'a wow_data::CreatureModelDataStore>,
        #[cfg(any(test, feature = "test-fixtures"))] fixtures: AuraMountControlFixtureRefsLikeCpp<'a>,
    ) -> AuraMountControlAccessLikeCpp<'a> {
        AuraMountControlAccessLikeCpp {
            core: self, display_store, model_store,
            #[cfg(any(test, feature = "test-fixtures"))]
            fixtures,
        }
    }
}

impl crate::session::HubMut<'_> {
    /// Select disjoint presentation and mount/control fields without reading
    /// the canonical Player or computing any movement value.
    pub fn aura_removal_mount_accesses_like_cpp(
        &mut self,
    ) -> (PlayerAuraRemovalAccessLikeCpp<'_>, AuraMountControlAccessLikeCpp<'_>) {
        let presentation = self.core.player_aura_removal_access_like_cpp(
            #[cfg(any(test, feature = "test-fixtures"))]
            super::AuraRemovalFixtureRefsLikeCpp::new(
                &mut self.fixtures.auras.player_aura_authority_complete_like_cpp,
                &mut self.fixtures.auras.player_spell_hit_aura_authority_tombstoned_like_cpp,
                &mut self.fixtures.auras.visible_auras,
                &mut self.fixtures.auras.canonical_threat_aura_snapshots_like_cpp,
                &mut self.fixtures.vehicles.player_mount_display_id_like_cpp,
                &mut self.fixtures.vehicles.player_mounted_like_cpp,
                &mut self.fixtures.presentation.player_unit_flags_like_cpp,
                &self.fixtures.presentation.player_object_scale_like_cpp,
            ),
        );
        let control = self.core.aura_mount_control_access_like_cpp(
            self.catalogs.creatures.display_info_store.as_deref(),
            self.catalogs.creatures.model_data_store.as_deref(),
            #[cfg(any(test, feature = "test-fixtures"))]
            AuraMountControlFixtureRefsLikeCpp::new(
                &mut self.fixtures.vehicles.player_mount_vehicle_kit_like_cpp,
                &mut self.fixtures.movement.movement_counter_like_cpp,
                &mut self.fixtures.movement.player_collision_height_like_cpp,
                &self.fixtures.movement.player_position,
                &mut self.fixtures.movement.player_movement_flags_like_cpp,
                &self.fixtures.movement.player_movement_time_like_cpp,
                &mut self.fixtures.movement.represented_can_swim_to_fly_transition_like_cpp,
                &self.fixtures.identity.player_scale_duration_like_cpp,
                &self.fixtures.identity.player_race,
                &self.fixtures.identity.player_gender,
                super::pet_control::AuraDismountPetFixtureRefsLikeCpp::new(
                    &self.fixtures.pets.represented_pet_guid_like_cpp,
                    &mut self.fixtures.pets.represented_pet_react_state_like_cpp,
                    &mut self.fixtures.pets.represented_pet_command_state_like_cpp,
                    &mut self.fixtures.pets.represented_pet_stable_like_cpp,
                    &mut self.fixtures.pets.represented_character_pet_rows_empty_authority_complete_like_cpp,
                    &mut self.fixtures.pets.represented_temporary_unsummoned_pet_number_like_cpp,
                    &mut self.fixtures.pets.represented_old_pet_spell_like_cpp,
                    &mut self.fixtures.pets.temporary_mount_pet_react_state_like_cpp,
                ),
                &mut self.fixtures.movement.movement_speed_rates_like_cpp,
                &mut self.fixtures.movement.forced_speed_changes_like_cpp,
                &self.fixtures.pets.represented_pet_guid_like_cpp,
                &mut self.fixtures.pets.represented_pet_movement_speed_rates_like_cpp,
                &mut self.fixtures.pets.represented_pet_speed_propagations_like_cpp,
                &self.fixtures.combat.in_combat,
                &mut self.fixtures.movement.last_fall_time_like_cpp,
                &mut self.fixtures.movement.last_fall_z_like_cpp,
            ),
        );
        (presentation, control)
    }
}

impl AuraMountControlAccessLikeCpp<'_> {
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn mount_kit_fixture_for_hydration_like_cpp(&self) -> &Option<Vehicle> {
        &*self.fixtures.kit
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn pet_guid_fixture_for_hydration_like_cpp(&self) -> &Option<wow_core::ObjectGuid> {
        self.fixtures.pet_guid
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn position_fixture_for_scaling_like_cpp(&self) -> &Option<wow_core::Position> {
        self.fixtures.position
    }

    pub fn mount_vehicle_kit_snapshot_like_cpp(&self) -> Option<Option<Vehicle>> {
        self.core.mount_vehicle_kit_snapshot_with_fixture_like_cpp(
            #[cfg(any(test, feature = "test-fixtures"))]
            self.fixtures.kit,
        )
    }

    pub fn remove_mount_vehicle_kit_like_cpp(&mut self) -> bool {
        self.core.remove_mount_vehicle_kit_with_fixture_like_cpp(
            #[cfg(any(test, feature = "test-fixtures"))]
            self.fixtures.kit,
        )
    }

    fn next_movement_counter_like_cpp(&mut self) -> Option<u32> {
        let canonical = self.core.with_owned_player_mut_like_cpp(|player| {
            player.unit_mut().next_movement_counter_like_cpp()
        });
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            let sequence_index = *self.fixtures.counter;
            *self.fixtures.counter = self.fixtures.counter.wrapping_add(1);
            return Some(sequence_index);
        }
        canonical
    }

    pub fn send_set_vehicle_rec_id_like_cpp(&mut self, vehicle_id: u32) {
        let Some(player_guid) = self.core.player_guid() else { return; };
        let vehicle_rec_id = i32::try_from(vehicle_id).unwrap_or(i32::MAX);
        let Some(sequence_index) = self.next_movement_counter_like_cpp() else { return; };
        self.core.send_packet(&wow_packet::packets::vehicle::MoveSetVehicleRecId {
            mover_guid: player_guid, sequence_index, vehicle_rec_id,
        });
        self.core.send_packet(&wow_packet::packets::vehicle::SetVehicleRecId {
            vehicle_guid: player_guid, vehicle_rec_id,
        });
    }

    pub fn enable_pet_controls_on_dismount_like_cpp(&mut self) {
        self.core.enable_pet_controls_on_dismount_with_fixture_refs_like_cpp(
            #[cfg(any(test, feature = "test-fixtures"))]
            &mut self.fixtures.pet,
        );
    }

    pub fn update_player_collision_height_like_cpp(
        &mut self, presentation: &PlayerAuraRemovalAccessLikeCpp<'_>, consumer_test: bool,
    ) {
        let Some((_, mount_display_id, object_scale)) = presentation.player_unit_presentation_snapshot_like_cpp() else { return; };
        let computed_height = if let (Some(display_store), Some(model_store)) = (self.display_store, self.model_store) {
            let native_display_id = crate::session::default_display_id(
                self.core.player_race_with_fixture_like_cpp(
                    #[cfg(any(test, feature = "test-fixtures"))]
                    self.fixtures.race,
                ),
                self.core.player_gender_with_fixture_like_cpp(
                    #[cfg(any(test, feature = "test-fixtures"))]
                    self.fixtures.gender,
                ),
            );
            let mount_display_id = u32::try_from(mount_display_id).ok().filter(|id| *id != 0);
            wow_data::unit_collision_height_like_cpp(
                object_scale, native_display_id, mount_display_id, display_store, model_store,
            )
        } else { None };
        let mount_display_id = u32::try_from(mount_display_id).unwrap_or(0);
        let _canonical_height = self.core.with_owned_player_mut_like_cpp(|player| {
            let unit = player.unit_mut();
            unit.set_mount_display_id(mount_display_id);
            if let Some(height) = computed_height { unit.set_collision_height_like_cpp(height); }
            unit.collision_height_like_cpp()
        });
        #[cfg(any(test, feature = "test-fixtures"))]
        if consumer_test {
            if let Some(height) = _canonical_height.or(computed_height)
                && (_canonical_height.is_some() || self.core.player_handle_like_cpp.is_none())
            {
                *self.fixtures.collision_height = height;
            }
        }
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let _ = consumer_test;
    }

    pub fn send_movement_set_collision_height_like_cpp(
        &mut self, presentation: &PlayerAuraRemovalAccessLikeCpp<'_>, reason: u8,
    ) {
        let Some(player_guid) = self.core.player_guid() else { return; };
        let Some((_, mount_display_id, object_scale)) = presentation.player_unit_presentation_snapshot_like_cpp() else { return; };
        let Some(collision_height) = self.core.with_owned_player_like_cpp(|player| player.unit().collision_height_like_cpp())
            .or_else(|| {
                #[cfg(any(test, feature = "test-fixtures"))]
                { return self.core.player_handle_like_cpp.is_none().then_some(*self.fixtures.collision_height); }
                #[cfg(not(any(test, feature = "test-fixtures")))]
                { None }
            }) else { return; };
        let Some((sequence_index, scale_duration)) = collision_sequence_then_scale_duration_like_cpp(
            self, Self::next_movement_counter_like_cpp,
            |access| {
                let scale_duration = access.core.with_owned_player_like_cpp(|player| player.gameplay_state().movement_control.scale_duration);
                #[cfg(any(test, feature = "test-fixtures"))]
                let scale_duration = if scale_duration.is_none() && access.core.player_handle_like_cpp.is_none() {
                    Some(*access.fixtures.scale_duration)
                } else { scale_duration };
                scale_duration
            },
        ) else { return; };
        self.core.send_packet(&wow_packet::packets::movement::MoveSetCollisionHeight {
            mover_guid: player_guid, sequence_index, height: collision_height, scale: object_scale,
            reason, mount_display_id: u32::try_from(mount_display_id).unwrap_or(0), scale_duration,
        });
        use wow_packet::ServerPacket;
        let Some(status) = self.current_player_movement_info_like_cpp(player_guid) else { return; };
        self.core.packet_publication_access_like_cpp().broadcast_to_movement_set_in_range_and_connection_like_cpp(
            wow_packet::packets::movement::MoveUpdateCollisionHeight { status, height: collision_height, scale: object_scale }.to_bytes(),
            crate::map_manager::VISIBILITY_RADIUS, false,
            #[cfg(any(test, feature = "test-fixtures"))]
            self.fixtures.position,
        );
    }

    fn current_player_movement_info_like_cpp(&self, player_guid: wow_core::ObjectGuid) -> Option<wow_packet::packets::movement::MovementInfo> {
        Some(wow_packet::packets::movement::MovementInfo {
            guid: player_guid,
            position: self.core.player_position_with_fixture_like_cpp(
                #[cfg(any(test, feature = "test-fixtures"))]
                self.fixtures.position,
            )?,
            flags: self.resolved_player_movement_flags_like_cpp()?,
            flags2: if self.resolved_can_swim_to_fly_transition_like_cpp()? {
                wow_constants::movement::MovementFlag2::CAN_SWIM_TO_FLY_TRANS
            } else { wow_constants::movement::MovementFlag2::NONE },
            time: self.resolved_player_movement_time_like_cpp()?,
            ..wow_packet::packets::movement::MovementInfo::default()
        })
    }

    fn resolved_player_movement_flags_like_cpp(&self) -> Option<wow_constants::movement::MovementFlag> {
        let canonical = self.core.with_owned_player_like_cpp(|player| player.unit().movement_flags_like_cpp());
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() { return Some(*self.fixtures.flags); }
        canonical
    }

    fn resolved_can_swim_to_fly_transition_like_cpp(&self) -> Option<bool> {
        let canonical = self.core.with_owned_player_like_cpp(|player| player.gameplay_state().movement_control.can_swim_to_fly_transition);
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() { return Some(*self.fixtures.can_swim_to_fly); }
        canonical
    }

    fn resolved_player_movement_time_like_cpp(&self) -> Option<u32> {
        let canonical = self.core.with_owned_player_like_cpp(|player| player.unit().movement_time_like_cpp());
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() { return Some(*self.fixtures.movement_time); }
        canonical
    }

    pub fn send_represented_mount_unit_update_like_cpp(&self, presentation: &PlayerAuraRemovalAccessLikeCpp<'_>, display_id: i32) {
        let Some(player_guid) = self.core.player_guid() else { return; };
        let Some((unit_flags, _, _)) = presentation.player_unit_presentation_snapshot_like_cpp() else { return; };
        use wow_packet::packets::update::{UnitDataValuesDeltaUpdate, UpdateObject};
        let mut data = UnitDataValuesDeltaUpdate::default();
        data.unit_data_mask[1] |= 1 << (41 - 32);
        data.unit_data_mask[1] |= 1 << (51 - 32);
        data.flags = unit_flags.bits();
        data.mount_display_id = display_id;
        self.core.send_packet(&UpdateObject::unit_values_update(player_guid, self.core.player_map_id_like_cpp(), data));
    }
}

impl SessionCore {
    pub fn mount_vehicle_kit_snapshot_with_fixture_like_cpp(
        &self,
        #[cfg(any(test, feature = "test-fixtures"))] fixture: &Option<Vehicle>,
    ) -> Option<Option<Vehicle>> {
        let canonical = self.with_owned_player_like_cpp(|player| player.mount_vehicle_kit_snapshot_like_cpp());
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(fixture.clone());
        }
        canonical
    }

    pub fn remove_mount_vehicle_kit_with_fixture_like_cpp(
        &self,
        #[cfg(any(test, feature = "test-fixtures"))] fixture: &mut Option<Vehicle>,
    ) -> bool {
        let canonical = self.with_owned_player_mut_like_cpp(|player| player.remove_mount_vehicle_kit_like_cpp()).is_some();
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical {
            return true;
        }
        #[cfg(any(test, feature = "test-fixtures"))]
        if self.player_handle_like_cpp.is_none() {
            if let Some(vehicle_kit) = fixture.as_mut() {
                vehicle_kit.uninstall();
            }
            *fixture = None;
            return true;
        }
        canonical
    }
}

impl PlayerAuraRemovalAccessLikeCpp<'_> {
    pub fn mount_vehicle_kit_snapshot_like_cpp(
        &self,
        #[cfg(any(test, feature = "test-fixtures"))] fixture: &Option<Vehicle>,
    ) -> Option<Option<Vehicle>> {
        self.core.mount_vehicle_kit_snapshot_with_fixture_like_cpp(
            #[cfg(any(test, feature = "test-fixtures"))]
            fixture,
        )
    }

    pub fn remove_mount_vehicle_kit_like_cpp(
        &self,
        #[cfg(any(test, feature = "test-fixtures"))] fixture: &mut Option<Vehicle>,
    ) -> bool {
        self.core.remove_mount_vehicle_kit_with_fixture_like_cpp(
            #[cfg(any(test, feature = "test-fixtures"))]
            fixture,
        )
    }
}
