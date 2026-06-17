// Copyright (c) 2026 alseif0x
// RustyCore - WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 - https://www.gnu.org/licenses/gpl-3.0.html

//! C++ `ObjectMgr::LoadMailLevelRewards` data model.

use std::collections::HashMap;

use anyhow::Result;
use tracing::info;
use wow_database::{WorldDatabase, WorldStatements};

use crate::conditions::RACEMASK_ALL_PLAYABLE_LIKE_CPP;

/// C++ `MAX_LEVEL` from `DataStores/DBCEnums.h` for the 3.4.3 client data set.
pub const MAX_LEVEL_LIKE_CPP: u8 = 123;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MailLevelRewardLikeCpp {
    pub race_mask: u64,
    pub mail_template_id: u32,
    pub sender_entry: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MailLevelRewardRowLikeCpp {
    pub level: u8,
    pub race_mask: u64,
    pub mail_template_id: u32,
    pub sender_entry: u32,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct MailLevelRewardLoadReportLikeCpp {
    pub rows_seen: usize,
    pub loaded_rows: usize,
    pub skipped_level_too_high: Vec<u8>,
    pub skipped_invalid_race_mask: Vec<(u8, u64)>,
    pub skipped_missing_mail_template: Vec<(u8, u32)>,
    pub skipped_missing_sender_creature: Vec<(u8, u32)>,
}

#[derive(Debug, Default, Clone)]
pub struct MailLevelRewardStoreLikeCpp {
    rewards_by_level: HashMap<u8, Vec<MailLevelRewardLikeCpp>>,
}

pub struct MailLevelRewardLoadOutcomeLikeCpp {
    pub store: MailLevelRewardStoreLikeCpp,
    pub report: MailLevelRewardLoadReportLikeCpp,
}

impl MailLevelRewardStoreLikeCpp {
    pub fn from_rows_like_cpp(
        rows: impl IntoIterator<Item = MailLevelRewardRowLikeCpp>,
        mut mail_template_exists: impl FnMut(u32) -> bool,
        mut creature_template_exists: impl FnMut(u32) -> bool,
    ) -> MailLevelRewardLoadOutcomeLikeCpp {
        let mut report = MailLevelRewardLoadReportLikeCpp::default();
        let mut rewards_by_level: HashMap<u8, Vec<MailLevelRewardLikeCpp>> = HashMap::new();

        for row in rows {
            report.rows_seen += 1;

            if row.level > MAX_LEVEL_LIKE_CPP {
                report.skipped_level_too_high.push(row.level);
                continue;
            }

            if row.race_mask & RACEMASK_ALL_PLAYABLE_LIKE_CPP == 0 {
                report
                    .skipped_invalid_race_mask
                    .push((row.level, row.race_mask));
                continue;
            }

            if !mail_template_exists(row.mail_template_id) {
                report
                    .skipped_missing_mail_template
                    .push((row.level, row.mail_template_id));
                continue;
            }

            if !creature_template_exists(row.sender_entry) {
                report
                    .skipped_missing_sender_creature
                    .push((row.level, row.sender_entry));
                continue;
            }

            rewards_by_level
                .entry(row.level)
                .or_default()
                .push(MailLevelRewardLikeCpp {
                    race_mask: row.race_mask,
                    mail_template_id: row.mail_template_id,
                    sender_entry: row.sender_entry,
                });
            report.loaded_rows += 1;
        }

        MailLevelRewardLoadOutcomeLikeCpp {
            store: Self { rewards_by_level },
            report,
        }
    }

    /// C++ `ObjectMgr::LoadMailLevelRewards`.
    pub async fn load_like_cpp(
        db: &WorldDatabase,
        mut mail_template_exists: impl FnMut(u32) -> bool,
        mut creature_template_exists: impl FnMut(u32) -> bool,
    ) -> Result<MailLevelRewardLoadOutcomeLikeCpp> {
        let stmt = db.prepare(WorldStatements::SEL_MAIL_LEVEL_REWARDS);
        let mut result = db.query(&stmt).await?;
        let mut rows = Vec::new();

        if !result.is_empty() {
            loop {
                rows.push(MailLevelRewardRowLikeCpp {
                    level: result.read(0),
                    race_mask: result.read(1),
                    mail_template_id: result.read(2),
                    sender_entry: result.read(3),
                });

                if !result.next_row() {
                    break;
                }
            }
        }

        let outcome = Self::from_rows_like_cpp(
            rows,
            &mut mail_template_exists,
            &mut creature_template_exists,
        );
        info!(
            "Loaded {} level dependent mail rewards",
            outcome.report.loaded_rows
        );
        Ok(outcome)
    }

    /// C++ `ObjectMgr::GetMailLevelReward`.
    pub fn mail_level_reward_like_cpp(
        &self,
        level: u8,
        race: u8,
    ) -> Option<&MailLevelRewardLikeCpp> {
        let race_mask = race_mask_for_race_like_cpp(race)?;
        self.rewards_by_level
            .get(&level)?
            .iter()
            .find(|reward| reward.race_mask & race_mask != 0)
    }

    pub fn rewards_for_level_like_cpp(&self, level: u8) -> &[MailLevelRewardLikeCpp] {
        self.rewards_by_level
            .get(&level)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn len(&self) -> usize {
        self.rewards_by_level.values().map(Vec::len).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.rewards_by_level.is_empty()
    }
}

fn race_mask_for_race_like_cpp(race: u8) -> Option<u64> {
    let bit = match race {
        34 => 11,
        35 => 12,
        36 => 13,
        37 => 14,
        52 => 16,
        70 => 15,
        1..=32 => u32::from(race - 1),
        _ => return None,
    };

    Some(1u64 << bit)
}

#[cfg(test)]
mod tests {
    use super::*;

    const RACE_HUMAN_MASK: u64 = 1 << (1 - 1);
    const RACE_ORC_MASK: u64 = 1 << (2 - 1);
    const RACE_DRACTHYR_ALLIANCE_MASK: u64 = 1 << 16;

    fn row(
        level: u8,
        race_mask: u64,
        mail_template_id: u32,
        sender_entry: u32,
    ) -> MailLevelRewardRowLikeCpp {
        MailLevelRewardRowLikeCpp {
            level,
            race_mask,
            mail_template_id,
            sender_entry,
        }
    }

    #[test]
    fn load_mail_level_rewards_filters_invalid_rows_like_cpp() {
        let outcome = MailLevelRewardStoreLikeCpp::from_rows_like_cpp(
            [
                row(10, RACE_HUMAN_MASK, 100, 200),
                row(MAX_LEVEL_LIKE_CPP + 1, RACE_HUMAN_MASK, 100, 200),
                row(11, 0, 100, 200),
                row(12, RACE_HUMAN_MASK, 999, 200),
                row(13, RACE_HUMAN_MASK, 100, 999),
            ],
            |mail_template_id| mail_template_id == 100,
            |sender_entry| sender_entry == 200,
        );

        assert_eq!(outcome.report.rows_seen, 5);
        assert_eq!(outcome.report.loaded_rows, 1);
        assert_eq!(
            outcome.report.skipped_level_too_high,
            vec![MAX_LEVEL_LIKE_CPP + 1]
        );
        assert_eq!(outcome.report.skipped_invalid_race_mask, vec![(11, 0)]);
        assert_eq!(
            outcome.report.skipped_missing_mail_template,
            vec![(12, 999)]
        );
        assert_eq!(
            outcome.report.skipped_missing_sender_creature,
            vec![(13, 999)]
        );
        assert_eq!(outcome.store.len(), 1);
    }

    #[test]
    fn get_mail_level_reward_returns_first_matching_race_like_cpp() {
        let outcome = MailLevelRewardStoreLikeCpp::from_rows_like_cpp(
            [
                row(20, RACE_ORC_MASK, 101, 201),
                row(20, RACE_HUMAN_MASK, 102, 202),
                row(20, RACE_HUMAN_MASK, 103, 203),
            ],
            |_| true,
            |_| true,
        );

        let reward = outcome
            .store
            .mail_level_reward_like_cpp(20, 1)
            .expect("human reward should match");

        assert_eq!(reward.mail_template_id, 102);
        assert_eq!(reward.sender_entry, 202);
        assert!(outcome.store.mail_level_reward_like_cpp(20, 3).is_none());
    }

    #[test]
    fn get_mail_level_reward_uses_extended_race_masks_like_cpp() {
        let outcome = MailLevelRewardStoreLikeCpp::from_rows_like_cpp(
            [row(30, RACE_DRACTHYR_ALLIANCE_MASK, 104, 204)],
            |_| true,
            |_| true,
        );

        assert_eq!(
            outcome
                .store
                .mail_level_reward_like_cpp(30, 52)
                .expect("dracthyr alliance reward should match")
                .mail_template_id,
            104
        );
    }
}
