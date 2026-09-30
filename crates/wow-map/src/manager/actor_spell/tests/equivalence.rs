//! A legacy counterparty fixture and an owned Actor execute one motor.
use super::*;

#[test]
fn legacy_and_canonical_preparation_consumption_match_state_queries_and_shared_rng() {
    for (counter, ai) in [(823_001, SpellAiKind::Combat), (823_003, SpellAiKind::Turret)] {
        let (mut canonical, guid, victim) = setup(counter);
        let mut legacy = crate::map_manager::MapManager::new();
        let mut incoming = fixtures::new_actor(counter, Position::xyz(10.0, 20.0, 30.0), true);
        configure(&mut incoming, victim);
        // The compatibility backend historically validates a canonical entity
        // snapshot beside its legacy actor. This clones only that entity; both
        // WorldCreature motors above were independently constructed, never cloned.
        let mut counterpart = crate::manager::MapManager::new(crate::MIN_GRID_DELAY_MS, 200);
        counterpart.create_world_map(1, 0);
        counterpart.find_map_mut(1, 0).unwrap().map_mut()
            .insert_map_object_record(MapObjectRecord::new_creature(incoming.creature.clone()).unwrap()).unwrap();
        assert_eq!(add_player(&mut counterpart, counter + 1), victim);
        assert!(legacy.add_creature(1, 0, 32, 32, incoming));
        let settings = Settings { ai, ..Settings::default() };
        let legacy_trace = Trace::default();
        let queue = policy(settings, &legacy_trace, |policies| legacy.prepare_spell_map(1, 0, 0, policies));
        let (actions, prepared) = queue.into_parts();
        assert_eq!(actions.len(), 1);
        let mut progress = policy(settings, &legacy_trace, |policies|
            counterpart.prepare_legacy_spell_action(&mut legacy, actions.into_iter().next().unwrap(), policies));
        let mut completions = Vec::new();
        let sync = loop {
            progress = match progress {
                SpellProgress::Complete(mut outcome) => {
                    outcome.completions.extend(completions);
                    break outcome;
                }
                SpellProgress::Publication { completion, continuation } => {
                    completions.push(completion);
                    policy(settings, &legacy_trace, |policies|
                        counterpart.resume_legacy_spell_publication(&mut legacy, continuation, policies))
                        .ok().unwrap()
                }
                SpellProgress::Pending(_) => panic!("legacy NoopTerrain"),
            };
        };
        let (tick, mut token) = fixtures::start(&mut canonical, 77, MapObjectUpdateSelectionLikeCpp::WholeTypedStores);
        let staged_trace = Trace::default();
        let progress = policy(settings, &staged_trace, |policies| canonical.prepare_spell(&tick, &mut token, true, policies)).unwrap();
        let ActorSpellProgress::Pending(request) = progress else { panic!("LOS") };
        let staged = complete!(&mut canonical, &tick, &mut token, settings, &staged_trace, policy(settings, &staged_trace, |policies| canonical.resume_spell_los(
            &tick, &mut token, request.into_continuation(), true, policies)).unwrap());
        assert_eq!(prepared.schedules_initialized, staged.schedules_initialized);
        assert_eq!(sync.spell_hits, staged.spell_hits); assert_eq!(sync.spell_misses, staged.spell_misses);
        assert_eq!(sync.canonical_cast_preconditions_passed, staged.canonical_cast_preconditions_passed);
        assert_eq!(*legacy_trace.borrow(), *staged_trace.borrow());
        let sync_actor = legacy.find_creature_mut(1, 0, guid).unwrap();
        let staged_actor = actor_mut(&mut canonical, guid);
        assert_eq!(sync_actor.can_swing(), staged_actor.can_swing());
        assert_eq!(sync_actor.runtime_elapsed_ms_like_cpp(), staged_actor.runtime_elapsed_ms_like_cpp());
        assert_eq!(sync_actor.runtime_motion_master_ticks_like_cpp(), staged_actor.runtime_motion_master_ticks_like_cpp());
        assert_eq!(sync_actor.runtime_rng_authority_complete_like_cpp(), staged_actor.runtime_rng_authority_complete_like_cpp());
        assert_eq!(sync_actor.random_creature_spell_hit_roll_like_cpp(), staged_actor.random_creature_spell_hit_roll_like_cpp());
    }
}

#[test]
fn later_primary_partial_completion_survives_owned_query_and_disposition() {
    let (mut manager, guid, victim) = setup(823_010);
    let other = fixtures::insert_actor(&mut manager, 823_012, Position::xyz(12.0, 20.0, 30.0), true);
    configure(actor_mut(&mut manager, other), victim);
    // Both actors are valid sources; put the victim to their right so both
    // satisfy the original behind-player gate when their query completes.
    manager.find_map_mut(1, 0).unwrap().map_mut().get_typed_player_mut(victim).unwrap()
        .unit_mut().world_mut().relocate(Position::xyz(15.0, 20.0, 30.0));
    let (tick, mut token) = fixtures::start(&mut manager, 77, MapObjectUpdateSelectionLikeCpp::WholeTypedStores);
    let ActorSpellProgress::Pending(first) = policy(Settings::default(), &Trace::default(),
        |policies| manager.prepare_spell(&tick, &mut token, true, policies)).unwrap() else { panic!("first query") };
    let ActorSpellProgress::Publication { completion, continuation } =
        policy(Settings::default(), &Trace::default(), |policies|
            manager.resume_spell_los(&tick, &mut token, first.into_continuation(), true, policies))
            .unwrap() else { panic!("first publication") };
    let ActorSpellProgress::Pending(second) = policy(Settings::default(), &Trace::default(), |policies|
        manager.resume_spell_publication(&tick, &mut token, continuation, policies))
        .unwrap() else { panic!("second query") };
    assert_eq!(second.partial().canonical_cast_preconditions_passed, 1);
    assert!(second.partial().completions.is_empty());
    let partial = manager.discard_spell_request(&tick, &mut token, second).ok().unwrap();
    assert_eq!(partial.canonical_cast_preconditions_passed, 1); assert!(partial.completions.is_empty());
    assert!(token.actor_operation.is_none());
    assert!(completion.caster_guid == guid || completion.caster_guid == other);
}
