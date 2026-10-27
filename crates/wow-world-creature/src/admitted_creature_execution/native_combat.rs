//! The same admitted combat future, with required Player death callbacks at
//! the lethal damage sites. Every synchronous Map borrow ends before await.

use super::*;

/// Bind same-tick respawns to the exact live records inserted into the current
/// selection by the lifecycle phase. Their zero identity is an insertion
/// marker, not an identity that can safely cross the later awaited callbacks.
fn capture_new_selection_record_identities_like_cpp(
    manager: &wow_map::MapManager,
    selection: &mut [AdmittedCreatureExecutionObjectLikeCpp],
) -> Result<(), &'static str> {
    for object in selection
        .iter_mut()
        .filter(|object| object.record_identity == 0)
    {
        if manager.map_incarnation_like_cpp(wow_map::MapKey::new(object.map_id, object.instance_id))
            != Some(object.incarnation)
        {
            return Err("Respawned Creature Map incarnation changed");
        }
        let Some(record_identity) = manager
            .find_map(object.map_id, object.instance_id)
            .and_then(|map| {
                map.map().with_creature_like_cpp(
                    object.creature_guid,
                    super::admission::canonical_creature_record_identity_like_cpp,
                )
            })
        else {
            return Err("Respawned Creature selection record is unavailable");
        };
        object.record_identity = record_identity;
    }
    Ok(())
}

/// Revalidate one pending swing's attacker against the exact record selected
/// for this phase. This check is repeated after every awaited combat callback
/// before the executor can continue against that Creature GUID.
fn selected_creature_record_is_current_like_cpp(
    manager: &wow_map::MapManager,
    selection: &[AdmittedCreatureExecutionObjectLikeCpp],
    map_id: u16,
    instance_id: u32,
    creature_guid: wow_core::ObjectGuid,
) -> bool {
    let Some(object) = selection.iter().find(|object| {
        object.map_id == u32::from(map_id)
            && object.instance_id == instance_id
            && object.creature_guid == creature_guid
    }) else {
        return false;
    };
    if manager.map_incarnation_like_cpp(wow_map::MapKey::new(object.map_id, object.instance_id))
        != Some(object.incarnation)
    {
        return false;
    }
    manager
        .find_map(object.map_id, object.instance_id)
        .and_then(|map| {
            map.map().with_creature_like_cpp(
                object.creature_guid,
                super::admission::canonical_creature_record_identity_like_cpp,
            )
        })
        .is_some_and(|identity| identity == object.record_identity)
}

type PrimaryPlayerResidenceLikeCpp = (wow_map::PlayerHandle, wow_map::MapKey, u64, u64);

fn primary_creature_record_identity_is_current_like_cpp(
    manager: &wow_map::MapManager,
    map_id: u16,
    instance_id: u32,
    creature_guid: wow_core::ObjectGuid,
    expected_identity: u64,
) -> bool {
    manager
        .find_map(u32::from(map_id), instance_id)
        .and_then(|managed| {
            managed.map().with_creature_like_cpp(
                creature_guid,
                super::admission::canonical_creature_record_identity_like_cpp,
            )
        })
        == Some(expected_identity)
}

fn selected_swing_and_primary_residence_are_current_like_cpp(
    manager: &wow_map::MapManager,
    admission: &AdmittedCreatureExecutionLikeCpp,
    selection: &[AdmittedCreatureExecutionObjectLikeCpp],
    swing: &super::super::creature_melee_tick::PendingCreatureSwingLikeCpp,
    primary_player: Option<PrimaryPlayerResidenceLikeCpp>,
    primary_creature_identity: Option<u64>,
) -> bool {
    if manager.tick_coordination_like_cpp()
        != wow_map::MapTickCoordinationStateLikeCpp::AwaitingSessions(admission.tick_epoch)
        || admission.maps.iter().any(|map| {
            manager.map_incarnation_like_cpp(wow_map::MapKey::new(map.map_id, map.instance_id))
                != Some(map.incarnation)
        })
        || !selected_creature_record_is_current_like_cpp(
            manager,
            selection,
            swing.map_id,
            swing.instance_id,
            swing.attacker_guid,
        )
    {
        return false;
    }
    if swing.victim_guid.is_player() {
        primary_player.is_some_and(|(handle, map_key, map_incarnation, residence_revision)| {
            manager.current_player_admission_like_cpp(handle.guid())
                == Some((handle, residence_revision))
                && manager.map_incarnation_like_cpp(map_key) == Some(map_incarnation)
                && manager.player_active_residence_revision_like_cpp(handle)
                    == Some((map_key, residence_revision))
        })
    } else {
        let Some(expected_identity) = primary_creature_identity else {
            return false;
        };
        primary_creature_record_identity_is_current_like_cpp(
            manager,
            swing.map_id,
            swing.instance_id,
            swing.victim_guid,
            expected_identity,
        )
    }
}

fn split_log_event_like_cpp(
    source_guid: wow_core::ObjectGuid,
    map_id: u16,
    instance_id: u32,
    packet_bytes: Vec<u8>,
) -> wow_world_core::runtime_plan::RuntimeEvent {
    wow_world_core::runtime_plan::RuntimeEvent {
        source_guid,
        recipients: wow_world_core::runtime_plan::RecipientRule::MapBroadcastVisible {
            map_id,
            instance_id,
        },
        packet_bytes,
    }
}

