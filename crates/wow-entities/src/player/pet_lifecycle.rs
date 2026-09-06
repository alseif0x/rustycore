// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Player-owned pet lifetime and stable bookkeeping.

use super::*;
use crate::PlayerPetLifecycleStateLikeCpp;

impl Player {
    pub fn pet_lifecycle_state_like_cpp(&self) -> &PlayerPetLifecycleStateLikeCpp {
        &self.gameplay_state.pet_lifecycle
    }

    pub fn pet_lifecycle_state_mut_like_cpp(&mut self) -> &mut PlayerPetLifecycleStateLikeCpp {
        &mut self.gameplay_state.pet_lifecycle
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn player_owns_pet_lifecycle_bookkeeping_like_cpp() {
        let mut player = Player::new(Some(1), false);
        let state = player.pet_lifecycle_state_mut_like_cpp();
        state.temporary_unsummoned_pet_number = 42;
        state.old_pet_spell = 1234;
        state.temporary_mount_react_state = Some(2);
        state.character_rows_empty_authority_complete = true;

        let state = player.pet_lifecycle_state_like_cpp();
        assert_eq!(state.temporary_unsummoned_pet_number, 42);
        assert_eq!(state.old_pet_spell, 1234);
        assert_eq!(state.temporary_mount_react_state, Some(2));
        assert!(state.character_rows_empty_authority_complete);
    }
}
