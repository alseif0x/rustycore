use super::*;

#[test]
fn nearby_melee_rejects_loaded_outside_selection_and_late_admission_without_rng() {
    let (mut manager, guid, victim) = setup(1811);
    let far = insert_actor(&mut manager, 1813, Position::xyz(-4000.0,-4000.0,30.0), false);
    let (tick, mut token) = start(&mut manager, 200, MapObjectUpdateSelectionLikeCpp::NearbyCells);
    let selected = manager.selected_actor_guids(&tick, &mut token).unwrap();
    assert!(selected.contains(&guid));
    assert!(!selected.contains(&far));
    let late = insert_actor(&mut manager, 1814, Position::xyz(12.0,20.0,30.0), false);
    let catalogs = Catalogs::default();
    for requested in [far, late] {
        assert_eq!(manager.apply_selected_creature_melee(&tick, &mut token, requested, &catalogs).err().unwrap(),
            ActorTickAccessError::OutsideSelection { guid: requested });
        assert!(manager.find_map(1, 0).unwrap().map().creature_actor(requested).unwrap()
            .runtime_rng_authority_complete_like_cpp());
    }
    assert!(catalogs.calls.borrow().is_empty());
    assert_untouched(&manager, guid, victim);
}

#[test]
fn admitted_guid_readmission_and_record_replacement_reject_before_any_melee_fact_query() {
    for actor_replacement in [false,true] {
        let (mut manager, guid, victim) = setup(1815);
        let (tick, mut token) = start(&mut manager, 200, MapObjectUpdateSelectionLikeCpp::WholeTypedStores);
        assert_eq!(manager.selected_actor_guids(&tick, &mut token).unwrap(), vec![guid]);
        let map = manager.find_map_mut(1, 0).unwrap().map_mut();
        let removed = map.remove_map_object(guid).unwrap();
        let incoming = new_actor(1815, Position::xyz(10.0,20.0,30.0), false);
        if actor_replacement {
            assert!(matches!(map.admit_creature_actor(incoming).unwrap(), CreatureActorAdmission::Inserted { .. }));
        } else {
            map.insert_map_object_record(MapObjectRecord::new_creature(incoming.creature).unwrap()).unwrap();
        }
        place_in_loaded_cell(map, guid, Position::xyz(10.0,20.0,30.0));
        let catalogs = Catalogs::default();
        let expected = if actor_replacement { ActorTickAccessError::WitnessMismatch { guid } }
            else { ActorTickAccessError::ActorUnavailable { guid } };
        assert_eq!(manager.apply_selected_creature_melee(&tick, &mut token, guid, &catalogs).err().unwrap(), expected);
        assert!(catalogs.calls.borrow().is_empty());
        assert_eq!(manager.find_map(1, 0).unwrap().map().get_typed_player(victim).unwrap().unit().data().health, 100);
        assert_eq!(manager.updater.pending_requests, 1);
        drop(removed);
    }
}

#[test]
fn selected_canonical_melee_consumes_the_same_box_and_rng_without_opening_a_slot_or_tail() {
    let (mut manager, guid, victim) = setup(1817);
    let (mut tick, mut token) = start(&mut manager, 200, MapObjectUpdateSelectionLikeCpp::WholeTypedStores);
    let map = manager.find_map(1, 0).unwrap().map();
    let address = map.creature_actor(guid).unwrap() as *const _ as usize;
    let witness = map.creature_actor_witness(guid).unwrap();
    let revision = map.get_typed_player(victim).unwrap().unit().health_state_revision_like_cpp();
    let catalogs = Catalogs::default();
    let outcome = manager.apply_selected_creature_melee(&tick, &mut token, guid, &catalogs).unwrap();
    assert_eq!(outcome.canonical_hits, 1);
    assert_eq!(outcome.commands[0].damage, 10);
    assert_eq!(outcome.commands[0].victim_health_after, 90);
    assert_eq!(outcome.commands[0].victim_health_state_revision_after, revision+1);
    assert_eq!(catalogs.calls.borrow()[0], "threat");
    let map = manager.find_map(1, 0).unwrap().map();
    let actor = map.creature_actor(guid).unwrap();
    assert_eq!(actor as *const _ as usize, address);
    assert!(map.creature_actor_witness(guid).unwrap().same_actor(&witness));
    assert!(!actor.runtime_rng_authority_complete_like_cpp());
    assert_eq!(actor.creature.ai_ownership().swing_timer_ms, actor.create_data.base_attack_time as u64);
    assert!(token.actor_operation.is_none());
    assert_eq!(manager.updater.pending_requests, 1);
    assert!(!manager.find_map(1, 0).unwrap().last_map_update_tail_summary_like_cpp().script_hook.invoked);
    assert_eq!(finish(&mut manager, &mut tick, token), ObjectMapFinishOutcome::Completed);
    assert!(manager.prepare_next_object_map(&mut tick, MapObjectUpdateSelectionLikeCpp::WholeTypedStores).unwrap().is_none());
    manager.finalize_object_tick(tick).unwrap();
    assert_eq!(manager.tick_coordination_like_cpp(), MapTickCoordinationStateLikeCpp::Idle);
}
