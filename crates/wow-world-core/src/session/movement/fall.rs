//! Hub operations for represented fall and jump state.

#[cfg(any(test, feature = "test-fixtures"))]
use crate::session::movement_protocol::MovementFallDamageEvent;
use wow_constants::MovementFlag;

#[cfg(any(test, feature = "test-fixtures"))]
impl crate::session::state::MovementState {
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fall_damage_events_like_cpp(&self) -> &[MovementFallDamageEvent] {
        &self.fall_damage_events_like_cpp
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn player_movement_jump_like_cpp(&self) -> &wow_packet::packets::movement::JumpInfo {
        &self.player_movement_jump_like_cpp
    }
}
impl crate::session::HubRef<'_> {
    pub fn resolved_fall_information_like_cpp(&self) -> Option<(u32, f32)> {
        let canonical = self
            .core
            .with_owned_player_like_cpp(|player| player.fall_information_like_cpp());
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            return Some((
                self.fixtures.movement.last_fall_time_like_cpp,
                self.fixtures.movement.last_fall_z_like_cpp,
            ));
        }
        canonical
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fall_information_like_cpp(&self) -> (u32, f32) {
        self.resolved_fall_information_like_cpp()
            .expect("test Player fall-information owner must resolve")
    }
}
impl crate::session::HubMut<'_> {
    pub fn set_player_movement_jump_like_cpp(
        &mut self,
        jump: wow_packet::packets::movement::JumpInfo,
    ) {
        #[cfg(any(test, feature = "test-fixtures"))]
        {
            self.fixtures.movement.player_movement_jump_like_cpp = jump;
        }
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let _ = jump;
    }

    pub fn update_fall_information_if_needed_like_cpp(
        &mut self,
        movement_info: &wow_packet::packets::movement::MovementInfo,
        is_fall_land: bool,
    ) {
        let Some((last_fall_time, last_fall_z)) =
            self.shared().resolved_fall_information_like_cpp()
        else {
            return;
        };
        if last_fall_time >= movement_info.jump.fall_time
            || last_fall_z <= movement_info.position.z
            || is_fall_land
        {
            self.set_fall_information_like_cpp(
                movement_info.jump.fall_time,
                movement_info.position.z,
            );
        }
    }

    pub fn move_represented_player_fall_like_cpp(&mut self) -> bool {
        let Some(mut movement_flags) = self.shared().resolved_player_movement_flags_like_cpp()
        else {
            return false;
        };
        if movement_flags.contains(MovementFlag::DISABLE_GRAVITY) {
            return false;
        }

        movement_flags.insert(MovementFlag::FALLING);
        self.set_player_movement_flags_like_cpp(movement_flags);
        if let Some(position) = self.shared().player_position_like_cpp() {
            self.set_fall_information_like_cpp(0, position.z);
        }
        true
    }
}
impl crate::session::HubMut<'_> {
    pub fn set_fall_information_like_cpp(&mut self, time: u32, z: f32) -> bool {
        let canonical = self
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player.set_fall_information_like_cpp(time, z);
            })
            .is_some();
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical || self.core.player_handle_like_cpp.is_none() {
            self.fixtures.movement.last_fall_time_like_cpp = time;
            self.fixtures.movement.last_fall_z_like_cpp = z;
        }
        canonical
            || cfg!(any(test, feature = "test-fixtures"))
                && self.core.player_handle_like_cpp.is_none()
    }
}
