//! Narrow fixture calls to the complete Group membership operations.

use super::WorldSession;

impl WorldSession {
    pub fn group_reset_update_sequence_for_test(&mut self) -> bool {
        self.reset_group_update_sequence_if_needed_like_cpp()
    }

    pub fn group_next_update_sequence_for_test(&mut self, category: u8) -> Option<i32> {
        self.next_group_update_sequence_number_like_cpp(category)
    }

    pub fn group_apply_leader_flag_for_test(&mut self) -> bool {
        self.apply_represented_group_leader_flag_like_cpp()
    }
}
