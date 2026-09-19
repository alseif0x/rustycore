//! Canonical-to-legacy replay for represented Creature melee victims.

use super::*;

pub(in crate::session) struct CreatureMeleeVictimSyncIdentityLikeCpp {
    pub(in crate::session) authority: OwnedLootAuthority,
    pub(in crate::session) health_state_revision_authority:
        wow_entities::HealthStateRevisionAuthorityLikeCpp,
    pub(in crate::session) spawn_id: u64,
    pub(in crate::session) loot_lifecycle_revision_before: u64,
    pub(in crate::session) loot_lifecycle_revision_after: u64,
    pub(in crate::session) death_state_before: wow_constants::DeathState,
    pub(in crate::session) death_state_after: wow_constants::DeathState,
    pub(in crate::session) ai_state_before: wow_entities::CreatureAiState,
    pub(in crate::session) ai_state_after: wow_entities::CreatureAiState,
}

pub(in crate::session) struct CreatureMeleeVictimSyncStateLikeCpp {
    pub(in crate::session) applied_damage: u32,
    pub(in crate::session) threat: Option<CreatureDamageThreatOutcomeLikeCpp>,
    pub(in crate::session) victim_health_before: u64,
    pub(in crate::session) victim_health_after: u64,
    pub(in crate::session) victim_health_state_revision_before: u64,
    pub(in crate::session) victim_health_state_revision_after: u64,
    pub(in crate::session) identity: CreatureMeleeVictimSyncIdentityLikeCpp,
}

pub(in crate::session) enum CreatureMeleeApplyResultLikeCpp {
    Ready,
    Hit {
        victim_applied_damage: u32,
        victim_threat: Option<CreatureDamageThreatOutcomeLikeCpp>,
        victim_health_before: u64,
        victim_health_after: u64,
        victim_health_state_revision_before: u64,
        victim_health_state_revision_after: u64,
        victim_creature_sync_identity: Option<CreatureMeleeVictimSyncIdentityLikeCpp>,
        over_damage: i32,
        target_level: u8,
        events: Vec<RuntimeEvent>,
    },
    OutOfRange,
    BadFacing,
    AttackerStateRejected,
    LosRejected,
    AttackerUnavailable,
    VictimNotAlive,
    MissingVictim,
}

#[derive(Clone, Copy)]
pub(super) struct PendingCreatureSwingLikeCpp {
    pub(super) map_id: u16,
    pub(super) instance_id: u32,
    pub(super) attacker_guid: ObjectGuid,
    pub(super) attacker_position: Position,
    pub(super) attacker_combat_reach: f32,
    pub(super) attacker_can_state_update: bool,
    pub(super) victim_guid: ObjectGuid,
}

pub(super) struct CreatureVictimCompatibilitySyncLikeCpp {
    pub(super) swing: PendingCreatureSwingLikeCpp,
    pub(super) state: CreatureMeleeVictimSyncStateLikeCpp,
}

struct CreatureVictimCompatibilitySyncChainLikeCpp {
    swing: PendingCreatureSwingLikeCpp,
    states: Vec<CreatureMeleeVictimSyncStateLikeCpp>,
}

