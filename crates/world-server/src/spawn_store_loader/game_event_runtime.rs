//! Game-event state transitions and caches over canonical spawn metadata.
//!
//! The implementation remains an inherent impl on the parent type so the parent
//! remains the single owner of all state and its public API is unchanged.

use super::*;

impl CanonicalSpawnMetadataLikeCpp {
    pub fn clear_active_game_events_like_cpp(&mut self) {
        self.game_event_active_set.clear_active_events_like_cpp();
    }
    pub fn represented_handle_game_event_quest_complete_like_cpp(
        &mut self,
        quest_id: u32,
        current_time_secs: u64,
    ) -> GameEventQuestCompleteOutcomeLikeCpp {
        let Some(record) = self
            .game_event_quest_conditions_by_quest
            .get(&quest_id)
            .copied()
        else {
            return GameEventQuestCompleteOutcomeLikeCpp::MissingQuestMapping { quest_id };
        };
        GameEventQuestCompleteOutcomeLikeCpp::Progress(
            self.represented_update_game_event_condition_progress_like_cpp(
                record.event_id,
                record.condition_id,
                record.num,
                current_time_secs,
            ),
        )
    }
    pub fn represented_update_game_event_condition_progress_like_cpp(
        &mut self,
        event_id: u16,
        condition_id: u32,
        num: f32,
        current_time_secs: u64,
    ) -> GameEventConditionProgressOutcomeLikeCpp {
        let Some(event) = self.game_events.event_like_cpp(event_id) else {
            return GameEventConditionProgressOutcomeLikeCpp::MissingEvent { event_id };
        };
        if !self
            .game_event_active_set
            .is_active_event_like_cpp(event_id)
        {
            return GameEventConditionProgressOutcomeLikeCpp::InactiveEvent { event_id };
        }
        if event.state_raw != GameEventStateLikeCpp::WorldConditions as u8 {
            return GameEventConditionProgressOutcomeLikeCpp::NotWorldConditions {
                event_id,
                state_raw: event.state_raw,
            };
        }
        let Some(condition) = event.conditions.get(&condition_id).copied() else {
            return GameEventConditionProgressOutcomeLikeCpp::MissingCondition {
                event_id,
                condition_id,
            };
        };
        if condition.done >= condition.req_num {
            return GameEventConditionProgressOutcomeLikeCpp::AlreadyComplete {
                event_id,
                condition_id,
                done: condition.done,
                req_num: condition.req_num,
            };
        }
        let done_before = condition.done;
        let done_after = (condition.done + num).min(condition.req_num);
        if let Some(event) = self.game_events.event_mut_like_cpp(event_id) {
            if let Some(condition) = event.conditions.get_mut(&condition_id) {
                condition.done = done_after;
            }
        }
        let check_outcome = self
            .game_events
            .check_one_game_event_conditions_like_cpp(event_id, current_time_secs);
        let completed_event = matches!(
            check_outcome,
            GameEventConditionCheckOutcomeLikeCpp::Completed(_)
        );
        let event_id_param = u8::try_from(event_id & 0x00ff).unwrap_or(0);
        GameEventConditionProgressOutcomeLikeCpp::Progressed(
            GameEventConditionProgressSummaryLikeCpp {
                event_id,
                condition_id,
                done_before,
                done_after,
                req_num: condition.req_num,
                persistence_event_id: event_id_param,
                completed_event,
                check_outcome,
                save_world_event_state_requested: completed_event,
                force_game_event_update_requested: completed_event,
            },
        )
    }
    pub fn start_game_event_like_cpp(
        &mut self,
        event_id: u16,
        overwrite: bool,
        current_time_secs: u64,
        world_conditions_met: bool,
    ) -> GameEventStartOutcomeLikeCpp {
        let Some(event) = self.game_events.event_mut_like_cpp(event_id) else {
            return GameEventStartOutcomeLikeCpp::MissingEvent { event_id };
        };
        let state_before_raw = event.state_raw;
        let normal_or_internal = state_before_raw == GameEventStateLikeCpp::Normal as u8
            || state_before_raw == GameEventStateLikeCpp::Internal as u8;
        if normal_or_internal {
            let active_added = self
                .game_event_active_set
                .add_active_event_like_cpp(event_id);
            if overwrite {
                event.start = current_time_secs;
                if event.end <= event.start {
                    event.end = event.start.saturating_add(u64::from(event.length));
                }
            }
            return GameEventStartOutcomeLikeCpp::Started(GameEventStartSummaryLikeCpp {
                event_id,
                state_before_raw,
                state_after_raw: event.state_raw,
                active_added,
                active_was_present: !active_added,
                apply_new_event_requested: true,
                save_world_event_state_requested: false,
                force_game_event_update_requested: false,
                completed: false,
            });
        }

        if event.state_raw == GameEventStateLikeCpp::WorldInactive as u8 {
            event.state_raw = GameEventStateLikeCpp::WorldConditions as u8;
        }

        let active_added = self
            .game_event_active_set
            .add_active_event_like_cpp(event_id);
        if world_conditions_met {
            event.state_raw = GameEventStateLikeCpp::WorldNextPhase as u8;
            if event.next_start == 0 {
                event.next_start = current_time_secs.saturating_add(
                    u64::from(event.length).saturating_mul(GAME_EVENT_MINUTE_SECS_LIKE_CPP),
                );
            }
        }

        GameEventStartOutcomeLikeCpp::Started(GameEventStartSummaryLikeCpp {
            event_id,
            state_before_raw,
            state_after_raw: event.state_raw,
            active_added,
            active_was_present: !active_added,
            apply_new_event_requested: true,
            save_world_event_state_requested: true,
            force_game_event_update_requested: overwrite && world_conditions_met,
            completed: world_conditions_met,
        })
    }

