//! Legacy creature melee tick and its damage application.
//!
//! Moved out of the Session root under #619. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

/// C++ `Unit::CalcAbsorbResist`'s represented stages for a player victim
/// (`Unit.cpp:1789-1930`), committed inside the same map-owned phase as the
/// victim's health write: the school-absorb loop, then the mana-shield loop.
///
/// C++ spends each shield effect's amount and the mana-shield drain while it
/// calculates the swing, so both are pool data and this map-owned stage is
/// their writer; the session's aura transition at delivery owns the *removal*
/// of an exhausted shield and its publication. A shield C++ would remove is
/// left at zero here, which the session removes through the same `remove_aura`
/// path it owns for every other aura.
///
/// Returns `None` when the map or the canonical player cannot be resolved,
/// which keeps the caller's pre-absorb damage unchanged. Otherwise the tuple is
/// `(absorbed, remaining damage, mana spent, consumptions)` in the delivery
/// command's shape, with the outcomes in `AbsorbAuraOrderPred` order followed
/// by the mana shields.
#[allow(clippy::type_complexity)]
fn apply_melee_absorb_to_canonical_player_like_cpp(
    canonical_manager: &mut wow_map::MapManager,
    map_id: u32,
    instance_id: u32,
    victim_guid: ObjectGuid,
    school_mask: u32,
    damage: u32,
    spell_store: &wow_data::SpellStore,
    difficulty_id: u8,
    difficulty_store: Option<&wow_data::DifficultyStore>,
    // C++ `CalcAbsorbResist`'s `auraAbsorbMod` from the attacker's
    // `SPELL_AURA_MOD_TARGET_ABSORB_SCHOOL`.
    ignore_absorb_pct: f32,
) -> Option<(
    u32,
    u32,
    u32,
    Vec<crate::session::mailbox::CreatureMeleeAbsorbConsumptionLikeCpp>,
)> {
    let managed = canonical_manager.find_map_mut(map_id, instance_id)?;
    let player = managed.map_mut().get_typed_player_mut(victim_guid)?;
    let auras = player
        .unit()
        .subsystems()
        .auras
        .runtime_applications_like_cpp()
        .clone();
    let shields = crate::session_rules::player_absorb_shields_like_cpp(
        &auras,
        spell_store,
        difficulty_id,
        difficulty_store,
        school_mask,
    );
    let absorb = crate::session_rules::represented_melee_absorb_like_cpp(
        &shields,
        damage,
        ignore_absorb_pct,
    );
    let mut consumptions = Vec::with_capacity(absorb.consumed.len());
    for consumption in &absorb.consumed {
        write_absorbed_shield_amount_like_cpp(player, consumption);
        consumptions.push(
            crate::session::mailbox::CreatureMeleeAbsorbConsumptionLikeCpp {
                slot: consumption.slot,
                consumed: consumption.consumed,
                removed: consumption.removed,
            },
        );
    }

    // C++ runs the mana-shield loop after the school-absorb loop
    // (`Unit.cpp:1886-1930`) over the damage the school shields left.
    let mana_shields = crate::session_rules::player_mana_shields_like_cpp(
        &auras,
        spell_store,
        difficulty_id,
        difficulty_store,
        school_mask,
    );
    let mana_before = player
        .unit()
        .get_power(wow_constants::PowerType::Mana)
        .max(0);
    let mana_absorb = crate::session_rules::represented_melee_mana_absorb_like_cpp(
        &mana_shields,
        absorb.damage,
        mana_before as u32,
        ignore_absorb_pct,
    );
    if mana_absorb.mana_spent > 0 {
        // `Unit::ModifyPower(POWER_MANA, -manaReduction)`: the same locked map
        // phase that commits the health write owns the drain, and the canonical
        // setter clamps it like C++.
        player.unit_mut().set_power(
            wow_constants::PowerType::Mana,
            mana_before - i32::try_from(mana_absorb.mana_spent).unwrap_or(i32::MAX),
        );
    }
    for consumption in &mana_absorb.consumed {
        write_absorbed_shield_amount_like_cpp(
            player,
            &crate::session_rules::RepresentedAbsorbConsumptionLikeCpp {
                slot: consumption.slot,
                effect_index: consumption.effect_index,
                consumed: consumption.consumed,
                remaining: consumption.remaining,
                removed: consumption.removed,
            },
        );
        consumptions.push(
            crate::session::mailbox::CreatureMeleeAbsorbConsumptionLikeCpp {
                slot: consumption.slot,
                consumed: consumption.consumed,
                removed: consumption.removed,
            },
        );
    }
    Some((
        absorb.absorbed + mana_absorb.absorbed,
        mana_absorb.damage,
        mana_absorb.mana_spent,
        consumptions,
    ))
}

