//! Represented quest objective progress and credit.
//!
//! Moved out of the Session root under #605. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    #[cfg(test)]
    pub(crate) async fn drain_represented_quest_objective_progress_like_cpp(&mut self) {
        let Some(generator) = self.item_guid_generator_like_cpp_for_bridge() else {
            return;
        };
        self.drain_represented_quest_objective_progress_with_generator_like_cpp(generator.as_ref())
            .await;
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/quest/objectives/f3_shims.rs"]
mod f3_shims;
