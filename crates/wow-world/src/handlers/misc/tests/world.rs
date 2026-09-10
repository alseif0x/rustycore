//! World packets.
//!
//! Separated from mod.rs under #709.

use super::*;

pub(super) fn unique_temp_data_dir(test_name: &str) -> std::path::PathBuf {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock before unix epoch")
        .as_nanos();
    let data_dir = std::env::temp_dir().join(format!("rustycore-{test_name}-{unique}"));
    std::fs::create_dir_all(data_dir.join("maps")).expect("create maps test dir");
    data_dir
}

pub(super) fn write_no_area_map_file_like_cpp(
    data_dir: &std::path::Path,
    map_id: u32,
    x: f32,
    y: f32,
    area_id: u16,
) {
    let (grid_x, grid_y) = crate::map_manager::terrain_grid_coords_for_wow_position_like_cpp(x, y);
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"MAPS");
    bytes.extend_from_slice(&10_u32.to_le_bytes());
    bytes.extend_from_slice(&0_u32.to_le_bytes());
    bytes.extend_from_slice(&44_u32.to_le_bytes());
    bytes.extend_from_slice(&8_u32.to_le_bytes());
    for _ in 0..6 {
        bytes.extend_from_slice(&0_u32.to_le_bytes());
    }
    assert_eq!(bytes.len(), 44);
    bytes.extend_from_slice(b"AREA");
    bytes.extend_from_slice(&1_u16.to_le_bytes());
    bytes.extend_from_slice(&area_id.to_le_bytes());
    std::fs::write(
        data_dir
            .join("maps")
            .join(format!("{map_id:04}_{grid_x:02}_{grid_y:02}.map")),
        bytes,
    )
    .expect("write test map");
}

pub(super) fn make_session() -> (crate::session::WorldSession, flume::Receiver<Vec<u8>>) {
    let (_pkt_tx, pkt_rx) = flume::bounded(8);
    let (send_tx, send_rx) = flume::bounded(16);
    (
        crate::session::WorldSession::new(
            1,
            "TestAccount".into(),
            0,
            2,
            9,
            54261,
            vec![0; 40],
            "enUS".into(),
            pkt_rx,
            send_tx,
        ),
        send_rx,
    )
}

pub(super) fn make_session_with_realm_send() -> (
    crate::session::WorldSession,
    flume::Receiver<Vec<u8>>,
    flume::Receiver<Vec<u8>>,
) {
    let (mut session, instance_rx) = make_session();
    let (realm_tx, realm_rx) = flume::bounded(8);
    session.install_realm_send_channel_for_test(realm_tx);
    (session, instance_rx, realm_rx)
}

