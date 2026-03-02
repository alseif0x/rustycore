// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Player level stats loaded from the `player_levelstats` world DB table.
//!
//! C# loads these at startup in `ObjectManager.LoadPlayerInfo()` and uses
//! them to compute base health, mana, armor, and primary stats for each
//! race/class/level combination.

use std::collections::HashMap;

use anyhow::{Context, Result};
use tracing::info;
use wow_database::{WorldDatabase, WorldStatements};

/// Base stats for a specific race/class/level combination.
#[derive(Debug, Clone, Copy)]
pub struct PlayerLevelStats {
    pub strength: u16,
    pub agility: u16,
    pub stamina: u16,
    pub intellect: u16,
    pub spirit: u16,
    pub base_health: u16,
    pub base_mana: u16,
}

impl PlayerLevelStats {
    /// Compute max health from base stats.
    ///
    /// Formula (matches C# `Player.InitStatsForLevel`):
    /// `MaxHealth = baseHealth + min(STA, 20) + max(0, STA - 20) * 10`
    pub fn max_health(&self) -> i64 {
        let sta = self.stamina as i64;
        let base = self.base_health as i64;
        let bonus = sta.min(20) + (sta - 20).max(0) * 10;
        base + bonus
    }

    /// Compute max mana from base stats.
    ///
    /// Formula (matches C# `Player.InitStatsForLevel`):
    /// `MaxMana = baseMana + min(INT, 20) + max(0, INT - 20) * 15`
    pub fn max_mana(&self) -> i64 {
        let int = self.intellect as i64;
        let base = self.base_mana as i64;
        let bonus = int.min(20) + (int - 20).max(0) * 15;
        base + bonus
    }

    /// Compute base armor from agility.
    ///
    /// Formula: `Armor = AGI * 2`
    pub fn base_armor(&self) -> i32 {
        self.agility as i32 * 2
    }

    /// Melee attack power from base stats (no gear).
    ///
    /// WotLK formulas from C# `Player.UpdateAttackPowerAndDamage`:
    /// - War/Pal/DK: STR*2 - 20
    /// - Hunter/Rogue: STR + AGI - 20
    /// - Shaman/Druid: STR*2 - 20
    /// - Casters: STR - 10
    pub fn melee_attack_power(&self, class: u8) -> i32 {
        let str = self.strength as i32;
        let agi = self.agility as i32;
        let ap = match class {
            1 | 2 | 6 => str * 2 - 20,        // Warrior, Paladin, DK
            3 | 4 => str + agi - 20,            // Hunter, Rogue
            7 | 11 => str * 2 - 20,             // Shaman, Druid
            _ => (str - 10).max(0),              // Casters
        };
        ap.max(0)
    }

    /// Ranged attack power from base stats (no gear).
    pub fn ranged_attack_power(&self, class: u8) -> i32 {
        let agi = self.agility as i32;
        let rap = match class {
            3 => agi * 2 - 20,       // Hunter: 2 RAP per AGI
            1 | 4 => agi - 10,       // Warrior, Rogue
            _ => 0,
        };
        rap.max(0)
    }

    /// Base unarmed min/max damage from attack power.
    ///
    /// Unarmed: base_speed=2.0s, damage = AP/14 * speed.
    pub fn base_melee_damage(&self, class: u8) -> (f32, f32) {
        let ap = self.melee_attack_power(class) as f32;
        let base = ap / 14.0 * 2.0; // 2.0s weapon speed
        let min = (base + 1.0).max(1.0);
        let max = min + 1.0;
        (min, max)
    }

    /// Base ranged min/max damage.
    pub fn base_ranged_damage(&self, class: u8) -> (f32, f32) {
        let rap = self.ranged_attack_power(class) as f32;
        if rap <= 0.0 {
            return (0.0, 0.0);
        }
        let base = rap / 14.0 * 2.8; // 2.8s ranged speed
        let min = (base + 1.0).max(1.0);
        let max = min + 2.0;
        (min, max)
    }

    /// Base dodge percentage from agility.
    ///
    /// Approximate base dodge + AGI contribution (varies by class at level 80).
    pub fn dodge_pct(&self, class: u8, level: u8) -> f32 {
        let agi = self.agility as f32;
        // AGI per 1% dodge at level 80 (approximate WotLK values)
        let (base, agi_per_pct) = match class {
            1 => (3.66, 59.88),     // Warrior
            2 => (3.49, 59.88),     // Paladin
            3 => (0.0, 33.56),      // Hunter
            4 => (2.09, 47.85),     // Rogue
            5 => (3.42, 66.75),     // Priest
            6 => (3.66, 59.88),     // DK
            7 => (2.11, 59.88),     // Shaman
            8 => (3.66, 82.0),      // Mage
            9 => (2.01, 66.75),     // Warlock
            11 => (5.61, 47.85),    // Druid
            _ => (3.0, 60.0),
        };
        // Scale agi_per_pct for low levels (lower levels need less AGI)
        let level_factor = if level >= 80 { 1.0 } else { (level as f32 / 80.0).max(0.1) };
        let scaled_agi_per_pct = agi_per_pct * level_factor;
        (base + agi / scaled_agi_per_pct.max(1.0)).max(0.0)
    }

