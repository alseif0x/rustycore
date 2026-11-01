// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The `creature.*` hook family (source API 2, #1444 H3b).
//!
//! Map-level notifications, modelled on AzerothCore
//! `AllCreatureScript::OnCreatureAddWorld` (`AllCreatureScript.h:43`) and
//! `UnitScript::OnUnitDeath` (`UnitScript.h:111`) for a creature victim. The
//! canonical map tick reports them after its guards are released, on the
//! world server's map loop, so their effects are the `world.*` ones: a log
//! line or a system message to every player in the world.
//!
//! Both are **non-persistent observations** of an in-memory transition. They
//! are reported by the first map update that visits the creature after the
//! transition (the next tick), not synchronously inside it.

use wow_core::ObjectGuid;

use crate::world::WorldEffects;

/// Where a creature is: its map, map instance and position.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct CreatureLocation {
    pub map_id: u32,
    pub instance_id: u32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub orientation: f32,
}

impl CreatureLocation {
    #[must_use]
    pub fn new(map_id: u32, instance_id: u32, x: f32, y: f32, z: f32, orientation: f32) -> Self {
        Self {
            map_id,
            instance_id,
            x,
            y,
            z,
            orientation,
        }
    }
}

/// `creature.added_to_world`: a creature was added to its map.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct CreatureAddedToWorldSnapshot {
    pub guid: ObjectGuid,
    /// The creature template id.
    pub entry: u32,
    pub location: CreatureLocation,
}

impl CreatureAddedToWorldSnapshot {
    #[must_use]
    pub fn new(guid: ObjectGuid, entry: u32, location: CreatureLocation) -> Self {
        Self {
            guid,
            entry,
            location,
        }
    }
}

/// `creature.died`: a creature entered `JUST_DIED`.
///
/// The killer is not reported: the death transition does not carry it. A
/// player kill is also reported to that player's session as
/// `player.creature_kill`.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct CreatureDiedSnapshot {
    pub guid: ObjectGuid,
    /// The creature template id.
    pub entry: u32,
    pub location: CreatureLocation,
}

impl CreatureDiedSnapshot {
    #[must_use]
    pub fn new(guid: ObjectGuid, entry: u32, location: CreatureLocation) -> Self {
        Self {
            guid,
            entry,
            location,
        }
    }
}

/// The `creature.*` family. Every method has an empty default.
pub trait CreatureModule: Send + Sync {
    /// AzerothCore `AllCreatureScript::OnCreatureAddWorld`
    /// (`AllCreatureScript.h:43`, `Creature::AddToWorld`). Runs on the map
    /// loop for every creature added, respawns included: keep it cheap.
    fn on_added_to_world(
        &self,
        _snapshot: &CreatureAddedToWorldSnapshot,
        _effects: &mut WorldEffects<'_>,
    ) {
    }

    /// AzerothCore `UnitScript::OnUnitDeath` (`UnitScript.h:111`,
    /// `Unit::setDeathState(JUST_DIED)`) for a creature: exactly once per
    /// transition into `JUST_DIED`.
    fn on_died(&self, _snapshot: &CreatureDiedSnapshot, _effects: &mut WorldEffects<'_>) {}
}
