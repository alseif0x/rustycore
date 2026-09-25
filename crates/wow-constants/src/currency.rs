// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Currency gain-source values emitted in quest and currency update packets.

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurrencyGainSourceLikeCpp {
    QuestReward = 3,
    QuestRewardIgnoreCaps = 29,
    WorldQuestReward = 35,
    WorldQuestRewardIgnoreCaps = 36,
    DailyQuestReward = 38,
    WeeklyQuestReward = 40,
}
