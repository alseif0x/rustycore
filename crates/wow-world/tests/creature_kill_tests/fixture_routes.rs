//! Real feature-only policy wiring; no injected event tape or selected identity.
use super::*;
use wow_constants::DeathState;

fn ordinary_session() -> WorldSession {
    let (_pkt_tx, pkt_rx) = flume::bounded(100);
    let (send_tx, _send_rx) = flume::unbounded();
    WorldSession::new(
        1, "TestAccount".into(), 0, 2, 9, 54261, vec![0u8; 40], "esES".into(),
        pkt_rx, send_tx,
    )
}

async fn kill_with_owned_player(mut session: WorldSession, counter: i64)
    -> (WorldSession, ObjectGuid, ObjectGuid)
{
    let player = install_canonical_player_owner_for_test(&mut session, 0, 0);
    session.fixture_melee_set_map_position(0, Position::new(10.0, 10.0, 0.0, 0.0));
    // These two policy cases share a real, alive generation-bearing Player.
    session.fixture_melee_mutate_player(|owner| {
        owner.unit_mut().set_max_health(100);
        owner.unit_mut().set_health(100);
    }).unwrap();
    let manager = shared_map_manager();
    let guid = test_creature_guid(counter);
    session.fixture_kill_prepare_melee(player, guid, 1, 0, 400);
    register_test_creature(&mut session, manager.clone(), guid, 40);
    session.fixture_kill_apply_damage(None, guid, 100).await.unwrap();
    let manager = manager.read().unwrap();
    let creature = manager.find_creature(0, 0, guid).unwrap();
    assert_eq!(creature.current_hp(), 0);
    assert_eq!(creature.creature.unit().death_state(), DeathState::Corpse);
    drop(manager);
    (session, player, guid)
}

#[tokio::test]
async fn ordinary_feature_session_real_kill_keeps_trace_empty_with_owned_player() {
    let session = ordinary_session();
    assert!(!session.character_lifecycle_fixture_is_enabled_for_test());
    let (session, _, _) = kill_with_owned_player(session, 18_041).await;
    assert!(session.character_player_handle_for_test().is_some());
    assert!(session.fixture_kill_observations().events.is_empty());
    assert!(session.fixture_kill_observations().pending_loot.is_empty());
}

#[tokio::test]
async fn explicit_fixture_same_owned_kill_retains_original_event_order() {
    let (session, _, _) = make_session();
    assert!(session.character_lifecycle_fixture_is_enabled_for_test());
    let (session, player, guid) = kill_with_owned_player(session, 18_042).await;
    assert!(session.character_player_handle_for_test().is_some());
    assert_eq!(session.fixture_kill_observations().events, &[
        RepresentedCreatureKillEventLikeCpp::KillerProc {
            attacker_guid: player, victim_guid: guid,
        },
        RepresentedCreatureKillEventLikeCpp::TapperTargetDiesProc {
            tapper_guid: player, victim_guid: guid,
        },
        RepresentedCreatureKillEventLikeCpp::VictimDeathProc { victim_guid: guid },
        RepresentedCreatureKillEventLikeCpp::DeliveredKillingBlowCriteria {
            player_guid: player, victim_guid: guid, quantity: 1,
        },
        RepresentedCreatureKillEventLikeCpp::DeathStateJustDied { victim_guid: guid },
        RepresentedCreatureKillEventLikeCpp::ZoneScriptUnitDeath { unit_guid: guid },
        RepresentedCreatureKillEventLikeCpp::LootFlagsApplied {
            creature_guid: guid, lootable: false, can_skin: false, skinnable: false,
        },
        RepresentedCreatureKillEventLikeCpp::CreatureOnHealthDepletedAi {
            creature_guid: guid, attacker_guid: player, is_kill: true,
        },
        RepresentedCreatureKillEventLikeCpp::CreatureJustDiedAi {
            creature_guid: guid, killer_guid: player,
        },
        RepresentedCreatureKillEventLikeCpp::ScriptMgrOnCreatureKill {
            killer_guid: player, creature_guid: guid,
        },
    ]);
}

