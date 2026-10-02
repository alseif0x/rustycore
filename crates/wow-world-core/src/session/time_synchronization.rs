// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Time synchronization state and monotonic game-time source.

use std::collections::{HashMap, VecDeque};
use std::sync::OnceLock;
use std::time::Instant;

/// The single session-owned state for request sequencing and clock-delta estimation.
pub struct TimeSynchronizationStateLikeCpp {
    pub next_counter: u32,
    pub timer_ms: u32,
    pub pending_requests: HashMap<u32, u32>,
    pub clock_delta_queue: VecDeque<(i64, u32)>,
    pub clock_delta: i64,
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
pub fn game_time_ms_like_cpp() -> u32 {
    static SERVER_START: OnceLock<Instant> = OnceLock::new();
    let start = SERVER_START.get_or_init(Instant::now);
    start.elapsed().as_millis() as u32
}
