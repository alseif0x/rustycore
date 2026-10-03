//! 02245dcd DB2Stores.cpp:1296-1298,1327-1328,2502-2639;
//! ItemEnchantmentMgr.cpp:107-176. Complete source numeric operations on
//! canonical inputs, not CalcValue/custom/Player readiness.
use super::*;
#[cfg(test)]
mod tests;

impl SpellCatalog {
    pub(super) fn build_value_indexes(&mut self) {
        // Source DB2Storage visits ascending final ID; last matching level /
        // expansion wins. ContentSetID is not an extra filter or key.
        for (&id, row) in &self.expected_stats {
            self.expected_stat_by_level
                .insert((row.lvl, row.expansion_id), id);
        }
        for (&id, row) in &self.content_tuning_x_expected {
            if self
                .expected_stat_mods
                .contains_key(&(row.expected_stat_mod_id as u32))
            {
                self.expected_mods_by_content_tuning
                    .entry(row.content_tuning_id)
                    .or_default()
                    .push(id);
            }
        }
    }

    /// DB2Manager::EvaluateExpectedStat, including genuine missing-row 1.0
    /// before stat selection. Unknown encrypted input is an error, not absence.
    /// f32 product order is source-observable; do not regroup base into fold.
    pub fn evaluate_expected_stat(
        &self,
        stat: ExpectedStatType,
        level: u32,
        expansion: i32,
        content_tuning_id: u32,
        unit_class: u32,
        active_milestone_season: i32,
    ) -> Result<f32> {
        ensure!(
            [42, 43, 45, 47]
                .into_iter()
                .all(|i| self.unknown_baseline_records[i] == 0),
            "Incomplete spell expected-stat inputs"
        );
        let Some(id) = self
            .expected_stat_by_level
            .get(&(level, expansion))
            .or_else(|| self.expected_stat_by_level.get(&(level, -2)))
        else {
            return Ok(1.0);
        };
        let baseline = &self.expected_stats[id];
        if matches!(stat, ExpectedStatType::None) {
            return Ok(0.0);
        }
        let mut value = baseline.value(stat);
        if let Some(mods) = self.expected_mods_by_content_tuning.get(&content_tuning_id) {
            let mut product = 1.0f32;
            for id in mods {
                let relation = &self.content_tuning_x_expected[id];
                if relation.min_mythic_plus_season_id != 0 {
                    if let Some(season) =
                        self.mythic_plus_season(relation.min_mythic_plus_season_id as u32)
                    {
                        if active_milestone_season < season.milestone_season {
                            continue;
                        }
                    }
                }
                if relation.max_mythic_plus_season_id != 0 {
                    if let Some(season) =
                        self.mythic_plus_season(relation.max_mythic_plus_season_id as u32)
                    {
                        if active_milestone_season >= season.milestone_season {
                            continue;
                        }
                    }
                }
                product *=
                    self.expected_stat_mods[&(relation.expected_stat_mod_id as u32)].value(stat);
            }
            value *= product;
        }
        let class_mod_id = match unit_class {
            1 => Some(4),
            2 => Some(2),
            4 => Some(3),
            8 => Some(1),
            _ => None,
        };
        if let Some(modifier) = class_mod_id.and_then(|id| self.expected_stat_mod(id)) {
            value *= modifier.value(stat);
        }
        Ok(value)
    }

    /// GetRandomPropertyPoints; same input widths and genuine missing/unknown
    /// distinction. Quality Rare+Chest selects SuperiorF[0] at null-caster startup.
    pub fn random_property_points(
        &self,
        item_level: u32,
        quality: u32,
        inventory_type: u32,
        subclass: u32,
    ) -> Result<f32> {
        ensure!(
            self.unknown_baseline_records[46] == 0,
            "Incomplete random-property input"
        );
        let index = match inventory_type {
            1 | 4 | 5 | 7 | 15 | 17 | 20 | 25 => 0,
            26 => {
                if subclass == 19 {
                    3
                } else {
                    0
                }
            }
            13 | 21 | 22 => 3,
            3 | 6 | 8 | 10 | 12 => 1,
            2 | 9 | 11 | 14 | 16 | 23 => 2,
            28 => 4,
            _ => return Ok(0.0),
        };
        ensure!(
            self.rand_prop_points_storage_last_index != u32::MAX,
            "Random-property storage size overflows source uint32"
        );
        let Some(row) = self.rand_prop_points(item_level) else {
            return Ok(0.0);
        };
        Ok(match quality {
            2 => row.good_f[index],
            3 | 7 => row.superior_f[index],
            4 | 5 | 6 => row.epic_f[index],
            _ => 0.0,
        })
    }
}

impl ExpectedStatRecord {
    fn value(&self, stat: ExpectedStatType) -> f32 {
        match stat {
            ExpectedStatType::CreatureHealth => self.creature_health,
            ExpectedStatType::PlayerHealth => self.player_health,
            ExpectedStatType::CreatureAutoAttackDps => self.creature_auto_attack_dps,
            ExpectedStatType::CreatureArmor => self.creature_armor,
            ExpectedStatType::PlayerMana => self.player_mana,
            ExpectedStatType::PlayerPrimaryStat => self.player_primary_stat,
            ExpectedStatType::PlayerSecondaryStat => self.player_secondary_stat,
            ExpectedStatType::ArmorConstant => self.armor_constant,
            ExpectedStatType::None => 0.0,
            ExpectedStatType::CreatureSpellDamage => self.creature_spell_damage,
        }
    }
}
impl ExpectedStatModRecord {
    fn value(&self, stat: ExpectedStatType) -> f32 {
        match stat {
            ExpectedStatType::CreatureHealth => self.creature_health_mod,
            ExpectedStatType::PlayerHealth => self.player_health_mod,
            ExpectedStatType::CreatureAutoAttackDps => self.creature_auto_attack_dps_mod,
            ExpectedStatType::CreatureArmor => self.creature_armor_mod,
            ExpectedStatType::PlayerMana => self.player_mana_mod,
            ExpectedStatType::PlayerPrimaryStat => self.player_primary_stat_mod,
            ExpectedStatType::PlayerSecondaryStat => self.player_secondary_stat_mod,
            ExpectedStatType::ArmorConstant => self.armor_constant_mod,
            ExpectedStatType::None => 0.0,
            ExpectedStatType::CreatureSpellDamage => self.creature_spell_damage_mod,
        }
    }
}
