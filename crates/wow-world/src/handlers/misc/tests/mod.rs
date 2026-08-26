//! Behaviour tests for [`super`].
//!
//! Extracted from `misc.rs`. Moving tests moves no invariant: the
//! production module boundary, its visibility and its owners are untouched.
//!
//! Dedenting by one level lets rustfmt collapse some argument lists onto a single
//! line, which drops their trailing commas; that is the only difference from the
//! original text.

#![cfg(test)]

mod account_data;
mod arena;
mod auction;
mod battle_pet;
mod calendar;
mod chat;
mod client_state;
mod collections;
mod corpse;
mod gameobject;
mod guild;
mod instance;
mod lfg;
mod player;
mod pvp;
mod reputation;
mod support;
mod trade;
mod travel;

use super::*;
use crate::session::directory::{
    PlayerBroadcastInfo, PlayerDirectoryIdentityLikeCpp, PlayerDirectoryPlacementLikeCpp,
    PlayerRegistry, PlayerSessionRegistrationLikeCpp,
};
use crate::session::mailbox::SessionCommand;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, RwLock};
use wow_constants::{
    ClientOpcodes, ConditionSourceType, ConditionType, ItemContext, ServerOpcodes,
    shared::DifficultyFlags,
};
use wow_core::{ObjectGuid, Position, guid::HighGuid};
use wow_data::area::AREA_FLAG_IS_SUBZONE_LIKE_CPP;
use wow_data::progression_rewards::{FactionEntry, FactionStore};
use wow_data::quest::{
    QUEST_ITEM_DROP_COUNT, QUEST_REWARD_CHOICES_COUNT, QUEST_REWARD_CURRENCY_COUNT,
    QUEST_REWARD_DISPLAY_SPELL_COUNT, QUEST_REWARD_ITEM_COUNT, QUEST_REWARD_REPUTATIONS_COUNT,
    QUEST_SPECIAL_FLAGS_DF_QUEST_LIKE_CPP, QuestStore, QuestTemplate,
};
use wow_data::reputation::{ReputationFlagsLikeCpp, ReputationRankLikeCpp};
use wow_data::{
    Condition, ConditionEntriesByTypeStore, DifficultyEntry, DifficultyStore, GraveyardStore,
    ItemRecord, ItemSearchNameEntry, ItemSearchNameStore, ItemSparseTemplateEntry, ItemStatsStore,
    ItemStore, MapDifficultyEntry, MapDifficultyStore, MapEntry, MapStore, SpellInfo, SpellStore,
};
use wow_database::SqlParam;
use wow_packet::ServerPacket;
use wow_packet::WorldPacket;
use wow_packet::packets::misc::TRADE_STATUS_INITIATED_LIKE_CPP;
use wow_packet::packets::misc::compress_account_data_like_cpp;
use wow_packet::packets::misc::{
    EQUIP_ERR_NOT_ENOUGH_MONEY_LIKE_CPP, TRADE_STATUS_FAILED_LIKE_CPP,
};
use wow_packet::packets::misc::{SUPPORT_SPAM_TYPE_CHAT_LIKE_CPP, empty_battle_pet_guid_like_cpp};
use wow_packet::packets::misc::{
    TRADE_STATUS_ACCEPTED_LIKE_CPP, TRADE_STATUS_STATE_CHANGED_LIKE_CPP,
    TRADE_STATUS_UNACCEPTED_LIKE_CPP,
};
use wow_social::group::{
    DIFFICULTY_NORMAL_LIKE_CPP, GROUP_FLAG_LFG_LIKE_CPP, GroupInfo, GroupRegistry, PendingInvites,
};

fn currency_entry(id: u32) -> wow_data::CurrencyTypesEntry {
    wow_data::CurrencyTypesEntry {
        id,
        category_id: 0,
        inventory_icon_file_id: 0,
        spell_weight: 0,
        spell_category: 0,
        max_qty: 0,
        max_earnable_per_week: 0,
        quality: 0,
        faction_id: 0,
        award_condition_id: 0,
        flags: wow_constants::CurrencyTypesFlags::empty(),
        flags_b: wow_constants::CurrencyTypesFlagsB::empty(),
    }
}

