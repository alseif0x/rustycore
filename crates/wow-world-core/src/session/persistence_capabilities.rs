// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Canonical power snapshots and the player save projection shared with World.

use wow_core::{ObjectGuid, Position};
use wow_entities::MAX_POWERS_PER_CLASS;

pub type CharacterPowerSnapshotLikeCpp = [Option<i32>; MAX_POWERS_PER_CLASS];

#[cfg(any(test, feature = "test-fixtures"))]
pub fn empty_character_power_snapshot_like_cpp() -> CharacterPowerSnapshotLikeCpp {
    [None; MAX_POWERS_PER_CLASS]
}

pub fn loaded_character_power_snapshot_like_cpp(
    powers: [i32; MAX_POWERS_PER_CLASS],
) -> CharacterPowerSnapshotLikeCpp {
    powers.map(|power| Some(power.max(0)))
}

pub fn character_power_snapshot_values_like_cpp(
    powers: &CharacterPowerSnapshotLikeCpp,
) -> Option<[i32; MAX_POWERS_PER_CLASS]> {
    let mut values = [0; MAX_POWERS_PER_CLASS];
    for (index, power) in powers.iter().copied().enumerate() {
        values[index] = power?;
    }
    Some(values)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayerSaveToDbSnapshotLikeCpp {
    pub guid: ObjectGuid,
    pub map_id: u16,
    pub instance_id: u32,
    pub position: Position,
    pub level: u8,
    pub xp: u32,
    pub money: u64,
    pub health: u32,
    pub max_health: u32,
    pub powers: CharacterPowerSnapshotLikeCpp,
}
