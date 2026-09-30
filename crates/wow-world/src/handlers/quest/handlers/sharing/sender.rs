//! Sender preflight, receiver iteration and quest-share publication.
use super::*;

impl WorldSession {


    /// CMSG_PUSH_QUEST_TO_PARTY — sender-side bounded quest share preflight.
    ///
    /// C++ anchors:
    /// - `Opcodes.cpp:746`: `STATUS_LOGGEDIN`, `PROCESS_THREADUNSAFE`, `HandlePushQuestToParty`.
    /// - `QuestPackets.cpp:658-661`: packet reads one `uint32 QuestID`.
    /// - `QuestHandler.cpp:603-756`: template lookup, `CanShareQuest`, quest-pool active,
    ///   group presence, then receiver iteration.
    ///
    /// Represented-partial: this records sender-local evidence only. It never mutates DB/maps,
    /// never sets receiver pending sharing, and never fans out packets to other sessions.
    /// If the session has no real `player_guid`, Rust records the existing evidence only and
    /// does not fabricate an empty sender GUID for `SMSG_QUEST_PUSH_RESULT`.
    pub async fn handle_push_quest_to_party(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match PushQuestToParty::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    ?error,
                    "PushQuestToParty: failed to read QuestID"
                );
                return;
            }
        };

        let Some(quest_store) = self.quests.store.as_ref().map(Arc::clone) else {
            debug!(
                account = self.account_id,
                quest_id = packet.quest_id,
                "PushQuestToParty: missing QuestStore, silent return like missing ObjectMgr template path"
            );
            return;
        };

        let Some(quest) = quest_store.get(packet.quest_id) else {
            debug!(
                account = self.account_id,
                quest_id = packet.quest_id,
                "PushQuestToParty: missing quest template, silent return like C++"
            );
            return;
        };

        let sender_guid = self.player_guid();
        if !self.represented_can_share_quest_like_cpp(quest) {
            self.send_push_quest_result_to_sender_if_available_like_cpp(
                sender_guid,
                quest_push_reason::NOT_ALLOWED,
            );
            self.record_represented_push_quest_to_party_outcome_like_cpp(
                RepresentedPushQuestToPartyOutcomeLikeCpp {
                    sender_guid,
                    quest_id: packet.quest_id,
                    target_guid: sender_guid,
                    reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::NotAllowed,
                    quest_pool_active_check_unrepresented: false,
                    group_runtime_unrepresented: false,
                    receiver_fanout_unrepresented: false,
                },
            );
            return;
        }

        let Some(quest_pool_store) = self.quests.pool_store.as_ref().map(Arc::clone) else {
            self.record_represented_push_quest_to_party_outcome_like_cpp(
                RepresentedPushQuestToPartyOutcomeLikeCpp {
                    sender_guid,
                    quest_id: packet.quest_id,
                    target_guid: sender_guid,
                    reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::QuestPoolActiveCheckUnrepresented,
                    quest_pool_active_check_unrepresented: true,
                    group_runtime_unrepresented: false,
                    receiver_fanout_unrepresented: false,
                },
            );
            return;
        };

        if !quest_pool_store.is_quest_active_like_cpp(packet.quest_id) {
            self.send_push_quest_result_to_sender_if_available_like_cpp(
                sender_guid,
                quest_push_reason::NOT_DAILY,
            );
            self.record_represented_push_quest_to_party_outcome_like_cpp(
                RepresentedPushQuestToPartyOutcomeLikeCpp {
                    sender_guid,
                    quest_id: packet.quest_id,
                    target_guid: sender_guid,
                    reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::NotDaily,
                    quest_pool_active_check_unrepresented: false,
                    group_runtime_unrepresented: false,
                    receiver_fanout_unrepresented: false,
                },
            );
            return;
        }

        if self.resolved_group_guid_like_cpp().is_none() {
            self.send_push_quest_result_to_sender_if_available_like_cpp(
                sender_guid,
                quest_push_reason::NOT_IN_PARTY,
            );
            self.record_represented_push_quest_to_party_outcome_like_cpp(
                RepresentedPushQuestToPartyOutcomeLikeCpp {
                    sender_guid,
                    quest_id: packet.quest_id,
                    target_guid: sender_guid,
                    reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::NotInParty,
                    quest_pool_active_check_unrepresented: false,
                    group_runtime_unrepresented: false,
                    receiver_fanout_unrepresented: false,
                },
            );
            return;
        }

        let Some(group_guid) = self.resolved_group_guid_like_cpp() else {
            return;
        };

        let Some(group_registry) = self.group_registry().map(Arc::clone) else {
            self.record_represented_push_quest_to_party_outcome_like_cpp(
                RepresentedPushQuestToPartyOutcomeLikeCpp {
                    sender_guid,
                    quest_id: packet.quest_id,
                    target_guid: sender_guid,
                    reason:
                        RepresentedPushQuestToPartyOutcomeReasonLikeCpp::GroupRuntimeUnrepresented,
                    quest_pool_active_check_unrepresented: false,
                    group_runtime_unrepresented: true,
                    receiver_fanout_unrepresented: true,
                },
            );
            return;
        };

        let Some(player_registry) = self.player_registry().map(Arc::clone) else {
            self.record_represented_push_quest_to_party_outcome_like_cpp(
                RepresentedPushQuestToPartyOutcomeLikeCpp {
                    sender_guid,
                    quest_id: packet.quest_id,
                    target_guid: sender_guid,
                    reason:
                        RepresentedPushQuestToPartyOutcomeReasonLikeCpp::GroupRuntimeUnrepresented,
                    quest_pool_active_check_unrepresented: false,
                    group_runtime_unrepresented: true,
                    receiver_fanout_unrepresented: true,
                },
            );
            return;
        };

        let Some(group_info) = group_registry.get(&group_guid).map(|entry| entry.clone()) else {
            self.record_represented_push_quest_to_party_outcome_like_cpp(
                RepresentedPushQuestToPartyOutcomeLikeCpp {
                    sender_guid,
                    quest_id: packet.quest_id,
                    target_guid: sender_guid,
                    reason:
                        RepresentedPushQuestToPartyOutcomeReasonLikeCpp::GroupRuntimeUnrepresented,
                    quest_pool_active_check_unrepresented: false,
                    group_runtime_unrepresented: true,
                    receiver_fanout_unrepresented: true,
                },
            );
            return;
        };

        let canonical_map_manager = self.canonical_map_manager.clone();
        let receiver_snapshots = group_info
            .members
            .iter()
            .copied()
            .filter(|member_guid| Some(*member_guid) != sender_guid)
            .filter_map(|member_guid| {
                player_registry
                    .quest_sharing_snapshot(member_guid, canonical_map_manager.as_ref())
                    .map(|receiver| (member_guid, receiver))
            })
            .collect::<Vec<_>>();

        if receiver_snapshots.is_empty() {
            self.record_represented_push_quest_to_party_outcome_like_cpp(
                RepresentedPushQuestToPartyOutcomeLikeCpp {
                    sender_guid,
                    quest_id: packet.quest_id,
                    target_guid: sender_guid,
                    reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverEligibilityUnrepresented,
                    quest_pool_active_check_unrepresented: false,
                    group_runtime_unrepresented: false,
                    receiver_fanout_unrepresented: true,
                },
            );
            return;
        }

        let mut blocked_by_unsupported_success_path = false;
        for (receiver_guid, receiver) in receiver_snapshots {
            if self.quest_share_admission_handled(
                quest, packet.quest_id, sender_guid, receiver_guid,
                &receiver, &player_registry, &mut blocked_by_unsupported_success_path,
            ) {
                continue;
            }
            if self.quest_share_requirements_handled(
                quest, packet.quest_id, sender_guid, receiver_guid,
                &receiver, &player_registry, &mut blocked_by_unsupported_success_path,
                &quest_store,
            ) {
                continue;
            }

            if quest.is_turn_in_like_cpp()
                && quest.is_repeatable()
                && !quest.is_daily_or_weekly_like_cpp()
            {
                let Some(sender_guid_for_receiver_command) = sender_guid else {
                    blocked_by_unsupported_success_path = true;
                    self.record_represented_push_quest_to_party_outcome_like_cpp(
                        RepresentedPushQuestToPartyOutcomeLikeCpp {
                            sender_guid,
                            quest_id: packet.quest_id,
                            target_guid: Some(receiver_guid),
                            reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverRepeatableTurnInRequestItemsPromptCommandFailed,
                            quest_pool_active_check_unrepresented: false,
                            group_runtime_unrepresented: false,
                            receiver_fanout_unrepresented: true,
                        },
                    );
                    continue;
                };

                let command = SessionCommand::SendRepeatableTurnInRequestItemsLikeCpp(
                    SendRepeatableTurnInRequestItemsLikeCppCommand {
                        sender_guid: sender_guid_for_receiver_command,
                        quest: quest.clone(),
                    },
                );

                if player_registry
                    .try_send_current_command(receiver.registration, command)
                    .is_err()
                {
                    blocked_by_unsupported_success_path = true;
                    self.record_represented_push_quest_to_party_outcome_like_cpp(
                        RepresentedPushQuestToPartyOutcomeLikeCpp {
                            sender_guid,
                            quest_id: packet.quest_id,
                            target_guid: Some(receiver_guid),
                            reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverRepeatableTurnInRequestItemsPromptCommandFailed,
                            quest_pool_active_check_unrepresented: false,
                            group_runtime_unrepresented: false,
                            receiver_fanout_unrepresented: true,
                        },
                    );
                    continue;
                }

                // C++ `HandlePushQuestToParty` sends Success to the sender before the
                // repeatable turn-in `SendQuestGiverRequestItems` receiver side effect.
                // Rust has an extra fallible queue hop, so emit represented Success only
                // after the receiver command has been accepted.
                self.send_push_quest_result_to_sender_with_title_if_available_like_cpp(
                    receiver_guid,
                    QUEST_PUSH_REASON_SUCCESS_LIKE_CPP,
                    String::new(),
                );

                self.record_represented_push_quest_to_party_outcome_like_cpp(
                    RepresentedPushQuestToPartyOutcomeLikeCpp {
                        sender_guid,
                        quest_id: packet.quest_id,
                        target_guid: Some(receiver_guid),
                        reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverRepeatableTurnInRequestItemsPrompted,
                        quest_pool_active_check_unrepresented: false,
                        group_runtime_unrepresented: false,
                        receiver_fanout_unrepresented: false,
                    },
                );
                continue;
            }

            let Some(sender_guid_for_receiver_command) = sender_guid else {
                blocked_by_unsupported_success_path = true;
                self.record_represented_push_quest_to_party_outcome_like_cpp(
                    RepresentedPushQuestToPartyOutcomeLikeCpp {
                        sender_guid,
                        quest_id: packet.quest_id,
                        target_guid: Some(receiver_guid),
                        reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverQuestDetailsPromptCommandFailed,
                        quest_pool_active_check_unrepresented: false,
                        group_runtime_unrepresented: false,
                        receiver_fanout_unrepresented: true,
                    },
                );
                continue;
            };

            let command = SessionCommand::SetQuestSharingInfoAndSendDetails(
                SetQuestSharingInfoAndSendDetailsCommand {
                    sender_guid: sender_guid_for_receiver_command,
                    quest: quest.clone(),
                },
            );

            if player_registry
                .try_send_current_command(receiver.registration, command)
                .is_err()
            {
                blocked_by_unsupported_success_path = true;
                self.record_represented_push_quest_to_party_outcome_like_cpp(
                    RepresentedPushQuestToPartyOutcomeLikeCpp {
                        sender_guid,
                        quest_id: packet.quest_id,
                        target_guid: Some(receiver_guid),
                        reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverQuestDetailsPromptCommandFailed,
                        quest_pool_active_check_unrepresented: false,
                        group_runtime_unrepresented: false,
                        receiver_fanout_unrepresented: true,
                    },
                );
                continue;
            }

            self.send_push_quest_result_to_sender_with_title_if_available_like_cpp(
                receiver_guid,
                QUEST_PUSH_REASON_SUCCESS_LIKE_CPP,
                String::new(),
            );
            self.record_represented_push_quest_to_party_outcome_like_cpp(
                RepresentedPushQuestToPartyOutcomeLikeCpp {
                    sender_guid,
                    quest_id: packet.quest_id,
                    target_guid: Some(receiver_guid),
                    reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSuccessQuestDetailsPrompted,
                    quest_pool_active_check_unrepresented: false,
                    group_runtime_unrepresented: false,
                    receiver_fanout_unrepresented: false,
                },
            );
        }

        if blocked_by_unsupported_success_path {
            self.record_represented_push_quest_to_party_outcome_like_cpp(
                RepresentedPushQuestToPartyOutcomeLikeCpp {
                    sender_guid,
                    quest_id: packet.quest_id,
                    target_guid: sender_guid,
                    reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverEligibilityUnrepresented,
                    quest_pool_active_check_unrepresented: false,
                    group_runtime_unrepresented: false,
                    receiver_fanout_unrepresented: true,
                },
            );
        }
    }
}
