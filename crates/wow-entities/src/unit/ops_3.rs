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
}
