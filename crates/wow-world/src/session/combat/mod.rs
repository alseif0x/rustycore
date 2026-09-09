//! Represented combat responsibility, separated from the
//! Session root under #617. Each submodule owns one complete operation
//! group; the canonical owners keep authority over the state they touch.

use super::*;
mod damage;
mod death;
mod melee;
mod state;
mod vitals;
