//! Spell-state regressions, part 26.
//!
//! Split out of `scenarios_spell_state_12.rs` when that module reached the
//! terminal file limit; the power-drain cases also pin the target-version
//! direct-damage guard in `Unit::SpellDamageBonusTaken`.

use super::*;

#[path = "scenarios_spell_state_26/creature_negative_damage_taken_aura.rs"]
mod creature_negative_damage_taken_aura;
/// C++ `Spell::EffectPowerDrain` logs the drained power even when the pool is
/// empty and still calls `EnergizeBySpell` for a non-self caster
/// (`SpellEffects.cpp:1090-1101`), so both logs carry zeros rather than being
/// skipped.

#[path = "scenarios_spell_state_26/power_drain_core.rs"]
mod power_drain_core;
#[path = "scenarios_spell_state_26/power_drain_damage_taken.rs"]
mod power_drain_damage_taken;
