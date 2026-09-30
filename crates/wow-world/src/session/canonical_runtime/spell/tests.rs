//! Original Spell fixtures/decoders, shared motor and real dormant APP consumer.
//! Mounted under Session tests to retain the existing fixture privacy.
use super::*;
use crate::session::canonical_runtime::spell::{catalogs, projection};
use crate::session::canonical_runtime::{run_canonical_spell, CanonicalSpellError,
    CanonicalSpellFailure, CanonicalSpellLosResolver, CanonicalSpellAbandonment};
use wow_map::{MapObjectTickContinuation, ObjectMapUpdateToken, MapObjectUpdateSelectionLikeCpp};

fn incoming(counter: i64, victim: ObjectGuid) -> crate::map_manager::WorldCreature {
    let mut actor = crate::map_manager::WorldCreature::new(test_creature_guid(counter), 9_001,
        Position::new(10.0, 10.0, 0.0, 0.0), 100, 25, 3, 5, 20.0, 100, 14, 0, 0);
    actor.creature.unit_mut().world_mut().set_map(0, 0).unwrap();
    actor.creature.unit_mut().world_mut().set_active(true);
    actor.creature.unit_mut().world_mut().object_mut().add_to_world();
    actor.creature.set_ai_identity_names_runtime_like_cpp("CombatAI", String::new());
    actor.creature.set_spell(0, 15_691);
    actor.creature.unit_mut().subsystems_mut().auras.set_spell_hit_aura_authority_inert_like_cpp(true);
    actor.creature.unit_mut().subsystems_mut().auras.set_spell_cast_log_aura_authority_inert_like_cpp(true);
    actor.enter_combat(victim);
    actor.mark_creature_spell_schedule_initialized_like_cpp();
    actor.schedule_creature_spell_slot_after_like_cpp(0, 0);
    actor.seed_runtime_rng_like_cpp(777);
    actor
}

fn insert(manager: &mut wow_map::MapManager, actor: crate::map_manager::WorldCreature) {
    let guid = actor.guid(); let position = actor.position();
    let map = manager.find_map_mut(0, 0).unwrap().map_mut();
    map.test_fixture_admit_creature_actor(actor);
    let cell = wow_map::cell_from_world(position.x, position.y);
    map.ensure_grid_loaded(&cell);
    map.get_ngrid_mut(wow_map::GridCoord::new(cell.grid_x(), cell.grid_y())).unwrap()
        .get_grid_type_mut(cell.cell_x(), cell.cell_y()).unwrap().grid_objects.creatures.insert(guid);
    assert!(map.add_to_active_like_cpp(guid).inserted_in_active_set);
}

fn setup(counter: i64) -> (SharedCanonicalMapManager, MapObjectTickContinuation, ObjectMapUpdateToken,
    ObjectGuid, ObjectGuid, LegacyCreatureAggroConfigLikeCpp) {
    let manager = shared_canonical_map_manager();
    let caster = test_creature_guid(counter); let victim = ObjectGuid::create_player(1, counter + 1);
    add_canonical_creature_spell_test_pair_like_cpp(&manager, caster, victim);
    let mut guard = manager.lock().unwrap();
    drop(guard.find_map_mut(0, 0).unwrap().map_mut().remove_map_object(caster).unwrap());
    insert(&mut guard, incoming(counter, victim));
    if guard.current_player_admission_like_cpp(victim).is_none() { guard.adopt_active_player_like_cpp(victim).unwrap(); }
    let plan = guard.begin_tick_like_cpp(1_000).into_started().unwrap();
    let mut tick = guard.begin_object_tick(plan).unwrap();
    let token = guard.prepare_next_object_map(&mut tick, MapObjectUpdateSelectionLikeCpp::WholeTypedStores).unwrap().unwrap();
    drop(guard);
    let mut config = creature_ai_spell_test_config_like_cpp(creature_ai_test_spell_info_like_cpp(15_691, 6, 0), false, 30.0);
    Arc::get_mut(config.spell_store.as_mut().unwrap()).unwrap().insert_spell_misc_attributes_like_cpp(
        15_691, represented_creature_spell_test_attributes_like_cpp(false));
    (manager, tick, token, caster, victim, config)
}

