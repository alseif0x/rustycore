use std::sync::Arc;
use tracing::warn;

use crate::session::state::SessionCore;
#[cfg(any(test, feature = "test-fixtures"))]
use crate::session::{
    RepresentedVehicleBaseMovementLikeCpp, RepresentedVehicleDismissMovementLikeCpp,
};
use wow_constants::MovementFlag;
use wow_core::{ObjectGuid, Position};

impl SessionCore {
    pub fn sync_canonical_player_position_if_same_or_detached_like_cpp(
        &mut self,
        map_id: u16,
        position: Position,
    ) {
        let (Some(manager), Some(handle)) = (
            self.canonical_map_manager.as_ref().map(Arc::clone),
            self.player_handle_like_cpp,
        ) else {
            return;
        };
        let Ok(mut manager) = manager.lock() else {
            return;
        };
        match manager.player_residence_like_cpp(handle) {
            Some(wow_map::PlayerResidenceLikeCpp::Detached) => {
                let _ = manager.with_player_mut_like_cpp(handle, |player| {
                    player.unit_mut().world_mut().relocate(position);
                });
            }
            Some(wow_map::PlayerResidenceLikeCpp::Active(key))
                if key.map_id == u32::from(map_id) =>
            {
                let _ = manager.relocate_player_like_cpp(handle, position);
            }
            Some(wow_map::PlayerResidenceLikeCpp::Active(_)) | None => {}
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MovementTransportMembershipLikeCpp {
    Detached,
    Attached(ObjectGuid),
}

#[cfg(any(test, feature = "test-fixtures"))]
impl crate::session::state::MovementState {
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_vehicle_dismiss_movements_like_cpp(
        &self,
    ) -> &[RepresentedVehicleDismissMovementLikeCpp] {
        &self.represented_vehicle_dismiss_movements_like_cpp
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_vehicle_base_movements_like_cpp(
        &self,
    ) -> &[RepresentedVehicleBaseMovementLikeCpp] {
        &self.represented_vehicle_base_movements_like_cpp
    }
}

impl crate::session::HubRef<'_> {
    pub fn adjust_client_movement_time_like_cpp(&self, time: u32) -> u32 {
        let movement_time = i64::from(time) + self.core.driver.time_synchronization.clock_delta;
        if self.core.driver.time_synchronization.clock_delta == 0
            || !(0..=i64::from(u32::MAX)).contains(&movement_time)
        {
            warn!(
                account = self.core.account_id,
                client_time = time,
                clock_delta = self.core.driver.time_synchronization.clock_delta,
                "The computed movement time using clockDelta is erroneous. Using fallback instead"
            );
            crate::session::game_time_ms_like_cpp()
        } else {
            movement_time as u32
        }
    }

    pub fn remove_all_dynamic_objects_for_current_player_like_cpp(
        &self,
    ) -> Option<wow_map::map::RemoveAllDynamicObjectsForCasterOutcomeLikeCpp> {
        let player_guid = self.core.player_guid()?;
        let map_key = self.core.current_canonical_player_map_key_like_cpp()?;
        let canonical = self.core.canonical_map_manager.as_ref()?;
        let mut manager = canonical.lock().ok()?;
        let managed = manager.find_map_mut(map_key.map_id, map_key.instance_id)?;
        Some(
            managed
                .map_mut()
                .remove_all_dynamic_objects_for_caster_like_cpp(player_guid),
        )
    }

    pub fn remove_all_area_triggers_for_current_player_like_cpp(
        &self,
    ) -> Option<wow_map::map::RemoveAllAreaTriggersForCasterOutcomeLikeCpp> {
        let player_guid = self.core.player_guid()?;
        let map_key = self.core.current_canonical_player_map_key_like_cpp()?;
        let canonical = self.core.canonical_map_manager.as_ref()?;
        let mut manager = canonical.lock().ok()?;
        let managed = manager.find_map_mut(map_key.map_id, map_key.instance_id)?;
        Some(
            managed
                .map_mut()
                .remove_all_area_triggers_for_caster_like_cpp(player_guid),
        )
    }

    /// Current `Unit::m_movementCounter` value (read without advancing). C++ reads this for
    /// `SMSG_RESUME_TOKEN.SequenceIndex` on far teleport (MovementHandler.cpp:109), before
    /// `SendInitialPacketsBeforeAddToMap` resets it.
    pub fn movement_counter_like_cpp(&self) -> Option<u32> {
        let canonical = self
            .core
            .with_owned_player_like_cpp(|player| player.unit().movement_counter_like_cpp());
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            return Some(self.fixtures.movement.movement_counter_like_cpp);
        }
        canonical
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn player_movement_flags_like_cpp(&self) -> MovementFlag {
        self.resolved_player_movement_flags_like_cpp()
            .expect("test Player movement owner must resolve")
    }

    pub fn resolved_mover_fixed_position_vehicle_like_cpp(&self) -> Option<bool> {
        let canonical = self.core.with_owned_player_like_cpp(|player| {
            player
                .gameplay_state()
                .movement_control
                .mover_fixed_position_vehicle
        });
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            return Some(
                self.fixtures
                    .movement
                    .represented_mover_fixed_position_vehicle_like_cpp,
            );
        }
        canonical
    }

    /// Resolve the active mover's `MoveSpline::Finalized()` admission state.
    ///
    /// `WorldSession::HandleMovementOpcode` rejects a packet while the mover's
    /// spline is still active (`MovementHandler.cpp:305-335`). Player motion
    /// belongs to the canonical Player; creature/pet motion is still executed
    /// by the legacy map runtime, whose `movement_finished` method is the
    /// corresponding `movespline` owner. The canonical creature projection is
    /// only a fallback for fixtures that do not install the legacy runtime.
    pub fn mover_spline_finalized_like_cpp(&self, mover_guid: ObjectGuid) -> Option<bool> {
        if self.core.player_guid() == Some(mover_guid) {
            let canonical = self.core.with_owned_player_like_cpp(|player| {
                player.unit().subsystems().motion.spline.finalized
            });
            #[cfg(any(test, feature = "test-fixtures"))]
            if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
                // Handle-less movement fixtures have no materialized Player;
                // C++'s freshly constructed MoveSpline is finalized.
                return Some(true);
            }
            return canonical;
        }

        let (map_id, instance_id) = self.core.current_legacy_runtime_map_key_like_cpp();
        if let Some(manager) = self.core.map_manager.as_ref().cloned() {
            let manager = manager
                .read()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if let Some(creature) = manager.find_creature(map_id, instance_id, mover_guid) {
                return Some(creature.movement_finished());
            }
        }

        let key = self.core.current_canonical_player_map_key_like_cpp()?;
        let manager = self.core.canonical_map_manager.as_ref()?.lock().ok()?;
        manager
            .find_map(key.map_id, key.instance_id)
            .and_then(|managed| {
                managed
                    .map()
                    .with_creature_or_pet_like_cpp(mover_guid, |creature, _| {
                        creature.unit().subsystems().motion.spline.finalized
                    })
            })
    }

    /// Resolve the active mover's current world position for MovementHandler's
    /// stale transport-packet guard (`MovementHandler.cpp:345-350`). C++
    /// applies the grid-size comparison to every `Unit*`, including a
    /// controlled creature or pet; keep the legacy map runtime as the first
    /// authority and the canonical map projection as its bounded fallback.
    pub fn mover_position_like_cpp(&self, mover_guid: ObjectGuid) -> Option<wow_core::Position> {
        if self.core.player_guid() == Some(mover_guid) {
            return self.player_position_like_cpp();
        }

        let (map_id, instance_id) = self.core.current_legacy_runtime_map_key_like_cpp();
        if let Some(manager) = self.core.map_manager.as_ref().cloned() {
            let manager = manager
                .read()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if let Some(creature) = manager.find_creature(map_id, instance_id, mover_guid) {
                return Some(creature.position());
            }
        }

        let key = self.core.current_canonical_player_map_key_like_cpp()?;
        let manager = self.core.canonical_map_manager.as_ref()?.lock().ok()?;
        manager
            .find_map(key.map_id, key.instance_id)
            .and_then(|managed| {
                managed
                    .map()
                    .with_creature_or_pet_like_cpp(mover_guid, |creature, _| {
                        creature.unit().world().position()
                    })
            })
    }

    /// Resolve the movement-force magnitude from the active Unit. C++ reads
    /// `mover->GetMovementForces()->GetModMagnitude()` in
    /// `HandleMoveSetModMovementForceMagnitudeAck` (`MovementHandler.cpp:638-650)`;
    /// a controlled Creature/Pet therefore cannot borrow the Player's value.
    pub fn mover_movement_force_mod_magnitude_like_cpp(
        &self,
        mover_guid: ObjectGuid,
    ) -> Option<f32> {
        if self.core.player_guid() == Some(mover_guid) {
            let canonical = self.core.with_owned_player_like_cpp(|player| {
                player.unit().movement_force_mod_magnitude_like_cpp()
            });
            #[cfg(any(test, feature = "test-fixtures"))]
            if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
                return Some(self.fixtures.movement.movement_force_mod_magnitude_like_cpp);
            }
            return canonical;
        }

        let (map_id, instance_id) = self.core.current_legacy_runtime_map_key_like_cpp();
        if let Some(manager) = self.core.map_manager.as_ref().cloned() {
            let manager = manager
                .read()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if let Some(creature) = manager.find_creature(map_id, instance_id, mover_guid) {
                return Some(
                    creature
                        .creature
                        .unit()
                        .movement_force_mod_magnitude_like_cpp(),
                );
            }
        }

        let key = self.core.current_canonical_player_map_key_like_cpp()?;
        let manager = self.core.canonical_map_manager.as_ref()?.lock().ok()?;
        manager
            .find_map(key.map_id, key.instance_id)
            .and_then(|managed| {
                managed
                    .map()
                    .with_creature_or_pet_like_cpp(mover_guid, |creature, _| {
                        creature.unit().movement_force_mod_magnitude_like_cpp()
                    })
            })
    }

