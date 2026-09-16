// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Canonical Player power and health regeneration tick.
//!
//! C++ calls `Player::Update` from the map's `ObjectUpdater` phase
//! (`Map.cpp:711`), which runs `m_regenTimer += p_time; RegenerateAll()`
//! (`Player.cpp:1047-1051`). `Player::RegenerateAll` (`Player.cpp:1609-1670`)
//! accumulates the two-second window, regenerates every represented power and
//! then runs `Player::RegenerateHealth` (`Player.cpp:1842-1882`) while the
//! window is pending. RustyCore keeps a session-owned player tick (the same
//! boundary already used for `DoMeleeAttackIfReady`), driven with the canonical
//! world/map tick diff, and owns the single writer for both transitions.
//!
//! The already-published `PlayerEffectiveCombatStatsLikeCpp` is the sole
//! source for `PowerRegenFlatModifier`/`PowerRegenInterruptedFlatModifier` and
//! `m_baseHealthRegen`; this module does not recompute spirit/MP5/aura inputs.

use super::*;

/// C++ `PowerTypeFlags::UseRegenInterrupt` (`DBCEnums.h:1799`).
const POWER_TYPE_FLAG_USE_REGEN_INTERRUPT_LIKE_CPP: i16 = 0x0002;

/// The aura modifiers C++ `Player::RegenerateHealth` reads for one tick.
struct HealthRegenAuraInputsLikeCpp {
    mod_regen: i32,
    health_regen_percent: f32,
    has_mod_regen_during_combat: bool,
    mod_regen_during_combat: i32,
    has_mod_health_regen_in_combat: bool,
    mod_health_regen_in_combat: i32,
}

/// One represented power prepared for the C++ `RegenerateAll` power loop.
struct RepresentedPowerRegenLikeCpp {
    power: PowerType,
    input: wow_entities::UnitPowerRegenInputLikeCpp,
}

/// Resolve the C++ `Player::RegenerateHealth` aura and game-table inputs from
/// the canonical Player and the published effective-stat snapshot. A missing
/// spell store or visible-aura map is treated as "no auras", matching the
/// fail-open shape already used by the mana-regen prevention gate.
fn resolve_health_regeneration_input_like_cpp(
    session: &WorldSession,
    stats: &wow_entities::PlayerEffectiveCombatStatsLikeCpp,
    regen_game_tables: &wow_data::RegenGameTablesLikeCpp,
) -> Option<wow_entities::UnitHealthRegenInputLikeCpp> {
    let (is_in_combat, is_stand_state) = session.canonical_player_snapshot_like_cpp(|player| {
        (
            player
                .unit()
                .unit_flags_like_cpp()
                .contains(UnitFlags::IN_COMBAT),
            player.unit().is_stand_state_like_cpp(),
        )
    })?;

    let effects_for = |aura_type: i32| {
        session
            .resolved_aura_effects_by_spell_aura_type_like_cpp(aura_type)
            .unwrap_or_default()
    };
    let mod_regen_effects = effects_for(wow_data::spell::aura_types::SPELL_AURA_MOD_REGEN);
    let percent_effects =
        effects_for(wow_data::spell::aura_types::SPELL_AURA_MOD_HEALTH_REGEN_PERCENT);
    let during_combat_effects =
        effects_for(wow_data::spell::aura_types::SPELL_AURA_MOD_REGEN_DURING_COMBAT);
    let in_combat_effects =
        effects_for(wow_data::spell::aura_types::SPELL_AURA_MOD_HEALTH_REGEN_IN_COMBAT);

    // C++ `Player::OCTRegenHPPerSpirit` (`Player.cpp:5162-5180`) reads the
    // level/class row from both HP regen tables and splits Spirit at 50.
    let spirit = stats.stats[4] as f32;
    let oct_regen_hp_per_spirit = regen_game_tables.oct_regen_hp_per_spirit_like_cpp(
        u16::from(session.player_level_like_cpp()),
        session.player_class_like_cpp(),
        spirit,
    );

    Some(wow_entities::UnitHealthRegenInputLikeCpp {
        level: session.player_level_like_cpp(),
        // C++ `sWorld->getRate(RATE_HEALTH)`; the config override is tracked
        // separately in `cpp-config-keys.tsv` and defaults to 1.0 here.
        rate_health: 1.0,
        is_in_combat,
        is_stand_state,
        oct_regen_hp_per_spirit,
        aura_mod_regen: mod_regen_effects.iter().map(|(_, amount)| *amount).sum(),
        aura_health_regen_percent: percent_effects
            .iter()
            .fold(1.0, |acc, (_, amount)| acc * (1.0 + *amount as f32 / 100.0)),
        has_mod_regen_during_combat: !during_combat_effects.is_empty(),
        aura_mod_regen_during_combat: during_combat_effects
            .iter()
            .map(|(_, amount)| *amount)
            .sum(),
        has_mod_health_regen_in_combat: !in_combat_effects.is_empty(),
        aura_mod_health_regen_in_combat: in_combat_effects.iter().map(|(_, amount)| *amount).sum(),
        base_health_regen: stats.health_regen,
        // C++ `Unit::IsPolymorphed()` reads `m_transformSpell`
        // (`Unit.cpp:9993-10000`), which the canonical Unit does not represent
        // yet. The transform-spell owner is a separate port gate.
        is_polymorphed: false,
    })
}

