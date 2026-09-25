// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Time synchronization: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::{WorldSession, info, rounded_median_u32};
use std::collections::{HashMap, VecDeque};
use std::sync::OnceLock;
use std::time::Instant;

/// The single session-owned state for request sequencing and clock-delta estimation.
pub(super) struct TimeSynchronizationStateLikeCpp {
    pub(super) next_counter: u32,
    pub(super) timer_ms: u32,
    pub(super) pending_requests: HashMap<u32, u32>,
    pub(super) clock_delta_queue: VecDeque<(i64, u32)>,
    pub(super) clock_delta: i64,
}

impl Default for TimeSynchronizationStateLikeCpp {
    fn default() -> Self {
        Self {
            next_counter: 0,
            timer_ms: 0,
            pending_requests: HashMap::new(),
            clock_delta_queue: VecDeque::with_capacity(6),
            clock_delta: 0,
        }
    }
}

/// Monotonic millisecond counter matching TrinityCore's `getMSTime()` scale.
pub(crate) fn game_time_ms_like_cpp() -> u32 {
    static SERVER_START: OnceLock<Instant> = OnceLock::new();
    let start = SERVER_START.get_or_init(Instant::now);
    start.elapsed().as_millis() as u32
}

impl WorldSession {
    pub(crate) fn reset_time_sync_like_cpp(&mut self) {
        self.time_synchronization.next_counter = 0;
        self.time_synchronization.pending_requests.clear();
    }

    pub(crate) fn record_time_sync_response_like_cpp(
        &mut self,
        sequence_index: u32,
        client_time: u32,
    ) {
        let Some(server_time_at_sent) = self
            .time_synchronization
            .pending_requests
            .remove(&sequence_index)
        else {
            return;
        };

        let received_time = crate::session::game_time_ms_like_cpp();
        let round_trip_duration = received_time.wrapping_sub(server_time_at_sent);
        let lag_delay = round_trip_duration / 2;
        let clock_delta =
            i64::from(server_time_at_sent) + i64::from(lag_delay) - i64::from(client_time);
        if std::env::var_os("RUSTYCORE_LOGIN_TRACE").is_some() {
            info!(
                account = self.account_id,
                sequence_index,
                client_time,
                server_time_at_sent,
                received_time,
                round_trip_duration,
                lag_delay,
                clock_delta,
                "RUST_LOGIN_TRACE time_sync_response"
            );
        }

        if self.time_synchronization.clock_delta_queue.len() == 6 {
            self.time_synchronization.clock_delta_queue.pop_front();
        }
        self.time_synchronization
            .clock_delta_queue
            .push_back((clock_delta, round_trip_duration));
        self.compute_new_clock_delta_like_cpp();
    }

    pub(in crate::session) fn compute_new_clock_delta_like_cpp(&mut self) {
        if self.time_synchronization.clock_delta_queue.is_empty() {
            return;
        }

        let mut latencies: Vec<u32> = self
            .time_synchronization
            .clock_delta_queue
            .iter()
            .map(|(_, round_trip_duration)| *round_trip_duration)
            .collect();
        latencies.sort_unstable();
        let latency_median = rounded_median_u32(&latencies);
        let latency_mean =
            latencies.iter().map(|v| f64::from(*v)).sum::<f64>() / latencies.len() as f64;
        let latency_variance = latencies
            .iter()
            .map(|v| {
                let diff = f64::from(*v) - latency_mean;
                diff * diff
            })
            .sum::<f64>()
            / latencies.len() as f64;
        let latency_standard_deviation = latency_variance.sqrt().round() as u32;

        let latency_threshold = latency_standard_deviation.saturating_add(latency_median);
        let mut clock_delta_sum = 0i64;
        let mut sample_size_after_filtering = 0u32;
        for (clock_delta, round_trip_duration) in &self.time_synchronization.clock_delta_queue {
            if *round_trip_duration < latency_threshold {
                clock_delta_sum += *clock_delta;
                sample_size_after_filtering += 1;
            }
        }

        if sample_size_after_filtering != 0 {
            let mean_clock_delta =
                (clock_delta_sum as f64 / f64::from(sample_size_after_filtering)).round() as i64;
            if (mean_clock_delta - self.time_synchronization.clock_delta).abs() > 25 {
                self.time_synchronization.clock_delta = mean_clock_delta;
            }
        } else if self.time_synchronization.clock_delta == 0 {
            self.time_synchronization.clock_delta = self
                .time_synchronization
                .clock_delta_queue
                .back()
                .map(|(clock_delta, _)| *clock_delta)
                .unwrap_or_default();
        }
    }

    #[cfg(test)]
    pub(crate) fn set_time_sync_clock_delta_for_test_like_cpp(&mut self, clock_delta: i64) {
        self.time_synchronization.clock_delta = clock_delta;
    }
}
