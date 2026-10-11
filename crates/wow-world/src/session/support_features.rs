// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Support features: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::{FeatureSystemStatus, FeatureSystemStatusGlueScreen, WorldSession};

#[cfg(test)]
pub(crate) mod test_fixtures;

#[cfg(test)]
#[path = "../../unit_tests/session/support_features/f3_shims.rs"]
mod f3_shims;
