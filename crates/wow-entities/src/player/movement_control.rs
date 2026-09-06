use super::Player;

impl Player {
    pub fn fall_information_like_cpp(&self) -> (u32, f32) {
        let state = &self.gameplay_state.movement_control;
        (state.last_fall_time, state.last_fall_z)
    }

    pub fn set_fall_information_like_cpp(&mut self, time: u32, z: f32) {
        let state = &mut self.gameplay_state.movement_control;
        state.last_fall_time = time;
        state.last_fall_z = z;
    }

    pub fn forced_speed_changes_like_cpp(&self, move_type_index: usize) -> Option<u8> {
        self.gameplay_state
            .movement_control
            .forced_speed_changes
            .get(move_type_index)
            .copied()
    }

    pub fn set_forced_speed_changes_like_cpp(&mut self, move_type_index: usize, count: u8) -> bool {
        let Some(value) = self
            .gameplay_state
            .movement_control
            .forced_speed_changes
            .get_mut(move_type_index)
        else {
            return false;
        };
        *value = count;
        true
    }

    pub fn increment_forced_speed_changes_like_cpp(
        &mut self,
        move_type_index: usize,
    ) -> Option<u8> {
        let value = self
            .gameplay_state
            .movement_control
            .forced_speed_changes
            .get_mut(move_type_index)?;
        *value = value.saturating_add(1);
        Some(*value)
    }

    pub fn consume_forced_speed_change_like_cpp(&mut self, move_type_index: usize) -> Option<u8> {
        let value = self
            .gameplay_state
            .movement_control
            .forced_speed_changes
            .get_mut(move_type_index)?;
        if *value > 0 {
            *value = value.saturating_sub(1);
        }
        Some(*value)
    }

    pub fn movement_force_mod_magnitude_changes_like_cpp(&self) -> u8 {
        self.gameplay_state
            .movement_control
            .movement_force_mod_magnitude_changes
    }

    pub fn set_movement_force_mod_magnitude_changes_like_cpp(&mut self, count: u8) {
        self.gameplay_state
            .movement_control
            .movement_force_mod_magnitude_changes = count;
    }

    pub fn consume_movement_force_mod_magnitude_change_like_cpp(&mut self) -> u8 {
        let count = &mut self
            .gameplay_state
            .movement_control
            .movement_force_mod_magnitude_changes;
        if *count > 0 {
            *count = count.saturating_sub(1);
        }
        *count
    }
}

#[cfg(test)]
mod tests {
    use super::Player;

    #[test]
    fn player_owns_fall_and_forced_movement_ack_state_like_cpp() {
        let mut player = Player::new(Some(7), false);

        player.set_fall_information_like_cpp(1_200, 87.5);
        assert_eq!(player.fall_information_like_cpp(), (1_200, 87.5));

        assert_eq!(player.forced_speed_changes_like_cpp(1), Some(0));
        assert_eq!(player.increment_forced_speed_changes_like_cpp(1), Some(1));
        assert_eq!(player.increment_forced_speed_changes_like_cpp(1), Some(2));
        assert_eq!(player.consume_forced_speed_change_like_cpp(1), Some(1));

        player.set_movement_force_mod_magnitude_changes_like_cpp(2);
        assert_eq!(
            player.consume_movement_force_mod_magnitude_change_like_cpp(),
            1
        );
        assert_eq!(player.movement_force_mod_magnitude_changes_like_cpp(), 1);
        assert_eq!(player.forced_speed_changes_like_cpp(99), None);

        assert_eq!(player.unit().movement_counter_like_cpp(), 0);
        assert_eq!(player.unit_mut().next_movement_counter_like_cpp(), 0);
        assert_eq!(player.unit_mut().next_movement_counter_like_cpp(), 1);
        player.unit_mut().reset_movement_counter_like_cpp();
        assert_eq!(player.unit().movement_counter_like_cpp(), 0);
        assert!(player.unit_mut().set_speed_rate_at_like_cpp(1, 1.5));
        assert_eq!(player.unit().speed_rate_at_like_cpp(1), Some(1.5));
        player
            .unit_mut()
            .set_movement_force_mod_magnitude_like_cpp(1.25);
        assert_eq!(player.unit().movement_force_mod_magnitude_like_cpp(), 1.25);
    }
}
