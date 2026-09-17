//! Represented melee attacks and swing timers.
//!
//! Moved out of the Session root under #617. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

/// One resolved white swing: the damage after every C++
/// `Unit::CalculateMeleeDamage` stage the represented model implements, plus the
/// hit outcome `SMSG_ATTACKERSTATEUPDATE` publishes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::session) struct RepresentedMeleeSwingLikeCpp {
    pub damage: u32,
    /// C++ `CalcDamageInfo::Blocked`.
    pub blocked: u32,
    /// C++ `CalcDamageInfo::HitInfo`.
    pub hit_info: u32,
    /// C++ `CalcDamageInfo::TargetState`.
    pub victim_state: u8,
}

impl RepresentedMeleeSwingLikeCpp {
    /// A plain landed hit, for the paths that resolve their own damage without
    /// the represented attack table (the creature-owned swing).
    pub(in crate::session) fn hit_like_cpp(damage: u32) -> Self {
        let (hit_info, victim_state) = crate::session_rules::melee_outcome_presentation_like_cpp(
            crate::session_rules::RepresentedMeleeOutcomeLikeCpp::Hit,
            false,
        );
        Self {
            damage,
            blocked: 0,
            hit_info,
            victim_state,
        }
    }
}

/// C++ `Unit::CalcArmorReducedDamage` inputs the swing owner resolves once per
/// victim. `NONE` means "no represented armour": every field is zero, so the
/// reduction is zero and the swing damage is unchanged.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(in crate::session) struct RepresentedArmorMitigationLikeCpp {
    pub attacker_level: u8,
    pub victim_level: u8,
    pub victim_armor: i32,
    pub armor_penetration_pct: f32,
    pub target_resistance_normal_aura: i32,
}

impl RepresentedArmorMitigationLikeCpp {
    pub(in crate::session) const NONE: Self = Self {
        attacker_level: 0,
        victim_level: 0,
        victim_armor: 0,
        armor_penetration_pct: 0.0,
        target_resistance_normal_aura: 0,
    };
}

impl WorldSession {
    pub(in crate::session) fn canonical_player_attack_state_like_cpp(
        &self,
    ) -> Option<Option<ObjectGuid>> {
        let guid = self.player_guid?;
        let map_id = u32::from(self.player_map_id_like_cpp());
        let manager = Arc::clone(self.canonical_map_manager.as_ref()?);
        let manager = manager.lock().ok()?;
        let mut result = None;
        manager.do_for_all_maps_with_map_id(map_id, |managed| {
            if result.is_none()
                && let Some(player) = managed.map().get_typed_player(guid)
            {
                result = Some(player.unit().attacking());
            }
        });
        result
    }
    pub(crate) fn set_player_attack_swing_error_like_cpp(&mut self, error: Option<u8>) {
        use wow_packet::ServerPacket;
        use wow_packet::packets::combat::AttackSwingError;

        let Some(publish) = self.mutate_canonical_player_like_cpp(|player| {
            player.set_attack_swing_error_like_cpp(error)
        }) else {
            return;
        };
        if publish {
            if let Some(reason) = error {
                let _ = self.send_tx().send(AttackSwingError { reason }.to_bytes());
            }
        }
    }
    pub(in crate::session) fn take_canonical_player_attack_swings_like_cpp(
        &mut self,
        diff_ms: u32,
        in_melee_range: bool,
        facing_target: bool,
        within_los: bool,
    ) -> Option<(Vec<RepresentedMeleeSwingLikeCpp>, Option<Option<u8>>)> {
        // C++ `CalculateMeleeDamage` resolves the victim-dependent terms per
        // swing; the session computes them for the victim the canonical Player
        // is attacking and hands them to the shared swing function. All of them
        // are hoisted: resolving them inside the mutable owner borrow would
        // re-enter the canonical manager lock.
        let melee_damage_bonus = self.represented_melee_damage_bonus_like_cpp();
        let armor_mitigation = self.represented_melee_armor_mitigation_like_cpp();
        let outcome_facts = self.represented_melee_outcome_facts_like_cpp();
        let damage_taken = self.represented_melee_damage_taken_like_cpp();
        self.mutate_canonical_player_like_cpp(|player| {
            take_canonical_player_attack_swings_like_cpp(
                player,
                diff_ms,
                in_melee_range,
                facing_target,
                within_los,
                melee_damage_bonus,
                armor_mitigation,
                outcome_facts,
                damage_taken,
            )
        })
        .flatten()
    }