    /// Reconcile the canonical map transport passenger set with the movement
    /// packet's requested transport. This is the C++ `AddPassenger`/
    /// `RemovePassenger` branch in `MovementHandler.cpp:361-390`; the map owns
    /// both the transport object and its passenger membership.
    pub fn reconcile_player_transport_membership_like_cpp(
        &self,
        player_guid: ObjectGuid,
        requested_transport_guid: Option<ObjectGuid>,
    ) -> MovementTransportMembershipLikeCpp {
        let Some(key) = self.core.current_canonical_player_map_key_like_cpp() else {
            #[cfg(any(test, feature = "test-fixtures"))]
            return requested_transport_guid
                .filter(|guid| !guid.is_empty())
                .map_or(MovementTransportMembershipLikeCpp::Detached, |guid| {
                    MovementTransportMembershipLikeCpp::Attached(guid)
                });
            #[cfg(not(any(test, feature = "test-fixtures")))]
            return MovementTransportMembershipLikeCpp::Detached;
        };
        let Some(manager) = self.core.canonical_map_manager.as_ref().cloned() else {
            return MovementTransportMembershipLikeCpp::Detached;
        };
        let Ok(mut manager) = manager.lock() else {
            return MovementTransportMembershipLikeCpp::Detached;
        };
        let Some(managed) = manager.find_map_mut(key.map_id, key.instance_id) else {
            return MovementTransportMembershipLikeCpp::Detached;
        };
        let map = managed.map_mut();
        let current_transport_guid = map
            .get_typed_transport_for_passenger_like_cpp(player_guid)
            .map(|transport| transport.world().guid());
        let requested_transport_guid = requested_transport_guid.filter(|guid| !guid.is_empty());

        if current_transport_guid == requested_transport_guid {
            return requested_transport_guid.map_or(
                MovementTransportMembershipLikeCpp::Detached,
                MovementTransportMembershipLikeCpp::Attached,
            );
        }

        if let Some(current_transport_guid) = current_transport_guid {
            if let Some(transport) = map.get_typed_transport_mut_like_cpp(current_transport_guid) {
                transport.remove_passenger(player_guid);
            }
        }

        let Some(requested_transport_guid) = requested_transport_guid else {
            return MovementTransportMembershipLikeCpp::Detached;
        };

        let Some(transport) = map.get_typed_transport_mut_like_cpp(requested_transport_guid) else {
            return MovementTransportMembershipLikeCpp::Detached;
        };
        if transport.add_passenger(player_guid)
            || transport.passengers().contains(&player_guid)
            || transport.static_passengers().contains(&player_guid)
        {
            MovementTransportMembershipLikeCpp::Attached(requested_transport_guid)
        } else {
            MovementTransportMembershipLikeCpp::Detached
        }
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn player_movement_time_like_cpp(&self) -> u32 {
        self.resolved_player_movement_time_like_cpp()
            .expect("test Player movement-time owner must resolve")
    }

    pub fn resolved_movement_force_mod_magnitude_changes_like_cpp(&self) -> Option<u8> {
        let canonical = self.core.with_owned_player_like_cpp(|player| {
            player.movement_force_mod_magnitude_changes_like_cpp()
        });
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            return Some(
                self.fixtures
                    .movement
                    .movement_force_mod_magnitude_changes_like_cpp,
            );
        }
        canonical
    }

