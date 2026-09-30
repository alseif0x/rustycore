use super::*;

#[test]
fn split_share_and_primary_kills_reserve_one_root_slot_in_mutation_order() {
    let (mut manager, root, victim) = setup(2510, 5, false, 100);
    let split_target = target(&mut manager, 2512, 5, false);
    let share_target = target(&mut manager, 2513, 5, false);
    let mut catalogs = Catalogs::inert();
    catalogs.effects.push(effect(
        wow_constants::spell::aura_types::SPELL_AURA_SPLIT_DAMAGE_PCT, 40, split_target));
    manager.find_map_mut(1, 0).unwrap().map_mut().get_typed_creature_mut(victim).unwrap()
        .unit_mut().subsystems_mut().auras.applied_auras.push(
            wow_entities::AppliedAuraRef::new(42, split_target, 3, 1));
    share(&mut manager, victim, share_target, 5, 50, &mut catalogs);
    let (tick, token) = start(&mut manager, 200, MapObjectUpdateSelectionLikeCpp::WholeTypedStores);
    let execution = manager.apply_selected_creature_melee_with_kills(
        &tick, token, root, &catalogs).ok().unwrap();
    let occurrences = execution.batch().occurrences();
    assert_eq!(occurrences.iter().map(|o| o.phase()).collect::<Vec<_>>(),
        vec![MeleeKillPhase::Split, MeleeKillPhase::Share, MeleeKillPhase::Primary]);
    assert_eq!(occurrences.iter().map(|o| captured(o).target_guid()).collect::<Vec<_>>(),
        vec![split_target, share_target, victim]);
    assert_eq!(execution.outcome().syncs.iter().map(|s| s.swing.victim_guid).collect::<Vec<_>>(),
        vec![split_target, share_target, victim]);
    assert_eq!(execution.outcome().canonical_creature_hits, 1);
    for target in [split_target, share_target, victim] {
        assert_eq!(health(&manager, target), 0);
    }
    let (_, token, batch) = execution.into_parts();
    assert!(batch.is_reserved());
    assert!(batch.reservation_error().is_none());
    let slot = token.actor_operation.as_ref().unwrap();
    assert_eq!(slot.guid, root);
    assert!(slot.matches_token(&token));
    assert!(slot.witness.same_actor(&batch.root_witness));
}

#[test]
fn root_killed_by_share_keeps_capture_and_secondary_sync_on_primary_rejection() {
    let (mut manager, root, victim) = setup(2530, 100, false, 10);
    let mut catalogs = Catalogs::inert();
    share(&mut manager, victim, root, 5, 1_000, &mut catalogs);
    let (tick, token) = start(&mut manager, 200, MapObjectUpdateSelectionLikeCpp::WholeTypedStores);
    let execution = manager.apply_selected_creature_melee_with_kills(
        &tick, token, root, &catalogs).ok().unwrap();
    assert_eq!(execution.outcome().melee_precondition_rejections, 1);
    assert_eq!(execution.outcome().canonical_hits, 0);
    assert!(execution.outcome().events.is_empty() && execution.outcome().commands.is_empty());
    assert_eq!(execution.outcome().syncs.len(), 1);
    assert_eq!(execution.outcome().syncs[0].swing.victim_guid, root);
    assert_eq!(health(&manager, victim), 100);
    assert_eq!(health(&manager, root), 0);
    assert_eq!(execution.batch().occurrences().len(), 1);
    assert_eq!(execution.batch().occurrences()[0].phase(), MeleeKillPhase::Share);
    assert_eq!(captured(&execution.batch().occurrences()[0]).target_guid(), root);
    assert!(execution.batch().is_reserved());
    assert_eq!(manager.find_map(1, 0).unwrap().map().creature_actor(root).unwrap()
        .creature.ai_ownership().swing_timer_ms, 0);
}

#[test]
fn repeated_share_rows_preserve_the_old_alive_guard_and_capture_only_actual_kills() {
    let (mut manager, root, victim) = setup(2540, 5, false, 100);
    let secondary = target(&mut manager, 2542, 5, false);
    let mut catalogs = Catalogs::inert();
    share(&mut manager, victim, secondary, 5, 50, &mut catalogs);
    share(&mut manager, victim, secondary, 6, 50, &mut catalogs);
    let (tick, token) = start(&mut manager, 200, MapObjectUpdateSelectionLikeCpp::WholeTypedStores);
    let execution = manager.apply_selected_creature_melee_with_kills(
        &tick, token, root, &catalogs).ok().unwrap();
    assert_eq!(execution.batch().occurrences().len(), 2);
    assert_eq!(execution.batch().occurrences()[0].phase(), MeleeKillPhase::Share);
    assert_eq!(execution.batch().occurrences()[1].phase(), MeleeKillPhase::Primary);
    assert_eq!(execution.outcome().syncs.len(), 2);
    assert_eq!(health(&manager, secondary), 0);
    assert_eq!(health(&manager, victim), 0);
}

#[test]
fn player_death_keeps_the_player_command_without_an_actor_kill_capture() {
    let (mut manager, root, _) = setup(2550, 100, false, 100);
    let victim = ObjectGuid::create_player(1, 2551);
    let mut player = wow_entities::Player::new(Some(1), false);
    player.unit_mut().world_mut().object_mut().create(victim);
    player.unit_mut().world_mut().set_map(1, 0).unwrap();
    player.unit_mut().world_mut().relocate(wow_core::Position::xyz(11.0,20.0,30.0));
    player.unit_mut().world_mut().object_mut().add_to_world();
    player.unit_mut().set_level(80);
    player.unit_mut().set_max_health(100);
    player.unit_mut().set_health(5);
    manager.find_map_mut(1, 0).unwrap().map_mut().insert_map_object_record(
        wow_entities::MapObjectRecord::new_player(player).unwrap()).unwrap();
    arm(&mut manager, root, victim, 100);
    let (tick, token) = start(&mut manager, 200, MapObjectUpdateSelectionLikeCpp::WholeTypedStores);
    let execution = manager.apply_selected_creature_melee_with_kills(
        &tick, token, root, &Catalogs::default()).ok().unwrap();
    assert_eq!(execution.outcome().canonical_hits, 1);
    assert_eq!(execution.outcome().commands[0].victim_health_after, 0);
    assert!(execution.batch().occurrences().is_empty());
    assert!(!execution.batch().is_reserved());
    let (_, token, _) = execution.into_parts();
    assert!(token.actor_operation.is_none());
}