fn ordered_combat_events_like_cpp(
    events: Vec<wow_world_core::runtime_plan::RuntimeEvent>,
    admission: &AdmittedCreatureExecutionLikeCpp,
    map_id: u16,
    instance_id: u32,
) -> Result<MapPlayerCombatCall, &'static str> {
    let producer_map_key = wow_map::MapKey::new(u32::from(map_id), instance_id);
    let producer_map_incarnation = admission
        .maps
        .iter()
        .find(|map| map.map_id == producer_map_key.map_id && map.instance_id == instance_id)
        .map(|map| map.incarnation)
        .ok_or("Ordered combat event Map was not admitted")?;
    Ok(MapPlayerCombatCall::RuntimeEvents {
        events,
        producer_map_key,
        producer_map_incarnation,
    })
}

fn attach_matching_attack_start_like_cpp(
    outcome: &mut AdmittedCreatureCombatPhasesOutcomeLikeCpp,
    command: &mut CommittedMapCreatureMelee,
) {
    if let Some((attack_start, attack_start_events)) = take_committed_attack_start_like_cpp(
        &mut outcome.aggro_committed_starts,
        &mut outcome.aggro.plan,
        command.command.attacker_guid,
        command.command.victim_guid,
        command.command.map_id,
        command.command.instance_id,
    ) {
        command.attack_start = Some(attack_start);
        command.attack_start_events = attack_start_events;
    }
}

fn split_damage_state_after_entry_like_cpp(
    state: &mut super::super::creature_melee_tick::MeleeSwingStateLikeCpp,
    remaining_damage: u32,
    absorbed: u32,
) {
    if absorbed == 0 {
        return;
    }
    state.absorbed_damage = state.absorbed_damage.saturating_add(absorbed);
    state.hit_info &= !(wow_packet::packets::combat::HIT_INFO_FULL_ABSORB
        | wow_packet::packets::combat::HIT_INFO_PARTIAL_ABSORB);
    state.hit_info |= if remaining_damage == 0 {
        wow_packet::packets::combat::HIT_INFO_FULL_ABSORB
    } else {
        wow_packet::packets::combat::HIT_INFO_PARTIAL_ABSORB
    };
    if let Some((info, _, _)) = state.creature_victim_presentation.as_mut() {
        *info &= !(wow_packet::packets::combat::HIT_INFO_FULL_ABSORB
            | wow_packet::packets::combat::HIT_INFO_PARTIAL_ABSORB);
        *info |= if remaining_damage == 0 {
            wow_packet::packets::combat::HIT_INFO_FULL_ABSORB
        } else {
            wow_packet::packets::combat::HIT_INFO_PARTIAL_ABSORB
        };
    }
}

/// Move one aggro start onto its admitted hit/death call and remove only that
/// start's duplicate deferred wire event. Other aggro events keep their order
/// and existing delivery path.
fn take_committed_attack_start_like_cpp(
    starts: &mut Vec<wow_world_core::session::mailbox::CreatureAttackStartLikeCppCommand>,
    aggro_plan: &mut wow_world_core::runtime_plan::RuntimePlan,
    attacker_guid: wow_core::ObjectGuid,
    victim_guid: wow_core::ObjectGuid,
    map_id: u16,
    instance_id: u32,
) -> Option<(
    wow_world_core::session::mailbox::CreatureAttackStartLikeCppCommand,
    Vec<wow_world_core::runtime_plan::RuntimeEvent>,
)> {
    let start_index = starts.iter().position(|start| {
        start.attacker_guid == attacker_guid
            && start.victim_guid == victim_guid
            && start.map_id == map_id
            && start.instance_id == instance_id
    })?;
    let mut start = starts.remove(start_index);
    use wow_packet::ServerPacket as _;
    let packet_bytes = wow_packet::packets::combat::AttackStart {
        attacker: start.attacker_guid,
        victim: start.victim_guid,
    }
    .to_bytes();
    let start_event_index = aggro_plan.events.iter().position(|event| {
        event.source_guid == start.attacker_guid && event.packet_bytes == packet_bytes
    });
    let mut attack_start_events = Vec::new();
    if let Some(start_event_index) = start_event_index {
        let mut retained = Vec::with_capacity(aggro_plan.events.len());
        for (index, event) in aggro_plan.events.drain(..).enumerate() {
            if index <= start_event_index && event.source_guid == start.attacker_guid {
                attack_start_events.push(event);
            } else {
                retained.push(event);
            }
        }
        aggro_plan.events = retained;
    }
    // The Session rail sends the packet prelude to the victim before the
    // melee prefix; the root runtime synchronously publishes these original
    // events to the other observers. With no matching plan event, Session
    // uses its fallback AttackStart delivery path.
    start.packet_already_broadcast = !attack_start_events.is_empty();
    Some((start, attack_start_events))
}

