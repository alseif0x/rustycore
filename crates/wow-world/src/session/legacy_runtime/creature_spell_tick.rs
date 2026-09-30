//! Compatibility producer prepares all maps before the original ordered drain.
//! AI, admission, scheduling and hit rules use the single Map-owned motor.
use super::*;
use super::super::canonical_runtime::spell::{catalogs, packets, legacy};
use wow_map::map_manager::SpellAction;

pub fn run_legacy_creature_spell_tick_once_like_cpp(
    legacy_map_manager: &crate::map_manager::SharedMapManager,
    canonical_map_manager: Option<&SharedCanonicalMapManager>,
    config: &LegacyCreatureAggroConfigLikeCpp,
) -> LegacyCreatureSpellTickOutcomeLikeCpp {
    let mut outcome = LegacyCreatureSpellTickOutcomeLikeCpp::default();
    if config.spell_store.is_none() { return outcome; }
    let mut difficulties = HashMap::new();
    if let Some(canonical) = canonical_map_manager && let Ok(manager) = canonical.lock() {
        manager.do_for_all_maps(|managed| {
            if let Ok(map_id) = u16::try_from(managed.map_id()) {
                difficulties.insert((map_id, managed.instance_id()), managed.difficulty());
            }
        });
    }
    let mut actions = Vec::new();
    {
        let mut manager = legacy_map_manager.write().unwrap_or_else(|poisoned| poisoned.into_inner());
        if manager.tick_owner() != crate::map_manager::RuntimeTickOwner::GlobalLegacy {
            outcome.skipped_owner_not_global = true;
            return outcome;
        }
        for (map_id, instance_id) in manager.active_map_keys() {
            let difficulty = difficulties.get(&(map_id, instance_id)).copied().unwrap_or(0);
            let queue = catalogs::with_policies(config, |policies|
                manager.prepare_spell_map(map_id, instance_id, difficulty, policies));
            let (prepared, counters) = queue.into_parts();
            packets::add_outcome(&mut outcome, counters);
            actions.extend(prepared);
        }
    }
    for action in actions {
        let cast = matches!(&action, SpellAction::Cast(_));
        if let SpellAction::Schedule(schedule) = action {
            let mut legacy = legacy_map_manager.write().unwrap_or_else(|poisoned| poisoned.into_inner());
            if !legacy.consume_spell_schedule(schedule) { outcome.runtime_rng_authority_rejections += 1; }
            continue;
        }
        let Some(canonical) = canonical_map_manager else {
            if cast {
                outcome.canonical_cast_missing_target += 1;
                outcome.casts_ready = outcome.casts_ready.saturating_sub(1);
            }
            continue;
        };
        let Some(mut delta) = legacy::consume_action(canonical, legacy_map_manager, action,
            config, &mut outcome.plan) else {
            if cast {
                outcome.canonical_cast_missing_target += 1;
                outcome.casts_ready = outcome.casts_ready.saturating_sub(1);
            }
            continue;
        };
        if cast && delta.canonical_cast_preconditions_passed == 0 {
            outcome.casts_ready = outcome.casts_ready.saturating_sub(1);
        }
        delta.casts_ready = 0;
        packets::add_outcome(&mut outcome, delta);
    }
    outcome
}
