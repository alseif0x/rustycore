// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3

//! C++ `ObjectMgr::LoadPlayerInfo` player stat data.
//!
//! Primary stats come from `player_classlevelstats` plus the signed
//! `player_racestats` modifiers. Base mana comes from the client
//! `gt/BaseMp.txt` GameTable through `ObjectMgr::GetPlayerClassLevelInfo`.
//! C++ does not consume the legacy C# `player_levelstats.basehp/basemana`
//! projection.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use anyhow::{Context, Result, bail};
use tracing::info;
use wow_database::{WorldDatabase, WorldStatements};

use crate::BaseMpGameTableLikeCpp;

/// C++ `PlayerLevelInfo` plus the class/level `GtBaseMP` value.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PlayerLevelStats {
    pub strength: u16,
    pub agility: u16,
    pub stamina: u16,
    pub intellect: u16,
    pub spirit: u16,
    pub base_mana: u32,
}

impl PlayerLevelStats {
    pub const fn primary_stats_like_cpp(&self) -> [u16; 5] {
        [
            self.strength,
            self.agility,
            self.stamina,
            self.intellect,
            self.spirit,
        ]
    }
}

/// Inputs currently represented by Rust for C++ `Player::UpdateAllStats`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayerStatSystemInputLikeCpp {
    pub base: PlayerLevelStats,
    pub class: u8,
    pub level: u8,
    pub attack_power_per_strength: u8,
    pub attack_power_per_agility: u8,
    pub ranged_attack_power_per_agility: u8,
    pub gear_stats: [i32; 5],
    pub gear_health: i32,
    pub gear_mana: i32,
    pub gear_armor: i32,
    pub gear_attack_power: i32,
    pub gear_ranged_attack_power: i32,
    pub rating_bonuses: [f32; 32],
    pub can_parry: bool,
    pub can_block: bool,
}

/// C++-shaped result of the represented `Player::UpdateAllStats` inputs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayerStatSystemProjectionLikeCpp {
    pub stats: [i32; 5],
    pub stat_pos_buff: [i32; 5],
    pub stat_neg_buff: [i32; 5],
    pub create_health: i32,
    pub base_mana: i32,
    pub max_health: i64,
    pub max_mana: i64,
    pub armor: i32,
    pub attack_power: i32,
    pub attack_power_mod_pos: i32,
    pub ranged_attack_power: i32,
    pub ranged_attack_power_mod_pos: i32,
    pub total_attack_power: i32,
    pub total_ranged_attack_power: i32,
    pub block_pct: f32,
    pub dodge_pct: f32,
    pub dodge_from_attr: f32,
    pub parry_pct: f32,
    pub parry_from_attr: f32,
    pub crit_pct: f32,
    pub ranged_crit_pct: f32,
    pub offhand_crit_pct: f32,
    pub spell_crit_pct: [f32; 7],
}

const DIMINISHING_K_LIKE_CPP: [f32; 14] = [
    0.9560, 0.9560, 0.9880, 0.9880, 0.9830, 0.9560, 0.9880, 0.9830, 0.9830, 0.9830, 0.9720, 0.9830,
    0.9880, 1.0,
];
const PARRY_CAP_LIKE_CPP: [f32; 14] = [
    65.631440, 65.631440, 145.560408, 145.560408, 0.0, 65.631440, 145.560408, 0.0, 0.0, 90.6425,
    0.0, 65.631440, 0.0, 0.0,
];
const DODGE_CAP_LIKE_CPP: [f32; 14] = [
    65.631440, 65.631440, 145.560408, 145.560408, 150.375940, 65.631440, 145.560408, 150.375940,
    150.375940, 145.560408, 116.890707, 145.560408, 145.560408, 0.0,
];

fn diminishing_returns_like_cpp(
    cap: &[f32; 14],
    class: u8,
    non_diminishing: f32,
    diminishing: f32,
) -> f32 {
    let Some(index) = class.checked_sub(1).map(usize::from) else {
        return non_diminishing;
    };
    let Some((&cap, &k)) = cap.get(index).zip(DIMINISHING_K_LIKE_CPP.get(index)) else {
        return non_diminishing;
    };
    if cap == 0.0 {
        return non_diminishing;
    }
    cap * diminishing / (diminishing + cap * k) + non_diminishing
}

fn health_bonus_from_stamina_like_cpp(stamina: i32) -> i64 {
    let stamina = i64::from(stamina);
    stamina.min(20) + (stamina - 20).max(0) * 10
}

