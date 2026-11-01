// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Publication of the process-wide world-session packet-handler composition.
//!
//! The single ordered composition lives in `wow-world`'s crate-level
//! `handler_composition` module as
//! `wow_world::handler_composition::compose_packet_handlers_like_cpp`. This
//! module only re-exports it, so the fixture dispatch table and every
//! process-wide consumer share one ordered-list authority and a new handler
//! owner is registered exactly once.

pub use wow_world::handler_composition::compose_packet_handlers_like_cpp;
