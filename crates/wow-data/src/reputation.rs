//! C++ reputation data helpers and `ObjectMgr` reputation world-table stores.

use std::collections::HashMap;

use anyhow::Result;
use tracing::{info, warn};
use wow_database::{SqlResult, WorldDatabase, WorldStatements};

use crate::creature_template::CreatureTemplateLifecycleStoreLikeCpp;
use crate::progression_rewards::FactionStore;

pub const REPUTATION_CAP_LIKE_CPP: i32 = 42_000;
pub const REPUTATION_BOTTOM_LIKE_CPP: i32 = -42_000;
pub const REPUTATION_RANK_THRESHOLDS_LIKE_CPP: [i32; 8] =
    [-42_000, -6_000, -3_000, 0, 3_000, 9_000, 21_000, 42_000];
pub const MAX_SPILLOVER_FACTIONS_LIKE_CPP: usize = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum ReputationRankLikeCpp {
    Hated = 0,
    Hostile = 1,
    Unfriendly = 2,
    Neutral = 3,
    Friendly = 4,
    Honored = 5,
    Revered = 6,
    Exalted = 7,
}

impl ReputationRankLikeCpp {
    pub const fn as_u8(self) -> u8 {
        self as u8
    }

    pub const fn from_u8_like_cpp(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Hated),
            1 => Some(Self::Hostile),
            2 => Some(Self::Unfriendly),
            3 => Some(Self::Neutral),
            4 => Some(Self::Friendly),
            5 => Some(Self::Honored),
            6 => Some(Self::Revered),
            7 => Some(Self::Exalted),
            _ => None,
        }
    }
}

