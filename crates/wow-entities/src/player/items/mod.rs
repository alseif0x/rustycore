// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Player item storage, equipment and enchantment.
//!
//! Named `items`, not `inventory`: a module of the latter name shadows the
//! `inventory` crate namespace, which the handler-registration guard rejects.

mod enchantment;
mod equipment;
mod storage;
