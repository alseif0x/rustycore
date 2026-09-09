//! Represented movement and world-transfer responsibility, separated from the
//! Session root under #603. Each submodule owns one complete operation
//! group; the canonical owners keep authority over the state they touch.

use super::*;
mod fall;
mod far_transfer;
mod movement_publication;
mod movement_validation;
mod speed;
mod state;
mod transfer;
