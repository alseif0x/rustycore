use std::{
    collections::HashMap,
    time::{SystemTime, UNIX_EPOCH},
};

use wow_constants::{
    Class, CreatureChaseMovementType, CreatureFlagsExtra, CreatureFlightMovementType,
    CreatureGroundMovementType, CreatureRandomMovementType, CreatureStaticFlags,
    CreatureStaticFlags4, CreatureType, CreatureTypeFlags, DeathState, PowerType, ShapeShiftForm,
    SheathState, TypeId, TypeMask, UnitDynFlags, UnitFlags, UnitFlags2, UnitFlags3, UnitMoveType,
    UnitPvpFlags, UnitStandStateType, UnitState, WeaponAttackType, movement::MovementFlag,
};
use wow_core::{ObjectGuid, Position};
use wow_loot::{
    CreatureLoot, LootInstallOutcome, OwnedLootAuthority, OwnedLootAuthorityStamp,
    OwnedLootSnapshot,
};

use crate::{
    BASE_MAXDAMAGE, BASE_MINDAMAGE, MoveFallPlan, MovementGeneratorKind,
    UNIT_MASK_CONTROLABLE_GUARDIAN, UNIT_MASK_GUARDIAN, UNIT_MASK_MINION, UNIT_MASK_PET,
    UNIT_MASK_TOTEM, UNIT_MASK_VEHICLE, Unit, VehicleAccessory, VehicleSeatAddon, VehicleSeatInfo,
    VisibilityDistanceTypeLikeCpp, VisibleItemValues,
};

mod ops_1;
mod ops_2;
mod ops_3;
mod state_1;
mod state_2;
#[allow(unused_imports)]
pub use ops_1::*;
#[allow(unused_imports)]
pub use ops_2::*;
#[allow(unused_imports)]
pub use ops_3::*;
#[allow(unused_imports)]
pub use state_1::*;
#[allow(unused_imports)]
pub use state_2::*;

#[cfg(test)]
#[path = "creature_tests.rs"]
mod tests;