fn set_difficulty_request(difficulty_id: u32) -> WorldPacket {
    let mut request = WorldPacket::new_empty();
    request.write_uint32(difficulty_id);
    request.reset_read();
    request
}

fn set_dungeon_difficulty_request(difficulty_id: u32) -> WorldPacket {
    let mut request = WorldPacket::new_empty();
    request.write_uint32(difficulty_id);
    request.reset_read();
    request
}

fn set_raid_difficulty_request(difficulty_id: i32, legacy: u8) -> WorldPacket {
    let mut request = WorldPacket::new_empty();
    request.write_int32(difficulty_id);
    request.write_uint8(legacy);
    request.reset_read();
    request
}

fn difficulty_entry(id: u32, instance_type: u8, flags: DifficultyFlags) -> DifficultyEntry {
    difficulty_entry_with_toggle(id, instance_type, flags, 0)
}

fn difficulty_entry_with_toggle(
    id: u32,
    instance_type: u8,
    flags: DifficultyFlags,
    toggle_difficulty_id: u8,
) -> DifficultyEntry {
    DifficultyEntry {
        id,
        instance_type,
        flags: flags.bits(),
        fallback_difficulty_id: 0,
        toggle_difficulty_id,
    }
}

fn map_entry(id: u32, instance_type: i8) -> MapEntry {
    MapEntry {
        id,
        instance_type,
        expansion_id: 0,
        parent_map_id: -1,
        cosmetic_parent_map_id: -1,
        flags1: 0,
        flags2: 0,
    }
}

fn area_entry(id: u32, parent_area_id: u16, flags: u32) -> wow_data::AreaTableEntry {
    wow_data::AreaTableEntry {
        id,
        continent_id: 571,
        parent_area_id,
        area_bit: -1,
        exploration_level: 0,
        mount_flags: 0,
        flags,
    }
}

fn unique_temp_data_dir(test_name: &str) -> std::path::PathBuf {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock before unix epoch")
        .as_nanos();
    let data_dir = std::env::temp_dir().join(format!("rustycore-{test_name}-{unique}"));
    std::fs::create_dir_all(data_dir.join("maps")).expect("create maps test dir");
    data_dir
}

