use super::*;
use crate::map::{CreatureActorAdmission, CreatureActorWitness};

// Fixture transport happens before the motor; no actor leaves its slot while
// the shared operation runs.
fn canonical_source(
    player_victim: bool,
    distance: f32,
) -> (
    MapManager,
    PendingCreatureSwingLikeCpp,
    CreatureActorWitness,
) {
    let (mut manager, attacker, swing) = setup(player_victim, distance);
    let map = manager.find_map_mut(0, 0).unwrap().map_mut();
    drop(map.remove_map_object(swing.attacker_guid).unwrap());
    let CreatureActorAdmission::Inserted { witness } = map.admit_creature_actor(attacker).unwrap()
    else {
        panic!("the source fixture requires a fresh actor slot");
    };
    (manager, swing, witness)
}

#[test]
fn legacy_and_canonical_sources_share_damage_absorb_order_and_revision_receipts() {
    let (mut legacy, mut actor, swing) = setup(true, 1.0);
    let (mut canonical, selected, witness) = canonical_source(true, 1.0);
    let mut results = Vec::new();
    for (manager, external) in [(&mut legacy, Some(&mut actor)), (&mut canonical, None)] {
        let player = manager
            .find_map_mut(0, 0)
            .unwrap()
            .map_mut()
            .get_typed_player_mut(swing.victim_guid)
            .unwrap();
        let revision = player.unit().health_state_revision_like_cpp();
        player
            .unit_mut()
            .set_power_index(wow_constants::PowerType::Mana, Some(0));
        player
            .unit_mut()
            .set_max_power(wow_constants::PowerType::Mana, 20);
        player
            .unit_mut()
            .set_power(wow_constants::PowerType::Mana, 20);
        player
            .unit_mut()
            .subsystems_mut()
            .auras
            .insert_runtime_application_like_cpp(application(1, 4));
        player
            .unit_mut()
            .subsystems_mut()
            .auras
            .insert_runtime_application_like_cpp(application(2, 3));
        let mut catalogs = Catalogs::inert();
        catalogs.school.push(RepresentedAbsorbShieldLikeCpp {
            slot: 1,
            amount: 4,
            ..Default::default()
        });
        catalogs.mana.push(RepresentedManaShieldLikeCpp {
            slot: 2,
            amount: 3,
            mana_multiplier: 2.0,
            ..Default::default()
        });
        let outcome = if let Some(actor) = external {
            manager.apply_legacy_creature_melee_swing(Some(actor), swing, &catalogs)
        } else {
            manager.apply_canonical_creature_melee_swing(
                crate::MapKey::new(0, 0),
                selected.attacker_guid,
                witness.clone(),
                selected,
                &catalogs,
            )
        };
        let command = &outcome.commands[0];
        results.push((
            command.damage,
            command.absorbed,
            command.mana_spent,
            command.victim_health_after,
            command.victim_health_state_revision_after - revision,
            command
                .absorb_consumptions
                .iter()
                .map(|c| (c.slot, c.consumed, c.removed))
                .collect::<Vec<_>>(),
        ));
        let calls = catalogs.calls.borrow();
        let school = calls.iter().position(|c| *c == "school").unwrap();
        assert_eq!(calls[school + 1], "mana");
        assert!(school < calls.iter().position(|c| *c == "share").unwrap());
        assert_eq!(
            manager
                .find_map(0, 0)
                .unwrap()
                .map()
                .get_typed_player(swing.victim_guid)
                .unwrap()
                .unit()
                .get_power(wow_constants::PowerType::Mana),
            14
        );
    }
    assert_eq!(results[0], results[1]);
    assert_eq!(
        results[1],
        (3, 7, 6, 97, 1, vec![(1, 4, true), (2, 3, true)])
    );
    assert!(
        canonical
            .find_map(0, 0)
            .unwrap()
            .map()
            .creature_actor_witness(selected.attacker_guid)
            .unwrap()
            .same_actor(&witness)
    );
}

