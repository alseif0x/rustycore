// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! trade capability handler tests.

use super::*;
use crate::session::TRADE_STATUS_PLAYER_BUSY_LIKE_CPP;
use wow_packet::packets::misc::{
    TRADE_STATUS_CANCELLED_LIKE_CPP, TRADE_STATUS_PLAYER_IGNORED_LIKE_CPP,
};

mod contents;
mod duel;
mod petition;
mod session;
