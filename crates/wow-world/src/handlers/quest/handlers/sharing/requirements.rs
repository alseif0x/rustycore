//! Receiver requirements gates in the original share-loop order.
//! TrinityCore a5f8da2e, QuestHandler.cpp:603-756; represented gaps are retained.
use super::*;

impl WorldSession {
    pub(super) fn quest_share_requirements_handled(
        &mut self,
        quest: &wow_data::quest::QuestTemplate,
        quest_id: u32,
        sender_guid: Option<ObjectGuid>,
        receiver_guid: ObjectGuid,
        receiver: &crate::session::directory::PlayerQuestSharingSnapshot,
        player_registry: &crate::session::directory::PlayerRegistry,
        blocked_by_unsupported_success_path: &mut bool,
        quest_store: &wow_data::quest::QuestStore,
    ) -> bool {
        // C++ then evaluates `Player::SatisfyQuestMinLevel(quest, false)`
        // followed by `SatisfyQuestMaxLevel(quest, false)`.  Receiver
        // `level` is a derived cross-session snapshot synchronized from
        // the receiver `WorldSession`, never source-of-truth in reverse.
        if !quest.meets_min_level(receiver.level) {
            let Some(sender_guid_for_receiver_packet) = sender_guid else {
                *blocked_by_unsupported_success_path = true;
                return true;
            };

            self.send_push_quest_result_to_sender_with_title_if_available_like_cpp(
                receiver_guid,
                QUEST_PUSH_REASON_LOW_LEVEL_LIKE_CPP,
                String::new(),
            );
            let _ = player_registry.send_current_packet(
                receiver.registration,
                QuestPushResultResponse {
                    sender_guid: sender_guid_for_receiver_packet,
                    result: QUEST_PUSH_REASON_LOW_LEVEL_TO_RECIPIENT_LIKE_CPP,
                    quest_title: quest.log_title.clone(),
                }
                .to_bytes(),
            );
            self.record_represented_push_quest_to_party_outcome_like_cpp(
                RepresentedPushQuestToPartyOutcomeLikeCpp {
                    sender_guid,
                    quest_id: quest_id,
                    target_guid: Some(receiver_guid),
                    reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestMinLevelLowLevel,
                    quest_pool_active_check_unrepresented: false,
                    group_runtime_unrepresented: false,
                    receiver_fanout_unrepresented: false,
                },
            );
            return true;
        }

        if !quest.meets_max_level(receiver.level) {
            let Some(sender_guid_for_receiver_packet) = sender_guid else {
                *blocked_by_unsupported_success_path = true;
                return true;
            };

            self.send_push_quest_result_to_sender_with_title_if_available_like_cpp(
                receiver_guid,
                QUEST_PUSH_REASON_HIGH_LEVEL_LIKE_CPP,
                String::new(),
            );
            let _ = player_registry.send_current_packet(
                receiver.registration,
                QuestPushResultResponse {
                    sender_guid: sender_guid_for_receiver_packet,
                    result: QUEST_PUSH_REASON_HIGH_LEVEL_TO_RECIPIENT_LIKE_CPP,
                    quest_title: quest.log_title.clone(),
                }
                .to_bytes(),
            );
            self.record_represented_push_quest_to_party_outcome_like_cpp(
                RepresentedPushQuestToPartyOutcomeLikeCpp {
                    sender_guid,
                    quest_id: quest_id,
                    target_guid: Some(receiver_guid),
                    reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestMaxLevelHighLevel,
                    quest_pool_active_check_unrepresented: false,
                    group_runtime_unrepresented: false,
                    receiver_fanout_unrepresented: false,
                },
            );
            return true;
        }

        // C++ order then evaluates `Player::SatisfyQuestClass(quest, false)`
        // followed by `SatisfyQuestRace(quest, false)`. Receiver class/race
        // are read-only `PlayerRegistry` snapshots derived from the receiver
        // `WorldSession`; never sync registry state back into the session.
        let receiver_class_mask = player_race_or_class_mask_like_cpp(receiver.class);
        if quest.allowable_classes != 0 && (quest.allowable_classes & receiver_class_mask) == 0
        {
            let Some(sender_guid_for_receiver_packet) = sender_guid else {
                *blocked_by_unsupported_success_path = true;
                return true;
            };

            self.send_push_quest_result_to_sender_with_title_if_available_like_cpp(
                receiver_guid,
                QUEST_PUSH_REASON_CLASS_LIKE_CPP,
                String::new(),
            );
            let _ = player_registry.send_current_packet(
                receiver.registration,
                QuestPushResultResponse {
                    sender_guid: sender_guid_for_receiver_packet,
                    result: QUEST_PUSH_REASON_CLASS_TO_RECIPIENT_LIKE_CPP,
                    quest_title: quest.log_title.clone(),
                }
                .to_bytes(),
            );
            self.record_represented_push_quest_to_party_outcome_like_cpp(
                RepresentedPushQuestToPartyOutcomeLikeCpp {
                    sender_guid,
                    quest_id: quest_id,
                    target_guid: Some(receiver_guid),
                    reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestClassWrongClass,
                    quest_pool_active_check_unrepresented: false,
                    group_runtime_unrepresented: false,
                    receiver_fanout_unrepresented: false,
                },
            );
            return true;
        }

        let receiver_race_mask = u64::from(player_race_or_class_mask_like_cpp(receiver.race));
        if quest.allowable_races != 0 && (quest.allowable_races & receiver_race_mask) == 0 {
            let Some(sender_guid_for_receiver_packet) = sender_guid else {
                *blocked_by_unsupported_success_path = true;
                return true;
            };

            self.send_push_quest_result_to_sender_with_title_if_available_like_cpp(
                receiver_guid,
                QUEST_PUSH_REASON_RACE_LIKE_CPP,
                String::new(),
            );
            let _ = player_registry.send_current_packet(
                receiver.registration,
                QuestPushResultResponse {
                    sender_guid: sender_guid_for_receiver_packet,
                    result: QUEST_PUSH_REASON_RACE_TO_RECIPIENT_LIKE_CPP,
                    quest_title: quest.log_title.clone(),
                }
                .to_bytes(),
            );
            self.record_represented_push_quest_to_party_outcome_like_cpp(
                RepresentedPushQuestToPartyOutcomeLikeCpp {
                    sender_guid,
                    quest_id: quest_id,
                    target_guid: Some(receiver_guid),
                    reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestRaceWrongRace,
                    quest_pool_active_check_unrepresented: false,
                    group_runtime_unrepresented: false,
                    receiver_fanout_unrepresented: false,
                },
            );
            return true;
        }

        // The receiver's standings come off its canonical `Player` since #252.
        // `None` means unknown, not zero: mid far-teleport the owner has left
        // the old map and has not reached the destination. Evaluating a
        // standing gate against an empty set would tell the sender the
        // receiver's reputation is too low when it may well qualify, so report
        // the eligibility as unrepresented instead.
        let Some(receiver_reputation_standings) = receiver.reputation_standings.as_ref() else {
            self.record_represented_push_quest_to_party_outcome_like_cpp(
                RepresentedPushQuestToPartyOutcomeLikeCpp {
                    sender_guid,
                    quest_id: quest_id,
                    target_guid: Some(receiver_guid),
                    reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverEligibilityUnrepresented,
                    quest_pool_active_check_unrepresented: false,
                    group_runtime_unrepresented: false,
                    receiver_fanout_unrepresented: false,
                },
            );
            return true;
        };

        let receiver_reputation_standing_like_cpp = |faction_id: u32| -> i32 {
            receiver_reputation_standings
                .iter()
                .find_map(|(stored_faction_id, standing)| {
                    (*stored_faction_id == faction_id).then_some(*standing)
                })
                .unwrap_or(0)
        };

        let reputation_failure_reason = if quest.required_min_rep_faction != 0
            && receiver_reputation_standing_like_cpp(quest.required_min_rep_faction)
                < quest.required_min_rep_value
        {
            Some(RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestReputationLowFaction)
        } else if quest.required_max_rep_faction != 0
            && receiver_reputation_standing_like_cpp(quest.required_max_rep_faction)
                >= quest.required_max_rep_value
        {
            Some(RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestReputationHighFaction)
        } else {
            None
        };

        if let Some(reason) = reputation_failure_reason {
            let Some(sender_guid_for_receiver_packet) = sender_guid else {
                *blocked_by_unsupported_success_path = true;
                return true;
            };

            self.send_push_quest_result_to_sender_with_title_if_available_like_cpp(
                receiver_guid,
                QUEST_PUSH_REASON_LOW_FACTION_LIKE_CPP,
                String::new(),
            );
            let _ = player_registry.send_current_packet(
                receiver.registration,
                QuestPushResultResponse {
                    sender_guid: sender_guid_for_receiver_packet,
                    result: QUEST_PUSH_REASON_LOW_FACTION_TO_RECIPIENT_LIKE_CPP,
                    quest_title: quest.log_title.clone(),
                }
                .to_bytes(),
            );
            self.record_represented_push_quest_to_party_outcome_like_cpp(
                RepresentedPushQuestToPartyOutcomeLikeCpp {
                    sender_guid,
                    quest_id: quest_id,
                    target_guid: Some(receiver_guid),
                    reason,
                    quest_pool_active_check_unrepresented: false,
                    group_runtime_unrepresented: false,
                    receiver_fanout_unrepresented: false,
                },
            );
            return true;
        }

        // C++ `Player::SatisfyQuestDependentQuests(quest, false)` preserves this
        // sub-gate order: PreviousQuest, DependentPreviousQuests,
        // BreadcrumbQuest, DependentBreadcrumbQuests. The recursive breadcrumb
        // branch remains unrepresented; later APP admission and command delivery
        // remain in their existing stages after these prerequisite gates.
        let previous_quest_prerequisite_failed =
            !PlayerQuestGameplayState::previous_quest_requirement_satisfied_from_membership(
                quest.prev_quest_id,
                |id| receiver.rewarded_quests.contains(&id),
                |id| receiver.active_quest_statuses.get(&id).copied(),
            );

        let mut prerequisite_failure_reason =
            previous_quest_prerequisite_failed.then_some(
                RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestPreviousQuestPrerequisite,
            );

        if prerequisite_failure_reason.is_none()
            && represented_satisfy_quest_dependent_previous_quests_failed_like_cpp(
                quest_store,
                quest,
                &receiver.rewarded_quests,
            )
        {
            prerequisite_failure_reason = Some(
                RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestDependentPreviousQuestsPrerequisite,
            );
        }

        if prerequisite_failure_reason.is_none() && quest.breadcrumb_for_quest_id != 0 {
            // C++ `SatisfyQuestBreadcrumbQuest` depends on
            // `CanTakeQuest(target,false)`. Do not fake it here; keep the
            // success path blocked until real/represented CanTakeQuest is available.
            *blocked_by_unsupported_success_path = true;
            self.record_represented_push_quest_to_party_outcome_like_cpp(
                RepresentedPushQuestToPartyOutcomeLikeCpp {
                    sender_guid,
                    quest_id: quest_id,
                    target_guid: Some(receiver_guid),
                    reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverEligibilityUnrepresented,
                    quest_pool_active_check_unrepresented: false,
                    group_runtime_unrepresented: false,
                    receiver_fanout_unrepresented: true,
                },
            );
            return true;
        }

        if prerequisite_failure_reason.is_none()
            && represented_satisfy_quest_dependent_breadcrumb_quests_failed_like_cpp(
                quest,
                &receiver.active_quest_statuses,
            )
        {
            prerequisite_failure_reason = Some(
                RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestDependentBreadcrumbQuestsPrerequisite,
            );
        }

        if let Some(reason) = prerequisite_failure_reason {
            let Some(sender_guid_for_receiver_packet) = sender_guid else {
                *blocked_by_unsupported_success_path = true;
                return true;
            };

            self.send_push_quest_result_to_sender_with_title_if_available_like_cpp(
                receiver_guid,
                QUEST_PUSH_REASON_PREREQUISITE_LIKE_CPP,
                String::new(),
            );
            let _ = player_registry.send_current_packet(
                receiver.registration,
                QuestPushResultResponse {
                    sender_guid: sender_guid_for_receiver_packet,
                    result: QUEST_PUSH_REASON_PREREQUISITE_TO_RECIPIENT_LIKE_CPP,
                    quest_title: quest.log_title.clone(),
                }
                .to_bytes(),
            );
            self.record_represented_push_quest_to_party_outcome_like_cpp(
                RepresentedPushQuestToPartyOutcomeLikeCpp {
                    sender_guid,
                    quest_id: quest_id,
                    target_guid: Some(receiver_guid),
                    reason,
                    quest_pool_active_check_unrepresented: false,
                    group_runtime_unrepresented: false,
                    receiver_fanout_unrepresented: false,
                },
            );
            return true;
        }

        if i32::from(receiver.active_expansion) < quest.expansion {
            let Some(sender_guid_for_receiver_packet) = sender_guid else {
                *blocked_by_unsupported_success_path = true;
                return true;
            };

            self.send_push_quest_result_to_sender_with_title_if_available_like_cpp(
                receiver_guid,
                QUEST_PUSH_REASON_EXPANSION_LIKE_CPP,
                String::new(),
            );
            let _ = player_registry.send_current_packet(
                receiver.registration,
                QuestPushResultResponse {
                    sender_guid: sender_guid_for_receiver_packet,
                    result: QUEST_PUSH_REASON_EXPANSION_TO_RECIPIENT_LIKE_CPP,
                    quest_title: quest.log_title.clone(),
                }
                .to_bytes(),
            );
            self.record_represented_push_quest_to_party_outcome_like_cpp(
                RepresentedPushQuestToPartyOutcomeLikeCpp {
                    sender_guid,
                    quest_id: quest_id,
                    target_guid: Some(receiver_guid),
                    reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestExpansionRequiredExpansion,
                    quest_pool_active_check_unrepresented: false,
                    group_runtime_unrepresented: false,
                    receiver_fanout_unrepresented: false,
                },
            );
            return true;
        }

        if !represented_can_take_quest_after_expansion_like_cpp(quest_store, quest, receiver)
        {
            let Some(sender_guid_for_receiver_packet) = sender_guid else {
                *blocked_by_unsupported_success_path = true;
                return true;
            };

            self.send_push_quest_result_to_sender_with_title_if_available_like_cpp(
                receiver_guid,
                QUEST_PUSH_REASON_INVALID_LIKE_CPP,
                String::new(),
            );
            let _ = player_registry.send_current_packet(
                receiver.registration,
                QuestPushResultResponse {
                    sender_guid: sender_guid_for_receiver_packet,
                    result: QUEST_PUSH_REASON_INVALID_TO_RECIPIENT_LIKE_CPP,
                    quest_title: quest.log_title.clone(),
                }
                .to_bytes(),
            );
            self.record_represented_push_quest_to_party_outcome_like_cpp(
                RepresentedPushQuestToPartyOutcomeLikeCpp {
                    sender_guid,
                    quest_id: quest_id,
                    target_guid: Some(receiver_guid),
                    reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverCanTakeQuestInvalid,
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
