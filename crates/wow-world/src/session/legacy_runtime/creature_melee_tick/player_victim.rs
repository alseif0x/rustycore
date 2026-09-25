//! C++ `Unit::CalculateMeleeDamage` for a player victim in the legacy creature
//! melee tick.
//!
//! Split out of `creature_melee_tick` as behavior-preserving structure: the
//! caller keeps the C++ map-update serialization, the RNG position and the
//! delivery of the presentation resolved here.

use super::absorption::apply_melee_absorb_to_canonical_player_like_cpp;
use super::*;

/// Runs C++ `Unit::CalculateMeleeDamage` for a player victim
/// (`Unit.cpp:1326-1466`): the pre-outcome mitigation, the attack-table roll
/// and the absorb stage, all against the canonical map the caller holds locked
/// in its established canonical -> legacy order.
///
/// Returns the damage `DealMeleeDamage` would commit and fills `swing_state`
/// with the presentation and absorb terms the caller's delivery publishes.
pub(super) fn player_victim_damage_like_cpp(
    canonical_manager: &mut wow_map::MapManager,
    attacker: &crate::map_manager::WorldCreature,
    swing: &PendingCreatureSwingLikeCpp,
    config: &crate::session::LegacyCreatureAggroConfigLikeCpp,
    damage: u32,
    swing_state: &mut MeleeSwingStateLikeCpp,
) -> u32 {
    match config.spell_store.as_deref() {
        Some(spell_store) => {
            let map_difficulty_id = canonical_manager
                .find_map(u32::from(swing.map_id), swing.instance_id)
                .map(|managed| managed.difficulty())
                .unwrap_or(0);
            // C++ `MeleeSpellMissChance`/`GetUnitDodgeChance`/
            // `GetUnitParryChance`/`GetUnitCriticalChanceDone`
            // (`Unit.cpp:11652-11685`, `2639-2786`): the creature's
            // `m_modMeleeHitChance` is zero (`Unit.cpp:360`), its
            // critical chance starts at `5.0` unless the template
            // carries `CREATURE_FLAG_EXTRA_NO_CRIT`, its expertise is
            // `MOD_EXPERTISE / 4` and it holds no offhand swing in this
            // bridge, so the dual-wield penalty never applies.
            let attacker_effects = crate::session_rules::creature_aura_effects_like_cpp(
                &attacker.creature.unit().subsystems().auras.applied_auras,
                spell_store,
                map_difficulty_id,
                config.difficulty_store.as_deref(),
            );
            let aura_sum = |aura_type: i32| -> f32 {
                attacker_effects
                    .iter()
                    .filter(|effect| effect.aura_type == aura_type)
                    .map(|effect| effect.amount as f32)
                    .sum()
            };
            // C++ `CalcArmorReducedDamage` (`Unit.cpp:1640-1651`) reads
            // the attacker's armour-penetration terms for the normal
            // school.
            let attacker_armor_pen = |aura_type: i32| -> f32 {
                attacker_effects
                    .iter()
                    .filter(|effect| effect.aura_type == aura_type && effect.misc_value & 0x01 != 0)
                    .map(|effect| effect.amount as f32)
                    .sum()
            };
            let attacker_target_resistance_normal_aura =
                attacker_armor_pen(wow_data::spell::aura_types::SPELL_AURA_MOD_TARGET_RESISTANCE);
            let attacker_ignore_target_resist_normal_pct = attacker_armor_pen(
                wow_data::spell::aura_types::SPELL_AURA_MOD_IGNORE_TARGET_RESIST,
            );
            // C++ `MeleeDamageBonusTaken`'s Sanctified Wrath bypass reads
            // the same attacker effects as `(MiscValue, amount)` pairs.
            let attacker_ignore_resist: Vec<(i32, i32)> = attacker_effects
                .iter()
                .filter(|effect| {
                    effect.aura_type
                        == wow_data::spell::aura_types::SPELL_AURA_MOD_IGNORE_TARGET_RESIST
                })
                .map(|effect| (effect.misc_value, effect.amount))
                .collect();
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
                // The represented bridge has no creature offhand
                // swing, so `haveOffhandWeapon()` never adds the
                // `19%` dual-wield miss penalty here.
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
            let victim = canonical_manager
                .find_map(u32::from(swing.map_id), swing.instance_id)
                .and_then(|managed| managed.map().get_typed_player(swing.victim_guid))
                .map(|player| {
                    let auras = player.unit().subsystems().auras.runtime_applications_like_cpp();
                    let aura_sum = |aura_type: i32| -> f32 {
                        crate::session_rules::player_aura_effects_by_spell_aura_type_like_cpp(
                            auras, spell_store, aura_type,
                        )
                        .into_iter()
                        .map(|(_, amount)| amount as f32)
                        .sum()
                    };
                    let health_pct = if player.unit().data().max_health == 0 {
                        100.0
                    } else {
                        100.0 * player.unit().data().health as f32
                            / player.unit().data().max_health as f32
                    };
                    let stats = player.effective_combat_stats_like_cpp();
                    // C++ `Unit::GetCreatureTypeMask` uses the
                    // player's race entry (or a shapeshift override).
                    // The runtime currently represents the race row;
                    // an absent row fails closed to the same zero mask
                    // used by the other data-backed projections.
                    let victim_creature_type_mask = config
                        .chr_races_store
                        .as_ref()
                        .and_then(|store| store.get(u32::from(player.race_like_cpp())))
                        .and_then(|race| u32::try_from(race.creature_type).ok())
                        .filter(|creature_type| *creature_type >= 1)
                        .and_then(|creature_type| 1_u32.checked_shl(creature_type - 1))
                        .unwrap_or(0);
                    let victim_aura_state_mask = player
                        .unit()
                        .subsystems()
                        .auras
                        .aura_state_mask
                        | crate::map_manager::WorldCreature::health_aura_state_like_cpp(
                            player.unit().data().health,
                            player.unit().data().max_health,
                            player.unit().is_alive(),
                        );
                    let victim_mechanic_mask =
                        crate::session_rules::aura_application_mechanic_mask_like_cpp(
                            auras,
                            spell_store,
                            config.difficulty_store.as_deref(),
                        );
                    let victim_effects =
                        crate::session_rules::player_aura_effects_all_like_cpp(
                            auras, spell_store,
                        )
                        .into_iter()
                        .map(|effect| effect.as_applied_like_cpp())
                        .collect::<Vec<_>>();
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
                let player_block_percent =
                    crate::session_rules::player_block_percent_like_cpp(
                        stats.shield_block,
                        config.expected_stat_store.as_ref().map_or(
                            // C++'s empty-store `EvaluateExpectedStat`
                            // fallback (`1.0`).
                            1.0,
                            |store| {
                                store.armor_constant_like_cpp(
                                    u32::from(attacker_facts.level),
                                    -2,
                                )
                            },
                        ),
                    );
                    let victim_position = player.unit().world().position();
                    let facts = crate::session_rules::RepresentedMeleeVictimFactsLikeCpp {
                        level: player.level_like_cpp(),
                        is_player: true,
                        // `CalculatePct`/`pct` division: the published
                        // `ParryPercentage` is already zero while
                        // `CanParry()` is false.
                        dodge_pct: stats.dodge_pct,
                        parry_pct: stats.parry_pct,
                        // C++ `GetUnitBlockChance`'s player branch reads
                        // the published `BlockPercentage`.
                        block_pct: stats.block_pct,
                        attacker_melee_hit_chance_pct: aura_sum(
                            wow_data::spell::aura_types::SPELL_AURA_MOD_ATTACKER_MELEE_HIT_CHANCE,
                        ),
                        attacker_melee_crit_chance_pct: aura_sum(
                            wow_data::spell::aura_types::SPELL_AURA_MOD_ATTACKER_MELEE_CRIT_CHANCE,
                        ) + aura_sum(
                            wow_data::spell::aura_types::SPELL_AURA_MOD_ATTACKER_SPELL_AND_WEAPON_CRIT_CHANCE,
                        ),
                        crit_chance_vs_target_health_pct:
                            crate::session_rules::player_aura_effects_full_by_spell_aura_type_like_cpp(
                                auras,
                                spell_store,
                                wow_data::spell::aura_types::SPELL_AURA_MOD_CRIT_CHANCE_VERSUS_TARGET_HEALTH,
                            )
                            .into_iter()
                            // C++ `!HealthBelowPct(MiscValueB)`.
                            .filter(|effect| health_pct >= effect.misc_value_b as f32)
                            .map(|effect| effect.amount as f32)
                            .sum(),
                        crit_chance_for_caster_pct:
                            crate::session_rules::player_aura_effects_full_by_spell_aura_type_like_cpp(
                                auras,
                                spell_store,
                                wow_data::spell::aura_types::SPELL_AURA_MOD_CRIT_CHANCE_FOR_CASTER,
                            )
                            .into_iter()
                            .filter(|effect| effect.caster_guid == swing.attacker_guid)
                            .map(|effect| effect.amount as f32)
                            .sum(),
                        faces_attacker: is_unit_facing_target_for_melee_like_cpp(
                            victim_position,
                            swing.attacker_position,
                        ),
                        is_controlled: player
                            .unit()
                            .has_unit_state(wow_constants::unit::UnitState::CONTROLLED.bits()),
                        is_stand_state: player.unit().is_stand_state_like_cpp(),
                        // C++ `Unit::IsImmunedToDamage` (`Unit.cpp:7318-7336`)
                        // accepts either the school-immunity registry or
                        // the damage-immunity registry when the whole
                        // physical school is covered.
                        is_immune_to_damage: [
                            wow_data::spell::aura_types::SPELL_AURA_SCHOOL_IMMUNITY,
                            wow_data::spell::aura_types::SPELL_AURA_DAMAGE_IMMUNITY,
                        ]
                        .into_iter()
                        .any(|aura_type| {
                            crate::session_rules::player_aura_effects_full_by_spell_aura_type_like_cpp(
                                auras,
                                spell_store,
                                aura_type,
                            )
                            .into_iter()
                            .any(|effect| effect.misc_value & 0x01 != 0)
                        }),
                        ..Default::default()
                    };
                    // C++ `CalcArmorReducedDamage`'s victim side: the
                    // player's published `GetArmor()` and the
                    // `SPELL_AURA_BYPASS_ARMOR_FOR_CASTER` sum for
                    // effects this attacker cast.
                    let bypass_armor_pct_by_caster =
                        crate::session_rules::player_aura_effects_full_by_spell_aura_type_like_cpp(
                            auras,
                            spell_store,
                            wow_data::spell::aura_types::SPELL_AURA_BYPASS_ARMOR_FOR_CASTER,
                        )
                        .into_iter()
                        .filter(|effect| effect.caster_guid == swing.attacker_guid)
                        .map(|effect| effect.amount as f32)
                        .sum::<f32>();
                    // C++ `Unit::MeleeDamageBonusTaken` for a white swing
                    // (`Unit.cpp:7670-7778`) reads the victim's whole
                    // active-aura list across several aura types, so the
                    // projection is unfiltered and re-shaped into the
                    // shared creature-aura form.
                    let taken_effects =
                        crate::session_rules::player_aura_effects_all_like_cpp(
                            auras, spell_store,
                        )
                        .into_iter()
                        .map(|effect| effect.as_applied_like_cpp())
                        .collect::<Vec<_>>();
                    (
                        facts,
                        stats.armor,
                        bypass_armor_pct_by_caster,
                        taken_effects,
                        player_block_percent,
                        creature_attacker_damage_bonus,
                    )
                });
            match victim {
                Some((
                    victim_facts,
                    victim_armor,
                    bypass_armor_pct_by_caster,
                    victim_taken_effects,
                    player_block_percent,
                    creature_attacker_damage_bonus,
                )) => {
                    // C++ `CalculateMeleeDamage` runs
                    // `MeleeDamageBonusTaken` and
                    // `CalcArmorReducedDamage` before the outcome switch
                    // (`Unit.cpp:1326-1343`), in that order.
                    let taken = crate::session_rules::melee_damage_taken_flat_pct_like_cpp(
                        &victim_taken_effects,
                        &attacker_ignore_resist,
                        swing.attacker_guid,
                        // C++ `SPELL_SCHOOL_MASK_NORMAL` (0x01).
                        0x01,
                    );
                    let after_done = crate::session_rules::melee_damage_bonus_done_apply_like_cpp(
                        damage,
                        creature_attacker_damage_bonus,
                    );
                    let after_taken =
                        crate::session_rules::melee_damage_taken_apply_like_cpp(taken, after_done);
                    let mitigated = crate::session_rules::armor_reduced_damage_like_cpp(
                        after_taken,
                        attacker_facts.level,
                        victim_facts.level,
                        victim_armor,
                        // CR_ARMOR_PENETRATION is a player-attacker
                        // rating.
                        0.0,
                        attacker_target_resistance_normal_aura as i32,
                        attacker_ignore_target_resist_normal_pct,
                        bypass_armor_pct_by_caster,
                    );
                    let inputs = crate::session_rules::melee_outcome_inputs_like_cpp(
                        &attacker_facts,
                        &victim_facts,
                    );
                    let rolled = crate::session_rules::rolled_melee_outcome_like_cpp(&inputs[0]);
                    let (damage, _blocked, original) =
                        crate::session_rules::melee_outcome_damage_like_cpp(
                            rolled,
                            mitigated,
                            attacker_facts.level,
                            victim_facts.level,
                            attacker_facts.crit_damage_multiplier,
                            player_block_percent,
                        );
                    let (info, state) =
                        crate::session_rules::melee_outcome_presentation_like_cpp(rolled, false);
                    swing_state.hit_info = info;
                    swing_state.victim_state = state;
                    swing_state.original_damage = original;
                    swing_state.outcome_represented = true;
                    if matches!(
                        rolled,
                        crate::session_rules::RepresentedMeleeOutcomeLikeCpp::Immune
                            | crate::session_rules::RepresentedMeleeOutcomeLikeCpp::Evade
                            | crate::session_rules::RepresentedMeleeOutcomeLikeCpp::Miss
                            | crate::session_rules::RepresentedMeleeOutcomeLikeCpp::Dodge
                            | crate::session_rules::RepresentedMeleeOutcomeLikeCpp::Parry
                    ) {
                        swing_state.avoided_outcome = Some(rolled);
                    }
                    // C++ `Unit::CalculateMeleeDamage`'s absorb stage
                    // (`Unit.cpp:1449-1466`) runs after the outcome
                    // switch and before `DealMeleeDamage`, so the
                    // committed damage and the published `SubDmg`
                    // already carry the reduced amount. Physical melee
                    // always uses `SPELL_SCHOOL_MASK_NORMAL`.
                    let absorb = apply_melee_absorb_to_canonical_player_like_cpp(
                        canonical_manager,
                        u32::from(swing.map_id),
                        swing.instance_id,
                        swing.victim_guid,
                        0x01,
                        damage,
                        spell_store,
                        map_difficulty_id,
                        config.difficulty_store.as_deref(),
                        crate::session_rules::represented_melee_ignore_absorb_like_cpp(
                            &attacker_effects,
                            0x01,
                        ),
                    );
                    let damage = match absorb {
                        Some((absorbed, remaining, spent, consumptions)) => {
                            if absorbed > 0 {
                                swing_state.absorbed_damage = absorbed;
                                swing_state.mana_spent = spent;
                                swing_state.absorb_consumptions = consumptions;
                                swing_state.hit_info |= if remaining == 0 {
                                    wow_packet::packets::combat::HIT_INFO_FULL_ABSORB
                                } else {
                                    wow_packet::packets::combat::HIT_INFO_PARTIAL_ABSORB
                                };
                            }
                            remaining
                        }
                        None => damage,
                    };
                    damage
                }
                None => damage,
            }
        }
        None => damage,
    }
}
