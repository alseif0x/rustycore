use std::collections::{BTreeMap, VecDeque};

use crate::random::urand_like_cpp;

const EVENT_ID_MASK_LIKE_CPP: u32 = 0x0000_FFFF;
const EVENT_PHASE_MASK_LIKE_CPP: u32 = 0xFF00_0000;
const MAX_EVENT_GROUP_OR_PHASE_LIKE_CPP: u32 = 8;

#[derive(Debug, Clone, Default)]
pub struct EventMap {
    time_ms: i64,
    phase_mask: u8,
    last_event_data: u32,
    events: BTreeMap<i64, VecDeque<u32>>,
    timer_series: BTreeMap<u32, VecDeque<i64>>,
}

impl EventMap {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset_like_cpp(&mut self) {
        self.events.clear();
        self.timer_series.clear();
        self.time_ms = 0;
        self.phase_mask = 0;
        self.last_event_data = 0;
    }

    pub fn update_like_cpp(&mut self, diff_ms: u32) {
        self.time_ms += i64::from(diff_ms);
    }

    #[must_use]
    pub const fn phase_mask_like_cpp(&self) -> u8 {
        self.phase_mask
    }

    #[must_use]
    pub fn is_empty_like_cpp(&self) -> bool {
        self.events.is_empty()
    }

    pub fn set_phase_like_cpp(&mut self, phase: u8) {
        if phase == 0 {
            self.phase_mask = 0;
        } else if phase <= MAX_EVENT_GROUP_OR_PHASE_LIKE_CPP as u8 {
            self.phase_mask = 1u8 << (phase - 1);
        }
    }

    pub fn add_phase_like_cpp(&mut self, phase: u8) {
        if phase != 0 && phase <= MAX_EVENT_GROUP_OR_PHASE_LIKE_CPP as u8 {
            self.phase_mask |= 1u8 << (phase - 1);
        }
    }

    pub fn remove_phase_like_cpp(&mut self, phase: u8) {
        if phase != 0 && phase <= MAX_EVENT_GROUP_OR_PHASE_LIKE_CPP as u8 {
            self.phase_mask &= !(1u8 << (phase - 1));
        }
    }

    #[must_use]
    pub fn is_in_phase_like_cpp(&self, phase: u8) -> bool {
        phase <= MAX_EVENT_GROUP_OR_PHASE_LIKE_CPP as u8
            && (phase == 0 || (self.phase_mask & (1u8 << (phase - 1))) != 0)
    }

    pub fn schedule_event_like_cpp(&mut self, event_id: u32, delay_ms: i64) {
        self.schedule_event_with_group_phase_like_cpp(event_id, delay_ms, 0, 0);
    }

    pub fn schedule_event_with_group_phase_like_cpp(
        &mut self,
        event_id: u32,
        delay_ms: i64,
        group: u32,
        phase: u8,
    ) {
        let event_data = encode_event_data_like_cpp(event_id, group, phase);
        self.events
            .entry(self.time_ms + delay_ms)
            .or_default()
            .push_back(event_data);
    }

    pub fn schedule_event_range_like_cpp(
        &mut self,
        event_id: u32,
        min_delay_ms: i64,
        max_delay_ms: i64,
        group: u32,
        phase: u8,
    ) {
        self.schedule_event_with_group_phase_like_cpp(
            event_id,
            randtime_delay_like_cpp(min_delay_ms, max_delay_ms),
            group,
            phase,
        );
    }

    pub fn reschedule_event_like_cpp(&mut self, event_id: u32, delay_ms: i64) {
        self.reschedule_event_with_group_phase_like_cpp(event_id, delay_ms, 0, 0);
    }

    pub fn reschedule_event_with_group_phase_like_cpp(
        &mut self,
        event_id: u32,
        delay_ms: i64,
        group: u32,
        phase: u8,
    ) {
        self.cancel_event_like_cpp(event_id);
        self.schedule_event_with_group_phase_like_cpp(event_id, delay_ms, group, phase);
    }

    pub fn repeat_like_cpp(&mut self, delay_ms: i64) {
        self.events
            .entry(self.time_ms + delay_ms)
            .or_default()
            .push_back(self.last_event_data);
    }

    pub fn repeat_range_like_cpp(&mut self, min_delay_ms: i64, max_delay_ms: i64) {
        self.repeat_like_cpp(randtime_delay_like_cpp(min_delay_ms, max_delay_ms));
    }

