//! Receiver admission gates in the original share-loop order.
//! TrinityCore a5f8da2e, QuestHandler.cpp:603-756; represented gaps are retained.
use super::*;

impl WorldSession {
    pub(super) fn quest_share_admission_handled(
        &mut self,
        quest: &wow_data::quest::QuestTemplate,
        quest_id: u32,
        sender_guid: Option<ObjectGuid>,
        receiver_guid: ObjectGuid,
        receiver: &crate::session::directory::PlayerQuestSharingSnapshot,
        player_registry: &crate::session::directory::PlayerRegistry,
        blocked_by_unsupported_success_path: &mut bool,
    ) -> bool {
        if receiver.pending_quest_sharing.is_some() {
            self.send_push_quest_result_to_sender_with_title_if_available_like_cpp(
                receiver_guid,
                QUEST_PUSH_REASON_BUSY_LIKE_CPP,
                String::new(),
            );
            self.record_represented_push_quest_to_party_outcome_like_cpp(
                RepresentedPushQuestToPartyOutcomeLikeCpp {
                    sender_guid,
                    quest_id: quest_id,
                    target_guid: Some(receiver_guid),
                    reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverBusy,
                    quest_pool_active_check_unrepresented: false,
                    group_runtime_unrepresented: false,
                    receiver_fanout_unrepresented: false,
                },
            );
            return true;
        }

        if !receiver.is_alive {
            self.send_push_quest_result_to_sender_with_title_if_available_like_cpp(
                receiver_guid,
                QUEST_PUSH_REASON_DEAD_LIKE_CPP,
                String::new(),
            );
            if let Some(sender_guid) = sender_guid {
                let _ = player_registry.send_current_packet(
                    receiver.registration,
                    QuestPushResultResponse {
                        sender_guid,
                        result: QUEST_PUSH_REASON_DEAD_TO_RECIPIENT_LIKE_CPP,
                        quest_title: quest.log_title.clone(),
                    }
                    .to_bytes(),
                );
            }
            self.record_represented_push_quest_to_party_outcome_like_cpp(
                RepresentedPushQuestToPartyOutcomeLikeCpp {
                    sender_guid,
                    quest_id: quest_id,
                    target_guid: Some(receiver_guid),
                    reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverDead,
                    quest_pool_active_check_unrepresented: false,
                    group_runtime_unrepresented: false,
                    receiver_fanout_unrepresented: false,
                },
            );
            return true;
        }

        if receiver.rewarded_quests.contains(&quest_id) {
            let Some(sender_guid_for_receiver_packet) = sender_guid else {
                *blocked_by_unsupported_success_path = true;
                return true;
            };

            self.send_push_quest_result_to_sender_with_title_if_available_like_cpp(
                receiver_guid,
                QUEST_PUSH_REASON_ALREADY_DONE_LIKE_CPP,
                String::new(),
            );
            let _ = player_registry.send_current_packet(
                receiver.registration,
                QuestPushResultResponse {
                    sender_guid: sender_guid_for_receiver_packet,
                    result: QUEST_PUSH_REASON_ALREADY_DONE_TO_RECIPIENT_LIKE_CPP,
                    quest_title: quest.log_title.clone(),
                }
                .to_bytes(),
            );
            self.record_represented_push_quest_to_party_outcome_like_cpp(
                RepresentedPushQuestToPartyOutcomeLikeCpp {
                    sender_guid,
                    quest_id: quest_id,
                    target_guid: Some(receiver_guid),
                    reason:
                        RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverAlreadyDone,
                    quest_pool_active_check_unrepresented: false,
                    group_runtime_unrepresented: false,
                    receiver_fanout_unrepresented: false,
                },
            );
            return true;
        }

        if let Some(status) = receiver
            .active_quest_statuses
            .get(&quest_id)
            .copied()
        {
            let Some(sender_guid_for_receiver_packet) = sender_guid else {
                *blocked_by_unsupported_success_path = true;
                return true;
            };

            if status == QUEST_STATUS_INCOMPLETE_LIKE_CPP
                || status == QUEST_STATUS_COMPLETE_LIKE_CPP
            {
                self.send_push_quest_result_to_sender_with_title_if_available_like_cpp(
                    receiver_guid,
                    QUEST_PUSH_REASON_ON_QUEST_LIKE_CPP,
                    String::new(),
                );
                let _ = player_registry.send_current_packet(
                    receiver.registration,
                    QuestPushResultResponse {
                        sender_guid: sender_guid_for_receiver_packet,
                        result: QUEST_PUSH_REASON_ON_QUEST_TO_RECIPIENT_LIKE_CPP,
                        quest_title: quest.log_title.clone(),
                    }
                    .to_bytes(),
                );
                self.record_represented_push_quest_to_party_outcome_like_cpp(
                    RepresentedPushQuestToPartyOutcomeLikeCpp {
                        sender_guid,
                        quest_id: quest_id,
                        target_guid: Some(receiver_guid),
                        reason:
                            RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverOnQuest,
                        quest_pool_active_check_unrepresented: false,
                        group_runtime_unrepresented: false,
                        receiver_fanout_unrepresented: false,
                    },
                );
                return true;
            }
        }

        // C++ `Player::SatisfyQuestLog(false)` checks `FindQuestSlot(0) <
        // MAX_QUEST_LOG_SIZE`; this represented cross-session seam uses
        // the receiver snapshot derived from the canonical Player quest
        // slots via `sync_player_registry_state_like_cpp()`.
        if receiver.active_quest_statuses.len() >= MAX_QUEST_LOG_SIZE_LIKE_CPP as usize {
            let Some(sender_guid_for_receiver_packet) = sender_guid else {
                *blocked_by_unsupported_success_path = true;
                return true;
            };

            self.send_push_quest_result_to_sender_with_title_if_available_like_cpp(
                receiver_guid,
                QUEST_PUSH_REASON_LOG_FULL_LIKE_CPP,
                String::new(),
            );
            let _ = player_registry.send_current_packet(
                receiver.registration,
                QuestPushResultResponse {
                    sender_guid: sender_guid_for_receiver_packet,
                    result: QUEST_PUSH_REASON_LOG_FULL_TO_RECIPIENT_LIKE_CPP,
                    quest_title: quest.log_title.clone(),
                }
                .to_bytes(),
            );
            self.record_represented_push_quest_to_party_outcome_like_cpp(
                RepresentedPushQuestToPartyOutcomeLikeCpp {
                    sender_guid,
                    quest_id: quest_id,
                    target_guid: Some(receiver_guid),
                    reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverLogFull,
                    quest_pool_active_check_unrepresented: false,
                    group_runtime_unrepresented: false,
                    receiver_fanout_unrepresented: false,
                },
            );
            return true;
        }

        // C++ `Player::SatisfyQuestDay(quest, false)` immediately follows
        // `SatisfyQuestLog(false)` in `WorldSession::HandlePushQuestToParty`.
        // Non-daily/non-DF quests pass this gate; already-completed daily
        // quests and represented DF quests send the same AlreadyDone pair
        // as the earlier rewarded/onquest branch.
        let already_satisfied_quest_day_like_cpp =
            PlayerQuestGameplayState::quest_day_cooldown_block_from_membership(
                &quest.eligibility_rules(),
                || receiver.df_quests.contains(&quest_id),
                || receiver.daily_quests_completed.contains(&quest_id),
            )
            .is_some();

        if already_satisfied_quest_day_like_cpp {
            let Some(sender_guid_for_receiver_packet) = sender_guid else {
                *blocked_by_unsupported_success_path = true;
                return true;
            };

            self.send_push_quest_result_to_sender_with_title_if_available_like_cpp(
                receiver_guid,
                QUEST_PUSH_REASON_ALREADY_DONE_LIKE_CPP,
                String::new(),
            );
            let _ = player_registry.send_current_packet(
                receiver.registration,
                QuestPushResultResponse {
                    sender_guid: sender_guid_for_receiver_packet,
                    result: QUEST_PUSH_REASON_ALREADY_DONE_TO_RECIPIENT_LIKE_CPP,
                    quest_title: quest.log_title.clone(),
                }
                .to_bytes(),
            );
            self.record_represented_push_quest_to_party_outcome_like_cpp(
                RepresentedPushQuestToPartyOutcomeLikeCpp {
                    sender_guid,
                    quest_id: quest_id,
                    target_guid: Some(receiver_guid),
                    reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestDayAlreadyDone,
                    quest_pool_active_check_unrepresented: false,
                    group_runtime_unrepresented: false,
                    receiver_fanout_unrepresented: false,
                },
            );
            return true;
        }


        false
    }
}
