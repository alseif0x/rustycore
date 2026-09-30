//! Original AI preparation; all primaries prepare before any queue consumption.
use super::*;

pub(super) fn combat(
    creature: &mut WorldCreature,
    guid: ObjectGuid,
    recipient_guid: ObjectGuid,
    map_id: u16,
    instance_id: u32,
    difficulty_id: u8,
    policies: &mut SpellPolicies<'_>,
    outcome: &mut SpellOutcome,
    pending_actions: &mut Vec<SpellAction>,
) {
    let spells = creature.creature.spells();

    if difficulty_id != 0 && spells.iter().any(|spell_id| *spell_id != 0) {
        // Effects/range/cooldowns have difficulty-aware stores,
        // but cast time, focus and SpellPower rows are still
        // hydrated only from difficulty 0. Do not claim a
        // successful active-difficulty cast from mixed metadata.
        outcome.spell_effects_unrepresented += 1;
        return;
    }
    if has_noninstant_spell(&spells, difficulty_id, policies) {
        outcome.noninstant_casts_unrepresented += 1;
        return;
    }
    if !creature.creature_spell_schedule_initialized_like_cpp() {
        creature.mark_creature_spell_schedule_initialized_like_cpp();
        outcome.schedules_initialized += 1;
        'spell_slots: for (slot, spell_id) in spells.into_iter().enumerate() {
            if spell_id == 0 {
                continue;
            }
            let Some(spell) = i32::try_from(spell_id)
                .ok()
                .and_then(|spell_id| (policies.info)(spell_id as u32, difficulty_id, true))
            else {
                outcome.missing_spell_metadata += 1;
                continue;
            };

            match condition(spell_id, difficulty_id, policies) {
                SpellCondition::Aggro => {
                    // C++ `CombatAI::JustEngagedWith` casts
                    // AICOND_AGGRO directly on `who`, bypassing
                    // `UnitAI::DoCast` target classification.
                    match disable(spell_id, map_id, creature, policies) {
                        SpellDisable::Enabled => {}
                        SpellDisable::Disabled => {
                            outcome.spells_disabled += 1;
                            continue;
                        }
                        SpellDisable::Unrepresented => {
                            outcome.spell_disable_context_unrepresented += 1;
                            creature.record_swing();
                            break 'spell_slots;
                        }
                    }
                    if !(policies.check)(
                        SpellPreparationCheck::RuntimeHooks,
                        spell_id,
                        difficulty_id,
                    ) {
                        outcome.spell_runtime_hooks_unrepresented += 1;
                        creature.record_swing();
                        break 'spell_slots;
                    }
                    if !(policies.check)(
                        SpellPreparationCheck::CastingRequirements,
                        spell_id,
                        difficulty_id,
                    ) {
                        outcome.spell_casting_requirements_unrepresented += 1;
                        creature.record_swing();
                        break 'spell_slots;
                    }
                    if !(policies.check)(
                        SpellPreparationCheck::ShapeshiftRequirements,
                        spell_id,
                        difficulty_id,
                    ) {
                        outcome.spell_casting_requirements_unrepresented += 1;
                        creature.record_swing();
                        break 'spell_slots;
                    }
                    if !(policies.check)(
                        SpellPreparationCheck::AuraRestrictions,
                        spell_id,
                        difficulty_id,
                    ) {
                        outcome.spell_effects_unrepresented += 1;
                        creature.record_swing();
                        break 'spell_slots;
                    }
                    if !cooldown_semantics(spell_id, difficulty_id, policies) {
                        outcome.spell_effects_unrepresented += 1;
                        creature.record_swing();
                        break 'spell_slots;
                    }
                    if combat_forbidden(spell_id, difficulty_id, policies) {
                        outcome.spell_effects_unrepresented += 1;
                        creature.record_swing();
                        break 'spell_slots;
                    }
                    if implicit_cost(&spell, difficulty_id, policies, creature) {
                        outcome.spell_effects_unrepresented += 1;
                        creature.record_swing();
                        break 'spell_slots;
                    }
                    if target_restrictions(spell_id, difficulty_id, policies) {
                        outcome.spell_effects_unrepresented += 1;
                        creature.record_swing();
                        break 'spell_slots;
                    }
                    match single_unit_topology(
                        &spell,
                        recipient_guid,
                        recipient_guid,
                        projectile(spell_id, difficulty_id, policies),
                    ) {
                        Ok(()) => {}
                        Err(SpellTopologyError::NonInstant) => {
                            outcome.noninstant_casts_unrepresented += 1;
                            creature.record_swing();
                            break 'spell_slots;
                        }
                        Err(SpellTopologyError::ProjectileOrAmmo) => {
                            outcome.spell_projectiles_unrepresented += 1;
                            creature.record_swing();
                            break 'spell_slots;
                        }
                        Err(SpellTopologyError::EffectOrTarget) => {
                            outcome.spell_effects_unrepresented += 1;
                            creature.record_swing();
                            break 'spell_slots;
                        }
                    }
                    let Ok(command) = cast_plan(
                        creature,
                        guid,
                        recipient_guid,
                        map_id,
                        instance_id,
                        spell_id,
                        &spell,
                        difficulty_id,
                        policies,
                    ) else {
                        outcome.spell_visuals_unrepresented += 1;
                        creature.record_swing();
                        break 'spell_slots;
                    };
                    pending_actions.push(SpellAction::Cast(SpellCast {
                        command,
                        difficulty_id,
                        turret_ai: false,
                    }));
                    outcome.casts_ready += 1;
                }
                SpellCondition::Combat => {
                    // Preserve the C++ slot order: an earlier
                    // AICOND_AGGRO cast (and its hit roll) runs
                    // before a later AICOND_COMBAT schedule
                    // draws its randomized initial delay.
                    let minimum = minimum(spell_id, &spell, difficulty_id, policies);
                    pending_actions.push(SpellAction::Schedule(SpellSchedule {
                        caster_guid: guid,
                        map_id,
                        instance_id,
                        engagement_epoch: creature.creature_spell_engagement_epoch_like_cpp(),
                        slot,
                        minimum_ms: minimum,
                    }));
                }
                SpellCondition::Die => {}
            }
        }
    }

    // C++ advances EventMap before this gate but executes no
    // due event while the creature owns UNIT_STATE_CASTING.
    if creature
        .creature
        .unit()
        .has_unit_state(UnitState::CASTING.bits())
    {
        outcome.unit_state_casting_skips += 1;
        return;
    }

    if let Some(slot) = creature.first_due_creature_spell_slot_like_cpp() {
        // C++ `EventMap::ExecuteEvent` removes the due event
        // before `DoCast`. A represented repeat schedule below
        // rearms it; an RNG tombstone deliberately leaves it
        // absent instead of retrying the same event every tick.
        creature.clear_creature_spell_slot_like_cpp(slot);
        let spell_id = spells[slot];
        let Some(spell) = i32::try_from(spell_id)
            .ok()
            .and_then(|spell_id| (policies.info)(spell_id as u32, difficulty_id, true))
        else {
            outcome.missing_spell_metadata += 1;
            return;
        };

        let target_kind = target(spell_id, &spell, difficulty_id, policies);
        if target_kind.requires_random_threat_selection() {
            // C++ Enemy/Debuff selection draws from the full
            // non-offline threat list before CastSpell. This
            // slice cannot yet prove every candidate's range,
            // aura and player/NPC filters. Stop exact spell RNG
            // accreditation instead of selecting the victim or
            // drawing the later repeat delay out of order.
            outcome.spell_effects_unrepresented += 1;
            creature.invalidate_runtime_rng_authority_like_cpp();
            return;
        }
        let target_guid = target_kind.resolve_single_player(guid, recipient_guid);

        // `CombatAI::UpdateAI` re-schedules after every
        // `DoCast` attempt, including a failed one. Defer the
        // delay draw until after a represented hit roll so the
        // shared creature RNG follows C++ DoCast -> ScheduleEvent.
        let minimum = minimum(spell_id, &spell, difficulty_id, policies);
        let schedule = SpellSchedule {
            caster_guid: guid,
            map_id,
            instance_id,
            engagement_epoch: creature.creature_spell_engagement_epoch_like_cpp(),
            slot,
            minimum_ms: minimum,
        };

        match disable(spell_id, map_id, creature, policies) {
            SpellDisable::Enabled => {}
            SpellDisable::Disabled => {
                outcome.spells_disabled += 1;
                pending_actions.push(SpellAction::Schedule(schedule));
                return;
            }
            SpellDisable::Unrepresented => {
                outcome.spell_disable_context_unrepresented += 1;
                pending_actions.push(SpellAction::Schedule(schedule));
                return;
            }
        }
        if !(policies.check)(SpellPreparationCheck::RuntimeHooks, spell_id, difficulty_id) {
            outcome.spell_runtime_hooks_unrepresented += 1;
            pending_actions.push(SpellAction::Schedule(schedule));
            return;
        }
        if !(policies.check)(
            SpellPreparationCheck::CastingRequirements,
            spell_id,
            difficulty_id,
        ) {
            outcome.spell_casting_requirements_unrepresented += 1;
            pending_actions.push(SpellAction::Schedule(schedule));
            return;
        }
        if !(policies.check)(
            SpellPreparationCheck::ShapeshiftRequirements,
            spell_id,
            difficulty_id,
        ) {
            outcome.spell_casting_requirements_unrepresented += 1;
            pending_actions.push(SpellAction::Schedule(schedule));
            return;
        }
        if !(policies.check)(
            SpellPreparationCheck::AuraRestrictions,
            spell_id,
            difficulty_id,
        ) {
            outcome.spell_effects_unrepresented += 1;
            pending_actions.push(SpellAction::Schedule(schedule));
            return;
        }
        if !cooldown_semantics(spell_id, difficulty_id, policies) {
            outcome.spell_effects_unrepresented += 1;
            pending_actions.push(SpellAction::Schedule(schedule));
            return;
        }
        if combat_forbidden(spell_id, difficulty_id, policies) {
            outcome.spell_effects_unrepresented += 1;
            pending_actions.push(SpellAction::Schedule(schedule));
            return;
        }
        if implicit_cost(&spell, difficulty_id, policies, creature) {
            outcome.spell_effects_unrepresented += 1;
            pending_actions.push(SpellAction::Schedule(schedule));
            return;
        }
        if target_restrictions(spell_id, difficulty_id, policies) {
            outcome.spell_effects_unrepresented += 1;
            pending_actions.push(SpellAction::Schedule(schedule));
            return;
        }
        match single_unit_topology(
            &spell,
            target_guid,
            recipient_guid,
            projectile(spell_id, difficulty_id, policies),
        ) {
            Ok(()) => {}
            Err(SpellTopologyError::NonInstant) => {
                outcome.noninstant_casts_unrepresented += 1;
                pending_actions.push(SpellAction::Schedule(schedule));
                return;
            }
            Err(SpellTopologyError::ProjectileOrAmmo) => {
                outcome.spell_projectiles_unrepresented += 1;
                pending_actions.push(SpellAction::Schedule(schedule));
                return;
            }
            Err(SpellTopologyError::EffectOrTarget) => {
                outcome.spell_effects_unrepresented += 1;
                pending_actions.push(SpellAction::Schedule(schedule));
                return;
            }
        }
        let Ok(command) = cast_plan(
            creature,
            guid,
            target_guid,
            map_id,
            instance_id,
            spell_id,
            &spell,
            difficulty_id,
            policies,
        ) else {
            outcome.spell_visuals_unrepresented += 1;
            pending_actions.push(SpellAction::Schedule(schedule));
            return;
        };
        pending_actions.push(SpellAction::Cast(SpellCast {
            command,
            difficulty_id,
            turret_ai: false,
        }));
        pending_actions.push(SpellAction::Schedule(schedule));
        outcome.casts_ready += 1;
    }
}
