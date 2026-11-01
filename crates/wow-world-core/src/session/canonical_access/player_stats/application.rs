//! Apply one owned stat calculation to the canonical Player without I/O.

use super::{PlayerDamagePctOperation, PlayerStatComputation};
use wow_constants::{DeathState, PowerType};
use wow_entities::Player;

/// Select the vital fields affected by a represented stat calculation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerStatApplicationCause {
    AllStats,
    StatAura {
        stat_mask: u8,
        preserve_health_pct: bool,
        damage_pct_operation: PlayerDamagePctOperation,
    },
    DamageModifiers(PlayerDamagePctOperation),
}

impl PlayerStatApplicationCause {
    pub fn damage_pct_operation(self) -> PlayerDamagePctOperation {
        match self {
            PlayerStatApplicationCause::AllStats => PlayerDamagePctOperation::PreserveOffhand,
            PlayerStatApplicationCause::StatAura {
                damage_pct_operation,
                ..
            }
            | PlayerStatApplicationCause::DamageModifiers(damage_pct_operation) => {
                damage_pct_operation
            }
        }
    }
}

/// Apply one already calculated snapshot to the canonical Player.
///
/// This is a synchronous application step with no external access: callers
/// prepare the owned calculation first and keep packet delivery, catalogs and
/// I/O outside it.
/// The field order follows the represented `Player::UpdateAllStats` path;
/// stat auras only recalculate health when they select stamina, while only
/// `AllStats` updates the existing Mana slot. `UpdateStats(INTELLECT)` does
/// not call `UpdateMaxPower` in target `StatSystem.cpp:105-152`.
pub fn apply_player_stat_computation(
    player: &mut Player,
    calculation: &PlayerStatComputation,
    cause: PlayerStatApplicationCause,
) {
    let previous = *player.effective_combat_stats_like_cpp();
    let weapon_damage_modifiers_changed = std::array::from_fn(|index| {
        previous.weapon_damage_pct[index] != calculation.effective.weapon_damage_pct[index]
            || previous.weapon_damage_flat[index] != calculation.effective.weapon_damage_flat[index]
    });
    player.replace_effective_combat_stats_for_values(calculation.effective);
    match cause {
        PlayerStatApplicationCause::StatAura { stat_mask, .. } => {
            player.mark_stat_aura_update_footprint_like_cpp(
                stat_mask,
                weapon_damage_modifiers_changed,
            );
        }
        PlayerStatApplicationCause::DamageModifiers(_) => {
            player.mark_damage_pct_update_footprint_like_cpp(weapon_damage_modifiers_changed);
        }
        PlayerStatApplicationCause::AllStats => {}
    }

    let update_health = match cause {
        PlayerStatApplicationCause::AllStats => true,
        PlayerStatApplicationCause::StatAura { stat_mask, .. } => {
            stat_mask == 0 || stat_mask & (1 << 2) != 0
        }
        PlayerStatApplicationCause::DamageModifiers(_) => false,
    };

    if update_health {
        let unit = player.unit_mut();
        let health_before = unit.data().health;
        let max_health_before = unit.data().max_health.max(1);
        let death_state_before = unit.death_state();
        let preserve_health_pct = matches!(
            cause,
            PlayerStatApplicationCause::StatAura {
                preserve_health_pct: true,
                ..
            }
        );
        let zero_health = if death_state_before == DeathState::Corpse {
            health_before == 0
        } else {
            !unit.is_alive()
        };
        let health_pct =
            preserve_health_pct.then(|| health_before as f32 * 100.0 / max_health_before as f32);
        let max_health = calculation
            .projection
            .max_health
            .max(1)
            .min(i64::from(u32::MAX)) as u64;
        unit.set_max_health(max_health);

        if let Some(health_pct) = health_pct {
            let max_health_after = unit.data().max_health;
            let restored = (max_health_after as f32 * health_pct / 100.0) as u64;
            unit.set_health(restored.max(if zero_health { 0 } else { 1 }));
        }
    }

    if cause == PlayerStatApplicationCause::AllStats
        && player.unit().get_power_index(PowerType::Mana).is_some()
    {
        let max_mana = calculation
            .projection
            .max_mana
            .max(0)
            .min(i64::from(i32::MAX)) as i32;
        player.unit_mut().set_max_power(PowerType::Mana, max_mana);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::PlayerStatComputation;
    use wow_data::PlayerStatSystemProjectionLikeCpp;
    use wow_entities::PlayerEffectiveCombatStatsLikeCpp;

    fn assert_mask_is_exact(actual: &wow_entities::UpdateMask, bits: &[usize]) {
        let mut expected = wow_entities::UpdateMask::new(actual.bits());
        for bit in bits {
            expected.set(*bit);
        }
        assert_eq!(actual.blocks(), expected.blocks());
    }

    fn calculation(max_health: i64, max_mana: i64) -> PlayerStatComputation {
        PlayerStatComputation {
            gear: Default::default(),
            projection: PlayerStatSystemProjectionLikeCpp {
                stats: [1, 2, 3, 4, 5],
                stat_pos_buff: [0; 5],
                stat_neg_buff: [0; 5],
                create_health: 100,
                base_mana: 900,
                max_health,
                max_mana,
                armor: 0,
                attack_power: 0,
                attack_power_mod_pos: 0,
                attack_power_multiplier: 0.0,
                ranged_attack_power: 0,
                ranged_attack_power_mod_pos: 0,
                ranged_attack_power_multiplier: 0.0,
                total_attack_power: 0,
                total_ranged_attack_power: 0,
                block_pct: 0.0,
                dodge_pct: 0.0,
                dodge_from_attr: 0.0,
                parry_pct: 0.0,
                parry_from_attr: 0.0,
                crit_pct: 0.0,
                ranged_crit_pct: 0.0,
                offhand_crit_pct: 0.0,
                spell_crit_pct: [0.0; 7],
                mod_damage_done_pos: [0; 7],
                mod_damage_done_neg: [0; 7],
                mod_healing_done_pos: 0,
                mod_damage_done_percent: [1.0; 7],
                mod_healing_done_percent: 1.0,
                mod_target_resistance: 0,
                mod_target_physical_resistance: 0,
                versatility_bonus: 0.0,
                override_spell_power_by_ap_percent: 0.0,
                override_ap_by_spell_power_percent: 0.0,
                weapon_damage_pct: [1.0; 3],
                weapon_damage_flat: [0.0; 3],
            },
            effective: PlayerEffectiveCombatStatsLikeCpp {
                stats: [1, 2, 3, 4, 5],
                mainhand_expertise: 4.0,
                ..Default::default()
            },
        }
    }

    fn player_with_vitals() -> Player {
        let mut player = Player::new(Some(7), false);
        player.unit_mut().set_max_health(100);
        player.unit_mut().set_health(80);
        player
    }

    #[test]
    fn lifecycle_stat_stamina_aura_clamps_health_without_changing_mana() {
        let mut player = player_with_vitals();
        player.unit_mut().set_power_index(PowerType::Mana, Some(2));
        player.unit_mut().set_max_power(PowerType::Mana, 200);
        player.unit_mut().set_power(PowerType::Mana, 175);
        player.unit_mut().set_create_mana_like_cpp(500);
        player.clear_data_changes();

        apply_player_stat_computation(
            &mut player,
            &calculation(60, 50),
            PlayerStatApplicationCause::StatAura {
                stat_mask: 1 << 2,
                preserve_health_pct: false,
                damage_pct_operation: PlayerDamagePctOperation::PreserveOffhand,
            },
        );

        assert_eq!(player.unit().data().max_health, 60);
        assert_eq!(player.unit().data().health, 60);
        assert_eq!(player.unit().get_max_power(PowerType::Mana), 200);
        assert_eq!(player.unit().get_power(PowerType::Mana), 175);
        assert_eq!(player.unit().get_create_mana_like_cpp(), 500);
        assert_eq!(
            player.effective_combat_stats_like_cpp().stats,
            [1, 2, 3, 4, 5]
        );
        assert!(player.unit().unit_data_changes_mask().is_set(5));
        assert!(player.unit().unit_data_changes_mask().is_set(6));
        assert!(!player.unit().unit_data_changes_mask().is_set(75));
    }

    #[test]
    fn stat_aura_marks_native_update_stats_footprints_when_values_are_equal() {
        let active_common = || {
            let mut bits = vec![0, 38, 59, 269];
            bits.extend(278..=283);
            bits.extend(285..=290);
            bits.sort_unstable();
            bits
        };
        let cases = [
            (1_u8, Some(2_usize)),
            (1 << 1, Some(3)),
            (1 << 3, None),
            (0, None),
        ];

        for (stat_mask, mana_index) in cases {
            let calculation = calculation(100, 1);
            let mut player = player_with_vitals();
            player
                .unit_mut()
                .set_power_index(PowerType::Mana, mana_index);
            player.replace_effective_combat_stats_like_cpp(calculation.effective);
            player.clear_data_changes();

            apply_player_stat_computation(
                &mut player,
                &calculation,
                PlayerStatApplicationCause::StatAura {
                    stat_mask,
                    preserve_health_pct: false,
                    damage_pct_operation: PlayerDamagePctOperation::PreserveOffhand,
                },
            );

            let selected_stats = if stat_mask == 0 { 0b1_1111 } else { stat_mask };
            let mut expected_unit = vec![174];
            for index in 0..5 {
                if selected_stats & (1 << index) != 0 {
                    expected_unit.push(175 + index);
                }
            }
            if let Some(mana_index) = mana_index {
                expected_unit.extend([116, 117 + mana_index, 127 + mana_index]);
            }
            let mut expected_active = active_common();
            if selected_stats & 0b11 != 0 {
                // C++ `UpdateStats(AGILITY)` also recalculates melee attack
                // power (`StatSystem.cpp:141-146`), writing 81/82/84.
                expected_unit.extend([32, 52, 53, 64, 81, 82, 84]);
            }
            if selected_stats & (1 << 1) != 0 {
                expected_unit.extend([85, 86, 88, 91, 92, 190, 191]);
                expected_active.extend([42, 46, 47, 48]);
            }
            if selected_stats & (1 << 3) != 0 {
                expected_active.extend(270..=276);
            }
            expected_unit.sort_unstable();
            expected_unit.dedup();
            expected_active.sort_unstable();
            expected_active.dedup();
            assert_mask_is_exact(player.unit().unit_data_changes_mask(), &expected_unit);
            assert_mask_is_exact(player.active_player_data_changes_mask(), &expected_active);
        }
    }

    #[test]
    fn stat_aura_changed_offhand_modifier_marks_zero_damage_leaves() {
        let mut player = player_with_vitals();
        let mut previous = PlayerEffectiveCombatStatsLikeCpp::default();
        previous.weapon_damage_pct[1] = 0.5;
        previous.weapon_damage[1] = [0.0, 0.0];
        player.replace_effective_combat_stats_like_cpp(previous);
        player.clear_data_changes();

        let mut calculation = calculation(100, 1);
        calculation.effective = previous;
        calculation.effective.weapon_damage_pct[1] = 0.25;
        calculation.effective.weapon_damage[1] = [0.0, 0.0];
        apply_player_stat_computation(
            &mut player,
            &calculation,
            PlayerStatApplicationCause::StatAura {
                stat_mask: 1,
                preserve_health_pct: false,
                damage_pct_operation: PlayerDamagePctOperation::PreserveOffhand,
            },
        );

        assert!(player.unit().unit_data_changes_mask().is_set(54));
        assert!(player.unit().unit_data_changes_mask().is_set(55));
        assert_eq!(
            player.effective_combat_stats_like_cpp().weapon_damage[1],
            [0.0, 0.0]
        );
        assert!(!player.unit().unit_data_changes_mask().is_set(83));
        assert!(!player.unit().unit_data_changes_mask().is_set(87));
    }

    #[test]
    fn damage_only_offhand_modifier_marks_zero_damage_leaves_without_stat_footprint() {
        let mut player = player_with_vitals();
        let mut previous = PlayerEffectiveCombatStatsLikeCpp::default();
        previous.weapon_damage[1] = [0.0, 0.0];
        player.replace_effective_combat_stats_like_cpp(previous);
        player.clear_data_changes();

        let mut calculation = calculation(100, 1);
        calculation.effective = previous;
        calculation.effective.weapon_damage_pct[1] = 0.0;
        apply_player_stat_computation(
            &mut player,
            &calculation,
            PlayerStatApplicationCause::DamageModifiers(PlayerDamagePctOperation::Offhand),
        );

        let mask = player.unit().unit_data_changes_mask();
        assert!(mask.is_set(32));
        assert!(mask.is_set(54));
        assert!(mask.is_set(55));
        for bit in [52, 53, 64, 91, 92, 174, 175] {
            assert!(!mask.is_set(bit), "unexpected primary-stat leaf {bit}");
        }
    }

    #[test]
    fn lifecycle_stat_all_stats_clamps_existing_mana_slot_without_remapping() {
        let mut player = player_with_vitals();
        player.unit_mut().set_power_index(PowerType::Mana, Some(3));
        player.unit_mut().set_display_power(PowerType::Rage);
        player.unit_mut().set_max_power(PowerType::Mana, 200);
        player.unit_mut().set_power(PowerType::Mana, 175);
        player.unit_mut().set_create_mana_like_cpp(500);

        apply_player_stat_computation(
            &mut player,
            &calculation(150, 100),
            PlayerStatApplicationCause::AllStats,
        );

        assert_eq!(player.unit().get_power_index(PowerType::Mana), Some(3));
        assert_eq!(player.unit().data().display_power, PowerType::Rage as u8);
        assert_eq!(player.unit().get_max_power(PowerType::Mana), 100);
        assert_eq!(player.unit().get_power(PowerType::Mana), 100);
        assert_eq!(player.unit().get_create_mana_like_cpp(), 500);
        assert_eq!(player.unit().data().max_health, 150);

        let mut no_mana_index = player_with_vitals();
        no_mana_index
            .unit_mut()
            .set_power_index(PowerType::Mana, None);
        let max_power_before = no_mana_index.unit().data().max_power;
        apply_player_stat_computation(
            &mut no_mana_index,
            &calculation(140, 50),
            PlayerStatApplicationCause::AllStats,
        );
        assert_eq!(no_mana_index.unit().get_power_index(PowerType::Mana), None);
        assert_eq!(no_mana_index.unit().data().max_power, max_power_before);
    }

    #[test]
    fn lifecycle_stat_nonstamina_and_damage_causes_do_not_change_vitals() {
        for cause in [
            PlayerStatApplicationCause::StatAura {
                stat_mask: 1,
                preserve_health_pct: false,
                damage_pct_operation: PlayerDamagePctOperation::PreserveOffhand,
            },
            PlayerStatApplicationCause::DamageModifiers(PlayerDamagePctOperation::AllAttacks),
        ] {
            let mut player = player_with_vitals();
            player.unit_mut().set_power_index(PowerType::Mana, Some(1));
            player.unit_mut().set_max_power(PowerType::Mana, 200);
            apply_player_stat_computation(&mut player, &calculation(10, 20), cause);
            assert_eq!(player.unit().data().max_health, 100);
            assert_eq!(player.unit().data().health, 80);
            assert_eq!(player.unit().get_max_power(PowerType::Mana), 200);
        }
    }

    #[test]
    fn lifecycle_stat_ability_health_percentage_preserves_death_specific_zero() {
        let cases = [
            (DeathState::Alive, 0, 1),
            (DeathState::Corpse, 0, 0),
            (DeathState::Dead, 1, 1),
        ];

        for (death_state, initial_health, expected_health) in cases {
            let mut player = Player::new(Some(7), false);
            player.unit_mut().set_max_health(100);
            player.unit_mut().set_health(initial_health);
            player.unit_mut().set_death_state(death_state);
            if death_state == DeathState::Dead {
                player.unit_mut().set_health(initial_health);
            }

            apply_player_stat_computation(
                &mut player,
                &calculation(200, 1),
                PlayerStatApplicationCause::StatAura {
                    stat_mask: 0,
                    preserve_health_pct: true,
                    damage_pct_operation: PlayerDamagePctOperation::PreserveOffhand,
                },
            );

            assert_eq!(player.unit().data().health, expected_health);
            assert_eq!(player.unit().death_state(), death_state);
            assert_eq!(player.unit().data().max_health, 200);
        }
    }
}
