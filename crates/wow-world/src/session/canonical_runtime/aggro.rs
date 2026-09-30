//! Complete canonical Aggro application adapter; dormant until B6 activation.
use super::super::*;
use wow_map::map_manager::{
    AggroAiFacts, AggroAiKind, AggroAiSelection, AggroAttackDecision,
    AggroCandidate, AggroDistanceFacts, AggroEffect, AggroEffectKind,
    AggroFactionTarget, AggroOutcome, AggroOwnerSnapshot, AggroPolicies,
    AggroSettings, AggroTurretFacts,
};
use wow_map::{ActorAggroProgress, MapObjectTickContinuation, ObjectMapUpdateToken};
use crate::map_manager::LiveTerrainHeights;

pub(in crate::session) mod catalogs;
pub(in crate::session) mod conversion;
pub(in crate::session) mod packets;
mod error;
pub use error::{CanonicalAggroError, CanonicalAggroFailure};

/// The caller owns publication and map finish/finalize. No guard crosses I/O;
/// no successful or failed path silently disposes a pending operation.
pub async fn run_canonical_aggro(
    manager: &SharedCanonicalMapManager,
    tick: &MapObjectTickContinuation,
    token: &mut ObjectMapUpdateToken,
    candidates: Vec<LegacyCreatureAggroCandidateLikeCpp>,
    config: &LegacyCreatureAggroConfigLikeCpp,
    terrain: Option<&Arc<LiveTerrainHeights>>,
) -> Result<LegacyCreatureAggroTickOutcomeLikeCpp, CanonicalAggroError> {
    let partial = LegacyCreatureAggroTickOutcomeLikeCpp::default();
    let candidates = candidates.into_iter().map(conversion::owned_candidate).collect();
    let mut progress = {
        let mut manager = match manager.lock() {
            Ok(manager) => manager,
            Err(_) => return Err(CanonicalAggroError { partial,
                failure: CanonicalAggroFailure::PreparePoisoned { candidates } }),
        };
        let settings = conversion::settings(config, token.key().map_id as u16);
        let result = catalogs::with_policies(config, |policies| manager.prepare_aggro(tick, token,
            candidates, settings, terrain.is_some(), policies));
        match result {
            Ok(progress) => progress,
            Err(failure) => return Err(CanonicalAggroError { partial, failure: CanonicalAggroFailure::Prepare(failure) }),
        }
    };
    loop {
        match progress {
            ActorAggroProgress::Complete(outcome) => {
                let mut result = partial;
                packets::append_outcome(&mut result, outcome);
                return Ok(result);
            }
            ActorAggroProgress::Pending(request) => {
                let Some(terrain) = terrain else {
                    return Err(CanonicalAggroError { partial, failure: CanonicalAggroFailure::MissingTerrain(request) });
                };
                let terrain = Arc::clone(terrain);
                // The worker owns continuation transport throughout the query.
                // A panicked closure cannot provide ownership evidence for a
                // settlement; the manager slot remains busy.
                let (continuation, response) = match tokio::task::spawn_blocking(move || request.resolve(&terrain)).await {
                    Ok(reply) => reply,
                    Err(_) => return Err(CanonicalAggroError { partial, failure: CanonicalAggroFailure::QueryPanicked }),
                };
                let mut manager = match manager.lock() {
                    Ok(manager) => manager,
                    Err(_) => return Err(CanonicalAggroError { partial,
                        failure: CanonicalAggroFailure::ResumePoisoned { continuation, response } }),
                };
                let result = catalogs::with_policies(config, |policies|
                    manager.resume_aggro_assistance_los(tick, token, continuation, response, policies));
                progress = match result {
                    Ok(progress) => progress,
                    Err(failure) => return Err(CanonicalAggroError { partial, failure: CanonicalAggroFailure::Resume(failure) }),
                };
            }
        }
    }
}

#[cfg(test)]
mod tests;
