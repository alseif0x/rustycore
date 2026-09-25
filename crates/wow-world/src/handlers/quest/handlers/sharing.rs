// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Sender-side quest sharing packet flow.

use super::*;

impl WorldSession {
    pub(crate) fn handle_send_repeatable_turn_in_request_items_command_like_cpp(
        &mut self,
        command: SendRepeatableTurnInRequestItemsLikeCppCommand,
    ) {
        self.send_repeatable_turn_in_request_items_like_cpp(command.sender_guid, &command.quest);
    }

    pub(crate) fn handle_set_quest_sharing_info_and_send_details_command_like_cpp(
        &mut self,
        command: SetQuestSharingInfoAndSendDetailsCommand,
    ) {
        let Some(receiver_guid) = self.player_guid() else {
            return;
        };

        self.set_represented_pending_quest_sharing_like_cpp(command.sender_guid, command.quest.id);
        self.send_represented_quest_giver_quest_details_like_cpp(
            receiver_guid,
            &command.quest,
            false,
        );
    }

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
            if receiver.pending_quest_sharing.is_some() {
                self.send_push_quest_result_to_sender_with_title_if_available_like_cpp(
                    receiver_guid,
                    QUEST_PUSH_REASON_BUSY_LIKE_CPP,
                    String::new(),
                );
                self.record_represented_push_quest_to_party_outcome_like_cpp(
                    RepresentedPushQuestToPartyOutcomeLikeCpp {
                        sender_guid,
                        quest_id: packet.quest_id,
                        target_guid: Some(receiver_guid),
                        reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverBusy,
                        quest_pool_active_check_unrepresented: false,
                        group_runtime_unrepresented: false,
                        receiver_fanout_unrepresented: false,
                    },
                );
                continue;
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
                        quest_id: packet.quest_id,
                        target_guid: Some(receiver_guid),
                        reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverDead,
                        quest_pool_active_check_unrepresented: false,
                        group_runtime_unrepresented: false,
                        receiver_fanout_unrepresented: false,
                    },
                );
                continue;
            }

            if receiver.rewarded_quests.contains(&packet.quest_id) {
                let Some(sender_guid_for_receiver_packet) = sender_guid else {
                    blocked_by_unsupported_success_path = true;
                    continue;
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
                        quest_id: packet.quest_id,
                        target_guid: Some(receiver_guid),
                        reason:
                            RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverAlreadyDone,
                        quest_pool_active_check_unrepresented: false,
                        group_runtime_unrepresented: false,
                        receiver_fanout_unrepresented: false,
                    },
                );
                continue;
            }

            if let Some(status) = receiver
                .active_quest_statuses
                .get(&packet.quest_id)
                .copied()
            {
                let Some(sender_guid_for_receiver_packet) = sender_guid else {
                    blocked_by_unsupported_success_path = true;
                    continue;
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
                            quest_id: packet.quest_id,
                            target_guid: Some(receiver_guid),
                            reason:
                                RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverOnQuest,
                            quest_pool_active_check_unrepresented: false,
                            group_runtime_unrepresented: false,
                            receiver_fanout_unrepresented: false,
                        },
                    );
                    continue;
                }
            }

            // C++ `Player::SatisfyQuestLog(false)` checks `FindQuestSlot(0) <
            // MAX_QUEST_LOG_SIZE`; this represented cross-session seam uses
            // the receiver snapshot derived from the canonical Player quest
            // slots via `sync_player_registry_state_like_cpp()`.
            if receiver.active_quest_statuses.len() >= MAX_QUEST_LOG_SIZE_LIKE_CPP as usize {
                let Some(sender_guid_for_receiver_packet) = sender_guid else {
                    blocked_by_unsupported_success_path = true;
                    continue;
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
                        quest_id: packet.quest_id,
                        target_guid: Some(receiver_guid),
                        reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverLogFull,
                        quest_pool_active_check_unrepresented: false,
                        group_runtime_unrepresented: false,
                        receiver_fanout_unrepresented: false,
                    },
                );
                continue;
            }

            // C++ `Player::SatisfyQuestDay(quest, false)` immediately follows
            // `SatisfyQuestLog(false)` in `WorldSession::HandlePushQuestToParty`.
            // Non-daily/non-DF quests pass this gate; already-completed daily
            // quests and represented DF quests send the same AlreadyDone pair
            // as the earlier rewarded/onquest branch.
            let already_satisfied_quest_day_like_cpp = if quest.is_df_quest_like_cpp() {
                receiver.df_quests.contains(&packet.quest_id)
            } else if quest.is_daily_like_cpp() {
                receiver.daily_quests_completed.contains(&packet.quest_id)
            } else {
                false
            };

            if already_satisfied_quest_day_like_cpp {
                let Some(sender_guid_for_receiver_packet) = sender_guid else {
                    blocked_by_unsupported_success_path = true;
                    continue;
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
                        quest_id: packet.quest_id,
                        target_guid: Some(receiver_guid),
                        reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestDayAlreadyDone,
                        quest_pool_active_check_unrepresented: false,
                        group_runtime_unrepresented: false,
                        receiver_fanout_unrepresented: false,
                    },
                );
                continue;
            }

            // C++ then evaluates `Player::SatisfyQuestMinLevel(quest, false)`
            // followed by `SatisfyQuestMaxLevel(quest, false)`.  Receiver
            // `level` is a derived cross-session snapshot synchronized from
            // the receiver `WorldSession`, never source-of-truth in reverse.
            if quest.min_level > 0 && i32::from(receiver.level) < quest.min_level {
                let Some(sender_guid_for_receiver_packet) = sender_guid else {
                    blocked_by_unsupported_success_path = true;
                    continue;
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
                        quest_id: packet.quest_id,
                        target_guid: Some(receiver_guid),
                        reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestMinLevelLowLevel,
                        quest_pool_active_check_unrepresented: false,
                        group_runtime_unrepresented: false,
                        receiver_fanout_unrepresented: false,
                    },
                );
                continue;
            }

            if quest.max_level > 0 && receiver.level > quest.max_level {
                let Some(sender_guid_for_receiver_packet) = sender_guid else {
                    blocked_by_unsupported_success_path = true;
                    continue;
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
                        quest_id: packet.quest_id,
                        target_guid: Some(receiver_guid),
                        reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestMaxLevelHighLevel,
                        quest_pool_active_check_unrepresented: false,
                        group_runtime_unrepresented: false,
                        receiver_fanout_unrepresented: false,
                    },
                );
                continue;
            }

            // C++ order then evaluates `Player::SatisfyQuestClass(quest, false)`
            // followed by `SatisfyQuestRace(quest, false)`. Receiver class/race
            // are read-only `PlayerRegistry` snapshots derived from the receiver
            // `WorldSession`; never sync registry state back into the session.
            let receiver_class_mask = player_race_or_class_mask_like_cpp(receiver.class);
            if quest.allowable_classes != 0 && (quest.allowable_classes & receiver_class_mask) == 0
            {
                let Some(sender_guid_for_receiver_packet) = sender_guid else {
                    blocked_by_unsupported_success_path = true;
                    continue;
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
                        quest_id: packet.quest_id,
                        target_guid: Some(receiver_guid),
                        reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestClassWrongClass,
                        quest_pool_active_check_unrepresented: false,
                        group_runtime_unrepresented: false,
                        receiver_fanout_unrepresented: false,
                    },
                );
                continue;
            }

            let receiver_race_mask = u64::from(player_race_or_class_mask_like_cpp(receiver.race));
            if quest.allowable_races != 0 && (quest.allowable_races & receiver_race_mask) == 0 {
                let Some(sender_guid_for_receiver_packet) = sender_guid else {
                    blocked_by_unsupported_success_path = true;
                    continue;
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
                        quest_id: packet.quest_id,
                        target_guid: Some(receiver_guid),
                        reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestRaceWrongRace,
                        quest_pool_active_check_unrepresented: false,
                        group_runtime_unrepresented: false,
                        receiver_fanout_unrepresented: false,
                    },
                );
                continue;
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
                        quest_id: packet.quest_id,
                        target_guid: Some(receiver_guid),
                        reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverEligibilityUnrepresented,
                        quest_pool_active_check_unrepresented: false,
                        group_runtime_unrepresented: false,
                        receiver_fanout_unrepresented: false,
                    },
                );
                continue;
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
                    blocked_by_unsupported_success_path = true;
                    continue;
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
                        quest_id: packet.quest_id,
                        target_guid: Some(receiver_guid),
                        reason,
                        quest_pool_active_check_unrepresented: false,
                        group_runtime_unrepresented: false,
                        receiver_fanout_unrepresented: false,
                    },
                );
                continue;
            }

            // C++ `Player::SatisfyQuestDependentQuests(quest, false)` preserves this
            // sub-gate order: PreviousQuest, DependentPreviousQuests,
            // BreadcrumbQuest, DependentBreadcrumbQuests. Expansion, CanTakeQuest,
            // success fanout, SetQuestSharingInfo, details, and auto-accept stay
            // deliberately unsupported after these represented prerequisites.
            let previous_quest_prerequisite_failed = if quest.prev_quest_id > 0 {
                !receiver
                    .rewarded_quests
                    .contains(&quest.prev_quest_id.unsigned_abs())
            } else if quest.prev_quest_id < 0 {
                receiver
                    .active_quest_statuses
                    .get(&quest.prev_quest_id.unsigned_abs())
                    .copied()
                    != Some(QUEST_STATUS_INCOMPLETE_LIKE_CPP)
            } else {
                false
            };

            let mut prerequisite_failure_reason =
                previous_quest_prerequisite_failed.then_some(
                    RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestPreviousQuestPrerequisite,
                );

            if prerequisite_failure_reason.is_none()
                && represented_satisfy_quest_dependent_previous_quests_failed_like_cpp(
                    &quest_store,
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
                blocked_by_unsupported_success_path = true;
                self.record_represented_push_quest_to_party_outcome_like_cpp(
                    RepresentedPushQuestToPartyOutcomeLikeCpp {
                        sender_guid,
                        quest_id: packet.quest_id,
                        target_guid: Some(receiver_guid),
                        reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverEligibilityUnrepresented,
                        quest_pool_active_check_unrepresented: false,
                        group_runtime_unrepresented: false,
                        receiver_fanout_unrepresented: true,
                    },
                );
                continue;
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
                    blocked_by_unsupported_success_path = true;
                    continue;
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
                        quest_id: packet.quest_id,
                        target_guid: Some(receiver_guid),
                        reason,
                        quest_pool_active_check_unrepresented: false,
                        group_runtime_unrepresented: false,
                        receiver_fanout_unrepresented: false,
                    },
                );
                continue;
            }

            if i32::from(receiver.active_expansion) < quest.expansion {
                let Some(sender_guid_for_receiver_packet) = sender_guid else {
                    blocked_by_unsupported_success_path = true;
                    continue;
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
                        quest_id: packet.quest_id,
                        target_guid: Some(receiver_guid),
                        reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestExpansionRequiredExpansion,
                        quest_pool_active_check_unrepresented: false,
                        group_runtime_unrepresented: false,
                        receiver_fanout_unrepresented: false,
                    },
                );
                continue;
            }

            if !represented_can_take_quest_after_expansion_like_cpp(&quest_store, quest, &receiver)
            {
                let Some(sender_guid_for_receiver_packet) = sender_guid else {
                    blocked_by_unsupported_success_path = true;
                    continue;
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
                        quest_id: packet.quest_id,
                        target_guid: Some(receiver_guid),
                        reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverCanTakeQuestInvalid,
                        quest_pool_active_check_unrepresented: false,
                        group_runtime_unrepresented: false,
                        receiver_fanout_unrepresented: false,
                    },
                );
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