#[test]
fn stale_some_handle_never_falls_back_or_mutates_replacement_player_for_kill_fixture() {
    let (mut session, _, _) = make_session();
    let player = install_canonical_player_owner_for_test(&mut session, 0, 0);
    let old = session.character_player_handle_for_test().unwrap();
    let canonical = canonical_map_manager_for_test(&session).unwrap().clone();
    let replacement = {
        let mut manager = canonical.lock().unwrap();
        let owner = manager.retire_player_like_cpp(old).unwrap();
        let replacement = manager.install_detached_player_like_cpp(owner).unwrap();
        manager.attach_player_like_cpp(
            replacement, wow_map::MapKey::new(0, 0), Position::default(),
        ).unwrap();
        manager.with_player_mut_like_cpp(replacement, |owner| owner.set_xp(77)).unwrap();
        replacement
    };
    let before = session.character_fixture_progression_inputs_for_test();
    session.fixture_kill_prepare_melee(player, test_creature_guid(18_043), 1, 99, 400);
    assert_eq!(session.character_player_handle_for_test(), Some(old));
    assert_eq!(session.character_fixture_progression_inputs_for_test(), before);
    let writes = std::cell::Cell::new(0);
    assert!(session.fixture_melee_mutate_player(|_| {
        writes.set(writes.get() + 1);
    }).is_none());
    assert_eq!(writes.get(), 0);
    assert!(player_quest_gameplay_snapshot_for_test(&session).is_none());
    assert!(session.fixture_kill_observations().events.is_empty());
    assert_eq!(canonical.lock().unwrap().with_player_like_cpp(
        replacement, |owner| owner.active_data().xp,
    ), Some(77));
}

#[tokio::test]
async fn pet_input_fallback_requires_explicit_handleless_fixture_mode() {
    use wow_packet::packets::pet::{
        COMMAND_FOLLOW_LIKE_CPP, COMMAND_STAY_LIKE_CPP,
        REACT_DEFENSIVE_LIKE_CPP, REACT_PASSIVE_LIKE_CPP,
    };
    let (fixture, _, _) = make_session();
    for (mut session, enabled, counter) in [
        (ordinary_session(), false, 18_044),
        (fixture, true, 18_045),
    ] {
        let player = ObjectGuid::create_player(1, 80);
        let pet = ObjectGuid::create_world_object(
            wow_core::guid::HighGuid::Pet, 0, 1, 0, 0, 500, 81,
        );
        assert_eq!(session.character_lifecycle_fixture_is_enabled_for_test(), enabled);
        assert!(session.character_player_handle_for_test().is_none());
        session.fixture_kill_bind_player(player, None);
        session.fixture_kill_set_pet_mode(
            Some(pet), REACT_PASSIVE_LIKE_CPP, COMMAND_STAY_LIKE_CPP,
        );
        {
            let inputs = session.fixture_kill_observations();
            assert_eq!(*inputs.pet_guid, if enabled { Some(pet) } else { None });
            assert_eq!(*inputs.pet_created_by_spell, 0);
            assert_eq!(*inputs.pet_react_state,
                if enabled { REACT_PASSIVE_LIKE_CPP } else { REACT_DEFENSIVE_LIKE_CPP });
            assert_eq!(*inputs.pet_command_state,
                if enabled { COMMAND_STAY_LIKE_CPP } else { COMMAND_FOLLOW_LIKE_CPP });
        }

        let manager = shared_map_manager();
        let victim = test_creature_guid(counter);
        register_test_creature(&mut session, manager.clone(), victim, 40);
        session.fixture_kill_apply_damage(None, victim, 100).await.unwrap();
        assert_eq!(manager.read().unwrap().find_creature(0, 0, victim).unwrap()
            .creature.unit().death_state(), DeathState::Corpse);
        assert_eq!(session.fixture_kill_observations().events.contains(
            &RepresentedCreatureKillEventLikeCpp::TapperPetKilledUnitAi {
                tapper_guid: player, pet_guid: pet, victim_guid: victim,
            },
        ), enabled);
        if !enabled {
            assert!(session.fixture_kill_observations().events.is_empty());
        }
    }
}