/// Replay one batch of already-committed canonical Creature victim transitions.
///
/// The complete chain is preflighted before any legacy mutation. Lock order,
/// FIFO state order, incarnation checks and reciprocal threat publication are
/// unchanged from the former inline melee-tick implementation.
pub(super) fn replay_creature_victim_syncs_like_cpp(
    legacy_map_manager: &crate::map_manager::SharedMapManager,
    canonical_map_manager: &SharedCanonicalMapManager,
    creature_victim_syncs: Vec<CreatureVictimCompatibilitySyncLikeCpp>,
    outcome: &mut LegacyCreatureMeleeTickOutcomeLikeCpp,
) {
    // Multiple attackers can commit against one creature during a single
    // batch. Chain their contiguous health revisions so canonical authority is
    // checked once against the final desired tuple, then replay each committed
    // transition into the legacy mirror in FIFO order.
    let mut chains: Vec<CreatureVictimCompatibilitySyncChainLikeCpp> = Vec::new();
    for sync in creature_victim_syncs {
        let contiguous = chains.iter_mut().find(|chain| {
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
            chains.push(CreatureVictimCompatibilitySyncChainLikeCpp {
                swing: sync.swing,
                states: vec![sync.state],
            });
        }
    }

    // Lock order is canonical -> legacy, matching the existing spell-cast
    // validation bridge. Holding canonical authority through the legacy CAS
    // closes the final window where a heal/death/respawn could otherwise make
    // this mirror write stale.
    for chain in chains {
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
                let map = managed.map();
                let victim_is_desired = map
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
                    .unwrap_or(false);
                let attackers_are_desired = chain.states.iter().all(|state| {
                    state.threat.as_ref().is_none_or(|threat| {
                        map.with_creature_like_cpp(threat.attacker_guid, |attacker| {
                            attacker.spawn_id() == threat.attacker_spawn_id
                                && attacker
                                    .loot_authority_like_cpp()
                                    .shares_storage_like_cpp(&threat.attacker_authority)
                                && attacker.loot_authority_like_cpp().lifecycle_like_cpp()
                                    != OwnedLootAuthorityLifecycle::Detached
                        })
                        .unwrap_or(false)
                    })
                });
                Some(victim_is_desired && attackers_are_desired)
            })
            .unwrap_or(false);
        if !canonical_is_desired {
            outcome.legacy_creature_victim_sync_cas_rejections += 1;
            continue;
        }

        let mut legacy_manager = legacy_map_manager
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut threat_attackers: Vec<
            &super::creature_melee_threat::CreatureDamageThreatOutcomeLikeCpp,
        > = Vec::new();
        for state in &chain.states {
            if let Some(threat) = &state.threat
                && !threat_attackers
                    .iter()
                    .any(|existing| existing.attacker_guid == threat.attacker_guid)
            {
                threat_attackers.push(threat);
            }
        }
        let attackers_match = threat_attackers.iter().all(|threat| {
            legacy_manager
                .find_creature(
                    chain.swing.map_id,
                    chain.swing.instance_id,
                    threat.attacker_guid,
                )
                .is_some_and(|attacker| {
                    attacker.creature.spawn_id() == threat.attacker_spawn_id
                        && attacker
                            .creature
                            .loot_authority_like_cpp()
                            .shares_storage_like_cpp(&threat.attacker_authority)
                        && attacker
                            .creature
                            .loot_authority_like_cpp()
                            .lifecycle_like_cpp()
                            != OwnedLootAuthorityLifecycle::Detached
                })
        });
        let chain_matches = legacy_manager
            .find_creature(
                chain.swing.map_id,
                chain.swing.instance_id,
                chain.swing.victim_guid,
            )
            .is_some_and(|victim| {
                let mut health = victim.creature.unit().data().health;
                let mut health_revision = victim.creature.unit().health_state_revision_like_cpp();
                let mut death_state = victim.creature.unit().death_state();
                let mut loot_revision = victim.creature.loot_lifecycle_revision_like_cpp();
                let mut ai_state = victim.creature.ai_ownership().state;
                chain.states.iter().all(|state| {
                    let identity = &state.identity;
                    let before_matches = health == state.victim_health_before
                        && health_revision == state.victim_health_state_revision_before
                        && death_state == identity.death_state_before
                        && loot_revision == identity.loot_lifecycle_revision_before
                        && ai_state == identity.ai_state_before
                        && victim.creature.spawn_id() == identity.spawn_id
                        && victim
                            .creature
                            .loot_authority_like_cpp()
                            .shares_storage_like_cpp(&identity.authority)
                        && victim
                            .creature
                            .unit()
                            .shares_health_state_revision_authority_like_cpp(
                                &identity.health_state_revision_authority,
                            )
                        && victim
                            .creature
                            .loot_authority_like_cpp()
                            .lifecycle_like_cpp()
                            != OwnedLootAuthorityLifecycle::Detached;
                    health = state.victim_health_after;
                    health_revision = state.victim_health_state_revision_after;
                    death_state = identity.death_state_after;
                    loot_revision = identity.loot_lifecycle_revision_after;
                    ai_state = identity.ai_state_after;
                    before_matches
                })
            });
        if !attackers_match || !chain_matches {
            outcome.legacy_creature_victim_sync_cas_rejections += 1;
            continue;
        }
        let game_time_secs = wow_entities::game_time_secs_like_cpp();
        let (all_synced, threat_refs) = {
            let Some(victim) = legacy_manager.find_creature_mut(
                chain.swing.map_id,
                chain.swing.instance_id,
                chain.swing.victim_guid,
            ) else {
                outcome.legacy_creature_victim_sync_cas_rejections += 1;
                continue;
            };
            let mut all_synced = true;
            for state in &chain.states {
                if apply_creature_melee_victim_sync_to_legacy_like_cpp(
                    victim,
                    state,
                    game_time_secs,
                ) {
                    outcome.legacy_creature_victim_syncs += 1;
                } else {
                    outcome.legacy_creature_victim_sync_cas_rejections += 1;
                    all_synced = false;
                    break;
                }
            }
            let threat_refs = if all_synced {
                threat_attackers
                    .iter()
                    .filter_map(|threat| {
                        victim
                            .creature
                            .unit()
                            .subsystems()
                            .combat
                            .threat_ref(threat.attacker_guid)
                            .copied()
                            .map(|threat_ref| (threat.attacker_guid, threat_ref))
                    })
                    .collect()
            } else {
                Vec::new()
            };
            (all_synced, threat_refs)
        };
        if all_synced {
            for threat in threat_attackers {
                let attacker_guid = threat.attacker_guid;
                let Some(attacker) = legacy_manager.find_creature_mut(
                    chain.swing.map_id,
                    chain.swing.instance_id,
                    attacker_guid,
                ) else {
                    outcome.legacy_creature_victim_sync_cas_rejections += 1;
                    continue;
                };
                attacker
                    .creature
                    .unit_mut()
                    .subsystems_mut()
                    .combat
                    .set_in_combat_with(chain.swing.victim_guid, false, false);
                if let Some((_, threat_ref)) =
                    threat_refs.iter().find(|(guid, _)| *guid == attacker_guid)
                {
                    attacker
                        .creature
                        .unit_mut()
                        .subsystems_mut()
                        .combat
                        .put_threatened_by_me_ref(chain.swing.victim_guid, *threat_ref);
                }
            }
        }
    }
}
