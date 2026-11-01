// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Shared gameplay limits used by both the session core and the application
//! layer. Single definition; consumers re-export rather than redeclare.

/// C++ `MAX_QUEST_LOG_SIZE` (`Quests/QuestDef.h:43`).
pub const MAX_QUEST_LOG_SIZE_LIKE_CPP: u8 = 25;
