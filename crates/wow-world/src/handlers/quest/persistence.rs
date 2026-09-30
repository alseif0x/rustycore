// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Quest status persistence and load.

use super::*;
use wow_data_model::quest_poi::{QuestPoiBlobData, QuestPoiBlobPoint};

mod item_mutations;

mod poi;
mod saves;
mod hydration;
