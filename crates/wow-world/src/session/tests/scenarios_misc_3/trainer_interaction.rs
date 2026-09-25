use super::*;

#[test]
fn npc_interaction_resolves_canonical_trainer_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 42);
    let trainer_guid = test_creature_guid(10);
    let vendor_guid = test_creature_guid(11);
    let pet_guid =
        ObjectGuid::create_world_object(wow_core::guid::HighGuid::Pet, 0, 1, 571, 0, 502, 12);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 571,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "Tester".to_string(),
        Position::new(10.0, 0.0, 0.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    session
        .ensure_canonical_world_map_for_current_player_like_cpp()
        .expect("canonical map");
    session.set_player_faction_template_like_cpp(1);

    add_canonical_test_creature(
        &canonical,
        trainer_guid,
        500,
        Position::new(14.0, 0.0, 0.0, 0.0),
        wow_constants::unit::NPCFlags1::TRAINER.bits(),
    );
    add_canonical_test_creature(
        &canonical,
        vendor_guid,
        501,
        Position::new(14.0, 0.0, 0.0, 0.0),
        wow_constants::unit::NPCFlags1::VENDOR.bits(),
    );
    {
        let mut canonical = canonical.lock().unwrap();
        let managed = canonical.find_map_mut(571, 0).unwrap();
        managed
            .map_mut()
            .get_typed_creature_mut(vendor_guid)
            .unwrap()
            .set_npc_flags2_runtime_like_cpp(wow_constants::unit::NPCFlags2::TRADESKILL_NPC.bits());
    }
    {
        let mut pet = wow_entities::Pet::new(player_guid, wow_entities::PetType::Summon);
        pet.creature_mut()
            .unit_mut()
            .world_mut()
            .object_mut()
            .create(pet_guid);
        pet.creature_mut()
            .unit_mut()
            .world_mut()
            .object_mut()
            .set_entry(502);
        pet.creature_mut()
            .unit_mut()
            .world_mut()
            .set_map(571, 0)
            .unwrap();
        pet.creature_mut()
            .unit_mut()
            .world_mut()
            .relocate(Position::new(14.0, 1.0, 0.0, 0.0));
        pet.creature_mut()
            .unit_mut()
            .world_mut()
            .set_combat_reach(1.0);
        pet.creature_mut().unit_mut().set_level(80);
        pet.creature_mut().unit_mut().set_max_health(100);
        pet.creature_mut().unit_mut().set_health(100);
        pet.creature_mut().set_ai_identity_runtime(
            1,
            35,
            wow_constants::unit::NPCFlags1::TRAINER.bits(),
            0,
        );
        pet.creature_mut()
            .unit_mut()
            .world_mut()
            .object_mut()
            .add_to_world();
        canonical
            .lock()
            .unwrap()
            .create_world_map(571, 0)
            .map_mut()
            .insert_map_object_record(wow_entities::MapObjectRecord::new_pet(pet).unwrap())
            .unwrap();
    }

    assert_eq!(
        session.canonical_creature_access_like_cpp(trainer_guid),
        Some(RepresentedCreatureAccessLikeCpp {
            entry: 500,
            position: Position::new(14.0, 0.0, 0.0, 0.0),
            npc_flags: wow_constants::unit::NPCFlags1::TRAINER.bits(),
            npc_flags2: 0,
            faction_template_id: 35,
            trainer_class: 0,
        })
    );
    assert_eq!(
        session.represented_npc_can_interact_with_like_cpp(
            trainer_guid,
            wow_constants::unit::NPCFlags1::TRAINER.bits(),
            0,
        ),
        Some(RepresentedCreatureAccessLikeCpp {
            entry: 500,
            position: Position::new(14.0, 0.0, 0.0, 0.0),
            npc_flags: wow_constants::unit::NPCFlags1::TRAINER.bits(),
            npc_flags2: 0,
            faction_template_id: 35,
            trainer_class: 0,
        })
    );
    assert_eq!(
        session.represented_npc_can_interact_with_like_cpp(
            pet_guid,
            wow_constants::unit::NPCFlags1::TRAINER.bits(),
            0,
        ),
        Some(RepresentedCreatureAccessLikeCpp {
            entry: 502,
            position: Position::new(14.0, 1.0, 0.0, 0.0),
            npc_flags: wow_constants::unit::NPCFlags1::TRAINER.bits(),
            npc_flags2: 0,
            faction_template_id: 35,
            trainer_class: 0,
        })
    );
    session
        .mutate_canonical_player_like_cpp(|player| {
            player
                .unit_mut()
                .world_mut()
                .object_mut()
                .remove_from_world();
        })
        .unwrap();
    assert_eq!(
        session.represented_npc_can_interact_with_like_cpp(
            trainer_guid,
            wow_constants::unit::NPCFlags1::TRAINER.bits(),
            0,
        ),
        None
    );
    session
        .mutate_canonical_player_like_cpp(|player| {
            player.unit_mut().world_mut().object_mut().add_to_world();
        })
        .unwrap();
    assert_eq!(
        session.represented_npc_can_interact_with_like_cpp(
            vendor_guid,
            wow_constants::unit::NPCFlags1::TRAINER.bits(),
            0,
        ),
        None
    );
    assert_eq!(
        session.represented_npc_can_interact_with_like_cpp(
            vendor_guid,
            0,
            wow_constants::unit::NPCFlags2::TRADESKILL_NPC.bits(),
        ),
        Some(RepresentedCreatureAccessLikeCpp {
            entry: 501,
            position: Position::new(14.0, 0.0, 0.0, 0.0),
            npc_flags: wow_constants::unit::NPCFlags1::VENDOR.bits(),
            npc_flags2: wow_constants::unit::NPCFlags2::TRADESKILL_NPC.bits(),
            faction_template_id: 35,
            trainer_class: 0,
        })
    );

    session.set_taxi_flight_state_like_cpp(
        RepresentedTaxiFlightNodeLikeCpp {
            map_id: 571,
            position: Position::new(10.0, 0.0, 0.0, 0.0),
            teleport_flag: false,
        },
        None,
    );
    assert_eq!(
        session.represented_npc_can_interact_with_like_cpp(
            trainer_guid,
            wow_constants::unit::NPCFlags1::TRAINER.bits(),
            0,
        ),
        None
    );
    assert!(session.replace_player_taxi_state_like_cpp(Default::default()));

    session.set_player_alive_like_cpp(false);
    assert_eq!(
        session.represented_npc_can_interact_with_like_cpp(
            trainer_guid,
            wow_constants::unit::NPCFlags1::TRAINER.bits(),
            0,
        ),
        None
    );
    session.set_player_alive_like_cpp(true);

    {
        let mut canonical = canonical.lock().unwrap();
        let managed = canonical.find_map_mut(571, 0).unwrap();
        managed
            .map_mut()
            .get_typed_creature_mut(trainer_guid)
            .unwrap()
            .unit_mut()
            .subsystems_mut()
            .control
            .set_charmer(player_guid, true);
    }
    assert_eq!(
        session.represented_npc_can_interact_with_like_cpp(
            trainer_guid,
            wow_constants::unit::NPCFlags1::TRAINER.bits(),
            0,
        ),
        None
    );
    {
        let mut canonical = canonical.lock().unwrap();
        let managed = canonical.find_map_mut(571, 0).unwrap();
        managed
            .map_mut()
            .get_typed_creature_mut(trainer_guid)
            .unwrap()
            .unit_mut()
            .subsystems_mut()
            .control
            .remove_charmer();
    }

    session.set_player_faction_template_like_cpp(1);
    session.set_faction_template_store(Arc::new(
        wow_data::progression_rewards::FactionTemplateStore::from_entries([
            faction_template_entry(35, 35, 0, 0, 1),
            faction_template_entry(1, 1, 0, 0, 0),
        ]),
    ));
    assert_eq!(
        session.represented_npc_can_interact_with_like_cpp(
            trainer_guid,
            wow_constants::unit::NPCFlags1::TRAINER.bits(),
            0,
        ),
        None
    );
    let legacy_manager = shared_map_manager();
    legacy_manager.write().unwrap().add_creature(
        571,
        0,
        0,
        0,
        crate::map_manager::WorldCreature::new(
            trainer_guid,
            500,
            Position::new(14.0, 0.0, 0.0, 0.0),
            100,
            80,
            1,
            2,
            0.0,
            1,
            35,
            wow_constants::unit::NPCFlags1::TRAINER.bits(),
            0,
        ),
    );
    session.set_map_manager(legacy_manager);
    assert_eq!(
        session.represented_npc_can_interact_with_like_cpp(
            trainer_guid,
            wow_constants::unit::NPCFlags1::TRAINER.bits(),
            0,
        ),
        None,
        "C++ GetNPCIfCanInteractWith rejects the canonical hostile NPC; legacy mirrors must not bypass that rejection"
    );

    {
        let mut canonical = canonical.lock().unwrap();
        let managed = canonical.find_map_mut(571, 0).unwrap();
        managed
            .map_mut()
            .get_typed_creature_mut(trainer_guid)
            .unwrap()
            .unit_mut()
            .set_unit_flags2_like_cpp(wow_constants::unit::UnitFlags2::INTERACT_WHILE_HOSTILE);
        let player = managed
            .map()
            .get_typed_player(player_guid)
            .expect("canonical Player owner remains active");
        assert!(player.unit().world().object().is_in_world());
        assert!(player.unit().is_alive());
        assert_eq!(player.unit().data().health, 100);
    }
    assert_eq!(
        session.represented_npc_can_interact_with_like_cpp(
            trainer_guid,
            wow_constants::unit::NPCFlags1::TRAINER.bits(),
            0,
        ),
        Some(RepresentedCreatureAccessLikeCpp {
            entry: 500,
            position: Position::new(14.0, 0.0, 0.0, 0.0),
            npc_flags: wow_constants::unit::NPCFlags1::TRAINER.bits(),
            npc_flags2: 0,
            faction_template_id: 35,
            trainer_class: 0,
        })
    );

    session.set_player_position_like_cpp(Position::new(40.0, 0.0, 0.0, 0.0));
    session
        .mutate_canonical_player_like_cpp(|player| {
            player
                .unit_mut()
                .world_mut()
                .relocate(Position::new(40.0, 0.0, 0.0, 0.0));
        })
        .unwrap();
    assert_eq!(
        session.represented_npc_can_interact_with_like_cpp(
            trainer_guid,
            wow_constants::unit::NPCFlags1::TRAINER.bits(),
            0,
        ),
        None
    );
}
