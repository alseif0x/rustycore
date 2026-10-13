// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Progression adapters: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.
//! Its test-only entry points moved to `handlers/progression_test_shims.rs`
//! (#1363 slice 7c).

#[cfg(test)]
use super::{WorldSession, catalogs};
#[cfg(test)]
use wow_constants::quest::QUEST_OBJECTIVE_PLAYERKILLS_LIKE_CPP;

#[cfg(test)]
#[path = "../../unit_tests/session/progression_adapters/f3_shims.rs"]
mod f3_shims;
