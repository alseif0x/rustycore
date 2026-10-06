// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Condition-object projections used by character handlers.
//!
//! These builders are kept beside the character state adapter because they
//! translate the canonical Player/Creature projections into the condition
//! engine's immutable snapshots; they do not own condition evaluation.

use super::*;

impl WorldSession {
    pub(crate) fn build_condition_creature_object_like_cpp(
        &mut self,
        npc_guid: ObjectGuid,
    ) -> Option<(WorldObject, wow_conditions::ConditionUnitSnapshot)> {
        self.core.mutate_world_creature(npc_guid, |creature| {
            let mut source =
                WorldObject::new(false, TypeId::Unit, TypeMask::OBJECT | TypeMask::UNIT);
            source.object_mut().create(creature.guid());
            source.object_mut().set_entry(creature.entry());
            let _ = source.set_map(creature.map_id(), creature.instance_id());
            source.relocate(creature.position());
            *source.phase_shift_mut() = creature.phase_shift().clone();
            let snapshot = wow_conditions::ConditionUnitSnapshot {
                level: u32::from(creature.level()),
                health: u64::from(creature.current_hp()),
                max_health: u64::from(creature.max_hp()),
                class_mask: 0,
                race: 0,
                creature_type: None,
                is_alive: creature.is_alive(),
                is_charmed: false,
                in_water: false,
                unit_state: 0,
                stand_state: UnitStandStateType::Stand as u32,
            };
            (source, snapshot)
        })
    }
}
