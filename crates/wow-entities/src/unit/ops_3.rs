//! Unit values, visibility and health-revision state operations, part 3 of 3.
//!
//! The inherent `Unit` impl is divided by responsibility under
//! #636; every method keeps its original body.

use super::*;

impl Unit {
    pub(super) fn set_f32_field(
        &mut self,
        bit: usize,
        value: f32,
        field: impl FnOnce(&mut UnitDataValues) -> &mut f32,
    ) {
        let target = field(&mut self.data);
        if *target != value {
            *target = value;
            self.mark_unit_data(bit);
        }
    }
    pub(super) fn mark_unit_data(&mut self, bit: usize) {
        self.unit_data_changes.set(UNIT_DATA_PARENT_BIT);
        self.unit_data_changes.set(bit);
        // C++ generated UpdateField setters call AddToObjectUpdateIfNeeded on
        // the owning Unit. Keep the canonical map queue in sync whenever a
        // live UnitData value changes; the map's SendObjectUpdates phase then
        // snapshots and clears this mask exactly once.
        self.world_mut()
            .object_mut()
            .add_to_object_update_if_needed();
    }
    pub(super) fn mark_unit_data_array(
        &mut self,
        parent_bit: usize,
        first_element_bit: usize,
        index: usize,
    ) {
        self.unit_data_changes.set(parent_bit);
        self.unit_data_changes.set(first_element_bit + index);
        self.world_mut()
            .object_mut()
            .add_to_object_update_if_needed();
    }
    pub(super) fn mark_unit_data_nested(&mut self, parent_bit: usize, bit: usize) {
        self.unit_data_changes.set(parent_bit);
        self.unit_data_changes.set(bit);
        self.world_mut()
            .object_mut()
            .add_to_object_update_if_needed();
    }
}

// ── Power regeneration (C++ `Player::Regenerate`) ────────────────
//
// C++ keeps the accumulators on `Player` (`m_regenTimer`/`m_regenTimerCount`/
// `m_powerFraction`) and the MP5 interrupt mark on `Unit`
// (`_regenMP5InterruptStartTime`); both are represented by
// `UnitPowerRegenStateLikeCpp` on the canonical `Unit`. The world layer
// resolves the DB2 `PowerTypeEntry` scalars and calls this operation.

/// The non-`Unit` inputs C++ `Player::Regenerate` reads for one power.
///
/// The DB2 `PowerTypeEntry` fields (`RegenPeace`, `RegenCombat`, `MinPower`,
/// `CenterPower`, `Flags`, `RegenInterruptTimeMS`) and the canonical
/// `PowerRegenFlatModifier`/`PowerRegenInterruptedFlatModifier` values are
/// supplied by the world layer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UnitPowerRegenInputLikeCpp {
    /// C++ `m_regenTimer` for this call.
    pub diff_ms: u32,
    pub regen_peace: f32,
    pub regen_combat: f32,
    pub min_power: i32,
    pub center_power: i32,
    pub use_regen_interrupt: bool,
    pub regen_interrupt_time_ms: i32,
    /// C++ `m_unitData->PowerRegenFlatModifier[powerIndex]`.
    pub power_regen_flat: f32,
    /// C++ `m_unitData->PowerRegenInterruptedFlatModifier[powerIndex]`.
    pub power_regen_interrupted: f32,
    /// C++ `IsPowerRegenInterruptedByMP5Rule()`.
    pub interrupted_by_mp5_rule: bool,
    /// C++ `sWorld->getRate(RATE_POWER_MANA)` family multiplier.
    pub rate: f32,
    /// C++ `GetTotalAuraMultiplierByMiscValue(SPELL_AURA_MOD_POWER_REGEN_PERCENT, power)`
    /// for the non-mana powers.
    pub power_regen_percent_multiplier: f32,
    /// C++ `GetTotalAuraModifierByMiscValue(SPELL_AURA_MOD_POWER_REGEN, power)`
    /// for the non-mana powers.
    pub power_regen_flat_aura: i32,
    pub now_ms: u32,
}

