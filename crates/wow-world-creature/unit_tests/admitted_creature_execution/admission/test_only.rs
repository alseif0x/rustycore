// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1363 slice 1: production-dead `admission` entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl IsolatedCreatureExecutionOutcomeLikeCpp {
    /// Number of publication events this transition produced.
    #[must_use]
    pub fn publication_events_like_cpp(&self) -> usize {
        self.deferred_publication.len()
    }
}