    /// Base parry percentage (melee classes only).
    pub fn parry_pct(&self, class: u8) -> f32 {
        match class {
            1 | 2 | 6 => 5.0,   // Warrior, Paladin, DK
            4 => 5.0,            // Rogue
            7 | 11 => 5.0,       // Shaman, Druid
            _ => 0.0,            // Casters don't parry by default
        }
    }

    /// Base crit percentage from agility.
    pub fn crit_pct(&self, class: u8, level: u8) -> f32 {
        let agi = self.agility as f32;
        // AGI per 1% crit at level 80
        let agi_per_pct = match class {
            1 => 62.22,   // Warrior
            2 => 62.22,   // Paladin
            3 => 83.33,   // Hunter
            4 => 51.02,   // Rogue
            6 => 62.22,   // DK
            7 => 62.22,   // Shaman
            11 => 51.02,  // Druid
            _ => 80.0,    // Casters
        };
        let level_factor = if level >= 80 { 1.0 } else { (level as f32 / 80.0).max(0.1) };
        let scaled = agi_per_pct * level_factor;
        5.0 + agi / scaled.max(1.0) // base 5% + AGI contribution
    }

    /// Base spell crit percentage from intellect.
    pub fn spell_crit_pct(&self, class: u8, level: u8) -> f32 {
        let int = self.intellect as f32;
        // INT per 1% spell crit at level 80
        let int_per_pct = match class {
            2 => 166.67,  // Paladin
            3 => 166.67,  // Hunter
            5 => 136.0,   // Priest
            7 => 166.67,  // Shaman
            8 => 151.0,   // Mage
            9 => 151.0,   // Warlock
            11 => 166.67, // Druid
            _ => 200.0,   // Non-casters
        };
        let level_factor = if level >= 80 { 1.0 } else { (level as f32 / 80.0).max(0.1) };
        let scaled = int_per_pct * level_factor;
        int / scaled.max(1.0) // no base spell crit
    }
}

/// In-memory store of player level stats keyed by (race, class, level).
pub struct PlayerStatsStore {
    stats: HashMap<(u8, u8, u8), PlayerLevelStats>,
}

impl PlayerStatsStore {
    /// Load all entries from `player_levelstats` table.
    pub async fn load(world_db: &WorldDatabase) -> Result<Self> {
        let stmt = world_db.prepare(WorldStatements::SEL_PLAYER_LEVELSTATS);
        let mut result = world_db
            .query(&stmt)
            .await
            .context("Failed to query player_levelstats")?;

        let mut stats = HashMap::new();

        if !result.is_empty() {
            loop {
                let race: u8 = result.read(0);
                let class: u8 = result.read(1);
                let level: u8 = result.read(2);
                let strength: u16 = result.read(3);
                let agility: u16 = result.read(4);
                let stamina: u16 = result.read(5);
                let intellect: u16 = result.read(6);
                let spirit: u16 = result.read(7);
                let base_health: u16 = result.read(8);
                let base_mana: u16 = result.read(9);

                stats.insert(
                    (race, class, level),
                    PlayerLevelStats {
                        strength,
                        agility,
                        stamina,
                        intellect,
                        spirit,
                        base_health,
                        base_mana,
                    },
                );

                if !result.next_row() {
                    break;
                }
            }
        }

        info!("Loaded {} player level stat entries", stats.len());
        Ok(Self { stats })
    }

    /// Look up stats for a specific race/class/level.
    pub fn get(&self, race: u8, class: u8, level: u8) -> Option<&PlayerLevelStats> {
        self.stats.get(&(race, class, level))
    }

    /// Number of entries loaded.
    pub fn len(&self) -> usize {
        self.stats.len()
    }

    /// Whether the store is empty.
    pub fn is_empty(&self) -> bool {
        self.stats.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn max_health_low_stamina() {
        let stats = PlayerLevelStats {
            strength: 20, agility: 20, stamina: 15,
            intellect: 20, spirit: 20, base_health: 28, base_mana: 0,
        };
        // STA <= 20: bonus = STA = 15
        assert_eq!(stats.max_health(), 28 + 15);
    }

    #[test]
    fn max_health_high_stamina() {
        let stats = PlayerLevelStats {
            strength: 20, agility: 20, stamina: 50,
            intellect: 20, spirit: 20, base_health: 100, base_mana: 0,
        };
        // STA > 20: bonus = 20 + (50 - 20) * 10 = 20 + 300 = 320
        assert_eq!(stats.max_health(), 100 + 320);
    }

    #[test]
    fn max_mana_low_intellect() {
        let stats = PlayerLevelStats {
            strength: 20, agility: 20, stamina: 20,
            intellect: 18, spirit: 20, base_health: 50, base_mana: 60,
        };
        // INT <= 20: bonus = INT = 18
        assert_eq!(stats.max_mana(), 60 + 18);
    }

    #[test]
    fn max_mana_high_intellect() {
        let stats = PlayerLevelStats {
            strength: 20, agility: 20, stamina: 20,
            intellect: 40, spirit: 20, base_health: 50, base_mana: 100,
        };
        // INT > 20: bonus = 20 + (40 - 20) * 15 = 20 + 300 = 320
        assert_eq!(stats.max_mana(), 100 + 320);
    }

    #[test]
    fn base_armor_from_agility() {
        let stats = PlayerLevelStats {
            strength: 20, agility: 35, stamina: 20,
            intellect: 20, spirit: 20, base_health: 50, base_mana: 0,
        };
        assert_eq!(stats.base_armor(), 70);
    }
}
