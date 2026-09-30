//! Compatibility transport uses the same frame and the original NoopTerrain.
//! APP acknowledges wire append before the Hit tombstone/next queue action.
use super::*;
impl MapManager {
    pub fn prepare_legacy_spell_action(
        &mut self,
        legacy: &mut LegacyMapManager,
        action: SpellAction,
        policies: &mut SpellPolicies<'_>,
    ) -> SpellProgress {
        let key = action.key();
        let map = self
            .find_map_mut(key.map_id, key.instance_id)
            .map(|managed| managed.map_mut());
        let mut backend = SpellMap::Legacy {
            manager: legacy,
            map,
            map_id: key.map_id as u16,
            instance_id: key.instance_id,
        };
        SpellQueue::from_action(action).consume(&mut backend, policies, false)
    }

    pub fn resume_legacy_spell_publication(
        &mut self,
        legacy: &mut LegacyMapManager,
        continuation: SpellPublicationPending,
        policies: &mut SpellPolicies<'_>,
    ) -> Result<SpellProgress, (SpellValidation, SpellPublicationPending)> {
        let key = continuation.key();
        let (caster, _) = continuation.referenced_guids();
        let map = self
            .find_map_mut(key.map_id, key.instance_id)
            .map(|managed| managed.map_mut());
        let validation = match (
            legacy.find_creature(key.map_id as u16, key.instance_id, caster),
            map.as_deref()
                .and_then(|map| map.get_typed_creature(caster)),
        ) {
            (Some(actor), Some(body)) => continuation.validate_live_cast(actor, body),
            _ => Err(SpellValidation::MissingTarget),
        };
        if let Err(error) = validation {
            return Err((error, continuation));
        }
        let mut backend = SpellMap::Legacy {
            manager: legacy,
            map,
            map_id: key.map_id as u16,
            instance_id: key.instance_id,
        };
        Ok(continuation.resume(&mut backend, policies, false))
    }

    pub fn resume_legacy_spell_los(
        &mut self,
        legacy: &mut LegacyMapManager,
        continuation: SpellLosPending,
        policies: &mut SpellPolicies<'_>,
    ) -> Result<SpellProgress, (SpellValidation, SpellLosPending)> {
        let key = continuation.key();
        let (caster, _) = continuation.referenced_guids();
        let map = self
            .find_map_mut(key.map_id, key.instance_id)
            .map(|managed| managed.map_mut());
        let validation = match (
            legacy.find_creature(key.map_id as u16, key.instance_id, caster),
            map.as_deref()
                .and_then(|map| map.get_typed_creature(caster)),
        ) {
            (Some(actor), Some(body)) => continuation.validate_live_cast(actor, body),
            _ => Err(SpellValidation::MissingTarget),
        };
        if let Err(error) = validation {
            return Err((error, continuation));
        }
        let mut backend = SpellMap::Legacy {
            manager: legacy,
            map,
            map_id: key.map_id as u16,
            instance_id: key.instance_id,
        };
        // NoopTerrain LOS is true; no provider is queried by this branch.
        Ok(continuation.resume(&mut backend, true, policies, false))
    }
}