#[tokio::test]
async fn canonical_pet_guid_success_preserves_some_none_without_handleless_fallback() {
    use wow_packet::packets::pet::{
        COMMAND_FOLLOW_LIKE_CPP, COMMAND_STAY_LIKE_CPP,
        REACT_DEFENSIVE_LIKE_CPP, REACT_PASSIVE_LIKE_CPP,
    };
    let (fixture, _, _) = make_session();
    for (mut session, enabled, counter) in [
        (ordinary_session(), false, 18_046),
        (fixture, true, 18_048),
    ] {
        let seed = ObjectGuid::create_world_object(
            wow_core::guid::HighGuid::Pet, 0, 1, 0, 0, 500, 82,
        );
        let pet = ObjectGuid::create_world_object(
            wow_core::guid::HighGuid::Pet, 0, 1, 0, 0, 500, 83,
        );
        // Seed a distinct handleless input before installing a real Player.
        session.fixture_kill_set_pet_mode(
            Some(seed), REACT_PASSIVE_LIKE_CPP, COMMAND_STAY_LIKE_CPP,
        );
        let before = {
            let inputs = session.fixture_kill_observations();
            (*inputs.pet_guid, *inputs.pet_created_by_spell,
                *inputs.pet_react_state, *inputs.pet_command_state)
        };
        assert_eq!(before.0, if enabled { Some(seed) } else { None });
        let player = install_canonical_player_owner_for_test(&mut session, 0, 0);
        let handle = session.character_player_handle_for_test().unwrap();
        assert_eq!(session.fixture_melee_player_snapshot(
            |owner| owner.gameplay_state().pet_guid,
        ), Some(None));
        session.fixture_melee_set_map_position(0, Position::new(10.0, 10.0, 0.0, 0.0));
        session.fixture_melee_mutate_player(|owner| {
            owner.unit_mut().set_max_health(100);
            owner.unit_mut().set_health(100);
        }).unwrap();
        let manager = shared_map_manager();
        let first = test_creature_guid(counter);
        session.fixture_kill_prepare_melee(player, first, 1, 0, 400);
        register_test_creature(&mut session, manager.clone(), first, 40);
        session.fixture_kill_apply_damage(None, first, 100).await.unwrap();
        assert!(!session.fixture_kill_observations().events.iter().any(|event| matches!(
            event, RepresentedCreatureKillEventLikeCpp::TapperPetKilledUnitAi { .. }
        )), "Some(None) from the canonical Player must not fall back to the seeded GUID");

        // This must succeed through the Player writer even for ordinary mode=false.
        session.fixture_kill_set_pet_mode(
            Some(pet), REACT_DEFENSIVE_LIKE_CPP, COMMAND_FOLLOW_LIKE_CPP,
        );
        assert_eq!(session.character_player_handle_for_test(), Some(handle));
        assert_eq!(session.fixture_melee_player_snapshot(
            |owner| owner.gameplay_state().pet_guid,
        ), Some(Some(pet)));
        let inputs = session.fixture_kill_observations();
        assert_eq!((*inputs.pet_guid, *inputs.pet_created_by_spell,
            *inputs.pet_react_state, *inputs.pet_command_state), before,
            "Some handle must not gain a missing-Pet input fallback");
        let second = test_creature_guid(counter + 1);
        register_test_creature(&mut session, manager.clone(), second, 40);
        session.fixture_kill_apply_damage(None, second, 100).await.unwrap();
        assert_eq!(manager.read().unwrap().find_creature(0, 0, second).unwrap()
            .creature.unit().death_state(), DeathState::Corpse);
        assert_eq!(session.fixture_kill_observations().events.contains(
            &RepresentedCreatureKillEventLikeCpp::TapperPetKilledUnitAi {
                tapper_guid: player, pet_guid: pet, victim_guid: second,
            },
        ), enabled);
    }
}

