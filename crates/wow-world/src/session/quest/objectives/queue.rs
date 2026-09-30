//! queue operations at the existing Quest application boundary.

use super::*;

impl WorldSession {
    pub(crate) fn enqueue_represented_quest_objective_progress_like_cpp(
        &mut self,
        event: RepresentedQuestObjectiveProgressEventLikeCpp,
    ) {
        self.quest_state
            .represented_quest_objective_progress_events_like_cpp
            .push_back(event);
    }
    pub(crate) async fn drain_represented_quest_objective_progress_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
    ) {
        if self
            .quest_state
            .represented_quest_objective_progress_draining_like_cpp
        {
            return;
        }

        self.quest_state
            .represented_quest_objective_progress_draining_like_cpp = true;
        while let Some(event) = self
            .quest_state
            .represented_quest_objective_progress_events_like_cpp
            .pop_front()
        {
            match event {
                RepresentedQuestObjectiveProgressEventLikeCpp::MoneyChanged {
                    old_money,
                    new_money,
                } => {
                    self.update_represented_money_quest_objective_progress_like_cpp(
                        item_guid_generator,
                        old_money,
                        new_money,
                    )
                    .await;
                }
                RepresentedQuestObjectiveProgressEventLikeCpp::CurrencyChanged {
                    currency_id,
                    change,
                } => {
                    let object_id = i32::try_from(currency_id).unwrap_or(i32::MAX);
                    self.update_represented_currency_quest_objective_progress_like_cpp(
                        item_guid_generator,
                        currency_id,
                        change,
                    )
                    .await;
                    self.update_represented_storing_value_quest_objective_progress_like_cpp(
                        item_guid_generator,
                        QUEST_OBJECTIVE_HAVE_CURRENCY_LIKE_CPP,
                        object_id,
                        change,
                        wow_core::ObjectGuid::new(0, 0),
                    )
                    .await;
                    self.update_represented_storing_value_quest_objective_progress_like_cpp(
                        item_guid_generator,
                        QUEST_OBJECTIVE_OBTAIN_CURRENCY_LIKE_CPP,
                        object_id,
                        change,
                        wow_core::ObjectGuid::new(0, 0),
                    )
                    .await;
                }
                RepresentedQuestObjectiveProgressEventLikeCpp::ReputationChanged {
                    faction_id,
                    change,
                } => {
                    let object_id = i32::try_from(faction_id).unwrap_or(i32::MAX);
                    self.update_represented_reputation_quest_objective_progress_like_cpp(
                        item_guid_generator,
                        QUEST_OBJECTIVE_MIN_REPUTATION_LIKE_CPP,
                        faction_id,
                        change,
                    )
                    .await;
                    self.update_represented_reputation_quest_objective_progress_like_cpp(
                        item_guid_generator,
                        QUEST_OBJECTIVE_MAX_REPUTATION_LIKE_CPP,
                        faction_id,
                        change,
                    )
                    .await;
                    self.update_represented_storing_value_quest_objective_progress_like_cpp(
                        item_guid_generator,
                        QUEST_OBJECTIVE_INCREASE_REPUTATION_LIKE_CPP,
                        object_id,
                        change,
                        wow_core::ObjectGuid::new(0, 0),
                    )
                    .await;
                }
            }
        }
        self.quest_state
            .represented_quest_objective_progress_draining_like_cpp = false;
    }
    #[cfg(test)]
    pub(crate) async fn drain_represented_quest_objective_progress_like_cpp(&mut self) {
        let Some(generator) = self.item_guid_generator_like_cpp_for_bridge() else {
            return;
        };
        self.drain_represented_quest_objective_progress_with_generator_like_cpp(generator.as_ref())
            .await;
    }
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) fn represented_quest_push_result_sender_mismatch_count_like_cpp(&self) -> u32 {
        self.quest_test_fixture_like_cpp
            .represented_quest_push_result_sender_mismatch_count_like_cpp
    }
}
