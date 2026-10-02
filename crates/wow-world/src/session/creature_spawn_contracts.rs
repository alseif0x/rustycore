// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Creature spawn contracts: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::PowerType;

/// Parameters for spawning nearby creatures after login.
pub struct PendingCreatureSpawn {
    pub map_id: u16,
    pub position: wow_core::Position,
    pub zone_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CreatureCreateStatsLikeCpp {
    pub health: i64,
    pub max_health: i64,
    pub power_type: PowerType,
    pub power: i32,
    pub max_power: i32,
    pub base_mana: i32,
}
