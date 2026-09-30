use super::*;
use super::fixtures_world::*;

#[tokio::test]
async fn binder_activate_sets_current_homebind_and_sends_bind_packets_like_cpp() {
    let (mut session, instance_rx, canonical) = make_bank_slot_session(16);
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
    let player_guid = session.player_guid().expect("loaded player");
    let homebind_port = CollectionLoadPortLikeCpp::new([]);
    session.set_player_lifecycle_port_like_cpp(homebind_port.clone());
    let (realm_tx, realm_rx) = flume::bounded::<Vec<u8>>(16);
    install_gossip_realm_send_channel_for_test(&mut session, realm_tx);
    let innkeeper = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 2456, 30);
    insert_binder_innkeeper(&canonical, innkeeper);
    set_gossip_zone_area_for_test(&mut session, 12, 34);
    install_binder_spell_fixture(&mut session);
    set_player_trainer_interaction_for_test(&mut session, innkeeper, 77);
    let _ = game_time_ms_for_test();
    std::thread::sleep(std::time::Duration::from_millis(2));
    let cast_time_lower_bound = game_time_ms_for_test();

    handle_binder_activate_for_test(&mut session, wow_packet::packets::gossip::Hello { unit: innkeeper })
        .await;
    let cast_time_upper_bound = game_time_ms_for_test();

    assert_eq!(
        gossip_homebind_for_test(&session),
        Some(RepresentedHomebindLikeCpp {
            map_id: 571,
            area_id: 34,
            position: Position::new(0.0, 0.0, 0.0, 0.0),
        })
    );
    let packets: Vec<Vec<u8>> = instance_rx.try_iter().collect();
    assert_eq!(
        packets
            .iter()
            .filter_map(|bytes| WorldPacket::from_bytes(bytes).server_opcode())
            .collect::<Vec<_>>(),
        vec![ServerOpcodes::SpellGo, ServerOpcodes::BindPointUpdate,]
    );
    assert_eq!(
        realm_rx
            .try_iter()
            .filter_map(|bytes| WorldPacket::from_bytes(&bytes).server_opcode())
            .collect::<Vec<_>>(),
        vec![ServerOpcodes::PlayerBound, ServerOpcodes::GossipComplete],
        "C++ routes PlayerBound and GossipComplete on realm"
    );
    assert!(
        player_interaction_source_guid_for_test(&session).is_none(),
        "C++ PlayerMenu::SendCloseGossip resets interaction provenance"
    );
    assert_eq!(player_interaction_trainer_id_for_test(&session), 0);
    let mut spell_go = WorldPacket::from_bytes(&packets[0]);
    assert_eq!(
        spell_go.read_uint16().expect("SpellGo opcode"),
        ServerOpcodes::SpellGo as u16
    );
    assert_eq!(
        spell_go.read_packed_guid().expect("SpellGo caster"),
        innkeeper,
        "C++ creature CastSpell keeps the innkeeper as visible caster"
    );
    assert_eq!(
        spell_go.read_packed_guid().expect("SpellGo caster unit"),
        innkeeper
    );
    let _ = spell_go.read_packed_guid().expect("SpellGo cast id");
    let _ = spell_go
        .read_packed_guid()
        .expect("SpellGo original cast id");
    assert_eq!(spell_go.read_int32().expect("SpellGo spell id"), 3286);
    let _ = SpellCastVisual::read(&mut spell_go).expect("SpellGo visual");
    assert_eq!(
        spell_go.read_uint32().expect("SpellGo cast flags"),
        0x0004_0101,
        "C++ bind SpellGo carries UNKNOWN_9 | PENDING | NO_GCD"
    );
    assert_eq!(spell_go.read_uint32().expect("SpellGo cast flags ex"), 0);
    let cast_time_ms = spell_go.read_uint32().expect("SpellGo cast time");
    assert!(
        (cast_time_lower_bound..=cast_time_upper_bound).contains(&cast_time_ms),
        "C++ SpellGo CastTime is the wrapping getMSTime() server timestamp"
    );
    for _ in 0..20 {
        if !homebind_port.homebind_requests().is_empty() {
            break;
        }
        tokio::task::yield_now().await;
    }
    assert_eq!(
        homebind_port.homebind_requests(),
        vec![wow_persistence::PlayerHomebindPersistenceRequestLikeCpp::UpdateLive {
            player_guid: player_guid.counter() as u64,
            map_id: 571,
            area_id: 34,
            x: 0.0,
            y: 0.0,
            z: 0.0,
            orientation: 0.0,
        }],
        "detached persistence failure does not suppress the immediate C++ bind packets"
    );
}

