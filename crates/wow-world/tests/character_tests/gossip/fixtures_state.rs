use super::*;

pub(crate) fn make_quest_status_session() -> (WorldSession, flume::Receiver<Vec<u8>>) {
    make_quest_status_session_with_fixture(true)
}

pub(crate) fn make_quest_status_session_with_fixture(
    selected: bool,
) -> (WorldSession, flume::Receiver<Vec<u8>>) {
    let (_pkt_tx, pkt_rx) = flume::bounded::<WorldPacket>(1);
    let (send_tx, send_rx) = flume::bounded::<Vec<u8>>(8);
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
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    set_loaded_player_identity_like_cpp(&mut session, 571, 1, 1, 80, 0);
    set_player_faction_template_for_test(&mut session, 1);
    set_player_position_for_test(&mut session, Position::new(10.0, 0.0, 0.0, 0.0));
    if selected {
        enable_gossip_fixture_for_test(&mut session);
    }
    (session, send_rx)
}

pub(crate) fn insert_bank_test_player_in_world(
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

pub(crate) fn creature_guid(entry: u32, counter: i64) -> ObjectGuid {
    ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, entry, counter)
}

pub(crate) fn insert_banker_creature(
    manager: &Arc<Mutex<wow_map::MapManager>>,
    guid: ObjectGuid,
    npc_flags: u32,
) {
    let mut manager = manager.lock().unwrap();
    insert_canonical_creature_with_npc_flags(&mut manager, guid, 2456, npc_flags);
}

pub(crate) fn insert_canonical_creature_with_npc_flags(
    manager: &mut wow_map::MapManager,
    guid: ObjectGuid,
    entry: u32,
    npc_flags: u32,
) {
    let mut creature = wow_entities::Creature::new(false);
    creature.unit_mut().world_mut().object_mut().create(guid);
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .set_entry(entry);
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
        .create_world_map(571, 0)
        .map_mut()
        .insert_map_object_record(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();
}

pub(crate) fn attach_map_manager(session: &mut WorldSession, manager: wow_map::MapManager) {
    session.set_canonical_map_manager(Arc::new(Mutex::new(manager)));
}

pub(crate) fn attach_legacy_creature(
    session: &mut WorldSession,
    guid: ObjectGuid,
    entry: u32,
    npc_flags: u32,
) {
    let manager = Arc::new(std::sync::RwLock::new(wow_world::map_manager::MapManager::new()));
    manager.write().unwrap().add_creature(
        571,
        0,
        0,
        0,
        wow_world::map_manager::WorldCreature::new(
            guid,
            entry,
            Position::new(10.0, 0.0, 0.0, 0.0),
            100,
            80,
            1,
            2,
            0.0,
            1,
            35,
            npc_flags,
            0,
        ),
    );
    session.set_map_manager(manager);
}

pub(crate) fn store_with_quests(ids: &[u32]) -> wow_data::quest::QuestStore {
    wow_data::quest::QuestStore::from_quests_like_cpp(
        ids.iter()
            .copied()
            .map(quest_template),
    )
}

pub(crate) fn quest_template(id: u32) -> wow_data::quest::QuestTemplate {
    super::character_quest_template(id)
}

pub(crate) fn mark_quest_rewarded_fixture_like_cpp(session: &mut WorldSession, quest_id: u32) {
    set_player_quest_gameplay_rewarded_for_test(session, quest_id);
}

pub(crate) fn quest_giver_hello_packet(guid: ObjectGuid) -> WorldPacket {
    let mut packet = WorldPacket::new_empty();
    packet.write_packed_guid(&guid);
    packet.reset_read();
    packet
}

pub(crate) fn gossip_message_counts(bytes: &[u8], expected_guid: ObjectGuid) -> (i32, i32) {
    assert_eq!(
        WorldPacket::from_bytes(bytes).server_opcode(),
        Some(ServerOpcodes::GossipMessage)
    );
    let mut packet = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(packet.read_packed_guid().unwrap(), expected_guid);
    let _gossip_id = packet.read_int32().unwrap();
    let _friendship_faction_id = packet.read_int32().unwrap();
    let option_count = packet.read_int32().unwrap();
    let quest_count = packet.read_int32().unwrap();
    (option_count, quest_count)
}

pub(crate) fn seed_represented_feign_death_like_cpp(session: &mut WorldSession, slot: u8) {
    let player_guid = session.player_guid().expect("player guid");
    mutate_canonical_player_for_test(&*session, |player| {
            player
                .unit_mut()
                .add_unit_state(wow_constants::unit::UnitState::DIED.bits());
        })
        .expect("canonical player");
    insert_gossip_visible_aura_for_test(
        session,
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

pub(crate) fn canonical_player_has_died_state_like_cpp(session: &mut WorldSession) -> bool {
    mutate_canonical_player_for_test(&*session, |player| {
            player
                .unit()
                .has_unit_state(wow_constants::unit::UnitState::DIED.bits())
        })
        .expect("canonical player")
}

pub(crate) fn drain_server_opcodes(send_rx: &flume::Receiver<Vec<u8>>) -> Vec<ServerOpcodes> {
    let mut opcodes = Vec::new();
    while let Ok(bytes) = send_rx.try_recv() {
        let packet = WorldPacket::from_bytes(&bytes);
        if let Some(opcode) = packet.server_opcode() {
            opcodes.push(opcode);
        }
    }
    opcodes
}
