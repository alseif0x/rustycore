//! Shared character-handler test fixtures, part 3.
//!
//! Separated from the character_tests root under #662; every fixture is unchanged.

use super::*;

pub(super) fn seed_represented_feign_death_like_cpp(session: &mut WorldSession, slot: u8) {
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

pub(super) fn canonical_player_has_died_state_like_cpp(session: &mut WorldSession) -> bool {
    session
        .mutate_canonical_player_like_cpp(|player| {
            player
                .unit()
                .has_unit_state(wow_constants::unit::UnitState::DIED.bits())
        })
        .expect("canonical player")
}

pub(super) fn install_bind_spell_fixture(session: &mut WorldSession) {
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

pub(super) fn make_binder_observer(
    guid_counter: u32,
    position: Position,
    innkeeper: ObjectGuid,
    visible: bool,
    registry: &Arc<crate::session::directory::PlayerRegistry>,
    canonical: &Arc<std::sync::Mutex<wow_map::MapManager>>,
) -> (WorldSession, flume::Receiver<Vec<u8>>) {
    let (mut observer, send_rx) = make_session_with_send_capacity(4);
    let guid = ObjectGuid::create_player(1, i64::from(guid_counter));
    observer.set_canonical_map_manager(Arc::clone(canonical));
    observer.attach_player_controller_like_cpp(crate::session::SessionPlayerController::new(
        guid,
        format!("Observer{guid_counter}"),
        position,
        571,
        1,
        1,
        80,
        0,
    ));
    observer.set_state(crate::session::SessionState::LoggedIn);
    observer.set_player_registry(Arc::clone(registry));
    if canonical
        .lock()
        .unwrap()
        .find_map(571, 0)
        .and_then(|map| map.map().get_typed_player(guid))
        .is_none()
    {
        let mut player = wow_entities::Player::new(Some(1), false);
        player.unit_mut().world_mut().object_mut().create(guid);
        player.unit_mut().world_mut().set_map(571, 0).unwrap();
        player.unit_mut().world_mut().relocate(position);
        player.unit_mut().world_mut().object_mut().add_to_world();
        canonical
            .lock()
            .unwrap()
            .create_world_map(571, 0)
            .map_mut()
            .insert_map_object_record(wow_entities::MapObjectRecord::new_player(player).unwrap())
            .unwrap();
    }
    if visible {
        observer.client_visible_guids_like_cpp.insert(innkeeper);
    }
    observer.register_in_player_registry();
    assert!(registry.fixture_update(guid, |placement| {
        placement.is_in_world = true;
        placement.position = position;
    }));
    (observer, send_rx)
}

pub(super) fn install_bank_move_item_fixture(
    session: &mut WorldSession,
    entry_id: u32,
    max_stack_size: i32,
) {
    session.set_item_store(Arc::new(wow_data::ItemStore::from_records([ItemRecord {
        id: entry_id,
        class_id: ItemClass::Miscellaneous as u8,
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
        entry_id,
        ItemSparseTemplateEntry {
            flags: [0; 4],
            bag_family: 0,
            start_quest_id: 0,
            stackable: max_stack_size,
            max_count: 0,
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
            zone_bound: [0; 2],
            required_reputation_faction: 0,
            allowable_class: -1,
            required_expansion: 0,
            bonding: ItemBondingType::None as u8,
            container_slots: 0,
            inventory_type: InventoryType::NonEquip as i8,
        },
    )])));
}

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
    session.insert_inventory_item_like_cpp(
        slot,
        InventoryItem {
            guid: item_guid,
            entry_id,
            db_guid,
            inventory_type: Some(InventoryType::NonEquip as u8),
        },
    );
    let item = session.make_inventory_item_object(
        item_guid,
        entry_id,
        player_guid,
        count,
        0,
        ItemContext::None,
        slot,
    );
    session.insert_inventory_item_object(item);
    item_guid
}

pub(super) fn insert_equippable_test_item(
    session: &mut WorldSession,
    bag: u8,
    slot: u8,
    entry_id: u32,
    db_guid: u64,
    inventory_type: InventoryType,
) -> ObjectGuid {
    let player_guid = session.player_guid().expect("test player");
    let item_guid = ObjectGuid::create_item(1, db_guid as i64);
    if bag == INVENTORY_SLOT_BAG_0 {
        session.insert_inventory_item_like_cpp(
            slot,
            InventoryItem {
                guid: item_guid,
                entry_id,
                db_guid,
                inventory_type: Some(inventory_type as u8),
            },
        );
    }
    let mut item = session.make_inventory_item_object(
        item_guid,
        entry_id,
        player_guid,
        1,
        0,
        ItemContext::None,
        slot,
    );
    if bag != INVENTORY_SLOT_BAG_0 {
        let bag_guid = session
            .inventory_items_like_cpp()
            .get(&bag)
            .expect("represented bag")
            .guid;
        item.set_container_guid_and_slot(bag_guid, bag);
    }
    session.insert_inventory_item_object(item);
    item_guid
}

pub(super) fn make_hearth_and_resurrect_session(
    area_flags: u32,
) -> (WorldSession, flume::Receiver<Vec<u8>>) {
    let (mut session, send_rx) = make_session_with_send_capacity(4);
    session.set_map_store(crate::teleport_test_fixtures::world_maps([571]));
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    crate::canonical_player_access::install_canonical_player_owner_for_test(&mut session, 571, 0);
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    session.set_player_position_like_cpp(Position::new(1.0, 2.0, 3.0, 0.5));
    session.set_player_zone_area_like_cpp(10, 77);
    session.set_player_alive_like_cpp(false);
    session.set_area_table_store(Arc::new(wow_data::AreaTableStore::from_entries([
        wow_data::AreaTableEntry {
            id: 77,
            continent_id: 571,
            parent_area_id: 0,
            area_bit: -1,
            exploration_level: 0,
            mount_flags: 0,
            flags: area_flags,
        },
    ])));
    let _ = session.set_represented_homebind_like_cpp(RepresentedHomebindLikeCpp {
        map_id: 571,
        area_id: 77,
        position: Position::new(10.0, 20.0, 30.0, 1.5),
    });
    (session, send_rx)
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

pub(super) fn expected_trigger_cinematic(cinematic_id: u32) -> Vec<u8> {
    let mut expected = (wow_constants::ServerOpcodes::TriggerCinematic as u16)
        .to_le_bytes()
        .to_vec();
    expected.extend_from_slice(&cinematic_id.to_le_bytes());
    expected.extend_from_slice(&ObjectGuid::EMPTY.to_raw_bytes());
    expected
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

pub(super) fn insert_area_spirit_healer_creature(
    manager: &Arc<std::sync::Mutex<wow_map::MapManager>>,
    guid: ObjectGuid,
    position: Position,
    npc_flags: u32,
    npc_flags2: u32,
) {
    let mut creature = wow_entities::Creature::new(false);
    creature.unit_mut().world_mut().object_mut().create(guid);
    creature.unit_mut().world_mut().object_mut().set_entry(91);
    creature.unit_mut().world_mut().set_map(571, 0).unwrap();
    creature.unit_mut().world_mut().relocate(position);
    creature.unit_mut().world_mut().set_combat_reach(1.0);
    creature.unit_mut().set_level(80);
    creature.unit_mut().set_max_health(100);
    creature.unit_mut().set_health(100);
    creature.set_ai_identity_runtime(1, 35, npc_flags, 0);
    creature.set_npc_flags2_runtime_like_cpp(npc_flags2);
    creature.unit_mut().world_mut().object_mut().add_to_world();

    manager
        .lock()
        .unwrap()
        .create_world_map(571, 0)
        .map_mut()
        .insert_map_object_record(wow_entities::MapObjectRecord::new_creature(creature).unwrap())
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

pub(super) fn mark_gameobject_questgiver(session: &mut WorldSession, guid: ObjectGuid) {
    let mut state = crate::session::RepresentedGameObjectUseState::default();
    state.go_type = Some(wow_entities::GAMEOBJECT_TYPE_QUESTGIVER as u8);
    session
        .represented_gameobject_use_states
        .insert(guid, state);
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

pub(super) fn gossip_catalog_option_like_cpp(
    menu_id: u32,
    option_id: u32,
    broadcast_text_id: u32,
) -> GossipMenuOptionCatalogRowLikeCpp {
    GossipMenuOptionCatalogRowLikeCpp {
        menu_id,
        gossip_option_id: 77,
        option_id,
        option_npc: 1,
        option_text: "Original option".to_owned(),
        option_broadcast_text_id: broadcast_text_id,
        language: 0,
        flags: 3,
        action_menu_id: 88,
        action_poi_id: 0,
        gossip_npc_option_id: None,
        box_coded: false,
        box_money: 25,
        box_text: "Confirm".to_owned(),
        box_broadcast_text_id: 0,
        spell_id: Some(99),
        override_icon_id: Some(4),
    }
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
    session.insert_inventory_item_like_cpp(
        slot,
        InventoryItem {
            guid: item_guid,
            entry_id: 700,
            db_guid: item_guid.counter() as u64,
            inventory_type: Some(InventoryType::Weapon as u8),
        },
    );
    let mut item = session.make_inventory_item_object(
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
    session.insert_inventory_item_object(item);
    item_guid
}

pub(super) fn character_enumeration_row_like_cpp() -> CharacterEnumerationRowLikeCpp {
    CharacterEnumerationRowLikeCpp {
        guid_low: 42,
        name: "PortBoundary".to_owned(),
        race: 1,
        class: 1,
        gender: 0,
        level: 20,
        zone: 12,
        map: 0,
        position_x: 1.0,
        position_y: 2.0,
        position_z: 3.0,
        guild_id: 0,
        player_flags: 0,
        at_login_flags: 0,
        pet_entry: 0,
        pet_display_id: 0,
        pet_level: 0,
        equipment_cache: String::new(),
        banned_guid: 0,
        list_slot: 0,
        last_played_time: 100,
        active_talent_group: 0,
        last_login_build: 54261,
        declined_genitive: "PortBoundaryGenitive".to_owned(),
    }
}

pub(super) fn creature_query_catalog_row_like_cpp() -> CreatureQueryTemplateLikeCpp {
    CreatureQueryTemplateLikeCpp {
        entry: 42,
        name: "Localized creature".to_owned(),
        subname: "Localized title".to_owned(),
        title_alt: "Localized alternate".to_owned(),
        icon_name: "Directions".to_owned(),
        creature_type: 7,
        creature_family: 8,
        classification: 9,
        kill_credits: [10, 11],
        civilian: true,
        racial_leader: false,
        movement_id: 12,
        required_expansion: 3,
        vignette_id: 13,
        unit_class: 1,
        widget_set_id: 14,
        widget_set_unit_condition_id: 15,
        hp_multi: 1.5,
        energy_multi: 2.5,
        creature_difficulty_id: 16,
        type_flags: [17, 18],
        displays: vec![CreatureQueryDisplayLikeCpp {
            display_id: 19,
            scale: 0.75,
            probability: 0.25,
        }],
    }
}

pub(super) fn gameobject_query_catalog_row_like_cpp() -> GameObjectQueryTemplateLikeCpp {
    let mut data = [0_i32; wow_data::WORLD_QUERY_GAMEOBJECT_DATA_COUNT_LIKE_CPP];
    data[0] = 7;
    data[34] = 41;
    GameObjectQueryTemplateLikeCpp {
        entry: 42,
        go_type: 3,
        display_id: 4,
        name: "Localized object".to_owned(),
        icon_name: "Directions".to_owned(),
        cast_bar_caption: "Opening".to_owned(),
        unk_string: "Unknown".to_owned(),
        size: 1.25,
        data,
        content_tuning_id: 42,
        min_money: 0,
        max_money: 0,
    }
}

pub(super) fn enum_pet_template_store(
    entry: u32,
    family: u32,
) -> wow_data::CreatureTemplateLifecycleStoreLikeCpp {
    wow_data::CreatureTemplateLifecycleStoreLikeCpp::from_templates([
        wow_data::CreatureTemplateLifecycleRecordLikeCpp {
            entry,
            name: String::new(),
            ai_name: String::new(),
            script_name: String::new(),
            required_expansion: 0,
            faction: 0,
            npc_flags: 0,
            speed_walk: 1.0,
            speed_run: 1.0,
            scale: 1.0,
            classification: 0,
            damage_school: 0,
            unit_flags: 0,
            unit_flags2: 0,
            unit_flags3: 0,
            creature_type: 0,
            family,
            trainer_class: 0,
            unit_class: 0,
            vehicle_id: 0,
            movement_type: 0,
            ground_movement_type: 1,
            swim_allowed: true,
            flight_movement_type: 0,
            rooted: false,
            chase_movement_type: 0,
            random_movement_type: 0,
            interaction_pause_timer_ms: 180_000,
            flags_extra: 0,
            string_id: String::new(),
            regen_health: true,
            spells: [0; wow_data::MAX_CREATURE_SPELLS_LIKE_CPP],
            models: Vec::new(),
        },
    ])
}
