use super::*;

pub fn basic_quest_template_for_loot_test(id: u32) -> wow_data::quest::QuestTemplate {
    crate::quest_template_fixture::quest_template_row_for_test(id, 0, String::new())
}

/// Creates a session with a bounded outbound packet channel and the explicit
/// in-memory persistence outcome used by this feature-gated test surface.
pub fn make_session_with_send_capacity(
    capacity: usize,
) -> (WorldSession, flume::Receiver<Vec<u8>>) {
    let (_packet_tx, packet_rx) = flume::bounded::<WorldPacket>(1);
    let (send_tx, send_rx) = flume::bounded::<Vec<u8>>(capacity);
    let mut session = WorldSession::new(
        1,
        "TestAccount".to_owned(),
        0,
        2,
        9,
        54_261,
        vec![0; 40],
        "esES".to_owned(),
        packet_rx,
        send_tx,
    );
    session.set_loot_money_persistence_test_result_like_cpp(true);
    (session, send_rx)
}

pub fn make_session_with_send() -> (WorldSession, flume::Receiver<Vec<u8>>) {
    make_session_with_send_capacity(1)
}

pub fn make_session() -> WorldSession {
    make_session_with_send().0
}

pub fn test_creature_guid(counter: i64) -> ObjectGuid {
    ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, counter)
}

pub fn test_gameobject_guid(counter: i64) -> ObjectGuid {
    ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 0, 0, 1, counter)
}

pub fn test_corpse_guid(counter: i64) -> ObjectGuid {
    ObjectGuid::create_world_object(HighGuid::Corpse, 0, 1, 0, 0, 1, counter)
}

pub fn represented_loot_object_guid_for_test(owner: ObjectGuid) -> ObjectGuid {
    if owner.is_empty() {
        return ObjectGuid::EMPTY;
    }
    ObjectGuid::create_world_object(
        HighGuid::LootObject,
        0,
        owner.realm_id(),
        owner.map_id(),
        0,
        0,
        owner.counter(),
    )
}

pub fn loot_unit_packet(object: ObjectGuid) -> WorldPacket {
    let mut packet = WorldPacket::new_empty();
    packet.write_packed_guid(&object);
    packet.reset_read();
    packet
}

pub fn loot_release_packet(object: ObjectGuid) -> WorldPacket {
    loot_unit_packet(object)
}

pub fn loot_money_packet() -> WorldPacket {
    let mut packet = WorldPacket::new_empty();
    packet.write_bit(false);
    packet.flush_bits();
    packet.reset_read();
    packet
}

pub fn loot_item_packet(object: ObjectGuid, loot_list_id: u8) -> WorldPacket {
    let mut packet = WorldPacket::new_empty();
    packet.write_uint32(1);
    packet.write_packed_guid(&object);
    packet.write_uint8(loot_list_id);
    packet.write_bit(false);
    packet.flush_bits();
    packet.reset_read();
    packet
}

pub fn install_master_loot_group_for_test(
    session: &mut WorldSession,
    master_guid: ObjectGuid,
    candidate_guid: ObjectGuid,
) {
    let group_registry = Arc::new(wow_social::group::GroupRegistry::default());
    let mut group = wow_social::group::GroupInfo::new(master_guid);
    group.add_member(candidate_guid);
    group.loot_method = LOOT_METHOD_MASTER_LIKE_CPP;
    group.master_looter_guid = master_guid;
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    session.set_group_registry(
        group_registry,
        Arc::new(wow_social::group::PendingInvites::default()),
    );
    assert!(session.set_owned_player_group_like_cpp(Some((group_guid, 0))));
    assert_eq!(session.resolved_group_guid_like_cpp(), Some(group_guid));
}

pub fn install_group_loot_group_for_test(
    session: &mut WorldSession,
    leader_guid: ObjectGuid,
    candidate_guid: ObjectGuid,
) {
    let group_registry = Arc::new(wow_social::group::GroupRegistry::default());
    let mut group = wow_social::group::GroupInfo::new(leader_guid);
    group.add_member(candidate_guid);
    group.loot_method = LOOT_METHOD_GROUP_LIKE_CPP;
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    session.set_group_registry(
        group_registry,
        Arc::new(wow_social::group::PendingInvites::default()),
    );
    assert!(session.set_owned_player_group_like_cpp(Some((group_guid, 0))));
    assert_eq!(session.resolved_group_guid_like_cpp(), Some(group_guid));
}