#[tokio::test]
async fn stale_some_handle_keeps_seeded_pet_inputs_and_replacement_owner_unchanged() {
    use wow_packet::packets::pet::{
        COMMAND_FOLLOW_LIKE_CPP, COMMAND_STAY_LIKE_CPP,
        REACT_DEFENSIVE_LIKE_CPP, REACT_PASSIVE_LIKE_CPP,
    };
    let (mut session, _, _) = make_session();
    let seed = ObjectGuid::create_world_object(
        wow_core::guid::HighGuid::Pet, 0, 1, 0, 0, 500, 84,
    );
    let replacement_pet = ObjectGuid::create_world_object(
        wow_core::guid::HighGuid::Pet, 0, 1, 0, 0, 500, 85,
    );
    let requested_pet = ObjectGuid::create_world_object(
        wow_core::guid::HighGuid::Pet, 0, 1, 0, 0, 500, 86,
    );
    session.fixture_kill_set_pet_mode(
        Some(seed), REACT_PASSIVE_LIKE_CPP, COMMAND_STAY_LIKE_CPP,
    );
    let player = install_canonical_player_owner_for_test(&mut session, 0, 0);
    session.fixture_melee_set_map_position(0, Position::new(10.0, 10.0, 0.0, 0.0));
    let victim = test_creature_guid(18_050);
    session.fixture_kill_prepare_melee(player, victim, 1, 0, 400);
    session.fixture_melee_mutate_player(|owner| {
        owner.unit_mut().set_max_health(100);
        owner.unit_mut().set_health(100);
        owner.set_pet_guid_like_cpp(Some(replacement_pet));
    }).unwrap();
    let before = {
        let inputs = session.fixture_kill_observations();
        (*inputs.pet_guid, *inputs.pet_created_by_spell,
            *inputs.pet_react_state, *inputs.pet_command_state)
    };
    assert_eq!(before.0, Some(seed));
    let old = session.character_player_handle_for_test().unwrap();
    let canonical = canonical_map_manager_for_test(&session).unwrap().clone();
    let replacement = {
        let mut manager = canonical.lock().unwrap();
        let owner = manager.retire_player_like_cpp(old).unwrap();
        let replacement = manager.install_detached_player_like_cpp(owner).unwrap();
        manager.attach_player_like_cpp(
            replacement, wow_map::MapKey::new(0, 0), Position::new(10.0, 10.0, 0.0, 0.0),
        ).unwrap();
        replacement
    };
    session.fixture_kill_set_pet_mode(
        Some(requested_pet), REACT_DEFENSIVE_LIKE_CPP, COMMAND_FOLLOW_LIKE_CPP,
    );
    assert_eq!(session.character_player_handle_for_test(), Some(old));
    {
        let inputs = session.fixture_kill_observations();
        assert_eq!((*inputs.pet_guid, *inputs.pet_created_by_spell,
            *inputs.pet_react_state, *inputs.pet_command_state), before);
    }
    assert_eq!(canonical.lock().unwrap().with_player_like_cpp(
        replacement, |owner| owner.gameplay_state().pet_guid,
    ), Some(Some(replacement_pet)));
    assert!(session.fixture_melee_mutate_player(|_| ()).is_none());

    // Retain the legacy damage route; test only its pet-input read, not an
    // invented claim that every stale handle rejects the whole legacy kill.
    let manager = shared_map_manager();
    register_test_creature(&mut session, manager.clone(), victim, 40);
    session.fixture_kill_apply_damage(None, victim, 100).await.unwrap();
    assert_eq!(manager.read().unwrap().find_creature(0, 0, victim).unwrap()
        .creature.unit().death_state(), DeathState::Corpse);
    assert!(!session.fixture_kill_observations().events.iter().any(|event| matches!(
        event, RepresentedCreatureKillEventLikeCpp::TapperPetKilledUnitAi { .. }
    )), "a stale Some handle must read neither the seeded input nor the replacement Player");
    assert_eq!(canonical.lock().unwrap().with_player_like_cpp(
        replacement, |owner| owner.gameplay_state().pet_guid,
    ), Some(Some(replacement_pet)));
}