pub async fn run_admitted_creature_combat_phases_with_native_death(
    admission: &AdmittedCreatureExecutionLikeCpp,
    manager: &SharedCanonicalMapManager,
    lease: &SharedCreatureExecutionLeaseLikeCpp,
    inputs: AdmittedCreatureCombatPhaseInputsLikeCpp<'_>,
    combat_call: &mut MapPlayerCombatCallback<'_>,
    submit_respawns: &mut (
             dyn FnMut(&mut Vec<wow_persistence::RespawnPersistenceMutationLikeCpp>) + Send
         ),
) -> Result<AdmittedCreatureCombatPhasesOutcomeLikeCpp, NativePlayerDeathFailure> {
    let owner = CreatureExecutionOwnerLikeCpp::CanonicalAdmitted;
    let admitted = {
        let manager = manager
            .lock()
            .map_err(|_| "Combat Map lock is unavailable")?;
        admit_and_claim_admitted_creature_execution_like_cpp(admission, &manager, lease, owner)
    };
    let mut outcome = AdmittedCreatureCombatPhasesOutcomeLikeCpp::refused_like_cpp(admitted, owner);
    if !admitted.is_admitted_like_cpp() {
        return Ok(outcome);
    }
    let (pending, difficulties) = {
        let mut manager = manager
            .lock()
            .map_err(|_| "Player melee Map lock is unavailable")?;
        outcome.player_melee.attackers_seen = inputs.player_melee_attackers.len();
        collect_player_melee_swings_on_manager_like_cpp(
            &mut manager,
            inputs.player_melee_attackers,
            admission.diff_ms,
            inputs.player_melee_phase_state,
            &mut outcome.player_melee,
        )
    };
    outcome.player_melee.swings_ready = pending.len();
    for swing in pending {
        let death = {
            let mut manager = manager
                .lock()
                .map_err(|_| "Player melee Map lock is unavailable")?;
            if !admission.fence_like_cpp(&manager).is_admitted_like_cpp() {
                return Err("Player melee admission changed during callback".into());
            }
            let mut death = None;
            super::super::player_tick::execute_player_melee_swing_before_native_death(
                &mut manager,
                &mut CanonicalPlayerMeleeCreatureVictimsLikeCpp,
                &swing,
                &difficulties,
                admission.diff_ms,
                inputs.config,
                &mut outcome.player_melee,
                Some(&mut death),
            );
            death
        };
        if let Some(death) = death {
            combat_call(MapPlayerCombatCall::NativeDeath(death?)).await?;
        }
    }
    let mut selection = admission.objects.clone();
    {
        let mut manager = manager
            .lock()
            .map_err(|_| "Creature combat Map lock is unavailable")?;
        if !admission.fence_like_cpp(&manager).is_admitted_like_cpp() {
            return Err("Creature combat admission changed during callback".into());
        }
        outcome.lifecycle.record_module_events_like_cpp = inputs.record_creature_module_events;
        run_creature_lifecycle_phase_on_manager_like_cpp(
            &mut manager,
            &admission.maps,
            &mut selection,
            inputs.terrain,
            inputs.map_store,
            inputs.now,
            &mut outcome.lifecycle,
        );
    }
    submit_respawns(&mut outcome.lifecycle.respawn_db_mutations);
    {
        let mut manager = manager
            .lock()
            .map_err(|_| "Creature phase Map lock is unavailable")?;
        if manager.tick_coordination_like_cpp()
            != wow_map::MapTickCoordinationStateLikeCpp::AwaitingSessions(admission.tick_epoch)
            || admission.maps.iter().any(|map| {
                manager.map_incarnation_like_cpp(wow_map::MapKey::new(map.map_id, map.instance_id))
                    != Some(map.incarnation)
            })
        {
            return Err("Creature phase Map admission changed after respawn submission".into());
        }
        capture_new_selection_record_identities_like_cpp(&manager, &mut selection)?;
        run_creature_movement_phase_on_manager_like_cpp(
            &mut manager,
            &admission.maps,
            &selection,
            inputs.terrain,
            inputs.mmap_config,
            inputs.mmap_pathfinder,
            inputs.chase_targets,
            admission.diff_ms,
            &mut outcome.movement,
        );
        outcome.clock_advanced_ms =
            (outcome.movement.creatures_seen as u64).saturating_mul(u64::from(admission.diff_ms));
        let aggro = run_creature_aggro_phase_on_manager_like_cpp(
            &mut manager,
            &admission.maps,
            &selection,
            inputs.terrain,
            inputs.aggro_candidates,
            inputs.config,
        );
        outcome.aggro = aggro.outcome;
        outcome.aggro_committed_starts = aggro.committed_starts;
        outcome.aggro_committed_stops = aggro.committed_stops;
        run_creature_spell_phase_on_manager_like_cpp(
            &mut manager,
            &admission.maps,
            &selection,
            inputs.config,
            &mut outcome.spell,
        );
        outcome.spell_phase_executed = true;
    }
    let pending = {
        let mut manager = manager
            .lock()
            .map_err(|_| "Creature melee Map lock is unavailable")?;
        let mut store = AdmittedCanonicalCreatureStoreLikeCpp::new_like_cpp(
            &mut manager,
            &admission.maps,
            &selection,
        );
        collect_creature_melee_swings_on_store_like_cpp(&mut store, &mut outcome.melee)
    };
    for swing in pending {
        let (prepared, committed, primary_creature_identity) = {
            let mut manager = manager
                .lock()
                .map_err(|_| "Creature melee Map lock is unavailable")?;
            if manager.tick_coordination_like_cpp()
                != wow_map::MapTickCoordinationStateLikeCpp::AwaitingSessions(admission.tick_epoch)
                || admission.maps.iter().any(|map| {
                    manager
                        .map_incarnation_like_cpp(wow_map::MapKey::new(map.map_id, map.instance_id))
                        != Some(map.incarnation)
                })
            {
                return Err("Creature melee Map admission changed during callback".into());
            }
            if !selected_creature_record_is_current_like_cpp(
                &manager,
                &selection,
                swing.map_id,
                swing.instance_id,
                swing.attacker_guid,
            ) {
                return Err("Creature melee selected record changed before calculation".into());
            }
            let primary_creature_identity = if swing.victim_guid.is_player() {
                None
            } else {
                manager
                    .find_map(u32::from(swing.map_id), swing.instance_id)
                    .and_then(|managed| {
                        managed.map().with_creature_like_cpp(
                            swing.victim_guid,
                            super::admission::canonical_creature_record_identity_like_cpp,
                        )
                    })
            };
            let first_command = outcome.melee.commands.len();
            let prepared = super::super::creature_melee_tick::prepare_creature_melee_swing_like_cpp(
                &mut manager,
                &mut CanonicalCreatureMeleeAttackersLikeCpp,
                swing,
                inputs.config,
                &mut outcome.melee,
            );
            if prepared.is_some()
                && !swing.victim_guid.is_player()
                && primary_creature_identity.is_none()
            {
                return Err("Creature melee primary Creature identity is unavailable".into());
            }
            let commands = outcome
                .melee
                .commands
                .drain(first_command..)
                .collect::<Vec<_>>();
            let mut committed_calls = Vec::with_capacity(commands.len());
            for command in commands {
                let mut committed_call =
                    super::super::native_player_death::capture_committed_creature_melee(
                        &manager, command,
                    )?;
                attach_matching_attack_start_like_cpp(&mut outcome, &mut committed_call);
                committed_calls.push(committed_call);
            }
            (prepared, committed_calls, primary_creature_identity)
        };
        for committed_call in committed {
            let primary_player = Some((
                committed_call.handle,
                committed_call.map_key,
                committed_call.map_incarnation,
                committed_call.residence_revision,
            ));
            combat_call(MapPlayerCombatCall::CommittedCreatureMelee(committed_call)).await?;
            let manager = manager
                .lock()
                .map_err(|_| "Melee callback Map lock is unavailable")?;
            if !selected_swing_and_primary_residence_are_current_like_cpp(
                &manager,
                admission,
                &selection,
                &swing,
                primary_player,
                primary_creature_identity,
            ) {
                return Err(
                    "Creature melee selection or Player residence changed during callback".into(),
                );
            }
        }
        let Some(mut prepared) = prepared else {
            continue;
        };

        let mut primary_player = None;
        if swing.victim_guid.is_player() {
            let committed_call = {
                let manager = manager
                    .lock()
                    .map_err(|_| "Creature melee absorb Map lock is unavailable")?;
                let command = super::super::creature_melee_tick::primary_creature_melee_prefix_command_like_cpp(
                    &manager,
                    &prepared,
                    prepared.damage,
                )
                .ok_or("Creature melee primary Player disappeared before absorb callbacks")?;
                let mut committed_call =
                    super::super::native_player_death::capture_committed_creature_melee(
                        &manager, command,
                    )?;
                committed_call.presentation_stage =
                    wow_world_core::session::mailbox::CreatureMeleePresentationStageLikeCpp::AbsorbOnly;
                attach_matching_attack_start_like_cpp(&mut outcome, &mut committed_call);
                committed_call
            };
            primary_player = Some((
                committed_call.handle,
                committed_call.map_key,
                committed_call.map_incarnation,
                committed_call.residence_revision,
            ));
            combat_call(MapPlayerCombatCall::CommittedCreatureMelee(committed_call)).await?;
            let manager = manager
                .lock()
                .map_err(|_| "Creature melee absorb callback Map lock is unavailable")?;
            if !selected_swing_and_primary_residence_are_current_like_cpp(
                &manager,
                admission,
                &selection,
                &swing,
                primary_player,
                primary_creature_identity,
            ) {
                return Err("Creature melee selection or primary Player residence changed after absorb callbacks".into());
            }
        } else {
            let absorb_events = {
                let manager = manager
                    .lock()
                    .map_err(|_| "Creature melee absorb Map lock is unavailable")?;
                if !selected_swing_and_primary_residence_are_current_like_cpp(
                    &manager,
                    admission,
                    &selection,
                    &swing,
                    None,
                    primary_creature_identity,
                ) {
                    return Err("Creature melee selection changed before absorb callbacks".into());
                }
                let mut events = take_committed_attack_start_like_cpp(
                    &mut outcome.aggro_committed_starts,
                    &mut outcome.aggro.plan,
                    swing.attacker_guid,
                    swing.victim_guid,
                    swing.map_id,
                    swing.instance_id,
                )
                .map(|(_, events)| events)
                .unwrap_or_default();
                events.append(&mut prepared.state.creature_victim_absorb_events);
                events
            };
            if !absorb_events.is_empty() {
                combat_call(ordered_combat_events_like_cpp(
                    absorb_events,
                    admission,
                    swing.map_id,
                    swing.instance_id,
                )?)
                .await?;
                let manager = manager
                    .lock()
                    .map_err(|_| "Creature absorb callback Map lock is unavailable")?;
                if !selected_swing_and_primary_residence_are_current_like_cpp(
                    &manager,
                    admission,
                    &selection,
                    &swing,
                    None,
                    primary_creature_identity,
                ) {
                    return Err("Creature melee selection changed after absorb callbacks".into());
                }
            }
        }

        let (split_identities, difficulty_id) = {
            let manager = manager
                .lock()
                .map_err(|_| "Creature split snapshot Map lock is unavailable")?;
            if !selected_swing_and_primary_residence_are_current_like_cpp(
                &manager,
                admission,
                &selection,
                &swing,
                primary_player,
                primary_creature_identity,
            ) {
                return Err("Creature melee selection changed before split snapshot".into());
            }
            let difficulty_id = manager
                .find_map(u32::from(swing.map_id), swing.instance_id)
                .map(|managed| managed.difficulty())
                .unwrap_or(0);
            let identities = inputs
                .config
                .spell_store
                .as_deref()
                .map_or_else(Vec::new, |spells| {
                    super::super::creature_melee_split::split_aura_identities_like_cpp(
                        &manager,
                        swing.map_id,
                        swing.instance_id,
                        swing.victim_guid,
                        spells,
                        difficulty_id,
                        inputs.config.difficulty_store.as_deref(),
                    )
                });
            (identities, difficulty_id)
        };
        if let Some(spell_store) = inputs.config.spell_store.as_deref() {
            for identity in split_identities {
                let entry = {
                    let mut manager = manager
                        .lock()
                        .map_err(|_| "Creature split entry Map lock is unavailable")?;
                    if !selected_swing_and_primary_residence_are_current_like_cpp(
                        &manager,
                        admission,
                        &selection,
                        &swing,
                        primary_player,
                        primary_creature_identity,
                    ) {
                        return Err("Creature melee selection changed before split entry".into());
                    }
                    super::super::creature_melee_split::apply_melee_split_entry_like_cpp(
                        &mut manager,
                        swing.map_id,
                        swing.instance_id,
                        swing.attacker_guid,
                        swing.victim_guid,
                        identity,
                        prepared.damage,
                        0x01,
                        prepared.attacker_facts.is_charmed_owned_by_player_or_player,
                        spell_store,
                        inputs.config.spell_misc_store.as_deref(),
                        inputs.config.spell_threat_store.as_deref(),
                        inputs.config.spell_chain_store.as_deref(),
                        difficulty_id,
                        inputs.config.difficulty_store.as_deref(),
                        true,
                    )?
                };
                let Some(mut entry) = entry else {
                    continue;
                };
                prepared.damage = entry.damage_after;
                split_damage_state_after_entry_like_cpp(
                    &mut prepared.state,
                    entry.damage_after,
                    entry.absorbed,
                );
                if let Some(pending_death) = entry.pending_death.take() {
                    combat_call(MapPlayerCombatCall::NativeDeath(pending_death)).await?;
                    let manager = manager
                        .lock()
                        .map_err(|_| "Split death completion Map lock is unavailable")?;
                    if !selected_swing_and_primary_residence_are_current_like_cpp(
                        &manager,
                        admission,
                        &selection,
                        &swing,
                        primary_player,
                        primary_creature_identity,
                    ) {
                        return Err("Creature melee selection or primary Player residence changed after split death".into());
                    }
                }
                let mut events = entry.mutation_events;
                events.push(split_log_event_like_cpp(
                    entry.combat_log_source_guid,
                    swing.map_id,
                    swing.instance_id,
                    entry.combat_log_packet,
                ));
                combat_call(ordered_combat_events_like_cpp(
                    events,
                    admission,
                    swing.map_id,
                    swing.instance_id,
                )?)
                .await?;
                let manager = manager
                    .lock()
                    .map_err(|_| "Split publication Map lock is unavailable")?;
                if !selected_swing_and_primary_residence_are_current_like_cpp(
                    &manager,
                    admission,
                    &selection,
                    &swing,
                    primary_player,
                    primary_creature_identity,
                ) {
                    return Err("Creature melee selection or primary Player residence changed after split publication".into());
                }
            }
        }

        if swing.victim_guid.is_player() {
            let committed_call = {
                let manager = manager
                    .lock()
                    .map_err(|_| "Creature AttackerState Map lock is unavailable")?;
                if !selected_swing_and_primary_residence_are_current_like_cpp(
                    &manager,
                    admission,
                    &selection,
                    &swing,
                    primary_player,
                    primary_creature_identity,
                ) {
                    return Err(
                        "Creature melee selection changed before AttackerStateUpdate".into(),
                    );
                }
                let command = super::super::creature_melee_tick::primary_creature_melee_prefix_command_like_cpp(
                    &manager,
                    &prepared,
                    prepared.damage,
                )
                .ok_or("Creature melee primary Player disappeared before AttackerStateUpdate")?;
                let mut committed_call =
                    super::super::native_player_death::capture_committed_creature_melee(
                        &manager, command,
                    )?;
                committed_call.presentation_stage =
                    wow_world_core::session::mailbox::CreatureMeleePresentationStageLikeCpp::AttackerStateOnly;
                attach_matching_attack_start_like_cpp(&mut outcome, &mut committed_call);
                let binding = (
                    committed_call.handle,
                    committed_call.map_key,
                    committed_call.map_incarnation,
                    committed_call.residence_revision,
                );
                if primary_player.is_some_and(|primary| primary != binding) {
                    return Err(
                        "Creature melee primary Player identity changed between prefix stages"
                            .into(),
                    );
                }
                committed_call
            };
            primary_player = Some((
                committed_call.handle,
                committed_call.map_key,
                committed_call.map_incarnation,
                committed_call.residence_revision,
            ));
            combat_call(MapPlayerCombatCall::CommittedCreatureMelee(committed_call)).await?;
            let manager = manager
                .lock()
                .map_err(|_| "AttackerState callback Map lock is unavailable")?;
            if !selected_swing_and_primary_residence_are_current_like_cpp(
                &manager,
                admission,
                &selection,
                &swing,
                primary_player,
                primary_creature_identity,
            ) {
                return Err("Creature melee selection or primary Player residence changed after AttackerStateUpdate".into());
            }
        } else {
            let event = {
                let mut manager = manager
                    .lock()
                    .map_err(|_| "Creature AttackerState Map lock is unavailable")?;
                if !selected_swing_and_primary_residence_are_current_like_cpp(
                    &manager,
                    admission,
                    &selection,
                    &swing,
                    None,
                    primary_creature_identity,
                ) {
                    return Err(
                        "Creature melee selection changed before AttackerStateUpdate".into(),
                    );
                }
                super::super::creature_movement_tick::preview_creature_melee_attacker_state_event_like_cpp(
                    &mut manager,
                    &swing,
                    prepared.damage,
                    prepared.state.creature_victim_presentation,
                    prepared.state.absorbed_damage,
                    prepared.attacker_facts.is_charmed_owned_by_player_or_player,
                    prepared.primary_threat_plan,
                )
                .map_err(|_| "Creature AttackerStateUpdate validation changed before share damage")?
            };
            combat_call(ordered_combat_events_like_cpp(
                vec![event],
                admission,
                swing.map_id,
                swing.instance_id,
            )?)
            .await?;
            let manager = manager
                .lock()
                .map_err(|_| "AttackerState publication Map lock is unavailable")?;
            if !selected_swing_and_primary_residence_are_current_like_cpp(
                &manager,
                admission,
                &selection,
                &swing,
                None,
                primary_creature_identity,
            ) {
                return Err("Creature melee selection changed after AttackerStateUpdate".into());
            }
        }

        let (primary_wire_health_before, represented_damage_done, share_identities) = {
            let manager = manager
                .lock()
                .map_err(|_| "Creature share snapshot Map lock is unavailable")?;
            if !selected_swing_and_primary_residence_are_current_like_cpp(
                &manager,
                admission,
                &selection,
                &swing,
                primary_player,
                primary_creature_identity,
            ) {
                return Err("Creature melee selection changed before share snapshot".into());
            }
            let managed = manager
                .find_map(u32::from(swing.map_id), swing.instance_id)
                .ok_or("Creature melee primary Map disappeared before share damage")?;
            let primary_wire_health_before = if swing.victim_guid.is_player() {
                managed
                    .map()
                    .get_typed_player(swing.victim_guid)
                    .map(|victim| victim.unit().data().health)
            } else {
                managed
                    .map()
                    .with_creature_like_cpp(swing.victim_guid, |victim| victim.unit().data().health)
            }
            .ok_or("Creature melee primary target disappeared before share damage")?;
            let represented_damage_done = if prepared.damage > 0 && !swing.victim_guid.is_player() {
                managed
                    .map()
                    .with_creature_like_cpp(swing.victim_guid, |victim| {
                        victim.calculate_damage_for_sparring_like_cpp(
                            true,
                            prepared.attacker_facts.is_charmed_owned_by_player_or_player,
                            prepared.damage,
                        )
                    })
                    .unwrap_or(prepared.damage)
            } else {
                prepared.damage
            };
            let share_identities =
                inputs
                    .config
                    .spell_store
                    .as_deref()
                    .map_or_else(Vec::new, |spells| {
                        super::super::creature_melee_share::share_aura_identities_like_cpp(
                            &manager,
                            swing.map_id,
                            swing.instance_id,
                            swing.victim_guid,
                            represented_damage_done,
                            spells,
                            managed.difficulty(),
                            inputs.config.difficulty_store.as_deref(),
                        )
                    });
            (
                primary_wire_health_before,
                represented_damage_done,
                share_identities,
            )
        };

        let mut primary_was_share_target = false;
        if let Some(spell_store) = inputs.config.spell_store.as_deref() {
            for identity in share_identities {
                let entry = {
                    let mut manager = manager
                        .lock()
                        .map_err(|_| "Creature share entry Map lock is unavailable")?;
                    if !selected_swing_and_primary_residence_are_current_like_cpp(
                        &manager,
                        admission,
                        &selection,
                        &swing,
                        primary_player,
                        primary_creature_identity,
                    ) {
                        return Err("Creature melee selection changed before share entry".into());
                    }
                    super::super::creature_melee_share::apply_melee_share_entry_like_cpp(
                        &mut manager,
                        swing.map_id,
                        swing.instance_id,
                        swing.attacker_guid,
                        swing.victim_guid,
                        identity,
                        represented_damage_done,
                        0x01,
                        prepared.attacker_facts.is_charmed_owned_by_player_or_player,
                        spell_store,
                        inputs.config.spell_misc_store.as_deref(),
                        inputs.config.spell_threat_store.as_deref(),
                        inputs.config.spell_chain_store.as_deref(),
                        difficulty_id,
                        inputs.config.difficulty_store.as_deref(),
                        true,
                    )?
                };
                let Some(mut entry) = entry else {
                    continue;
                };
                primary_was_share_target |= entry.primary_target;
                if let Some(pending_death) = entry.pending_death.take() {
                    combat_call(MapPlayerCombatCall::NativeDeath(pending_death)).await?;
                    let manager = manager
                        .lock()
                        .map_err(|_| "Share death completion Map lock is unavailable")?;
                    if !selected_swing_and_primary_residence_are_current_like_cpp(
                        &manager,
                        admission,
                        &selection,
                        &swing,
                        primary_player,
                        primary_creature_identity,
                    ) {
                        return Err("Creature melee selection or primary Player residence changed after share death".into());
                    }
                }
                if !entry.mutation_events.is_empty() {
                    combat_call(ordered_combat_events_like_cpp(
                        entry.mutation_events,
                        admission,
                        swing.map_id,
                        swing.instance_id,
                    )?)
                    .await?;
                    let manager = manager
                        .lock()
                        .map_err(|_| "Share publication Map lock is unavailable")?;
                    if !selected_swing_and_primary_residence_are_current_like_cpp(
                        &manager,
                        admission,
                        &selection,
                        &swing,
                        primary_player,
                        primary_creature_identity,
                    ) {
                        return Err("Creature melee selection or primary Player residence changed after share publication".into());
                    }
                }
            }
        }

        let secondary =
            super::super::creature_melee_tick::MeleeSecondaryTargetsOutcomeLikeCpp::awaited_like_cpp(
                prepared.damage,
                primary_wire_health_before,
                represented_damage_done,
                primary_was_share_target,
            );
        let (death, committed) = {
            let mut manager = manager
                .lock()
                .map_err(|_| "Creature primary apply Map lock is unavailable")?;
            if !selected_swing_and_primary_residence_are_current_like_cpp(
                &manager,
                admission,
                &selection,
                &swing,
                primary_player,
                primary_creature_identity,
            ) {
                return Err("Creature melee selection or primary Player residence changed before primary apply".into());
            }
            let mut death = None;
            let first_command = outcome.melee.commands.len();
            super::super::creature_melee_tick::finish_prepared_creature_melee_swing_like_cpp(
                &mut manager,
                &mut CanonicalCreatureMeleeAttackersLikeCpp,
                prepared,
                secondary,
                &mut outcome.melee,
                Some(&mut death),
                true,
            );
            let commands = outcome
                .melee
                .commands
                .drain(first_command..)
                .collect::<Vec<_>>();
            let mut committed_calls = Vec::with_capacity(commands.len());
            for command in commands {
                let mut committed_call =
                    super::super::native_player_death::capture_committed_creature_melee(
                        &manager, command,
                    )?;
                committed_call.presentation_stage =
                    wow_world_core::session::mailbox::CreatureMeleePresentationStageLikeCpp::HealthOnly;
                committed_calls.push(committed_call);
            }
            (death, committed_calls)
        };
        if let Some(death) = death {
            let pending_death = death?;
            combat_call(MapPlayerCombatCall::NativeDeath(pending_death)).await?;
            let mut manager = manager
                .lock()
                .map_err(|_| "Melee completion Map lock is unavailable")?;
            if !selected_swing_and_primary_residence_are_current_like_cpp(
                &manager,
                admission,
                &selection,
                &swing,
                primary_player,
                primary_creature_identity,
            ) {
                return Err(
                    "Creature melee selection or primary Player residence changed before rearm"
                        .into(),
                );
            }
            outcome.melee.attacking_interrupt_auras_removed +=
                CanonicalCreatureMeleeAttackersLikeCpp
                    .with_attacker_like_cpp(&mut manager, &swing, |attacker| {
                        let removed = attacker
                            .unit_mut()
                            .remove_attacking_interrupt_auras_like_cpp();
                        attacker.record_swing();
                        removed
                    })
                    .unwrap_or_default();
            outcome.melee.canonical_hits += 1;
        }
        for committed_call in committed {
            let binding = Some((
                committed_call.handle,
                committed_call.map_key,
                committed_call.map_incarnation,
                committed_call.residence_revision,
            ));
            if primary_player.is_some_and(|primary| primary != binding.unwrap()) {
                return Err(
                    "Creature melee primary Player identity changed before health publication"
                        .into(),
                );
            }
            combat_call(MapPlayerCombatCall::CommittedCreatureMelee(committed_call)).await?;
            let manager = manager
                .lock()
                .map_err(|_| "Melee health callback Map lock is unavailable")?;
            if !selected_swing_and_primary_residence_are_current_like_cpp(
                &manager,
                admission,
                &selection,
                &swing,
                binding,
                primary_creature_identity,
            ) {
                return Err("Creature melee selection or primary Player residence changed after health publication".into());
            }
        }
    }
    Ok(outcome)
}

