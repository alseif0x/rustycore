//! Private behavior tests for [`super`].
//!
//! These scenarios stay under the loot owner so they can exercise its private
//! state and methods without widening the production API. Public `wow-loot`
//! contracts are covered by the integration target in `tests/loot_tests`.
#![cfg(test)]

use super::{
    GAMEOBJECT_TYPE_AREADAMAGE,
    GAMEOBJECT_TYPE_BINDER, GAMEOBJECT_TYPE_CHAIR, GAMEOBJECT_TYPE_DOOR,
    GAMEOBJECT_TYPE_GUILD_BANK, GAMEOBJECT_TYPE_QUESTGIVER, INVENTORY_SLOT_BAG_0,
    INVENTORY_SLOT_ITEM_START, ITEM_FLAGS_CU_FOLLOW_LOOT_RULES_LIKE_CPP,
    ItemTemplateAddonLootMetadataLikeCpp, LOCK_KEY_SKILL_LIKE_CPP, LOCK_KEY_SPELL_LIKE_CPP,
    LOOT_METHOD_ROUND_ROBIN_LIKE_CPP, LOOT_MODE_DEFAULT_LIKE_CPP, LOOT_MODE_JUNK_FISH_LIKE_CPP,
    LOOT_SLOT_TYPE_ALLOW_LOOT_LIKE_CPP, LOOT_SLOT_TYPE_ROLL_ONGOING_LIKE_CPP,
    LootItemClaimCommitContextLikeCpp,
    ROLL_ALL_TYPE_NO_DISENCHANT_LIKE_CPP, ROLL_FLAG_TYPE_NEED_LIKE_CPP, ROLL_VOTE_GREED_LIKE_CPP,
    ROLL_VOTE_NEED_LIKE_CPP, ROLL_VOTE_NOT_EMITTED_YET_LIKE_CPP, ROLL_VOTE_NOT_VALID_LIKE_CPP,
    ROLL_VOTE_PASS_LIKE_CPP, RepresentedLootPlayerContext, SPELL_EFFECT_OPEN_LOCK_LIKE_CPP,
    SyncChestGameobjectStateAndRefreshLikeCppCommand,
    SyncGatheringNodeGameobjectStateAndRefreshLikeCppCommand,
    SyncGooberGameobjectStateAndRefreshLikeCppCommand,
    creature_loot_is_allowed_to_player_like_cpp,
    loot_is_looted_like_cpp,
    loot_item_context, loot_type_for_client_like_cpp,
    mark_loot_allowed_for_player_like_cpp,
    mark_loot_item_looted_for_player_like_cpp,
    represented_gameobject_display_box_contains_like_cpp,
    represented_gameobject_interaction_distance_like_cpp, represented_loot_object_guid_like_cpp,
    represented_loot_response_items_like_cpp,
};
use crate::player::inventory_persistence_test_fixture::PlayerInventoryPersistencePortFixtureLikeCpp;
use crate::session::directory::{
    PlayerDirectoryIdentityLikeCpp, PlayerDirectoryPlacementLikeCpp, PlayerRegistry,
    PlayerSessionRegistrationLikeCpp,
};
use crate::session::mailbox::{
    ApplyLootMoneyLikeCppCommand, KickLikeCppCommand, LootRollCommandIdentityLikeCpp,
    LootRollVoteCommand, MasterLootGiveResult,
    SendVisibleObjectValuesUpdateCommand, SessionCommand,
};
use crate::session::{
    AuraApplication, InventoryItem, SPELL_AURA_INTERRUPT_FLAG_LOOTING_LIKE_CPP, SpellCastState,
    WorldSession,
};
use crate::session::{
    LootMoneyDeliveryAddressLikeCpp,
    LootMoneyViewerFanoutLikeCpp, RepresentedGameObjectSpellCaster, RepresentedGameObjectUseEffect,
    RepresentedLootRollCriteriaEvent, SessionState, loot_money_durable_outcome_like_cpp,
};
use crate::session_policy::LootDropRatesLikeCpp;
use rand::{Rng, SeedableRng, rngs::StdRng};
use std::time::{Duration, Instant};
use std::{
    collections::{HashMap, HashSet},
    sync::{
        Arc, Barrier, Mutex, RwLock,
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
    },
};
use wow_ai::CreatureAI;
use wow_conditions::QUEST_STATUS_REWARDED_LIKE_CPP;
use wow_constants::{
    InventoryResult, InventoryType, ItemBondingType, ItemClass, ItemContext, ItemFieldFlags,
    ItemFlags2, ItemQuality, ServerOpcodes, TypeId, TypeMask, UnitDynFlags,
};
use wow_core::{ObjectGuid, ObjectGuidGenerator, Position, guid::HighGuid};
use wow_data::quest::{QUEST_REWARD_REPUTATIONS_COUNT, QuestObjective, QuestStore, QuestTemplate};
use wow_data::{
    AreaTableEntry, AreaTableStore, ChrSpecializationEntry, ChrSpecializationStore,
    ItemDisenchantLootEntry, ItemDisenchantLootStore, ItemRandomEnchantmentTemplateEntry,
    ItemRandomEnchantmentTemplateStore, ItemRandomPropertiesEntry, ItemRandomPropertiesStore,
    ItemRandomPropertyTemplateEntry, ItemRandomSuffixEntry, ItemRandomSuffixStore, ItemRecord,
    ItemSparseTemplateEntry, ItemStatsStore, ItemStore, RandPropPointsEntry, RandPropPointsStore,
    SpellEffectInfo, SpellInfo, SpellMiscEntry, SpellMiscStore, SpellRangeEntry, SpellRangeStore,
    SpellStore,
};
use wow_entities::{
    AccessorObjectKind, Creature, CreatureOwnedLoot, GAMEOBJECT_TYPE_CHEST,
    GAMEOBJECT_TYPE_FISHING_HOLE, GAMEOBJECT_TYPE_FISHING_NODE,
    GAMEOBJECT_TYPE_GATHERING_NODE, GAMEOBJECT_TYPE_GOOBER, GO_DYNFLAG_LO_NO_INTERACT, GameObject,
    GameObjectLootSource, GameObjectOwnedLoot, GatheringNodeUseSource, GoState, Item,
    ItemCreateInfo, LootEntry, LootEntryFlags, LootState, MAX_ITEM_SPELLS, MAX_MONEY_AMOUNT,
    ObjectChangedFields, Player, WorldObject, CreatureLoot,
};
use wow_loot::{
    GeneratedLootItem, LOOT_SLOT_TYPE_OWNER_LIKE_CPP, LootClaimPayload, LootConditionRowLikeCpp,
    LootStore, LootStoreItem, LootStoreItemContext, LootStoreKind, LootStores, LootTemplateRow,
    OwnedLootAuthority, OwnedLootAuthorityLifecycle,
};
use wow_packet::packets::loot::{
    LOOT_ERROR_MASTER_OTHER_LIKE_CPP, LOOT_ERROR_MASTER_UNIQUE_ITEM_LIKE_CPP,
    LOOT_ERROR_NO_LOOT_LIKE_CPP, LOOT_ERROR_PLAYER_NOT_FOUND_LIKE_CPP, LOOT_ERROR_TOO_FAR_LIKE_CPP,
    LOOT_RESPONSE_DEFAULT_FAILURE_REASON_LIKE_CPP, LOOT_RESPONSE_DEFAULT_THRESHOLD_LIKE_CPP,
    LOOT_TYPE_CHEST_LIKE_CPP, LOOT_TYPE_CORPSE_LIKE_CPP, LOOT_TYPE_CORPSE_PERSONAL_LIKE_CPP,
    LOOT_TYPE_DISENCHANTING_LIKE_CPP, LOOT_TYPE_FISHING_JUNK_LIKE_CPP, LOOT_TYPE_FISHING_LIKE_CPP,
    LOOT_TYPE_FISHINGHOLE_LIKE_CPP, LOOT_TYPE_GATHERING_NODE_LIKE_CPP, LOOT_TYPE_INSIGNIA_LIKE_CPP,
    LOOT_TYPE_ITEM_LIKE_CPP, LOOT_TYPE_MILLING_LIKE_CPP, LOOT_TYPE_NONE_LIKE_CPP,
    LOOT_TYPE_PICKPOCKETING_LIKE_CPP, LOOT_TYPE_PROSPECTING_LIKE_CPP, LOOT_TYPE_SKINNING_LIKE_CPP,
    LootResponse, LootRoll, MasterLootItem, SetLootSpecialization,
};
use wow_packet::packets::update::{
    CreatureCreateData, ObjectDataValuesUpdate, UnitDataValuesDeltaUpdate,
};
use wow_packet::{ServerPacket, WorldPacket};
use wow_persistence::PersistenceOutcomeLikeCpp;
#[path = "tests/canonical_world.rs"]
mod canonical_world;
use canonical_world::{
    attach_canonical_creature, attach_canonical_gameobject, attach_canonical_map_object,
    attach_loot_guid_allocator_for_owner, canonical_creature_snapshot,
    canonical_gameobject_snapshot, canonical_world_object, make_canonical_creature_for_session,
    make_canonical_gameobject_for_session,
};
#[path = "tests/loot_authority.rs"]
mod loot_authority;
use loot_authority::{
    authoritative_test_loot_like_cpp, authoritative_test_loot_response_like_cpp,
    insert_allowed_coin_loot_like_cpp, install_cached_test_creature_loot_authority_like_cpp,
    represented_disenchant_test_outputs_like_cpp,
    two_sessions_with_authoritative_creature_loot_like_cpp,
};
#[path = "tests/item_templates.rs"]
mod item_templates;
use item_templates::{
    install_disenchantable_test_item_template, install_limited_test_item_template,
    install_limited_test_item_template_with_flags2,
};
#[path = "tests/group_lifecycle.rs"]
mod group_lifecycle;
use group_lifecycle::{
    generation_guarded_group_loot_like_cpp, install_group_loot_group, install_master_loot_group,
    open_generation_guarded_group_roll_like_cpp, replace_generation_guarded_group_loot_like_cpp,
};
#[path = "tests/overworld_personal_loot.rs"]
mod overworld_personal_loot;
use overworld_personal_loot::{
    assert_overworld_personal_loot_claims_are_independent_like_cpp,
    assert_overworld_personal_loot_generation_like_cpp,
    overworld_personal_loot_test_fixture_like_cpp,
};

