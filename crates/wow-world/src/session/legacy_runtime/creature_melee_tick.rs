//! Application lock, catalog and publication boundary for creature melee.
use super::creature_melee_sync::PendingCreatureSwingLikeCpp;
use super::*;
use wow_map::map::{CreatureMeleeReadiness, creature_melee_readiness};

mod catalogs;
mod publication;

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
    creature.apply_player_melee(player_guid, tap_group_guids, canonical_swings)
}

/// Run the complete synchronous Map motor, retaining canonical -> legacy
/// guards for each swing and the original unlocked windows between swings.
pub fn run_legacy_creature_melee_tick_once_like_cpp(
    legacy_map_manager: &crate::map_manager::SharedMapManager,
    canonical_map_manager: Option<&SharedCanonicalMapManager>,
    config: &LegacyCreatureAggroConfigLikeCpp,
) -> LegacyCreatureMeleeTickOutcomeLikeCpp {
    use crate::map_manager::RuntimeTickOwner;
    let mut outcome = LegacyCreatureMeleeTickOutcomeLikeCpp::default();
    let mut pending_swings = Vec::new();
    {
        let mut manager = legacy_map_manager.write().unwrap_or_else(|poisoned| poisoned.into_inner());
        if manager.tick_owner() != RuntimeTickOwner::GlobalLegacy {
            outcome.skipped_owner_not_global = true;
            return outcome;
        }
        let map_keys = manager.active_map_keys();
        outcome.maps_seen = map_keys.len();
        for (map_id, instance_id) in map_keys {
            for guid in manager.creature_guids(map_id, instance_id) {
                let Some(creature) = manager.find_creature_mut(map_id, instance_id, guid) else { continue; };
                outcome.creatures_seen += 1;
                match creature_melee_readiness(creature, map_id, instance_id) {
                    CreatureMeleeReadiness::NotReady => {}
                    CreatureMeleeReadiness::Rejected => outcome.melee_precondition_rejections += 1,
                    CreatureMeleeReadiness::Ready(swing) => {
                        pending_swings.push(swing);
                        outcome.swings_ready += 1;
                    }
                }
            }
        }
    }
    let Some(canonical_map_manager) = canonical_map_manager else { return outcome; };
    let mut creature_victim_syncs = Vec::new();
    for swing in pending_swings {
        let result = {
            let Ok(mut canonical_manager) = canonical_map_manager.lock() else {
                outcome.melee_precondition_rejections += 1;
                continue;
            };
            let mut legacy_manager = legacy_map_manager.write().unwrap_or_else(|poisoned| poisoned.into_inner());
            let attacker = legacy_manager.find_creature_mut(swing.map_id, swing.instance_id, swing.attacker_guid);
            canonical_manager.apply_legacy_creature_melee_swing(attacker, swing, &catalogs::Catalogs(config))
        }; // Both guards end before any packet encoding/publication.
        outcome.melee_precondition_rejections += result.melee_precondition_rejections;
        outcome.attacker_incarnation_rejections += result.attacker_incarnation_rejections;
        outcome.melee_range_rejections += result.melee_range_rejections;
        outcome.melee_facing_rejections += result.melee_facing_rejections;
        outcome.attacker_state_rejections += result.attacker_state_rejections;
        outcome.melee_los_rejections += result.melee_los_rejections;
        outcome.attacking_interrupt_auras_removed += result.attacking_interrupt_auras_removed;
        outcome.melee_outcomes_unrepresented += result.melee_outcomes_unrepresented;
        outcome.canonical_hits += result.canonical_hits;
        outcome.canonical_creature_hits += result.canonical_creature_hits;
        outcome.commands.extend(result.commands.into_iter().map(publication::command));
        outcome.plan.events.extend(result.events.into_iter().filter_map(|item| publication::effect(item, &swing)));
        creature_victim_syncs.extend(result.syncs);
    }
    super::creature_melee_sync::replay_creature_victim_syncs_like_cpp(
        legacy_map_manager, canonical_map_manager, creature_victim_syncs, &mut outcome);
    outcome
}