#[tokio::test]
async fn canonical_app_resolves_outside_guard_then_preserves_start_go_and_recipient_snapshot() {
    let (manager, tick, mut token, caster, victim, config) = setup(824_001);
    let probe = Arc::clone(&manager);
    let resolver: Arc<CanonicalSpellLosResolver> = Arc::new(move |query| {
        assert!(probe.try_lock().is_ok()); assert_eq!(query.map_id, 0); true
    });
    let outcome = run_canonical_spell(&manager, &tick, &mut token, &config, Some(&resolver)).await.unwrap();
    assert_eq!(outcome.canonical_cast_preconditions_passed, 1); assert_eq!(outcome.plan.events.len(), 1);
    let (start, go) = decode_atomic_creature_spell_wire_pair_like_cpp(&outcome.plan.events[0]);
    assert_eq!(start.caster, caster); assert_eq!(go.caster, caster);
    assert_eq!(start.cast_id, go.cast_id); assert_eq!(start.cast_id.high_guid(), HighGuid::Cast);
    assert_eq!(start.original_cast_id, ObjectGuid::EMPTY); assert_eq!(go.original_cast_id, ObjectGuid::EMPTY);
    assert_eq!(start.cast_flags, 2); assert_eq!(start.cast_time_ms, 0);
    assert_eq!(go.cast_flags, 0x0004_0100); assert_eq!(start.target_flags, 2);
    assert_eq!(start.target_unit, victim); assert_eq!(go.target_unit, victim);
    assert_eq!(go.hit_targets.len() + go.miss_targets.len(), 1);
    match &outcome.plan.events[0].recipients {
        RecipientRule::NearbyVisibleDurableSpellCast { source_guid, map_id, instance_id, source_position, required_3d, .. } => {
            assert_eq!((*source_guid, *map_id, *instance_id), (caster, 0, 0));
            assert_eq!(*source_position, Position::new(10.0, 10.0, 0.0, 0.0)); assert!(!required_3d);
        }
        _ => panic!("original durable spell recipient rule"),
    }
}

#[tokio::test]
async fn missing_store_returns_default_before_any_invalid_manager_token_gate() {
    let (manager, tick, mut token, _, _, _) = setup(824_010);
    let (foreign, foreign_tick, _, _, _, _) = setup(824_012);
    let outcome = run_canonical_spell(&manager, &foreign_tick, &mut token,
        &LegacyCreatureAggroConfigLikeCpp::default(), None).await.unwrap();
    assert_eq!(outcome.maps_seen, 0); assert!(outcome.plan.events.is_empty());
    assert!(manager.lock().unwrap().begin_tick_like_cpp(1).is_busy());
    drop(tick); drop(foreign);
}

#[tokio::test]
async fn rejected_actor_aba_retains_reply_and_explicit_disposition_retains_partial_plan() {
    let (manager, tick, mut token, caster, victim, config) = setup(824_020);
    let replace = Arc::clone(&manager);
    let resolver: Arc<CanonicalSpellLosResolver> = Arc::new(move |_| {
        let mut guard = replace.lock().unwrap();
        drop(guard.find_map_mut(0, 0).unwrap().map_mut().remove_map_object(caster).unwrap());
        insert(&mut guard, incoming(824_020, victim)); true
    });
    let error = run_canonical_spell(&manager, &tick, &mut token, &config, Some(&resolver)).await.err().unwrap();
    assert!(matches!(&error.failure, CanonicalSpellFailure::Resume(failure) if failure.response));
    let pending = error.pending_effects().unwrap(); assert_eq!(pending.casts_ready, 1);
    let abandoned = error.dispose_owned_request(&manager, &tick, &mut token).ok().unwrap();
    assert_eq!(abandoned.abandonment, CanonicalSpellAbandonment::ObservedLosReply);
    assert_eq!(abandoned.partial.canonical_cast_preconditions_passed, 0);
    assert!(manager.lock().unwrap().begin_tick_like_cpp(1).is_busy());
}