#[cfg(test)]
mod tests {
    use super::*;
    use wow_world_core::runtime_plan::{RecipientRule, RuntimeEvent, RuntimePlan};
    use wow_world_test_support::canonical_creatures::shared_canonical_map_manager;
    use wow_world_test_support::map_objects::{
        add_canonical_test_creature_on_map, test_creature_guid,
    };

    #[test]
    fn split_log_runtime_event_keeps_the_secondary_source_guid_like_cpp() {
        let secondary_caster = test_creature_guid(98_000);
        let event = split_log_event_like_cpp(secondary_caster, 7, 3, vec![1, 2, 3]);
        assert_eq!(event.source_guid, secondary_caster);
        assert!(matches!(
            event.recipients,
            RecipientRule::MapBroadcastVisible {
                map_id: 7,
                instance_id: 3,
            }
        ));
        assert_eq!(event.packet_bytes, vec![1, 2, 3]);
    }

    #[test]
    fn committed_attack_start_moves_its_ordered_source_prelude_before_the_hit_like_cpp() {
        use wow_packet::ServerPacket as _;

        let attacker_guid = test_creature_guid(98_001);
        let victim_guid = test_creature_guid(98_002);
        let other_victim_guid = test_creature_guid(98_003);
        let other_source_guid = test_creature_guid(98_004);
        let matching = wow_world_core::session::mailbox::CreatureAttackStartLikeCppCommand {
            attacker_guid,
            victim_guid,
            previous_victim_guid: None,
            map_id: 0,
            instance_id: 0,
            packet_already_broadcast: true,
        };
        let unmatched = wow_world_core::session::mailbox::CreatureAttackStartLikeCppCommand {
            attacker_guid,
            victim_guid: other_victim_guid,
            previous_victim_guid: None,
            map_id: 0,
            instance_id: 0,
            packet_already_broadcast: true,
        };
        let attack_start_bytes = wow_packet::packets::combat::AttackStart {
            attacker: attacker_guid,
            victim: victim_guid,
        }
        .to_bytes();
        let unrelated_bytes = wow_packet::packets::combat::AttackStart {
            attacker: attacker_guid,
            victim: other_victim_guid,
        }
        .to_bytes();
        let reaction_bytes = wow_packet::packets::combat::AIReaction {
            unit_guid: attacker_guid,
            reaction: wow_constants::creature::AiReaction::Hostile,
        }
        .to_bytes();
        let stop_bytes = wow_packet::packets::combat::SAttackStop {
            attacker: attacker_guid,
            victim: other_victim_guid,
            now_dead: false,
        }
        .to_bytes();
        let other_source_bytes = wow_packet::packets::combat::AttackStart {
            attacker: other_source_guid,
            victim: victim_guid,
        }
        .to_bytes();
        let mut starts = vec![matching, unmatched];
        let mut plan = RuntimePlan {
            events: vec![
                RuntimeEvent {
                    source_guid: other_source_guid,
                    recipients: RecipientRule::SelfOnly,
                    packet_bytes: other_source_bytes.clone(),
                },
                RuntimeEvent {
                    source_guid: attacker_guid,
                    recipients: RecipientRule::SelfOnly,
                    packet_bytes: reaction_bytes.clone(),
                },
                RuntimeEvent {
                    source_guid: other_source_guid,
                    recipients: RecipientRule::SelfOnly,
                    packet_bytes: vec![9],
                },
                RuntimeEvent {
                    source_guid: attacker_guid,
                    recipients: RecipientRule::SelfOnly,
                    packet_bytes: stop_bytes.clone(),
                },
                RuntimeEvent {
                    source_guid: attacker_guid,
                    recipients: RecipientRule::SelfOnly,
                    packet_bytes: attack_start_bytes.clone(),
                },
                RuntimeEvent {
                    source_guid: attacker_guid,
                    recipients: RecipientRule::SelfOnly,
                    packet_bytes: unrelated_bytes.clone(),
                },
            ],
        };

        let (attached, events) = take_committed_attack_start_like_cpp(
            &mut starts,
            &mut plan,
            attacker_guid,
            victim_guid,
            0,
            0,
        )
        .expect("the admitted hit has a matching committed attack start");

        assert_eq!(attached.attacker_guid, attacker_guid);
        assert_eq!(attached.victim_guid, victim_guid);
        assert!(attached.packet_already_broadcast);
        assert_eq!(events.len(), 3);
        assert_eq!(events[0].packet_bytes, reaction_bytes);
        assert_eq!(events[1].packet_bytes, stop_bytes);
        assert_eq!(events[2].packet_bytes, attack_start_bytes);
        assert_eq!(starts.len(), 1, "the unmatched start remains deferred");
        assert_eq!(starts[0].victim_guid, other_victim_guid);
        assert_eq!(plan.events.len(), 3, "later and other-source events remain");
        assert_eq!(plan.events[0].packet_bytes, other_source_bytes);
        assert_eq!(plan.events[1].packet_bytes, vec![9]);
        assert_eq!(plan.events[2].packet_bytes, unrelated_bytes);

        let mut fallback = vec![
            wow_world_core::session::mailbox::CreatureAttackStartLikeCppCommand {
                attacker_guid,
                victim_guid,
                previous_victim_guid: None,
                map_id: 0,
                instance_id: 0,
                packet_already_broadcast: true,
            },
        ];
        let (fallback_start, fallback_events) = take_committed_attack_start_like_cpp(
            &mut fallback,
            &mut RuntimePlan::default(),
            attacker_guid,
            victim_guid,
            0,
            0,
        )
        .expect("the committed start remains available for fallback delivery");
        assert!(!fallback_start.packet_already_broadcast);
        assert!(fallback_events.is_empty());
    }

