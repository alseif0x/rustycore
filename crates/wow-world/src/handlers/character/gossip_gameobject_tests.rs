use crate::session::{
    AuraApplication, RepresentedAuraEffectLikeCpp, RepresentedGameObjectUseState,
    SessionPlayerController, WorldSession,
};
use crate::test_fixtures::{
    push_player_gossip_option_for_test, set_player_faction_template_for_test,
    set_player_interaction_source_for_test,
};
use std::sync::{Arc, Mutex};
use wow_constants::ServerOpcodes;
use wow_core::guid::HighGuid;
use wow_core::{EquipmentSetGuidGeneratorLikeCpp, ObjectGuid, ObjectGuidGenerator, Position};
use wow_data::{BankBagSlotPricesEntry, BankBagSlotPricesStore};
use wow_entities::{GAMEOBJECT_TYPE_GOOBER, GameObject, MapObjectRecord, Player};
use wow_packet::WorldPacket;

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
