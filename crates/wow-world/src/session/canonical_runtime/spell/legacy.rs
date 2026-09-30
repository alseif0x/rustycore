//! Synchronous compatibility transport, retaining the complete PRE lock scope.
use super::*;
use wow_map::map_manager::{SpellAction, SpellProgress};

pub(in crate::session) fn consume_action(canonical: &SharedCanonicalMapManager,
    legacy: &crate::map_manager::SharedMapManager, action: SpellAction,
    config: &LegacyCreatureAggroConfigLikeCpp, plan: &mut RuntimePlan,
) -> Option<SpellOutcome> {
    // PRE validation held canonical -> legacy through facts, reset, hit RNG,
    // cooldown, GUID, wire append and the Hit tombstone. NoopTerrain needs no
    // I/O here. Keep that synchronous serialization contract; the dormant
    // canonical transport below the separate entry is not an exclusion proof.
    let mut manager = canonical.lock().ok()?;
    let mut legacy = legacy.write().unwrap_or_else(|poisoned| poisoned.into_inner());
    let mut progress = catalogs::with_policies(config, |policies|
        manager.prepare_legacy_spell_action(&mut legacy, action, policies));
    loop {
        progress = match progress {
            SpellProgress::Complete(outcome) => return Some(outcome),
            SpellProgress::Publication { completion, continuation } => {
                creature_spell_publication::append_completion(plan, completion);
                match catalogs::with_policies(config, |policies|
                    manager.resume_legacy_spell_publication(&mut legacy, continuation, policies)) {
                    Ok(progress) => progress,
                    // Retain any already-appended publication. The unchanged
                    // guards prevent body replacement in this compatibility
                    // path; do not replay a roll if live validation rejects.
                    Err((_, continuation)) => return Some(continuation.into_partial()),
                }
            }
            SpellProgress::Pending(continuation) => {
                // The real legacy map uses NoopTerrain, so this branch performs
                // its original true LOS, never a provider call under the guard.
                match catalogs::with_policies(config, |policies|
                    manager.resume_legacy_spell_los(&mut legacy, continuation, policies)) {
                    Ok(progress) => progress,
                    Err((_, continuation)) => return Some(continuation.into_partial()),
                }
            }
        };
    }
}
