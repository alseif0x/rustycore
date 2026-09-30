//! Source-authored cases using the actual two storage rails and operation entries.
use super::*;
use crate::map_manager::{
    WorldCreature, pending_respawn_from_world_creature_like_cpp, world_to_grid_coords,
};
use crate::spawn::{RespawnKey, SpawnObjectType};
use std::time::Duration;
use wow_core::{ObjectGuid, Position, guid::HighGuid};
use wow_entities::Creature;
use wow_persistence::RespawnPersistenceMutationLikeCpp;

mod support;
use support::*;
mod death_and_queue;
mod disposition;
