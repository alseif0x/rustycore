//! Represented social responsibility, separated from the
//! Session root under #615. Each submodule owns one complete operation
//! group; the canonical owners keep authority over the state they touch.

use super::*;

mod contacts;
mod duel;
mod group;
mod guild;
mod trade;
