use crate::session::{
    AuraApplication, RepresentedAuraEffectLikeCpp, RepresentedGameObjectUseState,
    RepresentedHomebindLikeCpp, RepresentedTaxiFlightNodeLikeCpp, SessionPlayerController,
    WorldSession,
};
use crate::test_fixtures::{
    game_time_ms_for_test, player_interaction_source_guid_for_test,
    player_interaction_trainer_id_for_test, player_is_alive_for_test,
    push_player_gossip_option_for_test, set_player_faction_template_for_test,
    set_player_interaction_source_for_test, set_player_trainer_interaction_for_test,
    CollectionLoadPortLikeCpp,
};
use std::sync::{Arc, Mutex};
use wow_constants::unit::NPCFlags1;
use wow_constants::ServerOpcodes;
use wow_core::guid::HighGuid;
use wow_core::{EquipmentSetGuidGeneratorLikeCpp, ObjectGuid, ObjectGuidGenerator, Position};
use wow_data::{BankBagSlotPricesEntry, BankBagSlotPricesStore};
use wow_entities::{Creature, GAMEOBJECT_TYPE_GOOBER, GameObject, MapObjectRecord, Player};
use wow_packet::WorldPacket;
use wow_packet::packets::spell::SpellCastVisual;

#[tokio::test]
async fn gossip_select_accepts_represented_goober_menu_and_removes_feign_like_cpp() {
    const FEIGN_SLOT: u8 = 19;
    const GOSSIP_ID: u32 = 72;
    let (mut session, send_rx, canonical) = make_bank_slot_session(2);
    insert_bank_test_player_in_world(&session, &canonical);
    let goober = gameobject_guid(9304, 304);
    insert_gossip_gameobject(
        &canonical,
        goober,
        9304,
        Position::new(1.0, 0.0, 0.0, 0.0),
        GAMEOBJECT_TYPE_GOOBER as u8,
        true,
    );
    session.represented_gameobject_use_states.insert(
        goober,
        RepresentedGameObjectUseState {
            map_id: Some(571),
            position: Some(Position::new(1.0, 0.0, 0.0, 0.0)),
            go_type: Some(GAMEOBJECT_TYPE_GOOBER as u8),
            icon_name_allows_interaction_like_cpp: Some(true),
            ..Default::default()
        },
    );
    set_player_interaction_source_for_test(&mut session, goober);
    push_player_gossip_option_for_test(
        &mut session,
        crate::session::GossipOptionInfo {
            gossip_option_id: 71,
            menu_id: GOSSIP_ID,
            order_index: 0,
            option_npc: 0,
            action_menu_id: 0,
        },
    );
    seed_represented_feign_death_like_cpp(&mut session, FEIGN_SLOT);

    session
        .handle_gossip_select_option(wow_packet::packets::gossip::GossipSelectOption {
            gossip_unit: goober,
            gossip_id: GOSSIP_ID as i32,
            gossip_option_id: 71,
            promotion_code: String::new(),
        })
        .await;

    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::AuraUpdate],
        "a valid GOOBER follows the C++ generic GameObject interaction path"
    );
    assert!(!session.visible_auras.contains_key(&FEIGN_SLOT));
    assert!(!canonical_player_has_died_state_like_cpp(&mut session));
}

