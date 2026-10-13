//! Represented pet state at the Session boundary.
//!
//! Moved out of the Session root under #607. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    #[allow(dead_code)]
    pub(crate) fn set_represented_pet_mode_state_like_cpp(
        &mut self,
        pet_guid: Option<ObjectGuid>,
        react_state: u8,
        command_state: u8,
    ) {
        wow_world_lifecycle::set_represented_pet_mode_state_with_spell_like_cpp(
            self,
            pet_guid,
            react_state,
            command_state,
            0,
        );
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/pets/pet/f3_shims.rs"]
mod f3_shims;
