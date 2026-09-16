// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Canonical Player power regeneration tick.
//!
//! C++ calls `Player::Update` from the map's `ObjectUpdater` phase
//! (`Map.cpp:711`), which runs `m_regenTimer += p_time; RegenerateAll()`
//! (`Player.cpp:1047-1051`) and publishes each changed power through
//! `Unit::SetPower` (`Player.cpp:1813-1820`). RustyCore keeps a session-owned
//! player tick (the same boundary already used for `DoMeleeAttackIfReady`),
//! driven with the canonical word/map tick diff, and owns the single writer
//! for the power transition. The canonical `Player` still retains the
//! `m_regenTimer`/`m_powerFraction` state and the current power value.
//!
//! The already-published `PlayerEffectiveCombatStatsLikeCpp` is the sole
//! source for `PowerRegenFlatModifier`/`PowerRegenInterruptedFlatModifier`;
//! this module does not recompute spirit/MP5/aura inputs.

use super::*;

/// C++ `PowerTypeFlags::UseRegenInterrupt` (`DBCEnums.h:1799`).
const POWER_TYPE_FLAG_USE_REGEN_INTERRUPT_LIKE_CPP: i16 = 0x0002;

impl WorldSession {
    /// One `Player::RegenerateAll`/`Player::Regenerate(POWER_MANA)` step for
    /// the canonical Player, using `diff_ms` as the C++ `m_regenTimer`.
    ///
    /// C++ runs this for every power; only mana is represented today, so the
    /// remaining powers and `RegenerateHealth` remain explicit follow-up work.
    pub(crate) fn tick_player_mana_regeneration_like_cpp(
        &mut self,
        diff_ms: u32,
        power_types: &wow_data::character_progression::PowerTypeStore,
    ) {
        // C++ `Player::Update` guards the regen block with `IsAlive()`; the map
        // loop additionally requires `IsInWorld()`.
        let in_world = self
            .with_owned_player_like_cpp(|player| {
                player.unit().is_alive() && player.unit().world().object().is_in_world()
            })
            .unwrap_or(false);
        if !in_world {
            return;
        }

        // C++ `HasAuraTypeWithValue(SPELL_AURA_PREVENT_REGENERATE_POWER, power)`:
        // the aura effect amount is the `Powers` value it blocks.
        let mana = PowerType::Mana;
        let prevented = self
            .resolved_aura_effects_by_spell_aura_type_like_cpp(
                wow_data::spell::aura_types::SPELL_AURA_PREVENT_REGENERATE_POWER,
            )
            .map(|effects| {
                effects
                    .into_iter()
                    .any(|(_, amount)| amount == i32::from(mana as i8))
            })
            .unwrap_or(false);
        if prevented {
            return;
        }

        let Some(power_entry) = power_types.get_by_power_type_like_cpp(mana as i8) else {
            return;
        };
        let Some(stats) = self.canonical_player_effective_combat_stats_like_cpp() else {
            return;
        };
        let now_ms = crate::session_rules::game_time_ms_like_cpp();

        let outcome = self.with_owned_player_mut_for_power_like_cpp(|player| {
            player
                .unit_mut()
                .accumulate_power_regen_timer_like_cpp(diff_ms);
            let interrupted = player
                .unit()
                .is_power_regen_interrupted_by_mp5_rule_like_cpp(now_ms);
            let outcome = player.unit_mut().regenerate_power_like_cpp(
                mana,
                wow_entities::UnitPowerRegenInputLikeCpp {
                    diff_ms,
                    regen_peace: power_entry.regen_peace,
                    regen_combat: power_entry.regen_combat,
                    min_power: power_entry.min_power,
                    center_power: power_entry.center_power,
                    use_regen_interrupt: power_entry.flags
                        & POWER_TYPE_FLAG_USE_REGEN_INTERRUPT_LIKE_CPP
                        != 0,
                    regen_interrupt_time_ms: power_entry.regen_interrupt_time_ms,
                    power_regen_flat: stats.mana_regen,
                    power_regen_interrupted: stats.mana_regen_combat,
                    interrupted_by_mp5_rule: interrupted,
                    // C++ `sWorld->getRate(RATE_POWER_MANA)`; the config
                    // override is tracked separately in
                    // `cpp-config-keys.tsv` and defaults to 1.0 here.
                    rate: 1.0,
                    now_ms,
                },
            );
            player.unit_mut().finish_power_regen_tick_like_cpp();
            outcome
        });

        if let Some(wow_entities::UnitPowerRegenOutcomeLikeCpp::Applied {
            power: new_power,
            publish: true,
        }) = outcome
        {
            let Some(guid) = self.player_guid() else {
                return;
            };
            self.send_player_power_update_like_cpp(guid, mana, new_power);
        }
    }

    /// Test accessor for the canonical five-second-rule state after a cast.
    #[cfg(test)]
    pub(crate) fn represented_player_mp5_regen_interrupted_like_cpp(&self) -> bool {
        self.with_owned_player_like_cpp(|player| {
            player
                .unit()
                .is_power_regen_interrupted_by_mp5_rule_like_cpp(
                    crate::session_rules::game_time_ms_like_cpp(),
                )
        })
        .unwrap_or(false)
    }
}
