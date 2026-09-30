//! Creature-victim mitigation/outcome stages; no packet production.
use super::*;
use super::source::CreatureMeleeSource;
use super::absorption::*;
pub(super) fn creature_victim_damage_like_cpp(
    canonical_manager: &mut MapManager,
    source: &CreatureMeleeSource<'_>,
    swing: &PendingCreatureSwingLikeCpp,
    catalogs: &impl CreatureMeleeCatalogsLikeCpp,
    damage: u32,
    swing_state: &mut MeleeSwingStateLikeCpp,
) -> u32 {
    // Creature victim: the same pre-outcome mitigation C++
    // `CalculateMeleeDamage` applies (`Unit.cpp:1326-1343`), resolved
    // from the victim creature's armour and taken auras plus the
    // attacker's normal-school penetration terms. The outcome table and
    // its presentation stay on this branch's compatibility bridge.
    match catalogs.represented() {
        true => {
            let map_difficulty_id = canonical_manager
                .find_map(u32::from(swing.map_id), swing.instance_id)
                .map(|managed| managed.difficulty())
                .unwrap_or(0);
            let attacker_effects = source.effects(canonical_manager, catalogs, map_difficulty_id);
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
                source.flags_extra(canonical_manager),
            )
            .contains(wow_constants::CreatureFlagsExtra::NO_CRIT);
            let creature_crit_pct = if no_crit {
                0.0
            } else {
                5.0 + aura_sum(wow_constants::spell::aura_types::SPELL_AURA_MOD_WEAPON_CRIT_PERCENT)
                    + aura_sum(wow_constants::spell::aura_types::SPELL_AURA_MOD_CRIT_PCT)
            };
            let expertise_reduction_pct =
                aura_sum(wow_constants::spell::aura_types::SPELL_AURA_MOD_EXPERTISE) / 4.0;
            let attacker_facts = RepresentedMeleeAttackerFactsLikeCpp {
                level: source.level(canonical_manager),
                is_controlled_by_player: source.is_player_controlled(canonical_manager),
                no_crushing_blows: wow_constants::CreatureFlagsExtra::from_bits_truncate(
                    source.flags_extra(canonical_manager),
                )
                .contains(wow_constants::CreatureFlagsExtra::NO_CRUSHING_BLOWS),
                dual_wielding: false,
                crit_damage_multiplier: attacker_effects
                    .iter()
                    .filter(|effect| {
                        effect.aura_type
                            == wow_constants::spell::aura_types::SPELL_AURA_MOD_CRIT_DAMAGE_BONUS
                            && effect.misc_value & 0x01 != 0
                    })
                    .fold(1.0_f32, |total, effect| {
                        total * (1.0 + effect.amount as f32 / 100.0)
                    }),
                ignores_dual_wield_hit_penalty: false,
                melee_hit_chance_pct: 0.0,
                hit_chance_aura_pct: aura_sum(
                    wow_constants::spell::aura_types::SPELL_AURA_MOD_HIT_CHANCE,
                ),
                crit_pct: [creature_crit_pct, creature_crit_pct],
                autoattack_crit_aura_pct: aura_sum(
                    wow_constants::spell::aura_types::SPELL_AURA_MOD_AUTOATTACK_CRIT_CHANCE,
                ),
                expertise_reduction_pct: [expertise_reduction_pct, expertise_reduction_pct],
                dodge_reduction_pct: attacker_effects
                    .iter()
                    .filter(|effect| {
                        effect.aura_type
                            == wow_constants::spell::aura_types::SPELL_AURA_MOD_COMBAT_RESULT_CHANCE
                            && effect.misc_value == 2
                    })
                    .map(|effect| effect.amount as f32)
                    .sum::<f32>()
                    + aura_sum(wow_constants::spell::aura_types::SPELL_AURA_MOD_ENEMY_DODGE),
            };
            // C++ `MeleeDamageBonusTaken`'s Sanctified Wrath bypass.
            let attacker_ignore_resist: Vec<(i32, i32)> = attacker_effects
                .iter()
                .filter(|effect| {
                    effect.aura_type
                        == wow_constants::spell::aura_types::SPELL_AURA_MOD_IGNORE_TARGET_RESIST
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
                                catalogs.creature_effects(&victim.unit().subsystems().auras.applied_auras, map_difficulty_id);
                            let victim_creature_type_mask = catalogs.template_creature_type_mask(victim.entry());
                            let victim_aura_state_mask = victim
                                .unit()
                                .subsystems()
                                .auras
                                .aura_state_mask
                                | WorldCreature::health_aura_state_like_cpp(
                                    victim.unit().data().health,
                                    victim.unit().data().max_health,
                                    victim.is_alive(),
                                );
                            let victim_mechanic_mask =
                                catalogs.creature_mechanic_mask(&victim.unit().subsystems().auras.applied_auras, map_difficulty_id);
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
                            let facts = RepresentedMeleeVictimFactsLikeCpp {
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
                                    wow_constants::spell::aura_types::SPELL_AURA_MOD_DODGE_PERCENT,
                                ),
                                parry_aura_pct: victim_aura_sum(
                                    wow_constants::spell::aura_types::SPELL_AURA_MOD_PARRY_PERCENT,
                                ),
                                block_aura_pct: victim_aura_sum(
                                    wow_constants::spell::aura_types::SPELL_AURA_MOD_BLOCK_PERCENT,
                                ),
                                attacker_melee_hit_chance_pct: victim_aura_sum(
                                    wow_constants::spell::aura_types::SPELL_AURA_MOD_ATTACKER_MELEE_HIT_CHANCE,
                                ),
                                attacker_melee_crit_chance_pct: victim_aura_sum(
                                    wow_constants::spell::aura_types::SPELL_AURA_MOD_ATTACKER_MELEE_CRIT_CHANCE,
                                ) + victim_aura_sum(
                                    wow_constants::spell::aura_types::SPELL_AURA_MOD_ATTACKER_SPELL_AND_WEAPON_CRIT_CHANCE,
                                ),
                                crit_chance_vs_target_health_pct: effects
                                    .iter()
                                    .filter(|effect| {
                                        effect.aura_type
                                            == wow_constants::spell::aura_types::SPELL_AURA_MOD_CRIT_CHANCE_VERSUS_TARGET_HEALTH
                                            && health_pct >= effect.misc_value_b as f32
                                    })
                                    .map(|effect| effect.amount as f32)
                                    .sum(),
                                crit_chance_for_caster_pct: effects
                                    .iter()
                                    .filter(|effect| {
                                        effect.aura_type
                                            == wow_constants::spell::aura_types::SPELL_AURA_MOD_CRIT_CHANCE_FOR_CASTER
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
                                        wow_constants::spell::aura_types::SPELL_AURA_SCHOOL_IMMUNITY
                                            | wow_constants::spell::aura_types::SPELL_AURA_DAMAGE_IMMUNITY,
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
                                == wow_constants::spell::aura_types::
                                    SPELL_AURA_MELEE_ATTACK_POWER_ATTACKER_BONUS
                        })
                        .map(|effect| effect.amount)
                        .sum::<i32>();
                    let creature_attacker_damage_bonus =
                        melee_damage_bonus_done_from_effects_like_cpp(
                            &attacker_effects,
                            victim_attack_power_bonus,
                            victim_creature_type_mask,
                            victim_aura_state_mask,
                            victim_mechanic_mask,
                            false,
                            attack_power_multiplier(source.base_attack_speed(canonical_manager)),
                        );
                    let taken = melee_damage_taken_flat_pct_like_cpp(
                        &victim_effects,
                        &attacker_ignore_resist,
                        swing.attacker_guid,
                        // C++ `SPELL_SCHOOL_MASK_NORMAL` (0x01).
                        0x01,
                    );
                    let after_taken = melee_damage_taken_apply_like_cpp(
                        taken,
                        melee_damage_bonus_done_apply_like_cpp(
                            damage,
                            creature_attacker_damage_bonus,
                        ),
                    );
                    let mitigated = armor_reduced_damage_like_cpp(
                        after_taken,
                        source.level(canonical_manager),
                        victim_facts.level,
                        victim_armor,
                        // CR_ARMOR_PENETRATION is a player-attacker
                        // rating.
                        0.0,
                        normal_misc_sum(
                            wow_constants::spell::aura_types::SPELL_AURA_MOD_TARGET_RESISTANCE,
                        ) as i32,
                        normal_misc_sum(
                            wow_constants::spell::aura_types::SPELL_AURA_MOD_IGNORE_TARGET_RESIST,
                        ),
                        // The existing creature-victim branch supplies zero
                        // for this bypass term; extraction adds no projection.
                        0.0,
                    );
                    // C++ `Unit::RollMeleeOutcomeAgainst`
                    // (`Unit.cpp:2272-2310`).
                    let inputs = melee_outcome_inputs_like_cpp(
                        &attacker_facts,
                        &victim_facts,
                    );
                    let rolled = rolled_melee_outcome_like_cpp(&inputs[0]);
                    let mut info = MeleePresentation { outcome: Some(rolled), ..Default::default() };
                    let (outcome_damage, blocked, _original) =
                        melee_outcome_damage_like_cpp(
                            rolled,
                            mitigated,
                            attacker_facts.level,
                            victim_facts.level,
                            attacker_facts.crit_damage_multiplier,
                            CREATURE_BLOCK_PERCENT_LIKE_CPP,
                        );
                    swing_state.creature_victim_avoided = matches!(
                        rolled,
                        RepresentedMeleeOutcomeLikeCpp::Immune
                            | RepresentedMeleeOutcomeLikeCpp::Evade
                            | RepresentedMeleeOutcomeLikeCpp::Miss
                            | RepresentedMeleeOutcomeLikeCpp::Dodge
                            | RepresentedMeleeOutcomeLikeCpp::Parry
                    );
                    swing_state.outcome_represented = true;
                    let absorb = apply_melee_absorb_to_canonical_creature_like_cpp(canonical_manager, swing.map_id, swing.instance_id, swing.attacker_guid, swing.victim_guid, 0x01, outcome_damage, outcome_damage.min(i32::MAX as u32) as i32, catalogs, map_difficulty_id, represented_melee_ignore_absorb_like_cpp(
                            &attacker_effects,
                            0x01,
                        ));
                    let (absorbed, remaining) = absorb
                        .map(|(absorbed, remaining, events)| {
                            swing_state.creature_victim_absorb_events = events;
                            (absorbed, remaining)
                        })
                        .unwrap_or((0, outcome_damage));
                    if absorbed > 0 {
                        swing_state.absorbed_damage = absorbed;
                        info.add_absorb(remaining);
                    }
                    swing_state.creature_victim_presentation = Some((info, blocked as i32));
                    remaining
                }
                None => damage,
            }
        }
        false => damage,
    }
}
