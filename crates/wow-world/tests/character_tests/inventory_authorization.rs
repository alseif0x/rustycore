use super::*;
use wow_constants::unit::NPCFlags1;
use wow_world::test_fixtures::{make_inventory_bank_session_for_test as make_bank_slot_session, insert_inventory_binder_creature_for_test as insert_binder_creature};

fn insert_bank_authorization_test_player(
    session: &WorldSession,
    canonical: &std::sync::Arc<std::sync::Mutex<wow_map::MapManager>>,
) {
    let player_guid = session.player_guid().expect("player guid");
    let mut player = wow_entities::Player::new(Some(1), false);
    player.unit_mut().world_mut().object_mut().create(player_guid);
    player.unit_mut().world_mut().set_map(571, 0).unwrap();
    player
        .unit_mut()
        .world_mut()
        .relocate(Position::new(0.0, 0.0, 0.0, 0.0));
    player.unit_mut().world_mut().object_mut().add_to_world();
    canonical
        .lock()
        .unwrap()
        .create_world_map(571, 0)
        .map_mut()
        .insert_map_object_record(wow_entities::MapObjectRecord::new_player(player).unwrap())
        .unwrap();
}

#[test]
fn bank_authorization_reads_the_single_interaction_source_like_cpp() {
    let (mut session, _send_rx, canonical) = make_bank_slot_session(2);
    insert_bank_authorization_test_player(&session, &canonical);
    session.set_player_alive_like_cpp(true);
    set_player_faction_template_for_test(&mut session, 1);
    let banker = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 2456, 140);
    let vendor = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 2456, 141);
    insert_binder_creature(&canonical, banker, NPCFlags1::BANKER.bits());
    insert_binder_creature(&canonical, vendor, NPCFlags1::VENDOR.bits());

    set_player_trainer_interaction_for_test(&mut session, banker, 77);
    assert!(
        inventory_can_use_bank_for_test(&session),
        "C++ CanUseBank reads SourceGuid and does not impose an interaction kind or TrainerId gate"
    );

    set_player_interaction_source_for_test(&mut session, vendor);
    assert!(!inventory_can_use_bank_for_test(&session));

    set_player_interaction_source_for_test(&mut session, banker);
    assert!(inventory_can_use_bank_for_test(&session));
    assert!(reset_player_interaction_if_source_for_test(&mut session, banker));
    assert!(!inventory_can_use_bank_for_test(&session));
}
