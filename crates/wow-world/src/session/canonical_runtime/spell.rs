//! Complete canonical Spell application operation; producer remains dormant.
//! Identity/slot gates exclude tail and other Actor operations, not every
//! writer: Player operations and legacy writers do not honor this slot. LOS
//! suspension requires a future writer-exclusion contract before activation;
//! identity revalidation alone does not freeze aura/stats/cast/position facts.
use super::super::*;
use wow_map::map_manager::{SpellAiKind, SpellInfoFacts, SpellEffectFacts, SpellPowerFacts,
    SpellCondition, SpellTarget, SpellDisable, SpellPreparationCheck, SpellCooldown,
    SpellRange, SpellHitFacts, SpellFactionFacts, SpellHit, SpellLog, SpellOutcome, SpellPolicies};
use wow_map::{ActorSpellProgress, ActorSpellLosQuery, MapObjectTickContinuation, ObjectMapUpdateToken};

pub(in crate::session) mod catalogs;
pub(in crate::session) mod projection;
pub(in crate::session) mod packets;
mod error;
mod disposition;
pub(in crate::session) mod legacy;
pub use error::{CanonicalSpellError, CanonicalSpellFailure};
pub use disposition::{CanonicalSpellAbandoned, CanonicalSpellAbandonment};

/// APP owns the existing terrain/provider query. Endpoints are already shaped;
/// this callback receives neither a live entity nor map storage/guards.
pub type CanonicalSpellLosResolver = dyn Fn(ActorSpellLosQuery) -> bool + Send + Sync;

pub async fn run_canonical_spell(manager: &SharedCanonicalMapManager,
    tick: &MapObjectTickContinuation, token: &mut ObjectMapUpdateToken,
    config: &LegacyCreatureAggroConfigLikeCpp,
    line_of_sight: Option<&Arc<CanonicalSpellLosResolver>>,
) -> Result<LegacyCreatureSpellTickOutcomeLikeCpp, CanonicalSpellError> {
    let mut partial = LegacyCreatureSpellTickOutcomeLikeCpp::default();
    // Same missing-store early return as the compatibility producer.
    if config.spell_store.is_none() { return Ok(partial); }
    let mut progress = {
        let mut manager = manager.lock().map_err(|_| CanonicalSpellError {
            partial: LegacyCreatureSpellTickOutcomeLikeCpp::default(), failure: CanonicalSpellFailure::PreparePoisoned })?;
        catalogs::with_policies(config, |policies| manager.prepare_spell(tick, token,
            line_of_sight.is_some(), policies))
            .map_err(|failure| CanonicalSpellError { partial: LegacyCreatureSpellTickOutcomeLikeCpp::default(),
                failure: CanonicalSpellFailure::Prepare(failure) })?
    };
    loop {
        match progress {
            ActorSpellProgress::Complete(outcome) => {
                packets::replace_outcome(&mut partial, outcome);
                return Ok(partial);
            }
            ActorSpellProgress::Publication { completion, continuation } => {
                packets::replace_counters(&mut partial, continuation.partial());
                creature_spell_publication::append_completion(&mut partial.plan, completion);
                // Wire timestamp/append precede the Hit tombstone and any next
                // Schedule draw. No entity or manager guard crosses publication.
                let mut guard = match manager.lock() {
                    Ok(guard) => guard,
                    Err(_) => return Err(CanonicalSpellError { partial,
                        failure: CanonicalSpellFailure::PublicationPoisoned(continuation) }),
                };
                progress = match catalogs::with_policies(config, |policies|
                    guard.resume_spell_publication(tick, token, continuation, policies)) {
                    Ok(progress) => progress,
                    Err(failure) => return Err(CanonicalSpellError { partial,
                        failure: CanonicalSpellFailure::Publication(failure) }),
                };
            }
            ActorSpellProgress::Pending(request) => {
                packets::replace_counters(&mut partial, request.partial());
                let Some(resolve) = line_of_sight else {
                    return Err(CanonicalSpellError { partial, failure: CanonicalSpellFailure::MissingTerrain(request) });
                };
                let resolve = Arc::clone(resolve);
                let (continuation, response) = match tokio::task::spawn_blocking(move || request.resolve(|query| resolve(query))).await {
                    Ok(reply) => reply,
                    Err(_) => return Err(CanonicalSpellError { partial, failure: CanonicalSpellFailure::QueryPanicked }),
                };
                progress = {
                    let mut manager = match manager.lock() {
                        Ok(manager) => manager,
                        Err(_) => return Err(CanonicalSpellError { partial,
                            failure: CanonicalSpellFailure::ResumePoisoned { continuation, response } }),
                    };
                    match catalogs::with_policies(config, |policies|
                        manager.resume_spell_los(tick, token, continuation, response, policies)) {
                        Ok(progress) => progress,
                        Err(failure) => return Err(CanonicalSpellError { partial, failure: CanonicalSpellFailure::Resume(failure) }),
                    }
                };
            }
        }
    }
}
