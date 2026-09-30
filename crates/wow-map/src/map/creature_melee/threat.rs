//! Catalog planning and reciprocal nonlethal threat settlement.
use super::*;

pub(super) fn plan_creature_damage_threat_like_cpp(
    map: &ManagedMapInnerLikeCpp,
    attacker_guid: ObjectGuid,
    spell_id: Option<i32>,
    catalogs: &impl CreatureMeleeCatalogsLikeCpp,
    difficulty: u8,
) -> CreatureDamageThreatPlanLikeCpp {
    let facts = catalogs.threat_spell(spell_id, difficulty);
    let aura_multiplier = map
        .with_creature_like_cpp(attacker_guid, |attacker| {
            if !catalogs.represented() {
                return 1.0;
            }
            catalogs.threat_aura(
                &attacker.unit().subsystems().auras.applied_auras,
                difficulty,
                facts.school_mask,
            )
        })
        .unwrap_or(1.0);
    CreatureDamageThreatPlanLikeCpp {
        suppress: facts.suppress,
        no_initial_threat: facts.no_initial_threat,
        multiplier: facts.multiplier * aura_multiplier,
    }
}

pub(super) fn apply_creature_damage_threat_on_map_like_cpp(
    map: &mut ManagedMapInnerLikeCpp,
    victim_guid: ObjectGuid,
    attacker_guid: ObjectGuid,
    damage_taken: u32,
    plan: CreatureDamageThreatPlanLikeCpp,
) -> Option<CreatureDamageThreatOutcomeLikeCpp> {
    if victim_guid == attacker_guid || plan.suppress {
        return None;
    }
    let attacker_identity = map.with_creature_like_cpp(attacker_guid, |attacker| {
        (
            attacker.is_alive(),
            attacker.loot_authority_like_cpp().clone(),
            attacker.spawn_id(),
        )
    });
    let admitted = map
        .with_creature_like_cpp(victim_guid, |victim| {
            victim.is_alive()
                && (!plan.no_initial_threat || victim.unit().subsystems().combat.has_combat())
        })
        .unwrap_or(false)
        && attacker_identity
            .as_ref()
            .is_some_and(|(alive, _, _)| *alive);
    if !admitted {
        return None;
    }

    let amount = damage_taken as f32 * plan.multiplier;
    let victim_started = map
        .get_typed_creature_mut(victim_guid)?
        .unit_mut()
        .subsystems_mut()
        .combat
        .set_in_combat_with(attacker_guid, false, false);
    let attacker_started = map
        .get_typed_creature_mut(attacker_guid)?
        .unit_mut()
        .subsystems_mut()
        .combat
        .set_in_combat_with(victim_guid, false, false);
    if !victim_started || !attacker_started {
        return None;
    }

    map.get_typed_creature_mut(victim_guid)?
        .enter_ai_combat(attacker_guid);

    let threat_ref = {
        let victim = map.get_typed_creature_mut(victim_guid)?;
        victim
            .unit_mut()
            .subsystems_mut()
            .combat
            .add_threat(attacker_guid, amount);
        victim
            .unit()
            .subsystems()
            .combat
            .threat_ref(attacker_guid)
            .copied()
    };
    if let Some(threat_ref) = threat_ref {
        map.get_typed_creature_mut(attacker_guid)?
            .unit_mut()
            .subsystems_mut()
            .combat
            .put_threatened_by_me_ref(victim_guid, threat_ref);
    }
    let (_, attacker_authority, attacker_spawn_id) = attacker_identity?;
    Some(CreatureDamageThreatOutcomeLikeCpp {
        attacker_guid,
        attacker_authority,
        attacker_spawn_id,
        delta: amount,
    })
}
