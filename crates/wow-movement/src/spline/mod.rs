// C++ movement code stores durations/curve params as int32/float and casts between them.
#![allow(clippy::cast_precision_loss, clippy::cast_possible_wrap)]

use std::f32::consts::PI;

use bitflags::bitflags;
use wow_constants::movement::MovementFlag;
use wow_core::{ObjectGuid, Position};

mod fall_and_jump;
mod init;
mod interpolation;
mod move_spline;

pub use fall_and_jump::*;
pub use init::*;
pub use move_spline::*;

use fall_and_jump::MINIMAL_DURATION_MS_LIKE_CPP;

use interpolation::{
    distance_3d, evaluate_catmullrom, evaluate_derivative_catmullrom, evaluate_derivative_linear,
    evaluate_linear, init_catmull_storage, init_cyclic_catmull_storage, init_lengths, ms_to_sec,
    segment_length, wrap_angle_0_2pi,
};

use move_spline::SplineData;

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