    pub fn execute_event_like_cpp(&mut self) -> u32 {
        while let Some((&deadline, _)) = self.events.first_key_value() {
            if deadline > self.time_ms {
                return 0;
            }

            let event_data = pop_first_event_like_cpp(&mut self.events, deadline);
            if self.phase_mask != 0
                && (event_data & EVENT_PHASE_MASK_LIKE_CPP) != 0
                && (((event_data >> 24) as u8) & self.phase_mask) == 0
            {
                continue;
            }

            self.last_event_data = event_data;
            self.schedule_next_from_series_like_cpp(event_data);
            return event_data & EVENT_ID_MASK_LIKE_CPP;
        }

        0
    }

    pub fn delay_events_like_cpp(&mut self, delay_ms: i64) {
        if self.events.is_empty() {
            return;
        }

        let mut delayed = BTreeMap::new();
        for (deadline, events) in std::mem::take(&mut self.events) {
            delayed.insert(deadline + delay_ms, events);
        }
        self.events = delayed;
    }

    pub fn delay_events_group_like_cpp(&mut self, delay_ms: i64, group: u32) {
        if !valid_group_like_cpp(group) || self.events.is_empty() {
            return;
        }

        let mut delayed = BTreeMap::new();
        for (deadline, events) in std::mem::take(&mut self.events) {
            for event_data in events {
                let target_deadline = if event_has_group_like_cpp(event_data, group) {
                    deadline + delay_ms
                } else {
                    deadline
                };
                delayed
                    .entry(target_deadline)
                    .or_insert_with(VecDeque::new)
                    .push_back(event_data);
            }
        }
        self.events = delayed;
    }

    pub fn cancel_event_like_cpp(&mut self, event_id: u32) {
        if self.events.is_empty() {
            return;
        }

        retain_events_like_cpp(&mut self.events, |event_data| {
            (event_data & EVENT_ID_MASK_LIKE_CPP) != event_id
        });
        self.timer_series
            .retain(|event_data, _| (event_data & EVENT_ID_MASK_LIKE_CPP) != event_id);
    }

    pub fn cancel_event_group_like_cpp(&mut self, group: u32) {
        if !valid_group_like_cpp(group) || self.events.is_empty() {
            return;
        }

        retain_events_like_cpp(&mut self.events, |event_data| {
            !event_has_group_like_cpp(event_data, group)
        });
        self.timer_series
            .retain(|event_data, _| !event_has_group_like_cpp(*event_data, group));
    }

    #[must_use]
    pub fn time_until_event_like_cpp(&self, event_id: u32) -> i64 {
        for (deadline, events) in &self.events {
            if events
                .iter()
                .any(|event_data| (event_data & EVENT_ID_MASK_LIKE_CPP) == event_id)
            {
                return deadline - self.time_ms;
            }
        }

        i64::MAX
    }

    pub fn schedule_next_from_series_like_cpp(&mut self, event_data: u32) {
        let Some(series) = self.timer_series.get_mut(&event_data) else {
            return;
        };
        let Some(delay_ms) = series.pop_front() else {
            return;
        };

        self.schedule_event_like_cpp(event_data, delay_ms);
    }

    pub fn schedule_event_series_like_cpp(
        &mut self,
        event_id: u32,
        group: u8,
        phase: u8,
        time_series_ms: impl IntoIterator<Item = i64>,
    ) {
        let event_data = encode_event_data_like_cpp(event_id, u32::from(group), phase);
        self.timer_series
            .entry(event_data)
            .or_default()
            .extend(time_series_ms);
        self.schedule_next_from_series_like_cpp(event_data);
    }
}

fn encode_event_data_like_cpp(mut event_id: u32, group: u32, phase: u8) -> u32 {
    if valid_group_like_cpp(group) {
        event_id |= 1 << (group + 15);
    }

    if phase != 0 && phase <= MAX_EVENT_GROUP_OR_PHASE_LIKE_CPP as u8 {
        event_id |= 1 << (u32::from(phase) + 23);
    }

    event_id
}

fn valid_group_like_cpp(group: u32) -> bool {
    group != 0 && group <= MAX_EVENT_GROUP_OR_PHASE_LIKE_CPP
}

fn event_has_group_like_cpp(event_data: u32, group: u32) -> bool {
    valid_group_like_cpp(group) && (event_data & (1 << (group + 15))) != 0
}