fn write_no_area_map_file_like_cpp(
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

fn make_session() -> (crate::session::WorldSession, flume::Receiver<Vec<u8>>) {
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

fn make_session_with_realm_send() -> (
    crate::session::WorldSession,
    flume::Receiver<Vec<u8>>,
    flume::Receiver<Vec<u8>>,
) {
    let (mut session, instance_rx) = make_session();
    let (realm_tx, realm_rx) = flume::bounded(8);
    session.install_realm_send_channel_for_test(realm_tx);
    (session, instance_rx, realm_rx)
}

fn request_cemetery_list_packet(extra_payload: Option<u8>) -> WorldPacket {
    let mut packet = WorldPacket::new_empty();
    if let Some(byte) = extra_payload {
        packet.write_uint8(byte);
    }
    packet.reset_read();
    packet
}

fn read_cemetery_list_response(bytes: &[u8]) -> (bool, Vec<u32>) {
    let mut packet = WorldPacket::from_bytes(bytes);
    assert_eq!(
        packet.server_opcode(),
        Some(ServerOpcodes::RequestCemeteryListResponse)
    );
    assert_eq!(
        packet.read_uint16().unwrap(),
        ServerOpcodes::RequestCemeteryListResponse as u16
    );
    let is_gossip_triggered = packet.read_bit().unwrap();
    let count = packet.read_uint32().unwrap();
    let cemetery_ids = (0..count)
        .map(|_| packet.read_uint32().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(packet.remaining(), 0);
    (is_gossip_triggered, cemetery_ids)
}

fn graveyard_store_with_links(
    zone_id: u32,
    safe_loc_ids: impl IntoIterator<Item = u32>,
    conditions: impl IntoIterator<Item = Condition>,
) -> (Arc<GraveyardStore>, Arc<ConditionEntriesByTypeStore>) {
    let mut graveyard_store = GraveyardStore::default();
    for safe_loc_id in safe_loc_ids {
        graveyard_store.add_graveyard_link_like_cpp(safe_loc_id, zone_id);
    }
    let condition_store = Arc::new(ConditionEntriesByTypeStore::from_conditions_like_cpp(
        conditions,
    ));
    graveyard_store.attach_graveyard_conditions_like_cpp(condition_store.as_ref());
    (Arc::new(graveyard_store), condition_store)
}

fn graveyard_team_condition(zone_id: u32, safe_loc_id: u32, team: u32) -> Condition {
    Condition {
        source_type: ConditionSourceType::Graveyard,
        source_group: zone_id,
        source_entry: safe_loc_id as i32,
        condition_type: ConditionType::Team,
        condition_value1: team,
        ..Condition::default()
    }
}

fn quest_template(id: u32) -> QuestTemplate {
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

fn install_pending_bind_instance_context_like_cpp(
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

fn install_represented_guild_bank_like_cpp(
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

fn auto_guild_bank_item_packet(
    banker: ObjectGuid,
    bank_tab: u8,
    bank_slot: u8,
    container_item_slot: u8,
    container_slot: Option<u8>,
) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_guid(&banker);
    pkt.write_uint8(bank_tab);
    pkt.write_uint8(bank_slot);
    pkt.write_uint8(container_item_slot);
    pkt.write_bit(container_slot.is_some());
    pkt.flush_bits();
    if let Some(container_slot) = container_slot {
        pkt.write_uint8(container_slot);
    }
    pkt.reset_read();
    pkt
}

fn guild_bank_activate_packet(banker: ObjectGuid, full_update: bool) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_guid(&banker);
    pkt.write_bit(full_update);
    pkt.flush_bits();
    pkt.reset_read();
    pkt
}

fn guild_bank_query_tab_packet(banker: ObjectGuid, tab: u8, full_update: bool) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_guid(&banker);
    pkt.write_uint8(tab);
    pkt.write_bit(full_update);
    pkt.flush_bits();
    pkt.reset_read();
    pkt
}

fn guild_bank_money_packet(banker: ObjectGuid, money: u64) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_guid(&banker);
    pkt.write_uint64(money);
    pkt.reset_read();
    pkt
}

fn guild_bank_buy_tab_packet(banker: ObjectGuid, tab: u8) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_guid(&banker);
    pkt.write_uint8(tab);
    pkt.reset_read();
    pkt
}

fn guild_bank_update_tab_packet(
    banker: ObjectGuid,
    tab: u8,
    name: &str,
    icon: &str,
) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_guid(&banker);
    pkt.write_uint8(tab);
    pkt.write_bits(name.len() as u32, 7);
    pkt.write_bits(icon.len() as u32, 9);
    pkt.flush_bits();
    pkt.write_string(name);
    pkt.write_string(icon);
    pkt.reset_read();
    pkt
}

fn guild_bank_tab_query_packet(tab: i32) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_int32(tab);
    pkt.reset_read();
    pkt
}

fn guild_bank_set_tab_text_packet(tab: i32, text: &str) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_int32(tab);
    pkt.write_bits(text.len() as u32, 14);
    pkt.flush_bits();
    pkt.write_string(text);
    pkt.reset_read();
    pkt
}

fn auto_store_guild_bank_item_packet(
    banker: ObjectGuid,
    bank_tab: u8,
    bank_slot: u8,
) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_guid(&banker);
    pkt.write_uint8(bank_tab);
    pkt.write_uint8(bank_slot);
    pkt.reset_read();
    pkt
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

fn register_misc_test_creature(
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

fn battlemaster_hello_packet(unit: ObjectGuid) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_packed_guid(&unit);
    pkt.reset_read();
    pkt
}

fn battlefield_list_packet(list_id: i32) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_int32(list_id);
    pkt.reset_read();
    pkt
}

fn battlemaster_join_packet(queue_ids: &[u64], roles: u8, blacklist_map: [i32; 2]) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint32(queue_ids.len() as u32);
    pkt.write_uint8(roles);
    pkt.write_int32(blacklist_map[0]);
    pkt.write_int32(blacklist_map[1]);
    for queue_id in queue_ids {
        pkt.write_uint64(*queue_id);
    }
    pkt.reset_read();
    pkt
}

fn battlemaster_join_arena_packet(team_size_index: u8, roles: u8) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint8(team_size_index);
    pkt.write_uint8(roles);
    pkt.reset_read();
    pkt
}

