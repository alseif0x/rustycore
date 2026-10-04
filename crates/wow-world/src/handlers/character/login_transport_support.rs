// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Login transport create-block support: re-exported from
//! `wow-world-lifecycle` after the #1263 F5 move.

pub(crate) use wow_world_lifecycle::login_transport::*;
pub(crate) use wow_world_visibility::InitTransportsPlanLikeCpp;
