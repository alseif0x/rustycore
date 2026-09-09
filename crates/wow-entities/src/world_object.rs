use std::collections::BTreeMap;
use std::f32::consts::{PI, TAU};

use wow_constants::{PhaseFlags, PhaseShiftFlags, TypeId, TypeMask};
use wow_core::{ObjectGuid, Position};

use crate::{
    EntityObject,
    vehicle::{calculate_passenger_offset, calculate_passenger_position},
};

mod ops_1;
mod state;
#[allow(unused_imports)]
pub use ops_1::*;
#[allow(unused_imports)]
pub use state::*;

#[cfg(test)]
#[path = "world_object/tests/mod.rs"]
mod tests;
