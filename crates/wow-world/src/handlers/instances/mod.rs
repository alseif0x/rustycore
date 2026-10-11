// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Thin compatibility entry points for the application-owned instance handlers.

#[cfg(test)]
use wow_packet::packets::misc::SetSavedInstanceExtend;

#[cfg(test)]
use crate::session::registry::PacketHandlerEntry;
#[cfg(test)]
use tracing::{info, warn};
#[cfg(test)]
use wow_constants::ClientOpcodes;
#[cfg(test)]
use wow_core::ObjectGuid;
#[cfg(test)]
use wow_handler::{PacketProcessing, SessionStatus};
#[cfg(test)]
use wow_packet::ClientPacket;
#[cfg(test)]
use wow_packet::packets::instance::{
    InstanceInfo, InstanceLockInfo, InstanceLockResponse, InstanceReset, InstanceResetFailed,
    InstanceSaveCreated, PendingRaidLock,
};
#[cfg(test)]
use wow_packet::packets::misc::{CalendarRaidLockoutAdded, CalendarRaidLockoutUpdated};
#[cfg(test)]
use wow_persistence::{InstanceLockPersistenceOutcomeLikeCpp, InstanceLockPersistencePlanLikeCpp};

#[cfg(test)]
#[path = "../../../unit_tests/handlers/instances/tests/mod.rs"]
mod tests;

#[cfg(test)]
#[path = "../../../unit_tests/handlers/instances/mod/f3_shims.rs"]
mod f3_shims;
