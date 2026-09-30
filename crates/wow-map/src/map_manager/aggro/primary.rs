//! Primary phase in its original candidate/taunt/assistance/threat order.
use super::*;

impl AggroFrame {
    pub(crate) fn run_primaries(&mut self, backend: &mut AggroMap<'_>, policies: &mut AggroPolicies<'_>) {
        let map_id = self.map_id;
        let instance_id = self.instance_id;
        let config = self.settings;
        let guids = &self.primary_guids;
        let map_candidates: Vec<_> = self.candidates.iter().filter(|candidate|
            candidate.map_id == map_id && candidate.instance_id == instance_id
                && backend.player_present(candidate.player_guid)).collect();
        let owner_snapshots = &self.owners;
        let creature_factions = &self.factions;
        let assistance_calls = &mut self.calls;
        let outcome = &mut self.outcome;
        for guid in guids.iter().copied() {
            outcome.creatures_seen += 1;
            let Some(creature) = backend.actor_mut( guid) else {
                continue;
            };
            let expired_taunt_slots = creature.expire_taunt_auras_if_due_like_cpp();
            if !expired_taunt_slots.is_empty() {

                emit(outcome, map_id, instance_id, creature, AggroEffectKind::RemoveAuras(expired_taunt_slots));
            }
            let due_assistance = creature.take_due_assistance_like_cpp();
            let _ = creature;
            for (victim_guid, assistant_guids) in due_assistance {
                let victim = map_candidates
                    .iter()
                    .find(|candidate| candidate.player_guid == victim_guid)
                    .copied();
                let victim_snapshot = owner_snapshots.get(&victim_guid);
                if (victim.is_none() && victim_snapshot.is_none()) || !backend.victim_present(victim_guid) {
                    continue;
                }
                for assistant_guid in assistant_guids {
                    let Some(assistant) =
                        backend.actor_mut( assistant_guid)
                    else {
                        continue;
                    };
                    let flags = assistant.creature.unit().unit_flags_like_cpp();
                    if assistant.is_alive()
                        && !assistant.creature.is_in_combat()
                        && !assistant.creature.is_in_evade_mode_like_cpp()
                        && !assistant.creature.unit().has_unit_state(
                            (UnitState::STUNNED | UnitState::CONFUSED | UnitState::FLEEING).bits(),
                        )
                        && assistant
                            .creature
                            .has_react_state(wow_entities::ReactState::Aggressive)
                        && !assistant.creature.is_civilian_like_cpp()
                        && assistant
                            .creature
                            .unit()
                            .subsystems()
                            .control
                            .charmer_or_owner_guid()
                            .is_none()
                        && !flags.intersects(
                            UnitFlags::NON_ATTACKABLE
                                | UnitFlags::IMMUNE_TO_NPC
                                | UnitFlags::UNINTERACTIBLE,
                        )
                        && creature_factions.get(&guid)
                            == Some(&assistant.creature.unit().data().faction_template)
                        && (victim.is_some_and(|victim| {
                            candidate_targetable(
                                victim,
                            ) && candidate_hostile(
                                assistant, victim, policies,
                            )
                            .unwrap_or(false)
                        }) || victim_snapshot.is_some_and(|victim| {
                            victim.alive
                                && !victim.in_evade_mode
                                && snapshot_hostile(
                                    assistant, victim, policies,
                                )
                                .unwrap_or(false)
                        }))
                    {
                        // C++ `AssistDelayEvent` calls `SetNoCallAssistance(true)`
                        // only after the delayed `CanAssistTo` revalidation and
                        // immediately before `EngageWithTarget`.
                        assistant.set_no_call_assistance_like_cpp();
                        assistant.enter_combat(victim_guid);
                        assistant
                            .creature
                            .unit_mut()
                            .subsystems_mut()
                            .combat
                            .add_threat(victim_guid, 0.0);
                        outcome.assistance_starts += 1;
                        outcome.aggro_starts += 1;

                        emit(outcome, map_id, instance_id, assistant, AggroEffectKind::AttackStart { victim: victim_guid });
                        outcome.commands.push(
                            AggroAttackStart {
                                attacker_guid: assistant_guid,
                                victim_guid,
                                previous_victim_guid: None,
                                map_id,
                                instance_id,
                                packet_already_broadcast: true,
                            },
                        );
                    }
                }
            }
            let Some(creature) = backend.actor_mut( guid) else {
                continue;
            };
            match update_threat_victim(
                creature,
                &map_candidates,
                &config,
                &owner_snapshots,
                policies,
            ) {
                AggroThreatUpdate::Unchanged => {}
                AggroThreatUpdate::Switched { previous_victim } => {
                    outcome.victim_switches += 1;
                    if let Some(victim_guid) = creature.creature.ai_ownership().combat_target {
                        emit(outcome, map_id, instance_id, creature, AggroEffectKind::AttackStart { victim: victim_guid });
                        outcome.commands.push(
                            AggroAttackStart {
                                attacker_guid: guid,
                                victim_guid,
                                previous_victim_guid: (!previous_victim.is_empty())
                                    .then_some(previous_victim),
                                map_id,
                                instance_id,
                                packet_already_broadcast: true,
                            },
                        );
                    }
                }
                AggroThreatUpdate::Evade {
                    previous_victim,
                    participant_guids,
                    removed_taunt_slots,
                } => {

                    if let Some(previous_victim) = previous_victim {

                        emit(outcome, map_id, instance_id, creature, AggroEffectKind::AttackStop { victim: previous_victim });
                    }
                    for participant_guid in participant_guids {
                        outcome.stop_commands.push(
                            AggroAttackStop {
                                attacker_guid: guid,
                                victim_guid: participant_guid,
                                map_id,
                                instance_id,
                            },
                        );
                    }
                    if !removed_taunt_slots.is_empty() {
                        emit(outcome, map_id, instance_id, creature, AggroEffectKind::RemoveAuras(removed_taunt_slots));
                    }
                    outcome.evades_started += 1;
                    continue;
                }
            }
            if let Some(victim_guid) = creature.take_assistance_call_like_cpp() {
                assistance_calls.push((
                    guid,
                    victim_guid,
                    creature.creature.unit().world().clone(),
                    creature.creature.unit().data().faction_template,
                ));
            }
            if creature
                .creature
                .unit()
                .has_unit_state(UnitState::SIGHTLESS.bits())
            {
                outcome.sightless_creatures_skipped += 1;
                continue;
            }
            let ai_kind = match select_ai(creature, policies) {
                AggroAiSelection::Selected(ai_kind) => ai_kind,
                AggroAiSelection::Unrepresented => {
                    outcome.ai_selection_unrepresented += 1;
                    continue;
                }
            };
            if matches!(ai_kind, AggroAiKind::NoBaseLos) {
                outcome.ai_los_suppressed += 1;
                continue;
            }
            for candidate in &map_candidates {
                if !candidate_targetable(candidate) {
                    outcome.targetability_rejections += 1;
                    continue;
                }
                match candidate_visibility(
                    creature,
                    map_id,
                    instance_id,
                    candidate,
                    false,
                ) {
                    AggroVisibility::Allowed => {}
                    AggroVisibility::Rejected => {
                        if matches!(
                            candidate_visibility(
                                creature,
                                map_id,
                                instance_id,
                                candidate,
                                true,
                            ),
                            AggroVisibility::Allowed
                        ) {
                            let alert_triggered = trigger_alert(
                                creature, candidate, policies,
                            );
                            if alert_triggered {

                                outcome.alert_triggers += 1;
                                emit(outcome, map_id, instance_id, creature, AggroEffectKind::Alert);
                            } else {
                                outcome.alert_rejections += 1;
                            }
                        } else {
                            outcome.alert_rejections += 1;
                        }
                        outcome.visibility_rejections += 1;
                        continue;
                    }
                    AggroVisibility::Unrepresented => {
                        outcome.visibility_unrepresented += 1;
                        continue;
                    }
                }
                match candidate_hostile(
                    creature, candidate, policies,
                ) {
                    Some(true) => {}
                    Some(false) => {
                        outcome.hostility_rejections += 1;
                        continue;
                    }
                    None => {
                        outcome.hostility_unrepresented += 1;
                        continue;
                    }
                }
                if !candidate_accessible(
                    creature, candidate,
                ) {
                    outcome.accessibility_rejections += 1;
                    continue;
                }
                if creature.creature.is_in_evade_mode_like_cpp() {
                    outcome.attacker_evade_rejections += 1;
                    continue;
                }
                match can_attack(
                    &ai_kind, creature, candidate, policies,
                ) {
                    AggroAttackDecision::Allowed => {}
                    AggroAttackDecision::Rejected => {
                        outcome.ai_can_attack_rejections += 1;
                        continue;
                    }
                    AggroAttackDecision::Unrepresented => {
                        outcome.ai_can_attack_unrepresented += 1;
                        continue;
                    }
                }
                match candidate_leash(
                    creature,
                    candidate,
                    &config,
                    &owner_snapshots,
                ) {
                    AggroLeash::Allowed => {}
                    AggroLeash::OwnerPositionUnrepresented => {
                        outcome.owner_position_unrepresented += 1;
                        continue;
                    }
                    AggroLeash::HomeRangeRejected => {
                        outcome.home_range_rejections += 1;
                        continue;
                    }
                }
                if no_gray_aggro(
                    &config,
                    candidate.player_level,
                    candidate.player_gray_level,
                    creature.level(),
                ) {
                    outcome.gray_aggro_rejections += 1;
                    continue;
                }
                let effective_aggro_range =
                    (policies.attack_distance)(AggroDistanceFacts {
                        aggro_rate: config.creature_aggro_rate,
                        creature_combat_reach: creature.creature.unit().world().combat_reach(),
                        required_expansion: creature.creature.lifecycle_metadata().required_expansion,
                        max_player_level_config: config.max_player_level_config,
                        player_level_for_target: candidate.player_level,
                        creature_level_for_target: creature.level(),
                        creature_detect_range_aura_mod: creature
                            .creature
                            .unit()
                            .total_aura_modifier_like_cpp(
                                wow_constants::spell::aura_types::SPELL_AURA_MOD_DETECT_RANGE,
                            ) as f32,
                        player_detected_range_aura_mod: candidate.player_detected_range_aura_mod,
                    }) + creature.creature.combat_distance();

                if creature
                    .creature
                    .try_ai_aggro_with_effective_range_like_cpp(
                        candidate.player_guid,
                        &candidate.position,
                        candidate.player_combat_reach,
                        effective_aggro_range,
                    )
                {

                    creature
                        .creature
                        .unit_mut()
                        .subsystems_mut()
                        .combat
                        .add_threat(candidate.player_guid, 0.0);
                    creature.sync_runtime_motion_master_like_cpp();
                    if creature.runtime_motion_master_current_kind_like_cpp()
                        == Some(wow_movement::MovementGeneratorType::Chase)
                        && let Some(stop) = creature.stop_move_spline_like_cpp()
                    {

                        outcome.movement_interrupts += 1;
                        emit(outcome, map_id, instance_id, creature, AggroEffectKind::MoveStop(stop));
                    }
                    outcome.aggro_starts += 1;
                    emit(outcome, map_id, instance_id, creature, AggroEffectKind::AttackStart { victim: candidate.player_guid });
                    outcome.commands.push(
                        AggroAttackStart {
                            attacker_guid: guid,
                            victim_guid: candidate.player_guid,
                            previous_victim_guid: None,
                            map_id,
                            instance_id,
                            packet_already_broadcast: true,
                        },
                    );
                    if let Some(victim_guid) = creature.take_assistance_call_like_cpp() {
                        assistance_calls.push((
                            guid,
                            victim_guid,
                            creature.creature.unit().world().clone(),
                            creature.creature.unit().data().faction_template,
                        ));
                    }
                    break;
                }
            }
        }


        backend.commit_combat(outcome);
    }
}

fn no_gray_aggro(config: &AggroSettings, player_level: u8, player_gray_level: u8, creature_level: u8) -> bool {
    if creature_level > player_gray_level { return false; }
    let not_above = config.no_gray_aggro_above;
    let not_below = config.no_gray_aggro_below;
    if not_above == 0 && not_below == 0 { return false; }
    let player_level = u32::from(player_level);
    player_level <= not_below || (not_above > 0 && player_level >= not_above)
}