    pub fn player_transport_position_like_cpp(&self) -> Option<Position> {
        self.player_transport_state_like_cpp()
            .flatten()
            .map(|state| Position::new(state.x, state.y, state.z, state.orientation))
    }
}

impl crate::session::HubMut<'_> {
    pub fn remove_current_player_from_canonical_current_map_like_cpp(&mut self) -> bool {
        let Some(guid) = self.core.player_guid() else {
            return false;
        };
        let Some(manager) = self.core.canonical_map_manager.as_ref().map(Arc::clone) else {
            return false;
        };
        let Ok(mut manager) = manager.lock() else {
            return false;
        };
        let handle = match self.core.player_handle_like_cpp {
            Some(handle) if handle.guid() == guid => handle,
            Some(_) => return false,
            None => {
                // Adopt a unique legacy record before removing it. Discarding
                // RemoveFromMap's returned Box would destroy the live Player.
                let Ok(handle) = manager.adopt_active_player_like_cpp(guid) else {
                    return false;
                };
                self.core.player_handle_like_cpp = Some(handle);
                handle
            }
        };
        match manager.player_residence_like_cpp(handle) {
            Some(wow_map::PlayerResidenceLikeCpp::Active(_)) => {
                manager.detach_player_like_cpp(handle).is_ok()
            }
            Some(wow_map::PlayerResidenceLikeCpp::Detached) => true,
            None => false,
        }
    }

    pub fn remove_account_toy_like_cpp(&mut self, item_id: u32) -> bool {
        self.mutate_player_collection_state_like_cpp(|state| state.remove_toy_like_cpp(item_id))
            .unwrap_or(false)
    }

    pub(crate) fn clear_player_emote_state_on_movement_like_cpp(
        &mut self,
    ) -> Option<wow_packet::packets::update::UpdateObject> {
        if self.shared().resolved_player_emote_state_like_cpp() == Some(0) {
            return None;
        }

        self.set_player_emote_state_like_cpp(0)
    }

    /// Remove a GUID from the legit characters list.
    pub fn remove_legit_character(&mut self, guid: &ObjectGuid) {
        self.core
            .account_state
            .legit_characters
            .retain(|g| g != guid);
    }

    pub fn set_player_position_like_cpp(&mut self, position: wow_core::Position) {
        self.set_player_map_position_like_cpp(self.core.current_map_id, position);
    }

    /// Apply the server-side facing update used by C++ `Unit::SetOrientation`
    /// for a vehicle passenger. This intentionally leaves map coordinates and
    /// cell ownership untouched; the vehicle remains authoritative for them.
    pub fn set_player_orientation_like_cpp(&mut self, orientation: f32) -> bool {
        let Some(mut position) = self.shared().player_position_like_cpp() else {
            return false;
        };
        if position.orientation == orientation {
            return false;
        }
        position.orientation = orientation;
        let canonical = self
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player.unit_mut().world_mut().relocate(position);
            })
            .is_some();
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical || self.core.player_handle_like_cpp.is_none() {
            self.fixtures.movement.player_position = Some(position);
        }
        canonical
            || cfg!(any(test, feature = "test-fixtures"))
                && self.core.player_handle_like_cpp.is_none()
    }

    pub fn set_player_movement_time_like_cpp(&mut self, time: u32) {
        let _canonical = self
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player.unit_mut().set_movement_time_like_cpp(time);
            })
            .is_some();
        #[cfg(any(test, feature = "test-fixtures"))]
        if _canonical || self.core.player_handle_like_cpp.is_none() {
            self.fixtures.movement.player_movement_time_like_cpp = time;
        }
    }

    pub fn set_player_movement_flags_like_cpp(&mut self, flags: MovementFlag) {
        let _canonical = self
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player.unit_mut().set_movement_flags_like_cpp(flags);
            })
            .is_some();
        #[cfg(any(test, feature = "test-fixtures"))]
        if _canonical || self.core.player_handle_like_cpp.is_none() {
            self.fixtures.movement.player_movement_flags_like_cpp = flags;
        }
    }

    pub fn consume_movement_force_mod_magnitude_change_like_cpp(&mut self) -> Option<u8> {
        let canonical = self.core.with_owned_player_mut_like_cpp(|player| {
            player.consume_movement_force_mod_magnitude_change_like_cpp()
        });
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            if self
                .fixtures
                .movement
                .movement_force_mod_magnitude_changes_like_cpp
                > 0
            {
                self.fixtures
                    .movement
                    .movement_force_mod_magnitude_changes_like_cpp = self
                    .fixtures
                    .movement
                    .movement_force_mod_magnitude_changes_like_cpp
                    .saturating_sub(1);
            }
            return Some(
                self.fixtures
                    .movement
                    .movement_force_mod_magnitude_changes_like_cpp,
            );
        }
        canonical
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_movement_force_mod_magnitude_changes_like_cpp(&mut self, count: u8) {
        let canonical = self
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player.set_movement_force_mod_magnitude_changes_like_cpp(count);
            })
            .is_some();
        if canonical || self.core.player_handle_like_cpp.is_none() {
            self.fixtures
                .movement
                .movement_force_mod_magnitude_changes_like_cpp = count;
        }
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_movement_force_mod_magnitude_like_cpp(&mut self, magnitude: f32) {
        let canonical = self
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player
                    .unit_mut()
                    .set_movement_force_mod_magnitude_like_cpp(magnitude);
            })
            .is_some();
        if canonical || self.core.player_handle_like_cpp.is_none() {
            self.fixtures.movement.movement_force_mod_magnitude_like_cpp = magnitude;
        }
    }
}