pub(super) fn quest_template(id: u32) -> QuestTemplate {
    QuestTemplate {
        id,
        quest_type: 2,
        quest_level: 1,
        quest_max_scaling_level: 0,
        quest_package_id: 0,
        min_level: 1,
        quest_sort_id: 0,
        quest_info_id: 0,
        suggested_group_num: 0,
        reward_next_quest: 0,
        reward_xp_difficulty: 0,
        reward_xp_multiplier: 1.0,
        reward_money_difficulty: 0,
        reward_money_multiplier: 1.0,
        reward_bonus_money: 0,
        reward_display_spell: [0; QUEST_REWARD_DISPLAY_SPELL_COUNT],
        reward_spell: 0,
        reward_honor: 0,
        reward_title_id: 0,
        reward_skill_line_id: 0,
        reward_skill_points: 0,
        reward_mail_template_id: 0,
        reward_mail_delay_secs: 0,
        reward_mail_sender_entry: 0,
        reward_faction_ids: [0; QUEST_REWARD_REPUTATIONS_COUNT],
        reward_faction_values: [0; QUEST_REWARD_REPUTATIONS_COUNT],
        reward_faction_overrides: [0; QUEST_REWARD_REPUTATIONS_COUNT],
        reward_faction_cap_in: [0; QUEST_REWARD_REPUTATIONS_COUNT],
        reward_faction_flags: 0,
        source_item_id: 0,
        source_item_count: 0,
        source_spell_id: 0,
        limit_time_secs: 0,
        expansion: 0,
        flags: 0,
        flags_ex: 0,
        flags_ex2: 0,
        special_flags: 0,
        event_id_for_quest: 0,
        reward_items: [0; QUEST_REWARD_ITEM_COUNT],
        reward_amounts: [0; QUEST_REWARD_ITEM_COUNT],
        reward_currencies: [0; QUEST_REWARD_CURRENCY_COUNT],
        reward_currency_amounts: [0; QUEST_REWARD_CURRENCY_COUNT],
        item_drop: [0; QUEST_ITEM_DROP_COUNT],
        item_drop_quantity: [0; QUEST_ITEM_DROP_COUNT],
        log_title: format!("Quest {id}"),
        log_description: String::new(),
        quest_description: String::new(),
        area_description: String::new(),
        quest_completion_log: String::new(),
        objectives: Vec::new(),
        allowable_races: 0,
        allowable_classes: 0,
        max_level: 0,
        prev_quest_id: 0,
        next_quest_id: 0,
        exclusive_group: 0,
        breadcrumb_for_quest_id: 0,
        dependent_previous_quests: Vec::new(),
        dependent_breadcrumb_quests: Vec::new(),
        required_min_rep_faction: 0,
        required_min_rep_value: 0,
        required_max_rep_faction: 0,
        required_max_rep_value: 0,
        required_skill_id: 0,
        required_skill_points: 0,
        reward_choice_items: [(0, 0); QUEST_REWARD_CHOICES_COUNT],
        reward_choice_item_types: [0; QUEST_REWARD_CHOICES_COUNT],
    }
}

pub(super) fn install_pending_bind_instance_context_like_cpp(
    session: &mut crate::session::WorldSession,
    player_guid: ObjectGuid,
    map_id: u32,
    instance_id: u32,
    difficulty_id: u8,
    lock_id: u8,
) -> Arc<RwLock<wow_instances::InstanceLockMgr>> {
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(map_id as u16, 1, 1, 10, 0);
    session.set_map_store(Arc::new(MapStore::from_entries([MapEntry {
        id: map_id,
        instance_type: 2,
        expansion_id: 0,
        parent_map_id: -1,
        cosmetic_parent_map_id: -1,
        flags1: 0,
        flags2: 0,
    }])));
    session.set_difficulty_store(Arc::new(DifficultyStore::from_entries([difficulty_entry(
        u32::from(difficulty_id),
        2,
        DifficultyFlags::empty(),
    )])));
    session.set_map_difficulty_store(Arc::new(MapDifficultyStore::from_entries([
        MapDifficultyEntry {
            id: 1,
            message: String::new(),
            map_id,
            difficulty_id,
            lock_id,
            reset_interval: 2,
            max_players: 0,
            flags: 0,
        },
    ])));

    let canonical = Arc::new(Mutex::new(wow_map::MapManager::default()));
    canonical.lock().unwrap().create_map_entry(
        map_id,
        instance_id,
        difficulty_id,
        wow_map::ManagedMapKind::Dungeon {
            has_reset_schedule: true,
        },
    );
    session.set_canonical_map_manager(canonical);

    let mgr = Arc::new(RwLock::new(wow_instances::InstanceLockMgr::default()));
    session.set_instance_lock_mgr(Arc::clone(&mgr));
    mgr
}

pub(super) fn install_represented_guild_bank_like_cpp(
    session: &mut crate::session::WorldSession,
    banker: ObjectGuid,
    guild_id: u64,
) {
    let canonical = Arc::new(Mutex::new(wow_map::MapManager::default()));
    let position = Position::new(14.0, 0.0, 0.0, 0.0);

    session.set_loaded_player_identity_like_cpp(571, 1, 1, 10, 0);
    session.set_player_position_like_cpp(Position::new(10.0, 0.0, 0.0, 0.0));
    session.set_represented_guild_id_like_cpp(guild_id);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.record_represented_gameobject_runtime_state_like_cpp(
        571,
        banker,
        777,
        position,
        wow_entities::GAMEOBJECT_TYPE_GUILD_BANK as u8,
    );

    let mut gameobject = wow_entities::GameObject::new();
    gameobject.world_mut().object_mut().create(banker);
    gameobject.world_mut().object_mut().set_entry(777);
    gameobject.world_mut().set_map(571, 0).unwrap();
    gameobject.world_mut().relocate(position);
    gameobject.world_mut().object_mut().add_to_world();
    canonical
        .lock()
        .unwrap()
        .create_world_map(571, 0)
        .map_mut()
        .insert_map_object_record(
            wow_entities::MapObjectRecord::new_game_object(gameobject).unwrap(),
        )
        .unwrap();
}

