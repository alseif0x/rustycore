// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use crate::session::state::hub_support::RepresentedPlayerGearStatsLikeCpp;

impl super::PlayerStatsAccessLikeCpp<'_> {
    pub fn mana_regen_from_stats_like_cpp(&self, level: u8, class: u8, stats: [i32; 5]) -> f32 {
        // C++ `Player::OCTRegenMPPerSpirit` returns Spirit multiplied by the
        // level/class row from `RegenMPPerSpt.txt`; `UpdateManaRegen` then
        // multiplies that value by sqrt(Intellect). Aura percentages are a
        // separate producer and are intentionally not folded into this
        // equipment projection.
        (stats[3].max(0) as f32).sqrt()
            * stats[4] as f32
            * self.catalogs.mana_regen_ratio_like_cpp(level, class)
    }

    pub fn mana_regen_aura_multiplier_like_cpp(&self) -> f32 {
        self.with_aura_view(|view| view.mana_regen_aura_multiplier())
            .unwrap_or(1.0)
    }

    pub fn mana_regen_mp5_from_auras_like_cpp(&self, stats: [i32; 5]) -> f32 {
        self.with_aura_view(|view| view.mana_regen_mp5_from_auras(stats))
            .unwrap_or_default()
    }

    pub fn mana_regen_interrupt_modifier_like_cpp(&self) -> f32 {
        self.with_aura_view(|view| view.mana_regen_interrupt_modifier())
            .unwrap_or_default()
    }

    pub fn represented_resistance_aura_multiplier_like_cpp(
        &self,
        aura_type: i32,
        school_mask: i32,
    ) -> f32 {
        self.with_aura_view(|view| view.resistance_aura_multiplier(aura_type, school_mask))
            .unwrap_or(1.0)
    }

    pub fn represented_total_aura_multiplier_like_cpp(&self, aura_type: i32) -> f32 {
        self.with_aura_view(|view| view.total_aura_multiplier(aura_type))
            .unwrap_or(1.0)
    }

    pub fn represented_override_attack_power_by_spell_power_pct_like_cpp(&self) -> Option<f32> {
        self.with_aura_view(|view| view.override_attack_power_by_spell_power_percent())?
    }

    pub fn represented_attack_power_flat_aura_like_cpp(&self) -> i32 {
        self.with_aura_view(|view| view.attack_power_flat_aura())
            .unwrap_or_default()
    }

    pub fn represented_ranged_attack_power_flat_aura_like_cpp(&self, class: u8) -> i32 {
        self.with_aura_view(|view| view.ranged_attack_power_flat_aura(class))
            .unwrap_or_default()
    }

    pub fn represented_ranged_attack_power_total_pct_like_cpp(&self, class: u8) -> f32 {
        self.with_aura_view(|view| view.ranged_attack_power_total_pct(class))
            .unwrap_or(1.0)
    }

    pub fn represented_resistance_aura_flat_like_cpp(&self, school_mask: i32) -> f32 {
        self.with_aura_view(|view| view.resistance_aura_flat(school_mask))
            .unwrap_or_default()
    }

    pub fn represented_school_resistances_like_cpp(
        &self,
        gear: &RepresentedPlayerGearStatsLikeCpp,
    ) -> [i32; 6] {
        self.with_aura_view(|view| view.school_resistances(gear))
            .unwrap_or_else(|| {
                std::array::from_fn(|index| (gear.resistances[index + 1] as f32) as i32)
            })
    }

    pub fn represented_armor_of_stat_percent_like_cpp(&self) -> [i32; 5] {
        self.with_aura_view(|view| view.armor_of_stat_percent())
            .unwrap_or_default()
    }

    pub fn represented_total_aura_modifier_like_cpp(&self, aura_type: i32) -> f32 {
        self.with_aura_view(|view| view.total_aura_modifier(aura_type))
            .unwrap_or_default()
    }
}
