use super::*;

#[test]
fn legacy_only_npc_interaction_uses_combat_reach_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 44);
    let questgiver_guid = test_creature_guid(15);
    let manager = shared_map_manager();

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
    let mut creature = crate::map_manager::WorldCreature::new(
        questgiver_guid,
        505,
        Position::new(15.75, 0.0, 0.0, 0.0),
        100,
        80,
        1,
        2,
        0.0,
        1,
        35,
        wow_constants::unit::NPCFlags1::QUEST_GIVER.bits(),
        0,
    );
    creature.creature.unit_mut().set_combat_reach(0.0);
    manager
        .write()
        .unwrap()
        .add_creature(571, 0, 0, 0, creature);
    session.set_map_manager(Arc::clone(&manager));

    assert_eq!(
        session.represented_npc_can_interact_with_like_cpp(
            questgiver_guid,
            wow_constants::unit::NPCFlags1::QUEST_GIVER.bits(),
            0,
        ),
        None,
        "C++ Player::GetNPCIfCanInteractWith uses creature combat reach + 4.0 plus both combat radii, not a fixed 8-yard radius"
    );

    manager
        .write()
        .unwrap()
        .find_creature_mut(571, 0, questgiver_guid)
        .unwrap()
        .creature
        .unit_mut()
        .set_combat_reach(1.5);

    assert_eq!(
        session.represented_npc_can_interact_with_like_cpp(
            questgiver_guid,
            wow_constants::unit::NPCFlags1::QUEST_GIVER.bits(),
            0,
        ),
        Some(RepresentedCreatureAccessLikeCpp {
            entry: 505,
            position: Position::new(15.75, 0.0, 0.0, 0.0),
            npc_flags: wow_constants::unit::NPCFlags1::QUEST_GIVER.bits(),
            npc_flags2: 0,
            faction_template_id: 35,
            trainer_class: 0,
        })
    );
}

#[test]
fn visibility_distance_adds_both_combat_reaches_with_cpp_strict_boundary() {
    let source = Position::ZERO;
    let inside = Position::new(103.49, 0.0, 500.0, 0.0);
    let boundary = Position::new(103.5, 0.0, 0.0, 0.0);

    assert!(wow_core::visibility_distance_allows_like_cpp(
        &source, 1.5, &inside, 2.0, 100.0,
    ));
    assert!(
        !wow_core::visibility_distance_allows_like_cpp(&source, 1.5, &boundary, 2.0, 100.0,),
        "C++ Position::IsInDist2d uses a strict less-than comparison"
    );
}