/// What one `Player::Regenerate` call did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnitPowerRegenOutcomeLikeCpp {
    /// No represented power index, no change, or an early C++ return.
    NotApplicable,
    /// Power changed. `publish` distinguishes C++ `SetPower` (`true`) from the
    /// throttled `ClearChanged` write (`false`).
    Applied { power: i32, publish: bool },
}

impl Unit {
    /// C++ `Unit::IsPowerRegenInterruptedByMP5Rule`
    /// (`Unit.cpp:7779-7782`): true while the last mana-consuming cast is less
    /// than five seconds old.
    #[must_use]
    pub fn is_power_regen_interrupted_by_mp5_rule_like_cpp(&self, now_ms: u32) -> bool {
        now_ms.wrapping_sub(self.power_regen.regen_mp5_interrupt_start_ms) < 5_000
    }

    /// C++ `Unit::SetMP5RegenerationInterruptTime`, called by
    /// `Spell::TakePower` after the caster pays a mana cost.
    pub fn set_mp5_regeneration_interrupt_start_like_cpp(&mut self, now_ms: u32) {
        self.power_regen.regen_mp5_interrupt_start_ms = now_ms;
    }

    /// C++ `Player::InterruptPowerRegen`, sent by `Unit::EnergizeBySpell` for
    /// powers flagged `UseRegenInterrupt`. The world layer owns the
    /// `SMSG_INTERRUPT_POWER_REGEN` packet that accompanies this state change.
    pub fn interrupt_power_regen_like_cpp(&mut self, power: PowerType, now_ms: u32) {
        let Some(index) = self.get_power_index(power) else {
            return;
        };
        self.power_regen.regen_interrupt_timestamp_ms = now_ms;
        self.power_regen.power_fraction[index] = 0.0;
    }

    /// C++ `Player::RegenerateAll` accumulator update for one tick:
    /// `m_regenTimer += p_time; m_regenTimerCount += m_regenTimer`.
    pub fn accumulate_power_regen_timer_like_cpp(&mut self, diff_ms: u32) {
        self.power_regen.timer_ms = self.power_regen.timer_ms.saturating_add(diff_ms);
        self.power_regen.timer_count_ms = self
            .power_regen
            .timer_count_ms
            .saturating_add(self.power_regen.timer_ms);
    }

    /// C++ `Player::RegenerateAll` health gate reads the accumulated
    /// two-second window before it subtracts one window.
    #[must_use]
    pub fn power_regen_timer_ready_like_cpp(&self) -> bool {
        self.power_regen.timer_count_ms >= 2_000
    }

    /// C++ `Player::RegenerateAll` tail for the mana power: after the power
    /// loop C++ subtracts one two-second window and resets `m_regenTimer`.
    pub fn finish_power_regen_tick_like_cpp(&mut self) {
        if self.power_regen.timer_count_ms >= 2_000 {
            self.power_regen.timer_count_ms -= 2_000;
        }
        self.power_regen.timer_ms = 0;
    }

