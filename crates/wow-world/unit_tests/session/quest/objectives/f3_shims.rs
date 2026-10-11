// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    #[cfg(test)]
    pub(crate) fn represented_quest_push_result_sender_mismatch_count_like_cpp(&self) -> u32 {
        self.quest_state
            .represented_quest_push_result_sender_mismatch_count_like_cpp()
    }
}

impl WorldSession {
    pub(in crate::session) async fn update_represented_storing_flag_quest_objective_progress_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        objective_type: u8,
        object_id: i32,
        add_count: i32,
    ) {
        self.invalidate_player_quest_status_authority_like_cpp();
        use wow_packet::packets::quest::{QuestUpdateAddCreditSimple, QuestUpdateComplete};

        let Some(store) = self.catalogs.quests.store.clone() else {
            return;
        };

        let Some(quests) = self.player_quest_gameplay_snapshot_like_cpp() else {
            return;
        };
        let matching: Vec<(u32, usize, u32)> = quests
            .statuses_like_cpp()
            .values()
            .filter(|qs| qs.status == wow_conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP)
            .flat_map(|qs| {
                let quest = store.get(qs.quest_id)?;
                Some(quest.objectives.iter().filter_map(move |obj| {
                    if obj.obj_type != objective_type
                        || obj.object_id != object_id
                        || !obj.is_storing_flag_like_cpp()
                    {
                        return None;
                    }
                    let Ok(idx) = usize::try_from(obj.storage_index) else {
                        return None;
                    };
                    Some((qs.quest_id, idx, obj.id))
                }))
            })
            .flatten()
            .collect();

        let mut quests_to_complete = Vec::new();
        let mut quests_to_save = Vec::new();
        for (quest_id, obj_idx, objective_id) in matching {
            let Some((objective_was_complete, objective_is_now_complete)) = self
                .mutate_player_quest_gameplay_like_cpp(|quests| {
                    let qs = quests.status_mut_like_cpp(quest_id)?;
                    if qs.objective_counts.len() <= obj_idx {
                        qs.objective_counts.resize(obj_idx + 1, 0);
                    }
                    let before = qs.objective_counts[obj_idx] != 0;
                    qs.objective_counts[obj_idx] = i32::from(add_count > 0);
                    Some((before, qs.objective_counts[obj_idx] != 0))
                })
                .flatten()
            else {
                continue;
            };
            if objective_was_complete != objective_is_now_complete {
                quests_to_save.push(quest_id);
            }

            if add_count > 0 {
                self.send_packet(&QuestUpdateAddCreditSimple {
                    quest_id,
                    object_id,
                    objective_type,
                });
            }

            if !objective_was_complete && objective_is_now_complete {
                if let (Some(quest), Some(quests)) = (
                    store.get(quest_id),
                    self.player_quest_gameplay_snapshot_like_cpp(),
                ) {
                    let quest_already_rewarded =
                        quests.rewarded_quest_ids_like_cpp().contains(&quest_id);
                    if quests
                        .statuses_like_cpp()
                        .get(&quest_id)
                        .is_some_and(|status| {
                            wow_entities::represented_can_complete_quest_after_objective_like_cpp(
                                status,
                                &quest.objective_rules_like_cpp(),
                                objective_id,
                                quest_already_rewarded,
                            )
                        })
                    {
                        quests_to_complete.push((quest_id, objective_id));
                    }
                }
            }
        }

        for (quest_id, objective_id) in quests_to_complete {
            let Some(quest) = store.get(quest_id).cloned() else {
                continue;
            };
            let completed = self
                .complete_represented_quest_after_objective_with_generator_like_cpp(
                    item_guid_generator,
                    &quest,
                    objective_id,
                )
                .await;
            if completed {
                if self.represented_player_quest_status_like_cpp(quest_id)
                    == Some(Some(wow_conditions::QUEST_STATUS_COMPLETE_LIKE_CPP))
                {
                    self.send_packet(&QuestUpdateComplete { quest_id });
                }
                info!(
                    account = self.core.account_id,
                    quest_id, "Quest objectives complete"
                );
            }
        }
        self.save_changed_represented_quest_statuses_like_cpp(&mut quests_to_save)
            .await;
        self.sync_player_registry_state_like_cpp();
    }
}