    pub fn stop_game_event_like_cpp(
        &mut self,
        event_id: u16,
        overwrite: bool,
        current_time_secs: u64,
    ) -> GameEventStopOutcomeLikeCpp {
        let Some(event) = self.game_events.event_mut_like_cpp(event_id) else {
            return GameEventStopOutcomeLikeCpp::MissingEvent { event_id };
        };

        let state_before_raw = event.state_raw;
        let serverwide = state_before_raw != GameEventStateLikeCpp::Normal as u8
            && state_before_raw != GameEventStateLikeCpp::Internal as u8;
        let active_removed = self
            .game_event_active_set
            .remove_active_event_like_cpp(event_id);
        let mut condition_reset_requested = false;
        let mut delete_world_event_state_requested = false;
        let mut delete_condition_saves_requested = false;

        if overwrite && !serverwide {
            event.start = current_time_secs.saturating_sub(
                u64::from(event.length).saturating_mul(GAME_EVENT_MINUTE_SECS_LIKE_CPP),
            );
            if event.end <= event.start {
                event.end = event.start.saturating_add(u64::from(event.length));
            }
        } else if serverwide
            && (overwrite || state_before_raw != GameEventStateLikeCpp::WorldFinished as u8)
        {
            event.next_start = 0;
            event.state_raw = GameEventStateLikeCpp::WorldInactive as u8;
            condition_reset_requested = true;
            delete_world_event_state_requested = true;
            delete_condition_saves_requested = true;
        }

        GameEventStopOutcomeLikeCpp::Stopped(GameEventStopSummaryLikeCpp {
            event_id,
            state_before_raw,
            state_after_raw: event.state_raw,
            active_removed,
            active_was_present: active_removed,
            unapply_event_requested: true,
            serverwide,
            condition_reset_requested,
            delete_world_event_state_requested,
            delete_condition_saves_requested,
        })
    }