/// Commit one spent shield's `AuraEffect` remainder on the canonical player.
fn write_absorbed_shield_amount_like_cpp(
    player: &mut wow_entities::Player,
    consumption: &crate::session_rules::RepresentedAbsorbConsumptionLikeCpp,
) {
    crate::session::combat::write_absorbed_shield_amount_like_cpp(
        player,
        consumption.slot,
        consumption.effect_index,
        consumption.remaining,
    );
}

/// Apply one player's melee swings to a legacy creature.
///
/// Lifted out of `run_combat_tick` by #28. This is the write path that made
/// every logged-in session a writer of shared creature combat state; extracting
/// it is what lets the global loop become its sole owner. Damage arithmetic,
/// tap assignment, threat, the death branch and the swing record are unchanged.
pub(in crate::session) fn apply_player_melee_to_legacy_creature_like_cpp(
    creature: &mut crate::map_manager::WorldCreature,
    player_guid: ObjectGuid,
    tap_group_guids: &[ObjectGuid],
    canonical_swings: Option<&[crate::session::combat::RepresentedMeleeSwingLikeCpp]>,
) -> Option<PlayerMeleeCreatureHitLikeCpp> {
    if !creature.is_alive() {
        return None;
    }
    if creature.state() != wow_entities::CreatureAiState::InCombat {
        creature.enter_combat(player_guid);
    }
    let damages: Vec<crate::session::combat::RepresentedMeleeSwingLikeCpp> = match canonical_swings
    {
        Some(swings) => swings.to_vec(),
        None => {
            if !creature.can_swing() {
                return None;
            }
            vec![
                crate::session::combat::RepresentedMeleeSwingLikeCpp::hit_like_cpp(
                    creature.roll_damage()?.max(1),
                ),
            ]
        }
    };
    let entry = creature.entry();
    let level = creature.level();
    let mut swings = Vec::new();
    let mut swing_presentations = Vec::new();
    let mut died = false;
    let mut move_stop = None;
    for swing in damages {
        if !creature.is_alive() {
            break;
        }
        let damage = swing.damage;
        // C++ `DealMeleeDamage` applies nothing for a missed or avoided swing:
        // no damage, no tap and no threat.
        if damage == 0 {
            swings.push((0, false, -1));
            swing_presentations.push((
                swing.hit_info,
                swing.victim_state,
                swing.blocked,
                swing.original_damage,
            ));
            continue;
        }
        let health_before = creature.current_hp();
        creature
            .creature
            .set_tapped_by_player(player_guid, tap_group_guids);
        died = creature.take_damage_before_death_state_like_cpp(damage);
        let over_damage = if died {
            damage.saturating_sub(health_before) as i32
        } else {
            -1
        };
        creature
            .creature
            .unit_mut()
            .subsystems_mut()
            .combat
            .add_threat(player_guid, damage as f32);
        swings.push((damage, died, over_damage));
        swing_presentations.push((
            swing.hit_info,
            swing.victim_state,
            swing.blocked,
            swing.original_damage,
        ));
        if died {
            let combat = &mut creature.creature.unit_mut().subsystems_mut().combat;
            combat.clear_threat();
            combat.clear_attackers();
            move_stop = creature
                .stop_move_spline_like_cpp()
                .map(|stop| (stop.position, stop.spline_id));
            break;
        }
    }
    if canonical_swings.is_none() {
        creature.record_swing();
    }
    let values_update = creature.creature.unit().values_update();
    Some(PlayerMeleeCreatureHitLikeCpp {
        swings,
        swing_presentations,
        entry,
        level,
        died,
        move_stop,
        values_update,
    })
}
/// Runs one global legacy creature melee tick without spawning a loop.
///
/// This is dormant infrastructure for the next runtime slice after movement
/// and lifecycle. C++ contrast: `Creature::Update` calls
/// `DoMeleeAttackIfReady()` from the map object update phase. This function
/// preserves the pre-existing transitional damage bridge while the complete
/// C++ outcome/proc pipeline remains a later runtime slice. Spell-hit RNG
/// accreditation must not turn otherwise valid creature swings into no-ops.
pub fn run_legacy_creature_melee_tick_once_like_cpp(
    legacy_map_manager: &crate::map_manager::SharedMapManager,
    canonical_map_manager: Option<&SharedCanonicalMapManager>,
    config: &crate::session::LegacyCreatureAggroConfigLikeCpp,
) -> LegacyCreatureMeleeTickOutcomeLikeCpp {
    use crate::map_manager::RuntimeTickOwner;
    use wow_entities::CurrentSpellSlot;

    #[derive(Clone, Copy)]
    struct PendingCreatureSwingLikeCpp {
        map_id: u16,
        instance_id: u32,
        attacker_guid: ObjectGuid,
        attacker_position: Position,
        attacker_combat_reach: f32,
        attacker_can_state_update: bool,
        victim_guid: ObjectGuid,
    }

    struct CreatureVictimCompatibilitySyncLikeCpp {
        swing: PendingCreatureSwingLikeCpp,
        state: CreatureMeleeVictimSyncStateLikeCpp,
    }

    struct CreatureVictimCompatibilitySyncChainLikeCpp {
        swing: PendingCreatureSwingLikeCpp,
        states: Vec<CreatureMeleeVictimSyncStateLikeCpp>,
    }

    let mut outcome = LegacyCreatureMeleeTickOutcomeLikeCpp::default();
    let mut pending_swings = Vec::new();

    {
        let mut manager = legacy_map_manager
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if manager.tick_owner() != RuntimeTickOwner::GlobalLegacy {
            outcome.skipped_owner_not_global = true;
            return outcome;
        }

        let map_keys = manager.active_map_keys();
        outcome.maps_seen = map_keys.len();
        for (map_id, instance_id) in map_keys {
            let guids = manager.creature_guids(map_id, instance_id);
            for guid in guids {
                let Some(creature) = manager.find_creature_mut(map_id, instance_id, guid) else {
                    continue;
                };
                outcome.creatures_seen += 1;
                if !creature.can_swing() {
                    continue;
                }
                // C++ `TurretAI` calls `SetCanMelee(false)` in its
                // constructor. The transitional selector stores the explicit
                // DB AIName rather than a live AI object, so enforce that
                // constructor side effect at the global melee boundary.
                if creature.creature.lifecycle_metadata().ai_name == "TurretAI" {
                    outcome.melee_precondition_rejections += 1;
                    continue;
                }
                if !creature.creature.can_melee_like_cpp() {
                    outcome.melee_precondition_rejections += 1;
                    continue;
                }
                let unit = creature.creature.unit();
                if unit.has_unit_state(UnitState::CHARGING.bits())
                    || (unit.has_unit_state(UnitState::CASTING.bits())
                        && !unit
                            .current_spell(CurrentSpellSlot::Channeled)
                            .is_some_and(|spell| spell.allow_actions_during_channel))
                {
                    outcome.melee_precondition_rejections += 1;
                    continue;
                }
                let Some(victim_guid) = creature.creature.ai_ownership().combat_target else {
                    continue;
                };
                if !victim_guid.is_player() && !victim_guid.is_any_type_creature() {
                    continue;
                }
                pending_swings.push(PendingCreatureSwingLikeCpp {
                    map_id,
                    instance_id,
                    attacker_guid: guid,
                    attacker_position: creature.position(),
                    attacker_combat_reach: creature.creature.unit().world().combat_reach(),
                    attacker_can_state_update: creature
                        .creature
                        .unit()
                        .can_attacker_state_update_melee_like_cpp(false),
                    victim_guid,
                });
                outcome.swings_ready += 1;
            }
        }
    }

    let Some(canonical_map_manager) = canonical_map_manager else {
        return outcome;
    };

    let mut creature_victim_syncs = Vec::new();
    for mut swing in pending_swings {
        // One C++ map update owns attacker validation, RNG consumption, damage,
        // attacking-aura removal, and timer rearm as one serial operation. Hold
        // both transitional owners in the established canonical -> legacy order
        // so a target switch or same-GUID respawn cannot cross that commit.
        let Ok(mut canonical_manager) = canonical_map_manager.lock() else {
            outcome.melee_precondition_rejections += 1;
            continue;
        };
        let mut legacy_manager = legacy_map_manager
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let Some(attacker) =
            legacy_manager.find_creature_mut(swing.map_id, swing.instance_id, swing.attacker_guid)
        else {
            outcome.melee_precondition_rejections += 1;
            continue;
        };

        if !attacker.can_swing()
            || attacker.creature.ai_ownership().combat_target != Some(swing.victim_guid)
            || attacker.creature.lifecycle_metadata().ai_name == "TurretAI"
            || !attacker.creature.can_melee_like_cpp()
        {
            outcome.melee_precondition_rejections += 1;
            continue;
        }
        let unit = attacker.creature.unit();
        if unit.has_unit_state(UnitState::CHARGING.bits())
            || (unit.has_unit_state(UnitState::CASTING.bits())
                && !unit
                    .current_spell(CurrentSpellSlot::Channeled)
                    .is_some_and(|spell| spell.allow_actions_during_channel))
        {
            outcome.melee_precondition_rejections += 1;
            continue;
        }
        swing.attacker_position = attacker.position();
        swing.attacker_combat_reach = unit.world().combat_reach();
        swing.attacker_can_state_update = unit.can_attacker_state_update_melee_like_cpp(false);

        let canonical_attacker_is_same_incarnation = canonical_manager
            .find_map(u32::from(swing.map_id), swing.instance_id)
            .and_then(|managed| {
                managed
                    .map()
                    .with_creature_like_cpp(swing.attacker_guid, |canonical_attacker| {
                        canonical_attacker.spawn_id() == attacker.creature.spawn_id()
                            && canonical_attacker
                                .loot_authority_like_cpp()
                                .shares_storage_like_cpp(
                                    attacker.creature.loot_authority_like_cpp(),
                                )
                            && canonical_attacker
                                .unit()
                                .shares_health_state_revision_authority_like_cpp(
                                    &attacker
                                        .creature
                                        .unit()
                                        .health_state_revision_authority_like_cpp(),
                                )
                    })
            })
            .unwrap_or(false);
        if !canonical_attacker_is_same_incarnation {
            outcome.melee_precondition_rejections += 1;
            outcome.attacker_incarnation_rejections += 1;
            continue;
        }
        // Movement snapshots normally publish this position before melee, but
        // the two global ticks may overlap between their legacy read and
        // canonical write phases. With both owners locked and the incarnation
        // proven equal, align the canonical WorldObject used by the LOS check to
        // the same live position used for range and facing.
        if let Some(managed) =
            canonical_manager.find_map_mut(u32::from(swing.map_id), swing.instance_id)
        {
            let _ = managed
                .map_mut()
                .relocate_map_object_like_cpp(swing.attacker_guid, swing.attacker_position);
        }

        let apply = |canonical_manager: &mut wow_map::MapManager,
                     swing: &PendingCreatureSwingLikeCpp,
                     damage,
                     presentation: Option<(u32, u8, i32)>| {
            if swing.victim_guid.is_player() {
                apply_creature_melee_damage_to_canonical_player_on_map_like_cpp(
                    canonical_manager,
                    u32::from(swing.map_id),
                    swing.instance_id,
                    swing.attacker_guid,
                    swing.attacker_position,
                    swing.attacker_combat_reach,
                    swing.attacker_can_state_update,
                    swing.victim_guid,
                    damage,
                )
            } else {
                apply_creature_melee_damage_to_canonical_creature_on_map_like_cpp(
                    canonical_manager,
                    u32::from(swing.map_id),
                    swing.instance_id,
                    swing.attacker_guid,
                    swing.attacker_position,
                    swing.attacker_combat_reach,
                    swing.attacker_can_state_update,
                    swing.victim_guid,
                    damage,
                    presentation,
                )
            }
        };

        match apply(&mut canonical_manager, &swing, None, None) {
            CreatureMeleeApplyResultLikeCpp::Ready => {}
            CreatureMeleeApplyResultLikeCpp::Hit { .. } => {
                unreachable!("melee precondition validation must not mutate canonical health")
            }
            CreatureMeleeApplyResultLikeCpp::OutOfRange => {
                outcome.melee_range_rejections += 1;
                attacker.record_failed_swing_retry_like_cpp();
                continue;
            }
            CreatureMeleeApplyResultLikeCpp::BadFacing => {
                outcome.melee_facing_rejections += 1;
                attacker.record_failed_swing_retry_like_cpp();
                continue;
            }
            CreatureMeleeApplyResultLikeCpp::AttackerStateRejected => {
                outcome.attacker_state_rejections += 1;
                attacker.record_swing();
                continue;
            }
            CreatureMeleeApplyResultLikeCpp::LosRejected => {
                outcome.melee_los_rejections += 1;
                attacker.record_swing();
                continue;
            }
            CreatureMeleeApplyResultLikeCpp::VictimNotAlive => {
                // `DoMeleeAttackIfReady` resets BASE_ATTACK after
                // `AttackerStateUpdate` returns early for a dead victim.
                outcome.melee_precondition_rejections += 1;
                attacker.record_swing();
                continue;
            }
            CreatureMeleeApplyResultLikeCpp::AttackerUnavailable => {
                outcome.melee_precondition_rejections += 1;
                continue;
            }
            CreatureMeleeApplyResultLikeCpp::MissingVictim => {
                outcome.melee_precondition_rejections += 1;
                continue;
            }
        }

        if !swing.attacker_can_state_update {
            outcome.attacker_state_rejections += 1;
            attacker.record_swing();
            continue;
        }
        let Some(damage) = attacker.roll_damage() else {
            outcome.melee_precondition_rejections += 1;
            // `DoMeleeAttackIfReady` rearms BASE_ATTACK after
            // `AttackerStateUpdate` even if damage calculation cannot produce a
            // represented result.
            attacker.record_swing();
            continue;
        };
        // The compatibility bridge preserves the pre-existing damage and wire
        // behavior, but it does not model RollMeleeOutcomeAgainst or later
        // proc/daze draws. Keep gameplay running while preventing a later
        // creature spell from claiming an exact shared-RNG position.
        attacker.invalidate_runtime_rng_authority_like_cpp();
        let damage = damage.max(1);

        // C++ `CalculateMeleeDamage` rolls the attack table after mitigation and
        // before the outcome switch (`Unit.cpp:1341-1443`). A player victim
        // resolves miss/dodge/parry/crit here; the block band and the
        // player-victim armour/taken terms remain the documented boundary of
        // this slice. Every term needs the spell store, so without one the
        // pre-table always-hit bridge is preserved.
        let mut hit_info = wow_packet::packets::combat::HIT_INFO_AFFECTS_VICTIM;
        let mut victim_state = wow_packet::packets::combat::VICTIM_STATE_HIT;
        let mut original_damage = damage;
        let mut avoided_outcome = None;
        // The creature-victim branch publishes through the compatibility
        // bridge, so it carries its own presentation and avoid flag.
        let mut creature_victim_presentation: Option<(u32, u8, i32)> = None;
        let mut creature_victim_avoided = false;
        let mut outcome_represented = false;
        // C++ `CalcAbsorbResist`'s result for this swing: the absorbed amount
        // the packet publishes and every shield it spent. The victim session
        // owns the absorb-log publication and the aura transition, so it
        // receives the consumption list at delivery.
        let mut absorbed_damage = 0u32;
        let mut mana_spent = 0u32;
        let mut absorb_consumptions: Vec<
            crate::session::mailbox::CreatureMeleeAbsorbConsumptionLikeCpp,
        > = Vec::new();
        let damage = if swing.victim_guid.is_player() {
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
                            .filter(|effect| {
                                effect.aura_type == aura_type && effect.misc_value & 0x01 != 0
                            })
                            .map(|effect| effect.amount as f32)
                            .sum()
                    };
                    let attacker_target_resistance_normal_aura = attacker_armor_pen(
                        wow_data::spell::aura_types::SPELL_AURA_MOD_TARGET_RESISTANCE,
                    );
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
                        5.0 + aura_sum(
                            wow_data::spell::aura_types::SPELL_AURA_MOD_WEAPON_CRIT_PERCENT,
                        ) + aura_sum(wow_data::spell::aura_types::SPELL_AURA_MOD_CRIT_PCT)
                    };
                    let expertise_reduction_pct =
                        aura_sum(wow_data::spell::aura_types::SPELL_AURA_MOD_EXPERTISE) / 4.0;
                    let attacker_facts =
                        crate::session_rules::RepresentedMeleeAttackerFactsLikeCpp {
                            level: attacker.creature.level(),
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
                            expertise_reduction_pct: [
                                expertise_reduction_pct,
                                expertise_reduction_pct,
                            ],
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
                                is_immune_to_damage:
                                    crate::session_rules::player_aura_effects_full_by_spell_aura_type_like_cpp(
                                        auras,
                                        spell_store,
                                        wow_data::spell::aura_types::SPELL_AURA_SCHOOL_IMMUNITY,
                                    )
                                    .into_iter()
                                    .any(|effect| effect.misc_value & 0x01 != 0),
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
                            )
                        });
                    match victim {
                        Some((
                            victim_facts,
                            victim_armor,
                            bypass_armor_pct_by_caster,
                            victim_taken_effects,
                            player_block_percent,
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
                            let after_taken =
                                crate::session_rules::melee_damage_taken_apply_like_cpp(
                                    taken, damage,
                                );
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
                            let rolled =
                                crate::session_rules::rolled_melee_outcome_like_cpp(&inputs[0]);
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
                                crate::session_rules::melee_outcome_presentation_like_cpp(
                                    rolled, false,
                                );
                            hit_info = info;
                            victim_state = state;
                            original_damage = original;
                            outcome_represented = true;
                            if matches!(
                                rolled,
                                crate::session_rules::RepresentedMeleeOutcomeLikeCpp::Immune
                                    | crate::session_rules::RepresentedMeleeOutcomeLikeCpp::Evade
                                    | crate::session_rules::RepresentedMeleeOutcomeLikeCpp::Miss
                                    | crate::session_rules::RepresentedMeleeOutcomeLikeCpp::Dodge
                                    | crate::session_rules::RepresentedMeleeOutcomeLikeCpp::Parry
                            ) {
                                avoided_outcome = Some(rolled);
                            }
                            // C++ `Unit::CalculateMeleeDamage`'s absorb stage
                            // (`Unit.cpp:1449-1466`) runs after the outcome
                            // switch and before `DealMeleeDamage`, so the
                            // committed damage and the published `SubDmg`
                            // already carry the reduced amount. Physical melee
                            // always uses `SPELL_SCHOOL_MASK_NORMAL`.
                            let absorb = apply_melee_absorb_to_canonical_player_like_cpp(
                                &mut canonical_manager,
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
                                        absorbed_damage = absorbed;
                                        mana_spent = spent;
                                        absorb_consumptions = consumptions;
                                        hit_info |= if remaining == 0 {
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
        } else {
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
                            .filter(|effect| {
                                effect.aura_type == aura_type && effect.misc_value & 0x01 != 0
                            })
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
                        5.0 + aura_sum(
                            wow_data::spell::aura_types::SPELL_AURA_MOD_WEAPON_CRIT_PERCENT,
                        ) + aura_sum(wow_data::spell::aura_types::SPELL_AURA_MOD_CRIT_PCT)
                    };
                    let expertise_reduction_pct =
                        aura_sum(wow_data::spell::aura_types::SPELL_AURA_MOD_EXPERTISE) / 4.0;
                    let attacker_facts =
                        crate::session_rules::RepresentedMeleeAttackerFactsLikeCpp {
                            level: attacker.creature.level(),
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
                            expertise_reduction_pct: [
                                expertise_reduction_pct,
                                expertise_reduction_pct,
                            ],
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
                                        // `SPELL_AURA_SCHOOL_IMMUNITY` effect
                                        // whose `MiscValue` covers the normal
                                        // school.
                                        is_immune_to_damage: effects.iter().any(|effect| {
                                            effect.aura_type
                                                == wow_data::spell::aura_types::SPELL_AURA_SCHOOL_IMMUNITY
                                                && effect.misc_value & 0x01 != 0
                                        }),
                                    };
                                    (facts, victim.combat_log_stats_like_cpp().armor, effects)
                                })
                        });
                    match victim {
                        Some((victim_facts, victim_armor, victim_effects)) => {
                            let taken = crate::session_rules::melee_damage_taken_flat_pct_like_cpp(
                                &victim_effects,
                                &attacker_ignore_resist,
                                swing.attacker_guid,
                                // C++ `SPELL_SCHOOL_MASK_NORMAL` (0x01).
                                0x01,
                            );
                            let after_taken =
                                crate::session_rules::melee_damage_taken_apply_like_cpp(
                                    taken, damage,
                                );
                            let mitigated =
                                crate::session_rules::armor_reduced_damage_like_cpp(
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
                            let rolled =
                                crate::session_rules::rolled_melee_outcome_like_cpp(&inputs[0]);
                            let (info, state) =
                                crate::session_rules::melee_outcome_presentation_like_cpp(
                                    rolled, false,
                                );
                            let (outcome_damage, blocked, _original) =
                                crate::session_rules::melee_outcome_damage_like_cpp(
                                    rolled,
                                    mitigated,
                                    attacker_facts.level,
                                    victim_facts.level,
                                    attacker_facts.crit_damage_multiplier,
                                    crate::session_rules::CREATURE_BLOCK_PERCENT_LIKE_CPP,
                                );
                            creature_victim_presentation = Some((info, state, blocked as i32));
                            creature_victim_avoided = matches!(
                                rolled,
                                crate::session_rules::RepresentedMeleeOutcomeLikeCpp::Immune
                                    | crate::session_rules::RepresentedMeleeOutcomeLikeCpp::Evade
                                    | crate::session_rules::RepresentedMeleeOutcomeLikeCpp::Miss
                                    | crate::session_rules::RepresentedMeleeOutcomeLikeCpp::Dodge
                                    | crate::session_rules::RepresentedMeleeOutcomeLikeCpp::Parry
                            );
                            outcome_represented = true;
                            outcome_damage
                        }
                        None => damage,
                    }
                }
                None => damage,
            }
        };
        if !outcome_represented {
            outcome.melee_outcomes_unrepresented += 1;
        }

        // C++ `CalculateMeleeDamage` returns before `DealMeleeDamage` for an
        // avoided swing (`Unit.cpp:1345-1355`, `1395-1407`): no health write, no
        // death check and no proc. The command carries the victim's unchanged
        // canonical tuple so the session's revision gate stays exact.
        if let Some(avoided) = avoided_outcome {
            let victim = canonical_manager
                .find_map(u32::from(swing.map_id), swing.instance_id)
                .and_then(|managed| managed.map().get_typed_player(swing.victim_guid))
                .map(|victim| {
                    (
                        victim.unit().data().health,
                        victim.unit().health_state_revision_like_cpp(),
                        victim.unit().data().level.clamp(0, i32::from(u8::MAX)) as u8,
                    )
                });
            let Some((victim_health_after, victim_health_state_revision_after, target_level)) =
                victim
            else {
                outcome.melee_precondition_rejections += 1;
                continue;
            };
            // C++ `Unit::AttackerStateUpdate` removes the attacking-interrupt
            // auras before `CalculateMeleeDamage`, so an avoided swing removes
            // them too (`Unit.cpp:2172-2173`).
            outcome.attacking_interrupt_auras_removed += attacker
                .creature
                .unit_mut()
                .remove_attacking_interrupt_auras_like_cpp();
            attacker.record_swing();
            outcome.commands.push(
                crate::session::mailbox::ApplyCreatureMeleeDamageLikeCppCommand {
                    attacker_guid: swing.attacker_guid,
                    victim_guid: swing.victim_guid,
                    map_id: swing.map_id,
                    instance_id: swing.instance_id,
                    damage: 0,
                    over_damage: -1,
                    target_level,
                    victim_health_after,
                    victim_health_state_revision_after,
                    hit_info,
                    victim_state,
                    original_damage,
                    absorbed: 0,
                    mana_spent: 0,
                    absorb_consumptions: Vec::new(),
                },
            );
            continue;
        }

        let (
            victim_applied_damage,
            victim_health_before,
            victim_health_after,
            victim_health_state_revision_before,
            victim_health_state_revision_after,
            victim_creature_sync_identity,
            over_damage,
            target_level,
            events,
        ) = match apply(
            &mut canonical_manager,
            &swing,
            Some(damage),
            creature_victim_presentation,
        ) {
            CreatureMeleeApplyResultLikeCpp::Hit {
                victim_applied_damage,
                victim_health_before,
                victim_health_after,
                victim_health_state_revision_before,
                victim_health_state_revision_after,
                victim_creature_sync_identity,
                over_damage,
                target_level,
                events,
            } => (
                victim_applied_damage,
                victim_health_before,
                victim_health_after,
                victim_health_state_revision_before,
                victim_health_state_revision_after,
                victim_creature_sync_identity,
                over_damage,
                target_level,
                events,
            ),
            CreatureMeleeApplyResultLikeCpp::Ready => unreachable!(
                "melee apply with represented damage must not return validation readiness"
            ),
            CreatureMeleeApplyResultLikeCpp::OutOfRange => {
                outcome.melee_range_rejections += 1;
                attacker.record_failed_swing_retry_like_cpp();
                continue;
            }
            CreatureMeleeApplyResultLikeCpp::BadFacing => {
                outcome.melee_facing_rejections += 1;
                attacker.record_failed_swing_retry_like_cpp();
                continue;
            }
            CreatureMeleeApplyResultLikeCpp::AttackerStateRejected => {
                outcome.attacker_state_rejections += 1;
                attacker.record_swing();
                continue;
            }
            CreatureMeleeApplyResultLikeCpp::LosRejected => {
                outcome.melee_los_rejections += 1;
                attacker.record_swing();
                continue;
            }
            CreatureMeleeApplyResultLikeCpp::VictimNotAlive => {
                outcome.melee_precondition_rejections += 1;
                attacker.record_swing();
                continue;
            }
            CreatureMeleeApplyResultLikeCpp::AttackerUnavailable => {
                outcome.melee_precondition_rejections += 1;
                continue;
            }
            CreatureMeleeApplyResultLikeCpp::MissingVictim => {
                outcome.melee_precondition_rejections += 1;
                continue;
            }
        };
        outcome.attacking_interrupt_auras_removed += attacker
            .creature
            .unit_mut()
            .remove_attacking_interrupt_auras_like_cpp();
        attacker.record_swing();
        outcome.canonical_hits += 1;
        if swing.victim_guid.is_player() {
            outcome.commands.push(
                crate::session::mailbox::ApplyCreatureMeleeDamageLikeCppCommand {
                    attacker_guid: swing.attacker_guid,
                    victim_guid: swing.victim_guid,
                    map_id: swing.map_id,
                    instance_id: swing.instance_id,
                    damage,
                    over_damage,
                    target_level,
                    victim_health_after,
                    victim_health_state_revision_after,
                    hit_info,
                    victim_state,
                    original_damage,
                    absorbed: absorbed_damage,
                    mana_spent,
                    absorb_consumptions: absorb_consumptions.clone(),
                },
            );
        } else {
            if !creature_victim_avoided {
                outcome.canonical_creature_hits += 1;
            }
            outcome.plan.events.extend(events);
            if victim_health_state_revision_after != victim_health_state_revision_before {
                creature_victim_syncs.push(CreatureVictimCompatibilitySyncLikeCpp {
                    swing,
                    state: CreatureMeleeVictimSyncStateLikeCpp {
                        applied_damage: victim_applied_damage,
                        victim_health_before,
                        victim_health_after,
                        victim_health_state_revision_before,
                        victim_health_state_revision_after,
                        identity: victim_creature_sync_identity
                            .expect("creature victim commits carry incarnation authority"),
                    },
                });
            }
        }
    }

    // Multiple attackers can commit against one creature during a single
    // batch. Chain their contiguous health revisions so canonical authority is
    // checked once against the final desired tuple, then replay each committed
    // transition into the legacy mirror in FIFO order.
    let mut creature_victim_sync_chains: Vec<CreatureVictimCompatibilitySyncChainLikeCpp> =
        Vec::new();
    for sync in creature_victim_syncs {
        let contiguous = creature_victim_sync_chains.iter_mut().find(|chain| {
            let existing = chain
                .states
                .last()
                .expect("creature victim sync chains are never empty");
            chain.swing.map_id == sync.swing.map_id
                && chain.swing.instance_id == sync.swing.instance_id
                && chain.swing.victim_guid == sync.swing.victim_guid
                && existing.victim_health_after == sync.state.victim_health_before
                && existing.victim_health_state_revision_after
                    == sync.state.victim_health_state_revision_before
                && existing.identity.death_state_after == sync.state.identity.death_state_before
                && existing.identity.ai_state_after == sync.state.identity.ai_state_before
                && existing.identity.loot_lifecycle_revision_after
                    == sync.state.identity.loot_lifecycle_revision_before
                && existing.identity.spawn_id == sync.state.identity.spawn_id
                && existing
                    .identity
                    .authority
                    .shares_storage_like_cpp(&sync.state.identity.authority)
                && existing
                    .identity
                    .health_state_revision_authority
                    .shares_storage_like_cpp(&sync.state.identity.health_state_revision_authority)
        });
        if let Some(chain) = contiguous {
            chain.states.push(sync.state);
        } else {
            creature_victim_sync_chains.push(CreatureVictimCompatibilitySyncChainLikeCpp {
                swing: sync.swing,
                states: vec![sync.state],
            });
        }
    }

    // Lock order is canonical -> legacy, matching the existing spell-cast
    // validation bridge. Holding canonical authority through the legacy CAS
    // closes the final window where a heal/death/respawn could otherwise make
    // this mirror write stale.
    for chain in creature_victim_sync_chains {
        let desired = chain
            .states
            .last()
            .expect("creature victim sync chains are never empty");
        let Ok(mut canonical_manager) = canonical_map_manager.lock() else {
            outcome.legacy_creature_victim_sync_cas_rejections += 1;
            continue;
        };
        let canonical_is_desired = canonical_manager
            .find_map_mut(u32::from(chain.swing.map_id), chain.swing.instance_id)
            .and_then(|managed| {
                managed
                    .map()
                    .with_creature_like_cpp(chain.swing.victim_guid, |victim| {
                        let unit = victim.unit();
                        let identity = &desired.identity;
                        unit.data().health == desired.victim_health_after
                            && unit.death_state() == identity.death_state_after
                            && unit.health_state_revision_like_cpp()
                                == desired.victim_health_state_revision_after
                            && victim.spawn_id() == identity.spawn_id
                            && victim.loot_lifecycle_revision_like_cpp()
                                == identity.loot_lifecycle_revision_after
                            && victim.ai_ownership().state == identity.ai_state_after
                            && victim
                                .loot_authority_like_cpp()
                                .shares_storage_like_cpp(&identity.authority)
                            && victim
                                .unit()
                                .shares_health_state_revision_authority_like_cpp(
                                    &identity.health_state_revision_authority,
                                )
                            && victim.loot_authority_like_cpp().lifecycle_like_cpp()
                                != OwnedLootAuthorityLifecycle::Detached
                    })
            })
            .unwrap_or(false);
        if !canonical_is_desired {
            outcome.legacy_creature_victim_sync_cas_rejections += 1;
            continue;
        }

        let mut legacy_manager = legacy_map_manager
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let Some(victim) = legacy_manager.find_creature_mut(
            chain.swing.map_id,
            chain.swing.instance_id,
            chain.swing.victim_guid,
        ) else {
            outcome.legacy_creature_victim_sync_cas_rejections += 1;
            continue;
        };
        let game_time_secs = wow_entities::game_time_secs_like_cpp();
        for state in &chain.states {
            if apply_creature_melee_victim_sync_to_legacy_like_cpp(victim, state, game_time_secs) {
                outcome.legacy_creature_victim_syncs += 1;
            } else {
                outcome.legacy_creature_victim_sync_cas_rejections += 1;
                break;
            }
        }
    }

    outcome
}
