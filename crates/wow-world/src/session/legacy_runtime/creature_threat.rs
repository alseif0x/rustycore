//! Compatibility threat query/mirror seams and shared Aggro victim selection.
use super::*;
use super::super::canonical_runtime::aggro::{conversion, catalogs};

pub(in crate::session) fn legacy_creature_update_threat_victim_like_cpp(actor: &mut crate::map_manager::WorldCreature,
    candidates: &[&LegacyCreatureAggroCandidateLikeCpp], config: &LegacyCreatureAggroConfigLikeCpp,
    owners: &HashMap<ObjectGuid, LegacyCreatureAggroOwnerSnapshotLikeCpp>) -> LegacyCreatureThreatUpdateLikeCpp {
    let candidates: Vec<_> = candidates.iter().map(|candidate| conversion::owned_candidate((*candidate).clone())).collect();
    let refs: Vec<_> = candidates.iter().collect();
    let settings = conversion::settings(config, actor.map_id() as u16);
    let result = catalogs::with_policies(config, |policies| wow_map::map_manager::update_threat_victim(actor,
        &refs, &settings, &conversion::owners(owners), policies));
    match result {
        wow_map::map_manager::AggroThreatUpdate::Unchanged => LegacyCreatureThreatUpdateLikeCpp::Unchanged,
        wow_map::map_manager::AggroThreatUpdate::Switched { previous_victim } => LegacyCreatureThreatUpdateLikeCpp::Switched { previous_victim },
        wow_map::map_manager::AggroThreatUpdate::Evade { previous_victim, participant_guids, removed_taunt_slots } =>
            LegacyCreatureThreatUpdateLikeCpp::Evade { previous_victim, participant_guids, removed_taunt_slots },
    }
}
/// Read a canonical creature's threat toward one attacker, from an already
/// locked map.
///
/// Lifted by #28: the session variant took the canonical lock itself, so a
/// caller that already held the map had to drop it and take it again.
pub(in crate::session) fn creature_threat_value_on_map_like_cpp(
    map: &wow_map::ManagedMapInnerLikeCpp,
    creature_guid: ObjectGuid,
    attacker_guid: ObjectGuid,
) -> Option<f32> {
    map.with_creature_like_cpp(creature_guid, |creature| {
        creature
            .unit()
            .subsystems()
            .combat
            .threat_value(attacker_guid)
    })
    .flatten()
}
/// Mirror a legacy creature's threat toward its attacker into the canonical
/// map, on an already locked map.
///
/// Lifted by #28. The session variant acquired the canonical lock twice — once
/// inside `begin_canonical_player_combat_ref_like_cpp` and again for the mirror
/// itself. Taking the map as an argument collapses that to one acquisition for
/// both halves, which is also what lets the sessionless loop call it.
pub(in crate::session) fn mirror_creature_threat_from_attacker_on_map_like_cpp(
    map: &mut wow_map::ManagedMapInnerLikeCpp,
    creature_guid: ObjectGuid,
    attacker_guid: ObjectGuid,
    threat_value: f32,
) -> bool {
    if threat_value <= 0.0 {
        return false;
    }
    if !begin_combat_ref_on_map_like_cpp(map, attacker_guid, creature_guid, false, false, false) {
        return false;
    }

    let threat_ref = {
        let Some(creature) = map.get_typed_creature_mut(creature_guid) else {
            return false;
        };
        creature
            .unit_mut()
            .subsystems_mut()
            .combat
            .set_threat(attacker_guid, threat_value);
        creature
            .unit()
            .subsystems()
            .combat
            .threat_ref(attacker_guid)
            .copied()
    };

    let Some(threat_ref) = threat_ref else {
        return false;
    };
    let Some(attacker) = map.get_typed_player_mut(attacker_guid) else {
        return false;
    };
    attacker
        .unit_mut()
        .subsystems_mut()
        .combat
        .put_threatened_by_me_ref(creature_guid, threat_ref);
    true
}
