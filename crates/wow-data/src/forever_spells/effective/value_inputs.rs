//! Borrowed access to canonical, final spell-value input rows.
use super::*;
impl SpellCatalog {
    pub fn expected_stat(&self, id: u32) -> Option<&ExpectedStatRecord> {
        self.expected_stats.get(&id)
    }
    pub fn expected_stat_records(&self) -> impl Iterator<Item = &ExpectedStatRecord> {
        self.expected_stats.values()
    }
    pub fn expected_stat_mod(&self, id: u32) -> Option<&ExpectedStatModRecord> {
        self.expected_stat_mods.get(&id)
    }
    pub fn expected_stat_mod_records(&self) -> impl Iterator<Item = &ExpectedStatModRecord> {
        self.expected_stat_mods.values()
    }
    pub fn content_tuning(&self, id: u32) -> Option<&ContentTuningRecord> {
        self.content_tunings.get(&id)
    }
    pub fn content_tuning_records(&self) -> impl Iterator<Item = &ContentTuningRecord> {
        self.content_tunings.values()
    }
    /// CalcBaseValue uses only ExpansionID here. Missing is source -2;
    /// unknown encrypted coverage cannot establish that fallback.
    pub fn content_tuning_expansion(&self, id: u32) -> Result<i32> {
        ensure!(
            self.unknown_baseline_records[44] == 0,
            "Incomplete content-tuning input"
        );
        Ok(self.content_tuning(id).map_or(-2, |row| row.expansion_id))
    }
    pub fn content_tuning_x_expected(&self, id: u32) -> Option<&ContentTuningXExpectedRecord> {
        self.content_tuning_x_expected.get(&id)
    }
    pub fn content_tuning_x_expected_records(
        &self,
    ) -> impl Iterator<Item = &ContentTuningXExpectedRecord> {
        self.content_tuning_x_expected.values()
    }
    pub fn rand_prop_points(&self, id: u32) -> Option<&RandPropPointsRecord> {
        self.rand_prop_points.get(&id)
    }
    pub fn rand_prop_points_records(&self) -> impl Iterator<Item = &RandPropPointsRecord> {
        self.rand_prop_points.values()
    }
    /// SpellInfo.cpp:687-692: LookupEntry, then AssertEntry(GetNumRows()-1).
    /// A removed/unresolved last slot must fail, never select an earlier row.
    /// This source assert/overflow boundary is an explicit error, not zero.
    pub fn rand_prop_points_or_last(&self, id: u32) -> Result<&RandPropPointsRecord> {
        ensure!(
            self.unknown_baseline_records[46] == 0,
            "Incomplete random-property input"
        );
        ensure!(
            self.rand_prop_points_storage_last_index != u32::MAX,
            "Random-property storage size overflows source uint32"
        );
        self.rand_prop_points(id)
            .or_else(|| self.rand_prop_points(self.rand_prop_points_storage_last_index))
            .ok_or_else(|| anyhow::anyhow!("Required random-property last slot absent"))
    }
    pub fn mythic_plus_season(&self, id: u32) -> Option<&MythicPlusSeasonRecord> {
        self.mythic_plus_seasons.get(&id)
    }
    pub fn mythic_plus_season_records(&self) -> impl Iterator<Item = &MythicPlusSeasonRecord> {
        self.mythic_plus_seasons.values()
    }
}
