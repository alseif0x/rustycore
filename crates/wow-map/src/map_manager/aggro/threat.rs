//! Shared Aggro rules, preserving the legacy source gates and order.
use super::*;

pub fn update_threat_victim(
    creature: &mut WorldCreature,
    candidates: &[&AggroCandidate],
    config: &AggroSettings,
    owner_snapshots: &HashMap<ObjectGuid, AggroOwnerSnapshot>,
    policies: &mut AggroPolicies<'_>,
) -> AggroThreatUpdate {
    if creature.state() != wow_entities::CreatureAiState::InCombat {
        return AggroThreatUpdate::Unchanged;
    }

    let old_victim = creature.creature.ai_ownership().combat_target;
    if let Some(old_victim) = old_victim {
        creature
            .creature
            .unit_mut()
            .subsystems_mut()
            .combat
            .add_threat(old_victim, 0.0);
    }

    let candidate_by_guid: HashMap<_, _> = candidates
        .iter()
        .map(|candidate| (candidate.player_guid, *candidate))
        .collect();
    let mut eligible_candidate_guids: HashSet<_> = candidates
        .iter()
        .filter(|candidate| {
            candidate_targetable(candidate)
                && candidate_hostile(creature, candidate, policies).unwrap_or(false)
                && candidate_accessible(creature, candidate)
                && matches!(
                    candidate_visibility(
                        creature,
                        creature.map_id() as u16,
                        creature.instance_id(),
                        candidate,
                        false,
                    ),
                    AggroVisibility::Allowed
                )
                && matches!(
                    candidate_leash(creature, candidate, config, owner_snapshots,),
                    AggroLeash::Allowed
                )
        })
        .map(|candidate| candidate.player_guid)
        .collect();
    eligible_candidate_guids.extend(owner_snapshots.iter().filter_map(|(guid, snapshot)| {
        (*guid != creature.guid()
            && !candidate_by_guid.contains_key(guid)
            && snapshot.map_id == creature.map_id() as u16
            && snapshot.instance_id == creature.instance_id()
            && creature.phase_shift().can_see(&snapshot.phase_shift)
            && snapshot.alive
            && !snapshot.in_evade_mode
            && !snapshot.unit_flags.intersects(
                UnitFlags::NON_ATTACKABLE
                    | UnitFlags::NON_ATTACKABLE_2
                    | UnitFlags::NOT_ATTACKABLE_1
                    | UnitFlags::ON_TAXI
                    | UnitFlags::IMMUNE_TO_NPC
                    | UnitFlags::UNINTERACTIBLE,
            )
            && snapshot_hostile(creature, snapshot, policies).unwrap_or(false)
            && if snapshot.in_water {
                creature.creature.can_enter_water_like_cpp()
            } else {
                creature.creature.can_walk_like_cpp() || creature.creature.can_fly_like_cpp()
            }
            && matches!(
                snapshot_leash(creature, snapshot, config, owner_snapshots,),
                AggroLeash::Allowed
            ))
        .then_some(*guid)
    }));
    let threat_guids = creature
        .creature
        .unit()
        .subsystems()
        .combat
        .sorted_threat_guids();
    let (reevaluate_all_suppressed, pending_suppressed_threat) = creature
        .creature
        .unit_mut()
        .subsystems_mut()
        .combat
        .take_suppressed_reactivation_requests_like_cpp();
    let melee_school_mask = creature.creature.melee_damage_school_mask();
    for threat_guid in threat_guids {
        let previous_state = creature
            .creature
            .unit()
            .subsystems()
            .combat
            .threat_ref(threat_guid)
            .copied();
        let is_taunting = previous_state.is_some_and(|reference| reference.is_taunting());
        let should_be_suppressed = owner_snapshots.get(&threat_guid).is_some_and(|snapshot| {
            !is_taunting
                && ((melee_school_mask != 0
                    && ((snapshot.school_immunity_mask | snapshot.damage_immunity_mask)
                        & melee_school_mask)
                        == melee_school_mask)
                    || snapshot.has_confuse_aura
                    || snapshot.has_breakable_stun_aura)
        });
        let online_state = match previous_state.map(|reference| reference.online_state) {
            _ if !eligible_candidate_guids.contains(&threat_guid) => {
                wow_entities::ThreatOnlineState::Offline
            }
            Some(wow_entities::ThreatOnlineState::Online) if should_be_suppressed => {
                wow_entities::ThreatOnlineState::Suppressed
            }
            Some(wow_entities::ThreatOnlineState::Suppressed)
                if !should_be_suppressed
                    && (reevaluate_all_suppressed
                        || pending_suppressed_threat.contains_key(&threat_guid)) =>
            {
                wow_entities::ThreatOnlineState::Online
            }
            Some(wow_entities::ThreatOnlineState::Suppressed) => {
                wow_entities::ThreatOnlineState::Suppressed
            }
            Some(wow_entities::ThreatOnlineState::Offline) if should_be_suppressed => {
                wow_entities::ThreatOnlineState::Suppressed
            }
            _ => wow_entities::ThreatOnlineState::Online,
        };
        creature
            .creature
            .unit_mut()
            .subsystems_mut()
            .combat
            .set_threat_online_state(threat_guid, online_state);
        if online_state == wow_entities::ThreatOnlineState::Online {
            if let Some(amount) = pending_suppressed_threat.get(&threat_guid) {
                creature
                    .creature
                    .unit_mut()
                    .subsystems_mut()
                    .combat
                    .add_threat(threat_guid, *amount);
            }
        }
    }

    let highest_guid = creature
        .creature
        .unit()
        .subsystems()
        .combat
        .sorted_threat_guids()
        .into_iter()
        .find(|guid| {
            creature
                .creature
                .unit()
                .subsystems()
                .combat
                .threat_ref(*guid)
                .is_some_and(wow_entities::ThreatReferenceState::is_online)
        });
    if highest_guid.is_none() {
        let participant_guids = creature
            .creature
            .unit()
            .subsystems()
            .combat
            .sorted_threat_guids();
        let combat = &mut creature.creature.unit_mut().subsystems_mut().combat;
        combat.clear_threat();
        combat.clear_attackers();
        creature
            .creature
            .unit_mut()
            .add_unit_state(UnitState::EVADE.bits());
        creature.creature.clear_tap_list_for_evade();
        let removed_taunt_slots = creature.reset_combat();
        return AggroThreatUpdate::Evade {
            previous_victim: old_victim,
            participant_guids,
            removed_taunt_slots,
        };
    }

    let melee_candidate_guids = owner_snapshots
        .iter()
        .filter(|(guid, snapshot)| {
            eligible_candidate_guids.contains(guid)
                && within_melee_range(
                    creature.position(),
                    creature.creature.unit().world().combat_reach(),
                    snapshot.position,
                    snapshot.combat_reach,
                )
        })
        .map(|(guid, _)| *guid)
        .collect();
    let selected = creature
        .creature
        .unit_mut()
        .subsystems_mut()
        .combat
        .reselect_victim(&melee_candidate_guids);
    let Some(selected) = selected else {
        let participant_guids = creature
            .creature
            .unit()
            .subsystems()
            .combat
            .sorted_threat_guids();
        creature
            .creature
            .unit_mut()
            .add_unit_state(UnitState::EVADE.bits());
        let removed_taunt_slots = creature.reset_combat();
        return AggroThreatUpdate::Evade {
            previous_victim: old_victim,
            participant_guids,
            removed_taunt_slots,
        };
    };
    if !eligible_candidate_guids.contains(&selected) {
        let participant_guids = creature
            .creature
            .unit()
            .subsystems()
            .combat
            .sorted_threat_guids();
        creature
            .creature
            .unit_mut()
            .add_unit_state(UnitState::EVADE.bits());
        let removed_taunt_slots = creature.reset_combat();
        return AggroThreatUpdate::Evade {
            previous_victim: old_victim,
            participant_guids,
            removed_taunt_slots,
        };
    }

    if old_victim != Some(selected) {
        creature.enter_combat(selected);
        AggroThreatUpdate::Switched {
            previous_victim: old_victim.unwrap_or(ObjectGuid::EMPTY),
        }
    } else {
        AggroThreatUpdate::Unchanged
    }
}
