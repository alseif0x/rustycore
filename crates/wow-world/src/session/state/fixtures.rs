// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! `WorldSession::fixtures` facade; the aggregate is defined in `wow-world-core`.

#[cfg(any(test, feature = "test-fixtures"))]
pub(crate) use wow_world_core::session::state::SessionFixtures;
