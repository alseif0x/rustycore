// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Taxi contracts: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::ObjectGuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedActivateTaxiLikeCpp {
    pub vendor: ObjectGuid,
    pub node: u32,
    pub ground_mount_id: u32,
    pub flying_mount_id: u32,
    pub preferred_mount_display: u32,
}
