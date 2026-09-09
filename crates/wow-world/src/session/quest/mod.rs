//! Represented quest responsibility, separated from the
//! Session root under #605. Each submodule owns one complete operation
//! group; the canonical owners keep authority over the state they touch.

use super::*;

mod giver;
mod objectives;
mod persistence;
mod publication;
mod reset;
mod rewards;
mod state;
