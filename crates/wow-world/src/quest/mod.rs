// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Quest operations owned as complete units.
//!
//! Protocol adaptation for quests stays in `handlers::quest`; this module owns
//! the operations themselves — what they change, in which order, and what has
//! to be durable before the player keeps the result.

pub(crate) mod application;
