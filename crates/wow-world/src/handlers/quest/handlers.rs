// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Quest packet entry points and their handler registrations.

use super::*;
use wow_packet::ClientPacket;

mod acceptance;
mod queries;
mod reward_flow;
mod sharing;

mod registrations;
mod navigation;