pub fn install_limited_item_template_for_loot_test(
    session: &mut WorldSession,
    entry: u32,
    max_count: i32,
    flags2: u32,
) {
    session.set_item_store(Arc::new(ItemStore::from_records([ItemRecord {
        id: entry,
        class_id: ItemClass::Consumable as u8,
        subclass_id: 0,
        material: 0,
        inventory_type: InventoryType::NonEquip as i8,
        sheathe_type: 0,
        random_select: 0,
        random_suffix_group_id: 0,
        scaling_stat_distribution_id: 0,
        scaling_stat_value: 0,
    }])));
    session.set_item_stats_store(Arc::new(ItemStatsStore::from_sparse_templates([(
        entry,
        ItemSparseTemplateEntry {
            flags: [0, flags2, 0, 0],
            bag_family: 0,
            start_quest_id: 0,
            stackable: 20,
            max_count,
            lock_id: 0,
            required_reputation_rank: 0,
            sell_price: 0,
            buy_price: 0,
            vendor_stack_count: 1,
            price_variance: 1.0,
            price_random_value: 1.0,
            max_durability: 0,
            other_faction_item_id: 0,
            content_tuning_id: 0,
            player_level_to_item_level_curve_id: 0,
            limit_category: 0,
            instance_bound: 0,
            zone_bound: [0, 0],
            required_reputation_faction: 0,
            allowable_class: -1,
            required_expansion: 0,
            bonding: ItemBondingType::None as u8,
            container_slots: 0,
            inventory_type: InventoryType::NonEquip as i8,
        },
    )])));
}

pub fn install_basic_item_template_for_loot_test(
    session: &mut WorldSession,
    entry: u32,
    max_count: i32,
) {
    install_limited_item_template_for_loot_test(session, entry, max_count, 0);
}

pub fn test_creature_for_loot(guid: ObjectGuid, is_alive: bool) -> CreatureAI {
    let mut creature = CreatureAI::new(
        guid,
        1,
        Position::ZERO,
        100,
        1,
        1,
        2,
        0.0,
        1,
        35,
        0,
        0,
        0,
        0,
        0,
        None,
        0,
    );
    creature.is_alive = is_alive;
    creature
}

pub fn player_registration_for_loot_test(
    guid: ObjectGuid,
    send_tx: flume::Sender<Vec<u8>>,
) -> PlayerSessionRegistrationLikeCpp {
    let (command_tx, _command_rx) = flume::bounded(1);
    PlayerSessionRegistrationLikeCpp {
        identity: PlayerDirectoryIdentityLikeCpp {
            player_name: format!("Player{}", guid.counter()),
            account_id: guid.counter() as u32,
            battlenet_account_id: 0,
            recruiter_id: 0,
            race: 1,
            class: 1,
            sex: 0,
            active_expansion: 2,
        },
        placement: PlayerDirectoryPlacementLikeCpp {
            map_id: 0,
            instance_id: 0,
            position: Position::ZERO,
            is_in_world: true,
            level: 1,
            is_alive: true,
        },
        active_loot_rolls: Vec::new(),
        realm_send_tx: send_tx.clone(),
        send_tx,
        command_tx,
        session_phase_tx: crate::session::directory::detached_session_phase_rail_like_cpp(),
        durable_creature_runtime_commands_like_cpp: Default::default(),
        client_visible_guids_like_cpp: Default::default(),
        client_visible_transports_like_cpp: Default::default(),
        advanced_combat_logging_enabled_like_cpp: Default::default(),
        visibility_refresh_pending_like_cpp: Default::default(),
    }
}

pub fn drain_server_opcodes_for_loot_test(rx: &flume::Receiver<Vec<u8>>) -> Vec<u16> {
    let mut opcodes = Vec::new();
    while let Ok(bytes) = rx.try_recv() {
        let mut packet = WorldPacket::from_bytes(&bytes);
        opcodes.push(packet.read_uint16().unwrap());
    }
    opcodes
}

pub fn represented_loot_entry_for_test(
    loot_list_id: u8,
    item_id: u32,
    player_guid: ObjectGuid,
) -> LootEntry {
    LootEntry {
        loot_list_id,
        item_id,
        quantity: 1,
        random_properties_id: 0,
        random_properties_seed: 0,
        item_context: 0,
        flags: LootEntryFlags {
            follow_loot_rules: true,
            ..Default::default()
        },
        allowed_looters: vec![player_guid],
        roll_winner: ObjectGuid::EMPTY,
        ffa_looted_by: Vec::new(),
        taken: false,
    }
}
