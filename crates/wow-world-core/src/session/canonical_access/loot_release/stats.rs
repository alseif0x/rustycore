// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use crate::session::{PlayerStatsAccessLikeCpp, SessionCatalogs, SessionWorldConfig};
#[cfg(any(test, feature = "test-fixtures"))]
use crate::session::StatsFixtureRefs;
use super::LootReleaseOwnerAccessLikeCpp;

/// Inert catalogs/config inputs, kept separate from the release's mutable Core
/// borrow. The cfg(test) stats fixtures stay with the caller so one mutable
/// fixture owner can serve the release's stats pass and its readonly
/// projections without aliasing.
pub struct LootReleaseStatsInputsLikeCpp<'a> {
    catalogs: &'a SessionCatalogs,
    config: &'a SessionWorldConfig,
}

impl<'a> LootReleaseStatsInputsLikeCpp<'a> {
    pub fn corpse_decay_looted_rate_like_cpp(&self) -> f32 {
        self.config.loot_drop_rates_like_cpp().corpse_decay_looted
    }

    pub fn catalogs_like_cpp(&self) -> &'a SessionCatalogs {
        self.catalogs
    }

    pub fn config_like_cpp(&self) -> &'a SessionWorldConfig {
        self.config
    }

    pub fn new_like_cpp(catalogs: &'a SessionCatalogs, config: &'a SessionWorldConfig) -> Self {
        Self { catalogs, config }
    }
}

impl LootReleaseOwnerAccessLikeCpp<'_> {
    pub fn stats_like_cpp<'a>(
        &'a self,
        inputs: &'a mut LootReleaseStatsInputsLikeCpp<'_>,
        #[cfg(any(test, feature = "test-fixtures"))] race: &'a u8,
        #[cfg(any(test, feature = "test-fixtures"))] class: &'a u8,
        #[cfg(any(test, feature = "test-fixtures"))] level: &'a u8,
        #[cfg(any(test, feature = "test-fixtures"))] fixtures: StatsFixtureRefs<'a>,
    ) -> PlayerStatsAccessLikeCpp<'a> {
        #[cfg(any(test, feature = "test-fixtures"))]
        {
            self.core.player_stats_access_with_fixture_refs_like_cpp(
                inputs.catalogs, inputs.config, race, class, level, fixtures,
            )
        }
        #[cfg(not(any(test, feature = "test-fixtures")))]
        {
            let _ = &mut *inputs;
            self.core.player_stats_access_like_cpp(inputs.catalogs, inputs.config)
        }
    }
}