    pub fn update_game_events_like_cpp<F>(
        &mut self,
        current_time_secs: u64,
        is_system_init: bool,
        mut world_conditions_met: F,
    ) -> GameEventUpdateOutcomeLikeCpp
    where
        F: FnMut(u16) -> bool,
    {
        let mut scanned_event_ids = Vec::new();
        let mut check_outcomes = Vec::new();
        let mut next_check_outcomes = Vec::new();
        let mut activate = BTreeSet::new();
        let mut deactivate = BTreeSet::new();
        let mut negative_spawn_event_ids = Vec::new();
        let mut world_nextphase_finished = Vec::new();
        let mut world_conditions_save_requested = Vec::new();
        let mut invalid_check_outcomes = Vec::new();
        let mut invalid_next_check_outcomes = Vec::new();
        let mut start_conditions_met = BTreeMap::new();
        let mut next_event_delay_secs = MAX_GAME_EVENT_CHECK_DELAY_SECS_LIKE_CPP;

        for event_index in 1..self.game_events.len_like_cpp() {
            let Ok(event_id) = u16::try_from(event_index) else {
                continue;
            };
            scanned_event_ids.push(event_id);

            let check_outcome = self
                .game_events
                .check_one_game_event_like_cpp(event_id, current_time_secs);
            check_outcomes.push((event_id, check_outcome));

            match check_outcome {
                GameEventCheckOutcomeLikeCpp::Active(true) => {
                    let active_before_queue = self
                        .game_event_active_set
                        .is_active_event_like_cpp(event_id);

                    let mut nextphase_finished = false;
                    if let Some(event) = self.game_events.event_mut_like_cpp(event_id) {
                        if event.state_raw == GameEventStateLikeCpp::WorldNextPhase as u8
                            && event.next_start <= current_time_secs
                        {
                            let state_before_raw = event.state_raw;
                            let next_start_before = event.next_start;
                            event.state_raw = GameEventStateLikeCpp::WorldFinished as u8;
                            event.next_start = 0;
                            world_nextphase_finished.push(GameEventWorldNextPhaseFinishedLikeCpp {
                                event_id,
                                was_active_before_queue: active_before_queue,
                                state_before_raw,
                                state_after_raw: event.state_raw,
                                next_start_before,
                                next_start_after: event.next_start,
                                save_state_requested: true,
                            });
                            if active_before_queue {
                                deactivate.insert(event_id);
                            }
                            nextphase_finished = true;
                        }
                    }
                    if nextphase_finished {
                        continue;
                    }

                    let mut condition_met_for_start = false;
                    let mut condition_checked_during_scan = false;
                    if let Some(event) = self.game_events.event_mut_like_cpp(event_id) {
                        if event.state_raw == GameEventStateLikeCpp::WorldConditions as u8 {
                            condition_checked_during_scan = true;
                            if world_conditions_met(event_id) {
                                event.state_raw = GameEventStateLikeCpp::WorldNextPhase as u8;
                                if event.next_start == 0 {
                                    event.next_start = current_time_secs.saturating_add(
                                        u64::from(event.length)
                                            .saturating_mul(GAME_EVENT_MINUTE_SECS_LIKE_CPP),
                                    );
                                }
                                world_conditions_save_requested.push(
                                    GameEventWorldStateSaveEvidenceLikeCpp {
                                        event_id,
                                        state_after_raw: event.state_raw,
                                        next_start_after: event.next_start,
                                    },
                                );
                                condition_met_for_start = true;
                            }
                        }
                    }
                    if condition_checked_during_scan {
                        start_conditions_met.insert(event_id, condition_met_for_start);
                    }

                    if !active_before_queue {
                        activate.insert(event_id);
                    }
                }
                GameEventCheckOutcomeLikeCpp::Active(false) => {
                    if self
                        .game_event_active_set
                        .is_active_event_like_cpp(event_id)
                    {
                        deactivate.insert(event_id);
                    } else if !is_system_init {
                        negative_spawn_event_ids.push(-i16::try_from(event_id).unwrap_or(i16::MAX));
                    }
                }
                invalid @ (GameEventCheckOutcomeLikeCpp::MissingEvent { .. }
                | GameEventCheckOutcomeLikeCpp::MissingPrerequisite { .. }
                | GameEventCheckOutcomeLikeCpp::InvalidTimingZeroOccurrence { .. }) => {
                    invalid_check_outcomes.push(invalid);
                    continue;
                }
            }

            let next_check_outcome = self
                .game_events
                .next_check_like_cpp(event_id, current_time_secs);
            next_check_outcomes.push((event_id, next_check_outcome));
            match next_check_outcome {
                GameEventNextCheckOutcomeLikeCpp::DelaySecs(delay_secs) => {
                    next_event_delay_secs = next_event_delay_secs.min(delay_secs);
                }
                invalid @ (GameEventNextCheckOutcomeLikeCpp::MissingEvent { .. }
                | GameEventNextCheckOutcomeLikeCpp::InvalidTimingZeroOccurrence {
                    ..
                }) => {
                    invalid_next_check_outcomes.push(invalid);
                }
            }
        }

        let queued_activation_event_ids = activate.iter().copied().collect::<Vec<_>>();
        let queued_deactivation_event_ids = deactivate.iter().copied().collect::<Vec<_>>();

        let mut start_outcomes = Vec::new();
        for event_id in queued_activation_event_ids.iter().copied() {
            let start_outcome = self.start_game_event_like_cpp(
                event_id,
                false,
                current_time_secs,
                start_conditions_met
                    .get(&event_id)
                    .copied()
                    .unwrap_or_else(|| world_conditions_met(event_id)),
            );
            if matches!(
                start_outcome,
                GameEventStartOutcomeLikeCpp::Started(GameEventStartSummaryLikeCpp {
                    completed: true,
                    ..
                })
            ) {
                next_event_delay_secs = 0;
            }
            start_outcomes.push(start_outcome);
        }

        let mut stop_outcomes = Vec::new();
        for event_id in queued_deactivation_event_ids.iter().copied() {
            stop_outcomes.push(self.stop_game_event_like_cpp(event_id, false, current_time_secs));
        }

        GameEventUpdateOutcomeLikeCpp {
            current_time_secs,
            scanned_event_ids,
            check_outcomes,
            next_check_outcomes,
            queued_activation_event_ids,
            queued_deactivation_event_ids,
            start_outcomes,
            stop_outcomes,
            negative_spawn_event_ids,
            world_nextphase_finished,
            world_conditions_save_requested,
            invalid_check_outcomes,
            invalid_next_check_outcomes,
            next_event_delay_secs_before_padding: next_event_delay_secs,
            next_update_delay_millis: next_event_delay_secs
                .saturating_add(1)
                .saturating_mul(1_000),
        }
    }

