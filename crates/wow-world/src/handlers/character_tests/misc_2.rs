//! Misc scenarios for [`super`].
//!
//! Split out of character_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[tokio::test]
async fn area_spirit_healer_query_sends_time_for_valid_healer_like_cpp() {
    let (mut session, send_rx, canonical) = make_area_spirit_healer_session(4);
    let healer = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 91, 1);
    insert_area_spirit_healer_creature(
        &canonical,
        healer,
        Position::new(10.0, 0.0, 0.0, 0.0),
        NPCFlags1::AREA_SPIRIT_HEALER.bits(),
        0,
    );
    let mut request = WorldPacket::new_empty();
    request.write_packed_guid(&healer);

    session.handle_area_spirit_healer_query(request).await;

    let bytes = send_rx.try_recv().expect("area spirit healer time");
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        wow_constants::ServerOpcodes::AreaSpiritHealerTime as u16
    );
    let mut body = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(body.read_packed_guid().unwrap(), healer);
    assert_eq!(body.read_int32().unwrap(), 0);
}
#[tokio::test]
async fn area_spirit_healer_query_rejects_out_of_range_healer_like_cpp() {
    let (mut session, send_rx, canonical) = make_area_spirit_healer_session(1);
    let healer = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 91, 2);
    insert_area_spirit_healer_creature(
        &canonical,
        healer,
        Position::new(20.1, 0.0, 0.0, 0.0),
        NPCFlags1::AREA_SPIRIT_HEALER.bits(),
        0,
    );
    let mut request = WorldPacket::new_empty();
    request.write_packed_guid(&healer);

    session.handle_area_spirit_healer_query(request).await;

    assert!(send_rx.try_recv().is_err());
}
#[tokio::test]
async fn area_spirit_healer_queue_records_valid_healer_like_cpp() {
    let (mut session, send_rx, canonical) = make_area_spirit_healer_session(1);
    let healer = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 91, 3);
    insert_area_spirit_healer_creature(
        &canonical,
        healer,
        Position::new(10.0, 0.0, 0.0, 0.0),
        NPCFlags1::AREA_SPIRIT_HEALER.bits(),
        0,
    );
    let mut request = WorldPacket::new_empty();
    request.write_packed_guid(&healer);

    session.handle_area_spirit_healer_queue(request).await;

    assert_eq!(session.area_spirit_healer_guid_like_cpp(), Some(healer));
    assert!(send_rx.try_recv().is_err());
}
#[tokio::test]
async fn hearth_and_resurrect_rejects_area_without_cpp_flag() {
    let (mut session, send_rx) = make_hearth_and_resurrect_session(0);

    session
        .handle_hearth_and_resurrect(WorldPacket::new_empty())
        .await;

    assert!(!session.player_is_alive_like_cpp());
    assert!(send_rx.try_recv().is_err());
}
#[tokio::test]
async fn hearth_and_resurrect_rejects_player_in_flight_like_cpp() {
    let (mut session, send_rx) = make_hearth_and_resurrect_session(
        wow_data::AREA_FLAG_ALLOW_HEARTH_AND_RESURRECT_FROM_AREA_LIKE_CPP,
    );
    session.set_taxi_flight_state_like_cpp(
        RepresentedTaxiFlightNodeLikeCpp {
            map_id: 571,
            position: Position::new(1.0, 2.0, 3.0, 0.0),
            teleport_flag: false,
        },
        None,
    );

    session
        .handle_hearth_and_resurrect(WorldPacket::new_empty())
        .await;

    assert!(!session.player_is_alive_like_cpp());
    assert!(send_rx.try_recv().is_err());
}
#[test]
fn autostore_full_merge_reports_destination_stack_total_like_cpp() {
    let (mut session, _send_rx, _canonical) = make_bank_slot_session(1);
    install_bank_move_item_fixture(&mut session, 709, 10);
    insert_bank_move_test_item(
        &mut session,
        wow_entities::BANK_SLOT_ITEM_START,
        709,
        7_091,
        2,
    );
    insert_bank_move_test_item(&mut session, INVENTORY_SLOT_ITEM_START, 709, 7_092, 8);

    let plan = session
        .plan_inventory_storage_move_like_cpp(
            INVENTORY_SLOT_BAG_0,
            wow_entities::BANK_SLOT_ITEM_START,
            NULL_BAG,
            NULL_SLOT,
            InventoryStorageTargetLikeCpp::Inventory,
        )
        .expect("source item")
        .expect("valid inventory plan");

    assert!(plan.moved_destination.is_none());
    assert_eq!(plan.existing_updates[0].new_count, 10);
    assert_eq!(bank_store_item_added_quest_count_like_cpp(&plan), 10);
}
#[tokio::test]
async fn binder_activate_sets_current_homebind_and_sends_bind_packets_like_cpp() {
    let (mut session, instance_rx, canonical) = make_bank_slot_session(16);
    insert_bank_test_player_in_world(&session, &canonical);
    // Login adopts the canonical Player handle and the character arrives alive
    // with a faction. The cast identity allocator fails closed without the
    // handle, HandleBinderActivateOpcode returns early for a caster that is not
    // alive, and the interaction reaction check fails closed without a faction
    // template, so this fixture installs all three like production does.
    assert!(session.adopt_registered_canonical_player_fixture_like_cpp());
    assert!(
        crate::canonical_player_access::configure_canonical_player_vitals_for_test(
            &canonical,
            session.player_guid().expect("loaded player"),
            (100, 100, wow_constants::PowerType::Mana, 100, 100, 100),
        )
    );
    session.set_player_faction_template_like_cpp(1);
    // Login adopts the canonical Player handle and the character arrives alive.
    // The cast identity allocator fails closed without the handle, and
    // HandleBinderActivateOpcode returns early for a caster that is not alive,
    // so this fixture installs both like production does.
    let player_guid = session.player_guid().expect("loaded player");
    let homebind_port = HomebindPortFixtureLikeCpp::new([PersistenceOutcomeLikeCpp::Failed {
        reason: "detached write failure".to_owned(),
    }]);
    session.set_player_lifecycle_port_like_cpp(homebind_port.clone());
    let (realm_tx, realm_rx) = flume::bounded::<Vec<u8>>(16);
    session.install_realm_send_channel_for_test(realm_tx);
    let innkeeper = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 2456, 30);
    insert_banker_creature(&canonical, innkeeper, NPCFlags1::INNKEEPER.bits());
    session.set_player_zone_area_like_cpp(12, 34);
    install_bind_spell_fixture(&mut session);
    session.set_player_trainer_interaction_like_cpp(innkeeper, 77);
    let _ = WorldSession::game_time_ms_like_cpp();
    std::thread::sleep(std::time::Duration::from_millis(2));
    let cast_time_lower_bound = WorldSession::game_time_ms_like_cpp();

    session
        .handle_binder_activate(Hello { unit: innkeeper })
        .await;
    let cast_time_upper_bound = WorldSession::game_time_ms_like_cpp();

    assert_eq!(
        session.represented_homebind_like_cpp(),
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
        session.player_interaction_source_guid_like_cpp().is_none(),
        "C++ PlayerMenu::SendCloseGossip resets interaction provenance"
    );
    assert_eq!(session.player_interaction_trainer_id_like_cpp(), 0);
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
        if !homebind_port.requests().is_empty() {
            break;
        }
        tokio::task::yield_now().await;
    }
    assert_eq!(
        homebind_port.requests(),
        vec![PlayerHomebindPersistenceRequestLikeCpp::UpdateLive {
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
async fn binder_activate_rejects_non_innkeeper_like_cpp() {
    let (mut session, send_rx, canonical) = make_bank_slot_session(1);
    insert_bank_test_player_in_world(&session, &canonical);
    let creature = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 2456, 31);
    insert_banker_creature(&canonical, creature, NPCFlags1::BANKER.bits());
    session.set_player_zone_area_like_cpp(12, 34);

    session
        .handle_binder_activate(Hello { unit: creature })
        .await;

    assert!(session.represented_homebind_like_cpp().is_none());
    assert!(send_rx.try_recv().is_err());
}
#[tokio::test]
async fn binder_activate_rejects_player_outside_world_like_cpp() {
    let (mut session, send_rx, canonical) = make_bank_slot_session(1);
    insert_bank_test_player_in_world(&session, &canonical);
    let innkeeper = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 2456, 33);
    insert_banker_creature(&canonical, innkeeper, NPCFlags1::INNKEEPER.bits());
    session.set_player_zone_area_like_cpp(12, 34);
    install_bind_spell_fixture(&mut session);
    assert!(
        session
            .mutate_canonical_player_like_cpp(|player| {
                player
                    .unit_mut()
                    .world_mut()
                    .object_mut()
                    .remove_from_world();
            })
            .is_some(),
        "canonical player fixture"
    );
    assert!(session.player_is_alive_like_cpp());

    session
        .handle_binder_activate(Hello { unit: innkeeper })
        .await;

    assert!(session.represented_homebind_like_cpp().is_none());
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
    insert_banker_creature(&canonical, innkeeper, NPCFlags1::INNKEEPER.bits());
    session.set_player_zone_area_like_cpp(12, 34);
    install_bind_spell_fixture(&mut session);
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
    assert!(session.player_is_alive_like_cpp());

    session
        .handle_binder_activate(Hello { unit: innkeeper })
        .await;

    assert!(session.represented_homebind_like_cpp().is_none());
    assert!(
        send_rx.try_recv().is_err(),
        "C++ Player::IsInWorld is false after removal even while the session still has an alive player controller"
    );
}
#[tokio::test]
async fn gossip_catalog_port_preserves_read_order_and_localized_projection_like_cpp() {
    let (mut session, _) = make_quest_status_session();
    session.locale = "esES".to_owned();
    let menu_id = 700;
    let npc_guid = creature_guid(9001, 701);
    let port = GossipCatalogPortFixtureLikeCpp::new(
        [GossipCatalogReadOutcomeLikeCpp::Found(menu_id)],
        [GossipCatalogReadOutcomeLikeCpp::Found(vec![10, 20])],
        [GossipCatalogReadOutcomeLikeCpp::Found(900)],
        [GossipCatalogReadOutcomeLikeCpp::Found(vec![
            gossip_catalog_option_like_cpp(menu_id, 2, 901),
        ])],
        [GossipCatalogReadOutcomeLikeCpp::Found(
            "Opción localizada".to_owned(),
        )],
    );
    session.set_gossip_catalog_persistence_port_like_cpp(port.clone());

    let message = session
        .build_gossip_menu(9001, 0, npc_guid)
        .await
        .expect("typed catalog produces gossip message");

    assert_eq!(message.gossip_id, menu_id as i32);
    assert_eq!(message.broadcast_text_id, Some(900));
    assert_eq!(message.gossip_options.len(), 1);
    assert_eq!(message.gossip_options[0].text, "Opción localizada");
    assert_eq!(message.gossip_options[0].gossip_option_id, 77);
    assert_eq!(session.gossip_options.len(), 1);
    assert_eq!(
        port.requests(),
        vec![
            GossipCatalogRequestTraceLikeCpp::CreatureMenu(GossipCreatureMenuRequestLikeCpp {
                creature_entry: 9001,
            }),
            GossipCatalogRequestTraceLikeCpp::MenuTexts(GossipMenuCatalogRequestLikeCpp {
                menu_id,
            }),
            GossipCatalogRequestTraceLikeCpp::NpcText(GossipNpcTextCatalogRequestLikeCpp {
                npc_text_id: 20,
            }),
            GossipCatalogRequestTraceLikeCpp::MenuOptions(GossipMenuCatalogRequestLikeCpp {
                menu_id,
            }),
            GossipCatalogRequestTraceLikeCpp::BroadcastLocale(
                GossipBroadcastTextLocaleRequestLikeCpp {
                    broadcast_text_id: 901,
                    locale: "esES".to_owned(),
                },
            ),
        ]
    );
}
#[tokio::test]
async fn gossip_catalog_required_read_failure_stops_before_locale_like_cpp() {
    let (mut session, _) = make_quest_status_session();
    session.locale = "esES".to_owned();
    let port = GossipCatalogPortFixtureLikeCpp::new(
        [GossipCatalogReadOutcomeLikeCpp::Found(701)],
        [GossipCatalogReadOutcomeLikeCpp::Found(vec![21])],
        [GossipCatalogReadOutcomeLikeCpp::Missing],
        [GossipCatalogReadOutcomeLikeCpp::Failed {
            reason: "options unavailable".to_owned(),
        }],
        [],
    );
    session.set_gossip_catalog_persistence_port_like_cpp(port.clone());

    assert!(
        session
            .build_gossip_menu(9002, 0, creature_guid(9002, 702))
            .await
            .is_none()
    );
    assert_eq!(
        port.requests(),
        vec![
            GossipCatalogRequestTraceLikeCpp::CreatureMenu(GossipCreatureMenuRequestLikeCpp {
                creature_entry: 9002,
            }),
            GossipCatalogRequestTraceLikeCpp::MenuTexts(GossipMenuCatalogRequestLikeCpp {
                menu_id: 701,
            }),
            GossipCatalogRequestTraceLikeCpp::NpcText(GossipNpcTextCatalogRequestLikeCpp {
                npc_text_id: 21,
            }),
            GossipCatalogRequestTraceLikeCpp::MenuOptions(GossipMenuCatalogRequestLikeCpp {
                menu_id: 701,
            }),
        ]
    );
}
#[tokio::test]
async fn gossip_catalog_optional_reads_fail_to_existing_fallbacks_like_cpp() {
    let (mut session, _) = make_quest_status_session();
    session.locale = "esES".to_owned();
    let menu_id = 702;
    let port = GossipCatalogPortFixtureLikeCpp::new(
        [GossipCatalogReadOutcomeLikeCpp::Found(menu_id)],
        [GossipCatalogReadOutcomeLikeCpp::Missing],
        [GossipCatalogReadOutcomeLikeCpp::Failed {
            reason: "npc text unavailable".to_owned(),
        }],
        [GossipCatalogReadOutcomeLikeCpp::Found(vec![
            gossip_catalog_option_like_cpp(menu_id, 3, 902),
        ])],
        [GossipCatalogReadOutcomeLikeCpp::Failed {
            reason: "locale unavailable".to_owned(),
        }],
    );
    session.set_gossip_catalog_persistence_port_like_cpp(port);

    let message = session
        .build_gossip_menu(9003, 0, creature_guid(9003, 703))
        .await
        .expect("optional catalog failures retain the menu");
    assert_eq!(message.broadcast_text_id, None);
    assert_eq!(message.gossip_options[0].text, "Original option");
}
#[test]
fn start_positions_are_valid() {
    for race in [1, 2, 3, 4, 5, 6, 7, 8, 10, 11, 22] {
        let (map, x, y, z, _o) = start_position(race);
        assert!(map >= 0, "Race {race} has invalid map");
        // Positions should be non-zero (except possibly orientation)
        assert!(
            x != 0.0 || y != 0.0 || z != 0.0,
            "Race {race} has zero position"
        );
    }
}
#[test]
fn display_ids_are_valid() {
    for race in [1, 2, 3, 4, 5, 6, 7, 8, 10, 11] {
        for sex in [0u8, 1] {
            let id = default_display_id(race, sex);
            assert!(id > 0, "Race {race} sex {sex} has zero display ID");
        }
    }
}
#[tokio::test]
async fn invalid_gossip_hello_preserves_active_player_menu_state_like_cpp() {
    let (mut session, send_rx) = make_quest_status_session();
    let active_source = creature_guid(9306, 305);
    let invalid_source = creature_guid(9306, 999);
    session.set_player_trainer_interaction_like_cpp(active_source, 77);
    session
        .gossip_options
        .push(crate::session::GossipOptionInfo {
            gossip_option_id: 31,
            menu_id: 32,
            order_index: 33,
            option_npc: 6,
            action_menu_id: 0,
        });

    session
        .handle_gossip_hello(Hello {
            unit: invalid_source,
        })
        .await;

    assert!(
        send_rx.try_recv().is_err(),
        "C++ returns before publishing when GetNPCIfCanInteractWith rejects the source"
    );
    assert!(
        session.player_trainer_interaction_matches_like_cpp(active_source, 77),
        "invalid hello must not replace InteractionData"
    );
    assert_eq!(
        session.gossip_options.len(),
        1,
        "C++ clears PlayerMenu only after validating the source"
    );
}
#[tokio::test]
async fn valid_direct_service_hello_replaces_stale_trainer_provenance_like_cpp() {
    const FEIGN_SLOT: u8 = 24;
    let (mut session, send_rx, canonical) = make_bank_slot_session(4);
    insert_bank_test_player_in_world(&session, &canonical);
    let old_trainer = creature_guid(9306, 304);
    let vendor = creature_guid(2456, 305);
    insert_banker_creature(
        &canonical,
        vendor,
        NPCFlags1::GOSSIP.bits() | NPCFlags1::VENDOR.bits(),
    );
    session.set_player_trainer_interaction_like_cpp(old_trainer, 77);
    session
        .gossip_options
        .push(crate::session::GossipOptionInfo {
            gossip_option_id: 21,
            menu_id: 22,
            order_index: 23,
            option_npc: GOSSIP_OPTION_NPC_TRAINER_LIKE_CPP,
            action_menu_id: 0,
        });
    seed_represented_feign_death_like_cpp(&mut session, FEIGN_SLOT);

    session.handle_gossip_hello(Hello { unit: vendor }).await;

    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::AuraUpdate],
        "C++ removes fake death before dispatching the selected direct service"
    );
    assert!(
        !session.visible_auras.contains_key(&FEIGN_SLOT),
        "the direct-service shortcut represents a successful C++ gossip selection"
    );
    assert!(!canonical_player_has_died_state_like_cpp(&mut session));
    assert_eq!(
        session.player_interaction_source_guid_like_cpp(),
        Some(vendor),
        "Rust's direct-service shortcut must preserve C++ SendGossipMenu source ownership"
    );
    assert_eq!(
        session.player_interaction_trainer_id_like_cpp(),
        0,
        "opening another valid service invalidates an earlier trainer window"
    );
    assert!(session.gossip_options.is_empty());
}
#[tokio::test]
async fn gossip_hello_mixed_direct_service_keeps_service_like_cpp() {
    let (mut session, send_rx) = make_quest_status_session();
    let entry = 9309;
    let guid = creature_guid(entry, 309);
    let stale_source = creature_guid(entry, 999);
    let mut store = store_with_quests(&[3009]);
    store.starter_quests.entry(entry).or_default().push(3009);
    session.set_quest_store(Arc::new(store));
    session.set_player_trainer_interaction_like_cpp(stale_source, 77);
    session
        .gossip_options
        .push(crate::session::GossipOptionInfo {
            gossip_option_id: 91,
            menu_id: 92,
            order_index: 93,
            option_npc: 94,
            action_menu_id: 95,
        });
    attach_legacy_creature(
        &mut session,
        guid,
        entry,
        NPCFlags1::GOSSIP.bits() | NPCFlags1::QUEST_GIVER.bits() | NPCFlags1::BANKER.bits(),
    );

    session.handle_gossip_hello(Hello { unit: guid }).await;

    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::NpcInteractionOpenResult]
    );
    assert_eq!(
        session.player_interaction_source_guid_like_cpp(),
        Some(guid)
    );
    assert_eq!(session.player_interaction_trainer_id_like_cpp(), 0);
    assert!(
        session.gossip_options.is_empty(),
        "C++ HandleGossipHelloOpcode clears the prior menu before opening a direct service"
    );
}
#[tokio::test]
async fn gossip_hello_canonical_only_direct_fallback_uses_resolved_flags_like_cpp() {
    let (mut session, send_rx) = make_quest_status_session();
    let entry = 9311;
    let guid = creature_guid(entry, 311);
    let mut manager = wow_map::MapManager::default();
    insert_canonical_creature_with_npc_flags(
        &mut manager,
        guid,
        entry,
        NPCFlags1::GOSSIP.bits() | NPCFlags1::BANKER.bits(),
    );
    attach_map_manager(&mut session, manager);

    session.handle_gossip_hello(Hello { unit: guid }).await;

    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::NpcInteractionOpenResult],
        "C++-resolved canonical NPC flags must drive fallback interactions when the legacy mirror has no creature"
    );
    assert_eq!(
        session.player_interaction_source_guid_like_cpp(),
        Some(guid)
    );
    assert_eq!(session.player_interaction_trainer_id_like_cpp(), 0);
}
#[tokio::test]
async fn gossip_hello_trainer_fallback_uses_canonical_access_like_cpp() {
    let (mut session, send_rx) = make_quest_status_session();
    let entry = 15_513;
    let guid = creature_guid(entry, 515);
    let mut manager = wow_map::MapManager::default();
    insert_canonical_creature_with_npc_flags(
        &mut manager,
        guid,
        entry,
        NPCFlags1::TRAINER.bits() | NPCFlags1::TRAINER_CLASS.bits(),
    );
    attach_map_manager(&mut session, manager);

    session.handle_gossip_hello(Hello { unit: guid }).await;

    let bytes = send_rx.try_recv().expect("canonical trainer gossip menu");
    assert_eq!(gossip_message_counts(&bytes, guid), (1, 0));
    assert_eq!(
        session.player_interaction_source_guid_like_cpp(),
        Some(guid)
    );
    assert_eq!(session.player_interaction_trainer_id_like_cpp(), 0);
    assert_eq!(
        session.gossip_options[0].option_npc,
        GOSSIP_OPTION_NPC_TRAINER_LIKE_CPP
    );
    assert!(send_rx.try_recv().is_err());
}
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
#[tokio::test]
async fn gossip_select_trainer_only_source_opens_resolved_trainer_without_close_like_cpp() {
    const TRAINER_ID: u32 = 77;
    const FEIGN_SLOT: u8 = 21;
    let (mut session, send_rx, canonical) = make_bank_slot_session(4);
    insert_bank_test_player_in_world(&session, &canonical);
    let guid = creature_guid(2_456, 513);
    insert_banker_creature(&canonical, guid, NPCFlags1::TRAINER.bits());
    session.set_trainer_store_like_cpp(Arc::new(
        wow_data::TrainerStoreLikeCpp::from_rows_like_cpp(
            [wow_data::TrainerRowLikeCpp {
                id: TRAINER_ID,
                trainer_type: wow_data::TRAINER_TYPE_TRADESKILL_LIKE_CPP,
                greeting: "Train".to_string(),
            }],
            [],
            [],
            [wow_data::CreatureTrainerRowLikeCpp {
                creature_id: 2_456,
                trainer_id: TRAINER_ID,
                menu_id: 0,
                option_id: 0,
            }],
            |_| true,
            |_| true,
            |_| true,
            |_, _| true,
        )
        .store,
    ));

    session.handle_gossip_hello(Hello { unit: guid }).await;

    let menu_packet = send_rx.try_recv().expect("generated trainer-only menu");
    assert_eq!(
        WorldPacket::from_bytes(&menu_packet).server_opcode(),
        Some(ServerOpcodes::GossipMessage)
    );
    assert_eq!(gossip_message_counts(&menu_packet, guid), (1, 0));
    let option = session
        .gossip_options
        .first()
        .cloned()
        .expect("generated trainer option");
    assert_eq!(option.menu_id, 0);
    assert_eq!(option.order_index, 0);
    assert_eq!(
        option.gossip_option_id,
        GOSSIP_OPTION_ID_AUTO_TRAINER_LIKE_CPP
    );
    assert_eq!(option.option_npc, GOSSIP_OPTION_NPC_TRAINER_LIKE_CPP);
    assert_eq!(
        session.player_interaction_source_guid_like_cpp(),
        Some(guid)
    );
    assert_eq!(session.player_interaction_trainer_id_like_cpp(), 0);

    seed_represented_feign_death_like_cpp(&mut session, FEIGN_SLOT);

    session
        .handle_gossip_select_option(wow_packet::packets::gossip::GossipSelectOption {
            gossip_unit: guid,
            gossip_id: option.menu_id as i32,
            gossip_option_id: option.gossip_option_id,
            promotion_code: String::new(),
        })
        .await;

    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::AuraUpdate, ServerOpcodes::TrainerList],
        "the target fork's trainer-only generated option must be usable and must not pre-send GossipComplete"
    );
    assert!(session.player_trainer_interaction_matches_like_cpp(guid, TRAINER_ID as i32));
    assert!(!session.visible_auras.contains_key(&FEIGN_SLOT));
    assert!(!canonical_player_has_died_state_like_cpp(&mut session));
}
#[tokio::test]
async fn gossip_select_requires_exact_active_source_and_routes_exact_match_like_cpp() {
    let requested = creature_guid(15_513, 520);
    let other = creature_guid(15_513, 521);
    for (active_source, expected_opcodes) in [
        (None, Vec::new()),
        (Some(other), Vec::new()),
        (
            Some(requested),
            vec![ServerOpcodes::NpcInteractionOpenResult],
        ),
    ] {
        let (mut session, send_rx) = make_quest_status_session();
        attach_legacy_creature(
            &mut session,
            requested,
            15_513,
            NPCFlags1::GOSSIP.bits() | NPCFlags1::BANKER.bits(),
        );
        if let Some(active_source) = active_source {
            session.set_player_trainer_interaction_like_cpp(active_source, 77);
        }
        session
            .gossip_options
            .push(crate::session::GossipOptionInfo {
                gossip_option_id: 41,
                menu_id: 42,
                order_index: 43,
                option_npc: 6, // Banker gives an observable packet if routed.
                action_menu_id: 0,
            });

        session
            .handle_gossip_select_option(wow_packet::packets::gossip::GossipSelectOption {
                gossip_unit: requested,
                gossip_id: 42,
                gossip_option_id: 41,
                promotion_code: String::new(),
            })
            .await;

        assert_eq!(drain_server_opcodes(&send_rx), expected_opcodes);
        assert_eq!(session.gossip_options.len(), 1);
        if active_source == Some(requested) {
            assert_eq!(
                session.player_interaction_source_guid_like_cpp(),
                Some(requested)
            );
            assert_eq!(session.player_interaction_trainer_id_like_cpp(), 0);
        } else {
            assert_eq!(
                session.player_interaction_source_guid_like_cpp(),
                active_source
            );
            assert_eq!(
                session.player_interaction_trainer_id_like_cpp(),
                if active_source.is_some() { 77 } else { 0 }
            );
        }
    }
}
#[tokio::test]
async fn gossip_select_requires_exact_active_menu_id_like_cpp() {
    const FEIGN_SLOT: u8 = 22;
    let (mut session, send_rx, canonical) = make_bank_slot_session(2);
    insert_bank_test_player_in_world(&session, &canonical);
    let banker = creature_guid(15_513, 523);
    insert_banker_creature(
        &canonical,
        banker,
        NPCFlags1::GOSSIP.bits() | NPCFlags1::BANKER.bits(),
    );
    session.set_player_trainer_interaction_like_cpp(banker, 77);
    session
        .gossip_options
        .push(crate::session::GossipOptionInfo {
            gossip_option_id: 61,
            menu_id: 62,
            order_index: 63,
            option_npc: 6,
            action_menu_id: 0,
        });
    seed_represented_feign_death_like_cpp(&mut session, FEIGN_SLOT);

    session
        .handle_gossip_select_option(wow_packet::packets::gossip::GossipSelectOption {
            gossip_unit: banker,
            gossip_id: 999,
            gossip_option_id: 61,
            promotion_code: String::new(),
        })
        .await;

    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::AuraUpdate],
        "C++ removes fake death before Player::OnGossipSelect rejects a mismatched GossipID"
    );
    assert!(
        session.player_trainer_interaction_matches_like_cpp(banker, 77),
        "a mismatched packet GossipID must not route or replace InteractionData"
    );
    assert_eq!(session.gossip_options.len(), 1);
    assert!(!session.visible_auras.contains_key(&FEIGN_SLOT));
    assert!(!canonical_player_has_died_state_like_cpp(&mut session));
}
