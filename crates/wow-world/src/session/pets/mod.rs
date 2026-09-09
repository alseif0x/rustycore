//! Represented pet and battle-pet responsibility, separated from the
//! Session root under #607. Each submodule owns one complete operation
//! group; the canonical owners keep authority over the state they touch.

use super::*;
mod battle_pet;
mod battle_pet_journal;
mod battle_pet_publication;
mod battle_pet_slots;
mod persistence;
mod pet;
mod summoning;
