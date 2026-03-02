// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Movement flags for player and unit movement.

use bitflags::bitflags;

bitflags! {
    /// Primary movement flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct MovementFlag: u32 {
        const NONE                  = 0x0;
        const FORWARD               = 0x1;
        const BACKWARD              = 0x2;
        const STRAFE_LEFT           = 0x4;
        const STRAFE_RIGHT          = 0x8;
        const LEFT                  = 0x10;
        const RIGHT                 = 0x20;
        const PITCH_UP              = 0x40;
        const PITCH_DOWN            = 0x80;
        const WALKING               = 0x100;
        const DISABLE_GRAVITY       = 0x200;
        const ROOT                  = 0x400;
        const FALLING               = 0x800;
        const FALLING_FAR           = 0x1000;
        const PENDING_STOP          = 0x2000;
        const PENDING_STRAFE_STOP   = 0x4000;
        const PENDING_FORWARD       = 0x8000;
        const PENDING_BACKWARD      = 0x10000;
        const PENDING_STRAFE_LEFT   = 0x20000;
        const PENDING_STRAFE_RIGHT  = 0x40000;
        const PENDING_ROOT          = 0x80000;
        const SWIMMING              = 0x100000;
        const ASCENDING             = 0x200000;
        const DESCENDING            = 0x400000;
        const CAN_FLY               = 0x800000;
        const FLYING                = 0x1000000;
        const SPLINE_ELEVATION      = 0x2000000;
        const WATER_WALK            = 0x4000000;
        const FALLING_SLOW          = 0x8000000;
        const HOVER                 = 0x10000000;
        const DISABLE_COLLISION     = 0x20000000;

        const MASK_MOVING = Self::FORWARD.bits() | Self::BACKWARD.bits()
            | Self::STRAFE_LEFT.bits() | Self::STRAFE_RIGHT.bits()
            | Self::FALLING.bits() | Self::ASCENDING.bits() | Self::DESCENDING.bits();

        const MASK_TURNING = Self::LEFT.bits() | Self::RIGHT.bits()
            | Self::PITCH_UP.bits() | Self::PITCH_DOWN.bits();

        const MASK_MOVING_FLY = Self::FLYING.bits() | Self::ASCENDING.bits()
            | Self::DESCENDING.bits();

        const MASK_CREATURE_ALLOWED = Self::FORWARD.bits() | Self::DISABLE_GRAVITY.bits()
            | Self::ROOT.bits() | Self::SWIMMING.bits()
            | Self::CAN_FLY.bits() | Self::WATER_WALK.bits()
            | Self::FALLING_SLOW.bits() | Self::HOVER.bits()
            | Self::DISABLE_COLLISION.bits();

        const MASK_PLAYER_ONLY = Self::FLYING.bits();

        const MASK_HAS_PLAYER_STATUS_OPCODE = Self::DISABLE_GRAVITY.bits()
            | Self::ROOT.bits() | Self::CAN_FLY.bits()
            | Self::WATER_WALK.bits() | Self::FALLING_SLOW.bits()
            | Self::HOVER.bits() | Self::DISABLE_COLLISION.bits();
    }
}

bitflags! {
    /// Secondary movement flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct MovementFlag2: u32 {
        const NONE                                  = 0x0;
        const NO_STRAFE                             = 0x1;
        const NO_JUMPING                            = 0x2;
        const FULL_SPEED_TURNING                    = 0x4;
        const FULL_SPEED_PITCHING                   = 0x8;
        const ALWAYS_ALLOW_PITCHING                 = 0x10;
        const IS_VEHICLE_EXIT_VOLUNTARY             = 0x20;
        const WATERWALKING_FULL_PITCH               = 0x40;
        const VEHICLE_PASSENGER_IS_TRANSITION_ALLOWED = 0x80;
        const CAN_SWIM_TO_FLY_TRANS                 = 0x100;
        const UNK9                                  = 0x200;
        const CAN_TURN_WHILE_FALLING                = 0x400;
        const IGNORE_MOVEMENT_FORCES                = 0x800;
        const CAN_DOUBLE_JUMP                       = 0x1000;
        const DOUBLE_JUMP                           = 0x2000;
        // These flags are not sent
        const AWAITING_LOAD                         = 0x10000;
        const INTERPOLATED_MOVEMENT                 = 0x20000;
        const INTERPOLATED_TURNING                  = 0x40000;
        const INTERPOLATED_PITCHING                 = 0x80000;
    }
}

bitflags! {
    /// Tertiary movement flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct MovementFlags3: u32 {
        const NONE              = 0x00;
        const DISABLE_INERTIA   = 0x01;
        const CAN_ADV_FLY       = 0x02;
        const ADV_FLYING        = 0x04;
    }
}
