//! Complete dependent-previous Quest eligibility against borrowed catalog facts.
//!
//! TrinityCore a5f8da2ebf5424bf0450ca4e08843ecbf72577bd,
//! Player.cpp:15115–15171, Player::SatisfyQuestDependentPreviousQuests.
//! Missing catalog rows retain Rust's fail-closed result instead of C++'s ASSERT.
//! The application supplies its existing lazy exclusive-group iterator unchanged.

use super::PlayerQuestGameplayState;

#[cfg(test)]
mod tests;

impl PlayerQuestGameplayState {
    /// Whether the represented dependent-previous prerequisite blocks admission.
    /// Catalog lookup precedes rewarded membership; the first rewarded candidate
    /// decides the result, including an incomplete negative exclusive group.
    pub fn dependent_previous_quest_ids_block<I>(
        dependent_previous_ids: &[u32],
        mut previous_exclusive_group: impl FnMut(u32) -> Option<i32>,
        mut negative_group_ids: impl FnMut(i32) -> I,
        mut is_rewarded: impl FnMut(u32) -> bool,
    ) -> bool
    where
        I: Iterator<Item = u32>,
    {
        if dependent_previous_ids.is_empty() {
            return false;
        }

        for &prev_id in dependent_previous_ids {
            let Some(exclusive_group) = previous_exclusive_group(prev_id) else {
                return true;
            };

            if is_rewarded(prev_id) {
                if exclusive_group >= 0 {
                    return false;
                }

                for exclusive_quest_id in negative_group_ids(exclusive_group) {
                    if exclusive_quest_id != prev_id && !is_rewarded(exclusive_quest_id) {
                        return true;
                    }
                }

                return false;
            }
        }

        true
    }
}