fn make_session_with_send_capacity(
    capacity: usize,
) -> (WorldSession, flume::Receiver<Vec<u8>>) {
    let (_pkt_tx, pkt_rx) = flume::bounded::<WorldPacket>(1);
    let (send_tx, send_rx) = flume::bounded::<Vec<u8>>(capacity);
    let mut session = WorldSession::new(
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
    session.set_item_guid_generator_like_cpp(Arc::new(ObjectGuidGenerator::new(HighGuid::Item, 1)));
    session.set_equipment_set_guid_generator_like_cpp(Arc::new(
        EquipmentSetGuidGeneratorLikeCpp::new(1),
    ));
    (session, send_rx)
}

fn make_bank_slot_session(
    capacity: usize,
) -> (
    WorldSession,
    flume::Receiver<Vec<u8>>,
    Arc<Mutex<wow_map::MapManager>>,
) {
    let (mut session, send_rx) = make_session_with_send_capacity(capacity);
    let canonical = Arc::new(Mutex::new(wow_map::MapManager::new(60_000, 10)));
    let player_guid = ObjectGuid::create_player(1, 42);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "Tester".to_string(),
        Position::new(0.0, 0.0, 0.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    set_player_faction_template_for_test(&mut session, 1);
    session.set_bank_bag_slot_prices_store(Arc::new(BankBagSlotPricesStore::from_entries([
        BankBagSlotPricesEntry { id: 1, cost: 100 },
        BankBagSlotPricesEntry { id: 2, cost: 200 },
    ])));
    session.set_player_gold_like_cpp(150);
    session.set_player_bank_bag_slot_count_like_cpp(0);
    (session, send_rx, canonical)
}

fn insert_bank_test_player_in_world(
    session: &WorldSession,
    canonical: &Arc<Mutex<wow_map::MapManager>>,
) {
    let player_guid = session.player_guid().expect("player guid");
    let mut player = Player::new(Some(1), false);
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
        .insert_map_object_record(MapObjectRecord::new_player(player).unwrap())
        .unwrap();
}

fn gameobject_guid(entry: u32, counter: i64) -> ObjectGuid {
    ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, entry, counter)
}

fn insert_gossip_gameobject(
    manager: &Arc<Mutex<wow_map::MapManager>>,
    guid: ObjectGuid,
    entry: u32,
    position: Position,
    go_type: u8,
    is_in_world: bool,
) {
    let mut gameobject = GameObject::new();
    gameobject.world_mut().object_mut().create(guid);
    gameobject.world_mut().object_mut().set_entry(entry);
    gameobject.world_mut().set_map(571, 0).unwrap();
    gameobject.world_mut().relocate(position);
    gameobject.set_go_type(go_type);
    if is_in_world {
        gameobject.world_mut().object_mut().add_to_world();
    }
    manager
        .lock()
        .unwrap()
        .create_world_map(571, 0)
        .map_mut()
        .insert_map_object_record(MapObjectRecord::new_game_object(gameobject).unwrap())
        .unwrap();
}

fn seed_represented_feign_death_like_cpp(session: &mut WorldSession, slot: u8) {
    let player_guid = session.player_guid().expect("player guid");
    session
        .mutate_canonical_player_like_cpp(|player| {
            player
                .unit_mut()
                .add_unit_state(wow_constants::unit::UnitState::DIED.bits());
        })
        .expect("canonical player");
    session.visible_auras.insert(
        slot,
        AuraApplication {
            spell_id: 5384,
            difficulty_id: 0,
            caster_guid: player_guid,
            slot,
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
}

fn canonical_player_has_died_state_like_cpp(session: &mut WorldSession) -> bool {
    session
        .mutate_canonical_player_like_cpp(|player| {
            player
                .unit()
                .has_unit_state(wow_constants::unit::UnitState::DIED.bits())
        })
        .expect("canonical player")
}

#[tokio::test]
async fn gossip_select_gameobject_revalidates_cpp_interaction_boundaries() {
    const FEIGN_SLOT: u8 = 20;
    const GOSSIP_ID: u32 = 82;
    // Missing represented icon/type evidence is a Rust-only defensive case:
    // C++ reads the live template and GameObject directly, so it has no
    // equivalent absent-cache branch. The remaining rows mirror C++ gates.
    for (case, go_type, recorded_type, position, in_world, icon_allows, in_taxi, same_phase) in [
        (
            "point-icon",
            GAMEOBJECT_TYPE_GOOBER as u8,
            true,
            Position::new(1.0, 0.0, 0.0, 0.0),
            true,
            Some(false),
            false,
            true,
        ),
        (
            "missing-icon-evidence",
            GAMEOBJECT_TYPE_GOOBER as u8,
            true,
            Position::new(1.0, 0.0, 0.0, 0.0),
            true,
            None,
            false,
            true,
        ),
        (
            "missing-runtime-type",
            GAMEOBJECT_TYPE_GOOBER as u8,
            false,
            Position::new(1.0, 0.0, 0.0, 0.0),
            true,
            Some(true),
            false,
            true,
        ),
        (
            "gameobject-not-in-world",
            GAMEOBJECT_TYPE_GOOBER as u8,
            true,
            Position::new(1.0, 0.0, 0.0, 0.0),
            false,
            Some(true),
            false,
            true,
        ),
        (
            "outside-interaction-distance",
            GAMEOBJECT_TYPE_GOOBER as u8,
            true,
            Position::new(50.0, 0.0, 0.0, 0.0),
            true,
            Some(true),
            false,
            true,
        ),
        (
            "player-in-taxi-flight",
            GAMEOBJECT_TYPE_GOOBER as u8,
            true,
            Position::new(1.0, 0.0, 0.0, 0.0),
            true,
            Some(true),
            true,
            true,
        ),
        (
            "incompatible-phase",
            GAMEOBJECT_TYPE_GOOBER as u8,
            true,
            Position::new(1.0, 0.0, 0.0, 0.0),
            true,
            Some(true),
            false,
            false,
        ),
    ] {
        let (mut session, send_rx, canonical) = make_bank_slot_session(2);
        insert_bank_test_player_in_world(&session, &canonical);
        let gameobject = gameobject_guid(9305, 305);
        insert_gossip_gameobject(&canonical, gameobject, 9305, position, go_type, in_world);
        session.represented_gameobject_use_states.insert(
            gameobject,
            RepresentedGameObjectUseState {
                map_id: Some(571),
                position: Some(position),
                go_type: recorded_type.then_some(go_type),
                icon_name_allows_interaction_like_cpp: icon_allows,
                ..Default::default()
            },
        );
        set_player_interaction_source_for_test(&mut session, gameobject);
        push_player_gossip_option_for_test(
            &mut session,
            crate::session::GossipOptionInfo {
                gossip_option_id: 81,
                menu_id: GOSSIP_ID,
                order_index: 0,
                option_npc: 0,
                action_menu_id: 0,
            },
        );
        seed_represented_feign_death_like_cpp(&mut session, FEIGN_SLOT);
        if in_taxi {
            session.set_taxi_flight_state_like_cpp(
                RepresentedTaxiFlightNodeLikeCpp {
                    map_id: 571,
                    position: Position::new(1.0, 0.0, 0.0, 0.0),
                    teleport_flag: false,
                },
                None,
            );
        }
        if !same_phase {
            session.set_represented_player_phase_shift_like_cpp(
                wow_entities::PhaseShift::from_phases([10]),
            );
            session.record_represented_gameobject_phase_shift_like_cpp(
                gameobject,
                wow_entities::PhaseShift::from_phases([20]),
            );
        }

        session
            .handle_gossip_select_option(wow_packet::packets::gossip::GossipSelectOption {
                gossip_unit: gameobject,
                gossip_id: GOSSIP_ID as i32,
                gossip_option_id: 81,
                promotion_code: String::new(),
            })
            .await;

        assert!(
            send_rx.try_recv().is_err(),
            "{case}: rejection must precede fake-death removal and action routing"
        );
        assert!(
            session.visible_auras.contains_key(&FEIGN_SLOT),
            "{case}: rejected source must preserve fake death"
        );
        assert!(
            canonical_player_has_died_state_like_cpp(&mut session),
            "{case}: rejected source must preserve DIED state"
        );
        assert_eq!(
            player_interaction_source_guid_for_test(&session),
            Some(gameobject)
        );
    }
}

#[tokio::test]
async fn gossip_select_gameobject_rejects_npc_service_option_after_feign_like_cpp() {
    const FEIGN_SLOT: u8 = 23;
    const GOSSIP_ID: u32 = 92;
    let (mut session, send_rx, canonical) = make_bank_slot_session(2);
    insert_bank_test_player_in_world(&session, &canonical);
    let goober = gameobject_guid(9306, 306);
    insert_gossip_gameobject(
        &canonical,
        goober,
        9306,
        Position::new(1.0, 0.0, 0.0, 0.0),
        GAMEOBJECT_TYPE_GOOBER as u8,
        true,
    );
    session.represented_gameobject_use_states.insert(
        goober,
        RepresentedGameObjectUseState {
            map_id: Some(571),
            position: Some(Position::new(1.0, 0.0, 0.0, 0.0)),
            go_type: Some(GAMEOBJECT_TYPE_GOOBER as u8),
            icon_name_allows_interaction_like_cpp: Some(true),
            ..Default::default()
        },
    );
    set_player_interaction_source_for_test(&mut session, goober);
    push_player_gossip_option_for_test(
        &mut session,
        crate::session::GossipOptionInfo {
            gossip_option_id: 91,
            menu_id: GOSSIP_ID,
            order_index: 0,
            option_npc: 6, // Banker is invalid for every C++ GameObject menu.
            action_menu_id: 0,
        },
    );
    seed_represented_feign_death_like_cpp(&mut session, FEIGN_SLOT);

    session
        .handle_gossip_select_option(wow_packet::packets::gossip::GossipSelectOption {
            gossip_unit: goober,
            gossip_id: GOSSIP_ID as i32,
            gossip_option_id: 91,
            promotion_code: String::new(),
        })
        .await;

    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::AuraUpdate],
        "C++ revalidates the GO and removes fake death before rejecting its NPC service option"
    );
    assert_eq!(
        player_interaction_source_guid_for_test(&session),
        Some(goober)
    );
    assert!(!session.visible_auras.contains_key(&FEIGN_SLOT));
    assert!(!canonical_player_has_died_state_like_cpp(&mut session));
}

fn drain_server_opcodes(send_rx: &flume::Receiver<Vec<u8>>) -> Vec<ServerOpcodes> {
    let mut opcodes = Vec::new();
    while let Ok(bytes) = send_rx.try_recv() {
        let packet = WorldPacket::from_bytes(&bytes);
        if let Some(opcode) = packet.server_opcode() {
            opcodes.push(opcode);
        }
    }
    opcodes
}

fn insert_binder_creature(
    manager: &Arc<Mutex<wow_map::MapManager>>,
    guid: ObjectGuid,
    npc_flags: u32,
) {
    let mut creature = Creature::new(false);
    creature.unit_mut().world_mut().object_mut().create(guid);
    creature.unit_mut().world_mut().object_mut().set_entry(2456);
    creature.unit_mut().world_mut().set_map(571, 0).unwrap();
    creature
        .unit_mut()
        .world_mut()
        .relocate(Position::new(5.0, 0.0, 0.0, 0.0));
    creature.unit_mut().world_mut().set_combat_reach(1.0);
    creature.unit_mut().set_level(80);
    creature.unit_mut().set_max_health(100);
    creature.unit_mut().set_health(100);
    creature.set_ai_identity_runtime(1, 35, npc_flags, 0);
    creature.unit_mut().world_mut().object_mut().add_to_world();

    manager
        .lock()
        .unwrap()
        .create_world_map(571, 0)
        .map_mut()
        .insert_map_object_record(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();
}

fn insert_binder_innkeeper(
    manager: &Arc<Mutex<wow_map::MapManager>>,
    guid: ObjectGuid,
) {
    insert_binder_creature(manager, guid, NPCFlags1::INNKEEPER.bits());
}

fn install_binder_spell_fixture(session: &mut WorldSession) {
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        3286,
        wow_data::SpellInfo {
            spell_id: 3286,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_BIND,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_BIND,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));
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
    set_player_faction_template_for_test(&mut session, 1);
    let player_guid = session.player_guid().expect("loaded player");
    let homebind_port = CollectionLoadPortLikeCpp::new([]);
    session.set_player_lifecycle_port_like_cpp(homebind_port.clone());
    let (realm_tx, realm_rx) = flume::bounded::<Vec<u8>>(16);
    session.install_realm_send_channel_for_test(realm_tx);
    let innkeeper = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 2456, 30);
    insert_binder_innkeeper(&canonical, innkeeper);
    session.set_player_zone_area_like_cpp(12, 34);
    install_binder_spell_fixture(&mut session);
    set_player_trainer_interaction_for_test(&mut session, innkeeper, 77);
    let _ = game_time_ms_for_test();
    std::thread::sleep(std::time::Duration::from_millis(2));
    let cast_time_lower_bound = game_time_ms_for_test();

    session
        .handle_binder_activate(wow_packet::packets::gossip::Hello { unit: innkeeper })
        .await;
    let cast_time_upper_bound = game_time_ms_for_test();

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
    session.visible_auras.insert(
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

    session
        .handle_binder_activate(wow_packet::packets::gossip::Hello { unit: innkeeper })
        .await;

    assert!(session.represented_homebind_like_cpp().is_none());
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::AuraUpdate],
        "C++ removes feign death before SendBindPoint rejects an instanceable map"
    );
    assert!(!session.visible_auras.contains_key(&FEIGN_DEATH_SLOT));
    assert_eq!(
        session
            .mutate_canonical_player_like_cpp(|player| player
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
    session.set_player_zone_area_like_cpp(12, 34);

    session
        .handle_binder_activate(wow_packet::packets::gossip::Hello { unit: creature })
        .await;

    assert!(session.represented_homebind_like_cpp().is_none());
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn binder_activate_rejects_player_outside_world_like_cpp() {
    let (mut session, send_rx, canonical) = make_bank_slot_session(1);
    insert_bank_test_player_in_world(&session, &canonical);
    let innkeeper = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 2456, 33);
    insert_binder_innkeeper(&canonical, innkeeper);
    session.set_player_zone_area_like_cpp(12, 34);
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
    assert!(player_is_alive_for_test(&session));

    session
        .handle_binder_activate(wow_packet::packets::gossip::Hello { unit: innkeeper })
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
    insert_binder_innkeeper(&canonical, innkeeper);
    session.set_player_zone_area_like_cpp(12, 34);
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

    session
        .handle_binder_activate(wow_packet::packets::gossip::Hello { unit: innkeeper })
        .await;

    assert!(session.represented_homebind_like_cpp().is_none());
    assert!(
        send_rx.try_recv().is_err(),
        "C++ Player::IsInWorld is false after removal even while the session still has an alive player controller"
    );
}