#[tokio::test]
async fn worker_panic_has_no_owned_continuation_and_stays_fail_stop() {
    let (manager, tick, mut token, _, _, config) = setup(824_030);
    let resolver: Arc<CanonicalSpellLosResolver> = Arc::new(|_| panic!("query lost its owned continuation"));
    let error = run_canonical_spell(&manager, &tick, &mut token, &config, Some(&resolver)).await.err().unwrap();
    assert!(matches!(&error.failure, CanonicalSpellFailure::QueryPanicked));
    assert!(error.dispose_owned_request(&manager, &tick, &mut token).is_err());
    assert!(manager.lock().unwrap().begin_tick_like_cpp(1).is_busy());
}

#[test]
fn unlaunched_request_disposition_preserves_partial_owned_wire_plan() {
    let (manager, tick, mut token, _, _, config) = setup(824_040);
    let progress = catalogs::with_policies(&config, |policies|
        manager.lock().unwrap().prepare_spell(&tick, &mut token, true, policies)).unwrap();
    let wow_map::ActorSpellProgress::Pending(request) = progress else { panic!("owned request") };
    let mut partial = LegacyCreatureSpellTickOutcomeLikeCpp::default();
    partial.plan.events.push(RuntimeEvent { source_guid: ObjectGuid::EMPTY,
        recipients: RecipientRule::NearbyVisible { source_guid: ObjectGuid::EMPTY, map_id: 0, instance_id: 0,
            range: 25.0, source_position: Position::default(), required_3d: false }, packet_bytes: vec![3, 4, 5] });
    let error = CanonicalSpellError { partial, failure: CanonicalSpellFailure::MissingTerrain(request) };
    let abandoned = error.dispose_owned_request(&manager, &tick, &mut token).ok().unwrap();
    assert_eq!(abandoned.abandonment, CanonicalSpellAbandonment::UnlaunchedRequest);
    assert_eq!(abandoned.partial.plan.events[0].packet_bytes, vec![3, 4, 5]);
}

#[test]
fn catalog_projection_and_compat_target_topology_log_keep_original_fixtures() {
    let raw = creature_ai_test_spell_info_like_cpp(15_691, 6, 0);
    let config = creature_ai_spell_test_config_like_cpp(raw.clone(), false, 30.0);
    let facts = projection::spell_info(&raw);
    let target = catalogs::target_facts(15_691, &facts, 0, &config);
    assert_eq!(target, wow_map::map_manager::SpellTarget::Victim);
    assert_eq!(creature_ai_spell_target_like_cpp(15_691, &raw, 0, &config), CreatureAiSpellTargetLikeCpp::Victim);
    let guid = ObjectGuid::create_player(1, 824_050);
    assert_eq!(wow_map::map_manager::single_unit_topology(&facts, guid, guid, false),
        creature_ai_spell_single_unit_topology_like_cpp(&raw, guid, guid, false));
    let actor = incoming(824_051, guid);
    let old = creature_spell_cast_log_data_like_cpp(&actor.creature, &raw, 0).unwrap();
    let new = projection::log(wow_map::map_manager::cast_log(&actor.creature, &facts, 0).unwrap());
    assert_eq!(old.health, new.health); assert_eq!(old.attack_power, new.attack_power);
    assert_eq!(old.spell_power, new.spell_power); assert_eq!(old.armor, new.armor);
    assert_eq!(old.power_data, new.power_data);
}

