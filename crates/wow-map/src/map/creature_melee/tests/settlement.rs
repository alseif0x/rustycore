use super::*;

#[test]
fn school_then_mana_use_original_aura_snapshot_and_capture_post_commit_revision() {
    let (mut manager, mut attacker, swing) = setup(true, 1.0);
    let player = manager.find_map_mut(0, 0).unwrap().map_mut().get_typed_player_mut(swing.victim_guid).unwrap();
    let revision = player.unit().health_state_revision_like_cpp();
    player.unit_mut().set_power_index(wow_constants::PowerType::Mana, Some(0));
    player.unit_mut().set_max_power(wow_constants::PowerType::Mana, 20);
    player.unit_mut().set_power(wow_constants::PowerType::Mana, 20);
    player.unit_mut().subsystems_mut().auras.insert_runtime_application_like_cpp(application(1, 4));
    player.unit_mut().subsystems_mut().auras.insert_runtime_application_like_cpp(application(2, 3));
    let mut catalogs = Catalogs::inert();
    catalogs.school.push(RepresentedAbsorbShieldLikeCpp { slot: 1, amount: 4, ..Default::default() });
    catalogs.mana.push(RepresentedManaShieldLikeCpp { slot: 2, amount: 3, mana_multiplier: 2.0, ..Default::default() });
    let outcome = manager.apply_legacy_creature_melee_swing(Some(&mut attacker), swing, &catalogs);
    let command = &outcome.commands[0];
    assert_eq!((command.damage, command.absorbed, command.mana_spent), (3, 7, 6));
    assert_eq!(command.victim_health_after, 97);
    assert_eq!(command.victim_health_state_revision_after, revision + 1);
    assert!(command.presentation.partial_absorb && !command.presentation.full_absorb);
    assert_eq!(command.absorb_consumptions.iter().map(|c| (c.slot, c.consumed, c.removed)).collect::<Vec<_>>(), vec![(1,4,true),(2,3,true)]);
    let player = manager.find_map(0, 0).unwrap().map().get_typed_player(swing.victim_guid).unwrap();
    assert_eq!(player.unit().get_power(wow_constants::PowerType::Mana), 14);
    let auras = player.unit().subsystems().auras.runtime_applications_like_cpp();
    assert_eq!((auras[&1].represented_effect_amounts[0].amount,
        auras[&2].represented_effect_amounts[0].amount), (0,0));
    let calls = catalogs.calls.borrow();
    let school = calls.iter().position(|c| *c == "school").unwrap();
    assert_eq!(calls[school+1], "mana");
    assert!(school < calls.iter().position(|c| *c == "share").unwrap());
}

#[test]
fn lethal_player_commit_captures_health_death_health_revision_sequence() {
    let (mut manager, mut attacker, swing) = setup(true, 1.0);
    let player = manager.find_map_mut(0, 0).unwrap().map_mut().get_typed_player_mut(swing.victim_guid).unwrap();
    player.unit_mut().set_health(5);
    let revision = player.unit().health_state_revision_like_cpp();
    let outcome = manager.apply_legacy_creature_melee_swing(Some(&mut attacker), swing, &Catalogs::default());
    assert_eq!(outcome.commands[0].damage, 10);
    assert_eq!(outcome.commands[0].over_damage, 5);
    assert_eq!(outcome.commands[0].victim_health_after, 0);
    let player = manager.find_map(0, 0).unwrap().map().get_typed_player(swing.victim_guid).unwrap();
    assert_eq!(player.unit().death_state(), wow_constants::DeathState::JustDied);
    // Health and death each advance once; the explicit second SetHealth(0)
    // observes an already-zero value and adds no revision.
    assert_eq!(player.unit().health_state_revision_like_cpp(), revision + 2);
    assert_eq!(outcome.commands[0].victim_health_state_revision_after, revision + 2);
}

#[test]
fn primary_attack_state_precedes_values_and_receipt_observes_reciprocal_threat() {
    let (mut manager, mut attacker, swing) = setup(false, 1.0);
    let catalogs = Catalogs::default();
    let outcome = manager.apply_legacy_creature_melee_swing(Some(&mut attacker), swing, &catalogs);
    assert_eq!(outcome.canonical_creature_hits, 1);
    assert_eq!(outcome.events.len(), 2);
    assert!(matches!(&outcome.events[0], MeleeEffect::AttackState { damage: 10, .. }));
    assert!(matches!(&outcome.events[1], MeleeEffect::Values { guid, .. } if *guid == swing.victim_guid));
    assert_eq!(outcome.syncs.len(), 1);
    let receipt = &outcome.syncs[0].state;
    assert_eq!((receipt.victim_health_before, receipt.victim_health_after), (100,90));
    assert_eq!(receipt.threat.as_ref().unwrap().delta, 10.0);
    let map = manager.find_map(0, 0).unwrap().map();
    assert!(map.with_creature_like_cpp(swing.victim_guid, |c| c.unit().subsystems().combat.has_combat()).unwrap());
    assert!(map.with_creature_like_cpp(swing.attacker_guid, |c| c.unit().subsystems().combat.has_combat()).unwrap());
    assert_eq!(&*catalogs.calls.borrow(), &["threat", "damage_clock"]);
}

#[test]
fn self_share_can_kill_canonical_attacker_before_primary_second_admission() {
    let (mut manager, mut attacker, swing) = setup(false, 1.0);
    let applied = AppliedAuraRef::new(42, swing.attacker_guid, 3, 1);
    manager.find_map_mut(0, 0).unwrap().map_mut().get_typed_creature_mut(swing.victim_guid).unwrap()
        .unit_mut().subsystems_mut().auras.applied_auras.push(applied);
    let mut catalogs = Catalogs::inert();
    catalogs.shares.push(ShareAuraSnapshotLikeCpp {
        identity: ShareAuraIdentityLikeCpp::Creature { applied, effect_index: 0 },
        caster_guid: swing.attacker_guid, school_mask: 1, amount: 1_000,
    });
    let outcome = manager.apply_legacy_creature_melee_swing(Some(&mut attacker), swing, &catalogs);
    assert_eq!(outcome.melee_precondition_rejections, 1);
    assert_eq!(outcome.canonical_hits, 0);
    assert_eq!(health(&manager, swing.victim_guid), 100);
    assert_eq!(health(&manager, swing.attacker_guid), 0);
    // The late rejection drops publication, while the already committed secondary receipt remains.
    assert!(outcome.commands.is_empty() && outcome.events.is_empty());
    assert_eq!(outcome.syncs.len(), 1);
    assert_eq!(outcome.syncs[0].swing.victim_guid, swing.attacker_guid);
    assert_eq!(outcome.syncs[0].state.victim_health_after, 0);
    assert_eq!(outcome.attacking_interrupt_auras_removed, 0);
    assert_eq!(attacker.creature.ai_ownership().swing_timer_ms, 0);
}
