//! Apply one owned stat calculation to the canonical Player without I/O.

use super::PlayerStatComputation;
use wow_constants::{DeathState, PowerType};
use wow_entities::Player;

/// Select the vital fields affected by a represented stat calculation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerStatApplicationCause {
    AllStats,
    StatAura {
        stat_mask: u8,
        preserve_health_pct: bool,
    },
    DamageModifiers,
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
    player.replace_effective_combat_stats_for_values(calculation.effective);

    let update_health = match cause {
        PlayerStatApplicationCause::AllStats => true,
        PlayerStatApplicationCause::StatAura { stat_mask, .. } => {
            stat_mask == 0 || stat_mask & (1 << 2) != 0
        }
        PlayerStatApplicationCause::DamageModifiers => false,
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
            },
            PlayerStatApplicationCause::DamageModifiers,
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
                },
            );

            assert_eq!(player.unit().data().health, expected_health);
            assert_eq!(player.unit().death_state(), death_state);
            assert_eq!(player.unit().data().max_health, 200);
        }
    }
}
