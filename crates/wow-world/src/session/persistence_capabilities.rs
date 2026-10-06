// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Persistence capabilities: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

#[cfg(test)]
pub(in crate::session) use wow_world_core::session::persistence_capabilities::empty_character_power_snapshot_like_cpp;
pub(crate) use wow_world_core::session::persistence_capabilities::{
    CharacterPowerSnapshotLikeCpp, PlayerSaveToDbSnapshotLikeCpp,
};
pub(in crate::session) use wow_world_core::session::persistence_capabilities::{
    character_power_snapshot_values_like_cpp, loaded_character_power_snapshot_like_cpp,
};

pub use wow_world_lifecycle::{
    CatalogPersistenceCapabilitiesLikeCpp, PlayerPersistenceCapabilitiesLikeCpp,
    SessionAdmissionPersistenceLikeCpp, SessionPersistencePortsLikeCpp,
    WorldPersistenceCapabilitiesLikeCpp,
};
