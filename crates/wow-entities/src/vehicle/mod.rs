use std::collections::BTreeMap;

use wow_constants::{MovementFlag2, TypeId};
pub use wow_constants::{VehicleExitParameter, VehicleFlag};
use wow_core::{ObjectGuid, Position};

mod immunity;
mod plans;
mod seats;
mod vehicle;

pub use immunity::*;
pub use plans::*;
pub use seats::*;
pub use vehicle::*;

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