#[test]
fn canonical_actor_stays_available_to_primary_and_reciprocal_threat_lookups() {
    let (mut manager, swing, witness) = canonical_source(false, 1.0);
    let outcome = manager.apply_canonical_creature_melee_swing(
        crate::MapKey::new(0, 0),
        swing.attacker_guid,
        witness.clone(),
        swing,
        &Catalogs::default(),
    );
    assert_eq!(
        (outcome.canonical_hits, outcome.canonical_creature_hits),
        (1, 1)
    );
    assert_eq!(health(&manager, swing.victim_guid), 90);
    assert_eq!(outcome.syncs[0].state.threat.as_ref().unwrap().delta, 10.0);
    let actor = manager
        .find_map(0, 0)
        .unwrap()
        .map()
        .creature_actor(swing.attacker_guid)
        .unwrap();
    assert!(actor.creature.unit().subsystems().combat.has_combat());
    assert_eq!(
        actor.creature.ai_ownership().swing_timer_ms,
        actor.create_data.base_attack_time as u64
    );
    assert!(!actor.runtime_rng_authority_complete_like_cpp());
    assert!(
        manager
            .find_map(0, 0)
            .unwrap()
            .map()
            .creature_actor_witness(swing.attacker_guid)
            .unwrap()
            .same_actor(&witness)
    );
    assert!(matches!(
        &outcome.events[0],
        MeleeEffect::AttackState { .. }
    ));
    assert!(matches!(&outcome.events[1], MeleeEffect::Values { .. }));
}

#[test]
fn canonical_self_victim_lethal_commit_still_rearms_without_a_late_alive_gate() {
    let (mut manager, mut swing, witness) = canonical_source(false, 1.0);
    swing.victim_guid = swing.attacker_guid;
    let actor = manager
        .find_map_mut(0, 0)
        .unwrap()
        .map_mut()
        .creature_actor_mut(swing.attacker_guid)
        .unwrap();
    actor.enter_combat(swing.attacker_guid);
    actor.creature.unit_mut().set_health(5);
    let outcome = manager.apply_canonical_creature_melee_swing(
        crate::MapKey::new(0, 0),
        swing.attacker_guid,
        witness,
        swing,
        &Catalogs::default(),
    );
    assert_eq!(outcome.canonical_creature_hits, 1);
    assert_eq!(health(&manager, swing.attacker_guid), 0);
    assert!(outcome.syncs[0].state.threat.is_none());
    let actor = manager
        .find_map(0, 0)
        .unwrap()
        .map()
        .creature_actor(swing.attacker_guid)
        .unwrap();
    assert_eq!(
        actor.creature.unit().death_state(),
        wow_constants::DeathState::JustDied
    );
    assert_eq!(
        actor.creature.ai_ownership().swing_timer_ms,
        actor.create_data.base_attack_time as u64
    );
}

#[test]
fn canonical_self_share_retains_secondary_commit_when_primary_revalidation_rejects() {
    let (mut manager, swing, witness) = canonical_source(false, 1.0);
    let applied = AppliedAuraRef::new(42, swing.attacker_guid, 3, 1);
    manager
        .find_map_mut(0, 0)
        .unwrap()
        .map_mut()
        .get_typed_creature_mut(swing.victim_guid)
        .unwrap()
        .unit_mut()
        .subsystems_mut()
        .auras
        .applied_auras
        .push(applied);
    let mut catalogs = Catalogs::inert();
    catalogs.shares.push(ShareAuraSnapshotLikeCpp {
        identity: ShareAuraIdentityLikeCpp::Creature {
            applied,
            effect_index: 0,
        },
        caster_guid: swing.attacker_guid,
        school_mask: 1,
        amount: 1_000,
    });
    let outcome = manager.apply_canonical_creature_melee_swing(
        crate::MapKey::new(0, 0),
        swing.attacker_guid,
        witness,
        swing,
        &catalogs,
    );
    assert_eq!(outcome.melee_precondition_rejections, 1);
    assert_eq!(outcome.canonical_hits, 0);
    assert_eq!(health(&manager, swing.victim_guid), 100);
    assert_eq!(health(&manager, swing.attacker_guid), 0);
    assert!(outcome.events.is_empty() && outcome.commands.is_empty());
    assert_eq!(outcome.syncs.len(), 1);
    assert_eq!(outcome.syncs[0].swing.victim_guid, swing.attacker_guid);
    assert_eq!(outcome.syncs[0].state.victim_health_after, 0);
    let actor = manager
        .find_map(0, 0)
        .unwrap()
        .map()
        .creature_actor(swing.attacker_guid)
        .unwrap();
    assert_eq!(actor.creature.ai_ownership().swing_timer_ms, 0);
    assert_eq!(outcome.attacking_interrupt_auras_removed, 0);
}
