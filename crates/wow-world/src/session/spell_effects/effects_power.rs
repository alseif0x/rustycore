//! Represented effects that change power, energy and spell charges.
//!
//! Moved out of the Session root under #621. Behaviour is preserved.

use super::*;

impl WorldSession {
    /// C++ `Spell::EffectEnergize` (`SpellEffects.cpp:1488-1530`) /
    /// `Spell::EffectEnergizePct` (`SpellEffects.cpp:1532-1554`).
    ///
    /// The `EffectEnergize` branch applies the level-dependent overrides and the
    /// Runic Mana Injector engineering bonus before `Unit::EnergizeBySpell`,
    /// which interrupts the regeneration of a `PowerTypeFlags::UseRegenInterrupt`
    /// power (`Player::InterruptPowerRegen`, `Unit.cpp:6581-6585`), applies the
    /// power and then forwards `damage / 2` assisting threat with
    /// `ignoreModifiers = true` (`Unit.cpp:6578-6590`).
    ///
    /// Represented boundary: current canonical player target only. The
    /// caster-side level and skill are read from the session Player, so a
    /// non-player caster keeps the unmodified effect amount.
    pub(in crate::session) fn apply_energize_effect_like_cpp(
        &mut self,
        spell_id: i32,
        caster_guid: ObjectGuid,
        damage: i32,
        misc_value: i32,
        target_guid: ObjectGuid,
        percent: bool,
    ) -> bool {
        let Some(player_guid) = self.player_guid() else {
            return false;
        };
        if target_guid != player_guid || self.resolved_player_is_alive_like_cpp() != Some(true) {
            return false;
        }
        if misc_value < 0 || misc_value >= MAX_POWERS as i32 {
            return false;
        }
        let Ok(power_id) = u8::try_from(misc_value) else {
            return false;
        };
        let power = party_member_power_kind_from_u8_like_cpp(power_id);
        let damage = if percent {
            damage
        } else {
            self.energize_caster_scaled_amount_like_cpp(spell_id, caster_guid, damage)
        };
        // C++ `Unit::EnergizeBySpell` (`Unit.cpp:6581-6585`) interrupts the
        // target Player's regeneration before `ModifyPower` for a power whose
        // DB2 entry carries `PowerTypeFlags::UseRegenInterrupt`.
        if self
            .power_type_store_like_cpp()
            .is_some_and(|store| store.uses_regen_interrupt_like_cpp(power as i8))
        {
            self.interrupt_player_power_regen_like_cpp(power, misc_value);
        }

        let outcome = self
            .mutate_canonical_player_like_cpp(|player| {
                let max_power = player.get_max_power(power);
                if max_power <= 0 {
                    return None;
                }
                let requested = if percent {
                    ((i64::from(max_power) * i64::from(damage)) / 100)
                        .clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
                } else {
                    damage
                };
                let current = player.get_power(power);
                let after = current.saturating_add(requested).clamp(0, max_power);
                player.unit_mut().set_power(power, after);
                Some((requested, after - current))
            })
            .flatten();
        let Some((requested, applied)) = outcome else {
            return false;
        };
        // C++ `Unit::EnergizeBySpell` (`Unit.cpp:6586`) forwards `damage / 2`
        // assisting threat before the log, with `ignoreModifiers = true`.
        self.forward_assisting_threat_like_cpp(
            Some(spell_id),
            caster_guid,
            target_guid,
            requested as f32 / 2.0,
            true,
        );
        // C++ `Unit::EnergizeBySpell` (`Unit.cpp:6578-6590`): `gain` is the
        // delta `ModifyPower` actually applied and `OverEnergize` is what the
        // pool could not take.
        self.send_packet(&wow_packet::packets::combat::SpellEnergizeLog {
            target: target_guid,
            caster: caster_guid,
            spell_id,
            power_type: misc_value,
            amount: applied,
            over_energize: requested.saturating_sub(applied),
        });
        true
    }

    /// C++ `Player::InterruptPowerRegen` (`Player.cpp:1831-1840`): reset the
    /// canonical Player's regen interrupt timestamp and fractional power, then
    /// publish `SMSG_INTERRUPT_POWER_REGEN` with the `Powers` value.
    fn interrupt_player_power_regen_like_cpp(&mut self, power: PowerType, power_type: i32) {
        let now_ms = crate::session_rules::game_time_ms_like_cpp();
        let _ = self.mutate_canonical_player_like_cpp(|player| {
            player
                .unit_mut()
                .interrupt_power_regen_like_cpp(power, now_ms);
        });
        self.send_packet(&wow_packet::packets::combat::InterruptPowerRegen { power_type });
    }