impl WorldSession {
    /// One `Player::RegenerateAll` step for the canonical Player, using
    /// `diff_ms` as the C++ `m_regenTimer`.
    ///
    /// C++ loops every power; only the represented primary power is regenerated
    /// today (the power-index map does not represent alternate powers), so a
    /// non-mana class regenerates its primary power here. The two-second health
    /// window and `RegenerateHealth` run on the same canonical `Unit`.
    pub(crate) fn tick_player_regeneration_like_cpp(
        &mut self,
        diff_ms: u32,
        power_types: &wow_data::character_progression::PowerTypeStore,
        regen_game_tables: Option<&wow_data::RegenGameTablesLikeCpp>,
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
        // the aura effect amount is the `Powers` value it blocks. The check
        // gates only that power; the timer and health window still advance.
        let prevented_powers = self
            .resolved_aura_effects_by_spell_aura_type_like_cpp(
                wow_data::spell::aura_types::SPELL_AURA_PREVENT_REGENERATE_POWER,
            )
            .unwrap_or_default();
        let power_regen_percent_effects = self
            .resolved_aura_effects_by_spell_aura_type_like_cpp(
                wow_data::spell::aura_types::SPELL_AURA_MOD_POWER_REGEN_PERCENT,
            )
            .unwrap_or_default();
        let power_regen_flat_effects = self
            .resolved_aura_effects_by_spell_aura_type_like_cpp(
                wow_data::spell::aura_types::SPELL_AURA_MOD_POWER_REGEN,
            )
            .unwrap_or_default();

        let stats = self.canonical_player_effective_combat_stats_like_cpp();
        let health_input = match (stats.as_ref(), regen_game_tables) {
            (Some(stats), Some(regen_game_tables)) => {
                resolve_health_regeneration_input_like_cpp(self, stats, regen_game_tables)
            }
            _ => None,
        };
        let now_ms = crate::session_rules::game_time_ms_like_cpp();

        // C++ `for (Powers power = POWER_MANA; power < MAX_POWERS; ...)`: every
        // power with a represented index gets one `Regenerate` call. Only the
        // primary power index is represented today, so an alternate power is
        // skipped exactly like a `GetPowerIndex` miss.
        let represented_powers = self
            .with_owned_player_like_cpp(|player| {
                (0..wow_entities::MAX_POWERS as i8)
                    .filter_map(|raw| <PowerType as num_traits::FromPrimitive>::from_i8(raw))
                    .filter(|power| player.unit().get_power_index(*power).is_some())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let mut prepared = Vec::new();
        for power in represented_powers {
            let power_value = i32::from(power as i8);
            if prevented_powers
                .iter()
                .any(|(_, amount)| *amount == power_value)
            {
                continue;
            }
            let Some(power_entry) = power_types.get_by_power_type_like_cpp(power as i8) else {
                continue;
            };
            // C++ `UpdateManaRegen` folds the aura/stat producers into the
            // published mana fields; other powers read the UnitData flat field
            // (zero until the rune producer is ported) and the miss-value auras.
            let (power_regen_flat, power_regen_interrupted) = if power == PowerType::Mana {
                match stats.as_ref() {
                    Some(stats) => (stats.mana_regen, stats.mana_regen_combat),
                    None => continue,
                }
            } else {
                (0.0, 0.0)
            };
            let power_regen_percent_multiplier = power_regen_percent_effects
                .iter()
                .filter(|(misc_value, _)| *misc_value == power_value)
                .fold(1.0, |acc, (_, amount)| acc * (1.0 + *amount as f32 / 100.0));
            let power_regen_flat_aura = power_regen_flat_effects
                .iter()
                .filter(|(misc_value, _)| *misc_value == power_value)
                .map(|(_, amount)| *amount)
                .sum();
            prepared.push(RepresentedPowerRegenLikeCpp {
                power,
                input: wow_entities::UnitPowerRegenInputLikeCpp {
                    diff_ms,
                    regen_peace: power_entry.regen_peace,
                    regen_combat: power_entry.regen_combat,
                    min_power: power_entry.min_power,
                    center_power: power_entry.center_power,
                    use_regen_interrupt: power_entry.flags
                        & POWER_TYPE_FLAG_USE_REGEN_INTERRUPT_LIKE_CPP
                        != 0,
                    regen_interrupt_time_ms: power_entry.regen_interrupt_time_ms,
                    power_regen_flat,
                    power_regen_interrupted,
                    interrupted_by_mp5_rule: false,
                    // C++ `sWorld->getRate(RatesForPower[power])`; the config
                    // override is tracked separately in `cpp-config-keys.tsv`
                    // and defaults to 1.0 here.
                    rate: 1.0,
                    power_regen_percent_multiplier,
                    power_regen_flat_aura,
                    now_ms,
                },
            });
        }

        let published = self.with_owned_player_mut_for_power_like_cpp(|player| {
            // C++ `m_regenTimer += p_time; m_regenTimerCount += m_regenTimer`.
            player
                .unit_mut()
                .accumulate_power_regen_timer_like_cpp(diff_ms);

            let mut published = Vec::new();
            for represented in &prepared {
                let mut input = represented.input;
                input.interrupted_by_mp5_rule = represented.power == PowerType::Mana
                    && player
                        .unit()
                        .is_power_regen_interrupted_by_mp5_rule_like_cpp(now_ms);
                if let wow_entities::UnitPowerRegenOutcomeLikeCpp::Applied {
                    power: new_power,
                    publish: true,
                } = player
                    .unit_mut()
                    .regenerate_power_like_cpp(represented.power, input)
                {
                    published.push((represented.power, new_power));
                }
            }

            // C++ `if (m_regenTimerCount >= 2000)` health branch. The gate is
            // kept explicit so the `RegenerateHealth` call sites match
            // `Player::RegenerateAll`; the inner function would otherwise
            // compute the same no-op.
            if player.unit().power_regen_timer_ready_like_cpp()
                && let Some(input) = health_input
            {
                let passes_gate = !input.is_in_combat
                    || input.is_polymorphed
                    || input.base_health_regen != 0
                    || input.has_mod_regen_during_combat
                    || input.has_mod_health_regen_in_combat;
                if passes_gate {
                    let _ = player.unit_mut().regenerate_health_like_cpp(input);
                }
            }

            player.unit_mut().finish_power_regen_tick_like_cpp();
            published
        });

        let Some(published) = published else {
            return;
        };
        let Some(guid) = self.player_guid() else {
            return;
        };
        for (power, new_power) in published {
            // C++ `Unit::SetPower` sends one `SMSG_POWER_UPDATE` per changed
            // power on the publication boundary.
            self.send_player_power_update_like_cpp(guid, power, new_power);
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
