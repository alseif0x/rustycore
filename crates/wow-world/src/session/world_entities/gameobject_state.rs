//! Represented gameobject state publication, despawn and respawn.
//!
//! Moved out of the Session root under #599. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(crate) fn send_represented_gameobject_visibility_on_destroy_from_last_update_like_cpp(
        &mut self,
    ) -> usize {
        let (state, mut hub) = crate::session::split_world_entities_mut(self);
        state.send_represented_gameobject_visibility_on_destroy_from_last_update_like_cpp(&mut hub)
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/world_entities/gameobject_state/f3_shims.rs"]
mod f3_shims;
