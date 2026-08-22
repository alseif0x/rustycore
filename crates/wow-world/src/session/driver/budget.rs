// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Bounded work budget for one Session driver pass.
//!
//! The two ingestion phases share a single budget so a busy realm channel
//! cannot starve the instance channel or let one pass run unbounded.

/// Maximum number of packets processed per `update()` call.
pub(crate) const MAX_PACKETS_PER_UPDATE: usize = 100;
