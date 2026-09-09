//! Validation and append of a legacy creature spell cast plan.
//!
//! Moved out of the Session root under #619. Behaviour is preserved.

use super::*;

pub(in crate::session) fn validate_and_append_creature_spell_cast_like_cpp(
    canonical_map_manager: &SharedCanonicalMapManager,
    legacy_map_manager: &crate::map_manager::SharedMapManager,
    command: &CreatureSpellCastPlanLikeCpp,
    difficulty_id: u8,
    turret_ai: bool,
    config: &LegacyCreatureAggroConfigLikeCpp,
    plan: &mut RuntimePlan,
) -> CreatureSpellCastValidationResultLikeCpp {
    const SPELL_RANGE_MELEE_LIKE_CPP: u8 = 0x01;
    const SPELL_RANGE_RANGED_LIKE_CPP: u8 = 0x02;

    let Ok(mut manager) = canonical_map_manager.lock() else {
        return CreatureSpellCastValidationResultLikeCpp::MissingTarget;
    };
    // The canonical update path already establishes canonical -> legacy when
    // mirroring loaded grids. Use that same order so target/epoch validation,
    // TurretAI timer mutation and packet-plan construction are serialized
    // without lock inversion.
    let mut legacy_guard = legacy_map_manager
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let Some(legacy_caster) =
        legacy_guard.find_creature(command.map_id, command.instance_id, command.caster_guid)
    else {
        return CreatureSpellCastValidationResultLikeCpp::MissingTarget;
    };
    if !legacy_caster.is_alive()
        || legacy_caster.state() != wow_entities::CreatureAiState::InCombat
        || legacy_caster.creature.ai_ownership().combat_target != Some(command.target_guid)
        || legacy_caster.creature_spell_engagement_epoch_like_cpp() != command.engagement_epoch
    {
        return CreatureSpellCastValidationResultLikeCpp::MissingTarget;
    }
    // The planning phase read this creature before the canonical tick could
    // replace it. Prove the live legacy creature is still that incarnation
    // before any cooldown, timer or RNG state is consumed from it.
    if !command
        .caster_incarnation
        .matches_like_cpp(&legacy_caster.creature)
    {
        return CreatureSpellCastValidationResultLikeCpp::CasterIncarnationRejected;
    }
    let cooldown_now_ms = legacy_caster.runtime_elapsed_ms_like_cpp();
    let cooldown_profile = u32::try_from(command.spell_id).ok().and_then(|spell_id| {
        creature_ai_spell_cooldown_profile_like_cpp(spell_id, difficulty_id, config)
    });
    let caster_hit_aura_sources_are_empty = legacy_caster
        .creature
        .unit()
        .subsystems()
        .auras
        .has_complete_spell_hit_inert_aura_authority_like_cpp();
    let caster_has_no_owner_or_charmer = {
        let control = &legacy_caster.creature.unit().subsystems().control;
        control.owner_guid.is_none()
            && control.charmer_guid.is_none()
            && !control.controlled_by_player
            // C++ CanHaveGlobalCooldown treats any Creature with CharmInfo as
            // controlled even if its owner/charmer GUIDs are momentarily empty.
            // Keep that GCD-bearing surface outside this stock-AI slice.
            && !control.has_charm_info()
    };
    let Some(managed) = manager.find_map(u32::from(command.map_id), command.instance_id) else {
        return CreatureSpellCastValidationResultLikeCpp::MissingTarget;
    };
    let (hit_profile, source_position, visibility_range, full_log_data) = match {
        let map = managed.map();
        map.with_creature_like_cpp(command.caster_guid, |caster| {
            // A canonical replacement can hold the same GUID as the legacy creature
            // the plan was built from. Validating, starting cooldowns and publishing
            // START/GO from it while the timers and RNG come from the stale legacy
            // creature would mix two incarnations, so require the same identity here
            // too.
            if !command.caster_incarnation.matches_like_cpp(caster) {
                return Err(CreatureSpellCastValidationResultLikeCpp::CasterIncarnationRejected);
            }
            let Some(victim) = map.get_typed_player(command.target_guid) else {
                return Err(CreatureSpellCastValidationResultLikeCpp::MissingTarget);
            };
            if !caster.unit().is_alive() || !victim.unit().is_alive() {
                return Err(CreatureSpellCastValidationResultLikeCpp::MissingTarget);
            }
            // C++ constructs `MessageDistDeliverer` from the live caster during
            // `SendSpellGo`; it does not reuse the earlier AI-selection position.
            // Snapshot both fanout inputs from the same canonical caster whose
            // range, LOS and hit preconditions are validated below.
            let source_position = caster.unit().world().position();
            let visibility_range = caster.unit().world().get_visibility_range(map);
            let Some(spell_id_u32) = u32::try_from(command.spell_id).ok() else {
                return Err(CreatureSpellCastValidationResultLikeCpp::MissingTarget);
            };
            let Some(range) =
                creature_ai_effective_spell_range_like_cpp(spell_id_u32, difficulty_id, config)
            else {
                return Err(CreatureSpellCastValidationResultLikeCpp::OutOfRange);
            };

            let caster_reach = caster.unit().world().combat_reach().max(0.0);
            let victim_reach = victim.unit().world().combat_reach().max(0.0);
            let reach_sum = caster_reach + victim_reach;
            let melee_range = (reach_sum + 4.0 / 3.0).max(NOMINAL_MELEE_RANGE_LIKE_CPP);
            let mut minimum = range.range_min[0].max(0.0);
            let mut maximum = range.range_max[0].max(0.0);
            let turret_combat_maximum = maximum + reach_sum;
            if range.flags & SPELL_RANGE_MELEE_LIKE_CPP != 0 {
                minimum = 0.0;
                maximum = melee_range;
            } else {
                if range.flags & SPELL_RANGE_RANGED_LIKE_CPP != 0 {
                    minimum += melee_range;
                } else if minimum > 0.0 {
                    minimum += reach_sum;
                }
                maximum += reach_sum;
            }
            let caster_movement = caster.unit().movement_flags_like_cpp();
            let victim_movement = victim.unit().movement_flags_like_cpp();
            if caster_movement.intersects(MovementFlag::MASK_MOVING)
                && victim_movement.intersects(MovementFlag::MASK_MOVING)
                && !caster_movement.contains(MovementFlag::WALKING)
                && !victim_movement.contains(MovementFlag::WALKING)
            {
                // C++ `Spell::GetMinMaxRange(true)` adds this latency allowance
                // when both units run and the target is a player.
                maximum += 8.0 / 3.0;
            }
            let distance_sq = caster
                .unit()
                .world()
                .position()
                .distance_sq(&victim.unit().world().position());
            // `TurretAI::DoSpellAttackIfReady` first uses strict
            // `IsWithinCombatRange` with the raw spell max plus combat reaches;
            // `Spell::CheckRange(true)` (including moving allowance) follows.
            if turret_ai && distance_sq >= turret_combat_maximum * turret_combat_maximum {
                return Err(CreatureSpellCastValidationResultLikeCpp::OutOfRange);
            }
            if turret_ai {
                let Some(creature) = legacy_guard.find_creature_mut(
                    command.map_id,
                    command.instance_id,
                    command.caster_guid,
                ) else {
                    return Err(CreatureSpellCastValidationResultLikeCpp::MissingTarget);
                };
                // `UnitAI::DoSpellAttackIfReady` calls `CastSpell` after only the
                // raw-max combat-range gate, then resets BASE_ATTACK regardless of
                // whether Spell::CheckRange/CheckCast rejects min range or LOS.
                creature.record_swing();
            }
            if cooldown_profile.is_some_and(|profile| {
                !profile.passive
                    && caster.unit().subsystems().spells.history.has_cooldown(
                        profile.spell_id,
                        profile.category_id,
                        cooldown_now_ms,
                    )
            }) {
                // C++ Spell::CheckCast rejects this attempt before hit resolution.
                // CombatAI still executes its following ScheduleEvent, and
                // TurretAI has already reset BASE_ATTACK after calling CastSpell.
                return Err(CreatureSpellCastValidationResultLikeCpp::CooldownRejected);
            }
            if distance_sq > maximum * maximum || (minimum > 0.0 && distance_sq < minimum * minimum)
            {
                return Err(CreatureSpellCastValidationResultLikeCpp::OutOfRange);
            }
            let effective_attributes = config.spell_store.as_ref().and_then(|store| {
                store.misc_attributes_for_difficulty_like_cpp(
                    command.spell_id,
                    difficulty_id,
                    config.difficulty_store.as_deref(),
                )
            });
            let ignores_line_of_sight = effective_attributes.is_some_and(|attributes| {
                attributes[2] & wow_data::spell::attributes::SPELL_ATTR2_IGNORE_LINE_OF_SIGHT != 0
            });
            if !ignores_line_of_sight
                && !caster.unit().world().is_within_los_in_map(
                    victim.unit().world(),
                    map,
                    wow_entities::LineOfSightOptions::default(),
                )
            {
                return Err(CreatureSpellCastValidationResultLikeCpp::LosRejected);
            }

            if let Some(attributes) = effective_attributes
                && !creature_spell_target_is_valid_attack_target_like_cpp(
                    caster,
                    victim,
                    &attributes,
                    config,
                )
            {
                // C++ Spell::CheckCast rejects this before melee hit resolution.
                // Preserve the queued CombatAI repeat and exact RNG authority so
                // its following ScheduleEvent can draw the next delay.
                return Err(CreatureSpellCastValidationResultLikeCpp::TargetRejected);
            }

            let represented_cast = (|| {
                let spell_store = config.spell_store.as_ref()?;
                let spell = spell_store.get(command.spell_id)?;
                let metadata = spell_store.hit_metadata_for_difficulty_like_cpp(
                    command.spell_id,
                    difficulty_id,
                    config.difficulty_store.as_deref(),
                )?;
                let attributes = effective_attributes?;
                let active_effect_indices: Vec<u32> = spell
                    .effects()
                    .iter()
                    .filter(|effect| {
                        effect.effect != 0
                            && !wow_data::spell::spell_effect_types::is_cpp_null_or_unused_noop(
                                effect.effect,
                            )
                    })
                    .map(|effect| effect.effect_index)
                    .collect();
                let victim_hit_aura_sources_are_hit_inert = victim
                    .unit()
                    .subsystems()
                    .auras
                    .has_complete_spell_hit_inert_aura_authority_like_cpp();
                let target_has_no_vehicle_kit = victim.unit().subsystems().vehicle.kit.is_none();
                let caster_is_behind_player = !victim.unit().world().has_in_arc(
                    std::f32::consts::PI,
                    caster.unit().world(),
                    2.0,
                );
                if !caster_hit_aura_sources_are_empty
                    || !caster_has_no_owner_or_charmer
                    || !victim_hit_aura_sources_are_hit_inert
                    || !target_has_no_vehicle_kit
                    || !caster_is_behind_player
                {
                    return None;
                }
                let hit_profile = represented_creature_spell_hit_profile_like_cpp(
                    &metadata,
                    &active_effect_indices,
                    attributes,
                )?;
                Some((
                    hit_profile,
                    creature_spell_cast_log_data_like_cpp(caster, spell, difficulty_id),
                ))
            })();
            let (hit_profile, full_log_data) = represented_cast
                .map_or((None, None), |(profile, log_data)| {
                    (Some(profile), log_data)
                });
            Ok((
                hit_profile,
                source_position,
                visibility_range,
                full_log_data,
            ))
        })
    } {
        Some(Ok(prepared)) => prepared,
        Some(Err(result)) => return result,
        None => return CreatureSpellCastValidationResultLikeCpp::MissingTarget,
    };
    if !turret_ai
        && creature_ai_successful_untriggered_spell_resets_combat_timers_like_cpp(
            command,
            difficulty_id,
            config,
        )
    {
        let Some(creature) = legacy_guard.find_creature_mut(
            command.map_id,
            command.instance_id,
            command.caster_guid,
        ) else {
            return CreatureSpellCastValidationResultLikeCpp::MissingTarget;
        };
        // `Spell::ResetCombatTimers` rearms BASE_ATTACK before the same map
        // update can reach `DoMeleeAttackIfReady` (Spell.cpp:8363-8372).
        creature.record_swing();
    }
    let Some(hit_profile) = hit_profile else {
        let Some(creature) = legacy_guard.find_creature_mut(
            command.map_id,
            command.instance_id,
            command.caster_guid,
        ) else {
            return CreatureSpellCastValidationResultLikeCpp::MissingTarget;
        };
        // At this point C++ may have returned before its melee hit roll
        // (immunity/reflection) or after consuming it (avoidance/facing). The
        // exact shared-RNG position is therefore unknowable. Tombstone exact
        // creature-spell RNG accreditation; the pre-existing transitional
        // melee and movement runtimes remain available as best-effort work.
        creature.invalidate_runtime_rng_authority_like_cpp();
        return CreatureSpellCastValidationResultLikeCpp::HitResultUnrepresented;
    };
    let Some(full_log_data) = full_log_data else {
        let Some(creature) = legacy_guard.find_creature_mut(
            command.map_id,
            command.instance_id,
            command.caster_guid,
        ) else {
            return CreatureSpellCastValidationResultLikeCpp::MissingTarget;
        };
        creature.invalidate_runtime_rng_authority_like_cpp();
        return CreatureSpellCastValidationResultLikeCpp::HitResultUnrepresented;
    };
    let Some(creature) =
        legacy_guard.find_creature_mut(command.map_id, command.instance_id, command.caster_guid)
    else {
        return CreatureSpellCastValidationResultLikeCpp::MissingTarget;
    };
    let Some(roll) = creature.random_creature_spell_hit_roll_like_cpp() else {
        return CreatureSpellCastValidationResultLikeCpp::RuntimeRngAuthorityRejected;
    };
    let Some(hit_result) = resolve_creature_spell_hit_profile_like_cpp(hit_profile, Some(roll))
    else {
        let Some(creature) = legacy_guard.find_creature_mut(
            command.map_id,
            command.instance_id,
            command.caster_guid,
        ) else {
            return CreatureSpellCastValidationResultLikeCpp::MissingTarget;
        };
        creature.invalidate_runtime_rng_authority_like_cpp();
        return CreatureSpellCastValidationResultLikeCpp::HitResultUnrepresented;
    };
    if let Some(profile) = cooldown_profile
        && !profile.passive
        && (profile.recovery_time_ms != 0 || profile.category_recovery_time_ms != 0)
    {
        let Some(managed) = manager.find_map_mut(u32::from(command.map_id), command.instance_id)
        else {
            return CreatureSpellCastValidationResultLikeCpp::MissingTarget;
        };
        let Some(canonical_caster) = managed
            .map_mut()
            .get_typed_creature_mut(command.caster_guid)
        else {
            return CreatureSpellCastValidationResultLikeCpp::MissingTarget;
        };
        // C++ SendSpellCooldown runs before SendSpellGo and starts both the
        // spell and shared-category deadlines for any successful cast,
        // regardless of whether its target later hits or misses.
        canonical_caster
            .unit_mut()
            .subsystems_mut()
            .spells
            .history
            .start_cooldown(
                cooldown_now_ms,
                profile.spell_id,
                0,
                profile.recovery_time_ms,
                profile.category_id,
                profile.category_recovery_time_ms,
                false,
            );
    }
    // Validation and allocation share the already-held canonical manager guard.
    // Player and creature casts use one Map sequence, never process-local IDs.
    let counter = manager
        .find_map_mut(u32::from(command.map_id), command.instance_id)
        .expect("validated map remains present under its manager guard")
        .map_mut()
        .generate_low_guid_like_cpp(HighGuid::Cast)
        .expect("Cast is a supported map GUID sequence");
    let cast_id = represented_spell_cast_guid_for_map_like_cpp(
        command.caster_guid.realm_id(),
        command.map_id,
        command.spell_id,
        counter,
    );
    append_committed_creature_spell_packets_like_cpp(
        plan,
        command,
        cast_id,
        hit_result,
        source_position,
        visibility_range,
        &full_log_data,
    );
    if hit_result == CreatureSpellTargetHitResultLikeCpp::Hit {
        // C++ enters HandleLaunchPhase before SendSpellGo. Every HIT target
        // consumes at least the unconditional `roll_chance_f(critChance)` in
        // PreprocessSpellLaunch, followed by effect-specific value/variance
        // draws that this wire-only slice does not own. Publish the already
        // resolved HIT topology, then stop later creature-spell schedules or
        // hit results from claiming an exact shared-RNG position. Transitional
        // melee and movement continue best-effort instead of freezing gameplay.
        creature.invalidate_runtime_rng_authority_like_cpp();
    }
    CreatureSpellCastValidationResultLikeCpp::Ready(hit_result)
}
