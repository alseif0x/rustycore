//! Represented effects that change power, energy and spell charges.
//!
//! Moved out of the Session root under #621. Behaviour is preserved.

use super::*;

impl WorldSession {
    /// C++ `Spell::EffectEnergize` / `Spell::EffectEnergizePct`.
    ///
    /// Represented boundary: current canonical player target only. C++ spell-id
    /// special cases in `EffectEnergize` (Blood Fury, Burst of Energy, Runic
    /// Mana Injector engineering bonus) remain outside this bounded slice.
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
