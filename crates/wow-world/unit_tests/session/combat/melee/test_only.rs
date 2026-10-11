// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1363 slice 1: production-dead `melee` entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl WorldSession {
    /// C++ `Unit::MeleeDamageBonusTaken` (`Unit.cpp:1687-1759`) inputs for the
    /// canonical Player's current melee victim: the creature's applied-aura
    /// effects and the attacker's `SPELL_AURA_MOD_IGNORE_TARGET_RESIST` sum for
    /// the physical school. A canonical-player victim keeps `NONE`, like the
    /// other creature-only victim terms.
    pub(in crate::session) fn represented_melee_damage_taken_like_cpp(
        &self,
    ) -> crate::session_rules::RepresentedMeleeDamageTakenLikeCpp {
        use crate::session_rules::RepresentedMeleeDamageTakenLikeCpp;

        let Some(target_guid) = self
            .core
            .canonical_player_snapshot_like_cpp(|player| player.unit().attacking())
        else {
            return RepresentedMeleeDamageTakenLikeCpp::NONE;
        };
        let Some(target_guid) = target_guid else {
            return RepresentedMeleeDamageTakenLikeCpp::NONE;
        };
        let Some(attacker_guid) = self.player_guid() else {
            return RepresentedMeleeDamageTakenLikeCpp::NONE;
        };
        let Some(spell_store) = self.spell_store() else {
            return RepresentedMeleeDamageTakenLikeCpp::NONE;
        };
        let difficulty_id = self.core.current_map_difficulty_id_like_cpp();
        let victim_effects = {
            self.core
                .with_represented_creature_like_cpp(target_guid, |creature| {
                    crate::session_rules::creature_aura_effects_like_cpp(
                        &creature.unit().subsystems().auras.applied_auras,
                        spell_store,
                        difficulty_id,
                        self.catalogs.difficulty_store().map(AsRef::as_ref),
                    )
                })
                .unwrap_or_default()
        };
        let attacker_ignore_resist = crate::session::hub_ref(self)
            .resolved_aura_effects_by_spell_aura_type_like_cpp(
                wow_data::spell::aura_types::SPELL_AURA_MOD_IGNORE_TARGET_RESIST,
            )
            .unwrap_or_default();
        crate::session_rules::melee_damage_taken_flat_pct_like_cpp(
            &victim_effects,
            &attacker_ignore_resist,
            attacker_guid,
            0x01,
        )
    }
    /// C++ `Unit::RollMeleeOutcomeAgainst` (`Unit.cpp:2272-2310`) inputs for the
    /// canonical Player's current melee victim.
    ///
    /// The attacker side comes from the canonical Player snapshot and its live
    /// auras; the victim side is only representable while the victim is a
    /// creature, matching the creature-only armour rule. `canParryOrBlock` is
    /// C++'s `victim->HasInArc(M_PI, attacker)`, resolved from the two world
    /// positions.
    pub(in crate::session) fn represented_melee_outcome_facts_like_cpp(
        &self,
    ) -> (
        crate::session_rules::RepresentedMeleeAttackerFactsLikeCpp,
        crate::session_rules::RepresentedMeleeVictimFactsLikeCpp,
    ) {
        use crate::session_rules::{
            RepresentedMeleeAttackerFactsLikeCpp as AttackerFacts,
            RepresentedMeleeVictimFactsLikeCpp as VictimFacts,
        };

        let Some(target_guid) = self
            .core
            .canonical_player_snapshot_like_cpp(|player| player.unit().attacking())
        else {
            return (AttackerFacts::default(), VictimFacts::default());
        };
        let Some(target_guid) = target_guid else {
            return (AttackerFacts::default(), VictimFacts::default());
        };
        let Some((
            level,
            dual_wielding,
            melee_hit_chance_pct,
            crit_pct,
            offhand_crit_pct,
            mainhand_expertise,
            offhand_expertise,
            attacker_position,
        )) = self.core.canonical_player_snapshot_like_cpp(|player| {
            let stats = player.effective_combat_stats_like_cpp();
            (
                player.level_like_cpp(),
                player.has_offhand_weapon_for_attack_like_cpp()
                    && !player.is_in_feral_form_like_cpp(),
                stats.melee_hit_chance_pct,
                stats.crit_pct,
                stats.offhand_crit_pct,
                stats.mainhand_expertise,
                stats.offhand_expertise,
                player.unit().world().position(),
            )
        })
        else {
            return (AttackerFacts::default(), VictimFacts::default());
        };
        let aura_sum = |aura_type: i32| -> f32 {
            crate::session::hub_ref(self)
                .resolved_aura_effects_by_spell_aura_type_like_cpp(aura_type)
                .unwrap_or_default()
                .into_iter()
                .map(|(_, amount)| amount as f32)
                .sum()
        };
        let attacker = AttackerFacts {
            level,
            is_controlled_by_player: true,
            no_crushing_blows: true,
            dual_wielding,
            crit_damage_multiplier: crate::session::hub_ref(self)
                .resolved_aura_effects_by_spell_aura_type_like_cpp(
                    wow_data::spell::aura_types::SPELL_AURA_MOD_CRIT_DAMAGE_BONUS,
                )
                .unwrap_or_default()
                .into_iter()
                .filter(|(misc_value, _)| misc_value & 0x01 != 0)
                .fold(1.0_f32, |total, (_, amount)| {
                    total * (1.0 + amount as f32 / 100.0)
                }),
            ignores_dual_wield_hit_penalty: crate::session::hub_ref(self)
                .resolved_aura_effects_by_spell_aura_type_like_cpp(
                    wow_data::spell::aura_types::SPELL_AURA_IGNORE_DUAL_WIELD_HIT_PENALTY,
                )
                .is_some_and(|effects| !effects.is_empty()),
            melee_hit_chance_pct,
            hit_chance_aura_pct: aura_sum(wow_data::spell::aura_types::SPELL_AURA_MOD_HIT_CHANCE),
            crit_pct: [crit_pct, offhand_crit_pct],
            autoattack_crit_aura_pct: aura_sum(
                wow_data::spell::aura_types::SPELL_AURA_MOD_AUTOATTACK_CRIT_CHANCE,
            ),
            expertise_reduction_pct: [mainhand_expertise / 4.0, offhand_expertise / 4.0],
            // `GetUnitDodgeChance`'s attacker-side reductions: the
            // `VICTIMSTATE_DODGE` row of `SPELL_AURA_MOD_COMBAT_RESULT_CHANCE`
            // plus every `SPELL_AURA_MOD_ENEMY_DODGE` amount.
            dodge_reduction_pct: crate::session::hub_ref(self)
                .resolved_aura_effects_by_spell_aura_type_like_cpp(
                    wow_data::spell::aura_types::SPELL_AURA_MOD_COMBAT_RESULT_CHANCE,
                )
                .unwrap_or_default()
                .into_iter()
                .filter(|(misc_value, _)| *misc_value == 2)
                .map(|(_, amount)| amount as f32)
                .sum::<f32>()
                + aura_sum(wow_data::spell::aura_types::SPELL_AURA_MOD_ENEMY_DODGE),
        };
        let difficulty_id = self.core.current_map_difficulty_id_like_cpp();
        let victim = {
            self.core
                .with_represented_creature_like_cpp(target_guid, |creature| {
                    let victim_position = creature.position();
                    // The victim's avoidance and attacker-facing aura terms
                    // (`Unit::GetUnitDodgeChance` and friends).
                    let victim_aura_sum = |aura_type: i32| -> f32 {
                        self.spell_store().map_or(0.0, |spell_store| {
                            crate::session_rules::creature_aura_effects_like_cpp(
                                &creature.unit().subsystems().auras.applied_auras,
                                spell_store,
                                difficulty_id,
                                self.catalogs.difficulty_store().map(AsRef::as_ref),
                            )
                            .into_iter()
                            .filter(|effect| effect.aura_type == aura_type)
                            .map(|effect| effect.amount as f32)
                            .sum()
                        })
                    };
                    VictimFacts {
                        level: creature.level(),
                        is_creature: true,
                        is_player: false,
                        is_stand_state: true,
                        is_immune_to_damage: false,
                        is_totem: creature.is_totem_unit_type_like_cpp(),
                        is_evading_attacks: creature.is_evading_attacks_like_cpp(),
                        dodge_pct: creature.avoidance_like_cpp().dodge_pct,
                        parry_pct: creature.avoidance_like_cpp().parry_pct,
                        block_pct: creature.avoidance_like_cpp().block_pct,
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
                            wow_data::spell::aura_types::
                                SPELL_AURA_MOD_ATTACKER_SPELL_AND_WEAPON_CRIT_CHANCE,
                        ),
                        // C++ `GetUnitCriticalChanceTaken`'s conditional terms:
                        // `!HealthBelowPct(MiscValueB)` for the target-health
                        // aura and `GetCasterGUID() == attacker` for the
                        // for-caster aura.
                        crit_chance_vs_target_health_pct: {
                            let health_pct = if creature.max_hp() == 0 {
                                100.0
                            } else {
                                100.0 * creature.current_hp() as f32 / creature.max_hp() as f32
                            };
                            self.spell_store().map_or(0.0, |spell_store| {
                                crate::session_rules::creature_aura_effects_like_cpp(
                                    &creature.unit().subsystems().auras.applied_auras,
                                    spell_store,
                                    difficulty_id,
                                    self.catalogs.difficulty_store().map(AsRef::as_ref),
                                )
                                .into_iter()
                                .filter(|effect| {
                                    effect.aura_type
                                        == wow_data::spell::aura_types::
                                            SPELL_AURA_MOD_CRIT_CHANCE_VERSUS_TARGET_HEALTH
                                        && health_pct >= effect.misc_value_b as f32
                                })
                                .map(|effect| effect.amount as f32)
                                .sum()
                            })
                        },
                        crit_chance_for_caster_pct: {
                            let attacker_guid = self.player_guid();
                            self.spell_store().map_or(0.0, |spell_store| {
                                crate::session_rules::creature_aura_effects_like_cpp(
                                    &creature.unit().subsystems().auras.applied_auras,
                                    spell_store,
                                    difficulty_id,
                                    self.catalogs.difficulty_store().map(AsRef::as_ref),
                                )
                                .into_iter()
                                .filter(|effect| {
                                    effect.aura_type
                                        == wow_data::spell::aura_types::
                                            SPELL_AURA_MOD_CRIT_CHANCE_FOR_CASTER
                                        && Some(effect.caster_guid) == attacker_guid
                                })
                                .map(|effect| effect.amount as f32)
                                .sum()
                            })
                        },
                        faces_attacker: is_unit_facing_target_for_melee_like_cpp(
                            victim_position,
                            attacker_position,
                        ),
                        is_controlled: creature.unit().has_unit_state(
                            wow_constants::unit::UnitState::CONTROLLED.bits(),
                        ),
                    }
                })
        };
        (attacker, victim.unwrap_or_default())
    }
    /// C++ `Unit::CalcArmorReducedDamage` (`Unit.cpp:1623-1685`) inputs for the
    /// canonical Player's current melee victim.
    ///
    /// The victim's armour is only representable while it is a creature: a
    /// canonical-player victim's own snapshot belongs to that player's session,
    /// so it keeps the zero (`NONE`) entry instead of inventing a value. The
    /// attacker's melee damage school has no represented override, so the
    /// represented swing uses C++'s `SPELL_SCHOOL_MASK_NORMAL` default.
    pub(in crate::session) fn represented_melee_armor_mitigation_like_cpp(
        &self,
    ) -> RepresentedArmorMitigationLikeCpp {
        let Some(target_guid) = self
            .core
            .canonical_player_snapshot_like_cpp(|player| player.unit().attacking())
        else {
            return RepresentedArmorMitigationLikeCpp::NONE;
        };
        let Some(target_guid) = target_guid else {
            return RepresentedArmorMitigationLikeCpp::NONE;
        };
        let Some(armor_penetration_pct) = self.core.canonical_player_snapshot_like_cpp(|player| {
            player
                .effective_combat_stats_like_cpp()
                .armor_penetration_pct
        }) else {
            return RepresentedArmorMitigationLikeCpp::NONE;
        };
        let Some(attacker_level) = self
            .core
            .canonical_player_snapshot_like_cpp(|player| player.level_like_cpp())
        else {
            return RepresentedArmorMitigationLikeCpp::NONE;
        };
        let target_resistance_normal_aura = crate::session::hub_ref(self)
            .resolved_aura_effects_by_spell_aura_type_like_cpp(
                wow_data::spell::aura_types::SPELL_AURA_MOD_TARGET_RESISTANCE,
            )
            .unwrap_or_default()
            .into_iter()
            // C++ `SPELL_SCHOOL_MASK_NORMAL` (0x01).
            .filter(|(misc_value, _)| misc_value & 0x01 != 0)
            .map(|(_, amount)| amount)
            .sum::<i32>();
        // C++ applies every `SPELL_AURA_MOD_IGNORE_TARGET_RESIST` effect of the
        // normal school as `std::floor(AddPct(armor, -amount))`.
        let ignore_target_resist_normal_pct = crate::session::hub_ref(self)
            .resolved_aura_effects_by_spell_aura_type_like_cpp(
                wow_data::spell::aura_types::SPELL_AURA_MOD_IGNORE_TARGET_RESIST,
            )
            .unwrap_or_default()
            .into_iter()
            .filter(|(misc_value, _)| misc_value & 0x01 != 0)
            .map(|(_, amount)| amount as f32)
            .sum::<f32>();
        let victim = {
            self.core
                .with_represented_creature_like_cpp(target_guid, |creature| {
                    (creature.combat_log_stats_like_cpp().armor, creature.level())
                })
        };
        let Some((victim_armor, victim_level)) = victim else {
            return RepresentedArmorMitigationLikeCpp::NONE;
        };
        RepresentedArmorMitigationLikeCpp {
            attacker_level,
            victim_level,
            victim_armor,
            armor_penetration_pct,
            target_resistance_normal_aura,
            ignore_target_resist_normal_pct,
            // A creature victim's `SPELL_AURA_BYPASS_ARMOR_FOR_CASTER` needs a
            // represented creature-aura producer; the canonical player is the
            // only owner that resolves it today.
            bypass_armor_pct_by_caster: 0.0,
        }
    }
    /// C++ `Unit::MeleeDamageBonusDone`'s victim-state terms
    /// (`Unit.cpp:7558-7650`) for the canonical Player's current melee victim,
    /// one entry per melee attack type: the creature-type flat/multiplier terms,
    /// the victim's `HasAuraState` versus-aurastate multiplier and the
    /// `HasAuraWithMechanic` target-aura-mechanic multiplier. `NONE` when the
    /// victim, the aura container or the spell store cannot be resolved.
    pub(in crate::session) fn represented_melee_damage_bonus_like_cpp(
        &self,
    ) -> [RepresentedMeleeDamageBonusLikeCpp; 2] {
        let (Some(auras), Some(spell_store)) = (
            crate::session::hub_ref(self).resolved_player_visible_auras_like_cpp(),
            self.spell_store(),
        ) else {
            return [RepresentedMeleeDamageBonusLikeCpp::NONE; 2];
        };
        let Some(target_guid) = self
            .core
            .canonical_player_snapshot_like_cpp(|player| player.unit().attacking())
        else {
            return [RepresentedMeleeDamageBonusLikeCpp::NONE; 2];
        };
        let Some(target_guid) = target_guid else {
            return [RepresentedMeleeDamageBonusLikeCpp::NONE; 2];
        };
        let creature_type_mask = self.represented_target_creature_type_mask_like_cpp(target_guid);
        let victim_aura_state_mask = self.represented_target_aura_state_mask_like_cpp(target_guid);
        let victim_mechanic_mask = self.represented_target_mechanic_mask_like_cpp(target_guid);
        let base_attack_speed = self
            .core
            .canonical_player_snapshot_like_cpp(|player| player.unit().base_attack_speed())
            .unwrap_or([0; 3]);
        std::array::from_fn(|index| {
            let (flat, pct) = crate::session_rules::melee_damage_bonus_done_like_cpp(
                &auras,
                spell_store,
                creature_type_mask,
                victim_aura_state_mask,
                victim_mechanic_mask,
                false,
                crate::session::legacy_attack_power_multiplier_like_cpp(base_attack_speed[index]),
            );
            RepresentedMeleeDamageBonusLikeCpp { flat, pct }
        })
    }
}