    #[test]
    fn selected_respawn_identity_is_bound_then_rejects_a_replaced_record_like_cpp() {
        let canonical = shared_canonical_map_manager();
        let attacker_guid = test_creature_guid(98_011);
        add_canonical_test_creature_on_map(
            &canonical,
            attacker_guid,
            9001,
            wow_core::Position::new(10.0, 10.0, 0.0, 0.0),
            0,
            0,
            0,
        );
        let mut selection = vec![AdmittedCreatureExecutionObjectLikeCpp {
            map_id: 0,
            instance_id: 0,
            incarnation: 0,
            creature_guid: attacker_guid,
            record_identity: 0,
        }];
        {
            let manager = canonical
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            selection[0].incarnation = manager
                .map_incarnation_like_cpp(wow_map::MapKey::new(0, 0))
                .expect("the fixture map has a canonical incarnation");
            capture_new_selection_record_identities_like_cpp(&manager, &mut selection)
                .expect("the newly selected record is live");
            assert_ne!(selection[0].record_identity, 0);
            assert!(selected_creature_record_is_current_like_cpp(
                &manager,
                &selection,
                0,
                0,
                attacker_guid,
            ));
        }

        // Replacing the same GUID inside the same map incarnation must not
        // make a pending swing resolve to the new record.
        add_canonical_test_creature_on_map(
            &canonical,
            attacker_guid,
            9001,
            wow_core::Position::new(10.0, 10.0, 0.0, 0.0),
            0,
            0,
            0,
        );
        let manager = canonical
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        assert!(!selected_creature_record_is_current_like_cpp(
            &manager,
            &selection,
            0,
            0,
            attacker_guid,
        ));
    }

