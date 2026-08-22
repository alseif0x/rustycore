// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Unit threat subsystem.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u8)]
pub enum ThreatOnlineState {
    Offline = 0,
    Suppressed = 1,
    Online = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ThreatTauntState {
    Detaunt,
    None,
    Taunt(u32),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThreatReferenceState {
    pub base_amount: f32,
    pub temp_modifier: i32,
    pub online_state: ThreatOnlineState,
    pub taunt_state: ThreatTauntState,
}

impl ThreatReferenceState {
    pub fn threat(&self) -> f32 {
        (self.base_amount + self.temp_modifier as f32).max(0.0)
    }

    pub const fn is_online(&self) -> bool {
        matches!(self.online_state, ThreatOnlineState::Online)
    }

    pub const fn is_available(&self) -> bool {
        !matches!(self.online_state, ThreatOnlineState::Offline)
    }

    pub const fn is_offline(&self) -> bool {
        matches!(self.online_state, ThreatOnlineState::Offline)
    }

    pub const fn is_suppressed(&self) -> bool {
        matches!(self.online_state, ThreatOnlineState::Suppressed)
    }

    pub const fn is_taunting(&self) -> bool {
        matches!(self.taunt_state, ThreatTauntState::Taunt(_))
    }

    pub const fn is_detaunted(&self) -> bool {
        matches!(self.taunt_state, ThreatTauntState::Detaunt)
    }

    pub fn add_threat(&mut self, amount: f32) {
        if amount != 0.0 {
            self.base_amount = (self.base_amount + amount).max(0.0);
        }
    }

    pub fn scale_threat(&mut self, factor: f32) {
        self.base_amount *= factor.max(0.0);
    }

    pub fn modify_threat_by_percent(&mut self, percent: i32) {
        if percent != 0 {
            self.scale_threat(0.01 * (100 + percent) as f32);
        }
    }

    pub fn set_taunt_state(&mut self, taunt_state: ThreatTauntState) {
        self.taunt_state = taunt_state;
    }

    pub fn set_online_state(&mut self, online_state: ThreatOnlineState) {
        self.online_state = online_state;
    }
}