pub fn reputation_rank_from_standing_like_cpp(standing: i32) -> ReputationRankLikeCpp {
    let rank = REPUTATION_RANK_THRESHOLDS_LIKE_CPP
        .iter()
        .position(|threshold| standing < *threshold)
        .map(|idx| idx.saturating_sub(1))
        .unwrap_or(REPUTATION_RANK_THRESHOLDS_LIKE_CPP.len() - 1);

    ReputationRankLikeCpp::from_u8_like_cpp(rank as u8)
        .expect("rank index is bounded by C++ threshold table")
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct ReputationFlagsLikeCpp: u16 {
        const NONE = 0x0000;
        const VISIBLE = 0x0001;
        const AT_WAR = 0x0002;
        const HIDDEN = 0x0004;
        const HEADER = 0x0008;
        const PEACEFUL = 0x0010;
        const INACTIVE = 0x0020;
        const SHOW_PROPAGATED = 0x0040;
        const HEADER_SHOWS_BAR = 0x0080;
        const CAPITAL_CITY_FOR_RACE_CHANGE = 0x0100;
        const GUILD = 0x0200;
        const GARRISON_INVASION = 0x0400;
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReputationRewardRateEntryLikeCpp {
    pub quest_rate: f32,
    pub quest_daily_rate: f32,
    pub quest_weekly_rate: f32,
    pub quest_monthly_rate: f32,
    pub quest_repeatable_rate: f32,
    pub creature_rate: f32,
    pub spell_rate: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReputationRewardRateRowLikeCpp {
    pub faction_id: u32,
    pub rates: ReputationRewardRateEntryLikeCpp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReputationRewardRateSkipReasonLikeCpp {
    MissingFaction,
    NegativeQuestRate,
    NegativeQuestDailyRate,
    NegativeQuestWeeklyRate,
    NegativeQuestMonthlyRate,
    NegativeQuestRepeatableRate,
    NegativeCreatureRate,
    NegativeSpellRate,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SkippedReputationRewardRateRowLikeCpp {
    pub faction_id: u32,
    pub rates: ReputationRewardRateEntryLikeCpp,
    pub reason: ReputationRewardRateSkipReasonLikeCpp,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ReputationRewardRateLoadReportLikeCpp {
    pub loaded: usize,
    pub skipped: Vec<SkippedReputationRewardRateRowLikeCpp>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ReputationRewardRateStoreLikeCpp {
    rates_by_faction: HashMap<u32, ReputationRewardRateEntryLikeCpp>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RepSpilloverTemplateLikeCpp {
    pub faction: [u32; MAX_SPILLOVER_FACTIONS_LIKE_CPP],
    pub faction_rate: [f32; MAX_SPILLOVER_FACTIONS_LIKE_CPP],
    pub faction_rank: [u8; MAX_SPILLOVER_FACTIONS_LIKE_CPP],
}

impl RepSpilloverTemplateLikeCpp {
    pub const fn empty_like_cpp() -> Self {
        Self {
            faction: [0; MAX_SPILLOVER_FACTIONS_LIKE_CPP],
            faction_rate: [0.0; MAX_SPILLOVER_FACTIONS_LIKE_CPP],
            faction_rank: [0; MAX_SPILLOVER_FACTIONS_LIKE_CPP],
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RepSpilloverTemplateRowLikeCpp {
    pub faction_id: u32,
    pub template: RepSpilloverTemplateLikeCpp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepSpilloverTemplateSkipReasonLikeCpp {
    MissingSourceFaction,
    SourceFactionHasNoParent,
    MissingSpilloverFaction { slot: usize, faction_id: u32 },
    SpilloverFactionCannotHaveReputation { slot: usize, faction_id: u32 },
    InvalidRank { slot: usize, rank: u8 },
}

#[derive(Debug, Clone, PartialEq)]
pub struct SkippedRepSpilloverTemplateRowLikeCpp {
    pub faction_id: u32,
    pub reason: RepSpilloverTemplateSkipReasonLikeCpp,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct RepSpilloverTemplateLoadReportLikeCpp {
    pub loaded: usize,
    pub skipped: Vec<SkippedRepSpilloverTemplateRowLikeCpp>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct RepSpilloverTemplateStoreLikeCpp {
    templates_by_faction: HashMap<u32, RepSpilloverTemplateLikeCpp>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreatureOnKillReputationEntryLikeCpp {
    pub rep_faction_1: u32,
    pub rep_faction_2: u32,
    pub reputation_max_cap_1: u8,
    pub rep_value_1: i32,
    pub reputation_max_cap_2: u8,
    pub rep_value_2: i32,
    pub is_team_award_1: bool,
    pub is_team_award_2: bool,
    pub team_dependent: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreatureOnKillReputationRowLikeCpp {
    pub creature_id: u32,
    pub entry: CreatureOnKillReputationEntryLikeCpp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreatureOnKillReputationSkipReasonLikeCpp {
    MissingCreatureTemplate,
    MissingFaction1 { faction_id: u32 },
    MissingFaction2 { faction_id: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkippedCreatureOnKillReputationRowLikeCpp {
    pub creature_id: u32,
    pub entry: CreatureOnKillReputationEntryLikeCpp,
    pub reason: CreatureOnKillReputationSkipReasonLikeCpp,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CreatureOnKillReputationLoadReportLikeCpp {
    pub loaded: usize,
    pub skipped: Vec<SkippedCreatureOnKillReputationRowLikeCpp>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CreatureOnKillReputationStoreLikeCpp {
    entries_by_creature_id: HashMap<u32, CreatureOnKillReputationEntryLikeCpp>,
}

impl ReputationRewardRateStoreLikeCpp {
    pub fn from_rows_like_cpp(
        rows: impl IntoIterator<Item = ReputationRewardRateRowLikeCpp>,
        faction_store: &FactionStore,
    ) -> (Self, ReputationRewardRateLoadReportLikeCpp) {
        let mut store = Self::default();
        let mut report = ReputationRewardRateLoadReportLikeCpp::default();

        for row in rows {
            if faction_store.get(row.faction_id).is_none() {
                report.skipped.push(SkippedReputationRewardRateRowLikeCpp {
                    faction_id: row.faction_id,
                    rates: row.rates,
                    reason: ReputationRewardRateSkipReasonLikeCpp::MissingFaction,
                });
                continue;
            }

            let Some(reason) = validate_non_negative_rates_like_cpp(row.rates) else {
                store.rates_by_faction.insert(row.faction_id, row.rates);
                report.loaded += 1;
                continue;
            };

            report.skipped.push(SkippedReputationRewardRateRowLikeCpp {
                faction_id: row.faction_id,
                rates: row.rates,
                reason,
            });
        }

        (store, report)
    }

    pub async fn load_like_cpp(
        db: &WorldDatabase,
        faction_store: &FactionStore,
    ) -> Result<(Self, ReputationRewardRateLoadReportLikeCpp)> {
        let stmt = db.prepare(WorldStatements::SEL_REPUTATION_REWARD_RATE);
        let mut result = db.query(&stmt).await?;
        if result.is_empty() {
            info!("Loaded `reputation_reward_rate`, table is empty");
            return Ok((
                Self::default(),
                ReputationRewardRateLoadReportLikeCpp::default(),
            ));
        }

        let mut rows = Vec::new();
        loop {
            rows.push(ReputationRewardRateRowLikeCpp {
                faction_id: read_db_u32_like_cpp(&result, 0),
                rates: ReputationRewardRateEntryLikeCpp {
                    quest_rate: result.read(1),
                    quest_daily_rate: result.read(2),
                    quest_weekly_rate: result.read(3),
                    quest_monthly_rate: result.read(4),
                    quest_repeatable_rate: result.read(5),
                    creature_rate: result.read(6),
                    spell_rate: result.read(7),
                },
            });

            if !result.next_row() {
                break;
            }
        }

        let (store, report) = Self::from_rows_like_cpp(rows, faction_store);
        for skipped in &report.skipped {
            warn!(
                faction_id = skipped.faction_id,
                reason = ?skipped.reason,
                "Skipping reputation_reward_rate row like C++"
            );
        }
        info!(
            "Loaded {} reputation_reward_rate rows ({} skipped)",
            report.loaded,
            report.skipped.len()
        );
        Ok((store, report))
    }

    pub fn get(&self, faction_id: u32) -> Option<&ReputationRewardRateEntryLikeCpp> {
        self.rates_by_faction.get(&faction_id)
    }

    pub fn len(&self) -> usize {
        self.rates_by_faction.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rates_by_faction.is_empty()
    }
}

impl CreatureOnKillReputationStoreLikeCpp {
    pub fn from_rows_like_cpp(
        rows: impl IntoIterator<Item = CreatureOnKillReputationRowLikeCpp>,
        creature_template_store: &CreatureTemplateLifecycleStoreLikeCpp,
        faction_store: &FactionStore,
    ) -> (Self, CreatureOnKillReputationLoadReportLikeCpp) {
        let mut store = Self::default();
        let mut report = CreatureOnKillReputationLoadReportLikeCpp::default();

        for row in rows {
            if creature_template_store.get(row.creature_id).is_none() {
                report
                    .skipped
                    .push(SkippedCreatureOnKillReputationRowLikeCpp {
                        creature_id: row.creature_id,
                        entry: row.entry,
                        reason: CreatureOnKillReputationSkipReasonLikeCpp::MissingCreatureTemplate,
                    });
                continue;
            }

            if row.entry.rep_faction_1 != 0 && faction_store.get(row.entry.rep_faction_1).is_none()
            {
                report
                    .skipped
                    .push(SkippedCreatureOnKillReputationRowLikeCpp {
                        creature_id: row.creature_id,
                        entry: row.entry,
                        reason: CreatureOnKillReputationSkipReasonLikeCpp::MissingFaction1 {
                            faction_id: row.entry.rep_faction_1,
                        },
                    });
                continue;
            }

            if row.entry.rep_faction_2 != 0 && faction_store.get(row.entry.rep_faction_2).is_none()
            {
                report
                    .skipped
                    .push(SkippedCreatureOnKillReputationRowLikeCpp {
                        creature_id: row.creature_id,
                        entry: row.entry,
                        reason: CreatureOnKillReputationSkipReasonLikeCpp::MissingFaction2 {
                            faction_id: row.entry.rep_faction_2,
                        },
                    });
                continue;
            }

            store
                .entries_by_creature_id
                .insert(row.creature_id, row.entry);
            report.loaded += 1;
        }

        (store, report)
    }

    pub async fn load_like_cpp(
        db: &WorldDatabase,
        creature_template_store: &CreatureTemplateLifecycleStoreLikeCpp,
        faction_store: &FactionStore,
    ) -> Result<(Self, CreatureOnKillReputationLoadReportLikeCpp)> {
        let stmt = db.prepare(WorldStatements::SEL_CREATURE_ONKILL_REPUTATION);
        let mut result = db.query(&stmt).await?;
        if result.is_empty() {
            info!(
                "Loaded 0 creature award reputation definitions. DB table `creature_onkill_reputation` is empty."
            );
            return Ok((
                Self::default(),
                CreatureOnKillReputationLoadReportLikeCpp::default(),
            ));
        }

        let mut rows = Vec::new();
        loop {
            rows.push(CreatureOnKillReputationRowLikeCpp {
                creature_id: read_db_u32_like_cpp(&result, 0),
                entry: CreatureOnKillReputationEntryLikeCpp {
                    rep_faction_1: read_db_i16_like_cpp(&result, 1) as u32,
                    rep_faction_2: read_db_i16_like_cpp(&result, 2) as u32,
                    is_team_award_1: read_db_bool_like_cpp(&result, 3),
                    reputation_max_cap_1: read_db_u8_like_cpp(&result, 4),
                    rep_value_1: result.read(5),
                    is_team_award_2: read_db_bool_like_cpp(&result, 6),
                    reputation_max_cap_2: read_db_u8_like_cpp(&result, 7),
                    rep_value_2: result.read(8),
                    team_dependent: read_db_bool_like_cpp(&result, 9),
                },
            });

            if !result.next_row() {
                break;
            }
        }

        let (store, report) =
            Self::from_rows_like_cpp(rows, creature_template_store, faction_store);
        for skipped in &report.skipped {
            warn!(
                creature_id = skipped.creature_id,
                reason = ?skipped.reason,
                "Skipping creature_onkill_reputation row like C++"
            );
        }
        info!(
            "Loaded {} creature award reputation definitions ({} skipped)",
            report.loaded,
            report.skipped.len()
        );
        Ok((store, report))
    }

    pub fn get(&self, creature_id: u32) -> Option<&CreatureOnKillReputationEntryLikeCpp> {
        self.entries_by_creature_id.get(&creature_id)
    }

    pub fn len(&self) -> usize {
        self.entries_by_creature_id.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries_by_creature_id.is_empty()
    }
}

impl RepSpilloverTemplateStoreLikeCpp {
    pub fn from_rows_like_cpp(
        rows: impl IntoIterator<Item = RepSpilloverTemplateRowLikeCpp>,
        faction_store: &FactionStore,
    ) -> (Self, RepSpilloverTemplateLoadReportLikeCpp) {
        let mut store = Self::default();
        let mut report = RepSpilloverTemplateLoadReportLikeCpp::default();

        for row in rows {
            let Some(source_faction) = faction_store.get(row.faction_id) else {
                report.skipped.push(SkippedRepSpilloverTemplateRowLikeCpp {
                    faction_id: row.faction_id,
                    reason: RepSpilloverTemplateSkipReasonLikeCpp::MissingSourceFaction,
                });
                continue;
            };

            if source_faction.parent_faction_id == 0 {
                report.skipped.push(SkippedRepSpilloverTemplateRowLikeCpp {
                    faction_id: row.faction_id,
                    reason: RepSpilloverTemplateSkipReasonLikeCpp::SourceFactionHasNoParent,
                });
                continue;
            }

            let mut skip_reason = None;
            for slot in 0..MAX_SPILLOVER_FACTIONS_LIKE_CPP {
                let spillover_faction_id = row.template.faction[slot];
                if spillover_faction_id == 0 {
                    continue;
                }

                match faction_store.get(spillover_faction_id) {
                    None => {
                        skip_reason = Some(
                            RepSpilloverTemplateSkipReasonLikeCpp::MissingSpilloverFaction {
                                slot,
                                faction_id: spillover_faction_id,
                            },
                        );
                        break;
                    }
                    Some(spillover_faction)
                        if !spillover_faction.can_have_reputation_like_cpp() =>
                    {
                        skip_reason =
                            Some(RepSpilloverTemplateSkipReasonLikeCpp::SpilloverFactionCannotHaveReputation {
                                slot,
                                faction_id: spillover_faction_id,
                            });
                        break;
                    }
                    Some(_) => {}
                }

                if row.template.faction_rank[slot]
                    >= REPUTATION_RANK_THRESHOLDS_LIKE_CPP.len() as u8
                {
                    skip_reason = Some(RepSpilloverTemplateSkipReasonLikeCpp::InvalidRank {
                        slot,
                        rank: row.template.faction_rank[slot],
                    });
                    break;
                }
            }

            if let Some(reason) = skip_reason {
                report.skipped.push(SkippedRepSpilloverTemplateRowLikeCpp {
                    faction_id: row.faction_id,
                    reason,
                });
                continue;
            }

            store
                .templates_by_faction
                .insert(row.faction_id, row.template);
            report.loaded += 1;
        }

        (store, report)
    }

    pub async fn load_like_cpp(
        db: &WorldDatabase,
        faction_store: &FactionStore,
    ) -> Result<(Self, RepSpilloverTemplateLoadReportLikeCpp)> {
        let stmt = db.prepare(WorldStatements::SEL_REPUTATION_SPILLOVER_TEMPLATE);
        let mut result = db.query(&stmt).await?;
        if result.is_empty() {
            info!("Loaded `reputation_spillover_template`, table is empty");
            return Ok((
                Self::default(),
                RepSpilloverTemplateLoadReportLikeCpp::default(),
            ));
        }

        let mut rows = Vec::new();
        loop {
            rows.push(RepSpilloverTemplateRowLikeCpp {
                faction_id: u32::from(read_db_u16_like_cpp(&result, 0)),
                template: RepSpilloverTemplateLikeCpp {
                    faction: [
                        u32::from(read_db_u16_like_cpp(&result, 1)),
                        u32::from(read_db_u16_like_cpp(&result, 4)),
                        u32::from(read_db_u16_like_cpp(&result, 7)),
                        u32::from(read_db_u16_like_cpp(&result, 10)),
                        u32::from(read_db_u16_like_cpp(&result, 13)),
                    ],
                    faction_rate: [
                        result.read(2),
                        result.read(5),
                        result.read(8),
                        result.read(11),
                        result.read(14),
                    ],
                    faction_rank: [
                        read_db_u8_like_cpp(&result, 3),
                        read_db_u8_like_cpp(&result, 6),
                        read_db_u8_like_cpp(&result, 9),
                        read_db_u8_like_cpp(&result, 12),
                        read_db_u8_like_cpp(&result, 15),
                    ],
                },
            });

            if !result.next_row() {
                break;
            }
        }

        let (store, report) = Self::from_rows_like_cpp(rows, faction_store);
        for skipped in &report.skipped {
            warn!(
                faction_id = skipped.faction_id,
                reason = ?skipped.reason,
                "Skipping reputation_spillover_template row like C++"
            );
        }
        info!(
            "Loaded {} reputation_spillover_template rows ({} skipped)",
            report.loaded,
            report.skipped.len()
        );
        Ok((store, report))
    }

    pub fn get(&self, faction_id: u32) -> Option<&RepSpilloverTemplateLikeCpp> {
        self.templates_by_faction.get(&faction_id)
    }

    pub fn len(&self) -> usize {
        self.templates_by_faction.len()
    }

    pub fn is_empty(&self) -> bool {
        self.templates_by_faction.is_empty()
    }
}

fn validate_non_negative_rates_like_cpp(
    rates: ReputationRewardRateEntryLikeCpp,
) -> Option<ReputationRewardRateSkipReasonLikeCpp> {
    if rates.quest_rate < 0.0 {
        Some(ReputationRewardRateSkipReasonLikeCpp::NegativeQuestRate)
    } else if rates.quest_daily_rate < 0.0 {
        Some(ReputationRewardRateSkipReasonLikeCpp::NegativeQuestDailyRate)
    } else if rates.quest_weekly_rate < 0.0 {
        Some(ReputationRewardRateSkipReasonLikeCpp::NegativeQuestWeeklyRate)
    } else if rates.quest_monthly_rate < 0.0 {
        Some(ReputationRewardRateSkipReasonLikeCpp::NegativeQuestMonthlyRate)
    } else if rates.quest_repeatable_rate < 0.0 {
        Some(ReputationRewardRateSkipReasonLikeCpp::NegativeQuestRepeatableRate)
    } else if rates.creature_rate < 0.0 {
        Some(ReputationRewardRateSkipReasonLikeCpp::NegativeCreatureRate)
    } else if rates.spell_rate < 0.0 {
        Some(ReputationRewardRateSkipReasonLikeCpp::NegativeSpellRate)
    } else {
        None
    }
}

fn read_db_u32_like_cpp(result: &SqlResult, column: usize) -> u32 {
    if let Some(value) = result.try_read::<u32>(column) {
        return value;
    }
    if let Some(value) = result.try_read::<i32>(column) {
        return normalize_signed_db_u32_like_cpp(value);
    }
    if let Some(value) = result.try_read::<u16>(column) {
        return u32::from(value);
    }
    if let Some(value) = result.try_read::<i16>(column) {
        return normalize_signed_db_u32_like_cpp(i32::from(value));
    }
    if let Some(value) = result.try_read::<u8>(column) {
        return u32::from(value);
    }
    if let Some(value) = result.try_read::<i8>(column) {
        return normalize_signed_db_u32_like_cpp(i32::from(value));
    }
    0
}

fn read_db_u16_like_cpp(result: &SqlResult, column: usize) -> u16 {
    if let Some(value) = result.try_read::<u16>(column) {
        return value;
    }
    if let Some(value) = result.try_read::<i16>(column) {
        return normalize_signed_db_u16_like_cpp(i32::from(value));
    }
    if let Some(value) = result.try_read::<u8>(column) {
        return u16::from(value);
    }
    if let Some(value) = result.try_read::<i8>(column) {
        return u16::from(normalize_signed_db_u8_like_cpp(i32::from(value)));
    }
    if let Some(value) = result.try_read::<u32>(column) {
        return u16::try_from(value).unwrap_or(0);
    }
    if let Some(value) = result.try_read::<i32>(column) {
        return normalize_signed_db_u16_like_cpp(value);
    }
    0
}

fn read_db_u8_like_cpp(result: &SqlResult, column: usize) -> u8 {
    if let Some(value) = result.try_read::<u8>(column) {
        return value;
    }
    if let Some(value) = result.try_read::<i8>(column) {
        return normalize_signed_db_u8_like_cpp(i32::from(value));
    }
    if let Some(value) = result.try_read::<u16>(column) {
        return u8::try_from(value).unwrap_or(0);
    }
    if let Some(value) = result.try_read::<i16>(column) {
        return normalize_signed_db_u8_like_cpp(i32::from(value));
    }
    if let Some(value) = result.try_read::<u32>(column) {
        return u8::try_from(value).unwrap_or(0);
    }
    if let Some(value) = result.try_read::<i32>(column) {
        return normalize_signed_db_u8_like_cpp(value);
    }
    0
}

fn read_db_i16_like_cpp(result: &SqlResult, column: usize) -> i16 {
    if let Some(value) = result.try_read::<i16>(column) {
        return value;
    }
    if let Some(value) = result.try_read::<u16>(column) {
        return normalize_unsigned_db_i16_like_cpp(u32::from(value));
    }
    if let Some(value) = result.try_read::<i32>(column) {
        return i16::try_from(value).unwrap_or(0);
    }
    if let Some(value) = result.try_read::<u32>(column) {
        return normalize_unsigned_db_i16_like_cpp(value);
    }
    0
}

fn read_db_bool_like_cpp(result: &SqlResult, column: usize) -> bool {
    read_db_u8_like_cpp(result, column) == 1
}

fn normalize_signed_db_u32_like_cpp(value: i32) -> u32 {
    value as u32
}

fn normalize_signed_db_u16_like_cpp(value: i32) -> u16 {
    let converted = value as u16;
    if i32::from(converted) == value || (converted as i16) as i32 == value {
        converted
    } else {
        0
    }
}

fn normalize_signed_db_u8_like_cpp(value: i32) -> u8 {
    let converted = value as u8;
    if i32::from(converted) == value || (converted as i8) as i32 == value {
        converted
    } else {
        0
    }
}

fn normalize_unsigned_db_i16_like_cpp(value: u32) -> i16 {
    if value <= u32::from(u16::MAX) {
        value as i16
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::progression_rewards::{FactionEntry, FactionStore};

    fn rates(value: f32) -> ReputationRewardRateEntryLikeCpp {
        ReputationRewardRateEntryLikeCpp {
            quest_rate: value,
            quest_daily_rate: value,
            quest_weekly_rate: value,
            quest_monthly_rate: value,
            quest_repeatable_rate: value,
            creature_rate: value,
            spell_rate: value,
        }
    }

    fn faction_store(ids: impl IntoIterator<Item = u32>) -> FactionStore {
        FactionStore::from_entries(
            ids.into_iter()
                .map(|id| FactionEntry::for_test_like_cpp(id, id as i16)),
        )
    }

    #[test]
    fn signed_reputation_sql_columns_normalize_like_cpp_getuint_accessors() {
        assert_eq!(normalize_signed_db_u32_like_cpp(-1), u32::MAX);
        assert_eq!(normalize_signed_db_u16_like_cpp(-1), u16::MAX);
        assert_eq!(normalize_signed_db_u16_like_cpp(0x1_0000), 0);
        assert_eq!(normalize_signed_db_u8_like_cpp(-1), u8::MAX);
        assert_eq!(normalize_signed_db_u8_like_cpp(0x100), 0);
        assert_eq!(normalize_unsigned_db_i16_like_cpp(u32::from(u16::MAX)), -1);
        assert_eq!(
            normalize_unsigned_db_i16_like_cpp(u32::from(u16::MAX) + 1),
            0
        );
    }

    fn creature_template_store(
        ids: impl IntoIterator<Item = u32>,
    ) -> CreatureTemplateLifecycleStoreLikeCpp {
        CreatureTemplateLifecycleStoreLikeCpp::from_templates(ids.into_iter().map(|entry| {
            crate::creature_template::CreatureTemplateLifecycleRecordLikeCpp {
                entry,
                name: format!("Creature {entry}"),
                ai_name: String::new(),
                script_name: String::new(),
                faction: 35,
                npc_flags: 0,
                speed_walk: 1.0,
                speed_run: 1.0,
                scale: 1.0,
                classification: 0,
                damage_school: wow_constants::spell::SpellSchools::Normal as u8,
                unit_flags: 0,
                unit_flags2: 0,
                unit_flags3: 0,
                creature_type: 0,
                unit_class: 1,
                vehicle_id: 0,
                movement_type: 0,
                ground_movement_type: wow_constants::CreatureGroundMovementType::Run as u8,
                swim_allowed: true,
                flight_movement_type: 0,
                flags_extra: 0,
                string_id: String::new(),
                regen_health: true,
                spells: [0; crate::creature_template::MAX_CREATURE_SPELLS_LIKE_CPP],
                models: Vec::new(),
            }
        }))
    }

    #[test]
    fn reputation_reward_rate_load_validates_faction_like_cpp() {
        let factions = faction_store([7]);
        let (store, report) = ReputationRewardRateStoreLikeCpp::from_rows_like_cpp(
            [
                ReputationRewardRateRowLikeCpp {
                    faction_id: 7,
                    rates: rates(1.5),
                },
                ReputationRewardRateRowLikeCpp {
                    faction_id: 8,
                    rates: rates(2.0),
                },
            ],
            &factions,
        );

        assert_eq!(store.len(), 1);
        assert_eq!(store.get(7).map(|rate| rate.quest_rate), Some(1.5));
        assert_eq!(report.loaded, 1);
        assert_eq!(report.skipped.len(), 1);
        assert_eq!(
            report.skipped[0].reason,
            ReputationRewardRateSkipReasonLikeCpp::MissingFaction
        );
    }

    #[test]
    fn reputation_reward_rate_load_rejects_negative_rate_like_cpp() {
        let factions = faction_store([7]);
        let mut invalid = rates(1.0);
        invalid.quest_weekly_rate = -0.1;

        let (store, report) = ReputationRewardRateStoreLikeCpp::from_rows_like_cpp(
            [ReputationRewardRateRowLikeCpp {
                faction_id: 7,
                rates: invalid,
            }],
            &factions,
        );

        assert!(store.is_empty());
        assert_eq!(report.loaded, 0);
        assert_eq!(report.skipped.len(), 1);
        assert_eq!(
            report.skipped[0].reason,
            ReputationRewardRateSkipReasonLikeCpp::NegativeQuestWeeklyRate
        );
    }

    #[test]
    fn creature_onkill_reputation_load_validates_creature_and_factions_like_cpp() {
        let creatures = creature_template_store([100]);
        let factions = faction_store([7, 8]);
        let valid = CreatureOnKillReputationEntryLikeCpp {
            rep_faction_1: 7,
            rep_faction_2: 8,
            reputation_max_cap_1: ReputationRankLikeCpp::Honored.as_u8(),
            rep_value_1: 10,
            reputation_max_cap_2: ReputationRankLikeCpp::Friendly.as_u8(),
            rep_value_2: 20,
            is_team_award_1: true,
            is_team_award_2: false,
            team_dependent: true,
        };

        let (store, report) = CreatureOnKillReputationStoreLikeCpp::from_rows_like_cpp(
            [
                CreatureOnKillReputationRowLikeCpp {
                    creature_id: 100,
                    entry: valid,
                },
                CreatureOnKillReputationRowLikeCpp {
                    creature_id: 999,
                    entry: valid,
                },
                CreatureOnKillReputationRowLikeCpp {
                    creature_id: 100,
                    entry: CreatureOnKillReputationEntryLikeCpp {
                        rep_faction_1: 999,
                        ..valid
                    },
                },
                CreatureOnKillReputationRowLikeCpp {
                    creature_id: 100,
                    entry: CreatureOnKillReputationEntryLikeCpp {
                        rep_faction_1: 0,
                        rep_faction_2: 999,
                        ..valid
                    },
                },
            ],
            &creatures,
            &factions,
        );

        assert_eq!(store.len(), 1);
        assert_eq!(store.get(100), Some(&valid));
        assert_eq!(report.loaded, 1);
        assert_eq!(
            report
                .skipped
                .iter()
                .map(|skipped| skipped.reason)
                .collect::<Vec<_>>(),
            vec![
                CreatureOnKillReputationSkipReasonLikeCpp::MissingCreatureTemplate,
                CreatureOnKillReputationSkipReasonLikeCpp::MissingFaction1 { faction_id: 999 },
                CreatureOnKillReputationSkipReasonLikeCpp::MissingFaction2 { faction_id: 999 },
            ]
        );
    }

    #[test]
    fn reputation_spillover_template_load_validates_like_cpp() {
        let mut source = FactionEntry::for_test_like_cpp(10, 1);
        source.parent_faction_id = 1;
        let spill = FactionEntry::for_test_like_cpp(11, 2);
        let missing_parent = FactionEntry::for_test_like_cpp(12, 3);
        let mut hidden = FactionEntry::for_test_like_cpp(13, -1);
        hidden.parent_faction_id = 1;
        let factions = FactionStore::from_entries([source, spill, missing_parent, hidden]);

        let mut valid_template = RepSpilloverTemplateLikeCpp::empty_like_cpp();
        valid_template.faction[0] = 11;
        valid_template.faction_rate[0] = 0.5;
        valid_template.faction_rank[0] = ReputationRankLikeCpp::Honored.as_u8();

        let mut missing_spillover = RepSpilloverTemplateLikeCpp::empty_like_cpp();
        missing_spillover.faction[0] = 999;

        let mut invalid_rank = RepSpilloverTemplateLikeCpp::empty_like_cpp();
        invalid_rank.faction[0] = 11;
        invalid_rank.faction_rank[0] = 8;

        let mut cannot_have_reputation = RepSpilloverTemplateLikeCpp::empty_like_cpp();
        cannot_have_reputation.faction[0] = 13;

        let (store, report) = RepSpilloverTemplateStoreLikeCpp::from_rows_like_cpp(
            [
                RepSpilloverTemplateRowLikeCpp {
                    faction_id: 10,
                    template: valid_template,
                },
                RepSpilloverTemplateRowLikeCpp {
                    faction_id: 12,
                    template: RepSpilloverTemplateLikeCpp::empty_like_cpp(),
                },
                RepSpilloverTemplateRowLikeCpp {
                    faction_id: 10,
                    template: missing_spillover,
                },
                RepSpilloverTemplateRowLikeCpp {
                    faction_id: 10,
                    template: invalid_rank,
                },
                RepSpilloverTemplateRowLikeCpp {
                    faction_id: 10,
                    template: cannot_have_reputation,
                },
                RepSpilloverTemplateRowLikeCpp {
                    faction_id: 999,
                    template: RepSpilloverTemplateLikeCpp::empty_like_cpp(),
                },
            ],
            &factions,
        );

        assert_eq!(store.len(), 1);
        assert_eq!(store.get(10).expect("stored template").faction[0], 11);
        assert_eq!(report.loaded, 1);
        assert_eq!(
            report
                .skipped
                .iter()
                .map(|skipped| skipped.reason)
                .collect::<Vec<_>>(),
            vec![
                RepSpilloverTemplateSkipReasonLikeCpp::SourceFactionHasNoParent,
                RepSpilloverTemplateSkipReasonLikeCpp::MissingSpilloverFaction {
                    slot: 0,
                    faction_id: 999,
                },
                RepSpilloverTemplateSkipReasonLikeCpp::InvalidRank { slot: 0, rank: 8 },
                RepSpilloverTemplateSkipReasonLikeCpp::SpilloverFactionCannotHaveReputation {
                    slot: 0,
                    faction_id: 13,
                },
                RepSpilloverTemplateSkipReasonLikeCpp::MissingSourceFaction,
            ]
        );
    }

    #[test]
    fn reputation_rank_thresholds_match_cpp_boundaries() {
        use ReputationRankLikeCpp::{
            Exalted, Friendly, Hated, Honored, Hostile, Neutral, Revered, Unfriendly,
        };

        assert_eq!(REPUTATION_BOTTOM_LIKE_CPP, -42000);
        assert_eq!(REPUTATION_CAP_LIKE_CPP, 42000);
        assert_eq!(
            REPUTATION_RANK_THRESHOLDS_LIKE_CPP,
            [-42000, -6000, -3000, 0, 3000, 9000, 21000, 42000]
        );

        for (standing, expected) in [
            (-50000, Hated),
            (-42000, Hated),
            (-6001, Hated),
            (-6000, Hostile),
            (-3001, Hostile),
            (-3000, Unfriendly),
            (-1, Unfriendly),
            (0, Neutral),
            (2999, Neutral),
            (3000, Friendly),
            (8999, Friendly),
            (9000, Honored),
            (20999, Honored),
            (21000, Revered),
            (41999, Revered),
            (42000, Exalted),
            (50000, Exalted),
        ] {
            assert_eq!(reputation_rank_from_standing_like_cpp(standing), expected);
        }
    }

    #[test]
    fn reputation_flags_match_cpp_underlying_bits() {
        assert_eq!(ReputationFlagsLikeCpp::VISIBLE.bits(), 0x0001);
        assert_eq!(ReputationFlagsLikeCpp::AT_WAR.bits(), 0x0002);
        assert_eq!(ReputationFlagsLikeCpp::HIDDEN.bits(), 0x0004);
        assert_eq!(ReputationFlagsLikeCpp::HEADER.bits(), 0x0008);
        assert_eq!(ReputationFlagsLikeCpp::PEACEFUL.bits(), 0x0010);
        assert_eq!(ReputationFlagsLikeCpp::INACTIVE.bits(), 0x0020);
        assert_eq!(ReputationFlagsLikeCpp::SHOW_PROPAGATED.bits(), 0x0040);
        assert_eq!(ReputationFlagsLikeCpp::HEADER_SHOWS_BAR.bits(), 0x0080);
        assert_eq!(
            ReputationFlagsLikeCpp::CAPITAL_CITY_FOR_RACE_CHANGE.bits(),
            0x0100
        );
        assert_eq!(ReputationFlagsLikeCpp::GUILD.bits(), 0x0200);
        assert_eq!(ReputationFlagsLikeCpp::GARRISON_INVASION.bits(), 0x0400);
    }
}