fn misc_test_creature_create_data(
    guid: ObjectGuid,
    entry: u32,
    npc_flags: u64,
) -> wow_packet::packets::update::CreatureCreateData {
    wow_packet::packets::update::CreatureCreateData {
        guid,
        entry,
        display_id: 100,
        native_display_id: 100,
        display_scale: 1.0,
        native_x_display_scale: 1.0,
        bounding_radius: 0.389,
        combat_reach: 1.5,
        health: 100,
        max_health: 100,
        level: 80,
        faction_template: 35,
        npc_flags,
        unit_flags: 0,
        unit_flags2: 0,
        unit_flags3: 0,
        aura_state: 0x00D0_0000, // full-HP creature, mirrors C++ ModifyAuraState
        damage_school: wow_constants::spell::SpellSchools::Normal as u8,
        scale: 1.0,
        unit_class: 1,
        display_power: 1,
        power: [0; 10],
        max_power: [0; 10],
        base_mana: 0,
        virtual_items: [(0, 0, 0); 3],
        base_attack_time: 2000,
        ranged_attack_time: 0,
        movement_flags: 0,
        vehicle_id: 0,
        play_hover_anim: false,
        hover_height: 1.0,
        mount_display_id: 0,
        stand_state: 0,
        vis_flags: 0,
        anim_tier: 0,
        emote_state: 0,
        sheathe_state: wow_constants::unit::SheathState::Melee as u8,
        pvp_flags: 0,
        current_area_id: 0,
        speed_walk_rate: 1.0,
        speed_run_rate: 1.14286,
        ai_anim_kit_id: 0,
        movement_anim_kit_id: 0,
        melee_anim_kit_id: 0,
    }
}

pub(super) fn register_misc_test_creature(
    session: &mut crate::session::WorldSession,
    guid: ObjectGuid,
    entry: u32,
    npc_flags: u64,
) {
    session.set_map_manager(Arc::new(RwLock::new(crate::map_manager::MapManager::new())));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    session.register_world_creature(
        571,
        Position::new(12.0, 0.0, 0.0, 0.0),
        misc_test_creature_create_data(guid, entry, npc_flags),
        1,
        2,
        5.0,
        0,
        0,
        0,
        0,
        None,
        0,
        0,
        0,
        0,
        -1,
    );
}