    #[allow(dead_code)]
    pub fn game_event_like_cpp(&self, event_id: u16) -> Option<&GameEventDataLikeCpp> {
        self.game_events.event_like_cpp(event_id)
    }

    pub fn game_event_last_start_time_like_cpp(
        &self,
        event_id: u16,
        current_time_secs: u64,
    ) -> u64 {
        self.game_events
            .last_start_time_like_cpp(event_id, current_time_secs)
    }

    pub fn game_event_pool_ids_like_cpp(&self, event_id: i16) -> Option<&[u32]> {
        self.game_event_pools.pool_ids_like_cpp(event_id)
    }

    pub fn game_event_creature_guids_like_cpp(&self, event_id: i16) -> Option<&[SpawnId]> {
        self.game_event_spawn_guids
            .creature_guids_like_cpp(event_id)
    }

    pub fn game_event_gameobject_guids_like_cpp(&self, event_id: i16) -> Option<&[SpawnId]> {
        self.game_event_spawn_guids
            .gameobject_guids_like_cpp(event_id)
    }

    pub fn game_event_model_equip_like_cpp(
        &self,
        event_id: u16,
    ) -> Option<&[GameEventModelEquipRecordLikeCpp]> {
        self.game_event_model_equip.records_like_cpp(event_id)
    }