#[tokio::test]
async fn binder_activate_rejects_instanceable_map_like_cpp() {
    let (mut session, send_rx, canonical) = make_bank_slot_session(2);
    let innkeeper = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 2456, 31);
    insert_binder_innkeeper(&canonical, innkeeper);
    let player_guid = session.player_guid().expect("player guid");
    let mut player = Player::new(Some(1), false);
    player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    player.unit_mut().world_mut().set_map(571, 0).unwrap();
    player
        .unit_mut()
        .world_mut()
        .relocate(Position::new(0.0, 0.0, 0.0, 0.0));
    player.unit_mut().world_mut().object_mut().add_to_world();
    player
        .unit_mut()
        .add_unit_state(wow_constants::unit::UnitState::DIED.bits());
    canonical
        .lock()
        .unwrap()
        .create_world_map(571, 0)
        .map_mut()
        .insert_map_object_record(MapObjectRecord::new_player(player).unwrap())
        .unwrap();
    const FEIGN_DEATH_SLOT: u8 = 7;
    insert_gossip_visible_aura_for_test(
        &mut session,
        FEIGN_DEATH_SLOT,
        AuraApplication {
            spell_id: 5384,
            difficulty_id: 0,
            caster_guid: player_guid,
            slot: FEIGN_DEATH_SLOT,
            duration_total: 0,
            duration_remaining: 0,
            stack_count: 1,
            aura_flags: 0,
            effect_mask: 1,
            aura_interrupt_flags: 0,
            aura_interrupt_flags2: 0,
            represented_effect: Some(RepresentedAuraEffectLikeCpp::FeignDeath),
            represented_amount: 0,
            represented_effect_amounts: Vec::new(),
            represented_misc_value: None,
            represented_multiplier: 1.0,
            applied_at: std::time::Instant::now(),
        },
    );
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 571,
            instance_type: wow_data::map::MAP_INSTANCE,
            expansion_id: 2,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));

    handle_binder_activate_for_test(&mut session, wow_packet::packets::gossip::Hello { unit: innkeeper })
        .await;

    assert!(gossip_homebind_for_test(&session).is_none());
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::AuraUpdate],
        "C++ removes feign death before SendBindPoint rejects an instanceable map"
    );
    assert!(!has_gossip_visible_aura_for_test(&session, FEIGN_DEATH_SLOT));
    assert_eq!(
        mutate_canonical_player_for_test(&session, |player| player
                .unit()
                .has_unit_state(wow_constants::unit::UnitState::DIED.bits()))
            .expect("canonical player"),
        false
    );
}

#[tokio::test]
async fn binder_activate_rejects_non_innkeeper_like_cpp() {
    let (mut session, send_rx, canonical) = make_bank_slot_session(1);
    insert_bank_test_player_in_world(&session, &canonical);
    let creature = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 2456, 31);
    insert_binder_creature(&canonical, creature, NPCFlags1::BANKER.bits());
    set_gossip_zone_area_for_test(&mut session, 12, 34);

    handle_binder_activate_for_test(&mut session, wow_packet::packets::gossip::Hello { unit: creature })
        .await;

    assert!(gossip_homebind_for_test(&session).is_none());
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn binder_activate_rejects_player_outside_world_like_cpp() {
    let (mut session, send_rx, canonical) = make_bank_slot_session(1);
    insert_bank_test_player_in_world(&session, &canonical);
    let innkeeper = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 2456, 33);
    insert_binder_innkeeper(&canonical, innkeeper);
    set_gossip_zone_area_for_test(&mut session, 12, 34);
    assert!(
        mutate_canonical_player_for_test(&session, |player| {
                player
                    .unit_mut()
                    .world_mut()
                    .object_mut()
                    .remove_from_world();
            })
            .is_some(),
        "canonical player fixture"
    );
    assert!(player_is_alive_for_test(&session));

    handle_binder_activate_for_test(&mut session, wow_packet::packets::gossip::Hello { unit: innkeeper })
        .await;

    assert!(gossip_homebind_for_test(&session).is_none());
    assert!(
        send_rx.try_recv().is_err(),
        "C++ returns before interaction, bind mutation, and packets when Player::IsInWorld is false"
    );
}

#[tokio::test]
async fn binder_activate_rejects_player_missing_from_canonical_world_like_cpp() {
    let (mut session, send_rx, canonical) = make_bank_slot_session(1);
    insert_bank_test_player_in_world(&session, &canonical);
    let innkeeper = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 2456, 34);
    insert_binder_innkeeper(&canonical, innkeeper);
    set_gossip_zone_area_for_test(&mut session, 12, 34);
    let player_guid = session.player_guid().expect("player guid");
    assert!(
        canonical
            .lock()
            .unwrap()
            .find_map_mut(571, 0)
            .expect("canonical map")
            .map_mut()
            .remove_map_object(player_guid)
            .is_some(),
        "remove canonical player fixture"
    );
    assert!(player_is_alive_for_test(&session));

    handle_binder_activate_for_test(&mut session, wow_packet::packets::gossip::Hello { unit: innkeeper })
        .await;

    assert!(gossip_homebind_for_test(&session).is_none());
    assert!(
        send_rx.try_recv().is_err(),
        "C++ Player::IsInWorld is false after removal even while the session still has an alive player controller"
    );
}