    /// C++ `Unit::MeleeDamageBonusTaken` (`Unit.cpp:1687-1759`) inputs for the
    /// canonical Player's current melee victim: the creature's applied-aura
    /// effects and the attacker's `SPELL_AURA_MOD_IGNORE_TARGET_RESIST` sum for
    /// the physical school. A canonical-player victim keeps `NONE`, like the
    /// other creature-only victim terms.
    pub(in crate::session) fn represented_melee_damage_taken_like_cpp(
        &self,
    ) -> crate::session_rules::RepresentedMeleeDamageTakenLikeCpp {
        use crate::session_rules::RepresentedMeleeDamageTakenLikeCpp;

        let Some(target_guid) =
            self.canonical_player_snapshot_like_cpp(|player| player.unit().attacking())
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
        let Some(manager) = self.map_manager.as_ref() else {
            return RepresentedMeleeDamageTakenLikeCpp::NONE;
        };
        let instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        let victim_effects = {
            let manager = manager
                .read()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            manager
                .find_creature(self.player_map_id_like_cpp(), instance_id, target_guid)
                .map(|creature| {
                    crate::session_rules::creature_aura_effects_like_cpp(
                        &creature.creature.unit().subsystems().auras.applied_auras,
                        spell_store,
                        self.current_map_difficulty_id_like_cpp(),
                        self.difficulty_store().map(AsRef::as_ref),
                    )
                })
                .unwrap_or_default()
        };
        let attacker_ignore_resist = self
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

        let Some(target_guid) =
            self.canonical_player_snapshot_like_cpp(|player| player.unit().attacking())
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
        )) = self.canonical_player_snapshot_like_cpp(|player| {
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
            self.resolved_aura_effects_by_spell_aura_type_like_cpp(aura_type)
                .unwrap_or_default()
                .into_iter()
                .map(|(_, amount)| amount as f32)
                .sum()
        };
        let attacker = AttackerFacts {
            level,
            dual_wielding,
            ignores_dual_wield_hit_penalty: self
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
            dodge_reduction_pct: self
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
        let Some(manager) = self.map_manager.as_ref() else {
            return (attacker, VictimFacts::default());
        };
        let instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        let victim = {
            let manager = manager
                .read()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            manager
                .find_creature(self.player_map_id_like_cpp(), instance_id, target_guid)
                .map(|creature| {
                    let victim_position = creature.position();
                    // The victim's avoidance and attacker-facing aura terms
                    // (`Unit::GetUnitDodgeChance` and friends).
                    let victim_aura_sum = |aura_type: i32| -> f32 {
                        self.spell_store().map_or(0.0, |spell_store| {
                            crate::session_rules::creature_aura_effects_like_cpp(
                                &creature.creature.unit().subsystems().auras.applied_auras,
                                spell_store,
                                self.current_map_difficulty_id_like_cpp(),
                                self.difficulty_store().map(AsRef::as_ref),
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
                        is_totem: creature.creature.is_totem_unit_type_like_cpp(),
                        is_evading_attacks: creature.creature.is_evading_attacks_like_cpp(),
                        dodge_pct: creature.creature.avoidance_like_cpp().dodge_pct,
                        parry_pct: creature.creature.avoidance_like_cpp().parry_pct,
                        block_pct: creature.creature.avoidance_like_cpp().block_pct,
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
                                    &creature.creature.unit().subsystems().auras.applied_auras,
                                    spell_store,
                                    self.current_map_difficulty_id_like_cpp(),
                                    self.difficulty_store().map(AsRef::as_ref),
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
                                    &creature.creature.unit().subsystems().auras.applied_auras,
                                    spell_store,
                                    self.current_map_difficulty_id_like_cpp(),
                                    self.difficulty_store().map(AsRef::as_ref),
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
                        is_controlled: creature.creature.unit().has_unit_state(
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
        let Some(target_guid) =
            self.canonical_player_snapshot_like_cpp(|player| player.unit().attacking())
        else {
            return RepresentedArmorMitigationLikeCpp::NONE;
        };
        let Some(target_guid) = target_guid else {
            return RepresentedArmorMitigationLikeCpp::NONE;
        };
        let Some(armor_penetration_pct) = self.canonical_player_snapshot_like_cpp(|player| {
            player
                .effective_combat_stats_like_cpp()
                .armor_penetration_pct
        }) else {
            return RepresentedArmorMitigationLikeCpp::NONE;
        };
        let Some(attacker_level) =
            self.canonical_player_snapshot_like_cpp(|player| player.level_like_cpp())
        else {
            return RepresentedArmorMitigationLikeCpp::NONE;
        };
        let target_resistance_normal_aura = self
            .resolved_aura_effects_by_spell_aura_type_like_cpp(
                wow_data::spell::aura_types::SPELL_AURA_MOD_TARGET_RESISTANCE,
            )
            .unwrap_or_default()
            .into_iter()
            // C++ `SPELL_SCHOOL_MASK_NORMAL` (0x01).
            .filter(|(misc_value, _)| misc_value & 0x01 != 0)
            .map(|(_, amount)| amount)
            .sum::<i32>();
        let Some(manager) = self.map_manager.as_ref() else {
            return RepresentedArmorMitigationLikeCpp::NONE;
        };
        let instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        let victim = {
            let manager = manager
                .read()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            manager
                .find_creature(self.player_map_id_like_cpp(), instance_id, target_guid)
                .map(|creature| {
                    (
                        creature.creature.combat_log_stats_like_cpp().armor,
                        creature.level(),
                    )
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
            self.resolved_player_visible_auras_like_cpp(),
            self.spell_store(),
        ) else {
            return [RepresentedMeleeDamageBonusLikeCpp::NONE; 2];
        };
        let Some(target_guid) =
            self.canonical_player_snapshot_like_cpp(|player| player.unit().attacking())
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

    /// C++ `Unit::MeleeDamageBonusDone`'s `SPELL_AURA_MOD_AUTOATTACK_DAMAGE`
    /// factor for the canonical Player (`Unit.cpp:7620-7627`): `1.0` when the
    /// aura container or the spell store cannot be resolved.
    ///
    /// The owning session writes the result on the canonical Player through the
    /// aura-mutation sync, so the map-owned swing path only reads it.
    pub(in crate::session) fn represented_player_autoattack_damage_multiplier_like_cpp(
        &self,
    ) -> f32 {
        let (Some(auras), Some(spell_store)) = (
            self.resolved_player_visible_auras_like_cpp(),
            self.spell_store(),
        ) else {
            return 1.0;
        };
        crate::session_rules::represented_autoattack_damage_multiplier_like_cpp(&auras, spell_store)
    }
    fn canonical_unit_attack_target_state_like_cpp(
        &self,
        guid: ObjectGuid,
    ) -> (bool, bool, wow_entities::UnitAttackContextLikeCpp) {
        // Resolve the Player-owned control state before taking the map lock;
        // visibility checks below run while that same manager is borrowed.
        let moved_unit_guid = self.player_moved_unit_guid_like_cpp();
        let current_group_guid = self.resolved_group_guid_like_cpp();
        let player_phase_shift = self.represented_player_phase_shift_like_cpp();
        let Some(manager) = self.canonical_map_manager.as_ref() else {
            return (
                true,
                true,
                wow_entities::UnitAttackContextLikeCpp::default(),
            );
        };
        let Ok(manager) = manager.lock() else {
            return (
                true,
                true,
                wow_entities::UnitAttackContextLikeCpp::default(),
            );
        };
        let Some(map) = manager.find_map(u32::from(self.player_map_id_like_cpp()), 0) else {
            return (
                true,
                true,
                wow_entities::UnitAttackContextLikeCpp::default(),
            );
        };
        if let Some(player) = map.map().get_typed_player(guid) {
            let pvp_flags = player.unit().pvp_flags_like_cpp();
            let attacker_can_see_or_detect_target = player_phase_shift
                .as_ref()
                .is_some_and(|phase| phase.can_see(player.unit().world().phase_shift()))
                && self
                    .player_guid()
                    .and_then(|attacker_guid| map.map().get_typed_player(attacker_guid))
                    .map(|attacker| {
                        let mut target_unit = player.unit().clone();
                        let mut seer_unit = attacker.unit().clone();
                        self.apply_target_visibility_context_for_current_player_like_cpp(
                            &mut target_unit,
                            &mut seer_unit,
                            moved_unit_guid,
                            current_group_guid,
                        );
                        seer_unit.can_see_or_detect_unit_like_cpp(&target_unit, false, true, false)
                    })
                    .unwrap_or(true);
            return (
                player.unit().is_alive(),
                player.unit().world().object().is_in_world(),
                wow_entities::UnitAttackContextLikeCpp {
                    victim_is_game_master_player: player.is_game_master_like_cpp(),
                    victim_unit_state: player.unit().unit_state(),
                    victim_unit_flags: player.unit().unit_flags_like_cpp().bits(),
                    victim_has_affecting_player: true,
                    visibility_represented: true,
                    attacker_can_see_or_detect_target,
                    victim_in_sanctuary: pvp_flags.contains(UnitPvpFlags::SANCTUARY),
                    victim_is_pvp: pvp_flags.contains(UnitPvpFlags::PVP),
                    victim_is_ffa_pvp: pvp_flags.contains(UnitPvpFlags::FFA_PVP),
                    victim_has_pvp_unk1_flag: pvp_flags.contains(UnitPvpFlags::UNK1),
                    ..Default::default()
                },
            );
        }
        if let Some(result) = map.map().with_creature_like_cpp(guid, |creature| {
            let attacker_can_see_or_detect_target = player_phase_shift
                .as_ref()
                .is_some_and(|phase| phase.can_see(creature.unit().world().phase_shift()))
                && self
                    .player_guid()
                    .and_then(|attacker_guid| map.map().get_typed_player(attacker_guid))
                    .map(|attacker| {
                        let mut target_unit = creature.unit().clone();
                        let mut seer_unit = attacker.unit().clone();
                        self.apply_target_visibility_context_for_current_player_like_cpp(
                            &mut target_unit,
                            &mut seer_unit,
                            moved_unit_guid,
                            current_group_guid,
                        );
                        seer_unit.can_see_or_detect_unit_like_cpp(&target_unit, false, true, false)
                    })
                    .unwrap_or(true);
            let mut context = wow_entities::UnitAttackContextLikeCpp {
                victim_is_evading_creature: creature.is_evading_attacks_like_cpp(),
                victim_unit_state: creature.unit().unit_state(),
                victim_unit_flags: creature.unit().unit_flags_like_cpp().bits(),
                visibility_represented: true,
                attacker_can_see_or_detect_target,
                ..Default::default()
            };
            if let Some(reputation_snapshot) =
                self.attack_reputation_faction_snapshot_like_cpp(creature)
                && let Some(attacker_guid) = self.player_guid()
                && let Some(attacker) = map.map().get_typed_player(attacker_guid)
            {
                let player_has_contested_pvp_flag =
                    attacker.has_player_flag(PLAYER_FLAGS_CONTESTED_PVP_LIKE_CPP);
                let creature_has_forced_reputation_rank =
                    attacker.has_forced_reputation_rank_like_cpp(reputation_snapshot.faction_id);
                let player_has_reputation_state = match reputation_snapshot.can_have_reputation {
                    Some(false) => false,
                    Some(true) => {
                        attacker.has_reputation_state_like_cpp(reputation_snapshot.faction_id)
                    }
                    None => true,
                };
                if (reputation_snapshot.contested_guard && player_has_contested_pvp_flag)
                    || creature_has_forced_reputation_rank
                    || player_has_reputation_state
                {
                    context.player_creature_reputation_represented = true;
                    context.creature_is_contested_guard = reputation_snapshot.contested_guard;
                    context.player_has_contested_pvp_flag = player_has_contested_pvp_flag;
                    context.creature_has_forced_reputation_rank =
                        creature_has_forced_reputation_rank;
                    context.player_at_war_with_creature_faction = player_has_reputation_state
                        && attacker.is_at_war_with_faction_like_cpp(reputation_snapshot.faction_id);
                }
            }
            (
                creature.is_alive(),
                creature.unit().world().object().is_in_world(),
                context,
            )
        }) {
            return result;
        }
        (
            true,
            true,
            wow_entities::UnitAttackContextLikeCpp::default(),
        )
    }
    fn player_vehicle_seat_allows_attack_like_cpp(&self) -> bool {
        let Some((seat_flags, _)) = self.player_vehicle_seat_state_like_cpp() else {
            return false;
        };
        match seat_flags {
            Some(flags) => flags & VEHICLE_SEAT_FLAG_CAN_ATTACK != 0,
            None => true,
        }
    }
    fn add_canonical_attacker_like_cpp(&mut self, victim: ObjectGuid, attacker: ObjectGuid) {
        if self
            .mutate_canonical_player_by_guid_like_cpp(victim, |victim| {
                victim.unit_mut().add_attacker_like_cpp(attacker)
            })
            .is_some()
        {
            return;
        }
        let _ = self.mutate_canonical_creature_by_guid_like_cpp(victim, |victim| {
            victim.unit_mut().add_attacker_like_cpp(attacker)
        });
    }
    pub(crate) fn start_player_attack_like_cpp(
        &mut self,
        victim: ObjectGuid,
    ) -> PlayerAttackStartLikeCppResult {
        let _ = self.ensure_canonical_world_map_for_current_player_like_cpp();
        let player_guid = self.player_guid();
        let Some((attacker_unit_flags, _, _)) = self.player_unit_presentation_snapshot_like_cpp()
        else {
            return PlayerAttackStartLikeCppResult::Rejected;
        };
        let attacker_is_mounted_player = attacker_unit_flags.contains(UnitFlags::MOUNT);
        let (victim_alive, victim_in_world, mut attack_context) =
            self.canonical_unit_attack_target_state_like_cpp(victim);
        if !self.player_vehicle_seat_allows_attack_like_cpp() {
            self.set_combat_target_like_cpp(None);
            self.set_in_combat_like_cpp(false);
            if self.selection_guid_like_cpp() == Some(victim) {
                self.set_selection_guid_like_cpp(None);
            }
            return PlayerAttackStartLikeCppResult::Rejected;
        }
        attack_context.attacker_is_mounted_player = attacker_is_mounted_player;
        attack_context.attacker_unit_flags = attacker_unit_flags.bits();
        attack_context.attacker_has_affecting_player = true;
        #[cfg_attr(not(test), allow(unused_mut))]
        let mut attacker_pvp_flags =
            self.with_owned_player_like_cpp(|player| player.unit().pvp_flags_like_cpp());
        #[cfg(test)]
        if attacker_pvp_flags.is_none() && self.player_handle_like_cpp.is_none() {
            attacker_pvp_flags =
                player_guid.and_then(|guid| self.canonical_player_pvp_flags_like_cpp(guid));
        }
        let attacker_pvp_flags = attacker_pvp_flags.unwrap_or_default();
        attack_context.attacker_in_sanctuary = attacker_pvp_flags.contains(UnitPvpFlags::SANCTUARY);
        attack_context.attacker_is_ffa_pvp = attacker_pvp_flags.contains(UnitPvpFlags::FFA_PVP);
        attack_context.attacker_has_pvp_unk1_flag = attacker_pvp_flags.contains(UnitPvpFlags::UNK1);
        if attack_context.victim_has_affecting_player {
            attack_context.sanctuary_represented = true;
            attack_context.pvp_represented = true;
            attack_context.player_player_duel_in_progress = player_guid
                .and_then(|guid| self.canonical_player_duel_in_progress_like_cpp(guid, victim))
                .unwrap_or(false);
        }
        attack_context.attacker_is_player_uber = player_guid
            .and_then(|guid| {
                self.canonical_player_has_player_flag_like_cpp(guid, PLAYER_FLAGS_UBER_LIKE_CPP)
            })
            .unwrap_or(false);
        let combat_relation_represented = attack_context.relation_represented;
        let combat_attacker_is_friendly_to_victim = attack_context.attacker_is_friendly_to_victim;
        let combat_victim_is_friendly_to_attacker = attack_context.victim_is_friendly_to_attacker;
        self.set_selection_guid_like_cpp(Some(victim));
        let outcome = self.mutate_canonical_player_like_cpp(|player| {
            player.unit_mut().attack_with_context_like_cpp(
                victim,
                victim_alive,
                victim_in_world,
                true,
                attack_context,
            )
        });
        let previous = match outcome {
            Some(wow_entities::UnitAttackStartOutcome::NewTarget { previous }) => previous,
            Some(
                wow_entities::UnitAttackStartOutcome::MeleeStartedSameTarget
                | wow_entities::UnitAttackStartOutcome::MeleeStoppedSameTarget
                | wow_entities::UnitAttackStartOutcome::NoChangeSameTarget,
            ) => None,
            Some(
                wow_entities::UnitAttackStartOutcome::InvalidSelfTarget
                | wow_entities::UnitAttackStartOutcome::InvalidDeadAttacker
                | wow_entities::UnitAttackStartOutcome::InvalidDeadVictim
                | wow_entities::UnitAttackStartOutcome::InvalidVictimNotInWorld
                | wow_entities::UnitAttackStartOutcome::InvalidMountedAttacker
                | wow_entities::UnitAttackStartOutcome::InvalidAttackerEvading
                | wow_entities::UnitAttackStartOutcome::InvalidVictimGameMaster
                | wow_entities::UnitAttackStartOutcome::InvalidVictimEvading
                | wow_entities::UnitAttackStartOutcome::InvalidAttackTarget,
            )
            | None => {
                self.set_combat_target_like_cpp(None);
                self.set_in_combat_like_cpp(false);
                if self.selection_guid_like_cpp() == Some(victim) {
                    self.set_selection_guid_like_cpp(None);
                }
                return PlayerAttackStartLikeCppResult::Rejected;
            }
        };
        if let Some(player_guid) = player_guid {
            if let Some(previous) = previous {
                self.remove_canonical_attacker_like_cpp(previous, player_guid);
            }
            self.add_canonical_attacker_like_cpp(victim, player_guid);
            let _ = self.mutate_world_creature(victim, |victim| {
                victim
                    .creature
                    .unit_mut()
                    .add_attacker_like_cpp(player_guid);
            });
            let _ = self.begin_canonical_player_combat_ref_like_cpp(
                player_guid,
                victim,
                combat_relation_represented,
                combat_attacker_is_friendly_to_victim,
                combat_victim_is_friendly_to_attacker,
            );
        }
        self.set_in_combat_like_cpp(true);
        let send_attack_start = matches!(
            outcome,
            Some(
                wow_entities::UnitAttackStartOutcome::NewTarget { .. }
                    | wow_entities::UnitAttackStartOutcome::MeleeStartedSameTarget
            )
        );
        PlayerAttackStartLikeCppResult::Accepted { send_attack_start }
    }
    pub(crate) fn stop_player_attack_like_cpp(&mut self) -> Option<ObjectGuid> {
        let player_guid = self.player_guid()?;
        let target = match self.mutate_canonical_player_like_cpp(|player| {
            match player.unit_mut().attack_stop_like_cpp() {
                wow_entities::UnitAttackStopOutcome::Stopped { victim } => Some(victim),
                wow_entities::UnitAttackStopOutcome::NoVictim => None,
            }
        }) {
            Some(Some(victim)) => victim,
            // C++ Unit::AttackStop returns false when m_attacking is null; a
            // stale session mirror must not invent a victim when canonical
            // player state exists and says there is none.
            Some(None) => {
                self.set_combat_target_like_cpp(None);
                self.set_in_combat_like_cpp(false);
                return None;
            }
            None => self.resolved_combat_target_like_cpp().flatten()?,
        };
        self.set_combat_target_like_cpp(None);
        self.set_in_combat_like_cpp(false);
        if self.selection_guid_like_cpp() == Some(target) {
            self.set_selection_guid_like_cpp(None);
        }
        self.remove_canonical_attacker_like_cpp(target, player_guid);
        let _ = self.mutate_world_creature(target, |victim| {
            victim
                .creature
                .unit_mut()
                .remove_attacker_like_cpp(player_guid);
        });
        Some(target)
    }
    pub(crate) fn player_class_attack_power_coefficients_like_cpp(
        &self,
        class: u8,
    ) -> Option<(u8, u8, u8)> {
        self.chr
            .classes_store
            .as_ref()?
            .get(u32::from(class))
            .map(|entry| {
                (
                    entry.attack_power_per_strength,
                    entry.attack_power_per_agility,
                    entry.ranged_attack_power_per_agility,
                )
            })
    }
}
