// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Login spell/skill projections: re-exported from `wow-world-spell` after the
//! #1263 F5 move.

pub(in crate::handlers::character) use wow_world_spell::login_spell_rules::*;
