//! Represented instance and difficulty responsibility, separated from the
//! Session root under #613. Each submodule owns one complete operation
//! group; the canonical owners keep authority over the state they touch.

use super::*;
mod binding;
mod difficulty;
mod instance;
mod map_key;
mod map_resolution;