fn mana_bonus_from_intellect_like_cpp(intellect: i32) -> i64 {
    let intellect = i64::from(intellect);
    intellect.min(20) + (intellect - 20).max(0) * 15
}

/// Represent the exact no-aura branches of C++ `Player::UpdateAllStats`.
///
/// Item flat modifiers and combat ratings are included. Aura percentage
/// modifiers remain owned by the separate represented aura runtime.
pub fn calculate_player_stat_system_like_cpp(
    input: PlayerStatSystemInputLikeCpp,
) -> PlayerStatSystemProjectionLikeCpp {
    let base_stats = input.base.primary_stats_like_cpp().map(i32::from);
    let stats =
        std::array::from_fn(|index| base_stats[index].saturating_add(input.gear_stats[index]));
    let stat_pos_buff = input.gear_stats.map(|value| value.max(0));
    let stat_neg_buff = input.gear_stats.map(|value| value.min(0));

    let max_health =
        i64::from(input.gear_health).saturating_add(health_bonus_from_stamina_like_cpp(stats[2]));
    let base_mana = i32::try_from(input.base.base_mana).unwrap_or(i32::MAX);
    let mana_bonus = if base_mana > 0 {
        mana_bonus_from_intellect_like_cpp(stats[3])
    } else {
        0
    };
    let max_mana = i64::from(base_mana)
        .saturating_add(i64::from(input.gear_mana))
        .saturating_add(mana_bonus);

    let class_specific_attack_power = match input.class {
        1 | 2 | 6 => f32::from(input.level) * 3.0 - 20.0,
        3 | 4 | 7 | 11 => f32::from(input.level) * 2.0 - 20.0,
        _ => -20.0,
    };
    let attack_power = ((stats[0] as f32 * f32::from(input.attack_power_per_strength)).max(0.0)
        + (stats[1] as f32 * f32::from(input.attack_power_per_agility)).max(0.0)
        + class_specific_attack_power) as i32;
    let ranged_attack_power = ((f32::from(input.level) + (stats[1] as f32).max(0.0))
        * f32::from(input.ranged_attack_power_per_agility)
        - 10.0) as i32;

    let rating = |index: usize| input.rating_bonuses.get(index).copied().unwrap_or(0.0);
    let crit_pct = 5.0 + rating(8);
    let ranged_crit_pct = 5.0 + rating(9);
    let spell_crit = 5.0 + rating(10);
    let dodge_pct = diminishing_returns_like_cpp(&DODGE_CAP_LIKE_CPP, input.class, 0.0, rating(2));
    let parry_pct = if input.can_parry
        && PARRY_CAP_LIKE_CPP
            .get(usize::from(input.class.saturating_sub(1)))
            .is_some_and(|cap| *cap > 0.0)
    {
        diminishing_returns_like_cpp(&PARRY_CAP_LIKE_CPP, input.class, 5.0, rating(3))
    } else {
        0.0
    };
    let block_pct = if input.can_block {
        5.0 + rating(4)
    } else {
        0.0
    };

    PlayerStatSystemProjectionLikeCpp {
        stats,
        stat_pos_buff,
        stat_neg_buff,
        create_health: 0,
        base_mana,
        max_health,
        max_mana,
        armor: stats[1].saturating_mul(2).saturating_add(input.gear_armor),
        attack_power,
        attack_power_mod_pos: input.gear_attack_power,
        ranged_attack_power,
        ranged_attack_power_mod_pos: input
            .gear_attack_power
            .saturating_add(input.gear_ranged_attack_power),
        total_attack_power: attack_power.saturating_add(input.gear_attack_power),
        total_ranged_attack_power: ranged_attack_power
            .saturating_add(input.gear_attack_power)
            .saturating_add(input.gear_ranged_attack_power),
        block_pct,
        dodge_pct,
        dodge_from_attr: 0.0,
        parry_pct,
        parry_from_attr: 0.0,
        crit_pct,
        ranged_crit_pct,
        offhand_crit_pct: crit_pct,
        spell_crit_pct: [spell_crit; 7],
    }
}

/// In-memory C++ player level information keyed by `(race, class, level)`.
pub struct PlayerStatsStore {
    stats: HashMap<(u8, u8, u8), PlayerLevelStats>,
}

