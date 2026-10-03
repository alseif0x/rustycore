use wow_core::ObjectGuid;

use crate::{RepresentedGameObjectUseEffect, RepresentedGameObjectUseState, WorldEntitiesState};

impl WorldEntitiesState {
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
