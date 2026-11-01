// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use wow_core::ObjectGuid;

pub const PLAYER_FLAGS_TAXI_BENCHMARK_LIKE_CPP: u32 = 0x0002_0000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepresentedActivateTaxiLikeCpp {
    pub vendor: ObjectGuid,
    pub node: u32,
    pub ground_mount_id: u32,
    pub flying_mount_id: u32,
    pub preferred_mount_display: u32,
}