    #[test]
    fn awaited_creature_primary_rejects_same_guid_record_replacement_like_cpp() {
        let canonical = shared_canonical_map_manager();
        let victim_guid = test_creature_guid(98_021);
        add_canonical_test_creature_on_map(
            &canonical,
            victim_guid,
            9001,
            wow_core::Position::new(10.0, 10.0, 0.0, 0.0),
            0,
            0,
            0,
        );
        let old_identity = {
            let manager = canonical
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            manager
                .find_map(0, 0)
                .unwrap()
                .map()
                .with_creature_like_cpp(
                    victim_guid,
                    super::super::admission::canonical_creature_record_identity_like_cpp,
                )
                .unwrap()
        };
        {
            let manager = canonical
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            assert!(primary_creature_record_identity_is_current_like_cpp(
                &manager,
                0,
                0,
                victim_guid,
                old_identity,
            ));
        }
        add_canonical_test_creature_on_map(
            &canonical,
            victim_guid,
            9001,
            wow_core::Position::new(10.0, 10.0, 0.0, 0.0),
            0,
            0,
            0,
        );
        let manager = canonical
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        assert!(!primary_creature_record_identity_is_current_like_cpp(
            &manager,
            0,
            0,
            victim_guid,
            old_identity,
        ));
    }
}
