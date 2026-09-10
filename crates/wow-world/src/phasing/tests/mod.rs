//! Phasing regressions.
//!
//! Separated from phasing.rs under #683.

use super::super::*;

use super::*;
use wow_constants::{PhaseFlags, PhaseShiftFlags, TypeId, TypeMask};
use wow_core::{ObjectGuid, guid::HighGuid};
use wow_data::{
    AreaTableEntry, MapEntry, MapStore, PhaseEntry, PhaseXPhaseGroupEntry,
    phase::{PHASE_ENTRY_FLAG_COSMETIC, PHASE_ENTRY_FLAG_PERSONAL},
};

fn world_object() -> WorldObject {
    let mut object = WorldObject::new(false, TypeId::Unit, TypeMask::UNIT);
    object.object_mut().create(ObjectGuid::create_world_object(
        HighGuid::Creature,
        0,
        0,
        571,
        0,
        1,
        1,
    ));
    object
}

fn unit(guid: ObjectGuid) -> Unit {
    let mut unit = Unit::new(true);
    unit.world_mut().object_mut().create(guid);
    unit
}

fn phase_store() -> PhaseStore {
    PhaseStore::from_entries([
        PhaseEntry { id: 10, flags: 0 },
        PhaseEntry {
            id: 20,
            flags: PHASE_ENTRY_FLAG_PERSONAL,
        },
        PhaseEntry {
            id: 30,
            flags: PHASE_ENTRY_FLAG_COSMETIC,
        },
    ])
}

fn phase_group_store(phase_store: &PhaseStore) -> PhaseGroupStore {
    PhaseGroupStore::from_entries(
        phase_store,
        [
            PhaseXPhaseGroupEntry {
                id: 1,
                phase_id: 10,
                phase_group_id: 7,
            },
            PhaseXPhaseGroupEntry {
                id: 2,
                phase_id: 20,
                phase_group_id: 7,
            },
            PhaseXPhaseGroupEntry {
                id: 3,
                phase_id: 99,
                phase_group_id: 7,
            },
        ],
    )
}

fn area_store() -> AreaTableStore {
    AreaTableStore::from_entries([
        AreaTableEntry {
            id: 100,
            continent_id: 0,
            parent_area_id: 0,
            area_bit: -1,
            exploration_level: 0,
            mount_flags: 0,
            flags: 0,
        },
        AreaTableEntry {
            id: 101,
            continent_id: 0,
            parent_area_id: 100,
            area_bit: -1,
            exploration_level: 0,
            mount_flags: 0,
            flags: 0,
        },
        AreaTableEntry {
            id: 102,
            continent_id: 0,
            parent_area_id: 100,
            area_bit: -1,
            exploration_level: 0,
            mount_flags: 0,
            flags: 0,
        },
    ])
}

fn phase_info_store(area_store: &AreaTableStore, phase_store: &PhaseStore) -> PhaseInfoStore {
    let mut store = PhaseInfoStore::from_phase_store_like_cpp(phase_store);
    store.load_area_phases_from_rows_like_cpp(
        area_store,
        phase_store,
        [(100, 10), (101, 10), (100, 20), (102, 30)],
    );
    store
}

fn map(id: u32, parent_map_id: i16) -> MapEntry {
    MapEntry {
        id,
        instance_type: 0,
        expansion_id: 0,
        parent_map_id,
        cosmetic_parent_map_id: -1,
        flags1: 0,
        flags2: 0,
    }
}

fn terrain_swap_store() -> TerrainSwapStore {
    let map_store = MapStore::from_entries([map(1, -1), map(571, -1), map(609, 571), map(700, 1)]);
    TerrainSwapStore::from_rows_like_cpp(
        &map_store,
        [(609, 42), (609, 43), (700, 70)],
        [(571, 609), (1, 700)],
        |_| true,
    )
}

mod scenarios;