fn battlemaster_join_skirmish_packet(
    bg_type_id: u32,
    bracket_id: u32,
    as_group: u8,
    is_rated: u8,
) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint32(bg_type_id);
    pkt.write_uint32(bracket_id);
    pkt.write_uint8(as_group);
    pkt.write_uint8(is_rated);
    pkt.reset_read();
    pkt
}

fn battlefield_port_packet(
    requester_guid: ObjectGuid,
    slot: u32,
    ride_type: u32,
    time: i64,
    unknown925: bool,
    accepted_invite: bool,
) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_packed_guid(&requester_guid);
    pkt.write_uint32(slot);
    pkt.write_uint32(ride_type);
    pkt.write_int64(time);
    pkt.write_bit(unknown925);
    pkt.flush_bits();
    pkt.write_bit(accepted_invite);
    pkt.flush_bits();
    pkt.reset_read();
    pkt
}

fn battleground_queue_id_like_cpp(
    battlemaster_list_id: u16,
    queue_type: u8,
    rated: bool,
    team_size: u8,
) -> u64 {
    u64::from(battlemaster_list_id)
        | (u64::from(queue_type & 0x0F) << 16)
        | (u64::from(u8::from(rated)) << 20)
        | (u64::from(team_size & 0x3F) << 24)
        | 0x1F10_0000_0000_0000
}

fn battlemaster_entry_like_cpp(
    id: u32,
    instance_type: i8,
    flags: i8,
) -> wow_data::BattlemasterListEntry {
    wow_data::BattlemasterListEntry {
        id,
        instance_type,
        holiday_world_state: 0,
        flags,
    }
}

fn broadcast_info_with_command_tx(
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
        info: PlayerBroadcastInfo {
            combat_reach: 0.0,
            liquid_status: 0,
            active_loot_rolls: Vec::new(),
            in_combat: false,
            pass_on_group_loot: false,
            enchanting_skill: 0,
            transport: None,
            is_afk: false,
            is_dnd: false,
            in_vehicle: false,
            has_vehicle_kit_like_cpp: false,
            party_member_vehicle_seat: 0,
            zone_id: 0,
            spec_id: 0,
            unit_flags: 0,
            unit_state: 0,
            is_game_master: false,
            dungeon_difficulty_id: 1,
            pending_quest_sharing: None,
            known_spells: Vec::new(),
            active_quest_statuses: Default::default(),
            active_quest_objective_counts: Default::default(),
            rewarded_quests: Default::default(),
            completed_achievements: Default::default(),
            daily_quests_completed: Default::default(),
            df_quests: Default::default(),
            faction_template_id: 0,
            forced_reputation_ranks: Vec::new(),
            inventory_item_counts: Default::default(),
            party_member_phase_states: Default::default(),
            party_member_auras: Vec::new(),
            party_member_pet_stats: None,
            gray_level: 0,
            display_id: 49,
            visible_items: std::sync::Arc::new([(0, 0, 0); 19]),
            customizations: std::sync::Arc::default(),
        },
        realm_send_tx: send_tx.clone(),
        send_tx,
        command_tx,
        durable_creature_runtime_commands_like_cpp: Default::default(),
        client_visible_guids_like_cpp: Default::default(),
        advanced_combat_logging_enabled_like_cpp: Default::default(),
        visibility_refresh_pending_like_cpp: Default::default(),
    }
}

fn resurrect_response_packet(resurrecter: ObjectGuid, response: u32) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_packed_guid(&resurrecter);
    pkt.write_uint32(response);
    pkt
}

fn repop_request_packet(check_instance: bool) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bit(check_instance);
    pkt.flush_bits();
    pkt
}

fn port_graveyard_packet() -> WorldPacket {
    WorldPacket::new_empty()
}

fn reclaim_corpse_packet(corpse_guid: ObjectGuid) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bytes(&corpse_guid.to_raw_bytes());
    pkt
}

fn update_account_data_packet(
    player_guid: ObjectGuid,
    data_type: u8,
    time: i64,
    data: &str,
) -> WorldPacket {
    let compressed_data = compress_account_data_like_cpp(data).unwrap();
    let mut pkt = WorldPacket::new_empty();
    pkt.write_packed_guid(&player_guid);
    pkt.write_int64(time);
    pkt.write_uint32(data.len() as u32);
    pkt.write_bits(u32::from(data_type), 4);
    pkt.write_uint32(compressed_data.len() as u32);
    pkt.write_bytes(&compressed_data);
    pkt
}