    #[allow(dead_code)]
    pub fn game_event_npc_flags_like_cpp(
        &self,
        event_id: u16,
    ) -> Option<&[GameEventNpcFlagRecordLikeCpp]> {
        self.game_event_npc_flags.records_like_cpp(event_id)
    }

    #[allow(dead_code)]
    pub fn game_event_creature_quests_like_cpp(
        &self,
        event_id: u16,
    ) -> Option<&[GameEventQuestRelationRecordLikeCpp]> {
        self.game_event_quest_relations
            .creature_records_like_cpp(event_id)
    }

    #[allow(dead_code)]
    pub fn game_event_gameobject_quests_like_cpp(
        &self,
        event_id: u16,
    ) -> Option<&[GameEventQuestRelationRecordLikeCpp]> {
        self.game_event_quest_relations
            .gameobject_records_like_cpp(event_id)
    }

    #[allow(dead_code)]
    pub fn game_event_npc_vendors_like_cpp(
        &self,
        event_id: u16,
    ) -> Option<&[GameEventNpcVendorRecordLikeCpp]> {
        self.game_event_npc_vendors.records_like_cpp(event_id)
    }

    #[allow(dead_code)]
    pub fn game_event_npc_vendor_records_for_entry_like_cpp(
        &self,
        event_id: u16,
        entry: u32,
    ) -> Option<Vec<&GameEventNpcVendorRecordLikeCpp>> {
        self.game_event_npc_vendors
            .records_for_entry_like_cpp(event_id, entry)
    }

    pub fn game_event_active_npc_vendor_items_like_cpp(
        &self,
        entry: u32,
    ) -> &[GameEventNpcVendorRecordLikeCpp] {
        self.game_event_vendor_cache_by_entry
            .get(&entry)
            .map_or(&[], Vec::as_slice)
    }

    pub fn game_event_active_creature_quest_relations_like_cpp(
        &self,
        giver_id: u32,
    ) -> &[GameEventQuestRelationRecordLikeCpp] {
        self.game_event_active_creature_quest_relations_by_giver
            .get(&giver_id)
            .map_or(&[], Vec::as_slice)
    }

    pub fn game_event_active_gameobject_quest_relations_like_cpp(
        &self,
        giver_id: u32,
    ) -> &[GameEventQuestRelationRecordLikeCpp] {
        self.game_event_active_gameobject_quest_relations_by_giver
            .get(&giver_id)
            .map_or(&[], Vec::as_slice)
    }

    fn has_creature_quest_active_event_except_like_cpp(
        &self,
        quest_id: u32,
        event_id: u16,
    ) -> bool {
        self.game_event_active_set
            .active_event_ids_like_cpp()
            .filter(|active_event_id| *active_event_id != event_id)
            .any(|active_event_id| {
                self.game_event_quest_relations
                    .creature_records_like_cpp(active_event_id)
                    .is_some_and(|records| records.iter().any(|record| record.quest_id == quest_id))
            })
    }

    fn has_gameobject_quest_active_event_except_like_cpp(
        &self,
        quest_id: u32,
        event_id: u16,
    ) -> bool {
        self.game_event_active_set
            .active_event_ids_like_cpp()
            .filter(|active_event_id| *active_event_id != event_id)
            .any(|active_event_id| {
                self.game_event_quest_relations
                    .gameobject_records_like_cpp(active_event_id)
                    .is_some_and(|records| records.iter().any(|record| record.quest_id == quest_id))
            })
    }

