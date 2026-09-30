//! Lineage: World entities 20 threat/taunt, 21 Point/range, 22 leash, 23 visibility.
use super::*;

#[test]
fn candidate_order_and_reciprocal_player_links_are_committed_once() {
    let (mut manager, guid) = fixtures::manager_with_actor(810_001);
    configure(&mut manager, guid);
    let first = add_player(&mut manager, 810_002);
    let second = add_player(&mut manager, 810_003);
    let (tick, mut token) = fixtures::start(&mut manager, 333, MapObjectUpdateSelectionLikeCpp::WholeTypedStores);
    let mut config = settings(); config.family_assistance_radius = 0.0;
    let outcome = complete(policies(|policies| manager.prepare_aggro(&tick, &mut token,
        vec![candidate(first), candidate(second)], config, false, policies)).unwrap());
    assert_eq!(outcome.aggro_starts, 1);
    assert_eq!(outcome.commands[0].victim_guid, first);
    assert_eq!(actor(&manager, guid).creature.ai_ownership().combat_target, Some(first));
    let player = manager.find_map(1, 0).unwrap().map().get_typed_player(first).unwrap();
    assert!(player.unit().subsystems().combat.is_in_combat_with(guid));
    assert!(actor(&manager, guid).creature.unit().subsystems().combat.is_in_combat_with(first));
    assert!(token.actor_operation.is_none());
}

#[test]
fn aggro_does_not_advance_clock_motion_master_or_rng_prefix() {
    let (mut manager, guid) = fixtures::manager_with_actor(810_010);
    configure(&mut manager, guid);
    let victim = add_player(&mut manager, 810_011);
    let elapsed = actor(&manager, guid).runtime_elapsed_ms_like_cpp();
    let ticks = actor(&manager, guid).runtime_motion_master_ticks_like_cpp();
    let mut expected = fixtures::new_actor(810_010, actor(&manager, guid).position(), true);
    expected.seed_runtime_rng_like_cpp(0x5757);
    let (tick, mut token) = fixtures::start(&mut manager, 9_999, MapObjectUpdateSelectionLikeCpp::WholeTypedStores);
    let mut config = settings(); config.family_assistance_radius = 0.0;
    complete(policies(|policies| manager.prepare_aggro(&tick, &mut token, vec![candidate(victim)], config, false, policies)).unwrap());
    assert_eq!(actor(&manager, guid).runtime_elapsed_ms_like_cpp(), elapsed);
    assert_eq!(actor(&manager, guid).runtime_motion_master_ticks_like_cpp(), ticks);
    assert_eq!(actor_mut(&mut manager, guid).runtime_spell_schedule_rng_next_like_cpp(), expected.runtime_spell_schedule_rng_next_like_cpp());
}

#[test]
fn taunt_expiry_is_before_attack_start_and_preserves_original_aura_slots() {
    let (mut manager, guid) = fixtures::manager_with_actor(810_020);
    configure(&mut manager, guid);
    let victim = add_player(&mut manager, 810_021);
    let slot = actor_mut(&mut manager, guid).apply_taunt_aura_like_cpp(victim, 355, 7, 1).unwrap();
    actor_mut(&mut manager, guid).advance_runtime_clock_like_cpp(1);
    let (tick, mut token) = fixtures::start(&mut manager, 19, MapObjectUpdateSelectionLikeCpp::WholeTypedStores);
    let mut config = settings(); config.family_assistance_radius = 0.0;
    let outcome = complete(policies(|policies| manager.prepare_aggro(&tick, &mut token, vec![candidate(victim)], config, false, policies)).unwrap());
    assert!(matches!(&outcome.effects[0].kind, AggroEffectKind::RemoveAuras(slots) if slots == &[slot]));
    assert!(matches!(&outcome.effects.last().unwrap().kind, AggroEffectKind::AttackStart { victim: target } if *target == victim));
}

#[test]
fn missing_canonical_player_cannot_engage_or_publish_a_start() {
    let (mut manager, guid) = fixtures::manager_with_actor(810_030);
    configure(&mut manager, guid);
    let missing = ObjectGuid::create_player(1, 810_031);
    let (tick, mut token) = fixtures::start(&mut manager, 55, MapObjectUpdateSelectionLikeCpp::WholeTypedStores);
    let outcome = complete(policies(|policies| manager.prepare_aggro(&tick, &mut token, vec![candidate(missing)], settings(), false, policies)).unwrap());
    assert!(outcome.commands.is_empty());
    assert!(outcome.effects.is_empty());
    assert!(actor(&manager, guid).creature.ai_ownership().combat_target.is_none());
}

#[test]
fn all_primaries_use_exact_workset_order() {
    let (mut manager, first) = fixtures::manager_with_actor(810_040);
    let second = fixtures::insert_actor(&mut manager, 810_041, Position::xyz(10.0, 20.0, 30.0), true);
    configure(&mut manager, first); configure(&mut manager, second);
    let victim = add_player(&mut manager, 810_042);
    let (tick, mut token) = fixtures::start(&mut manager, 100, MapObjectUpdateSelectionLikeCpp::WholeTypedStores);
    let order = manager.selected_actor_guids(&tick, &mut token).unwrap();
    let mut config = settings(); config.family_assistance_radius = 0.0;
    let outcome = complete(policies(|policies| manager.prepare_aggro(&tick, &mut token, vec![candidate(victim)], config, false, policies)).unwrap());
    assert_eq!(outcome.commands.iter().map(|command| command.attacker_guid).collect::<Vec<_>>(), order);
}

#[test]
fn no_online_threat_evades_and_unlinks_the_existing_player_pair() {
    let (mut manager, guid) = fixtures::manager_with_actor(810_050);
    configure(&mut manager, guid);
    let victim = add_player(&mut manager, 810_051);
    actor_mut(&mut manager, guid).enter_combat(victim);
    manager.execute_map_command_like_cpp(1, 0, crate::MapCommandLikeCpp::CreatureAttackStart {
        attacker_guid: guid, victim_guid: victim, previous_victim_guid: None });
    let (tick, mut token) = fixtures::start(&mut manager, 1, MapObjectUpdateSelectionLikeCpp::WholeTypedStores);
    let outcome = complete(policies(|policies| manager.prepare_aggro(&tick, &mut token, Vec::new(), settings(), false, policies)).unwrap());
    assert_eq!(outcome.evades_started, 1);
    assert!(actor(&manager, guid).creature.ai_ownership().combat_target.is_none());
    assert!(!manager.find_map(1, 0).unwrap().map().get_typed_player(victim).unwrap().unit().subsystems().combat.has_combat());
}
