use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

use wow_constants::{
    DeathState, Gender, PowerType, ShapeShiftForm, SheathState, SpellState, TypeId, TypeMask,
    UnitFlags, UnitFlags2, UnitFlags3, UnitPvpFlags, UnitStandStateType, UnitState,
    WeaponAttackType, movement::MovementFlag,
};
use wow_core::ObjectGuid;

use crate::{
    CurrentSpellRef, CurrentSpellSlot, MotionMasterAddToWorldOutcomeLikeCpp, ObjectDataUpdate,
    UnitSubsystems, UpdateMask, VehicleKitRemoveOutcomeLikeCpp, VisibleItemValues, WorldObject,
    update_fields::{TYPEID_UNIT, UNIT_DATA_BITS},
};

mod ops_1;
mod ops_2;
mod ops_3;
mod state;
#[allow(unused_imports)]
pub use ops_1::*;
#[allow(unused_imports)]
pub use ops_2::*;
#[allow(unused_imports)]
pub use ops_3::*;
#[allow(unused_imports)]
pub use state::*;

#[cfg(test)]
#[path = "unit/tests/mod.rs"]
mod tests;
