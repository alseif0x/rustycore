use super::*;
use super::fixtures_world::*;
use wow_world::session::SessionState;
use wow_core::{EquipmentSetGuidGeneratorLikeCpp, ObjectGuidGenerator};

fn make_binder_observer(
    guid_counter: u32,
    position: Position,
    innkeeper: ObjectGuid,
    visible: bool,
    registry: &Arc<PlayerRegistry>,
    canonical: &Arc<Mutex<wow_map::MapManager>>,
) -> (WorldSession, flume::Receiver<Vec<u8>>) {
    let (_pkt_tx, pkt_rx) = flume::bounded::<WorldPacket>(1);
    let (send_tx, send_rx) = flume::bounded::<Vec<u8>>(4);
    let mut observer = WorldSession::new(
        1,
        "TestAccount".into(),
        0,
        2,
        9,
        54261,
        vec![0u8; 40],
        "esES".into(),
        pkt_rx,
        send_tx,
    );
    observer.set_item_guid_generator_like_cpp(Arc::new(ObjectGuidGenerator::new(
        HighGuid::Item,
        1,
    )));
    observer.set_equipment_set_guid_generator_like_cpp(Arc::new(
        EquipmentSetGuidGeneratorLikeCpp::new(1),
    ));
    let guid = ObjectGuid::create_player(1, i64::from(guid_counter));
    observer.set_canonical_map_manager(Arc::clone(canonical));
    attach_gossip_observer_controller_for_test(
        &mut observer,
        guid,
        format!("Observer{guid_counter}"),
        position,
        571,
        1,
        1,
        80,
        0,
    );
    observer.set_state(SessionState::LoggedIn);
    observer.set_player_registry(Arc::clone(registry));
    if canonical
        .lock()
        .unwrap()
        .find_map(571, 0)
        .and_then(|map| map.map().get_typed_player(guid))
        .is_none()
    {
        let mut player = Player::new(Some(1), false);
        player.unit_mut().world_mut().object_mut().create(guid);
        player.unit_mut().world_mut().set_map(571, 0).unwrap();
        player.unit_mut().world_mut().relocate(position);
        player.unit_mut().world_mut().object_mut().add_to_world();
        canonical
            .lock()
            .unwrap()
            .create_world_map(571, 0)
            .map_mut()
            .insert_map_object_record(MapObjectRecord::new_player(player).unwrap())
            .unwrap();
    }
    if visible {
        insert_client_visible_guid_for_test(&mut observer, innkeeper);
    }
    register_gossip_player_for_test(&observer);
    assert!(registry.fixture_update(guid, |placement| {
        placement.is_in_world = true;
        placement.position = position;
    }));
    (observer, send_rx)
}

#[tokio::test]
async fn binder_activate_fans_spell_go_to_visible_nearby_observers_like_cpp() {
    let (mut session, sender_rx, canonical) = make_bank_slot_session(16);
    insert_bank_test_player_in_world(&session, &canonical);
    // Login adopts the canonical Player handle and the character arrives alive
    // with a faction. The cast identity allocator fails closed without the
    // handle, HandleBinderActivateOpcode returns early for a caster that is not
    // alive, and the interaction reaction check fails closed without a faction
    // template, so this fixture installs all three like production does.
    assert!(adopt_gossip_canonical_player_for_test(&mut session));
    assert!(
        wow_world::canonical_player_access::configure_canonical_player_vitals_for_test(
            &canonical,
            session.player_guid().expect("loaded player"),
            (100, 100, wow_constants::PowerType::Mana, 100, 100, 100),
        )
    );
    set_player_faction_template_for_test(&mut session, 1);
    let innkeeper = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 2456, 32);
    insert_binder_innkeeper(&canonical, innkeeper);
    set_gossip_zone_area_for_test(&mut session, 12, 34);
    install_binder_spell_fixture(&mut session);

    let registry = Arc::new(PlayerRegistry::default());
    session.set_player_registry(Arc::clone(&registry));
    let (mut nearby_visible, nearby_visible_rx) = make_binder_observer(
        43,
        Position::new(10.0, 0.0, 0.0, 0.0),
        innkeeper,
        true,
        &registry,
        &canonical,
    );
    let (mut nearby_hidden, nearby_hidden_rx) = make_binder_observer(
        44,
        Position::new(12.0, 0.0, 0.0, 0.0),
        innkeeper,
        false,
        &registry,
        &canonical,
    );
    let (mut distant_visible, distant_visible_rx) = make_binder_observer(
        45,
        Position::new(5_000.0, 0.0, 0.0, 0.0),
        innkeeper,
        true,
        &registry,
        &canonical,
    );

    handle_binder_activate_for_test(&mut session, wow_packet::packets::gossip::Hello { unit: innkeeper })
        .await;
    let activating_player_spell_go = sender_rx.try_recv().expect("activator SpellGo");

    process_gossip_session_commands_for_test(&mut nearby_visible)
        .await;
    process_gossip_session_commands_for_test(&mut nearby_hidden)
        .await;
    process_gossip_session_commands_for_test(&mut distant_visible)
        .await;

    assert_eq!(
        nearby_visible_rx
            .try_recv()
            .expect("visible nearby observer SpellGo"),
        activating_player_spell_go
    );
    assert!(nearby_visible_rx.try_recv().is_err());
    assert!(
        nearby_hidden_rx.try_recv().is_err(),
        "C++ HaveAtClient gate rejects a non-visible innkeeper"
    );
    assert!(
        distant_visible_rx.try_recv().is_err(),
        "C++ MessageDistDeliverer rejects observers outside visibility range"
    );
}
