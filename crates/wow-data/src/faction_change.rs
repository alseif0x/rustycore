// Copyright (c) 2026 alseif0x
// RustyCore - WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 - https://www.gnu.org/licenses/gpl-3.0.html

//! C++ `ObjectMgr::LoadFactionChange*` represented stores.

use std::collections::HashMap;

use anyhow::Result;
use wow_database::{WorldDatabase, WorldStatements};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FactionChangePairKindLikeCpp {
    Achievement,
    Quest,
    Reputation,
    Spell,
    Title,
}

impl FactionChangePairKindLikeCpp {
    fn table_name_like_cpp(self) -> &'static str {
        match self {
            Self::Achievement => "player_factionchange_achievement",
            Self::Quest => "player_factionchange_quests",
            Self::Reputation => "player_factionchange_reputations",
            Self::Spell => "player_factionchange_spells",
            Self::Title => "player_factionchange_title",
        }
    }

    fn label_like_cpp(self) -> &'static str {
        match self {
            Self::Achievement => "Achievement",
            Self::Quest => "Quest",
            Self::Reputation => "Reputation",
            Self::Spell => "Spell",
            Self::Title => "Title",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FactionChangePairRowLikeCpp {
    pub alliance_id: u32,
    pub horde_id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FactionChangeValidationErrorLikeCpp {
    pub kind: FactionChangePairKindLikeCpp,
    pub side: FactionChangeSideLikeCpp,
    pub id: u32,
    pub table: &'static str,
}

impl FactionChangeValidationErrorLikeCpp {
    pub fn cpp_message_like_cpp(&self) -> String {
        format!(
            "{} {} ({}) referenced in `{}` does not exist, pair skipped!",
            self.kind.label_like_cpp(),
            self.id,
            self.side.column_like_cpp(),
            self.table
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FactionChangeSideLikeCpp {
    Alliance,
    Horde,
}

impl FactionChangeSideLikeCpp {
    fn column_like_cpp(self) -> &'static str {
        match self {
            Self::Alliance => "alliance_id",
            Self::Horde => "horde_id",
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct FactionChangeLoadReportLikeCpp {
    pub achievement_rows_seen: usize,
    pub quest_rows_seen: usize,
    pub reputation_rows_seen: usize,
    pub spell_rows_seen: usize,
    pub title_rows_seen: usize,
    pub item_rows_seen: usize,
    pub validation_errors: Vec<FactionChangeValidationErrorLikeCpp>,
    pub item_derivation_pending: bool,
}

impl FactionChangeLoadReportLikeCpp {
    pub fn total_sql_rows_seen_like_cpp(&self) -> usize {
        self.achievement_rows_seen
            + self.quest_rows_seen
            + self.reputation_rows_seen
            + self.spell_rows_seen
            + self.title_rows_seen
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct FactionChangeStoreLikeCpp {
    achievements: HashMap<u32, u32>,
    quests: HashMap<u32, u32>,
    reputations: HashMap<u32, u32>,
    spells: HashMap<u32, u32>,
    titles: HashMap<u32, u32>,
    items_alliance_to_horde: HashMap<u32, u32>,
    items_horde_to_alliance: HashMap<u32, u32>,
}

pub struct FactionChangeLoadOutcomeLikeCpp {
    pub store: FactionChangeStoreLikeCpp,
    pub report: FactionChangeLoadReportLikeCpp,
}

impl FactionChangeStoreLikeCpp {
    pub fn from_validated_rows_like_cpp<
        AchievementExists,
        QuestExists,
        ReputationExists,
        SpellExists,
        TitleExists,
    >(
        achievements: impl IntoIterator<Item = FactionChangePairRowLikeCpp>,
        quests: impl IntoIterator<Item = FactionChangePairRowLikeCpp>,
        reputations: impl IntoIterator<Item = FactionChangePairRowLikeCpp>,
        spells: impl IntoIterator<Item = FactionChangePairRowLikeCpp>,
        titles: impl IntoIterator<Item = FactionChangePairRowLikeCpp>,
        mut achievement_exists: AchievementExists,
        mut quest_exists: QuestExists,
        mut reputation_exists: ReputationExists,
        mut spell_exists: SpellExists,
        mut title_exists: TitleExists,
    ) -> FactionChangeLoadOutcomeLikeCpp
    where
        AchievementExists: FnMut(u32) -> bool,
        QuestExists: FnMut(u32) -> bool,
        ReputationExists: FnMut(u32) -> bool,
        SpellExists: FnMut(u32) -> bool,
        TitleExists: FnMut(u32) -> bool,
    {
        let mut store = Self::default();
        let mut report = FactionChangeLoadReportLikeCpp {
            item_derivation_pending: true,
            ..Default::default()
        };

        report.achievement_rows_seen = load_pair_map_like_cpp(
            FactionChangePairKindLikeCpp::Achievement,
            achievements,
            &mut achievement_exists,
            &mut store.achievements,
            &mut report.validation_errors,
        );
        report.quest_rows_seen = load_pair_map_like_cpp(
            FactionChangePairKindLikeCpp::Quest,
            quests,
            &mut quest_exists,
            &mut store.quests,
            &mut report.validation_errors,
        );
        report.reputation_rows_seen = load_pair_map_like_cpp(
            FactionChangePairKindLikeCpp::Reputation,
            reputations,
            &mut reputation_exists,
            &mut store.reputations,
            &mut report.validation_errors,
        );
        report.spell_rows_seen = load_pair_map_like_cpp(
            FactionChangePairKindLikeCpp::Spell,
            spells,
            &mut spell_exists,
            &mut store.spells,
            &mut report.validation_errors,
        );
        report.title_rows_seen = load_pair_map_like_cpp(
            FactionChangePairKindLikeCpp::Title,
            titles,
            &mut title_exists,
            &mut store.titles,
            &mut report.validation_errors,
        );

        FactionChangeLoadOutcomeLikeCpp { store, report }
    }

    /// C++ `ObjectMgr::LoadFactionChangeAchievements`, `LoadFactionChangeQuests`,
    /// `LoadFactionChangeReputations`, `LoadFactionChangeSpells`, and
    /// `LoadFactionChangeTitles`.
    pub async fn load_like_cpp<
        AchievementExists,
        QuestExists,
        ReputationExists,
        SpellExists,
        TitleExists,
    >(
        db: &WorldDatabase,
        achievement_exists: AchievementExists,
        quest_exists: QuestExists,
        reputation_exists: ReputationExists,
        spell_exists: SpellExists,
        title_exists: TitleExists,
    ) -> Result<FactionChangeLoadOutcomeLikeCpp>
    where
        AchievementExists: FnMut(u32) -> bool,
        QuestExists: FnMut(u32) -> bool,
        ReputationExists: FnMut(u32) -> bool,
        SpellExists: FnMut(u32) -> bool,
        TitleExists: FnMut(u32) -> bool,
    {
        let achievements =
            load_pair_rows_like_cpp(db, WorldStatements::SEL_FACTION_CHANGE_ACHIEVEMENTS).await?;
        let quests =
            load_pair_rows_like_cpp(db, WorldStatements::SEL_FACTION_CHANGE_QUESTS).await?;
        let reputations =
            load_pair_rows_like_cpp(db, WorldStatements::SEL_FACTION_CHANGE_REPUTATIONS).await?;
        let spells =
            load_pair_rows_like_cpp(db, WorldStatements::SEL_FACTION_CHANGE_SPELLS).await?;
        let titles =
            load_pair_rows_like_cpp(db, WorldStatements::SEL_FACTION_CHANGE_TITLES).await?;

        Ok(Self::from_validated_rows_like_cpp(
            achievements,
            quests,
            reputations,
            spells,
            titles,
            achievement_exists,
            quest_exists,
            reputation_exists,
            spell_exists,
            title_exists,
        ))
    }

    pub fn achievement_pair_like_cpp(&self, alliance_id: u32) -> Option<u32> {
        self.achievements.get(&alliance_id).copied()
    }

    pub fn quest_pair_like_cpp(&self, alliance_id: u32) -> Option<u32> {
        self.quests.get(&alliance_id).copied()
    }

    pub fn reputation_pair_like_cpp(&self, alliance_id: u32) -> Option<u32> {
        self.reputations.get(&alliance_id).copied()
    }

    pub fn spell_pair_like_cpp(&self, alliance_id: u32) -> Option<u32> {
        self.spells.get(&alliance_id).copied()
    }

    pub fn title_pair_like_cpp(&self, alliance_id: u32) -> Option<u32> {
        self.titles.get(&alliance_id).copied()
    }

    pub fn item_alliance_to_horde_like_cpp(&self, alliance_item_id: u32) -> Option<u32> {
        self.items_alliance_to_horde.get(&alliance_item_id).copied()
    }

    pub fn item_horde_to_alliance_like_cpp(&self, horde_item_id: u32) -> Option<u32> {
        self.items_horde_to_alliance.get(&horde_item_id).copied()
    }

    pub fn achievement_len(&self) -> usize {
        self.achievements.len()
    }

    pub fn quest_len(&self) -> usize {
        self.quests.len()
    }

    pub fn reputation_len(&self) -> usize {
        self.reputations.len()
    }

    pub fn spell_len(&self) -> usize {
        self.spells.len()
    }

    pub fn title_len(&self) -> usize {
        self.titles.len()
    }

    pub fn item_alliance_to_horde_len(&self) -> usize {
        self.items_alliance_to_horde.len()
    }

    pub fn item_horde_to_alliance_len(&self) -> usize {
        self.items_horde_to_alliance.len()
    }
}

fn load_pair_map_like_cpp<Exists>(
    kind: FactionChangePairKindLikeCpp,
    rows: impl IntoIterator<Item = FactionChangePairRowLikeCpp>,
    exists: &mut Exists,
    map: &mut HashMap<u32, u32>,
    errors: &mut Vec<FactionChangeValidationErrorLikeCpp>,
) -> usize
where
    Exists: FnMut(u32) -> bool,
{
    let mut count = 0;

    for row in rows {
        count += 1;
        if !exists(row.alliance_id) {
            errors.push(FactionChangeValidationErrorLikeCpp {
                kind,
                side: FactionChangeSideLikeCpp::Alliance,
                id: row.alliance_id,
                table: kind.table_name_like_cpp(),
            });
        } else if !exists(row.horde_id) {
            errors.push(FactionChangeValidationErrorLikeCpp {
                kind,
                side: FactionChangeSideLikeCpp::Horde,
                id: row.horde_id,
                table: kind.table_name_like_cpp(),
            });
        } else {
            map.insert(row.alliance_id, row.horde_id);
        }
    }

    count
}

async fn load_pair_rows_like_cpp(
    db: &WorldDatabase,
    statement: WorldStatements,
) -> Result<Vec<FactionChangePairRowLikeCpp>> {
    let stmt = db.prepare(statement);
    let mut result = db.query(&stmt).await?;
    let mut rows = Vec::new();

    if !result.is_empty() {
        loop {
            rows.push(FactionChangePairRowLikeCpp {
                alliance_id: result.read(0),
                horde_id: result.read(1),
            });

            if !result.next_row() {
                break;
            }
        }
    }

    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pair(alliance_id: u32, horde_id: u32) -> FactionChangePairRowLikeCpp {
        FactionChangePairRowLikeCpp {
            alliance_id,
            horde_id,
        }
    }

    #[test]
    fn faction_change_pairs_validate_and_map_alliance_to_horde_like_cpp() {
        let outcome = FactionChangeStoreLikeCpp::from_validated_rows_like_cpp(
            [pair(10, 20)],
            [pair(11, 21)],
            [pair(12, 22)],
            [pair(13, 23)],
            [pair(14, 24)],
            |id| matches!(id, 10 | 20),
            |id| matches!(id, 11 | 21),
            |id| matches!(id, 12 | 22),
            |id| matches!(id, 13 | 23),
            |id| matches!(id, 14 | 24),
        );

        assert_eq!(outcome.store.achievement_pair_like_cpp(10), Some(20));
        assert_eq!(outcome.store.quest_pair_like_cpp(11), Some(21));
        assert_eq!(outcome.store.reputation_pair_like_cpp(12), Some(22));
        assert_eq!(outcome.store.spell_pair_like_cpp(13), Some(23));
        assert_eq!(outcome.store.title_pair_like_cpp(14), Some(24));
        assert_eq!(outcome.report.total_sql_rows_seen_like_cpp(), 5);
        assert!(outcome.report.validation_errors.is_empty());
    }

    #[test]
    fn faction_change_invalid_rows_are_counted_but_skipped_like_cpp() {
        let outcome = FactionChangeStoreLikeCpp::from_validated_rows_like_cpp(
            [pair(1, 2), pair(3, 4), pair(5, 6)],
            [],
            [],
            [],
            [],
            |id| matches!(id, 1 | 2 | 3),
            |_| true,
            |_| true,
            |_| true,
            |_| true,
        );

        assert_eq!(outcome.report.achievement_rows_seen, 3);
        assert_eq!(outcome.store.achievement_len(), 1);
        assert_eq!(outcome.store.achievement_pair_like_cpp(1), Some(2));
        assert_eq!(outcome.store.achievement_pair_like_cpp(3), None);
        assert_eq!(outcome.store.achievement_pair_like_cpp(5), None);
        assert_eq!(outcome.report.validation_errors.len(), 2);
        assert_eq!(
            outcome.report.validation_errors[0].cpp_message_like_cpp(),
            "Achievement 4 (horde_id) referenced in `player_factionchange_achievement` does not exist, pair skipped!"
        );
        assert_eq!(
            outcome.report.validation_errors[1].cpp_message_like_cpp(),
            "Achievement 5 (alliance_id) referenced in `player_factionchange_achievement` does not exist, pair skipped!"
        );
    }

    #[test]
    fn faction_change_duplicate_alliance_overwrites_like_cpp_unordered_map_assignment() {
        let outcome = FactionChangeStoreLikeCpp::from_validated_rows_like_cpp(
            [pair(100, 200), pair(100, 201)],
            [],
            [],
            [],
            [],
            |_| true,
            |_| true,
            |_| true,
            |_| true,
            |_| true,
        );

        assert_eq!(outcome.report.achievement_rows_seen, 2);
        assert_eq!(outcome.store.achievement_len(), 1);
        assert_eq!(outcome.store.achievement_pair_like_cpp(100), Some(201));
    }

    #[test]
    fn faction_change_items_are_declared_pending_until_other_faction_item_id_is_ported() {
        let outcome = FactionChangeStoreLikeCpp::from_validated_rows_like_cpp(
            [],
            [],
            [],
            [],
            [],
            |_| true,
            |_| true,
            |_| true,
            |_| true,
            |_| true,
        );

        assert!(outcome.report.item_derivation_pending);
        assert_eq!(outcome.report.item_rows_seen, 0);
        assert_eq!(outcome.store.item_alliance_to_horde_len(), 0);
        assert_eq!(outcome.store.item_horde_to_alliance_len(), 0);
    }
}
