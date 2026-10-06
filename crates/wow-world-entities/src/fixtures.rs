use std::time::Instant;

use wow_core::ObjectGuid;
use wow_entities::PhaseShift;

use crate::{
    RepresentedGameObjectCriteriaEvent, RepresentedGameObjectUseState, WorldEntitiesState,
};

impl WorldEntitiesState {
    pub fn set_creature_tick_for_test_like_cpp(&mut self, tick: u32) {
        self.creature_tick = tick;
    }

    pub fn insert_represented_gameobject_use_state_for_test_like_cpp(
        &mut self,
        guid: ObjectGuid,
        state: RepresentedGameObjectUseState,
    ) -> Option<RepresentedGameObjectUseState> {
        self.represented_gameobject_use_states.insert(guid, state)
    }

    pub fn insert_represented_gameobject_phase_shift_for_test_like_cpp(
        &mut self,
        guid: ObjectGuid,
        phase_shift: PhaseShift,
    ) -> Option<PhaseShift> {
        self.represented_gameobject_phase_shifts
            .insert(guid, phase_shift)
    }

    pub fn clear_represented_gameobject_use_effects_for_test_like_cpp(&mut self) {
        self.represented_gameobject_use_effects.clear();
    }

    pub fn represented_gameobject_criteria_events_for_test_like_cpp(
        &self,
    ) -> &[RepresentedGameObjectCriteriaEvent] {
        &self.represented_gameobject_criteria_events
    }

    pub fn pending_creature_kill_loot_for_test_like_cpp(&self) -> &[ObjectGuid] {
        &self.pending_creature_kill_loot_like_cpp
    }

    pub fn set_represented_creature_auras_applied_at_for_test_like_cpp(
        &mut self,
        mut applied_at: impl FnMut() -> Instant,
    ) {
        for aura in &mut self.represented_creature_auras_like_cpp {
            aura.applied_at = applied_at();
        }
    }
}