    /// C++ `Player::Regenerate(Powers power)` for one represented power.
    ///
    /// The caller owns the `IsAlive`, `SPELL_AURA_PREVENT_REGENERATE_POWER`
    /// and DB2 lookup gates and supplies their results in `input`. This method
    /// owns the arithmetic, the fraction carry and the publication decision so
    /// the canonical `Unit` stays the single authority for the transition.
    pub fn regenerate_power_like_cpp(
        &mut self,
        power: PowerType,
        input: UnitPowerRegenInputLikeCpp,
    ) -> UnitPowerRegenOutcomeLikeCpp {
        let Some(index) = self.get_power_index(power) else {
            return UnitPowerRegenOutcomeLikeCpp::NotApplicable;
        };

        // C++ `PowerTypeFlags::UseRegenInterrupt` early return.
        if input.use_regen_interrupt {
            let interrupt_ms =
                u32::try_from(input.regen_interrupt_time_ms.max(0)).unwrap_or(u32::MAX);
            if self
                .power_regen
                .regen_interrupt_timestamp_ms
                .saturating_add(interrupt_ms)
                >= input.now_ms
            {
                return UnitPowerRegenOutcomeLikeCpp::NotApplicable;
            }
        }

        let mut cur_value = self.get_power(power);
        let mut add_value = if power == PowerType::Mana && input.interrupted_by_mp5_rule {
            (input.regen_combat + input.power_regen_interrupted) * 0.001 * input.diff_ms as f32
        } else {
            (input.regen_peace + input.power_regen_flat) * 0.001 * input.diff_ms as f32
        };
        add_value *= input.rate;

        // C++ `if (power != POWER_MANA)` aura producers. Mana regen is
        // calculated in `Player::UpdateManaRegen` and already folded into the
        // published `PowerRegenFlatModifier`/`PowerRegenInterruptedFlatModifier`.
        if power != PowerType::Mana {
            add_value *= input.power_regen_percent_multiplier;
            // C++ `(power != POWER_ENERGY) ? m_regenTimerCount : m_regenTimer`.
            let flat_timer_ms = if power != PowerType::Energy {
                self.power_regen.timer_count_ms
            } else {
                self.power_regen.timer_ms
            };
            add_value += input.power_regen_flat_aura as f32 * flat_timer_ms as f32 / (5.0 * 1000.0);
        }

        let mut min_power = input.min_power;
        let mut max_power = self.get_max_power(power);

        // C++ `if (powerType->CenterPower)` branch.
        if input.center_power != 0 {
            if cur_value > input.center_power {
                add_value = -add_value.abs();
                min_power = input.center_power;
            } else if cur_value < input.center_power {
                add_value = add_value.abs();
                max_power = input.center_power;
            } else {
                return UnitPowerRegenOutcomeLikeCpp::NotApplicable;
            }
        }

        add_value += self.power_regen.power_fraction[index];
        let integer_value = add_value.abs() as i32;

        if add_value < 0.0 {
            if cur_value <= min_power {
                return UnitPowerRegenOutcomeLikeCpp::NotApplicable;
            }
        } else if add_value > 0.0 {
            if cur_value >= max_power {
                return UnitPowerRegenOutcomeLikeCpp::NotApplicable;
            }
        } else {
            return UnitPowerRegenOutcomeLikeCpp::NotApplicable;
        }

        let mut forces_set_power = false;
        if add_value < 0.0 {
            if cur_value > min_power + integer_value {
                cur_value -= integer_value;
                self.power_regen.power_fraction[index] = add_value + integer_value as f32;
            } else {
                cur_value = min_power;
                self.power_regen.power_fraction[index] = 0.0;
                forces_set_power = true;
            }
        } else if cur_value + integer_value <= max_power {
            cur_value += integer_value;
            self.power_regen.power_fraction[index] = add_value - integer_value as f32;
        } else {
            cur_value = max_power;
            self.power_regen.power_fraction[index] = 0.0;
            forces_set_power = true;
        }

        // C++ `if (m_regenTimerCount >= 2000 || forcesSetPower) SetPower(...)
        // else` suppressed `SetUpdateFieldValue` + `ClearChanged`.
        let publish = self.power_regen.timer_count_ms >= 2_000 || forces_set_power;
        if publish {
            self.set_power(power, cur_value);
        } else {
            self.set_power_suppressing_object_update_like_cpp(power, cur_value);
        }

        UnitPowerRegenOutcomeLikeCpp::Applied {
            power: cur_value,
            publish,
        }
    }
}

// ── Health regeneration (C++ `Player::RegenerateHealth`) ─────────
//
// C++ runs this from the `Player::RegenerateAll` two-second window
// (`Player.cpp:1643-1647`) and applies the result through `Unit::ModifyHealth`
// (`Unit.cpp:8115-8155`). The canonical `Unit` owns the health transition; the
// world layer resolves the DB2/game-table and aura inputs and supplies them.
// The polymorph branch depends on C++ `Unit::m_transformSpell`
// (`Unit.cpp:9993-10000`), which Rust does not represent yet; callers pass
// `is_polymorphed: false` until that transform state is ported.