pub(super) fn broadcast_info_with_command_tx(
    command_tx: flume::Sender<SessionCommand>,
) -> PlayerSessionRegistrationLikeCpp {
    let (send_tx, _send_rx) = flume::bounded::<Vec<u8>>(4);
    PlayerSessionRegistrationLikeCpp {
        identity: PlayerDirectoryIdentityLikeCpp {
            player_name: "TestPlayer".to_string(),
            account_id: 1,
            recruiter_id: 0,
            race: 1,
            class: 1,
            sex: 0,
            active_expansion: 2,
        },
        placement: PlayerDirectoryPlacementLikeCpp {
            map_id: 571,
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
        durable_creature_runtime_commands_like_cpp: Default::default(),
        client_visible_guids_like_cpp: Default::default(),
        advanced_combat_logging_enabled_like_cpp: Default::default(),
        visibility_refresh_pending_like_cpp: Default::default(),
    }
}

pub(super) fn add_canonical_flight_master_for_misc_test(
    canonical: &crate::session::SharedCanonicalMapManager,
    guid: ObjectGuid,
    position: Position,
) {
    let mut creature = wow_entities::Creature::new(false);
    creature.unit_mut().world_mut().object_mut().create(guid);
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .set_entry(90_001);
    creature.unit_mut().world_mut().set_map(571, 0).unwrap();
    creature.unit_mut().world_mut().relocate(position);
    creature.unit_mut().world_mut().set_combat_reach(1.0);
    creature.unit_mut().set_level(80);
    creature.unit_mut().set_max_health(100);
    creature.unit_mut().set_health(100);
    creature.unit_mut().world_mut().object_mut().add_to_world();
    creature.set_ai_identity_runtime(1, 35, 0x2000, 0);

    canonical
        .lock()
        .unwrap()
        .create_world_map(571, 0)
        .map_mut()
        .insert_map_object_record(wow_entities::MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();
}

pub(super) fn add_canonical_auctioneer_for_misc_test(
    canonical: &crate::session::SharedCanonicalMapManager,
    guid: ObjectGuid,
    position: Position,
    npc_flags: u32,
) {
    let mut creature = wow_entities::Creature::new(false);
    creature.unit_mut().world_mut().object_mut().create(guid);
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .set_entry(90_002);
    creature.unit_mut().world_mut().set_map(571, 0).unwrap();
    creature.unit_mut().world_mut().relocate(position);
    creature.unit_mut().world_mut().set_combat_reach(1.0);
    creature.unit_mut().set_level(80);
    creature.unit_mut().set_max_health(100);
    creature.unit_mut().set_health(100);
    creature.unit_mut().world_mut().object_mut().add_to_world();
    creature.set_ai_identity_runtime(1, 35, npc_flags, 0);

    canonical
        .lock()
        .unwrap()
        .create_world_map(571, 0)
        .map_mut()
        .insert_map_object_record(wow_entities::MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();
}

pub(super) fn install_trade_test_spell(session: &mut crate::session::WorldSession, spell_id: i32) {
    let mut spell_store = SpellStore::new();
    spell_store.insert(spell_id, trade_test_spell_info(spell_id));
    session.set_spell_store(Arc::new(spell_store));
    session.set_known_spells_like_cpp(vec![spell_id]);
}

pub(super) fn insert_trade_test_item(
    session: &mut crate::session::WorldSession,
    owner_guid: ObjectGuid,
    slot: u8,
    item_guid: ObjectGuid,
    entry_id: u32,
) {
    session.insert_inventory_item_like_cpp(
        slot,
        crate::session::InventoryItem {
            guid: item_guid,
            entry_id,
            db_guid: item_guid.counter() as u64,
            inventory_type: None,
        },
    );
    let item = session.make_inventory_item_object(
        item_guid,
        entry_id,
        owner_guid,
        1,
        0,
        ItemContext::None,
        slot,
    );
    session.insert_inventory_item_object(item);
}

pub(super) fn install_add_toy_item_templates(
    session: &mut crate::session::WorldSession,
    toy_item_id: u32,
    toy_flags2: u32,
) {
    session.set_item_store(Arc::new(ItemStore::from_records([
        ItemRecord {
            id: 101,
            class_id: wow_constants::ItemClass::Container as u8,
            subclass_id: 0,
            material: 0,
            inventory_type: wow_constants::InventoryType::Bag as i8,
            sheathe_type: 0,
            random_select: 0,
            random_suffix_group_id: 0,
            scaling_stat_distribution_id: 0,
            scaling_stat_value: 0,
        },
        ItemRecord {
            id: toy_item_id,
            class_id: wow_constants::ItemClass::Miscellaneous as u8,
            subclass_id: 0,
            material: 0,
            inventory_type: wow_constants::InventoryType::NonEquip as i8,
            sheathe_type: 0,
            random_select: 0,
            random_suffix_group_id: 0,
            scaling_stat_distribution_id: 0,
            scaling_stat_value: 0,
        },
    ])));
    session.set_item_search_name_store(Arc::new(ItemSearchNameStore::from_entries([
        ItemSearchNameEntry {
            id: 101,
            allowable_race: 0,
            display: String::new(),
            overall_quality_id: 1,
            expansion_id: 0,
            min_faction_id: 0,
            min_reputation: 0,
            allowable_class: 0,
            required_level: 0,
            required_skill: 0,
            required_skill_rank: 0,
            required_ability: 0,
            item_level: 1,
            flags: [0; 4],
        },
        ItemSearchNameEntry {
            id: toy_item_id,
            allowable_race: 0,
            display: String::new(),
            overall_quality_id: 1,
            expansion_id: 0,
            min_faction_id: 0,
            min_reputation: 0,
            allowable_class: 0,
            required_level: 0,
            required_skill: 0,
            required_skill_rank: 0,
            required_ability: 0,
            item_level: 1,
            flags: [0; 4],
        },
    ])));
    session.set_item_stats_store(Arc::new(
        ItemStatsStore::from_sparse_and_random_property_templates(
            [
                (
                    101,
                    ItemSparseTemplateEntry {
                        flags: [0; 4],
                        bag_family: 0,
                        start_quest_id: 0,
                        stackable: 1,
                        max_count: 0,
                        lock_id: 0,
                        required_reputation_rank: 0,
                        sell_price: 0,
                        buy_price: 0,
                        vendor_stack_count: 1,
                        price_variance: 0.0,
                        price_random_value: 0.0,
                        max_durability: 0,
                        other_faction_item_id: 0,
                        content_tuning_id: 0,
                        player_level_to_item_level_curve_id: 0,
                        limit_category: 0,
                        instance_bound: 0,
                        zone_bound: [0, 0],
                        required_reputation_faction: 0,
                        allowable_class: 0,
                        required_expansion: 0,
                        bonding: wow_constants::ItemBondingType::None as u8,
                        container_slots: 12,
                        inventory_type: wow_constants::InventoryType::Bag as i8,
                    },
                ),
                (
                    toy_item_id,
                    ItemSparseTemplateEntry {
                        flags: [0, toy_flags2, 0, 0],
                        bag_family: 0,
                        start_quest_id: 0,
                        stackable: 1,
                        max_count: 0,
                        lock_id: 0,
                        required_reputation_rank: 0,
                        sell_price: 0,
                        buy_price: 0,
                        vendor_stack_count: 1,
                        price_variance: 0.0,
                        price_random_value: 0.0,
                        max_durability: 0,
                        other_faction_item_id: 0,
                        content_tuning_id: 0,
                        player_level_to_item_level_curve_id: 0,
                        limit_category: 0,
                        instance_bound: 0,
                        zone_bound: [0, 0],
                        required_reputation_faction: 0,
                        allowable_class: 0,
                        required_expansion: 0,
                        bonding: wow_constants::ItemBondingType::None as u8,
                        container_slots: 0,
                        inventory_type: wow_constants::InventoryType::NonEquip as i8,
                    },
                ),
            ],
            [],
        ),
    ));
}

pub(super) fn shared_canonical_map_manager_for_misc_test()
-> crate::session::SharedCanonicalMapManager {
    Arc::new(Mutex::new(wow_map::MapManager::default()))
}

pub(super) fn add_canonical_test_player_on_map_for_misc_test(
    canonical: &crate::session::SharedCanonicalMapManager,
    guid: ObjectGuid,
    position: Position,
    map_id: u32,
    instance_id: u32,
) {
    let mut player = wow_entities::Player::new(Some(1), false);
    player.unit_mut().world_mut().object_mut().create(guid);
    player.unit_mut().world_mut().set_name("ToyDynamicTester");
    player
        .unit_mut()
        .world_mut()
        .set_map(map_id, instance_id)
        .unwrap();
    player.unit_mut().world_mut().relocate(position);
    player.unit_mut().set_max_health(100);
    player.unit_mut().set_health(100);
    player.unit_mut().set_faction(1);
    player.unit_mut().world_mut().object_mut().add_to_world();

    canonical
        .lock()
        .unwrap()
        .create_world_map(map_id, instance_id)
        .map_mut()
        .insert_map_object_record(wow_entities::MapObjectRecord::new_player(player).unwrap())
        .unwrap();
}