fn randtime_delay_like_cpp(min_delay_ms: i64, max_delay_ms: i64) -> i64 {
    let diff = max_delay_ms - min_delay_ms;
    assert!(diff >= 0, "randtime min must be <= max like C++ ASSERT");
    assert!(
        diff <= i64::from(u32::MAX),
        "randtime diff must fit u32 like C++ ASSERT"
    );
    min_delay_ms + i64::from(urand_like_cpp(0, diff as u32))
}

fn pop_first_event_like_cpp(events: &mut BTreeMap<i64, VecDeque<u32>>, deadline: i64) -> u32 {
    let queue = events
        .get_mut(&deadline)
        .expect("first event deadline must exist");
    let event_data = queue
        .pop_front()
        .expect("first event deadline must have an event");
    if queue.is_empty() {
        events.remove(&deadline);
    }
    event_data
}

fn retain_events_like_cpp<F>(events: &mut BTreeMap<i64, VecDeque<u32>>, mut keep: F)
where
    F: FnMut(u32) -> bool,
{
    let mut retained = BTreeMap::new();
    for (deadline, queue) in std::mem::take(events) {
        let queue = queue
            .into_iter()
            .filter(|event_data| keep(*event_data))
            .collect::<VecDeque<_>>();
        if !queue.is_empty() {
            retained.insert(deadline, queue);
        }
    }
    *events = retained;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_map_schedule_update_execute_matches_cpp_ready_gate() {
        let mut events = EventMap::new();
        events.schedule_event_like_cpp(7, 100);

        events.update_like_cpp(99);
        assert_eq!(events.execute_event_like_cpp(), 0);

        events.update_like_cpp(1);
        assert_eq!(events.execute_event_like_cpp(), 7);
        assert_eq!(events.execute_event_like_cpp(), 0);
    }

    #[test]
    fn event_map_phase_filters_and_discards_non_matching_events_like_cpp() {
        let mut events = EventMap::new();
        events.schedule_event_with_group_phase_like_cpp(1, 0, 0, 1);
        events.schedule_event_with_group_phase_like_cpp(2, 0, 0, 2);
        events.set_phase_like_cpp(2);

        assert_eq!(events.execute_event_like_cpp(), 2);
        assert_eq!(events.execute_event_like_cpp(), 0);
    }

    #[test]
    fn event_map_repeat_preserves_group_and_phase_bits_like_cpp() {
        let mut events = EventMap::new();
        events.schedule_event_with_group_phase_like_cpp(5, 0, 3, 2);
        events.set_phase_like_cpp(2);

        assert_eq!(events.execute_event_like_cpp(), 5);
        events.repeat_like_cpp(10);
        events.update_like_cpp(10);

        assert_eq!(events.execute_event_like_cpp(), 5);
    }

    #[test]
    fn event_map_cancel_and_delay_group_use_encoded_group_bits_like_cpp() {
        let mut events = EventMap::new();
        events.schedule_event_with_group_phase_like_cpp(1, 10, 1, 0);
        events.schedule_event_with_group_phase_like_cpp(2, 10, 2, 0);
        events.delay_events_group_like_cpp(10, 1);

        events.update_like_cpp(10);
        assert_eq!(events.execute_event_like_cpp(), 2);
        assert_eq!(events.execute_event_like_cpp(), 0);

        events.cancel_event_group_like_cpp(1);
        events.update_like_cpp(10);
        assert_eq!(events.execute_event_like_cpp(), 0);
    }

    #[test]
    fn event_map_series_requeues_next_delay_after_execute_like_cpp() {
        let mut events = EventMap::new();
        events.schedule_event_series_like_cpp(9, 0, 0, [5, 7]);

        events.update_like_cpp(5);
        assert_eq!(events.execute_event_like_cpp(), 9);
        assert_eq!(events.time_until_event_like_cpp(9), 7);

        events.update_like_cpp(7);
        assert_eq!(events.execute_event_like_cpp(), 9);
        assert_eq!(events.execute_event_like_cpp(), 0);
    }

    #[test]
    fn event_map_reschedule_cancels_existing_event_id_like_cpp() {
        let mut events = EventMap::new();
        events.schedule_event_like_cpp(3, 5);
        events.reschedule_event_like_cpp(3, 20);

        events.update_like_cpp(5);
        assert_eq!(events.execute_event_like_cpp(), 0);

        events.update_like_cpp(15);
        assert_eq!(events.execute_event_like_cpp(), 3);
    }
}
