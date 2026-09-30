//! Shared character-handler test fixtures, part 3.
//!
//! Separated from the character_tests root under #662; every fixture is unchanged.

use super::*;

pub(super) fn install_equippable_item_fixture(
    session: &mut WorldSession,
    entry_id: u32,
    inventory_type: InventoryType,
    strength: Option<i16>,
) {
    session.set_item_store(Arc::new(wow_data::ItemStore::from_records([ItemRecord {
        id: entry_id,
        class_id: ItemClass::Weapon as u8,
        subclass_id: ItemSubClassWeapon::Sword as u8,
        material: 0,
        inventory_type: inventory_type as i8,
        sheathe_type: 0,
        random_select: 0,
        random_suffix_group_id: 0,
        scaling_stat_distribution_id: 0,
        scaling_stat_value: 0,
    }])));
    let sparse = ItemSparseTemplateEntry {
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
        price_variance: 1.0,
        price_random_value: 1.0,
        max_durability: 100,
        other_faction_item_id: 0,
        content_tuning_id: 0,
        player_level_to_item_level_curve_id: 0,
        limit_category: 0,
        instance_bound: 0,
        zone_bound: [0; 2],
        required_reputation_faction: 0,
        allowable_class: -1,
        required_expansion: 0,
        bonding: ItemBondingType::None as u8,
        container_slots: 0,
        inventory_type: inventory_type as i8,
    };
    let stats = strength.into_iter().map(|amount| {
        (
            entry_id,
            ItemStatEntry {
                stats: std::array::from_fn(|index| {
                    if index == 0 {
                        (ItemModType::Strength as i8, amount)
                    } else {
                        (ItemModType::None as i8, 0)
                    }
                }),
                resistances: [0; 7],
                armor: 0,
            },
        )
    });
    session.set_item_stats_store(Arc::new(
        ItemStatsStore::from_stats_sparse_and_random_property_templates(
            stats,
            [(entry_id, sparse)],
            [],
        ),
    ));
}

pub(super) fn insert_bank_move_test_item(
    session: &mut WorldSession,
    slot: u8,
    entry_id: u32,
    db_guid: u64,
    count: u32,
) -> ObjectGuid {
    let player_guid = session.player_guid().expect("test player");
    let item_guid = ObjectGuid::create_item(1, db_guid as i64);
    insert_inventory_item_for_test(session,
        slot,
        InventoryItem {
            guid: item_guid,
            entry_id,
            db_guid,
            inventory_type: Some(InventoryType::NonEquip as u8),
        },
    );
    let item = make_inventory_item_object_for_test(&session,
        item_guid,
        entry_id,
        player_guid,
        count,
        0,
        ItemContext::None,
        slot,
    );
    insert_inventory_item_object_for_test(session, item);
    item_guid
}

pub(super) fn chr_class_entry(id: u32, cinematic_sequence_id: u16) -> ChrClassesEntry {
    ChrClassesEntry {
        id,
        name: String::new(),
        filename: String::new(),
        name_male: String::new(),
        name_female: String::new(),
        pet_name_token: String::new(),
        create_screen_file_data_id: 0,
        select_screen_file_data_id: 0,
        icon_file_data_id: 0,
        low_res_screen_file_data_id: 0,
        flags: 0,
        starting_level: 1,
        armor_type_mask: 0,
        cinematic_sequence_id,
        default_spec: 0,
        has_strength_attack_bonus: 0,
        primary_stat_priority: 0,
        display_power: 0,
        ranged_attack_power_per_agility: 0,
        attack_power_per_agility: 0,
        attack_power_per_strength: 0,
        spell_class_set: 0,
        roles_mask: 0,
        damage_bonus_stat: 0,
        has_relic_slot: 0,
    }
}

pub(super) fn chr_race_entry(id: u32, cinematic_sequence_id: i16) -> ChrRacesEntry {
    ChrRacesEntry {
        id,
        client_prefix: String::new(),
        client_file_string: String::new(),
        name: String::new(),
        flags: 0,
        male_display_id: 0,
        female_display_id: 0,
        high_res_male_display_id: 0,
        high_res_female_display_id: 0,
        res_sickness_spell_id: 0,
        splash_sound_id: 0,
        create_screen_file_data_id: 0,
        select_screen_file_data_id: 0,
        low_res_screen_file_data_id: 0,
        altered_form_start_visual_kit_id: [0; 3],
        altered_form_finish_visual_kit_id: [0; 3],
        heritage_armor_achievement_id: 0,
        starting_level: 1,
        ui_display_order: 0,
        playable_race_bit: 0,
        female_skeleton_file_data_id: 0,
        male_skeleton_file_data_id: 0,
        helmet_anim_scaling_race_id: 0,
        transmogrify_disabled_slot_mask: 0,
        faction_id: 0,
        cinematic_sequence_id,
        base_language: 0,
        creature_type: 0,
        alliance: 0,
        race_related: 0,
        unaltered_visual_race_id: 0,
        default_class_id: 0,
        neutral_race_id: 0,
    }
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
        reward_currencies: [0; wow_data::quest::QUEST_REWARD_CURRENCY_COUNT],
        reward_currency_amounts: [0; wow_data::quest::QUEST_REWARD_CURRENCY_COUNT],
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

pub(super) fn store_with_quests(ids: &[u32]) -> QuestStore {
    QuestStore::from_quests_like_cpp(ids.iter().copied().map(quest_template))
}

pub(super) fn creature_guid(entry: u32, counter: i64) -> ObjectGuid {
    ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, entry, counter)
}

pub(super) fn gameobject_guid(entry: u32, counter: i64) -> ObjectGuid {
    ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, entry, counter)
}