fn make_session_with_send_capacity(capacity: usize) -> (WorldSession, flume::Receiver<Vec<u8>>) {
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
    session.set_loot_money_persistence_test_result_like_cpp(true);
    (session, send_rx)
}

fn make_session_with_send() -> (WorldSession, flume::Receiver<Vec<u8>>) {
    make_session_with_send_capacity(1)
}

fn make_session() -> WorldSession {
    make_session_with_send().0
}

fn loot_item_packet(object: ObjectGuid, loot_list_id: u8) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint32(1);
    pkt.write_packed_guid(&object);
    pkt.write_uint8(loot_list_id);
    pkt.write_bit(false);
    pkt.flush_bits();
    pkt.reset_read();
    pkt
}

fn loot_unit_packet(object: ObjectGuid) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_packed_guid(&object);
    pkt.reset_read();
    pkt
}

fn represented_loot_entry(loot_list_id: u8, item_id: u32, player_guid: ObjectGuid) -> LootEntry {
    LootEntry {
        loot_list_id,
        item_id,
        quantity: 1,
        random_properties_id: 0,
        random_properties_seed: 0,
        item_context: 0,
        flags: LootEntryFlags {
            follow_loot_rules: true,
            freeforall: false,
            blocked: false,
            counted: false,
            under_threshold: false,
            needs_quest: false,
        },
        allowed_looters: vec![player_guid],
        roll_winner: ObjectGuid::EMPTY,
        ffa_looted_by: Vec::new(),
        taken: false,
    }
}


