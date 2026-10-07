// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Cinematic adapter: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.
//! The opening-cinematic selection moved to the `wow-world-application`
//! character owner (#1263 F5); this module keeps its cfg(test) shims.

#[cfg(test)]
#[path = "../../unit_tests/session/cinematic_adapter/f3_shims.rs"]
mod f3_shims;