    pub fn update_game_event_quest_relation_cache_like_cpp(
        &mut self,
        event_id: u16,
        activate: bool,
    ) -> GameEventQuestRelationCacheUpdateSummaryLikeCpp {
        let mut summary = GameEventQuestRelationCacheUpdateSummaryLikeCpp {
            event_id,
            activate,
            ..GameEventQuestRelationCacheUpdateSummaryLikeCpp::default()
        };

        match self
            .game_event_quest_relations
            .creature_records_like_cpp(event_id)
            .map(<[_]>::to_vec)
        {
            Some(records) => self.update_game_event_creature_quest_relation_cache_records_like_cpp(
                event_id,
                activate,
                &records,
                &mut summary,
            ),
            None => summary.creature_missing_event_bucket = true,
        }

        match self
            .game_event_quest_relations
            .gameobject_records_like_cpp(event_id)
            .map(<[_]>::to_vec)
        {
            Some(records) => self
                .update_game_event_gameobject_quest_relation_cache_records_like_cpp(
                    event_id,
                    activate,
                    &records,
                    &mut summary,
                ),
            None => summary.gameobject_missing_event_bucket = true,
        }

        summary
    }

    fn update_game_event_creature_quest_relation_cache_records_like_cpp(
        &mut self,
        event_id: u16,
        activate: bool,
        records: &[GameEventQuestRelationRecordLikeCpp],
        summary: &mut GameEventQuestRelationCacheUpdateSummaryLikeCpp,
    ) {
        summary.creature_records_seen = records.len();
        if activate {
            for record in records {
                self.game_event_active_creature_quest_relations_by_giver
                    .entry(record.giver_id)
                    .or_default()
                    .push(*record);
                summary.creature_inserted += 1;
            }
            return;
        }

        for record in records {
            if self.has_creature_quest_active_event_except_like_cpp(record.quest_id, event_id) {
                summary.creature_skipped_active_other_event += 1;
                continue;
            }
            let Some(active_records) = self
                .game_event_active_creature_quest_relations_by_giver
                .get_mut(&record.giver_id)
            else {
                summary.creature_remove_misses += 1;
                continue;
            };
            let Some(index) = active_records.iter().position(|active_record| {
                active_record.giver_id == record.giver_id
                    && active_record.quest_id == record.quest_id
            }) else {
                summary.creature_no_match += 1;
                continue;
            };
            active_records.remove(index);
            summary.creature_removed += 1;
            if active_records.is_empty() {
                self.game_event_active_creature_quest_relations_by_giver
                    .remove(&record.giver_id);
            }
        }
    }

    fn update_game_event_gameobject_quest_relation_cache_records_like_cpp(
        &mut self,
        event_id: u16,
        activate: bool,
        records: &[GameEventQuestRelationRecordLikeCpp],
        summary: &mut GameEventQuestRelationCacheUpdateSummaryLikeCpp,
    ) {
        summary.gameobject_records_seen = records.len();
        if activate {
            for record in records {
                self.game_event_active_gameobject_quest_relations_by_giver
                    .entry(record.giver_id)
                    .or_default()
                    .push(*record);
                summary.gameobject_inserted += 1;
            }
            return;
        }

        for record in records {
            if self.has_gameobject_quest_active_event_except_like_cpp(record.quest_id, event_id) {
                summary.gameobject_skipped_active_other_event += 1;
                continue;
            }
            let Some(active_records) = self
                .game_event_active_gameobject_quest_relations_by_giver
                .get_mut(&record.giver_id)
            else {
                summary.gameobject_remove_misses += 1;
                continue;
            };
            let Some(index) = active_records.iter().position(|active_record| {
                active_record.giver_id == record.giver_id
                    && active_record.quest_id == record.quest_id
            }) else {
                summary.gameobject_no_match += 1;
                continue;
            };
            active_records.remove(index);
            summary.gameobject_removed += 1;
            if active_records.is_empty() {
                self.game_event_active_gameobject_quest_relations_by_giver
                    .remove(&record.giver_id);
            }
        }
    }

