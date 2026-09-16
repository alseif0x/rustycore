// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Gameplay policy value types composed by `world-server`.
//!
//! These values originate in world-server configuration and affect gameplay
//! decisions after authentication. Capability bundles borrow process-owned
//! values into the relevant handlers; mutable per-session counters remain on
//! their canonical runtime owner.

/// C++ `World::rate_values` subset used by loot generation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LootDropRatesLikeCpp {
    pub item_poor: f32,
    pub item_normal: f32,
    pub item_uncommon: f32,
    pub item_rare: f32,
    pub item_epic: f32,
    pub item_legendary: f32,
    pub item_artifact: f32,
    pub item_referenced: f32,
    pub item_referenced_amount: f32,
    pub money: f32,
    pub corpse_decay_looted: f32,
}

impl Default for LootDropRatesLikeCpp {
    fn default() -> Self {
        Self {
            item_poor: 1.0,
            item_normal: 1.0,
            item_uncommon: 1.0,
            item_rare: 1.0,
            item_epic: 1.0,
            item_legendary: 1.0,
            item_artifact: 1.0,
            item_referenced: 1.0,
            item_referenced_amount: 1.0,
            money: 1.0,
            corpse_decay_looted: 0.5,
        }
    }
}

/// C++ `World::rate_values` subset used by reputation gain.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReputationRatesLikeCpp {
    pub gain: f32,
    pub low_level_kill: f32,
    pub low_level_quest: f32,
    pub recruit_a_friend_bonus: f32,
    pub recruit_a_friend_distance: f32,
}

impl Default for ReputationRatesLikeCpp {
    fn default() -> Self {
        Self {
            gain: 1.0,
            low_level_kill: 1.0,
            low_level_quest: 1.0,
            recruit_a_friend_bonus: 0.1,
            recruit_a_friend_distance: 100.0,
        }
    }
}

/// C++ `World::rate_values` subset consumed by `Player::Regenerate` and
/// `Player::RegenerateHealth` (`World.cpp:615-623`, `Player.cpp:1681-1747`,
/// `Player.cpp:1842-1882`).
///
/// C++ `RatesForPower` only maps a subset of `Powers`; the remaining entries
/// use `MAX_RATES` (no multiplication), which [`Self::rate_for_power_like_cpp`]
/// returns as `None`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayerRegenerationRatesLikeCpp {
    /// C++ `RATE_HEALTH` (`Rate.Health`).
    pub health: f32,
    /// C++ `RATE_POWER_MANA` (`Rate.Mana`).
    pub mana: f32,
    /// C++ `RATE_POWER_RAGE_LOSS` (`Rate.Rage.Loss`).
    pub rage_loss: f32,
    /// C++ `RATE_POWER_FOCUS` (`Rate.Focus`).
    pub focus: f32,
    /// C++ `RATE_POWER_ENERGY` (`Rate.Energy`).
    pub energy: f32,
    /// C++ `RATE_POWER_RUNIC_POWER_LOSS` (`Rate.RunicPower.Loss`).
    pub runic_power_loss: f32,
}

impl Default for PlayerRegenerationRatesLikeCpp {
    fn default() -> Self {
        Self {
            health: 1.0,
            mana: 1.0,
            rage_loss: 1.0,
            focus: 1.0,
            energy: 1.0,
            runic_power_loss: 1.0,
        }
    }
}

impl PlayerRegenerationRatesLikeCpp {
    /// C++ `RatesForPower[power]`: `Some(rate)` for the represented powers and
    /// `None` for the `MAX_RATES` entries. Unrepresented powers (soul shards,
    /// lunar power, combo points, ...) remain an explicit boundary.
    #[must_use]
    pub fn rate_for_power_like_cpp(&self, power: wow_constants::PowerType) -> Option<f32> {
        match power {
            wow_constants::PowerType::Mana => Some(self.mana),
            wow_constants::PowerType::Rage => Some(self.rage_loss),
            wow_constants::PowerType::Focus => Some(self.focus),
            wow_constants::PowerType::Energy => Some(self.energy),
            wow_constants::PowerType::RunicPower => Some(self.runic_power_loss),
            _ => None,
        }
    }
}

/// C++ `ChatLevelReq.*` process policy value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChatLevelRequirementsLikeCpp {
    pub channel: u8,
    pub whisper: u8,
    pub emote: u8,
    pub say: u8,
    pub yell: u8,
}

impl Default for ChatLevelRequirementsLikeCpp {
    fn default() -> Self {
        Self {
            channel: 1,
            whisper: 1,
            emote: 1,
            say: 1,
            yell: 1,
        }
    }
}

/// C++ `ListenRange.*` process policy value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChatListenRangesLikeCpp {
    pub say: f32,
    pub text_emote: f32,
    pub yell: f32,
}

impl Default for ChatListenRangesLikeCpp {
    fn default() -> Self {
        Self {
            say: 25.0,
            text_emote: 25.0,
            yell: 300.0,
        }
    }
}

/// C++ `ChatFlood.*` process policy value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChatFloodConfigLikeCpp {
    pub message_count: u32,
    pub message_delay_secs: u32,
    pub addon_message_count: u32,
    pub addon_message_delay_secs: u32,
    pub mute_time_secs: u32,
}

impl Default for ChatFloodConfigLikeCpp {
    fn default() -> Self {
        Self {
            message_count: 10,
            message_delay_secs: 1,
            addon_message_count: 100,
            addon_message_delay_secs: 1,
            mute_time_secs: 10,
        }
    }
}

/// C++ `PacketSpoof.*` gameplay policy carried by `WorldSession::DosProtection`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PacketSpoofConfigLikeCpp {
    pub policy: u32,
    pub ban_mode: u32,
    pub ban_duration_secs: u32,
}

impl PacketSpoofConfigLikeCpp {
    pub const POLICY_LOG: u32 = 0;
    pub const POLICY_KICK: u32 = 1;
    pub const POLICY_BAN: u32 = 2;
    pub const BAN_ACCOUNT: u32 = 0;
    pub const BAN_IP: u32 = 2;
}

impl Default for PacketSpoofConfigLikeCpp {
    fn default() -> Self {
        Self {
            policy: Self::POLICY_KICK,
            ban_mode: Self::BAN_ACCOUNT,
            ban_duration_secs: 86_400,
        }
    }
}