/// The non-`Unit` inputs C++ `Player::RegenerateHealth` reads once.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UnitHealthRegenInputLikeCpp {
    pub level: u8,
    /// C++ `sWorld->getRate(RATE_HEALTH)`.
    pub rate_health: f32,
    /// C++ `Unit::IsInCombat()` / `HasUnitFlag(UNIT_FLAG_IN_COMBAT)`.
    pub is_in_combat: bool,
    /// C++ `Unit::IsStandState()`.
    pub is_stand_state: bool,
    /// C++ `Player::OCTRegenHPPerSpirit()`.
    pub oct_regen_hp_per_spirit: f32,
    /// Sum of `SPELL_AURA_MOD_REGEN` effects.
    pub aura_mod_regen: i32,
    /// `GetTotalAuraMultiplier(SPELL_AURA_MOD_HEALTH_REGEN_PERCENT)`.
    pub aura_health_regen_percent: f32,
    /// `HasAuraType(SPELL_AURA_MOD_REGEN_DURING_COMBAT)`.
    pub has_mod_regen_during_combat: bool,
    /// Sum of `SPELL_AURA_MOD_REGEN_DURING_COMBAT` effects.
    pub aura_mod_regen_during_combat: i32,
    /// `HasAuraType(SPELL_AURA_MOD_HEALTH_REGEN_IN_COMBAT)`, which is a
    /// presence check in the `RegenerateAll` gate even when the sum is zero.
    pub has_mod_health_regen_in_combat: bool,
    /// Sum of `SPELL_AURA_MOD_HEALTH_REGEN_IN_COMBAT` effects.
    pub aura_mod_health_regen_in_combat: i32,
    /// C++ `Player::m_baseHealthRegen`.
    pub base_health_regen: i32,
    /// C++ `Unit::IsPolymorphed()`. Always false until the transform spell is
    /// represented on the canonical `Unit`.
    pub is_polymorphed: bool,
}

impl Unit {
    /// C++ `Player::RegenerateHealth` (`Player.cpp:1842-1882`).
    ///
    /// Returns the health gain reported by `Unit::ModifyHealth`. A full-health
    /// or non-positive result performs no write.
    pub fn regenerate_health_like_cpp(&mut self, input: UnitHealthRegenInputLikeCpp) -> i64 {
        let cur_value = self.data.health;
        let max_value = self.data.max_health;
        if cur_value >= max_value {
            return 0;
        }

        let mut health_increase_rate = input.rate_health;
        if input.level < 15 {
            health_increase_rate = input.rate_health * (2.066 - f32::from(input.level) * 0.066);
        }

        let mut add_value = 0.0f32;
        if input.is_polymorphed {
            add_value = max_value as f32 / 3.0;
        } else if !input.is_in_combat || input.has_mod_regen_during_combat {
            add_value = input.oct_regen_hp_per_spirit * health_increase_rate;

            if !input.is_in_combat {
                add_value *= input.aura_health_regen_percent;
                add_value += input.aura_mod_regen as f32 * 0.4;
            } else if input.has_mod_regen_during_combat {
                // C++ `ApplyPct` (`Util.h:90-93`) = `base * pct / 100`.
                add_value = add_value * input.aura_mod_regen_during_combat as f32 / 100.0;
            }

            if !input.is_stand_state {
                add_value *= 1.5;
            }
        }

        add_value += input.aura_mod_health_regen_in_combat as f32;
        add_value += input.base_health_regen as f32 / 2.5;

        if add_value < 0.0 {
            add_value = 0.0;
        }

        // C++ `ModifyHealth(int32(addValue))` truncates toward zero.
        self.modify_health_like_cpp(add_value as i64)
    }

