//! Legacy creature spell tick and the cast validation it appends.
//!
//! Moved out of the Session root under #619. Behaviour is preserved; the
//! canonical owner of this state is unchanged.
//!
//! #1263 F6-8D3a-1b: the selection half of the phase
//! ([`collect_creature_spell_actions_on_store_like_cpp`]) is written once
//! against [`CreaturePhaseStoreLikeCpp`], so the legacy bridge and the admitted
//! canonical executor run the same body; the deferred actions it returns are
//! drained by `creature_spell_actions.rs`. The legacy entry keeps only the
//! guard, the owner check and the lock scopes.

use super::*;

/// Runs the C++ stock `CombatAI`/`TurretAI` template-spell decision once.
///
/// Selection, cooldown mutation, target preconditions, and START/GO packet
/// construction stay under the global map-owned runtime. Effect execution is
/// intentionally not invented here: damage/heal calculation belongs to M3.2,
/// while this M2.6 slice proves the cast decision and wire topology.
pub fn run_legacy_creature_spell_tick_once_like_cpp(
    legacy_map_manager: &crate::map_manager::SharedMapManager,
    canonical_map_manager: Option<&SharedCanonicalMapManager>,
    config: &LegacyCreatureAggroConfigLikeCpp,
) -> LegacyCreatureSpellTickOutcomeLikeCpp {
    use crate::map_manager::RuntimeTickOwner;

    let mut outcome = LegacyCreatureSpellTickOutcomeLikeCpp::default();
    let Some(spell_store) = config.spell_store.as_ref() else {
        return outcome;
    };
    let mut map_difficulties = HashMap::new();
    if let Some(canonical_map_manager) = canonical_map_manager
        && let Ok(manager) = canonical_map_manager.lock()
    {
        map_difficulties = creature_spell_map_difficulties_like_cpp(&manager);
    }
    // #1263 F6-8C: the canonical designated owner decides which creature may
    // select a template spell. Read once, before the legacy write guard.
    let ownership =
        canonical_creature_ownership_like_cpp(canonical_map_manager, legacy_map_manager);
    let mut manager = legacy_map_manager
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if manager.tick_owner() != RuntimeTickOwner::GlobalLegacy {
        outcome.skipped_owner_not_global = true;
        return outcome;
    }
    let pending_actions = collect_creature_spell_actions_on_store_like_cpp(
        &mut *manager,
        &ownership,
        &map_difficulties,
        spell_store,
        config,
        &mut outcome,
    );
    drop(manager);
    drain_creature_spell_actions_like_cpp(
        &mut LegacyCreatureSpellActionOwnerLikeCpp {
            legacy_map_manager,
            canonical_map_manager,
        },
        &ownership,
        pending_actions,
        config,
        &mut outcome,
    );
    outcome
}

/// The difficulty of every canonical map, keyed like the legacy residences.
pub(super) fn creature_spell_map_difficulties_like_cpp(
    manager: &wow_map::MapManager,
) -> HashMap<(u16, u32), u8> {
    let mut map_difficulties = HashMap::new();
    manager.do_for_all_maps(|managed| {
        if let Ok(map_id) = u16::try_from(managed.map_id()) {
            map_difficulties.insert((map_id, managed.instance_id()), managed.difficulty());
        }
    });
    map_difficulties
}

