//! Borrowed operations on represented gameobject-use values.
//!
//! These references retain the caller's state ownership. Session adapters use
//! their existing per-Session entries; this does not make those values shared
//! canonical GameObject state. Clocks, catalogs and publication stay with callers.

use std::time::{Duration, Instant};
use wow_core::{ObjectGuid, Position};
use super::{ChairUseSource, GoState, LootState, TrapUseSource, GO_FLAG_IN_USE,
    GAMEOBJECT_TYPE_BUTTON, GAMEOBJECT_TYPE_DOOR, GAMEOBJECT_TYPE_TRAP};

mod timed;
mod chair;
#[cfg(test)]
mod tests;

pub struct GameObjectUseValues<'a> {
    loot_state: &'a mut Option<LootState>,
    loot_state_unit_guid: &'a mut ObjectGuid,
    go_state: &'a mut Option<GoState>,
    prev_go_state: &'a mut Option<GoState>,
    gameobject_flags: &'a mut u32,
    cooldown_until: &'a mut Option<Instant>,
    go_type: &'a mut Option<u8>,
    trap_use_source: &'a mut Option<TrapUseSource>,
    chair_slots: &'a mut Vec<Option<ObjectGuid>>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum CooldownOutcome {
    NoCooldown,
    Rejected,
    Started { cooldown_secs: u32 },
}

#[derive(Debug, PartialEq, Eq)]
pub enum TrapUseEffect {
    CooldownRejected,
    CastSpell { spell_id: u32 },
    CooldownStarted { cooldown_secs: u32 },
}

pub struct ChairPlacement {
    slot: u32,
    teleport_position: Position,
    raw_stand_state: u32,
    trigger_event: Option<u32>,
}

impl ChairPlacement {
    pub fn slot(&self) -> u32 { self.slot }
    pub fn teleport_position(&self) -> Position { self.teleport_position }
    pub fn raw_stand_state(&self) -> u32 { self.raw_stand_state }
    pub fn trigger_event(&self) -> Option<u32> { self.trigger_event }
}

impl<'a> GameObjectUseValues<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn borrow(
        loot_state: &'a mut Option<LootState>,
        loot_state_unit_guid: &'a mut ObjectGuid,
        go_state: &'a mut Option<GoState>,
        prev_go_state: &'a mut Option<GoState>,
        gameobject_flags: &'a mut u32,
        cooldown_until: &'a mut Option<Instant>,
        go_type: &'a mut Option<u8>,
        trap_use_source: &'a mut Option<TrapUseSource>,
        chair_slots: &'a mut Vec<Option<ObjectGuid>>,
    ) -> Self {
        Self { loot_state, loot_state_unit_guid, go_state, prev_go_state,
            gameobject_flags, cooldown_until, go_type, trap_use_source, chair_slots }
    }
}