fn request_account_data_packet(player_guid: ObjectGuid, data_type: u8) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_packed_guid(&player_guid);
    pkt.write_bits(u32::from(data_type), 4);
    pkt.flush_bits();
    pkt
}

fn activate_taxi_packet(
    vendor: ObjectGuid,
    node: u32,
    ground_mount_id: u32,
    flying_mount_id: u32,
) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_packed_guid(&vendor);
    pkt.write_uint32(node);
    pkt.write_uint32(ground_mount_id);
    pkt.write_uint32(flying_mount_id);
    pkt
}

fn add_canonical_flight_master_for_misc_test(
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

fn add_canonical_auctioneer_for_misc_test(
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

fn bug_report_packet(report_type: bool, diag_info: &str, text: &str) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bit(report_type);
    pkt.write_bits(diag_info.len() as u32, 12);
    pkt.write_bits(text.len() as u32, 10);
    pkt.flush_bits();
    pkt.write_string(diag_info);
    pkt.write_string(text);
    pkt.reset_read();
    pkt
}

fn submit_user_feedback_packet(is_suggestion: bool, note: &str) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_int32(571);
    pkt.write_float(1.25);
    pkt.write_float(2.5);
    pkt.write_float(3.75);
    pkt.write_float(4.0);
    pkt.write_int32(9);
    pkt.write_bits((note.len() + 1) as u32, 24);
    pkt.write_bit(is_suggestion);
    pkt.write_string(note);
    pkt.write_uint8(0);
    pkt.reset_read();
    pkt
}

fn support_ticket_submit_bug_packet(message: &str) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_int32(571);
    pkt.write_float(1.25);
    pkt.write_float(2.5);
    pkt.write_float(3.75);
    pkt.write_float(4.0);
    pkt.write_int32(9);
    pkt.write_bits(message.len() as u32, 10);
    pkt.write_string(message);
    pkt
}

fn support_ticket_submit_complaint_packet(note: &str) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    let target = ObjectGuid::create_player(1, 42);
    pkt.write_int32(571);
    pkt.write_float(1.25);
    pkt.write_float(2.5);
    pkt.write_float(3.75);
    pkt.write_float(4.0);
    pkt.write_int32(9);
    pkt.write_packed_guid(&target);
    pkt.write_int32(1);
    pkt.write_int32(2);
    pkt.write_int32(4);
    pkt.write_uint32(0); // ChatLog.Lines.Count
    pkt.write_bit(false); // ReportLineIndex.HasValue
    pkt.flush_bits();
    pkt.write_bits(note.len() as u32, 10);
    pkt.write_bit(false); // MailInfo
    pkt.write_bit(false); // CalendarInfo
    pkt.write_bit(false); // PetInfo
    pkt.write_bit(false); // GuildInfo
    pkt.write_bit(false); // LFGListSearchResult
    pkt.write_bit(false); // LFGListApplicant
    pkt.write_bit(false); // ClubMessage
    pkt.write_bit(false); // ClubFinderResult
    pkt.write_bit(false); // Unused910
    pkt.flush_bits();
    pkt.write_uint32(0); // HorusChatLog.Lines.Count
    pkt.write_string(note);
    pkt
}

fn support_ticket_submit_suggestion_packet(message: &str) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bits(message.len() as u32, 10);
    pkt.write_string(message);
    pkt
}

fn object_update_recovery_packet(guid: ObjectGuid) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_packed_guid(&guid);
    pkt.reset_read();
    pkt
}

fn stand_state_change_packet(state: u32) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint32(state);
    pkt.reset_read();
    pkt
}

fn can_duel_packet(target_guid: ObjectGuid, to_the_death: bool) -> WorldPacket {
    let mut packet = WorldPacket::new_empty();
    packet.write_bytes(&target_guid.to_raw_bytes());
    packet.write_bit(to_the_death);
    packet.flush_bits();
    packet.reset_read();
    packet
}

fn duel_response_packet(arbiter_guid: ObjectGuid, accepted: bool, forfeited: bool) -> WorldPacket {
    let mut packet = WorldPacket::new_empty();
    packet.write_bytes(&arbiter_guid.to_raw_bytes());
    packet.write_bit(accepted);
    packet.write_bit(forfeited);
    packet.flush_bits();
    packet.reset_read();
    packet
}