#[tokio::test]
async fn later_query_panic_retains_the_already_appended_cast_and_never_claims_settlement() {
    let victim = ObjectGuid::create_player(1, 824_061);
    let mut config = creature_ai_spell_test_config_like_cpp(creature_ai_test_spell_info_like_cpp(15_691, 6, 0), false, 30.0);
    Arc::get_mut(config.spell_store.as_mut().unwrap()).unwrap().insert_spell_misc_attributes_like_cpp(
        15_691, represented_creature_spell_test_attributes_like_cpp(false));
    let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    // Admit both primaries before freezing the token's selected plan.
    let fresh = shared_canonical_map_manager();
    add_canonical_creature_spell_test_pair_like_cpp(&fresh, test_creature_guid(824_060), victim);
    let mut guard = fresh.lock().unwrap();
    drop(guard.find_map_mut(0, 0).unwrap().map_mut().remove_map_object(test_creature_guid(824_060)).unwrap());
    insert(&mut guard, incoming(824_060, victim));
    insert(&mut guard, incoming(824_062, victim));
    if guard.current_player_admission_like_cpp(victim).is_none() {
        guard.adopt_active_player_like_cpp(victim).unwrap();
    }
    let plan = guard.begin_tick_like_cpp(1_000).into_started().unwrap();
    let mut tick = guard.begin_object_tick(plan).unwrap();
    let mut token = guard.prepare_next_object_map(&mut tick, MapObjectUpdateSelectionLikeCpp::WholeTypedStores).unwrap().unwrap();
    drop(guard);
    // Probe the manager actually executing this operation, outside its guard.
    let probe = Arc::clone(&fresh); let observed = Arc::clone(&calls);
    let resolver: Arc<CanonicalSpellLosResolver> = Arc::new(move |_| {
        assert!(probe.try_lock().is_ok());
        if observed.fetch_add(1, std::sync::atomic::Ordering::SeqCst) != 0 { panic!("second query"); }
        true
    });
    let error = run_canonical_spell(&fresh, &tick, &mut token, &config, Some(&resolver)).await.err().unwrap();
    assert!(matches!(&error.failure, CanonicalSpellFailure::QueryPanicked));
    assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 2);
    assert_eq!(error.partial.plan.events.len(), 1);
    assert_eq!(error.partial.canonical_cast_preconditions_passed, 1);
    let (start, go) = decode_atomic_creature_spell_wire_pair_like_cpp(&error.partial.plan.events[0]);
    assert_eq!(start.cast_id, go.cast_id); assert_eq!(go.target_unit, victim);
    assert!(error.dispose_owned_request(&fresh, &tick, &mut token).is_err());
    assert!(fresh.lock().unwrap().begin_tick_like_cpp(1).is_busy());
}

#[test]
fn published_cast_rejection_disposition_retains_atomic_packets_and_original_failure() {
    let (manager, tick, mut token, caster, victim, config) = setup(824_070);
    let progress = catalogs::with_policies(&config, |policies|
        manager.lock().unwrap().prepare_spell(&tick, &mut token, false, policies)).unwrap();
    let wow_map::ActorSpellProgress::Publication { completion, continuation } = progress
        else { panic!("owned publication") };
    let mut partial = LegacyCreatureSpellTickOutcomeLikeCpp::default();
    crate::session::canonical_runtime::spell::packets::replace_counters(&mut partial, continuation.partial());
    crate::session::creature_spell_publication::append_completion(&mut partial.plan, completion);
    {
        let mut guard = manager.lock().unwrap();
        drop(guard.find_map_mut(0, 0).unwrap().map_mut().remove_map_object(caster).unwrap());
        insert(&mut guard, incoming(824_070, victim));
    }
    let failure = catalogs::with_policies(&config, |policies|
        manager.lock().unwrap().resume_spell_publication(&tick, &mut token, continuation, policies)).err().unwrap();
    let error = CanonicalSpellError { partial, failure: CanonicalSpellFailure::Publication(failure) };
    let (foreign, foreign_tick, mut foreign_token, _, _, _) = setup(824_070);
    let error = error.dispose_owned_request(&foreign, &foreign_tick, &mut foreign_token).err().unwrap();
    assert!(matches!(&error.failure, CanonicalSpellFailure::Publication(_)));
    assert_eq!(error.partial.plan.events.len(), 1);
    let abandoned = error.dispose_owned_request(&manager, &tick, &mut token).ok().unwrap();
    assert_eq!(abandoned.abandonment, CanonicalSpellAbandonment::PublishedCast);
    assert_eq!(abandoned.partial.plan.events.len(), 1);
    assert_eq!(abandoned.partial.canonical_cast_preconditions_passed, 1);
    let (start, go) = decode_atomic_creature_spell_wire_pair_like_cpp(&abandoned.partial.plan.events[0]);
    assert_eq!(start.cast_id, go.cast_id);
    assert!(manager.lock().unwrap().begin_tick_like_cpp(1).is_busy());
}
