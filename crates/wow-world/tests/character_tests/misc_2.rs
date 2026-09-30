//! Misc scenarios for [`super`].
//!
//! Split out of character_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[tokio::test]
async fn gossip_hello_trainer_fallback_rejects_canonical_player_out_of_world_like_cpp() {
    let (mut session, send_rx) = make_quest_status_session();
    let player_guid = session.player_guid().unwrap();
    let entry = 15_513;
    let guid = creature_guid(entry, 514);
    let mut player = wow_entities::Player::new(Some(1), false);
    player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    player.unit_mut().world_mut().set_map(571, 0).unwrap();
    player
        .unit_mut()
        .world_mut()
        .relocate(Position::new(10.0, 0.0, 0.0, 0.0));

    let mut manager = wow_map::MapManager::default();
    manager
        .create_world_map(571, 0)
        .map_mut()
        .insert_map_object_record(wow_entities::MapObjectRecord::new_player(player).unwrap())
        .unwrap();
    attach_map_manager(&mut session, manager);
    attach_legacy_creature(&mut session, guid, entry, NPCFlags1::TRAINER.bits());

    session.handle_gossip_hello(Hello { unit: guid }).await;

    assert!(
        send_rx.try_recv().is_err(),
        "C++ HandleGossipHelloOpcode gates the trainer fallback through GetNPCIfCanInteractWith"
    );
}