fn accept_trade_packet(state_index: u32) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint32(state_index);
    pkt.reset_read();
    pkt
}

fn clear_trade_item_packet(trade_slot: u8) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint8(trade_slot);
    pkt.reset_read();
    pkt
}

fn set_trade_item_packet(trade_slot: u8, pack_slot: u8, item_slot_in_pack: u8) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint8(trade_slot);
    pkt.write_uint8(pack_slot);
    pkt.write_uint8(item_slot_in_pack);
    pkt.reset_read();
    pkt
}

fn set_trade_gold_packet(coinage: u64) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint64(coinage);
    pkt.reset_read();
    pkt
}

fn set_trade_spell_packet(spell_id: u32, pack_slot: u8, item_slot_in_pack: u8) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint32(spell_id);
    pkt.write_uint8(pack_slot);
    pkt.write_uint8(item_slot_in_pack);
    pkt.reset_read();
    pkt
}

fn sign_petition_packet(petition_guid: ObjectGuid, choice: u8) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bytes(&petition_guid.to_raw_bytes());
    pkt.write_uint8(choice);
    pkt.reset_read();
    pkt
}

fn decline_petition_packet(petition_guid: ObjectGuid) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bytes(&petition_guid.to_raw_bytes());
    pkt.reset_read();
    pkt
}

fn query_petition_packet(petition_id: u32, item_guid: ObjectGuid) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint32(petition_id);
    pkt.write_bytes(&item_guid.to_raw_bytes());
    pkt.reset_read();
    pkt
}

fn trade_test_spell_info(spell_id: i32) -> SpellInfo {
    SpellInfo {
        spell_id,
        cast_time_ms: 0,
        cooldown_ms: 0,
        recovery_time_ms: 0,
        effect_type: 0,
        effect_base_points: 0,
        effect_bonus_coefficient: 0.0,
        aura_type: None,
        display_flags: 0,
        requires_spell_focus: 0,
        power_costs: Vec::new(),
        effects: Vec::new(),
    }
}

fn install_trade_test_spell(session: &mut crate::session::WorldSession, spell_id: i32) {
    let mut spell_store = SpellStore::new();
    spell_store.insert(spell_id, trade_test_spell_info(spell_id));
    session.set_spell_store(Arc::new(spell_store));
    session.set_known_spells_like_cpp(vec![spell_id]);
}

fn insert_trade_test_item(
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

fn save_cuf_profiles_packet(
    profiles: impl IntoIterator<Item = wow_packet::packets::misc::CufProfile>,
) -> WorldPacket {
    let profiles: Vec<_> = profiles.into_iter().collect();
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint16(ClientOpcodes::SaveCufProfiles as u16);
    pkt.write_uint32(profiles.len() as u32);
    for profile in profiles {
        pkt.write_bits(profile.profile_name.len() as u32, 7);
        for option in 0..wow_packet::packets::misc::CUF_BOOL_OPTIONS_COUNT_LIKE_CPP {
            pkt.write_bit(profile.bool_options & (1 << option) != 0);
        }
        pkt.write_uint16(profile.frame_height);
        pkt.write_uint16(profile.frame_width);
        pkt.write_uint8(profile.sort_by);
        pkt.write_uint8(profile.health_text);
        pkt.write_uint8(profile.top_point);
        pkt.write_uint8(profile.bottom_point);
        pkt.write_uint8(profile.left_point);
        pkt.write_uint16(profile.top_offset);
        pkt.write_uint16(profile.bottom_offset);
        pkt.write_uint16(profile.left_offset);
        pkt.write_string(&profile.profile_name);
    }
    WorldPacket::from_bytes(pkt.data())
}

fn cuf_profile(name: &str, frame_height: u16) -> wow_packet::packets::misc::CufProfile {
    wow_packet::packets::misc::CufProfile {
        profile_name: name.to_string(),
        frame_height,
        frame_width: 128,
        sort_by: 2,
        health_text: 3,
        top_point: 4,
        bottom_point: 5,
        left_point: 6,
        top_offset: 7,
        bottom_offset: 8,
        left_offset: 9,
        bool_options: (1 << 0) | (1 << 26),
    }
}

fn install_add_toy_item_templates(
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

fn shared_canonical_map_manager_for_misc_test() -> crate::session::SharedCanonicalMapManager {
    Arc::new(Mutex::new(wow_map::MapManager::default()))
}

fn add_canonical_test_player_on_map_for_misc_test(
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
    player.unit_mut().world_mut().object_mut().add_to_world();

    canonical
        .lock()
        .unwrap()
        .create_world_map(map_id, instance_id)
        .map_mut()
        .insert_map_object_record(wow_entities::MapObjectRecord::new_player(player).unwrap())
        .unwrap();
}

fn collection_item_set_favorite_packet(
    collection_type: u32,
    id: u32,
    favorite: bool,
) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint16(ClientOpcodes::CollectionItemSetFavorite as u16);
    pkt.write_uint32(collection_type);
    pkt.write_uint32(id);
    pkt.write_bit(favorite);
    pkt.flush_bits();
    pkt
}

fn battle_pet_clear_fanfare_packet(pet_guid: ObjectGuid) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint16(ClientOpcodes::BattlePetClearFanfare as u16);
    pkt.write_packed_guid(&pet_guid);
    pkt
}