fn loot_release_packet(object: ObjectGuid) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_packed_guid(&object);
    pkt.reset_read();
    pkt
}

fn loot_money_packet() -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bit(false);
    pkt.flush_bits();
    pkt.reset_read();
    pkt
}

fn recv_packet_with_opcode(
    rx: &flume::Receiver<Vec<u8>>,
    opcode: wow_constants::ServerOpcodes,
) -> WorldPacket {
    for _ in 0..8 {
        let sent = rx.try_recv().unwrap();
        let mut packet = WorldPacket::from_bytes(&sent);
        if packet.read_uint16().unwrap() == opcode as u16 {
            return packet;
        }
    }
    panic!("expected packet opcode {:?}", opcode);
}

fn drain_server_opcodes_like_cpp(rx: &flume::Receiver<Vec<u8>>) -> Vec<u16> {
    let mut opcodes = Vec::new();
    while let Ok(bytes) = rx.try_recv() {
        let mut packet = WorldPacket::from_bytes(&bytes);
        opcodes.push(packet.read_uint16().unwrap());
    }
    opcodes
}

fn test_creature(guid: ObjectGuid, is_alive: bool) -> CreatureAI {
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

fn register_test_creature_like_cpp(session: &mut WorldSession, creature: CreatureAI) {
    if session.map_manager.is_none() {
        session.set_map_manager(Arc::new(RwLock::new(crate::map_manager::MapManager::new())));
    }

    let create_data = CreatureCreateData {
        guid: creature.guid,
        entry: creature.entry,
        display_id: creature.display_id,
        native_display_id: creature.display_id,
        display_scale: 1.0,
        native_x_display_scale: 1.0,
        bounding_radius: 0.389,
        combat_reach: 1.5,
        health: i64::from(creature.hp.max(1)),
        max_health: i64::from(creature.max_hp.max(1)),
        level: creature.level,
        faction_template: creature.faction as i32,
        npc_flags: u64::from(creature.npc_flags),
        unit_flags: creature.unit_flags,
        unit_flags2: 0,
        unit_flags3: 0,
        aura_state: crate::map_manager::WorldCreature::health_aura_state_like_cpp(
            u64::from(creature.hp.max(1)),
            u64::from(creature.max_hp.max(1)),
            true,
        ),
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
    };
    let guid = creature.guid;
    let is_alive = creature.is_alive;
    session.register_world_creature(
        session.player_map_id_like_cpp(),
        creature.current_pos,
        create_data,
        creature.min_dmg,
        creature.max_dmg,
        creature.aggro_radius,
        creature.loot_id,
        0,
        creature.gold_min,
        creature.gold_max,
        creature.boss_id,
        creature.dungeon_encounter_id,
        0,
        0,
        0,
        -1,
    );
    if !is_alive {
        let _ = session.mutate_world_creature(guid, |world_creature| {
            world_creature.creature.mark_ai_dead(0);
        });
    }
}

fn install_quest_bound_loot_objective_like_cpp(
    session: &mut WorldSession,
    quest_id: u32,
    item_id: u32,
    current_count: i32,
    required_count: i32,
) {
    let mut quest = test_quest_template(quest_id);
    quest.objectives.push(QuestObjective {
        id: quest_id * 10,
        quest_id,
        obj_type: 1,
        order: 0,
        storage_index: 0,
        object_id: item_id as i32,
        amount: required_count,
        flags: 0,
        flags2: 1,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    session.quest_test_fixture_like_cpp.player_quests.insert(
        quest_id,
        crate::handlers::quest::PlayerQuestStatus {
            quest_id,
            status: crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: vec![current_count],
            slot: 0,
        },
    );
}

fn tap_test_creature_like_cpp(
    session: &mut WorldSession,
    creature_guid: ObjectGuid,
    player_guid: ObjectGuid,
) {
    let _ = session.mutate_world_creature(creature_guid, |world_creature| {
        world_creature
            .creature
            .set_tapped_by_player(player_guid, &[]);
    });
}

fn loot_response_failure_reason(sent: &[u8]) -> u8 {
    loot_response_failure_reason_and_threshold(sent).0
}

fn loot_response_threshold(sent: &[u8]) -> u8 {
    loot_response_failure_reason_and_threshold(sent).1
}

fn loot_response_failure_reason_and_threshold(sent: &[u8]) -> (u8, u8) {
    let mut pkt = WorldPacket::from_bytes(&sent[2..]);
    let _owner = pkt.read_packed_guid().unwrap();
    let _loot_obj = pkt.read_packed_guid().unwrap();
    let failure_reason = pkt.read_uint8().unwrap();
    let _acquire_reason = pkt.read_uint8().unwrap();
    let _loot_method = pkt.read_uint8().unwrap();
    let threshold = pkt.read_uint8().unwrap();
    (failure_reason, threshold)
}

fn test_creature_guid(counter: i64) -> ObjectGuid {
    ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, counter)
}

fn test_corpse_guid(counter: i64) -> ObjectGuid {
    ObjectGuid::create_world_object(HighGuid::Corpse, 0, 1, 0, 0, 1, counter)
}

fn broadcast_info(
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

fn loot_condition(
    condition_type_or_reference: i32,
    value1: u32,
    value2: u32,
    value3: u32,
) -> LootConditionRowLikeCpp {
    LootConditionRowLikeCpp {
        else_group: 0,
        condition_type_or_reference,
        condition_target: 0,
        value1,
        value2,
        value3,
        string_value1: String::new(),
        negative: false,
        script_name: String::new(),
    }
}

fn test_quest_template(id: u32) -> QuestTemplate {
    QuestTemplate {
        id,
        quest_type: 0,
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
        reward_display_spell: [0; 3],
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
        reward_items: [0; 4],
        reward_amounts: [0; 4],
        reward_currencies: [0; 4],
        reward_currency_amounts: [0; 4],
        item_drop: [0; 4],
        item_drop_quantity: [0; 4],
        log_title: String::new(),
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
        reward_choice_items: [(0, 0); 6],
        reward_choice_item_types: [0; 6],
    }
}

fn install_active_spell_cast(session: &mut WorldSession, player_guid: ObjectGuid) {
    session.set_active_spell_cast_like_cpp(Some(SpellCastState {
        spell_id: 133,
        target_guid: player_guid,
        target_data: wow_entities::SpellCastTargetsLikeCpp {
            flags: 0x2,
            unit: player_guid,
            ..Default::default()
        },
        cast_id: ObjectGuid::create_world_object(HighGuid::Cast, 0, 1, 0, 0, 1, 7),
        cast_start_time: std::time::Instant::now(),
        cast_time_ms: 30_000,
        spell_visual: wow_entities::SpellCastVisualLikeCpp {
            spell_visual_id: 1,
            script_visual_id: 0,
        },
        metadata: crate::session::SpellCastMetadata::default(),
    }));
}

fn install_visible_aura_with_interrupt_flags(
    session: &mut WorldSession,
    slot: u8,
    spell_id: i32,
    caster_guid: ObjectGuid,
    aura_interrupt_flags: u32,
) {
    session.visible_auras.insert(
        slot,
        AuraApplication {
            spell_id,
            difficulty_id: 0,
            caster_guid,
            slot,
            duration_total: 30_000,
            duration_remaining: 30_000,
            stack_count: 1,
            aura_flags: 0x0000_0001,
            effect_mask: 0x0000_0001,
            aura_interrupt_flags,
            aura_interrupt_flags2: 0,
            represented_effect: None,
            represented_amount: 0,
            represented_effect_amounts: Vec::new(),
            represented_misc_value: None,
            represented_multiplier: 1.0,
            applied_at: std::time::Instant::now(),
        },
    );
}

fn test_gameobject_guid(counter: i64) -> ObjectGuid {
    ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 0, 0, 1, counter)
}

async fn open_test_ae_pair_like_cpp(
    session: &mut WorldSession,
    player_guid: ObjectGuid,
    primary_guid: ObjectGuid,
    secondary_guid: ObjectGuid,
) -> OwnedLootAuthority {
    session.set_player_guid(Some(player_guid));
    session.set_enable_ae_loot_like_cpp(true);
    session.set_player_position_like_cpp(Position::ZERO);
    register_test_creature_like_cpp(session, test_creature(primary_guid, false));
    register_test_creature_like_cpp(session, test_creature(secondary_guid, false));
    insert_allowed_coin_loot_like_cpp(session, primary_guid, player_guid, 7);
    insert_allowed_coin_loot_like_cpp(session, secondary_guid, player_guid, 7);

    session
        .handle_loot_unit(loot_unit_packet(primary_guid))
        .await;

    assert!(session.is_active_loot_guid(primary_guid));
    assert!(session.loot_views.contains_owner(&secondary_guid));
    let authority = session
        .represented_owned_loot_authority_like_cpp(secondary_guid)
        .expect("the secondary AE owner must expose its object-owned authority");
    assert!(
        authority
            .snapshot_for_player_like_cpp(player_guid)
            .unwrap()
            .loot
            .players_looting
            .contains(&player_guid)
    );
    authority
}

#[path = "tests/creature_1.rs"]
mod creature_1;
#[path = "tests/creature_2.rs"]
mod creature_2;
#[path = "tests/gameobject_1.rs"]
mod gameobject_1;
#[path = "tests/gameobject_2.rs"]
mod gameobject_2;
#[path = "tests/gameobject_3.rs"]
mod gameobject_3;
#[path = "tests/gameobject_4.rs"]
mod gameobject_4;
#[path = "tests/item_2.rs"]
mod item_2;
#[path = "tests/item_3.rs"]
mod item_3;
#[path = "tests/item_4.rs"]
mod item_4;
#[path = "tests/login.rs"]
mod login;
#[path = "tests/loot_1.rs"]
mod loot_1;
#[path = "tests/loot_2.rs"]
mod loot_2;
#[path = "tests/loot_3.rs"]
mod loot_3;
#[path = "tests/loot_4.rs"]
mod loot_4;
#[path = "tests/loot_6.rs"]
mod loot_6;
#[path = "tests/misc_1.rs"]
mod misc_1;
#[path = "tests/misc_2.rs"]
mod misc_2;
#[path = "tests/quest.rs"]
mod quest;
#[path = "tests/spell.rs"]
mod spell;