pub(super) fn faction_template_entry(
    id: u32,
    faction: u16,
    faction_group: u8,
    friend_group: u8,
    enemy: u16,
) -> wow_data::progression_rewards::FactionTemplateEntry {
    let mut enemies = [0; 8];
    enemies[0] = enemy;
    wow_data::progression_rewards::FactionTemplateEntry {
        id,
        faction,
        flags: 0,
        faction_group,
        friend_group,
        enemy_group: 0,
        enemies,
        friend: [0; 8],
    }
}

pub(super) fn insert_creature(manager: &mut wow_map::MapManager, guid: ObjectGuid, entry: u32) {
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
        .relocate(Position::new(10.0, 0.0, 0.0, 0.0));
    creature.unit_mut().set_level(80);
    creature.set_ai_identity_runtime(1, 35, NPCFlags1::QUEST_GIVER.bits(), 0);
    manager
        .create_world_map(571, 0)
        .map_mut()
        .insert_map_object_record(wow_entities::MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();
}

pub(super) fn insert_gameobject(manager: &mut wow_map::MapManager, guid: ObjectGuid, entry: u32) {
    let mut gameobject = wow_entities::GameObject::new();
    gameobject.world_mut().object_mut().create(guid);
    gameobject.world_mut().object_mut().set_entry(entry);
    gameobject.world_mut().set_map(571, 0).unwrap();
    gameobject
        .world_mut()
        .relocate(Position::new(10.0, 0.0, 0.0, 0.0));
    gameobject.world_mut().object_mut().add_to_world();
    manager
        .create_world_map(571, 0)
        .map_mut()
        .insert_map_object_record(
            wow_entities::MapObjectRecord::new_game_object(gameobject).unwrap(),
        )
        .unwrap();
}

pub(super) fn insert_gossip_gameobject(
    manager: &Arc<std::sync::Mutex<wow_map::MapManager>>,
    guid: ObjectGuid,
    entry: u32,
    position: Position,
    go_type: u8,
    is_in_world: bool,
) {
    let mut gameobject = wow_entities::GameObject::new();
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
        .insert_map_object_record(
            wow_entities::MapObjectRecord::new_game_object(gameobject).unwrap(),
        )
        .unwrap();
}

pub(super) fn insert_banker_creature(
    manager: &Arc<std::sync::Mutex<wow_map::MapManager>>,
    guid: ObjectGuid,
    npc_flags: u32,
) {
    let mut manager = manager.lock().unwrap();
    insert_canonical_creature_with_npc_flags(&mut manager, guid, 2456, npc_flags);
}

pub(super) fn insert_canonical_creature_with_npc_flags(
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
        .insert_map_object_record(wow_entities::MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();
}

pub(super) fn attach_map_manager(session: &mut WorldSession, manager: wow_map::MapManager) {
    session.set_canonical_map_manager(Arc::new(std::sync::Mutex::new(manager)));
}

pub(super) fn attach_legacy_creature(
    session: &mut WorldSession,
    guid: ObjectGuid,
    entry: u32,
    npc_flags: u32,
) {
    let manager = Arc::new(std::sync::RwLock::new(crate::map_manager::MapManager::new()));
    manager.write().unwrap().add_creature(
        571,
        0,
        0,
        0,
        crate::map_manager::WorldCreature::new(
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

pub(super) fn tracked_query_packet(guids: &[ObjectGuid]) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint32(guids.len() as u32);
    for guid in guids {
        pkt.write_packed_guid(guid);
    }
    pkt
}

pub(super) fn quest_giver_hello_packet(guid: ObjectGuid) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_packed_guid(&guid);
    pkt.reset_read();
    pkt
}

pub(super) fn gossip_message_counts(bytes: &[u8], expected_guid: ObjectGuid) -> (i32, i32) {
    assert_eq!(
        wow_packet::WorldPacket::from_bytes(bytes).server_opcode(),
        Some(ServerOpcodes::GossipMessage)
    );
    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(pkt.read_packed_guid().unwrap(), expected_guid);
    let _gossip_id = pkt.read_int32().unwrap();
    let _friendship_faction_id = pkt.read_int32().unwrap();
    let option_count = pkt.read_int32().unwrap();
    let quest_count = pkt.read_int32().unwrap();
    (option_count, quest_count)
}

pub(super) fn recv_status_multiple(send_rx: &flume::Receiver<Vec<u8>>) -> Vec<(ObjectGuid, u64)> {
    let bytes = send_rx
        .try_recv()
        .expect("quest giver status multiple packet");
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        wow_constants::ServerOpcodes::QuestGiverStatusMultiple as u16
    );
    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
    let count = pkt.read_int32().unwrap();
    assert!(count >= 0);
    let mut statuses = Vec::new();
    for _ in 0..count {
        statuses.push((pkt.read_packed_guid().unwrap(), pkt.read_uint64().unwrap()));
    }
    statuses
}

pub(super) fn insert_cancel_temp_enchant_test_item(
    session: &mut WorldSession,
    player_guid: ObjectGuid,
    slot: u8,
    enchantment_id: i32,
) -> ObjectGuid {
    let item_guid = ObjectGuid::create_item(1, 70_000 + i64::from(slot));
    insert_inventory_item_for_test(session,
        slot,
        InventoryItem {
            guid: item_guid,
            entry_id: 700,
            db_guid: item_guid.counter() as u64,
            inventory_type: Some(InventoryType::Weapon as u8),
        },
    );
    let mut item = make_inventory_item_object_for_test(&session,
        item_guid,
        700,
        player_guid,
        1,
        0,
        ItemContext::None,
        slot,
    );
    item.set_enchantment(
        EnchantmentSlot::EnhancementTemporary,
        enchantment_id,
        12_000,
        3,
    );
    insert_inventory_item_object_for_test(session, item);
    item_guid
}
