// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Login, world entry, logout and the client-state handshake. The world
//! entry, logout, login packet bursts, login loaders and login recovery moved
//! to `wow-world-character` (#1363 slice 8d); the continue-login coordinator
//! stays here.

use super::*;

mod login;
