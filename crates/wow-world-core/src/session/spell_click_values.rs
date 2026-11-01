// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

#[cfg(any(test, feature = "test-fixtures"))]
use wow_constants::MovementFlag;
#[cfg(any(test, feature = "test-fixtures"))]
use wow_core::{ObjectGuid, Position};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RepresentedGameObjectAccessLikeCpp {
    pub entry: u32,
    pub position: wow_core::Position,
}

#[cfg(any(test, feature = "test-fixtures"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepresentedVehicleSeatChangeRequestLikeCpp {
    pub seat_id: i8,
    pub next: bool,
}

#[cfg(any(test, feature = "test-fixtures"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepresentedVehicleSeatSpellClickRequestLikeCpp {
    pub vehicle_guid: ObjectGuid,
    pub seat_id: i8,
    pub planned_casts: usize,
    pub exact_context_unrepresented: bool,
}

#[cfg(any(test, feature = "test-fixtures"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepresentedVehicleEnterRequestLikeCpp {
    pub vehicle_guid: ObjectGuid,
}

#[cfg(any(test, feature = "test-fixtures"))]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RepresentedVehicleDismissMovementLikeCpp {
    pub vehicle_guid: ObjectGuid,
    pub sanitized_flags: MovementFlag,
    pub position: Position,
    pub time: u32,
}

#[cfg(any(test, feature = "test-fixtures"))]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RepresentedVehicleBaseMovementLikeCpp {
    pub vehicle_guid: ObjectGuid,
    pub sanitized_flags: MovementFlag,
    pub position: Position,
    pub time: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RepresentedCreatureAccessLikeCpp {
    pub entry: u32,
    pub position: wow_core::Position,
    pub npc_flags: u32,
    pub npc_flags2: u32,
    pub trainer_class: u8,
    pub faction_template_id: u32,
}
