use std::sync::Arc;

use wow_data::character_progression::PowerTypeStore;
use wow_data::{
    CreatureAddonStoreLikeCpp, CreatureBaseStatsStoreLikeCpp,
    CreatureClassificationHealthRatesLikeCpp, CreatureDifficultyStoreLikeCpp,
    CreatureEquipmentStoreLikeCpp,
};

/// Process-owned C++ `ObjectMgr` creature materialization catalogs and
/// `World` creature-health policy.
///
/// C++ resolves these through `sObjectMgr`/`sWorld` while `Creature::InitEntry`,
/// `UpdateLevelDependantStats`, `LoadEquipment`, and `GetCreatureAddon` build a
/// creature (`Creature.cpp:491-615,1550-1615,1931-1965,2722-2755`). A
/// `WorldSession` borrows them only while adapting visibility; it owns none of
/// the stores or rates.
pub struct CreatureSpawnCatalogsLikeCpp {
    pub difficulty: Arc<CreatureDifficultyStoreLikeCpp>,
    pub base_stats: Arc<CreatureBaseStatsStoreLikeCpp>,
    pub health_rates: CreatureClassificationHealthRatesLikeCpp,
    pub addons: Arc<CreatureAddonStoreLikeCpp>,
    pub equipment: Arc<CreatureEquipmentStoreLikeCpp>,
    pub power_types: Arc<PowerTypeStore>,
}

#[cfg(any(test, feature = "test-fixtures"))]
impl Default for CreatureSpawnCatalogsLikeCpp {
    fn default() -> Self {
        Self {
            difficulty: Arc::new(CreatureDifficultyStoreLikeCpp::default()),
            base_stats: Arc::new(CreatureBaseStatsStoreLikeCpp::default()),
            health_rates: CreatureClassificationHealthRatesLikeCpp::default(),
            addons: Arc::new(CreatureAddonStoreLikeCpp::default()),
            equipment: Arc::new(CreatureEquipmentStoreLikeCpp::default()),
            power_types: Arc::new(PowerTypeStore::from_entries([])),
        }
    }
}