impl PlayerStatsStore {
    /// Load the exact C++ sources used by `ObjectMgr::LoadPlayerInfo`.
    pub async fn load(
        world_db: &WorldDatabase,
        data_dir: impl AsRef<Path>,
        max_player_level: u8,
        valid_race_classes: &[(u8, u8)],
    ) -> Result<Self> {
        let race_stmt = world_db.prepare(WorldStatements::SEL_PLAYER_RACESTATS);
        let mut race_result = world_db
            .query(&race_stmt)
            .await
            .context("Failed to query player_racestats")?;
        let mut race_rows = Vec::new();
        if !race_result.is_empty() {
            loop {
                race_rows.push((
                    race_result.read::<u8>(0),
                    [
                        race_result.read::<i16>(1),
                        race_result.read::<i16>(2),
                        race_result.read::<i16>(3),
                        race_result.read::<i16>(4),
                        race_result.read::<i16>(5),
                    ],
                ));
                if !race_result.next_row() {
                    break;
                }
            }
        }
        if race_rows.is_empty() {
            bail!("Loaded 0 race stats definitions: player_racestats is empty");
        }

        let class_stmt = world_db.prepare(WorldStatements::SEL_PLAYER_CLASSLEVELSTATS);
        let mut class_result = world_db
            .query(&class_stmt)
            .await
            .context("Failed to query player_classlevelstats")?;
        let mut class_rows = Vec::new();
        if !class_result.is_empty() {
            loop {
                let read_stat_like_cpp = |column| {
                    class_result
                        .try_read::<u16>(column)
                        .or_else(|| {
                            class_result
                                .try_read::<i16>(column)
                                .map(|value| value as u16)
                        })
                        .with_context(|| {
                            format!(
                                "Failed to decode player_classlevelstats column {column} as uint16"
                            )
                        })
                };
                class_rows.push((
                    class_result.read::<u8>(0),
                    class_result.read::<u8>(1),
                    [
                        read_stat_like_cpp(2)?,
                        read_stat_like_cpp(3)?,
                        read_stat_like_cpp(4)?,
                        read_stat_like_cpp(5)?,
                        read_stat_like_cpp(6)?,
                    ],
                ));
                if !class_result.next_row() {
                    break;
                }
            }
        }
        if class_rows.is_empty() {
            bail!("Loaded 0 level stats definitions: player_classlevelstats is empty");
        }

        let base_mp = BaseMpGameTableLikeCpp::load(data_dir)
            .context("Failed to load gt/BaseMp.txt for player class-level stats")?;
        let store = Self::from_cpp_sources(
            valid_race_classes.iter().copied(),
            race_rows,
            class_rows,
            &base_mp,
            max_player_level,
        )?;
        info!(
            "Loaded {} C++ player race/class/level stat entries",
            store.len()
        );
        Ok(store)
    }

    /// Build the same combined rows as C++ `ObjectMgr::LoadPlayerInfo`.
    ///
    /// Missing class-level rows after level 1 inherit the previous level.
    /// A class represented in the input without level-1 data is rejected,
    /// matching C++'s fatal integrity check for playable combinations.
    pub fn from_cpp_sources(
        valid_race_classes: impl IntoIterator<Item = (u8, u8)>,
        race_rows: impl IntoIterator<Item = (u8, [i16; 5])>,
        class_rows: impl IntoIterator<Item = (u8, u8, [u16; 5])>,
        base_mp: &BaseMpGameTableLikeCpp,
        max_player_level: u8,
    ) -> Result<Self> {
        if max_player_level == 0 {
            bail!("CONFIG_MAX_PLAYER_LEVEL must be at least 1");
        }

        let valid_race_classes: HashSet<(u8, u8)> = valid_race_classes.into_iter().collect();
        if valid_race_classes.is_empty() {
            bail!("playercreateinfo has no valid race/class combinations");
        }

        let race_modifiers: HashMap<u8, [i16; 5]> = race_rows.into_iter().collect();
        if race_modifiers.is_empty() {
            bail!("player_racestats is empty");
        }

        let mut class_level_stats = HashMap::new();
        for (class, level, primary_stats) in class_rows {
            if level == 0 || level > max_player_level {
                continue;
            }
            class_level_stats.insert((class, level), primary_stats);
        }
        if class_level_stats.is_empty() {
            bail!("player_classlevelstats has no rows within the configured level range");
        }

        let required_classes: HashSet<u8> =
            valid_race_classes.iter().map(|&(_, class)| class).collect();
        for &class in &required_classes {
            let Some(mut previous) = class_level_stats.get(&(class, 1)).copied() else {
                bail!("Class {class} level 1 does not have stats data");
            };
            for level in 2..=max_player_level {
                match class_level_stats.get(&(class, level)).copied() {
                    Some(stats) if stats[0] != 0 => previous = stats,
                    _ => {
                        class_level_stats.insert((class, level), previous);
                    }
                }
            }
        }

        let mut stats = HashMap::new();
        for (race, class) in valid_race_classes {
            let race_modifiers = race_modifiers.get(&race).copied().unwrap_or([0; 5]);
            for level in 1..=max_player_level {
                let Some(class_stats) = class_level_stats.get(&(class, level)) else {
                    continue;
                };
                let combined: [u16; 5] = std::array::from_fn(|index| {
                    // C++ assigns the promoted `uint16 + int16` result back
                    // to `uint16`. Valid world rows remain non-negative.
                    (i32::from(class_stats[index]) + i32::from(race_modifiers[index])) as u16
                });
                if level == 1 && combined[0] == 0 {
                    bail!("Race {race} Class {class} Level 1 does not have stats data");
                }
                stats.insert(
                    (race, class, level),
                    PlayerLevelStats {
                        strength: combined[0],
                        agility: combined[1],
                        stamina: combined[2],
                        intellect: combined[3],
                        spirit: combined[4],
                        base_mana: base_mp.base_mana_like_cpp(class, level).unwrap_or(0),
                    },
                );
            }
        }

        Ok(Self { stats })
    }