fn battle_pet_delete_pet_packet(pet_guid: ObjectGuid) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint16(0xBADD);
    pkt.write_packed_guid(&pet_guid);
    pkt
}

fn cage_battle_pet_packet(pet_guid: ObjectGuid) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint16(0xBADD);
    pkt.write_packed_guid(&pet_guid);
    pkt
}

fn battle_pet_modify_name_packet(
    pet_guid: ObjectGuid,
    name: &str,
    declined_names: Option<[&str; 5]>,
) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint16(0xBADD);
    pkt.write_packed_guid(&pet_guid);
    pkt.write_bits(name.len() as u32, 7);
    pkt.write_bit(declined_names.is_some());
    if let Some(declined_names) = declined_names {
        for declined_name in declined_names {
            pkt.write_bits(declined_name.len() as u32, 7);
        }
        for declined_name in declined_names {
            pkt.write_string(declined_name);
        }
    }
    pkt.write_string(name);
    pkt
}

fn battle_pet_set_flags_packet(pet_guid: ObjectGuid, flags: u16, control_type: u8) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint16(ClientOpcodes::BattlePetSetFlags as u16);
    pkt.write_packed_guid(&pet_guid);
    pkt.write_uint16(flags);
    pkt.write_bits(u32::from(control_type), 2);
    pkt.flush_bits();
    pkt
}

fn battle_pet_set_battle_slot_packet(pet_guid: ObjectGuid, slot: u8) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint16(ClientOpcodes::BattlePetSetBattleSlot as u16);
    pkt.write_packed_guid(&pet_guid);
    pkt.write_uint8(slot);
    pkt
}

fn battle_pet_summon_packet(pet_guid: ObjectGuid) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint16(ClientOpcodes::BattlePetSummon as u16);
    pkt.write_packed_guid(&pet_guid);
    pkt
}

fn battle_pet_update_notify_packet(pet_guid: ObjectGuid) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint16(ClientOpcodes::BattlePetUpdateNotify as u16);
    pkt.write_packed_guid(&pet_guid);
    pkt
}

fn battle_pet_update_display_notify_packet() -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint16(ClientOpcodes::BattlePetUpdateDisplayNotify as u16);
    pkt
}

fn dismiss_critter_packet(critter_guid: ObjectGuid) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_guid(&critter_guid);
    pkt
}

fn query_battle_pet_name_packet(battle_pet_id: ObjectGuid, unit_guid: ObjectGuid) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint16(ClientOpcodes::QueryBattlePetName as u16);
    pkt.write_packed_guid(&battle_pet_id);
    pkt.write_packed_guid(&unit_guid);
    pkt
}

fn battle_pet_request_journal_lock_packet() -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint16(ClientOpcodes::BattlePetRequestJournalLock as u16);
    pkt
}

fn battle_pet_request_journal_packet() -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint16(ClientOpcodes::BattlePetRequestJournal as u16);
    pkt
}

fn accept_wargame_invite_packet(inviter_name: &str) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_string(inviter_name);
    pkt.write_uint8(0);
    pkt.reset_read();
    pkt
}