impl crate::session::HubMut<'_> {
    pub fn set_player_map_position_like_cpp(&mut self, map_id: u16, position: wow_core::Position) {
        if self.core.player_map_id_like_cpp() != map_id {
            self.core
                .invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        }
        self.core.current_map_id = map_id;
        self.core
            .sync_canonical_player_position_if_same_or_detached_like_cpp(map_id, position);
        #[cfg(any(test, feature = "test-fixtures"))]
        if self.core.player_handle_like_cpp.is_none()
            || self.shared().player_position_like_cpp() == Some(position)
        {
            self.fixtures.movement.player_position = Some(position);
        }
    }

    /// C++ `Unit::m_movementCounter` post-increment: returns the current value and advances
    /// it. Used as the SequenceIndex of movement-control packets (vehicle-rec, collision,
    /// near-teleport, speed/flag) and read for `SMSG_RESUME_TOKEN` on far teleport.
    pub fn next_movement_counter_like_cpp(&mut self) -> Option<u32> {
        let canonical = self.core.with_owned_player_mut_like_cpp(|player| {
            player.unit_mut().next_movement_counter_like_cpp()
        });
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            let sequence_index = self.fixtures.movement.movement_counter_like_cpp;
            self.fixtures.movement.movement_counter_like_cpp = self
                .fixtures
                .movement
                .movement_counter_like_cpp
                .wrapping_add(1);
            return Some(sequence_index);
        }
        canonical
    }

    /// C++ `Player::SendInitialPacketsBeforeAddToMap` resets `m_movementCounter` to 0 for a
    /// non-seamless add (login / far teleport). Player.cpp:23483.
    pub fn reset_movement_counter_like_cpp(&mut self) -> bool {
        let canonical = self
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player.unit_mut().reset_movement_counter_like_cpp();
            })
            .is_some();
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical || self.core.player_handle_like_cpp.is_none() {
            self.fixtures.movement.movement_counter_like_cpp = 0;
        }
        canonical
            || cfg!(any(test, feature = "test-fixtures"))
                && self.core.player_handle_like_cpp.is_none()
    }

    pub fn set_player_moved_unit_guid_like_cpp(&mut self, guid: ObjectGuid) {
        #[cfg_attr(not(any(test, feature = "test-fixtures")), allow(unused_variables))]
        let canonical = self
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player
                    .unit_mut()
                    .subsystems_mut()
                    .control
                    .set_moved_unit((!guid.is_empty()).then_some(guid));
            })
            .is_some();
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical || self.core.player_handle_like_cpp.is_none() {
            self.fixtures.movement.player_moved_unit_guid_like_cpp = guid;
        }
    }
}