    /// C++ `Unit::ModifyHealth` (`Unit.cpp:8115-8155`). Positive changes rely
    /// on the `UnitData::Health` field; C++ only sends the explicit
    /// `SMSG_HEALTH_UPDATE` on a negative delta.
    pub fn modify_health_like_cpp(&mut self, delta: i64) -> i64 {
        if delta == 0 {
            return 0;
        }

        let cur_health = self.data.health.min(i64::MAX as u64) as i64;
        let value = delta + cur_health;
        if value <= 0 {
            self.set_health(0);
            return -cur_health;
        }

        let max_health = self.data.max_health.min(i64::MAX as u64) as i64;
        let gain;
        if value < max_health {
            self.set_health(value as u64);
            gain = value - cur_health;
        } else if cur_health != max_health {
            self.set_health(max_health as u64);
            gain = max_health - cur_health;
        } else {
            gain = 0;
        }

        gain
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mana_unit(current: i32, max: i32) -> Unit {
        let mut unit = Unit::new(true);
        unit.set_power_index(PowerType::Mana, Some(0));
        unit.set_max_power(PowerType::Mana, max);
        unit.set_power(PowerType::Mana, current);
        // Fixture setup marks the power field; start from a clean mask so the
        // assertions observe only the regeneration write.
        unit.clear_unit_data_changes();
        unit
    }

    fn mana_input(power_regen_flat: f32, diff_ms: u32) -> UnitPowerRegenInputLikeCpp {
        UnitPowerRegenInputLikeCpp {
            diff_ms,
            regen_peace: 0.0,
            regen_combat: 0.0,
            min_power: 0,
            center_power: 0,
            use_regen_interrupt: false,
            regen_interrupt_time_ms: 0,
            power_regen_flat,
            power_regen_interrupted: 0.0,
            interrupted_by_mp5_rule: false,
            rate: 1.0,
            power_regen_percent_multiplier: 1.0,
            power_regen_flat_aura: 0,
            now_ms: 0,
        }
    }

    #[test]
    fn mana_regeneration_suppresses_publication_before_two_seconds_like_cpp() {
        let mut unit = mana_unit(100, 1_000);
        unit.accumulate_power_regen_timer_like_cpp(1_000);

        let outcome = unit.regenerate_power_like_cpp(PowerType::Mana, mana_input(10.0, 1_000));
        unit.finish_power_regen_tick_like_cpp();

        assert_eq!(
            outcome,
            UnitPowerRegenOutcomeLikeCpp::Applied {
                power: 110,
                publish: false
            }
        );
        assert_eq!(unit.get_power(PowerType::Mana), 110);
        assert!(
            !unit
                .unit_data_changes_mask()
                .is_set(UNIT_DATA_POWER_FIRST_BIT),
            "throttled write must clear the power changed bit"
        );
    }

    #[test]
    fn mana_regeneration_publishes_and_resets_count_at_two_seconds_like_cpp() {
        let mut unit = mana_unit(100, 1_000);
        unit.accumulate_power_regen_timer_like_cpp(2_000);

        let outcome = unit.regenerate_power_like_cpp(PowerType::Mana, mana_input(10.0, 2_000));

        assert_eq!(
            outcome,
            UnitPowerRegenOutcomeLikeCpp::Applied {
                power: 120,
                publish: true
            }
        );
        assert!(
            unit.unit_data_changes_mask()
                .is_set(UNIT_DATA_POWER_FIRST_BIT)
        );
        unit.finish_power_regen_tick_like_cpp();
        assert_eq!(unit.power_regen.timer_count_ms, 0);
        assert_eq!(unit.power_regen.timer_ms, 0);
    }

    #[test]
    fn mana_regeneration_carries_the_fraction_between_ticks_like_cpp() {
        let mut unit = mana_unit(100, 1_000);

        unit.accumulate_power_regen_timer_like_cpp(1_000);
        let first = unit.regenerate_power_like_cpp(PowerType::Mana, mana_input(0.5, 1_000));
        unit.finish_power_regen_tick_like_cpp();
        assert_eq!(
            first,
            UnitPowerRegenOutcomeLikeCpp::Applied {
                power: 100,
                publish: false
            }
        );
        assert!((unit.power_regen.power_fraction[0] - 0.5).abs() < 0.0001);

        unit.accumulate_power_regen_timer_like_cpp(1_000);
        let second = unit.regenerate_power_like_cpp(PowerType::Mana, mana_input(0.5, 1_000));
        unit.finish_power_regen_tick_like_cpp();
        assert_eq!(
            second,
            UnitPowerRegenOutcomeLikeCpp::Applied {
                power: 101,
                publish: true
            }
        );
        assert!(unit.power_regen.power_fraction[0].abs() < 0.0001);
    }

    #[test]
    fn mana_regeneration_clamps_at_max_and_forces_publication_like_cpp() {
        let mut unit = mana_unit(999, 1_000);
        unit.accumulate_power_regen_timer_like_cpp(1_000);

        let outcome = unit.regenerate_power_like_cpp(PowerType::Mana, mana_input(10.0, 1_000));

        assert_eq!(
            outcome,
            UnitPowerRegenOutcomeLikeCpp::Applied {
                power: 1_000,
                publish: true
            }
        );
        assert!(
            unit.unit_data_changes_mask()
                .is_set(UNIT_DATA_POWER_FIRST_BIT)
        );
    }

    #[test]
    fn mana_regeneration_uses_interrupted_flat_under_the_mp5_rule_like_cpp() {
        let mut unit = mana_unit(100, 1_000);
        unit.set_mp5_regeneration_interrupt_start_like_cpp(1_000);
        assert!(unit.is_power_regen_interrupted_by_mp5_rule_like_cpp(5_999));
        assert!(!unit.is_power_regen_interrupted_by_mp5_rule_like_cpp(6_000));

        unit.accumulate_power_regen_timer_like_cpp(1_000);
        let mut input = mana_input(999.0, 1_000);
        input.power_regen_interrupted = 7.0;
        input.interrupted_by_mp5_rule = true;
        let outcome = unit.regenerate_power_like_cpp(PowerType::Mana, input);
        unit.finish_power_regen_tick_like_cpp();

        assert_eq!(
            outcome,
            UnitPowerRegenOutcomeLikeCpp::Applied {
                power: 107,
                publish: false
            }
        );
    }

    #[test]
    fn mana_regeneration_returns_early_for_use_regen_interrupt_like_cpp() {
        let mut unit = mana_unit(100, 1_000);
        unit.interrupt_power_regen_like_cpp(PowerType::Mana, 1_000);
        unit.accumulate_power_regen_timer_like_cpp(1_000);

        let mut input = mana_input(10.0, 1_000);
        input.use_regen_interrupt = true;
        input.regen_interrupt_time_ms = 5_000;
        input.now_ms = 3_000;

        assert_eq!(
            unit.regenerate_power_like_cpp(PowerType::Mana, input),
            UnitPowerRegenOutcomeLikeCpp::NotApplicable
        );
        assert_eq!(unit.get_power(PowerType::Mana), 100);
    }

    #[test]
    fn suppressed_power_write_keeps_the_value_without_marking_the_field_like_cpp() {
        let mut unit = mana_unit(100, 1_000);
        unit.set_power_suppressing_object_update_like_cpp(PowerType::Mana, 150);

        assert_eq!(unit.get_power(PowerType::Mana), 150);
        assert!(
            !unit
                .unit_data_changes_mask()
                .is_set(UNIT_DATA_POWER_FIRST_BIT)
        );
    }

    fn health_unit(current: u32, max: u32) -> Unit {
        let mut unit = Unit::new(true);
        unit.set_max_health(u64::from(max));
        unit.set_health(u64::from(current));
        unit
    }

    fn health_input(spirit_regen: f32) -> UnitHealthRegenInputLikeCpp {
        UnitHealthRegenInputLikeCpp {
            level: 80,
            rate_health: 1.0,
            is_in_combat: false,
            is_stand_state: true,
            oct_regen_hp_per_spirit: spirit_regen,
            aura_mod_regen: 0,
            aura_health_regen_percent: 1.0,
            has_mod_regen_during_combat: false,
            aura_mod_regen_during_combat: 0,
            has_mod_health_regen_in_combat: false,
            aura_mod_health_regen_in_combat: 0,
            base_health_regen: 0,
            is_polymorphed: false,
        }
    }

    #[test]
    fn health_regeneration_uses_spirit_rate_and_clamps_at_max_like_cpp() {
        let mut unit = health_unit(100, 1_000);
        assert_eq!(unit.regenerate_health_like_cpp(health_input(10.0)), 10);
        assert_eq!(unit.data().health, 110);
        assert!(unit.unit_data_changes_mask().is_set(UNIT_DATA_HEALTH_BIT));

        assert_eq!(unit.regenerate_health_like_cpp(health_input(999.0)), 890);
        assert_eq!(unit.data().health, 1_000);
    }

    #[test]
    fn health_regeneration_uses_the_under_15_level_rate_like_cpp() {
        let mut unit = health_unit(100, 1_000);
        let mut input = health_input(10.0);
        input.level = 10;
        input.rate_health = 2.0;

        // C++ `2.0 * (2.066 - 10 * 0.066) = 2.812`, truncated to 28.
        assert_eq!(unit.regenerate_health_like_cpp(input), 28);
        assert_eq!(unit.data().health, 128);
    }

    #[test]
    fn health_regeneration_applies_percent_and_flat_regen_like_cpp() {
        let mut unit = health_unit(100, 1_000);
        let mut input = health_input(10.0);
        input.aura_health_regen_percent = 1.5;
        input.aura_mod_regen = 10;

        // `10 * 1.5 + 10 * 0.4 = 19`.
        assert_eq!(unit.regenerate_health_like_cpp(input), 19);
        assert_eq!(unit.data().health, 119);
    }

    #[test]
    fn health_regeneration_multiplies_non_standing_state_like_cpp() {
        let mut unit = health_unit(100, 1_000);
        let mut input = health_input(10.0);
        input.is_stand_state = false;

        assert_eq!(unit.regenerate_health_like_cpp(input), 15);
        assert_eq!(unit.data().health, 115);
    }

    #[test]
    fn health_regeneration_in_combat_needs_the_during_combat_aura_like_cpp() {
        let mut unit = health_unit(100, 1_000);
        let mut suppressed = health_input(10.0);
        suppressed.is_in_combat = true;
        assert_eq!(unit.regenerate_health_like_cpp(suppressed), 0);
        assert_eq!(unit.data().health, 100);

        let mut during_combat = health_input(10.0);
        during_combat.is_in_combat = true;
        during_combat.has_mod_regen_during_combat = true;
        during_combat.aura_mod_regen_during_combat = 50;
        // C++ `ApplyPct(10, 50) = 5`.
        assert_eq!(unit.regenerate_health_like_cpp(during_combat), 5);
        assert_eq!(unit.data().health, 105);
    }

    #[test]
    fn health_regeneration_adds_always_on_modifiers_like_cpp() {
        let mut unit = health_unit(100, 1_000);
        let mut input = health_input(10.0);
        input.is_in_combat = true;
        input.has_mod_regen_during_combat = true;
        input.aura_mod_regen_during_combat = 100;
        input.aura_mod_health_regen_in_combat = 3;
        input.base_health_regen = 5;

        // `10 * 100 / 100 + 3 + 5 / 2.5 = 15`.
        assert_eq!(unit.regenerate_health_like_cpp(input), 15);
        assert_eq!(unit.data().health, 115);
    }

    #[test]
    fn health_regeneration_does_nothing_at_full_health_like_cpp() {
        let mut unit = health_unit(1_000, 1_000);
        assert_eq!(unit.regenerate_health_like_cpp(health_input(999.0)), 0);
        assert_eq!(unit.data().health, 1_000);
    }

    fn power_unit(power: PowerType, current: i32, max: i32) -> Unit {
        let mut unit = Unit::new(true);
        unit.set_power_index(power, Some(0));
        unit.set_max_power(power, max);
        unit.set_power(power, current);
        unit.clear_unit_data_changes();
        unit
    }

    fn non_mana_input(regen_peace: f32, diff_ms: u32) -> UnitPowerRegenInputLikeCpp {
        UnitPowerRegenInputLikeCpp {
            diff_ms,
            regen_peace,
            regen_combat: 0.0,
            min_power: 0,
            center_power: 0,
            use_regen_interrupt: false,
            regen_interrupt_time_ms: 0,
            power_regen_flat: 0.0,
            power_regen_interrupted: 0.0,
            interrupted_by_mp5_rule: false,
            rate: 1.0,
            power_regen_percent_multiplier: 1.0,
            power_regen_flat_aura: 0,
            now_ms: 0,
        }
    }

    #[test]
    fn non_mana_regeneration_applies_percent_and_flat_aura_like_cpp() {
        let mut unit = power_unit(PowerType::Energy, 50, 100);
        unit.accumulate_power_regen_timer_like_cpp(1_000);
        let mut input = non_mana_input(10.0, 1_000);
        input.power_regen_percent_multiplier = 1.5;
        input.power_regen_flat_aura = 5;

        // C++ `10 * 1.5 + 5 * m_regenTimer / 5000 = 15 + 1` for Energy.
        assert_eq!(
            unit.regenerate_power_like_cpp(PowerType::Energy, input),
            UnitPowerRegenOutcomeLikeCpp::Applied {
                power: 66,
                publish: false
            }
        );
        assert_eq!(unit.get_power(PowerType::Energy), 66);
    }

    #[test]
    fn non_mana_regeneration_uses_the_energy_timer_for_the_flat_aura_like_cpp() {
        let mut unit = power_unit(PowerType::Energy, 50, 100);
        unit.accumulate_power_regen_timer_like_cpp(1_000);
        unit.accumulate_power_regen_timer_like_cpp(1_000);

        let mut input = non_mana_input(0.0, 2_000);
        input.power_regen_flat_aura = 5;
        // C++ Energy uses `m_regenTimer` (2000): `5 * 2000 / 5000 = 2`.
        assert_eq!(
            unit.regenerate_power_like_cpp(PowerType::Energy, input),
            UnitPowerRegenOutcomeLikeCpp::Applied {
                power: 52,
                publish: true
            }
        );
    }

    #[test]
    fn non_mana_regeneration_uses_the_accumulated_window_for_other_powers_like_cpp() {
        let mut unit = power_unit(PowerType::Rage, 50, 100);
        unit.accumulate_power_regen_timer_like_cpp(1_000);
        unit.accumulate_power_regen_timer_like_cpp(1_000);

        let mut input = non_mana_input(0.0, 2_000);
        input.power_regen_flat_aura = 5;
        // C++ non-Energy uses `m_regenTimerCount` (3000): `5 * 3000 / 5000 = 3`.
        assert_eq!(
            unit.regenerate_power_like_cpp(PowerType::Rage, input),
            UnitPowerRegenOutcomeLikeCpp::Applied {
                power: 53,
                publish: true
            }
        );
    }

    #[test]
    fn mana_regeneration_ignores_the_non_mana_aura_producers_like_cpp() {
        let mut unit = mana_unit(100, 1_000);
        unit.accumulate_power_regen_timer_like_cpp(1_000);
        let mut input = mana_input(10.0, 1_000);
        input.power_regen_percent_multiplier = 99.0;
        input.power_regen_flat_aura = 99;

        assert_eq!(
            unit.regenerate_power_like_cpp(PowerType::Mana, input),
            UnitPowerRegenOutcomeLikeCpp::Applied {
                power: 110,
                publish: false
            }
        );
    }
}
