use super::Player;
use crate::Vehicle;
use wow_constants::{TypeId, UnitFlags};
use wow_core::ObjectGuid;

impl Player {
    /// Apply the Player's mount presentation fields as one owner transition.
    ///
    /// TrinityCore's `Unit::Mount`/`Unit::Dismount` update the Unit-owned
    /// `MountDisplayID` and `UNIT_FLAG_MOUNT` fields together
    /// (`Entities/Unit/Unit.cpp:7822-7865`). Session owns the aura and
    /// collision side effects around this projection; the Player owns the
    /// canonical Unit fields.
    pub fn set_mount_presentation_like_cpp(&mut self, display_id: u32, mounted: bool) {
        self.unit_mut().set_mount_display_id(display_id);
        let mut flags = self.unit().unit_flags_like_cpp();
        if mounted {
            flags.insert(UnitFlags::MOUNT);
        } else {
            flags.remove(UnitFlags::MOUNT);
        }
        self.unit_mut().set_unit_flags_like_cpp(flags);
    }

    /// C++ `Unit::GetVehicleKit()` for the Player-owned mount vehicle.
    pub fn mount_vehicle_kit_snapshot_like_cpp(&self) -> Option<Vehicle> {
        self.gameplay_state().mount_vehicle_kit.clone()
    }

    /// C++ `Unit::CreateVehicleKit` installs the kit on the Unit that owns it.
    pub fn install_mount_vehicle_kit_like_cpp(&mut self, vehicle_kit: Vehicle) {
        self.gameplay_state_mut().mount_vehicle_kit = Some(vehicle_kit);
    }

    /// Clear the Player-owned kit without adding any packet or aura side
    /// effects. This is the narrow state operation used by aura teardown
    /// paths that already performed their own represented cleanup.
    pub fn clear_mount_vehicle_kit_like_cpp(&mut self) {
        self.gameplay_state_mut().mount_vehicle_kit = None;
    }

    /// C++ `Unit::RemoveVehicleKit`: uninstall passengers before releasing the
    /// owner-side kit. Packet and aura side effects remain Session operations.
    pub fn remove_mount_vehicle_kit_like_cpp(&mut self) -> bool {
        let Some(vehicle_kit) = self.gameplay_state_mut().mount_vehicle_kit.as_mut() else {
            return false;
        };
        vehicle_kit.uninstall();
        self.gameplay_state_mut().mount_vehicle_kit = None;
        true
    }

    /// C++ `Unit::ExitVehicle`'s ejectable-seat mutation for a Player-owned
    /// vehicle kit. The caller owns the resulting side effects and packets.
    pub fn eject_mount_vehicle_passenger_like_cpp(&mut self, passenger_guid: ObjectGuid) -> bool {
        let passenger_type_id = if passenger_guid.is_player() {
            TypeId::Player
        } else {
            TypeId::Unit
        };
        let Some(vehicle_kit) = self.gameplay_state_mut().mount_vehicle_kit.as_mut() else {
            return false;
        };
        if !vehicle_kit
            .seat_info_for_passenger_like_cpp(passenger_guid)
            .is_some_and(|seat| seat.ejectable)
        {
            return false;
        }
        vehicle_kit
            .remove_passenger_plan_like_cpp(
                passenger_guid,
                passenger_type_id,
                false,
                false,
                false,
                false,
            )
            .is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::Player;
    use crate::{Vehicle, VehicleSeatAddon, VehicleSeatInfo};
    use wow_constants::{TypeId, UnitFlags};
    use wow_core::{ObjectGuid, Position};

    fn player() -> Player {
        Player::new(Some(1), false)
    }

    fn vehicle(player: &Player, passenger: ObjectGuid, ejectable: bool) -> Vehicle {
        let mut vehicle = Vehicle::new(
            player.guid(),
            TypeId::Player,
            Position::default(),
            77,
            88,
            [(
                0,
                VehicleSeatInfo {
                    id: 0,
                    attachment_offset: Position::default(),
                    can_enter_or_exit: true,
                    usable_by_override: false,
                    can_control: false,
                    can_switch_from_seat: false,
                    ejectable,
                    disables_gravity: false,
                    passenger_not_selectable: false,
                    keep_pet: true,
                },
                VehicleSeatAddon::default(),
            )],
        );
        vehicle.install();
        assert!(vehicle.add_vehicle_passenger(passenger, 0));
        vehicle
    }

    #[test]
    fn player_owns_mount_vehicle_kit_lifecycle_like_cpp() {
        let mut player = player();
        let passenger = ObjectGuid::create_player(1, 2);
        player.install_mount_vehicle_kit_like_cpp(vehicle(&player, passenger, true));
        assert_eq!(
            player
                .mount_vehicle_kit_snapshot_like_cpp()
                .unwrap()
                .vehicle_id(),
            77
        );
        assert!(player.eject_mount_vehicle_passenger_like_cpp(passenger));
        assert!(player.remove_mount_vehicle_kit_like_cpp());
        assert!(player.mount_vehicle_kit_snapshot_like_cpp().is_none());
    }

    #[test]
    fn player_names_mount_presentation_transition_like_cpp() {
        let mut player = player();
        player.set_mount_presentation_like_cpp(4321, true);
        assert_eq!(player.unit().data().mount_display_id, 4321);
        assert!(
            player
                .unit()
                .unit_flags_like_cpp()
                .contains(UnitFlags::MOUNT)
        );

        player.set_mount_presentation_like_cpp(0, false);
        assert_eq!(player.unit().data().mount_display_id, 0);
        assert!(
            !player
                .unit()
                .unit_flags_like_cpp()
                .contains(UnitFlags::MOUNT)
        );
    }
}
