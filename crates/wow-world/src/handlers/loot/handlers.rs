// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The creature loot-release VALUES receiver moved to `wow-world-loot-ops` and
//! the creature melee-damage receiver to `wow-world-entities` (#1363 slice 7a);
//! the loot command receivers and the `CMSG_LOOT_MONEY` body moved to
//! `wow-world-loot-ops` (`session_loot::handlers`) in slice 5c. Only the test
//! entry points stay here.

#[cfg(test)]
mod test_shims;
