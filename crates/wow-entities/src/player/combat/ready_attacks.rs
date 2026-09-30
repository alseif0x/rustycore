//! Canonical Player melee readiness and attack-timer transitions.

use super::Player;
use crate::CurrentSpellSlot;
use wow_constants::{UnitState, WeaponAttackType};

impl Player {
    /// One represented C++ `Unit::DoMeleeAttackIfReady` pass.
    ///
    /// Source: `a5f8da2e`, `Unit.cpp:451-464` (timer pause/update), `628-630`
    /// (reset), `2085-2156` (readiness) and `2158-2186` (attack admission).
    /// The caller supplies range, facing, LOS and the existing sibling delay.
    /// It resolves damage and randomness at each callback, before that attack's
    /// timer reset. This operation owns the Player/Unit reads and mutations;
    /// victim resolution, locks and publication stay with the caller.
    ///
    /// `None` means no ready attack was processed. The inner error update is
    /// `None` when mainhand was not processed, `Some(None)` to clear its error,
    /// or `Some(Some(reason))` for its range/facing error. Offhand leaves it alone.
    ///
    /// Preserved represented gaps: production callers still supply LOS as true;
    /// a current melee spell is finished here rather than executing its C++ cast.
    #[allow(clippy::too_many_arguments)]
    pub fn take_ready_melee_attacks<T>(
        &mut self,
        diff_ms: u32,
        in_melee_range: bool,
        facing_target: bool,
        within_los: bool,
        attack_display_delay_ms: u32,
        mut resolve_swing: impl FnMut(WeaponAttackType, [f32; 2], f32) -> T,
    ) -> Option<(Vec<T>, Option<Option<u8>>)> {
        // C++ `Unit::MeleeDamageBonusDone`'s `SPELL_AURA_MOD_AUTOATTACK_DAMAGE`
        // product, written by the owning session and read here by both owners.
        let autoattack_damage_multiplier = self.unit().mod_autoattack_damage_pct_like_cpp();
        // C++ `DoMeleeAttackIfReady` reads the `UnitData` ranges recalculated by
        // `UpdateDamagePhysical`; the Player-owned snapshot is the Rust equivalent.
        let base_weapon_damage = self.weapon_damage_like_cpp(WeaponAttackType::BaseAttack);
        let offhand_weapon_damage = self.weapon_damage_like_cpp(WeaponAttackType::OffAttack);
        // C++ `Unit::DoMeleeAttackIfReady` admits the offhand branch only when
        // `!IsInFeralForm() && haveOffhandWeapon()` (Unit.cpp:2140). Resolve both
        // predicates before borrowing the mutable Unit; dual-wield capability by
        // itself is not an equipped weapon.
        let has_offhand_weapon = self.has_offhand_weapon_for_attack_like_cpp();
        let is_in_feral_form = self.is_in_feral_form_like_cpp();
        let unit = self.unit_mut();
        let spell_pauses_combat_timer = [CurrentSpellSlot::Generic, CurrentSpellSlot::Channeled]
            .into_iter()
            .any(|slot| {
                unit.current_spell(slot)
                    .is_some_and(|spell| spell.delay_combat_timer_during_cast)
            });
        if !spell_pauses_combat_timer {
            unit.update_attack_timers_like_cpp(diff_ms);
        }
        let mut swings = Vec::new();
        let mut processed_ready_attack = false;
        let mut base_attack_error_update = None;
        // C++: Unit::DoMeleeAttackIfReady, Unit.cpp:2087 exits before
        // processing swings unless UNIT_STATE_MELEE_ATTACKING is present.
        if !unit.has_unit_state(UnitState::MELEE_ATTACKING.bits()) {
            return None;
        }
        // C++: Unit::DoMeleeAttackIfReady, Unit.cpp:2090 exits while charging.
        if unit.has_unit_state(UnitState::CHARGING.bits()) {
            return None;
        }
        // C++: Unit::DoMeleeAttackIfReady returns while casting unless
        // the active channeled spell explicitly allows actions.
        if unit.has_unit_state(UnitState::CASTING.bits()) {
            let channeled = unit.current_spell(CurrentSpellSlot::Channeled);
            if !channeled.is_some_and(|spell| spell.allow_actions_during_channel) {
                return None;
            }
        }
        let has_auto_attack_error = !in_melee_range || !facing_target;
        let melee_state_update_allowed =
            within_los && unit.can_attacker_state_update_melee_like_cpp(false);

        if unit.is_attack_ready_like_cpp(WeaponAttackType::BaseAttack) {
            processed_ready_attack = true;
            if has_auto_attack_error {
                base_attack_error_update = Some(Some(if !in_melee_range { 0 } else { 1 }));
                unit.set_attack_timer(WeaponAttackType::BaseAttack, 100);
            } else {
                base_attack_error_update = Some(None);
                if has_offhand_weapon
                    && unit.attack_timer(WeaponAttackType::OffAttack) < attack_display_delay_ms
                {
                    unit.set_attack_timer(WeaponAttackType::OffAttack, attack_display_delay_ms);
                }
                if melee_state_update_allowed {
                    unit.remove_attacking_interrupt_auras_like_cpp();
                    if unit.current_spell(CurrentSpellSlot::Melee).is_some() {
                        let _ = unit.finish_spell(CurrentSpellSlot::Melee);
                    } else {
                        swings.push(resolve_swing(
                            WeaponAttackType::BaseAttack,
                            base_weapon_damage,
                            autoattack_damage_multiplier,
                        ));
                    }
                }
                unit.reset_attack_timer_like_cpp(WeaponAttackType::BaseAttack);
            }
        }

        if !is_in_feral_form
            && has_offhand_weapon
            && unit.is_attack_ready_like_cpp(WeaponAttackType::OffAttack)
        {
            processed_ready_attack = true;
            if has_auto_attack_error {
                unit.set_attack_timer(WeaponAttackType::OffAttack, 100);
            } else {
                if unit.attack_timer(WeaponAttackType::BaseAttack) < attack_display_delay_ms {
                    unit.set_attack_timer(WeaponAttackType::BaseAttack, attack_display_delay_ms);
                }
                if melee_state_update_allowed {
                    unit.remove_attacking_interrupt_auras_like_cpp();
                    swings.push(resolve_swing(
                        WeaponAttackType::OffAttack,
                        offhand_weapon_damage,
                        autoattack_damage_multiplier,
                    ));
                }
                unit.reset_attack_timer_like_cpp(WeaponAttackType::OffAttack);
            }
        }

        processed_ready_attack.then_some((swings, base_attack_error_update))
    }
}

#[cfg(test)]
mod tests;