/// The selection half of `CombatAI::UpdateAI` / `TurretAI::UpdateAI`
/// (`CombatAI.cpp:91-107, 217-223`): for every creature of the store, the
/// schedule initialisation, the due-event choice and the represented cast
/// plan, deferred as actions in C++ order. #1263 F6-8D3a-1b: one body for the
/// legacy store and the admitted canonical store.
pub(super) fn collect_creature_spell_actions_on_store_like_cpp<S>(
    store: &mut S,
    ownership: &CanonicalCreatureOwnershipLikeCpp,
    map_difficulties: &HashMap<(u16, u32), u8>,
    spell_store: &wow_data::SpellStore,
    config: &LegacyCreatureAggroConfigLikeCpp,
    outcome: &mut LegacyCreatureSpellTickOutcomeLikeCpp,
) -> Vec<PendingCreatureSpellActionLikeCpp>
where
    S: CreaturePhaseStoreLikeCpp + ?Sized,
{
    let mut pending_actions = Vec::new();
    let map_keys = store.phase_map_keys_like_cpp();
    outcome.maps_seen = map_keys.len();
    for (map_id, instance_id) in map_keys {
        let difficulty_id = map_difficulties
            .get(&(map_id, instance_id))
            .copied()
            .unwrap_or(0);
        for guid in store.phase_creature_guids_like_cpp(map_id, instance_id) {
            let Some(creature) = store.phase_creature_mut_like_cpp(map_id, instance_id, guid)
            else {
                continue;
            };
            outcome.creatures_seen += 1;
            // #1263 F6-8C: a creature whose exact residence has no canonical
            // incarnation is not the canonical owner's object, so it selects no
            // spell from the legacy representation.
            if !ownership.decides_like_cpp(map_id, instance_id, guid) {
                outcome.canonical_incarnation_rejections += 1;
                continue;
            }
            if !creature.is_alive() || creature.state() != wow_entities::CreatureAiState::InCombat {
                continue;
            }
            let Some(recipient_guid) = creature.ai_ownership().combat_target else {
                continue;
            };
            // Creature-vs-creature effects need a future map-owned generic
            // spell executor and are intentionally not fabricated here.
            if !recipient_guid.is_player() {
                continue;
            }

            let ai_kind = match legacy_creature_ai_selection_decision_like_cpp(creature, config) {
                LegacyCreatureAiSelectionDecisionLikeCpp::Selected(ai_kind) => ai_kind,
                LegacyCreatureAiSelectionDecisionLikeCpp::ScriptRegistryUnrepresented => {
                    outcome.ai_selection_unrepresented += 1;
                    continue;
                }
            };
            let spells = creature.spells();

            match ai_kind {
                CreatureAiKindLikeCpp::CombatAI => {
                    if difficulty_id != 0 && spells.iter().any(|spell_id| *spell_id != 0) {
                        // Effects/range/cooldowns have difficulty-aware stores,
                        // but cast time, focus and SpellPower rows are still
                        // hydrated only from difficulty 0. Do not claim a
                        // successful active-difficulty cast from mixed metadata.
                        outcome.spell_effects_unrepresented += 1;
                        continue;
                    }
                    if creature_ai_has_temporally_unrepresented_noninstant_spell_like_cpp(
                        &spells,
                        difficulty_id,
                        config,
                    ) {
                        outcome.noninstant_casts_unrepresented += 1;
                        continue;
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
                                .and_then(|spell_id| spell_store.get(spell_id))
                            else {
                                outcome.missing_spell_metadata += 1;
                                continue;
                            };
                            let spell = creature_ai_effective_spell_info_like_cpp(
                                spell,
                                difficulty_id,
                                config,
                            );
                            match creature_ai_spell_condition_like_cpp(
                                spell_id,
                                difficulty_id,
                                config,
                            ) {
                                CreatureAiSpellConditionLikeCpp::Aggro => {
                                    // C++ `CombatAI::JustEngagedWith` casts
                                    // AICOND_AGGRO directly on `who`, bypassing
                                    // `UnitAI::DoCast` target classification.
                                    match creature_ai_spell_disable_decision_like_cpp(
                                        spell_id, map_id, creature, config,
                                    ) {
                                        CreatureSpellDisableDecisionLikeCpp::Enabled => {}
                                        CreatureSpellDisableDecisionLikeCpp::Disabled => {
                                            outcome.spells_disabled += 1;
                                            continue;
                                        }
                                        CreatureSpellDisableDecisionLikeCpp::ContextUnrepresented => {
                                            outcome.spell_disable_context_unrepresented += 1;
                                            creature.record_swing();
                                            break 'spell_slots;
                                        }
                                    }
                                    if !config
                                        .spell_has_no_unrepresented_runtime_hooks_like_cpp(spell_id)
                                    {
                                        outcome.spell_runtime_hooks_unrepresented += 1;
                                        creature.record_swing();
                                        break 'spell_slots;
                                    }
                                    if !config
                                        .spell_has_no_unrepresented_casting_requirements_like_cpp(
                                            spell_id,
                                        )
                                    {
                                        outcome.spell_casting_requirements_unrepresented += 1;
                                        creature.record_swing();
                                        break 'spell_slots;
                                    }
                                    if !config
                                        .spell_has_no_unrepresented_shapeshift_requirements_like_cpp(
                                            spell_id,
                                        )
                                    {
                                        outcome.spell_casting_requirements_unrepresented += 1;
                                        creature.record_swing();
                                        break 'spell_slots;
                                    }
                                    if !config
                                        .spell_has_no_unrepresented_aura_restrictions_like_cpp(
                                            spell_id,
                                            difficulty_id,
                                        )
                                    {
                                        outcome.spell_effects_unrepresented += 1;
                                        creature.record_swing();
                                        break 'spell_slots;
                                    }
                                    if !creature_ai_spell_has_represented_cooldown_semantics_like_cpp(
                                        spell_id,
                                        difficulty_id,
                                        config,
                                    ) {
                                        outcome.spell_effects_unrepresented += 1;
                                        creature.record_swing();
                                        break 'spell_slots;
                                    }
                                    if creature_ai_spell_is_combat_forbidden_like_cpp(
                                        spell_id,
                                        difficulty_id,
                                        config,
                                    ) {
                                        outcome.spell_effects_unrepresented += 1;
                                        creature.record_swing();
                                        break 'spell_slots;
                                    }
                                    if creature_ai_zero_power_rows_have_unrepresented_implicit_cost_like_cpp(
                                        &spell,
                                        difficulty_id,
                                        config,
                                        creature,
                                    ) {
                                        outcome.spell_effects_unrepresented += 1;
                                        creature.record_swing();
                                        break 'spell_slots;
                                    }
                                    if creature_ai_spell_has_unrepresented_target_restrictions_like_cpp(
                                        spell_id,
                                        difficulty_id,
                                        config,
                                    ) {
                                        outcome.spell_effects_unrepresented += 1;
                                        creature.record_swing();
                                        break 'spell_slots;
                                    }
                                    match creature_ai_spell_single_unit_topology_like_cpp(
                                        &spell,
                                        recipient_guid,
                                        recipient_guid,
                                        creature_ai_spell_requires_projectile_payload_like_cpp(
                                            spell_id,
                                            difficulty_id,
                                            config,
                                        ),
                                    ) {
                                        Ok(()) => {}
                                        Err(CreatureAiSpellRepresentationRejectionLikeCpp::NonInstant) => {
                                            outcome.noninstant_casts_unrepresented += 1;
                                            creature.record_swing();
                                            break 'spell_slots;
                                        }
                                        Err(CreatureAiSpellRepresentationRejectionLikeCpp::ProjectileOrAmmo) => {
                                            outcome.spell_projectiles_unrepresented += 1;
                                            creature.record_swing();
                                            break 'spell_slots;
                                        }
                                        Err(CreatureAiSpellRepresentationRejectionLikeCpp::EffectOrTarget) => {
                                            outcome.spell_effects_unrepresented += 1;
                                            creature.record_swing();
                                            break 'spell_slots;
                                        }
                                    }
                                    let Ok(command) = creature_ai_spell_plan_like_cpp(
                                        creature,
                                        guid,
                                        recipient_guid,
                                        map_id,
                                        instance_id,
                                        spell_id,
                                        &spell,
                                        difficulty_id,
                                        config,
                                    ) else {
                                        outcome.spell_visuals_unrepresented += 1;
                                        creature.record_swing();
                                        break 'spell_slots;
                                    };
                                    pending_actions.push(PendingCreatureSpellActionLikeCpp::Cast(
                                        PendingCreatureSpellCastLikeCpp {
                                            command,
                                            difficulty_id,
                                            turret_ai: false,
                                        },
                                    ));
                                    outcome.casts_ready += 1;
                                }
                                CreatureAiSpellConditionLikeCpp::Combat => {
                                    // Preserve the C++ slot order: an earlier
                                    // AICOND_AGGRO cast (and its hit roll) runs
                                    // before a later AICOND_COMBAT schedule
                                    // draws its randomized initial delay.
                                    let minimum = creature_ai_spell_initial_cooldown_like_cpp(
                                        spell_id,
                                        &spell,
                                        difficulty_id,
                                        config,
                                    );
                                    pending_actions.push(
                                        PendingCreatureSpellActionLikeCpp::Schedule(
                                            PendingCreatureSpellScheduleLikeCpp {
                                                caster_guid: guid,
                                                map_id,
                                                instance_id,
                                                engagement_epoch: creature
                                                    .creature_spell_engagement_epoch_like_cpp(),
                                                slot,
                                                minimum_ms: minimum,
                                            },
                                        ),
                                    );
                                }
                                CreatureAiSpellConditionLikeCpp::Die => {}
                            }
                        }
                    }

                    // C++ advances EventMap before this gate but executes no
                    // due event while the creature owns UNIT_STATE_CASTING.
                    if creature.unit().has_unit_state(UnitState::CASTING.bits()) {
                        outcome.unit_state_casting_skips += 1;
                        continue;
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
                            .and_then(|spell_id| spell_store.get(spell_id))
                        else {
                            outcome.missing_spell_metadata += 1;
                            continue;
                        };
                        let spell =
                            creature_ai_effective_spell_info_like_cpp(spell, difficulty_id, config);
                        let target_kind = creature_ai_spell_target_like_cpp(
                            spell_id,
                            &spell,
                            difficulty_id,
                            config,
                        );
                        if target_kind.requires_random_threat_selection_like_cpp() {
                            // C++ Enemy/Debuff selection draws from the full
                            // non-offline threat list before CastSpell. This
                            // slice cannot yet prove every candidate's range,
                            // aura and player/NPC filters. Stop exact spell RNG
                            // accreditation instead of selecting the victim or
                            // drawing the later repeat delay out of order.
                            outcome.spell_effects_unrepresented += 1;
                            creature.invalidate_runtime_rng_authority_like_cpp();
                            continue;
                        }
                        let target_guid = target_kind
                            .resolve_for_single_player_threat_list_like_cpp(guid, recipient_guid);

                        // `CombatAI::UpdateAI` re-schedules after every
                        // `DoCast` attempt, including a failed one. Defer the
                        // delay draw until after a represented hit roll so the
                        // shared creature RNG follows C++ DoCast -> ScheduleEvent.
                        let minimum = creature_ai_spell_repeat_cooldown_like_cpp(
                            spell_id,
                            &spell,
                            difficulty_id,
                            config,
                        );
                        let schedule = PendingCreatureSpellScheduleLikeCpp {
                            caster_guid: guid,
                            map_id,
                            instance_id,
                            engagement_epoch: creature.creature_spell_engagement_epoch_like_cpp(),
                            slot,
                            minimum_ms: minimum,
                        };

                        match creature_ai_spell_disable_decision_like_cpp(
                            spell_id, map_id, creature, config,
                        ) {
                            CreatureSpellDisableDecisionLikeCpp::Enabled => {}
                            CreatureSpellDisableDecisionLikeCpp::Disabled => {
                                outcome.spells_disabled += 1;
                                pending_actions
                                    .push(PendingCreatureSpellActionLikeCpp::Schedule(schedule));
                                continue;
                            }
                            CreatureSpellDisableDecisionLikeCpp::ContextUnrepresented => {
                                outcome.spell_disable_context_unrepresented += 1;
                                pending_actions
                                    .push(PendingCreatureSpellActionLikeCpp::Schedule(schedule));
                                continue;
                            }
                        }
                        if !config.spell_has_no_unrepresented_runtime_hooks_like_cpp(spell_id) {
                            outcome.spell_runtime_hooks_unrepresented += 1;
                            pending_actions
                                .push(PendingCreatureSpellActionLikeCpp::Schedule(schedule));
                            continue;
                        }
                        if !config
                            .spell_has_no_unrepresented_casting_requirements_like_cpp(spell_id)
                        {
                            outcome.spell_casting_requirements_unrepresented += 1;
                            pending_actions
                                .push(PendingCreatureSpellActionLikeCpp::Schedule(schedule));
                            continue;
                        }
                        if !config
                            .spell_has_no_unrepresented_shapeshift_requirements_like_cpp(spell_id)
                        {
                            outcome.spell_casting_requirements_unrepresented += 1;
                            pending_actions
                                .push(PendingCreatureSpellActionLikeCpp::Schedule(schedule));
                            continue;
                        }
                        if !config.spell_has_no_unrepresented_aura_restrictions_like_cpp(
                            spell_id,
                            difficulty_id,
                        ) {
                            outcome.spell_effects_unrepresented += 1;
                            pending_actions
                                .push(PendingCreatureSpellActionLikeCpp::Schedule(schedule));
                            continue;
                        }
                        if !creature_ai_spell_has_represented_cooldown_semantics_like_cpp(
                            spell_id,
                            difficulty_id,
                            config,
                        ) {
                            outcome.spell_effects_unrepresented += 1;
                            pending_actions
                                .push(PendingCreatureSpellActionLikeCpp::Schedule(schedule));
                            continue;
                        }
                        if creature_ai_spell_is_combat_forbidden_like_cpp(
                            spell_id,
                            difficulty_id,
                            config,
                        ) {
                            outcome.spell_effects_unrepresented += 1;
                            pending_actions
                                .push(PendingCreatureSpellActionLikeCpp::Schedule(schedule));
                            continue;
                        }
                        if creature_ai_zero_power_rows_have_unrepresented_implicit_cost_like_cpp(
                            &spell,
                            difficulty_id,
                            config,
                            creature,
                        ) {
                            outcome.spell_effects_unrepresented += 1;
                            pending_actions
                                .push(PendingCreatureSpellActionLikeCpp::Schedule(schedule));
                            continue;
                        }
                        if creature_ai_spell_has_unrepresented_target_restrictions_like_cpp(
                            spell_id,
                            difficulty_id,
                            config,
                        ) {
                            outcome.spell_effects_unrepresented += 1;
                            pending_actions
                                .push(PendingCreatureSpellActionLikeCpp::Schedule(schedule));
                            continue;
                        }
                        match creature_ai_spell_single_unit_topology_like_cpp(
                            &spell,
                            target_guid,
                            recipient_guid,
                            creature_ai_spell_requires_projectile_payload_like_cpp(
                                spell_id,
                                difficulty_id,
                                config,
                            ),
                        ) {
                            Ok(()) => {}
                            Err(CreatureAiSpellRepresentationRejectionLikeCpp::NonInstant) => {
                                outcome.noninstant_casts_unrepresented += 1;
                                pending_actions
                                    .push(PendingCreatureSpellActionLikeCpp::Schedule(schedule));
                                continue;
                            }
                            Err(
                                CreatureAiSpellRepresentationRejectionLikeCpp::ProjectileOrAmmo,
                            ) => {
                                outcome.spell_projectiles_unrepresented += 1;
                                pending_actions
                                    .push(PendingCreatureSpellActionLikeCpp::Schedule(schedule));
                                continue;
                            }
                            Err(CreatureAiSpellRepresentationRejectionLikeCpp::EffectOrTarget) => {
                                outcome.spell_effects_unrepresented += 1;
                                pending_actions
                                    .push(PendingCreatureSpellActionLikeCpp::Schedule(schedule));
                                continue;
                            }
                        }
                        let Ok(command) = creature_ai_spell_plan_like_cpp(
                            creature,
                            guid,
                            target_guid,
                            map_id,
                            instance_id,
                            spell_id,
                            &spell,
                            difficulty_id,
                            config,
                        ) else {
                            outcome.spell_visuals_unrepresented += 1;
                            pending_actions
                                .push(PendingCreatureSpellActionLikeCpp::Schedule(schedule));
                            continue;
                        };
                        pending_actions.push(PendingCreatureSpellActionLikeCpp::Cast(
                            PendingCreatureSpellCastLikeCpp {
                                command,
                                difficulty_id,
                                turret_ai: false,
                            },
                        ));
                        pending_actions.push(PendingCreatureSpellActionLikeCpp::Schedule(schedule));
                        outcome.casts_ready += 1;
                    }
                }
                CreatureAiKindLikeCpp::TurretAI => {
                    if difficulty_id != 0 && spells[0] != 0 {
                        // See the CombatAI gate above: TurretAI has the same
                        // unhydrated difficulty-specific cast metadata.
                        outcome.spell_effects_unrepresented += 1;
                        continue;
                    }
                    // C++ TurretAI only ever reads `m_spells[0]`; unrelated
                    // template slots cannot put it into UNIT_STATE_CASTING.
                    if creature_ai_has_temporally_unrepresented_noninstant_spell_like_cpp(
                        &spells[..1],
                        difficulty_id,
                        config,
                    ) {
                        outcome.noninstant_casts_unrepresented += 1;
                        continue;
                    }
                    if creature.unit().has_unit_state(UnitState::CASTING.bits()) {
                        outcome.unit_state_casting_skips += 1;
                        continue;
                    }
                    let spell_id = spells[0];
                    if spell_id == 0 || !creature.can_swing() {
                        continue;
                    }
                    let Some(spell) = i32::try_from(spell_id)
                        .ok()
                        .and_then(|spell_id| spell_store.get(spell_id))
                    else {
                        outcome.missing_spell_metadata += 1;
                        continue;
                    };
                    let spell =
                        creature_ai_effective_spell_info_like_cpp(spell, difficulty_id, config);
                    match creature_ai_spell_disable_decision_like_cpp(
                        spell_id, map_id, creature, config,
                    ) {
                        CreatureSpellDisableDecisionLikeCpp::Enabled => {}
                        CreatureSpellDisableDecisionLikeCpp::Disabled => {
                            outcome.spells_disabled += 1;
                            // C++ reaches `CastSpell` and lets `Spell::CheckCast`
                            // reject the disabled spell, so BASE_ATTACK is still
                            // consumed whenever the raw combat-range gate in
                            // `DoSpellAttackIfReady` admitted the attempt.
                            // Deciding that here would use the pre-tick legacy
                            // position, so defer it to the canonical drain.
                            pending_actions.push(
                                PendingCreatureSpellActionLikeCpp::TurretRejectedAttempt(
                                    TurretRejectedCastAttemptLikeCpp {
                                        caster_guid: guid,
                                        target_guid: recipient_guid,
                                        map_id,
                                        instance_id,
                                        engagement_epoch: creature
                                            .creature_spell_engagement_epoch_like_cpp(),
                                        spell_id,
                                        difficulty_id,
                                    },
                                ),
                            );
                            continue;
                        }
                        CreatureSpellDisableDecisionLikeCpp::ContextUnrepresented => {
                            outcome.spell_disable_context_unrepresented += 1;
                            continue;
                        }
                    }
                    if !config.spell_has_no_unrepresented_runtime_hooks_like_cpp(spell_id) {
                        outcome.spell_runtime_hooks_unrepresented += 1;
                        continue;
                    }
                    if !config.spell_has_no_unrepresented_casting_requirements_like_cpp(spell_id) {
                        outcome.spell_casting_requirements_unrepresented += 1;
                        continue;
                    }
                    if !config.spell_has_no_unrepresented_shapeshift_requirements_like_cpp(spell_id)
                    {
                        outcome.spell_casting_requirements_unrepresented += 1;
                        creature.record_swing();
                        continue;
                    }
                    if !config.spell_has_no_unrepresented_aura_restrictions_like_cpp(
                        spell_id,
                        difficulty_id,
                    ) {
                        outcome.spell_effects_unrepresented += 1;
                        continue;
                    }
                    if !creature_ai_spell_has_represented_cooldown_semantics_like_cpp(
                        spell_id,
                        difficulty_id,
                        config,
                    ) {
                        outcome.spell_effects_unrepresented += 1;
                        continue;
                    }
                    if creature_ai_spell_is_combat_forbidden_like_cpp(
                        spell_id,
                        difficulty_id,
                        config,
                    ) {
                        outcome.spell_effects_unrepresented += 1;
                        // TurretAI resets BASE_ATTACK after its CastSpell call
                        // even when CheckCast rejects the peaceful-only spell.
                        creature.record_swing();
                        continue;
                    }
                    if creature_ai_zero_power_rows_have_unrepresented_implicit_cost_like_cpp(
                        &spell,
                        difficulty_id,
                        config,
                        creature,
                    ) {
                        outcome.spell_effects_unrepresented += 1;
                        continue;
                    }
                    if creature_ai_spell_has_unrepresented_target_restrictions_like_cpp(
                        spell_id,
                        difficulty_id,
                        config,
                    ) {
                        outcome.spell_effects_unrepresented += 1;
                        continue;
                    }
                    match creature_ai_spell_single_unit_topology_like_cpp(
                        &spell,
                        recipient_guid,
                        recipient_guid,
                        creature_ai_spell_requires_projectile_payload_like_cpp(
                            spell_id,
                            difficulty_id,
                            config,
                        ),
                    ) {
                        Ok(()) => {}
                        Err(CreatureAiSpellRepresentationRejectionLikeCpp::NonInstant) => {
                            outcome.noninstant_casts_unrepresented += 1;
                            continue;
                        }
                        Err(CreatureAiSpellRepresentationRejectionLikeCpp::ProjectileOrAmmo) => {
                            outcome.spell_projectiles_unrepresented += 1;
                            continue;
                        }
                        Err(CreatureAiSpellRepresentationRejectionLikeCpp::EffectOrTarget) => {
                            outcome.spell_effects_unrepresented += 1;
                            continue;
                        }
                    }
                    let Ok(command) = creature_ai_spell_plan_like_cpp(
                        creature,
                        guid,
                        recipient_guid,
                        map_id,
                        instance_id,
                        spell_id,
                        &spell,
                        difficulty_id,
                        config,
                    ) else {
                        outcome.spell_visuals_unrepresented += 1;
                        continue;
                    };
                    pending_actions.push(PendingCreatureSpellActionLikeCpp::Cast(
                        PendingCreatureSpellCastLikeCpp {
                            command,
                            difficulty_id,
                            turret_ai: true,
                        },
                    ));
                    outcome.casts_ready += 1;
                }
                _ => {}
            }
        }
    }
    pending_actions
}
