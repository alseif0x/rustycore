// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use super::{PlayerAuraRemovalAccessLikeCpp, SessionCore};
use crate::session::{PlayerStatsAccessLikeCpp, SessionCatalogs, SessionWorldConfig};
#[cfg(any(test, feature = "test-fixtures"))]
use crate::session::{StatsAuraFixtureRefs, StatsCombatFixtureRefs, StatsFixtureRefs};

/// Reborrow the aura owner's selected fields only for a synchronous stats pass.
/// Construction retains references and does not read canonical state.
pub struct AuraStatsAccessBuilderLikeCpp<'a> {
    core: &'a SessionCore,
    catalogs: &'a SessionCatalogs,
    config: &'a SessionWorldConfig,
    #[cfg(any(test, feature = "test-fixtures"))]
    combat: StatsCombatFixtureRefs<'a>,
    #[cfg(any(test, feature = "test-fixtures"))]
    race: &'a u8,
    #[cfg(any(test, feature = "test-fixtures"))]
    class: &'a u8,
    #[cfg(any(test, feature = "test-fixtures"))]
    level: &'a u8,
}

impl SessionCore {
    pub fn aura_stats_access_builder_like_cpp<'a>(
        &'a self, catalogs: &'a SessionCatalogs, config: &'a SessionWorldConfig,
        #[cfg(any(test, feature = "test-fixtures"))] combat: StatsCombatFixtureRefs<'a>,
        #[cfg(any(test, feature = "test-fixtures"))] race: &'a u8,
        #[cfg(any(test, feature = "test-fixtures"))] class: &'a u8,
        #[cfg(any(test, feature = "test-fixtures"))] level: &'a u8,
    ) -> AuraStatsAccessBuilderLikeCpp<'a> {
        AuraStatsAccessBuilderLikeCpp {
            core: self, catalogs, config,
            #[cfg(any(test, feature = "test-fixtures"))] combat,
            #[cfg(any(test, feature = "test-fixtures"))] race,
            #[cfg(any(test, feature = "test-fixtures"))] class,
            #[cfg(any(test, feature = "test-fixtures"))] level,
        }
    }
}

impl AuraStatsAccessBuilderLikeCpp<'_> {
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(super) fn health_refs_like_cpp(&self) -> (&u32, &u32, &bool) {
        self.combat.health_refs_like_cpp()
    }
    pub fn reborrow_like_cpp<'a>(
        &'a mut self, aura: &'a PlayerAuraRemovalAccessLikeCpp<'_>,
        #[cfg(any(test, feature = "test-fixtures"))] form: &'a u32,
    ) -> PlayerStatsAccessLikeCpp<'a> {
        #[cfg(any(test, feature = "test-fixtures"))]
        return self.core.player_stats_access_with_fixture_refs_like_cpp(
            self.catalogs, self.config, self.race, self.class, self.level,
            StatsFixtureRefs::new_like_cpp(
                self.combat.reborrow_like_cpp(),
                StatsAuraFixtureRefs::new_like_cpp(
                    form, &*aura.fixtures.complete, &*aura.fixtures.tombstoned,
                    &*aura.fixtures.visible, &*aura.fixtures.threat,
                ),
            ),
        );
        #[cfg(not(any(test, feature = "test-fixtures")))]
        {
            let _ = aura;
            self.core.player_stats_access_like_cpp(self.catalogs, self.config)
        }
    }
}