impl crate::session::HubRef<'_> {
    pub fn current_player_movement_info_like_cpp(
        &self,
        player_guid: ObjectGuid,
    ) -> Option<wow_packet::packets::movement::MovementInfo> {
        Some(wow_packet::packets::movement::MovementInfo {
            guid: player_guid,
            position: self.player_position_like_cpp()?,
            flags: self.resolved_player_movement_flags_like_cpp()?,
            flags2: if self.resolved_can_swim_to_fly_transition_like_cpp()? {
                wow_constants::movement::MovementFlag2::CAN_SWIM_TO_FLY_TRANS
            } else {
                wow_constants::movement::MovementFlag2::NONE
            },
            time: self.resolved_player_movement_time_like_cpp()?,
            ..wow_packet::packets::movement::MovementInfo::default()
        })
    }

    pub fn player_position_like_cpp(&self) -> Option<wow_core::Position> {
        let canonical = self
            .core
            .with_owned_player_like_cpp(|player| player.unit().world().position());
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            return self.fixtures.movement.player_position;
        }
        canonical
    }

    pub fn resolved_player_movement_flags_like_cpp(&self) -> Option<MovementFlag> {
        let canonical = self
            .core
            .with_owned_player_like_cpp(|player| player.unit().movement_flags_like_cpp());
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            return Some(self.fixtures.movement.player_movement_flags_like_cpp);
        }
        canonical
    }

    pub fn player_moved_unit_guid_like_cpp(&self) -> Option<ObjectGuid> {
        let canonical = self.core.with_owned_player_like_cpp(|player| {
            player.unit().subsystems().control.unit_moved_by_me
        });
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            let guid = if self
                .fixtures
                .movement
                .player_moved_unit_guid_like_cpp
                .is_empty()
            {
                self.core.player_guid()?
            } else {
                self.fixtures.movement.player_moved_unit_guid_like_cpp
            };
            return Some(guid);
        }
        canonical.flatten()
    }

    pub fn resolved_player_movement_time_like_cpp(&self) -> Option<u32> {
        let canonical = self
            .core
            .with_owned_player_like_cpp(|player| player.unit().movement_time_like_cpp());
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            return Some(self.fixtures.movement.player_movement_time_like_cpp);
        }
        canonical
    }
}
