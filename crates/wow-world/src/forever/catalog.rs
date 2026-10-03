//! ObjectMgr::LoadRaceAndClassExpansionRequirements, target 02245dcd:10596.
//! SQL requirements are intersected with real target DB2 presence, never with
//! the legacy hardcoded race/class combinations.

use std::collections::BTreeMap;
use wow_data::forever_character_ids::ForeverAchievementIds;
use wow_data::forever_initialization::InitializationCatalog;
use wow_persistence::forever::AvailabilityRows;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClassAvailability {
    pub id: u8,
    pub active_expansion: u8,
    pub account_expansion: u8,
    pub minimum_active_expansion: u8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RaceAvailability {
    pub id: u8,
    pub classes: Vec<ClassAvailability>,
    pub unlock_expansion: u8,
    pub unlock_achievement: u32,
}

pub struct CharacterCatalog {
    races: Vec<RaceAvailability>,
}

impl CharacterCatalog {
    pub fn load(
        identities: &InitializationCatalog,
        achievements: &ForeverAchievementIds,
        rows: AvailabilityRows,
    ) -> Result<Self, &'static str> {
        let mut races: BTreeMap<u8, BTreeMap<u8, (u8, u8)>> = BTreeMap::new();
        let mut minimum = BTreeMap::<u8, u8>::new();
        for row in rows.classes {
            let class = row.class;
            if identities.race(u32::from(row.race_id)).is_none()
                || identities.class(u32::from(class.class_id)).is_none()
                || class.active_expansion >= 12
                || class.account_expansion >= 13
            {
                continue;
            }
            if races
                .entry(row.race_id)
                .or_default()
                .insert(
                    class.class_id,
                    (class.active_expansion, class.account_expansion),
                )
                .is_some()
            {
                return Err("duplicate race/class availability");
            }
            minimum
                .entry(class.class_id)
                .and_modify(|value| *value = (*value).min(class.active_expansion))
                .or_insert(class.active_expansion);
        }
        let mut unlocks = BTreeMap::new();
        for row in rows.unlocks {
            if identities.race(u32::from(row.race_id)).is_none() || row.expansion >= 13 {
                continue;
            }
            if row.achievement_id != 0 && !achievements.contains(row.achievement_id) {
                // C++ skips an SQL unlock row whose achievement does not exist.
                continue;
            }
            if unlocks
                .insert(row.race_id, (row.expansion, row.achievement_id))
                .is_some()
            {
                return Err("duplicate race unlock requirement");
            }
            races.entry(row.race_id).or_default();
        }
        let races: Vec<_> = races
            .into_iter()
            .map(|(id, classes)| {
                let (unlock_expansion, unlock_achievement) =
                    unlocks.get(&id).copied().unwrap_or_default();
                RaceAvailability {
                    id,
                    unlock_expansion,
                    unlock_achievement,
                    classes: classes
                        .into_iter()
                        .map(
                            |(id, (active_expansion, account_expansion))| ClassAvailability {
                                id,
                                active_expansion,
                                account_expansion,
                                minimum_active_expansion: minimum[&id],
                            },
                        )
                        .collect(),
                }
            })
            .collect();
        if races.is_empty() {
            return Err("empty target availability");
        }
        Ok(Self { races })
    }

    pub fn races(&self) -> &[RaceAvailability] {
        &self.races
    }

    #[cfg(test)]
    pub(super) fn fixture() -> Self {
        Self {
            races: vec![RaceAvailability {
                id: 1,
                classes: vec![ClassAvailability {
                    id: 1,
                    active_expansion: 0,
                    account_expansion: 0,
                    minimum_active_expansion: 0,
                }],
                unlock_expansion: 0,
                unlock_achievement: 0,
            }],
        }
    }
}
