// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use crate::session::{PlayerStatsAccessLikeCpp, SessionCatalogs, SessionWorldConfig};
#[cfg(any(test, feature = "test-fixtures"))]
use crate::session::StatsFixtureRefs;
use super::LootReleaseOwnerAccessLikeCpp;

/// Inert stats inputs, kept separate from the release's mutable Core borrow.
pub struct LootReleaseStatsInputsLikeCpp<'a> {
    catalogs: &'a SessionCatalogs,
    config: &'a SessionWorldConfig,
    #[cfg(any(test, feature = "test-fixtures"))]
    race: &'a u8,
    #[cfg(any(test, feature = "test-fixtures"))]
    class: &'a u8,
    #[cfg(any(test, feature = "test-fixtures"))]
    level: &'a u8,
    #[cfg(any(test, feature = "test-fixtures"))]
    fixtures: StatsFixtureRefs<'a>,
}

impl<'a> LootReleaseStatsInputsLikeCpp<'a> {
    pub fn corpse_decay_looted_rate_like_cpp(&self) -> f32 {
        self.config.loot_drop_rates_like_cpp().corpse_decay_looted
    }

    pub fn new_like_cpp(
        catalogs: &'a SessionCatalogs,
        config: &'a SessionWorldConfig,
        #[cfg(any(test, feature = "test-fixtures"))] race: &'a u8,
        #[cfg(any(test, feature = "test-fixtures"))] class: &'a u8,
        #[cfg(any(test, feature = "test-fixtures"))] level: &'a u8,
        #[cfg(any(test, feature = "test-fixtures"))] fixtures: StatsFixtureRefs<'a>,
    ) -> Self {
        Self {
            catalogs, config,
            #[cfg(any(test, feature = "test-fixtures"))]
            race,
            #[cfg(any(test, feature = "test-fixtures"))]
            class,
            #[cfg(any(test, feature = "test-fixtures"))]
            level,
            #[cfg(any(test, feature = "test-fixtures"))]
            fixtures,
        }
    }
}

impl LootReleaseOwnerAccessLikeCpp<'_> {
    pub fn stats_like_cpp<'a>(
        &'a self, inputs: &'a mut LootReleaseStatsInputsLikeCpp<'_>,
    ) -> PlayerStatsAccessLikeCpp<'a> {
        #[cfg(any(test, feature = "test-fixtures"))]
        {
            self.core.player_stats_access_with_fixture_refs_like_cpp(
                inputs.catalogs, inputs.config, inputs.race, inputs.class,
                inputs.level, inputs.fixtures.reborrow_like_cpp(),
            )
        }
        #[cfg(not(any(test, feature = "test-fixtures")))]
        {
            self.core.player_stats_access_like_cpp(inputs.catalogs, inputs.config)
        }
    }
}
