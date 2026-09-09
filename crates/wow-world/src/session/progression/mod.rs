//! Represented character-progression responsibility, separated from the
//! Session root under #611. Each submodule owns one complete operation
//! group; the canonical owners keep authority over the state they touch.

use super::*;
mod reputation;
mod skills;
mod talents;