    pub fn update_game_event_npc_vendor_cache_like_cpp(
        &mut self,
        event_id: u16,
        activate: bool,
    ) -> GameEventNpcVendorCacheUpdateSummaryLikeCpp {
        let mut summary = GameEventNpcVendorCacheUpdateSummaryLikeCpp {
            event_id,
            activate,
            ..GameEventNpcVendorCacheUpdateSummaryLikeCpp::default()
        };
        let Some(records) = self.game_event_npc_vendors.records_like_cpp(event_id) else {
            summary.missing_event_bucket = true;
            return summary;
        };
        summary.records_seen = records.len();

        if activate {
            for record in records {
                self.game_event_vendor_cache_by_entry
                    .entry(record.entry)
                    .or_default()
                    .push(record.clone());
                summary.items_added += 1;
            }
            return summary;
        }

        for record in records {
            let Some(cached_records) = self.game_event_vendor_cache_by_entry.get_mut(&record.entry)
            else {
                summary.remove_misses += 1;
                continue;
            };
            let before = cached_records.len();
            cached_records.retain(|cached| {
                cached.item != record.item || cached.vendor_type != record.vendor_type
            });
            let removed = before.saturating_sub(cached_records.len());
            if removed == 0 {
                summary.no_match += 1;
            } else {
                summary.items_removed += removed;
            }
            if cached_records.is_empty() {
                self.game_event_vendor_cache_by_entry.remove(&record.entry);
            }
        }
        summary
    }

    #[allow(dead_code)]
    pub fn game_event_npc_flag_mask_like_cpp(
        &self,
        spawn_id: SpawnId,
        active_event_ids: &[u16],
    ) -> u64 {
        self.game_event_npc_flags
            .game_event_npc_flag_mask_like_cpp(spawn_id, active_event_ids)
    }

    pub fn change_game_event_model_equip_baseline_like_cpp(
        &mut self,
        event_id: u16,
        activate: bool,
    ) -> GameEventModelEquipBaselineChangeSummaryLikeCpp {
        let mut summary = GameEventModelEquipBaselineChangeSummaryLikeCpp {
            event_id,
            activate,
            ..GameEventModelEquipBaselineChangeSummaryLikeCpp::default()
        };

        let Some(records) = self.game_event_model_equip.records_mut_like_cpp(event_id) else {
            summary.missing_event_bucket = true;
            return summary;
        };
        summary.records_seen = records.len();

        for record in records {
            if self
                .spawn_store
                .spawn_data(SpawnObjectType::Creature, record.spawn_id)
                .is_none()
            {
                summary.missing_spawn_metadata += 1;
                summary.record_outcomes.push(
                    GameEventModelEquipBaselineRecordOutcomeLikeCpp::MissingSpawnMetadata {
                        spawn_id: record.spawn_id,
                    },
                );
                continue;
            }

            let Some(row) = self.creature_runtime_rows.get_mut(&record.spawn_id) else {
                summary.missing_creature_runtime_rows += 1;
                summary.record_outcomes.push(
                    GameEventModelEquipBaselineRecordOutcomeLikeCpp::MissingCreatureRuntimeRow {
                        spawn_id: record.spawn_id,
                    },
                );
                continue;
            };

            if activate {
                record.model_id_prev = row.model_id;
                record.equipment_id_prev = u8::try_from(row.equipment_id).unwrap_or(0);
                row.model_id = record.model_id;
                row.equipment_id = i8::try_from(record.equipment_id).unwrap_or(i8::MAX);
            } else {
                row.model_id = record.model_id_prev;
                row.equipment_id = i8::try_from(record.equipment_id_prev).unwrap_or(i8::MAX);
            }

            summary.records_applied += 1;
            summary.record_outcomes.push(
                GameEventModelEquipBaselineRecordOutcomeLikeCpp::Applied {
                    spawn_id: record.spawn_id,
                    model_id_prev: record.model_id_prev,
                    equipment_id_prev: record.equipment_id_prev,
                    model_id_after: row.model_id,
                    equipment_id_after: u8::try_from(row.equipment_id).unwrap_or(0),
                },
            );
        }

        summary
    }
}
