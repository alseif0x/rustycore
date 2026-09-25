//! C++ `Unit::CalculateMeleeDamage` for a creature victim in the legacy
//! creature melee tick.
//!
//! Split out of `creature_melee_tick` as behavior-preserving structure: the
//! caller keeps the C++ map-update serialization, the RNG position and the
//! delivery of the presentation resolved here.

use super::absorption::apply_melee_absorb_to_canonical_creature_like_cpp;
use super::*;

/// Runs C++ `Unit::CalculateMeleeDamage` for a creature victim
/// (`Unit.cpp:1326-1466`): the pre-outcome mitigation, the attack-table roll
/// and the absorb stage, all against the canonical map the caller holds locked
/// in its established canonical -> legacy order.
///
/// Returns the damage `DealMeleeDamage` would commit and fills `swing_state`
/// with the presentation and absorb terms the caller's delivery publishes.
pub(super) fn creature_victim_damage_like_cpp(
    canonical_manager: &mut wow_map::MapManager,
    attacker: &crate::map_manager::WorldCreature,
    swing: &PendingCreatureSwingLikeCpp,
    config: &crate::session::LegacyCreatureAggroConfigLikeCpp,
    damage: u32,
    swing_state: &mut MeleeSwingStateLikeCpp,
) -> u32 {
    // Creature victim: the same pre-outcome mitigation C++
    // `CalculateMeleeDamage` applies (`Unit.cpp:1326-1343`), resolved
    // from the victim creature's armour and taken auras plus the
    // attacker's normal-school penetration terms. The outcome table and
    // its presentation stay on this branch's compatibility bridge.
    match config.spell_store.as_deref() {
        Some(spell_store) => {
            let map_difficulty_id = canonical_manager
                .find_map(u32::from(swing.map_id), swing.instance_id)
                .map(|managed| managed.difficulty())
                .unwrap_or(0);
            let attacker_effects = crate::session_rules::creature_aura_effects_like_cpp(
                &attacker.creature.unit().subsystems().auras.applied_auras,
                spell_store,
                map_difficulty_id,
                config.difficulty_store.as_deref(),
            );
            let normal_misc_sum = |aura_type: i32| -> f32 {
                attacker_effects
                    .iter()
                    .filter(|effect| effect.aura_type == aura_type && effect.misc_value & 0x01 != 0)
                    .map(|effect| effect.amount as f32)
                    .sum()
            };
            let aura_sum = |aura_type: i32| -> f32 {
                attacker_effects
                    .iter()
                    .filter(|effect| effect.aura_type == aura_type)
                    .map(|effect| effect.amount as f32)
                    .sum()
            };
            let no_crit = wow_constants::CreatureFlagsExtra::from_bits_truncate(
                attacker.creature.lifecycle_metadata().flags_extra,
            )
            .contains(wow_constants::CreatureFlagsExtra::NO_CRIT);
            let creature_crit_pct = if no_crit {
                0.0
            } else {
                5.0 + aura_sum(wow_data::spell::aura_types::SPELL_AURA_MOD_WEAPON_CRIT_PERCENT)
                    + aura_sum(wow_data::spell::aura_types::SPELL_AURA_MOD_CRIT_PCT)
            };
            let expertise_reduction_pct =
                aura_sum(wow_data::spell::aura_types::SPELL_AURA_MOD_EXPERTISE) / 4.0;
            let attacker_facts = crate::session_rules::RepresentedMeleeAttackerFactsLikeCpp {
                level: attacker.creature.level(),
                is_controlled_by_player: attacker
                    .creature
                    .is_charmed_owned_by_player_or_player_like_cpp(),
                no_crushing_blows: wow_constants::CreatureFlagsExtra::from_bits_truncate(
                    attacker.creature.lifecycle_metadata().flags_extra,
                )
                .contains(wow_constants::CreatureFlagsExtra::NO_CRUSHING_BLOWS),
                dual_wielding: false,
                crit_damage_multiplier: attacker_effects
                    .iter()
                    .filter(|effect| {
                        effect.aura_type
                            == wow_data::spell::aura_types::SPELL_AURA_MOD_CRIT_DAMAGE_BONUS
                            && effect.misc_value & 0x01 != 0
                    })
                    .fold(1.0_f32, |total, effect| {
                        total * (1.0 + effect.amount as f32 / 100.0)
                    }),
                ignores_dual_wield_hit_penalty: false,
                melee_hit_chance_pct: 0.0,
                hit_chance_aura_pct: aura_sum(
                    wow_data::spell::aura_types::SPELL_AURA_MOD_HIT_CHANCE,
                ),
                crit_pct: [creature_crit_pct, creature_crit_pct],
                autoattack_crit_aura_pct: aura_sum(
                    wow_data::spell::aura_types::SPELL_AURA_MOD_AUTOATTACK_CRIT_CHANCE,
                ),
                expertise_reduction_pct: [expertise_reduction_pct, expertise_reduction_pct],
                dodge_reduction_pct: attacker_effects
                    .iter()
                    .filter(|effect| {
                        effect.aura_type
                            == wow_data::spell::aura_types::SPELL_AURA_MOD_COMBAT_RESULT_CHANCE
                            && effect.misc_value == 2
                    })
                    .map(|effect| effect.amount as f32)
                    .sum::<f32>()
                    + aura_sum(wow_data::spell::aura_types::SPELL_AURA_MOD_ENEMY_DODGE),
            };
            // C++ `MeleeDamageBonusTaken`'s Sanctified Wrath bypass.
            let attacker_ignore_resist: Vec<(i32, i32)> = attacker_effects
                .iter()
                .filter(|effect| {
                    effect.aura_type
                        == wow_data::spell::aura_types::SPELL_AURA_MOD_IGNORE_TARGET_RESIST
                })
                .map(|effect| (effect.misc_value, effect.amount))
                .collect();
            let victim = canonical_manager
                .find_map(u32::from(swing.map_id), swing.instance_id)
                .and_then(|managed| {
                    managed
                        .map()
                        .with_creature_like_cpp(swing.victim_guid, |victim| {
                            // C++ `RollMeleeOutcomeAgainst`'s
                            // creature-victim facts
                            // (`Unit.cpp:2272-2360`): the
                            // `CreatureAvoidanceLikeCpp` bases, the
                            // victim's percentage and attacker-side
                            // aura sums, the facing/controlled gates and
                            // the health-conditioned critical.
                            let effects =
                                crate::session_rules::creature_aura_effects_like_cpp(
                                    &victim.unit().subsystems().auras.applied_auras,
                                    spell_store,
                                    map_difficulty_id,
                                    config.difficulty_store.as_deref(),
                                );
                            let victim_creature_type_mask = config
                                .creature_template_lifecycle_store
                                .as_ref()
                                .and_then(|store| store.get(victim.entry()))
                                .and_then(|template| {
                                    (template.creature_type >= 1).then(|| {
                                        1_u32.checked_shl(template.creature_type - 1)
                                    })
                                })
                                .flatten()
                                .unwrap_or(0);
                            let victim_aura_state_mask = victim
                                .unit()
                                .subsystems()
                                .auras
                                .aura_state_mask
                                | crate::map_manager::WorldCreature::health_aura_state_like_cpp(
                                    victim.unit().data().health,
                                    victim.unit().data().max_health,
                                    victim.is_alive(),
                                );
                            let victim_mechanic_mask =
                                crate::session_rules::applied_aura_mechanic_mask_like_cpp(
                                    &victim.unit().subsystems().auras.applied_auras,
                                    spell_store,
                                    map_difficulty_id,
                                    config.difficulty_store.as_deref(),
                                );
                            let victim_aura_sum = |aura_type: i32| -> f32 {
                                effects
                                    .iter()
                                    .filter(|effect| effect.aura_type == aura_type)
                                    .map(|effect| effect.amount as f32)
                                    .sum()
                            };
                            let health_pct = if victim.unit().data().max_health == 0 {
                                100.0
                            } else {
                                100.0 * victim.unit().data().health as f32
                                    / victim.unit().data().max_health as f32
                            };
                            let avoidance = victim.avoidance_like_cpp();
                            let facts = crate::session_rules::RepresentedMeleeVictimFactsLikeCpp {
                                level: victim
                                    .unit()
                                    .data()
                                    .level
                                    .clamp(0, i32::from(u8::MAX))
                                    as u8,
                                is_creature: true,
                                is_player: false,
                                is_totem: victim.is_totem_unit_type_like_cpp(),
                                is_evading_attacks: victim.is_evading_attacks_like_cpp(),
                                dodge_pct: avoidance.dodge_pct,
                                parry_pct: avoidance.parry_pct,
                                block_pct: avoidance.block_pct,
                                dodge_aura_pct: victim_aura_sum(
                                    wow_data::spell::aura_types::SPELL_AURA_MOD_DODGE_PERCENT,
                                ),
                                parry_aura_pct: victim_aura_sum(
                                    wow_data::spell::aura_types::SPELL_AURA_MOD_PARRY_PERCENT,
                                ),
                                block_aura_pct: victim_aura_sum(
                                    wow_data::spell::aura_types::SPELL_AURA_MOD_BLOCK_PERCENT,
                                ),
                                attacker_melee_hit_chance_pct: victim_aura_sum(
                                    wow_data::spell::aura_types::SPELL_AURA_MOD_ATTACKER_MELEE_HIT_CHANCE,
                                ),
                                attacker_melee_crit_chance_pct: victim_aura_sum(
                                    wow_data::spell::aura_types::SPELL_AURA_MOD_ATTACKER_MELEE_CRIT_CHANCE,
                                ) + victim_aura_sum(
                                    wow_data::spell::aura_types::SPELL_AURA_MOD_ATTACKER_SPELL_AND_WEAPON_CRIT_CHANCE,
                                ),
                                crit_chance_vs_target_health_pct: effects
                                    .iter()
                                    .filter(|effect| {
                                        effect.aura_type
                                            == wow_data::spell::aura_types::SPELL_AURA_MOD_CRIT_CHANCE_VERSUS_TARGET_HEALTH
                                            && health_pct >= effect.misc_value_b as f32
                                    })
                                    .map(|effect| effect.amount as f32)
                                    .sum(),
                                crit_chance_for_caster_pct: effects
                                    .iter()
                                    .filter(|effect| {
                                        effect.aura_type
                                            == wow_data::spell::aura_types::SPELL_AURA_MOD_CRIT_CHANCE_FOR_CASTER
                                            && effect.caster_guid == swing.attacker_guid
                                    })
                                    .map(|effect| effect.amount as f32)
                                    .sum(),
                                faces_attacker: is_unit_facing_target_for_melee_like_cpp(
                                    victim.unit().world().position(),
                                    swing.attacker_position,
                                ),
                                is_controlled: victim.unit().has_unit_state(
                                    wow_constants::unit::UnitState::CONTROLLED.bits(),
                                ),
                                is_stand_state: true,
                                // C++ `IsImmunedToDamage(NORMAL)`: a
                                // `Unit::IsImmunedToDamage` accepts both
                                // `SPELL_AURA_SCHOOL_IMMUNITY` and
                                // `SPELL_AURA_DAMAGE_IMMUNITY` masks.
                                is_immune_to_damage: effects.iter().any(|effect| {
                                    matches!(
                                        effect.aura_type,
                                        wow_data::spell::aura_types::SPELL_AURA_SCHOOL_IMMUNITY
                                            | wow_data::spell::aura_types::SPELL_AURA_DAMAGE_IMMUNITY,
                                    ) && effect.misc_value & 0x01 != 0
                                }),
                            };
                            (
                                facts,
                                victim.combat_log_stats_like_cpp().armor,
                                effects,
                                victim_creature_type_mask,
                                victim_aura_state_mask,
                                victim_mechanic_mask,
                            )
                        })
                });
            match victim {
                Some((
                    victim_facts,
                    victim_armor,
                    victim_effects,
                    victim_creature_type_mask,
                    victim_aura_state_mask,
                    victim_mechanic_mask,
                )) => {
                    let victim_attack_power_bonus = victim_effects
                        .iter()
                        .filter(|effect| {
                            effect.aura_type
                                == wow_data::spell::aura_types::
                                    SPELL_AURA_MELEE_ATTACK_POWER_ATTACKER_BONUS
                        })
                        .map(|effect| effect.amount)
                        .sum::<i32>();
                    let creature_attacker_damage_bonus =
                        crate::session_rules::melee_damage_bonus_done_from_effects_like_cpp(
                            &attacker_effects,
                            victim_attack_power_bonus,
                            victim_creature_type_mask,
                            victim_aura_state_mask,
                            victim_mechanic_mask,
                            false,
                            crate::session::legacy_attack_power_multiplier_like_cpp(
                                attacker.creature.unit().base_attack_speed()[0],
                            ),
                        );
                    let taken = crate::session_rules::melee_damage_taken_flat_pct_like_cpp(
                        &victim_effects,
                        &attacker_ignore_resist,
                        swing.attacker_guid,
                        // C++ `SPELL_SCHOOL_MASK_NORMAL` (0x01).
                        0x01,
                    );
                    let after_taken = crate::session_rules::melee_damage_taken_apply_like_cpp(
                        taken,
                        crate::session_rules::melee_damage_bonus_done_apply_like_cpp(
                            damage,
                            creature_attacker_damage_bonus,
                        ),
                    );
                    let mitigated = crate::session_rules::armor_reduced_damage_like_cpp(
                        after_taken,
                        attacker.creature.level(),
                        victim_facts.level,
                        victim_armor,
                        // CR_ARMOR_PENETRATION is a player-attacker
                        // rating.
                        0.0,
                        normal_misc_sum(
                            wow_data::spell::aura_types::SPELL_AURA_MOD_TARGET_RESISTANCE,
                        ) as i32,
                        normal_misc_sum(
                            wow_data::spell::aura_types::SPELL_AURA_MOD_IGNORE_TARGET_RESIST,
                        ),
                        // A creature victim's bypass aura needs a
                        // represented creature-aura producer.
                        0.0,
                    );
                    // C++ `Unit::RollMeleeOutcomeAgainst`
                    // (`Unit.cpp:2272-2310`).
                    let inputs = crate::session_rules::melee_outcome_inputs_like_cpp(
                        &attacker_facts,
                        &victim_facts,
                    );
                    let rolled = crate::session_rules::rolled_melee_outcome_like_cpp(&inputs[0]);
                    let (mut info, state) =
                        crate::session_rules::melee_outcome_presentation_like_cpp(rolled, false);
                    let (outcome_damage, blocked, _original) =
                        crate::session_rules::melee_outcome_damage_like_cpp(
                            rolled,
                            mitigated,
                            attacker_facts.level,
                            victim_facts.level,
                            attacker_facts.crit_damage_multiplier,
                            crate::session_rules::CREATURE_BLOCK_PERCENT_LIKE_CPP,
                        );
                    swing_state.creature_victim_avoided = matches!(
                        rolled,
                        crate::session_rules::RepresentedMeleeOutcomeLikeCpp::Immune
                            | crate::session_rules::RepresentedMeleeOutcomeLikeCpp::Evade
                            | crate::session_rules::RepresentedMeleeOutcomeLikeCpp::Miss
                            | crate::session_rules::RepresentedMeleeOutcomeLikeCpp::Dodge
                            | crate::session_rules::RepresentedMeleeOutcomeLikeCpp::Parry
                    );
                    swing_state.outcome_represented = true;
                    let absorb = apply_melee_absorb_to_canonical_creature_like_cpp(
                        canonical_manager,
                        swing.map_id,
                        swing.instance_id,
                        swing.attacker_guid,
                        swing.victim_guid,
                        0x01,
                        outcome_damage,
                        outcome_damage.min(i32::MAX as u32) as i32,
                        spell_store,
                        map_difficulty_id,
                        config.difficulty_store.as_deref(),
                        crate::session_rules::represented_melee_ignore_absorb_like_cpp(
                            &attacker_effects,
                            0x01,
                        ),
                    );
                    let (absorbed, remaining) = absorb
                        .map(|(absorbed, remaining, events)| {
                            swing_state.creature_victim_absorb_events = events;
                            (absorbed, remaining)
                        })
                        .unwrap_or((0, outcome_damage));
                    if absorbed > 0 {
                        swing_state.absorbed_damage = absorbed;
                        info |= if remaining == 0 {
                            wow_packet::packets::combat::HIT_INFO_FULL_ABSORB
                        } else {
                            wow_packet::packets::combat::HIT_INFO_PARTIAL_ABSORB
                        };
                    }
                    swing_state.creature_victim_presentation = Some((info, state, blocked as i32));
                    remaining
                }
                None => damage,
            }
        }
        None => damage,
    }
}
