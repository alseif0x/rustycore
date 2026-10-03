use wow_core::ObjectGuid;
use wow_entities::PhaseShift;

use crate::{RepresentedGameObjectUseEffect, RepresentedGameObjectUseState, WorldEntitiesState};
#[cfg(any(test, feature = "test-fixtures"))]
use crate::RepresentedGameObjectCriteriaEvent;

impl WorldEntitiesState {
    pub fn represented_gameobject_phase_shift_like_cpp(
        &self,
        guid: ObjectGuid,
    ) -> Option<&PhaseShift> {
        self.represented_gameobject_phase_shifts.get(&guid)
    }

    pub fn remove_represented_gameobject_phase_shift_like_cpp(
        &mut self,
        guid: ObjectGuid,
    ) -> Option<PhaseShift> {
        self.represented_gameobject_phase_shifts.remove(&guid)
    }

    pub fn represented_gameobject_use_effects_len_like_cpp(&self) -> usize {
        self.represented_gameobject_use_effects.len()
    }

    pub fn represented_gameobject_use_effects_since_like_cpp(
        &self,
        start: usize,
    ) -> &[RepresentedGameObjectUseEffect] {
        self.represented_gameobject_use_effects
            .get(start..)
            .unwrap_or(&[])
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn record_represented_gameobject_criteria_event_like_cpp(
        &mut self,
        event: RepresentedGameObjectCriteriaEvent,
    ) {
        self.represented_gameobject_criteria_events.push(event);
    }

    pub fn represented_gameobject_use_states_iter_like_cpp(
        &self,
    ) -> impl Iterator<Item = (&ObjectGuid, &RepresentedGameObjectUseState)> + '_ {
        self.represented_gameobject_use_states.iter()
    }

    pub fn represented_gameobject_use_state_like_cpp(
        &self,
        guid: ObjectGuid,
    ) -> Option<&RepresentedGameObjectUseState> {
        self.represented_gameobject_use_states.get(&guid)
    }

    pub fn represented_gameobject_use_state_mut_like_cpp(
        &mut self,
        guid: ObjectGuid,
    ) -> Option<&mut RepresentedGameObjectUseState> {
        self.represented_gameobject_use_states.get_mut(&guid)
    }

    pub fn ensure_represented_gameobject_use_state_like_cpp(
        &mut self,
        guid: ObjectGuid,
    ) -> &mut RepresentedGameObjectUseState {
        self.represented_gameobject_use_states
            .entry(guid)
            .or_default()
    }

    pub fn record_represented_gameobject_use_effect_like_cpp(
        &mut self,
        effect: RepresentedGameObjectUseEffect,
    ) {
        self.represented_gameobject_use_effects.push(effect);
    }
}