    pub fn get(&self, race: u8, class: u8, level: u8) -> Option<&PlayerLevelStats> {
        self.stats.get(&(race, class, level))
    }

    /// Test/fixture constructor for already-combined C++ rows.
    pub fn from_entries(
        entries: impl IntoIterator<Item = ((u8, u8, u8), PlayerLevelStats)>,
    ) -> Self {
        Self {
            stats: entries.into_iter().collect(),
        }
    }

    pub fn len(&self) -> usize {
        self.stats.len()
    }

    pub fn is_empty(&self) -> bool {
        self.stats.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::BaseMpEntryLikeCpp;

    fn base_mp_fixture() -> BaseMpGameTableLikeCpp {
        BaseMpGameTableLikeCpp::from_rows([
            BaseMpEntryLikeCpp::from_columns([
                0.0, 31.0, 0.0, 155.0, 31.0, 155.0, 31.0, 155.0, 0.0, 0.0, 155.0, 0.0,
            ]),
            BaseMpEntryLikeCpp::from_columns([
                0.0, 34.0, 0.0, 170.0, 34.0, 170.0, 34.0, 170.0, 0.0, 0.0, 170.0, 0.0,
            ]),
        ])
    }

    #[test]
    fn combines_class_stats_race_modifiers_and_base_mp_like_cpp() {
        let store = PlayerStatsStore::from_cpp_sources(
            [(1, 5), (2, 5)],
            [(1, [3, -1, 2, 4, 0]), (2, [-2, 1, 0, -3, 5])],
            [(5, 1, [10, 11, 12, 13, 14])],
            &base_mp_fixture(),
            1,
        )
        .expect("valid C++ sources");

        assert_eq!(
            store.get(1, 5, 1),
            Some(&PlayerLevelStats {
                strength: 13,
                agility: 10,
                stamina: 14,
                intellect: 17,
                spirit: 14,
                base_mana: 155,
            })
        );
        assert_eq!(
            store.get(2, 5, 1).unwrap().primary_stats_like_cpp(),
            [8, 12, 12, 10, 19]
        );
    }

    #[test]
    fn fills_missing_class_level_from_previous_level_like_cpp() {
        let store = PlayerStatsStore::from_cpp_sources(
            [(1, 11)],
            [(1, [0; 5])],
            [(11, 1, [1, 2, 3, 4, 5]), (11, 3, [10, 20, 30, 40, 50])],
            &base_mp_fixture(),
            3,
        )
        .expect("valid gapped C++ sources");

        assert_eq!(
            store.get(1, 11, 2).unwrap().primary_stats_like_cpp(),
            [1, 2, 3, 4, 5]
        );
        assert_eq!(
            store.get(1, 11, 3).unwrap().primary_stats_like_cpp(),
            [10, 20, 30, 40, 50]
        );
        assert_eq!(store.get(1, 11, 2).unwrap().base_mana, 34);
        assert_eq!(store.get(1, 11, 3).unwrap().base_mana, 0);
    }

    #[test]
    fn rejects_class_without_level_one_like_cpp() {
        let error = PlayerStatsStore::from_cpp_sources(
            [(1, 5)],
            [(1, [0; 5])],
            [(5, 2, [10; 5])],
            &base_mp_fixture(),
            2,
        )
        .err()
        .expect("missing level 1 must fail");

        assert!(error.to_string().contains("level 1"));
    }

    #[test]
    fn only_builds_and_validates_playercreateinfo_combinations_like_cpp() {
        let store = PlayerStatsStore::from_cpp_sources(
            [(1, 5)],
            [(2, [50; 5])],
            [
                (5, 1, [10, 11, 12, 13, 14]),
                // This unused class has no level-1 row. C++ never allocates
                // levelInfo for it without a matching playercreateinfo row.
                (8, 2, [20; 5]),
            ],
            &base_mp_fixture(),
            2,
        )
        .expect("unused race/class combinations must not affect integrity");

        assert_eq!(
            store.get(1, 5, 1).unwrap().primary_stats_like_cpp(),
            [10, 11, 12, 13, 14],
            "a race missing from player_racestats uses C++ zero modifiers"
        );
        assert!(store.get(2, 5, 1).is_none());
        assert!(store.get(1, 8, 2).is_none());
    }

    #[test]
    fn stat_system_uses_create_health_zero_base_mp_and_chrclasses_ap_coefficients() {
        let projection = calculate_player_stat_system_like_cpp(PlayerStatSystemInputLikeCpp {
            base: PlayerLevelStats {
                strength: 10,
                agility: 12,
                stamina: 30,
                intellect: 40,
                spirit: 20,
                base_mana: 155,
            },
            class: 5,
            level: 80,
            attack_power_per_strength: 0,
            attack_power_per_agility: 0,
            ranged_attack_power_per_agility: 0,
            gear_stats: [0, 0, 5, 3, 0],
            gear_health: 100,
            gear_mana: 50,
            gear_armor: 25,
            gear_attack_power: 17,
            gear_ranged_attack_power: 4,
            rating_bonuses: [0.0; 32],
            can_parry: false,
            can_block: false,
        });

        assert_eq!(projection.create_health, 0);
        assert_eq!(projection.max_health, 100 + 20 + 15 * 10);
        assert_eq!(projection.base_mana, 155);
        assert_eq!(projection.max_mana, 155 + 50 + 20 + 23 * 15);
        assert_eq!(projection.armor, 12 * 2 + 25);
        assert_eq!(projection.attack_power, -20);
        assert_eq!(projection.attack_power_mod_pos, 17);
        assert_eq!(projection.total_attack_power, -3);
        assert_eq!(projection.ranged_attack_power, -10);
        assert_eq!(projection.ranged_attack_power_mod_pos, 21);
    }

    #[test]
    fn stat_system_uses_cpp_rating_and_diminishing_return_branches() {
        let mut rating_bonuses = [0.0; 32];
        rating_bonuses[2] = 10.0;
        rating_bonuses[3] = 10.0;
        rating_bonuses[4] = 2.0;
        rating_bonuses[8] = 3.0;
        rating_bonuses[9] = 4.0;
        rating_bonuses[10] = 5.0;

        let projection = calculate_player_stat_system_like_cpp(PlayerStatSystemInputLikeCpp {
            base: PlayerLevelStats::default(),
            class: 1,
            level: 80,
            attack_power_per_strength: 2,
            attack_power_per_agility: 0,
            ranged_attack_power_per_agility: 0,
            gear_stats: [0; 5],
            gear_health: 0,
            gear_mana: 0,
            gear_armor: 0,
            gear_attack_power: 0,
            gear_ranged_attack_power: 0,
            rating_bonuses,
            can_parry: true,
            can_block: true,
        });

        let expected_dodge = 65.631440 * 10.0 / (10.0 + 65.631440 * 0.9560);
        let expected_parry = expected_dodge + 5.0;
        assert!((projection.dodge_pct - expected_dodge).abs() < 0.00001);
        assert!((projection.parry_pct - expected_parry).abs() < 0.00001);
        assert_eq!(projection.block_pct, 7.0);
        assert_eq!(projection.crit_pct, 8.0);
        assert_eq!(projection.ranged_crit_pct, 9.0);
        assert_eq!(projection.spell_crit_pct, [10.0; 7]);
        assert_eq!(projection.dodge_from_attr, 0.0);
        assert_eq!(projection.parry_from_attr, 0.0);
    }
}
