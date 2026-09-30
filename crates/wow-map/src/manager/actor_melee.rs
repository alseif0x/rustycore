//! One selected, synchronous creature-melee operation under the caller's guard.
//! The old entry opens no slot; original kill capture is a separate opt-in entry.
use super::{MapManager, MapObjectTickContinuation, ObjectMapUpdateToken,
    ObjectMapTickError, ActorTickAccessError};
use crate::map::{CreatureMeleeCatalogsLikeCpp, CreatureMeleeReadiness,
    CreatureMeleeSwingOutcome, creature_melee_readiness};
use wow_core::ObjectGuid;

mod kill_origin;
pub(crate) use kill_origin::MeleeKillCollector;
pub use kill_origin::{SelectedMeleeExecution, PendingMeleeKills, PreparedMeleeKill, PreparedMeleeLoot,
    MeleeKillPhaseError, MeleeKillCaptureError, MeleeLootError};

impl MapManager {
    pub(crate) fn apply_selected_creature_melee(
        &mut self,
        tick: &MapObjectTickContinuation,
        token: &mut ObjectMapUpdateToken,
        guid: ObjectGuid,
        catalogs: &impl CreatureMeleeCatalogsLikeCpp,
    ) -> Result<CreatureMeleeSwingOutcome, ActorTickAccessError> {
        self.require_current_actor_token(tick, token)?;
        if let Some(operation) = &token.actor_operation {
            return Err(ActorTickAccessError::Tick(ObjectMapTickError::ActorOperationInFlight {
                guid: operation.guid,
            }));
        }
        let witness = self.selected_actor_witness(token, guid, None)?;
        let key = token.key();
        // The current swing/publication model represents u16 map IDs. Reject
        // an unrepresentable key without looking up a different truncated map.
        let Ok(map_id) = u16::try_from(key.map_id) else {
            return Ok(CreatureMeleeSwingOutcome {
                melee_precondition_rejections: 1,
                ..Default::default()
            });
        };
        let readiness = {
            let actor = self.maps.get(&key).expect("the validated token retains its map")
                .map().creature_actor(guid).expect("the admitted witness retains its actor");
            creature_melee_readiness(actor, map_id, key.instance_id)
        };
        match readiness {
            CreatureMeleeReadiness::NotReady => Ok(CreatureMeleeSwingOutcome::default()),
            CreatureMeleeReadiness::Rejected => Ok(CreatureMeleeSwingOutcome {
                melee_precondition_rejections: 1,
                ..Default::default()
            }),
            CreatureMeleeReadiness::Ready(swing) => Ok(self.apply_canonical_creature_melee_swing(
                key, guid, witness, swing, catalogs)),
        }
    }
    /// Produces kill provenance only; it does not schedule or complete Kill work.
    pub fn apply_selected_creature_melee_with_kills(
        &mut self,
        tick: &MapObjectTickContinuation,
        mut token: ObjectMapUpdateToken,
        guid: ObjectGuid,
        catalogs: &impl CreatureMeleeCatalogsLikeCpp,
    ) -> Result<SelectedMeleeExecution, (ActorTickAccessError, ObjectMapUpdateToken)> {
        let admission = (|| {
            self.require_current_actor_token(tick, &token)?;
            if let Some(operation) = &token.actor_operation {
                return Err(ActorTickAccessError::Tick(ObjectMapTickError::ActorOperationInFlight {
                    guid: operation.guid,
                }));
            }
            let witness = self.selected_actor_witness(&mut token, guid, None)?;
            let key = token.key();
            let readiness = match u16::try_from(key.map_id) {
                Ok(map_id) => {
                    let actor = self.maps.get(&key).expect("the validated token retains its map")
                        .map().creature_actor(guid).expect("the admitted witness retains its actor");
                    creature_melee_readiness(actor, map_id, key.instance_id)
                }
                Err(_) => CreatureMeleeReadiness::Rejected,
            };
            Ok((key, witness, readiness))
        })();
        let (key, witness, readiness) = match admission {
            Ok(admitted) => admitted,
            Err(error) => return Err((error, token)),
        };
        let mut kills = Some(MeleeKillCollector::new(&mut token, guid, witness.clone()));
        let outcome = match readiness {
            CreatureMeleeReadiness::NotReady => CreatureMeleeSwingOutcome::default(),
            CreatureMeleeReadiness::Rejected => CreatureMeleeSwingOutcome {
                melee_precondition_rejections: 1,
                ..Default::default()
            },
            CreatureMeleeReadiness::Ready(swing) => self.apply_canonical_creature_melee_swing_with_kills(
                key, guid, witness, swing, catalogs, &mut kills),
        };
        let batch = kills.take().expect("the producer retains its collector").finish();
        drop(kills);
        Ok(SelectedMeleeExecution::new(outcome, token, batch))
    }
}

#[cfg(test)]
mod tests;
