//! Instance lifecycle state regression scenarios.
//!
//! Separated from the lib.rs root under #658.

use super::*;

fn player(counter: i64) -> ObjectGuid {
    ObjectGuid::new(0x10, counter)
}

fn encounter(id: u32, difficulty_id: i32) -> DungeonEncounterEntry {
    encounter_with_bit(id, difficulty_id, 0)
}

fn encounter_with_bit(id: u32, difficulty_id: i32, bit: i8) -> DungeonEncounterEntry {
    DungeonEncounterEntry {
        id,
        map_id: 631,
        difficulty_id,
        order_index: 0,
        bit,
        flags: 0,
        faction: -1,
    }
}

fn raid_entries() -> MapDb2Entries {
    MapDb2Entries {
        map_id: 631,
        difficulty_id: 4,
        lock_id: 7,
        reset_interval: MapDifficultyResetInterval::Weekly,
        max_players: 25,
        is_flex_locking: false,
        is_using_encounter_locks: false,
    }
}

fn flex_entries() -> MapDb2Entries {
    MapDb2Entries {
        is_flex_locking: true,
        is_using_encounter_locks: true,
        ..raid_entries()
    }
}

fn update_event(instance_id: u32, bit: Option<u8>) -> InstanceLockUpdateEvent {
    InstanceLockUpdateEvent {
        instance_id,
        new_data: "bosses:1".to_string(),
        instance_completed_encounters_mask: 0b100,
        completed_encounter_bit: bit,
        entrance_world_safe_loc_id: Some(42),
    }
}

mod scenarios_1;
mod scenarios_2;
