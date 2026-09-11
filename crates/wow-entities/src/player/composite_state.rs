// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Whole-state installs for the four composite Player states the session
//! mutates through a snapshot.
//!
//! Each session mutator takes a copy of the state, runs its caller's closure on
//! that copy and writes the result back. Only the write-back touched the
//! Player, so naming the install retires the broad reach outright instead of
//! relocating it: the closure never sees the Player at all.
//!
//! Each state is a composite of fields C++ sets one by one; the anchors are
//! recorded per method so the grouping stays traceable to them.

use crate::{
    Player, PlayerBattlegroundState, PlayerCollectionStateLikeCpp, PlayerCurrency,
    PlayerGuildState, PlayerPersistentCapabilityStateLikeCpp, PlayerTradeStateLikeCpp,
};
use std::collections::HashMap;

impl Player {
    /// Install the Player's guild membership state, the composite of C++
    /// `Player::SetInGuild` (Player.h:1938), `Player::SetGuildRank`
    /// (Player.h:1939) and `Player::SetGuildIdInvited` (Player.h:1943), plus
    /// the `_LoadGuild` authority marker that records membership as resolved
    /// even when the character has no guild.
    pub fn install_guild_state_like_cpp(&mut self, guild: PlayerGuildState) {
        self.gameplay_state_mut().guild = guild;
    }

    /// Install the Player's trade state, C++ `Player::m_trade`: the handler
    /// creates it on both sides when a trade opens (TradeHandler.cpp:694-695)
    /// and `None` is the normal no-trade state C++ clears back to
    /// (Player.cpp:12877).
    pub fn install_trade_state_like_cpp(&mut self, trade: Option<PlayerTradeStateLikeCpp>) {
        self.gameplay_state_mut().trade = trade;
    }

    /// Install the Player's persistent capability state: the at-login flags
    /// C++ keeps in `Player::m_atLoginFlags` (`SetAtLoginFlag`, Player.h:2474)
    /// together with the proficiency masks behind
    /// `Player::AddWeaponProficiency` and `Player::AddArmorProficiency`
    /// (Player.h:1433-1434).
    pub fn install_persistent_capabilities_like_cpp(
        &mut self,
        capabilities: PlayerPersistentCapabilityStateLikeCpp,
    ) {
        self.gameplay_state_mut().persistent_capabilities = capabilities;
    }

    /// Install the Player's battleground state, the composite of C++
    /// `Player::m_bgData` (Player.h:2335-2337 read it) and the arena-team
    /// invitation `Player::SetArenaTeamIdInvited` (Player.h:1956) keeps beside
    /// it.
    pub fn install_battleground_state_like_cpp(&mut self, battleground: PlayerBattlegroundState) {
        self.gameplay_state_mut().battleground = battleground;
    }

    /// Install the Player's currency map, C++ `Player::_currencyStorage`
    /// (Player.h:2938), as its two writers leave it: the character load
    /// (`Player::_LoadCurrency`, Player.cpp:6771) and gameplay changes
    /// (`Player::ModifyCurrency`, Player.cpp:6916). The caller decides which
    /// of the two it is performing.
    pub fn install_currencies_like_cpp(&mut self, currencies: HashMap<u32, PlayerCurrency>) {
        self.gameplay_state_mut().currencies = currencies;
    }

    /// Install the represented account-wide collection state.
    ///
    /// Departure kept explicit: C++ owns collections on the session
    /// (`WorldSession::_collectionMgr`, WorldSession.h:1938), not on the
    /// Player. RustyCore already keeps the represented state on the Player;
    /// naming the install makes that departure visible at the boundary instead
    /// of burying it in a raw field write, and moving the owner is a separate
    /// question from this one.
    pub fn install_collection_state_like_cpp(&mut self, collections: PlayerCollectionStateLikeCpp) {
        self.gameplay_state_mut().collections = collections;
    }
}
