//! authority operations at the existing Quest application boundary.

use super::*;

impl WorldSession {
    pub(crate) fn record_represented_rewarded_quest_row_like_cpp(&mut self, quest_id: u32) {
        let _ = self.mutate_player_quest_gameplay_like_cpp(|state| {
            state.set_rewarded_row_like_cpp(quest_id, true);
        });
    }
    pub(crate) fn complete_player_quest_status_authority_load_like_cpp(&mut self) {
        let _ = self.mutate_player_quest_gameplay_like_cpp(|state| {
            state.set_status_authority_complete_like_cpp(true);
        });
    }
    pub(crate) fn represented_player_has_rewarded_quest_like_cpp(
        &self,
        quest_id: u32,
    ) -> Option<bool> {
        Some(
            self.player_quest_gameplay_snapshot_like_cpp()?
                .rewarded_quest_ids_like_cpp()
                .contains(&quest_id),
        )
    }
    pub(in crate::session) fn set_loaded_quest_completed_bit_like_cpp(
        &mut self,
        quest_bit: u32,
    ) -> bool {
        if quest_bit == 0 {
            return false;
        }

        let field_offset = (quest_bit - 1) / QUESTS_COMPLETED_BITS_PER_BLOCK;
        if field_offset as usize >= QUESTS_COMPLETED_BITS_SIZE {
            return false;
        }

        let canonical_changed = self
            .mutate_canonical_player_like_cpp(|player| {
                player.set_quest_completed_bit_like_cpp(quest_bit, true)
            })
            .unwrap_or(false);
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            return self
                .quest_test_fixture_like_cpp
                .represented_quest_completed_bits_like_cpp
                .insert(quest_bit);
        }
        canonical_changed
    }
    pub(in crate::session) fn clear_loaded_quest_completed_bit_like_cpp(
        &mut self,
        quest_bit: u32,
    ) -> bool {
        if quest_bit == 0 {
            return false;
        }

        let field_offset = (quest_bit - 1) / QUESTS_COMPLETED_BITS_PER_BLOCK;
        if field_offset as usize >= QUESTS_COMPLETED_BITS_SIZE {
            return false;
        }

        let canonical_changed = self
            .mutate_canonical_player_like_cpp(|player| {
                player.set_quest_completed_bit_like_cpp(quest_bit, false)
            })
            .unwrap_or(false);
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            return self
                .quest_test_fixture_like_cpp
                .represented_quest_completed_bits_like_cpp
                .remove(&quest_bit);
        }
        canonical_changed
    }
}
