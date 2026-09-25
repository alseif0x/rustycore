// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Auction and trade packet handlers.

mod auction;
mod trade;

const SILVER_LIKE_CPP: u64 = 100;
const MIN_AUCTION_TIME_MINUTES_LIKE_CPP: u32 = 12 * 60;
const SHORT_AUCTION_TIME_MINUTES_LIKE_CPP: u32 = MIN_AUCTION_TIME_MINUTES_LIKE_CPP;
const MEDIUM_AUCTION_TIME_MINUTES_LIKE_CPP: u32 = 2 * MIN_AUCTION_TIME_MINUTES_LIKE_CPP;
const LONG_AUCTION_TIME_MINUTES_LIKE_CPP: u32 = 4 * MIN_AUCTION_TIME_MINUTES_LIKE_CPP;

#[cfg(test)]
#[path = "economy/tests/mod.rs"]
mod tests;
