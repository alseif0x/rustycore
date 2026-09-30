//! Game-event state, condition, and timing models.

use std::collections::{BTreeMap, BTreeSet};

use anyhow::Result;

use crate::spawn_store_loader::{
    GAME_EVENT_MINUTE_SECS_LIKE_CPP, MAX_GAME_EVENT_CHECK_DELAY_SECS_LIKE_CPP,
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(in crate::spawn_store_loader) struct GameEventSizingLikeCpp {
    game_event_size: i32,
    slot_count: usize,
}

impl GameEventSizingLikeCpp {
    pub(in crate::spawn_store_loader) fn from_max_event_entry_like_cpp(
        max_event_entry: Option<u32>,
    ) -> Self {
        let max_event_id = max_event_entry.unwrap_or(0).saturating_add(1);
        let slot_count = max_event_id.saturating_mul(2).saturating_sub(1) as usize;
        let game_event_size = i32::try_from(max_event_id).unwrap_or(i32::MAX);
        Self {
            game_event_size,
            slot_count,
        }
    }

    pub(in crate::spawn_store_loader::game_event_catalog) const fn game_event_size_like_cpp(
        self,
    ) -> i32 {
        self.game_event_size
    }

    pub(in crate::spawn_store_loader::game_event_catalog) const fn slot_count_like_cpp(
        self,
    ) -> usize {
        self.slot_count
    }

    pub(in crate::spawn_store_loader::game_event_catalog) fn master_slot_count_like_cpp(
        self,
    ) -> usize {
        usize::try_from(self.game_event_size).unwrap_or(usize::MAX)
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum GameEventStateLikeCpp {
    Normal = 0,
    WorldInactive = 1,
    WorldConditions = 2,
    WorldNextPhase = 3,
    WorldFinished = 4,
    Internal = 5,
}

#[allow(dead_code)]
impl GameEventStateLikeCpp {
    pub fn from_raw_like_cpp(state_raw: u8) -> Option<Self> {
        match state_raw {
            0 => Some(Self::Normal),
            1 => Some(Self::WorldInactive),
            2 => Some(Self::WorldConditions),
            3 => Some(Self::WorldNextPhase),
            4 => Some(Self::WorldFinished),
            5 => Some(Self::Internal),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameEventCheckOutcomeLikeCpp {
    Active(bool),
    MissingEvent { event_id: u16 },
    MissingPrerequisite { event_id: u16 },
    InvalidTimingZeroOccurrence { event_id: u16 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameEventPrerequisiteInsertOutcomeLikeCpp {
    Loaded,
    Duplicate,
    OutOfRangeEvent,
    NonWorldEvent,
    OutOfRangePrerequisite,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameEventNextCheckOutcomeLikeCpp {
    DelaySecs(u64),
    MissingEvent { event_id: u16 },
    InvalidTimingZeroOccurrence { event_id: u16 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameEventHolidayActiveOutcomeLikeCpp {
    Active(bool),
    MissingActiveEvent { event_id: u16 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameEventStartOutcomeLikeCpp {
    Started(GameEventStartSummaryLikeCpp),
    MissingEvent { event_id: u16 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameEventStartSummaryLikeCpp {
    pub event_id: u16,
    pub state_before_raw: u8,
    pub state_after_raw: u8,
    pub active_added: bool,
    pub active_was_present: bool,
    pub apply_new_event_requested: bool,
    pub save_world_event_state_requested: bool,
    pub force_game_event_update_requested: bool,
    pub completed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameEventStopOutcomeLikeCpp {
    Stopped(GameEventStopSummaryLikeCpp),
    MissingEvent { event_id: u16 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameEventStopSummaryLikeCpp {
    pub event_id: u16,
    pub state_before_raw: u8,
    pub state_after_raw: u8,
    pub active_removed: bool,
    pub active_was_present: bool,
    pub unapply_event_requested: bool,
    pub serverwide: bool,
    pub condition_reset_requested: bool,
    pub delete_world_event_state_requested: bool,
    pub delete_condition_saves_requested: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameEventWorldStateSaveEvidenceLikeCpp {
    pub event_id: u16,
    pub state_after_raw: u8,
    pub next_start_after: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameEventWorldNextPhaseFinishedLikeCpp {
    pub event_id: u16,
    pub was_active_before_queue: bool,
    pub state_before_raw: u8,
    pub state_after_raw: u8,
    pub next_start_before: u64,
    pub next_start_after: u64,
    pub save_state_requested: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameEventUpdateOutcomeLikeCpp {
    pub current_time_secs: u64,
    pub scanned_event_ids: Vec<u16>,
    pub check_outcomes: Vec<(u16, GameEventCheckOutcomeLikeCpp)>,
    pub next_check_outcomes: Vec<(u16, GameEventNextCheckOutcomeLikeCpp)>,
    pub queued_activation_event_ids: Vec<u16>,
    pub queued_deactivation_event_ids: Vec<u16>,
    pub start_outcomes: Vec<GameEventStartOutcomeLikeCpp>,
    pub stop_outcomes: Vec<GameEventStopOutcomeLikeCpp>,
    pub negative_spawn_event_ids: Vec<i16>,
    pub world_nextphase_finished: Vec<GameEventWorldNextPhaseFinishedLikeCpp>,
    pub world_conditions_save_requested: Vec<GameEventWorldStateSaveEvidenceLikeCpp>,
    pub invalid_check_outcomes: Vec<GameEventCheckOutcomeLikeCpp>,
    pub invalid_next_check_outcomes: Vec<GameEventNextCheckOutcomeLikeCpp>,
    pub next_event_delay_secs_before_padding: u64,
    pub next_update_delay_millis: u64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GameEventConditionLikeCpp {
    pub req_num: f32,
    pub done: f32,
    pub max_world_state: u16,
    pub done_world_state: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameEventWorldStateUpdateSourceLikeCpp {
    Done,
    Max,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameEventWorldStateValueSkipReasonLikeCpp {
    NonFinite,
    Negative,
    OutOfI32Range,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameEventWorldStateUpdateEvidenceLikeCpp {
    pub event_id: u16,
    pub condition_id: u32,
    pub variable_id: u32,
    pub value: i32,
    pub source: GameEventWorldStateUpdateSourceLikeCpp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameEventWorldStateUpdateSkipLikeCpp {
    pub event_id: u16,
    pub condition_id: u32,
    pub variable_id: u32,
    pub source: GameEventWorldStateUpdateSourceLikeCpp,
    pub reason: GameEventWorldStateValueSkipReasonLikeCpp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameEventWorldStateUpdateOutcomeLikeCpp {
    Updates {
        event_id: u16,
        updates: Vec<GameEventWorldStateUpdateEvidenceLikeCpp>,
        skipped: Vec<GameEventWorldStateUpdateSkipLikeCpp>,
    },
    MissingEvent {
        event_id: u16,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameEventConditionApplyOutcomeLikeCpp {
    Loaded,
    OutOfRangeEvent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameEventConditionSaveApplyOutcomeLikeCpp {
    Loaded,
    OutOfRangeEvent,
    MissingCondition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameEventConditionCheckOutcomeLikeCpp {
    Completed(GameEventConditionCheckSummaryLikeCpp),
    NotCompleted {
        event_id: u16,
        blocking_condition_id: u32,
    },
    MissingEvent {
        event_id: u16,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameEventConditionCheckSummaryLikeCpp {
    pub event_id: u16,
    pub condition_count: usize,
    pub state_before_raw: u8,
    pub state_after_raw: u8,
    pub next_start_before: u64,
    pub next_start_after: u64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GameEventQuestConditionRecordLikeCpp {
    pub quest_id: u32,
    pub event_id: u16,
    pub condition_id: u32,
    pub num: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GameEventQuestCompleteOutcomeLikeCpp {
    MissingQuestMapping { quest_id: u32 },
    Progress(GameEventConditionProgressOutcomeLikeCpp),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GameEventConditionProgressOutcomeLikeCpp {
    Progressed(GameEventConditionProgressSummaryLikeCpp),
    MissingEvent {
        event_id: u16,
    },
    InactiveEvent {
        event_id: u16,
    },
    NotWorldConditions {
        event_id: u16,
        state_raw: u8,
    },
    MissingCondition {
        event_id: u16,
        condition_id: u32,
    },
    AlreadyComplete {
        event_id: u16,
        condition_id: u32,
        done: f32,
        req_num: f32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GameEventConditionProgressSummaryLikeCpp {
    pub event_id: u16,
    pub condition_id: u32,
    pub done_before: f32,
    pub done_after: f32,
    pub req_num: f32,
    pub persistence_event_id: u8,
    pub completed_event: bool,
    pub check_outcome: GameEventConditionCheckOutcomeLikeCpp,
    pub save_world_event_state_requested: bool,
    pub force_game_event_update_requested: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GameEventDataLikeCpp {
    pub event_id: u16,
    pub start: u64,
    pub end: u64,
    pub next_start: u64,
    pub occurence: u32,
    pub length: u32,
    pub holiday_id: u32,
    pub holiday_stage: u8,
    pub state_raw: u8,
    pub prerequisite_events: BTreeSet<u16>,
    pub conditions: BTreeMap<u32, GameEventConditionLikeCpp>,
    pub description: String,
    pub announce: u8,
}

impl Default for GameEventDataLikeCpp {
    fn default() -> Self {
        Self {
            event_id: 0,
            start: 1,
            end: 0,
            next_start: 0,
            occurence: 0,
            length: 0,
            holiday_id: 0,
            holiday_stage: 0,
            state_raw: GameEventStateLikeCpp::Normal as u8,
            prerequisite_events: BTreeSet::new(),
            conditions: BTreeMap::new(),
            description: String::new(),
            announce: 0,
        }
    }
}

#[allow(dead_code)]
impl GameEventDataLikeCpp {
    pub fn state_like_cpp(&self) -> Option<GameEventStateLikeCpp> {
        GameEventStateLikeCpp::from_raw_like_cpp(self.state_raw)
    }

    pub fn is_valid_like_cpp(&self) -> bool {
        self.length > 0 || self.state_raw > GameEventStateLikeCpp::Normal as u8
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct GameEventDataStoreLikeCpp {
    events: Vec<GameEventDataLikeCpp>,
}

#[allow(dead_code)]
impl GameEventDataStoreLikeCpp {
    pub fn from_game_event_max_entry_like_cpp(max_event_entry: Option<u32>) -> Self {
        Self::from_game_event_sizing_like_cpp(
            GameEventSizingLikeCpp::from_max_event_entry_like_cpp(max_event_entry),
        )
    }

    pub(in crate::spawn_store_loader) fn from_game_event_sizing_like_cpp(
        sizing: GameEventSizingLikeCpp,
    ) -> Self {
        let mut events = vec![GameEventDataLikeCpp::default(); sizing.master_slot_count_like_cpp()];
        for (event_id, event) in events.iter_mut().enumerate() {
            event.event_id = u16::try_from(event_id).unwrap_or(u16::MAX);
        }
        Self { events }
    }

    pub fn len_like_cpp(&self) -> usize {
        self.events.len()
    }

    pub fn event_like_cpp(&self, event_id: u16) -> Option<&GameEventDataLikeCpp> {
        self.events.get(usize::from(event_id))
    }

    pub fn prerequisite_events_like_cpp(&self, event_id: u16) -> Option<&BTreeSet<u16>> {
        self.event_like_cpp(event_id)
            .map(|event| &event.prerequisite_events)
    }

    pub fn insert_prerequisite_event_like_cpp(
        &mut self,
        event_id: u16,
        prerequisite_event: u32,
    ) -> GameEventPrerequisiteInsertOutcomeLikeCpp {
        let event_index = usize::from(event_id);
        if event_index >= self.events.len() {
            return GameEventPrerequisiteInsertOutcomeLikeCpp::OutOfRangeEvent;
        }

        let state_raw = self.events[event_index].state_raw;
        if state_raw == GameEventStateLikeCpp::Normal as u8
            || state_raw == GameEventStateLikeCpp::Internal as u8
        {
            return GameEventPrerequisiteInsertOutcomeLikeCpp::NonWorldEvent;
        }

        let Ok(prerequisite_event_id) = u16::try_from(prerequisite_event) else {
            return GameEventPrerequisiteInsertOutcomeLikeCpp::OutOfRangePrerequisite;
        };
        if usize::from(prerequisite_event_id) >= self.events.len() {
            return GameEventPrerequisiteInsertOutcomeLikeCpp::OutOfRangePrerequisite;
        }

        if self.events[event_index]
            .prerequisite_events
            .insert(prerequisite_event_id)
        {
            GameEventPrerequisiteInsertOutcomeLikeCpp::Loaded
        } else {
            GameEventPrerequisiteInsertOutcomeLikeCpp::Duplicate
        }
    }

    pub fn check_one_game_event_like_cpp(
        &self,
        event_id: u16,
        current_time_secs: u64,
    ) -> GameEventCheckOutcomeLikeCpp {
        let Some(event) = self.event_like_cpp(event_id) else {
            return GameEventCheckOutcomeLikeCpp::MissingEvent { event_id };
        };

        match event.state_like_cpp() {
            Some(
                GameEventStateLikeCpp::WorldConditions | GameEventStateLikeCpp::WorldNextPhase,
            ) => GameEventCheckOutcomeLikeCpp::Active(true),
            Some(GameEventStateLikeCpp::WorldFinished | GameEventStateLikeCpp::Internal) => {
                GameEventCheckOutcomeLikeCpp::Active(false)
            }
            Some(GameEventStateLikeCpp::WorldInactive) => {
                if event.prerequisite_events.is_empty() {
                    return GameEventCheckOutcomeLikeCpp::Active(false);
                }

                for &prerequisite_event_id in &event.prerequisite_events {
                    let Some(prerequisite_event) = self.event_like_cpp(prerequisite_event_id)
                    else {
                        return GameEventCheckOutcomeLikeCpp::MissingPrerequisite {
                            event_id: prerequisite_event_id,
                        };
                    };
                    let prerequisite_state = prerequisite_event.state_like_cpp();
                    let prerequisite_done = matches!(
                        prerequisite_state,
                        Some(
                            GameEventStateLikeCpp::WorldNextPhase
                                | GameEventStateLikeCpp::WorldFinished
                        )
                    );
                    if !prerequisite_done || prerequisite_event.next_start > current_time_secs {
                        return GameEventCheckOutcomeLikeCpp::Active(false);
                    }
                }

                GameEventCheckOutcomeLikeCpp::Active(true)
            }
            Some(GameEventStateLikeCpp::Normal) | None => {
                Self::check_periodic_window_like_cpp(event, current_time_secs)
            }
        }
    }

    pub fn last_start_time_like_cpp(&self, event_id: u16, current_time_secs: u64) -> u64 {
        let Some(event) = self.event_like_cpp(event_id) else {
            return 0;
        };
        if event.state_like_cpp() != Some(GameEventStateLikeCpp::Normal) {
            return 0;
        }
        let Some(period_secs) = periodic_occurence_secs_like_cpp(event.occurence) else {
            return 0;
        };
        current_time_secs
            .saturating_sub(current_time_secs.saturating_sub(event.start) % period_secs)
    }

    pub fn next_check_like_cpp(
        &self,
        event_id: u16,
        current_time_secs: u64,
    ) -> GameEventNextCheckOutcomeLikeCpp {
        let Some(event) = self.event_like_cpp(event_id) else {
            return GameEventNextCheckOutcomeLikeCpp::MissingEvent { event_id };
        };

        if matches!(
            event.state_like_cpp(),
            Some(GameEventStateLikeCpp::WorldNextPhase | GameEventStateLikeCpp::WorldFinished)
        ) && event.next_start >= current_time_secs
        {
            return GameEventNextCheckOutcomeLikeCpp::DelaySecs(
                event.next_start.saturating_sub(current_time_secs),
            );
        }

        if event.state_like_cpp() == Some(GameEventStateLikeCpp::WorldConditions) {
            return if event.length != 0 {
                GameEventNextCheckOutcomeLikeCpp::DelaySecs(
                    u64::from(event.length).saturating_mul(GAME_EVENT_MINUTE_SECS_LIKE_CPP),
                )
            } else {
                GameEventNextCheckOutcomeLikeCpp::DelaySecs(
                    MAX_GAME_EVENT_CHECK_DELAY_SECS_LIKE_CPP,
                )
            };
        }

        if current_time_secs > event.end {
            return GameEventNextCheckOutcomeLikeCpp::DelaySecs(
                MAX_GAME_EVENT_CHECK_DELAY_SECS_LIKE_CPP,
            );
        }

        if event.start > current_time_secs {
            return GameEventNextCheckOutcomeLikeCpp::DelaySecs(event.start - current_time_secs);
        }

        let Some(period_secs) = periodic_occurence_secs_like_cpp(event.occurence) else {
            return GameEventNextCheckOutcomeLikeCpp::InvalidTimingZeroOccurrence { event_id };
        };
        let length_secs = u64::from(event.length).saturating_mul(GAME_EVENT_MINUTE_SECS_LIKE_CPP);
        let elapsed_in_period = current_time_secs.saturating_sub(event.start) % period_secs;
        let delay = if elapsed_in_period < length_secs {
            length_secs.saturating_sub(elapsed_in_period)
        } else {
            period_secs.saturating_sub(elapsed_in_period)
        };
        let end_delay = event.end.saturating_sub(current_time_secs);
        GameEventNextCheckOutcomeLikeCpp::DelaySecs(
            if event.end < current_time_secs.saturating_add(delay) {
                end_delay
            } else {
                delay
            },
        )
    }

    pub fn apply_game_event_condition_row_like_cpp(
        &mut self,
        event_id: u16,
        condition_id: u32,
        req_num: f32,
        max_world_state: u16,
        done_world_state: u16,
    ) -> GameEventConditionApplyOutcomeLikeCpp {
        let Some(event) = self.event_mut_like_cpp(event_id) else {
            return GameEventConditionApplyOutcomeLikeCpp::OutOfRangeEvent;
        };

        event.conditions.insert(
            condition_id,
            GameEventConditionLikeCpp {
                req_num,
                done: 0.0,
                max_world_state,
                done_world_state,
            },
        );
        GameEventConditionApplyOutcomeLikeCpp::Loaded
    }

    pub fn apply_game_event_condition_save_row_like_cpp(
        &mut self,
        event_id: u16,
        condition_id: u32,
        done: f32,
    ) -> GameEventConditionSaveApplyOutcomeLikeCpp {
        let Some(event) = self.event_mut_like_cpp(event_id) else {
            return GameEventConditionSaveApplyOutcomeLikeCpp::OutOfRangeEvent;
        };
        let Some(condition) = event.conditions.get_mut(&condition_id) else {
            return GameEventConditionSaveApplyOutcomeLikeCpp::MissingCondition;
        };

        condition.done = done;
        GameEventConditionSaveApplyOutcomeLikeCpp::Loaded
    }

    pub fn send_world_state_update_evidence_like_cpp(
        &self,
        event_id: u16,
    ) -> GameEventWorldStateUpdateOutcomeLikeCpp {
        let Some(event) = self.event_like_cpp(event_id) else {
            return GameEventWorldStateUpdateOutcomeLikeCpp::MissingEvent { event_id };
        };

        let mut updates = Vec::new();
        let mut skipped = Vec::new();
        for (&condition_id, condition) in &event.conditions {
            if condition.done_world_state != 0 {
                push_game_event_world_state_update_like_cpp(
                    event_id,
                    condition_id,
                    u32::from(condition.done_world_state),
                    condition.done,
                    GameEventWorldStateUpdateSourceLikeCpp::Done,
                    &mut updates,
                    &mut skipped,
                );
            }
            if condition.max_world_state != 0 {
                push_game_event_world_state_update_like_cpp(
                    event_id,
                    condition_id,
                    u32::from(condition.max_world_state),
                    condition.req_num,
                    GameEventWorldStateUpdateSourceLikeCpp::Max,
                    &mut updates,
                    &mut skipped,
                );
            }
        }

        GameEventWorldStateUpdateOutcomeLikeCpp::Updates {
            event_id,
            updates,
            skipped,
        }
    }

    pub fn check_one_game_event_conditions_like_cpp(
        &mut self,
        event_id: u16,
        current_time_secs: u64,
    ) -> GameEventConditionCheckOutcomeLikeCpp {
        let Some(event) = self.event_mut_like_cpp(event_id) else {
            return GameEventConditionCheckOutcomeLikeCpp::MissingEvent { event_id };
        };

        for (&condition_id, condition) in &event.conditions {
            if condition.done < condition.req_num {
                return GameEventConditionCheckOutcomeLikeCpp::NotCompleted {
                    event_id,
                    blocking_condition_id: condition_id,
                };
            }
        }

        let state_before_raw = event.state_raw;
        let next_start_before = event.next_start;
        event.state_raw = GameEventStateLikeCpp::WorldNextPhase as u8;
        if event.next_start == 0 {
            event.next_start = current_time_secs.saturating_add(
                u64::from(event.length).saturating_mul(GAME_EVENT_MINUTE_SECS_LIKE_CPP),
            );
        }

        GameEventConditionCheckOutcomeLikeCpp::Completed(GameEventConditionCheckSummaryLikeCpp {
            event_id,
            condition_count: event.conditions.len(),
            state_before_raw,
            state_after_raw: event.state_raw,
            next_start_before,
            next_start_after: event.next_start,
        })
    }

    fn check_periodic_window_like_cpp(
        event: &GameEventDataLikeCpp,
        current_time_secs: u64,
    ) -> GameEventCheckOutcomeLikeCpp {
        if !(event.start < current_time_secs && current_time_secs < event.end) {
            return GameEventCheckOutcomeLikeCpp::Active(false);
        }
        let Some(period_secs) = periodic_occurence_secs_like_cpp(event.occurence) else {
            return GameEventCheckOutcomeLikeCpp::InvalidTimingZeroOccurrence {
                event_id: event.event_id,
            };
        };
        let length_secs = u64::from(event.length).saturating_mul(GAME_EVENT_MINUTE_SECS_LIKE_CPP);
        let elapsed_in_period = current_time_secs.saturating_sub(event.start) % period_secs;
        GameEventCheckOutcomeLikeCpp::Active(elapsed_in_period < length_secs)
    }

    pub fn iter_like_cpp(&self) -> impl Iterator<Item = &GameEventDataLikeCpp> {
        self.events.iter()
    }

    pub(in crate::spawn_store_loader) fn event_mut_like_cpp(
        &mut self,
        event_id: u16,
    ) -> Option<&mut GameEventDataLikeCpp> {
        self.events.get_mut(usize::from(event_id))
    }

    #[cfg(test)]
    pub(crate) fn with_event_like_cpp(mut self, event: GameEventDataLikeCpp) -> Self {
        if let Some(slot) = self.event_mut_like_cpp(event.event_id) {
            *slot = event;
        }
        self
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GameEventActiveSetLikeCpp {
    active_events: BTreeSet<u16>,
}

#[allow(dead_code)]
impl GameEventActiveSetLikeCpp {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_active_event_like_cpp(&mut self, event_id: u16) -> bool {
        self.active_events.insert(event_id)
    }

    pub fn remove_active_event_like_cpp(&mut self, event_id: u16) -> bool {
        self.active_events.remove(&event_id)
    }

    pub fn clear_active_events_like_cpp(&mut self) {
        self.active_events.clear();
    }

    pub fn is_active_event_like_cpp(&self, event_id: u16) -> bool {
        self.active_events.contains(&event_id)
    }

    pub fn active_event_ids_like_cpp(&self) -> impl Iterator<Item = u16> + '_ {
        self.active_events.iter().copied()
    }

    pub fn is_holiday_active_like_cpp(
        &self,
        events: &GameEventDataStoreLikeCpp,
        holiday_id: u32,
    ) -> GameEventHolidayActiveOutcomeLikeCpp {
        if holiday_id == 0 {
            return GameEventHolidayActiveOutcomeLikeCpp::Active(false);
        }

        for event_id in self.active_event_ids_like_cpp() {
            let Some(event) = events.event_like_cpp(event_id) else {
                return GameEventHolidayActiveOutcomeLikeCpp::MissingActiveEvent { event_id };
            };

            if event.holiday_id == holiday_id {
                return GameEventHolidayActiveOutcomeLikeCpp::Active(true);
            }
        }

        GameEventHolidayActiveOutcomeLikeCpp::Active(false)
    }
}

fn periodic_occurence_secs_like_cpp(occurence_minutes: u32) -> Option<u64> {
    (occurence_minutes != 0)
        .then(|| u64::from(occurence_minutes).saturating_mul(GAME_EVENT_MINUTE_SECS_LIKE_CPP))
}

fn push_game_event_world_state_update_like_cpp(
    event_id: u16,
    condition_id: u32,
    variable_id: u32,
    raw_value: f32,
    source: GameEventWorldStateUpdateSourceLikeCpp,
    updates: &mut Vec<GameEventWorldStateUpdateEvidenceLikeCpp>,
    skipped: &mut Vec<GameEventWorldStateUpdateSkipLikeCpp>,
) {
    match world_state_value_i32_like_cpp(raw_value) {
        Ok(value) => updates.push(GameEventWorldStateUpdateEvidenceLikeCpp {
            event_id,
            condition_id,
            variable_id,
            value,
            source,
        }),
        Err(reason) => skipped.push(GameEventWorldStateUpdateSkipLikeCpp {
            event_id,
            condition_id,
            variable_id,
            source,
            reason,
        }),
    }
}

fn world_state_value_i32_like_cpp(
    raw_value: f32,
) -> Result<i32, GameEventWorldStateValueSkipReasonLikeCpp> {
    if !raw_value.is_finite() {
        return Err(GameEventWorldStateValueSkipReasonLikeCpp::NonFinite);
    }
    if raw_value < 0.0 {
        return Err(GameEventWorldStateValueSkipReasonLikeCpp::Negative);
    }
    let truncated = raw_value.trunc();
    if f64::from(truncated) > f64::from(i32::MAX) {
        return Err(GameEventWorldStateValueSkipReasonLikeCpp::OutOfI32Range);
    }
    Ok(truncated as i32)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::spawn_store_loader) struct GameEventConditionSaveRowLikeCpp {
    pub(in crate::spawn_store_loader) event_id: u16,
    pub(in crate::spawn_store_loader) condition_id: u32,
    pub(in crate::spawn_store_loader) done: f32,
}