    /// C++ `Spell::EffectEnergize`'s caster-scaled amount
    /// (`SpellEffects.cpp:1507-1527`).
    ///
    /// The switch runs before `Unit::EnergizeBySpell`, so it uses the caster's
    /// level and, for the Runic Mana Injector, the caster Player's engineering
    /// skill. `AddPct(damage, 25)` is `damage += int32(damage * 25 / 100.0f)`.
    fn energize_caster_scaled_amount_like_cpp(
        &self,
        spell_id: i32,
        caster_guid: ObjectGuid,
        damage: i32,
    ) -> i32 {
        let caster_is_player = self.player_guid() == Some(caster_guid);
        let caster_level = if caster_is_player {
            self.player_level_like_cpp()
        } else {
            0
        };
        let mut damage = match spell_id {
            // Blood Fury: `damage -= 10 * max(0, min(30, level - 60))`.
            24_571 => damage - 10 * i32::from(caster_level.saturating_sub(60).min(30)),
            // Burst of Energy: `damage -= 4 * max(0, min(15, level - 60))`.
            24_532 => damage - 4 * i32::from(caster_level.saturating_sub(60).min(15)),
            _ => damage,
        };
        // Runic Mana Injector: engineers gain 25% more.
        if spell_id == 67_490
            && caster_is_player
            && self
                .resolved_player_skill_value_like_cpp(wow_entities::SKILL_ENGINEERING_LIKE_CPP)
                .is_some_and(|value| value != 0)
        {
            damage += (damage as f32 * 25.0 / 100.0) as i32;
        }
        damage
    }
    /// C++ `Spell::EffectPowerDrain` / `Spell::EffectPowerBurn`.
    ///
    /// Represented boundary: current canonical player target/caster only.
    /// `EffectPowerDrain` does not restore power on self-drain in C++, and this
    /// represented path has no generic non-self caster yet. `EffectPowerBurn`
    /// applies the drained amount as damage with multiplier 1.0; the exact C++
    /// `SpellEffectInfo::CalcValueMultiplier` and take-power log packet remain
    /// outside this bounded slice.
    pub(in crate::session) fn apply_power_drain_effect_like_cpp(
        &mut self,
        damage: i32,
        misc_value: i32,
        target_guid: ObjectGuid,
        burn_damage: bool,
    ) -> bool {
        let Some(player_guid) = self.player_guid() else {
            return false;
        };
        if target_guid != player_guid
            || self.resolved_player_is_alive_like_cpp() != Some(true)
            || damage < 0
        {
            return false;
        }
        if misc_value < 0 || misc_value >= MAX_POWERS as i32 {
            return false;
        }
        let Ok(power_id) = u8::try_from(misc_value) else {
            return false;
        };
        let power = party_member_power_kind_from_u8_like_cpp(power_id);

        let drained = self
            .mutate_canonical_player_like_cpp(|player| {
                if party_member_power_kind_from_u8_like_cpp(player.unit().data().display_power)
                    != power
                {
                    return 0;
                }
                let current = player.get_power(power).max(0);
                let drain = current.min(damage);
                player.unit_mut().set_power(power, current - drain);
                drain
            })
            .unwrap_or(0);

        if burn_damage && drained > 0 {
            let _ = self.apply_owned_player_damage_like_cpp(
                u32::try_from(drained).unwrap_or(u32::MAX),
                wow_constants::DeathState::Corpse,
            );
            self.sync_player_registry_state_like_cpp();
        }

        drained > 0
    }
    /// C++ `Spell::EffectModifySpellCharges`.
    ///
    /// C++ restores one consumed charge per positive `damage` step on
    /// `effectInfo->MiscValue`. Packet fanout for `SendSetSpellCharges` is
    /// still outside this represented seam.
    pub(in crate::session) fn apply_modify_spell_charges_effect_like_cpp(
        &mut self,
        damage: i32,
        charge_category_id: i32,
        target_guid: ObjectGuid,
    ) -> u32 {
        let Some(player_guid) = self.player_guid() else {
            return 0;
        };
        if target_guid != player_guid || damage <= 0 {
            return 0;
        }
        let Ok(charge_category_id) = u32::try_from(charge_category_id) else {
            return 0;
        };
        if charge_category_id == 0 {
            return 0;
        }

        let restored = self
            .mutate_canonical_player_like_cpp(|player| {
                let history = &mut player.unit_mut().subsystems_mut().spells.history;
                let mut restored = 0;
                for _ in 0..damage {
                    if history.restore_charge(charge_category_id) {
                        restored += 1;
                    }
                }
                restored
            })
            .unwrap_or(0);

        let mut represented_restored = 0;
        for _ in 0..damage {
            if self.restore_represented_character_spell_charge_like_cpp(charge_category_id) {
                represented_restored += 1;
            }
        }

        restored.max(represented_restored)
    }
}
