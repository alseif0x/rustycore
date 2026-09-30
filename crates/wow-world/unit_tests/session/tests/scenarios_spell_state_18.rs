//! Session scenarios exercising the represented spell state responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[tokio::test]
async fn spell_cpp_null_primary_effect_is_represented_noop_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 67);
    session.set_player_guid(Some(player_guid));
    session.set_player_health_like_cpp(88, 100);

    let mut spell_store = wow_data::SpellStore::new();
    let primary_noop_spells = [
        (
            856_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_CREATE_HOUSE,
        ),
        (
            750_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_BIND_SIGHT,
        ),
        (
            756_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_CREATE_HOUSE,
        ),
        (
            757_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_THREAT_ALL,
        ),
        (
            758_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_SURVEY,
        ),
        (
            759_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_SHOW_CORPSE_LOOT,
        ),
        (
            760_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_112,
        ),
        (
            761_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_CALL_PET,
        ),
        (
            762_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_OBLITERATE_ITEM,
        ),
        (
            763_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_ALLOW_CONTROL_PET,
        ),
        (
            764_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_175,
        ),
        (
            765_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_DESPAWN_PERSISTENT_AREA_AURA,
        ),
        (
            766_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_UPDATE_AREATRIGGER,
        ),
        (
            767_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_DESPAWN_AREATRIGGER,
        ),
        (
            768_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_183,
        ),
        (
            769_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_REPUTATION_2,
        ),
        (
            770_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_185,
        ),
        (
            771_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_186,
        ),
        (
            772_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_RANDOMIZE_ARCHAEOLOGY_DIGSITES,
        ),
        (
            773_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_SUMMON_STABLED_PET_AS_GUARDIAN,
        ),
        (
            774_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_LOOT,
        ),
        (
            775_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_CHANGE_PARTY_MEMBERS,
        ),
        (
            776_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_TELEPORT_TO_DIGSITE,
        ),
        (
            777_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_START_PET_BATTLE,
        ),
        (
            778_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_DESPAWN_SUMMON,
        ),
        (
            779_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_ALTER_ITEM,
        ),
        (
            780_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_LAUNCH_QUEST_TASK,
        ),
        (
            781_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_SET_REPUTATION,
        ),
        (
            782_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_LEARN_GARRISON_BUILDING,
        ),
        (
            783_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_LEARN_GARRISON_SPECIALIZATION,
        ),
        (
            784_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_CREATE_GARRISON,
        ),
        (
            785_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_UPGRADE_CHARACTER_SPELLS,
        ),
        (
            786_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_CREATE_SHIPMENT,
        ),
        (
            787_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_UPGRADE_GARRISON,
        ),
        (
            788_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_218,
        ),
        (
            789_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_ADD_GARRISON_FOLLOWER,
        ),
        (
            790_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_ADD_GARRISON_MISSION,
        ),
        (
            791_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_CHANGE_ITEM_BONUSES,
        ),
        (
            792_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_ACTIVATE_GARRISON_BUILDING,
        ),
        (
            793_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_TRIGGER_ACTION_SET,
        ),
        (
            794_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_TELEPORT_TO_LFG_DUNGEON,
        ),
        (
            795_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_228,
        ),
        (
            796_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_SET_FOLLOWER_QUALITY,
        ),
        (
            797_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_230,
        ),
        (
            798_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_INCREASE_FOLLOWER_EXPERIENCE,
        ),
        (
            799_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_REMOVE_PHASE,
        ),
        (
            800_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_RANDOMIZE_FOLLOWER_ABILITIES,
        ),
        (
            801_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_234,
        ),
        (
            802_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_INCREASE_SKILL,
        ),
        (
            803_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_END_GARRISON_BUILDING_CONSTRUCTION,
        ),
        (
            804_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_GIVE_ARTIFACT_POWER,
        ),
        (
            805_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_GIVE_ARTIFACT_POWER_NO_BONUS,
        ),
        (
            849_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_LEARN_FOLLOWER_ABILITY,
        ),
        (
            850_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_FINISH_GARRISON_MISSION,
        ),
        (
            851_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_ADD_GARRISON_MISSION_SET,
        ),
        (
            852_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_FINISH_SHIPMENT,
        ),
        (
            853_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_FORCE_EQUIP_ITEM,
        ),
        (
            854_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_TAKE_SCREENSHOT,
        ),
        (
            855_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_SET_GARRISON_CACHE_SIZE,
        ),
        (
            806_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_MODIFY_KEYSTONE,
        ),
        (
            807_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_RESPEC_AZERITE_EMPOWERED_ITEM,
        ),
        (
            808_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_SUMMON_STABLED_PET,
        ),
        (
            809_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_SCRAP_ITEM,
        ),
        (
            810_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_REPAIR_ITEM,
        ),
        (
            811_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_REMOVE_GEM,
        ),
        (
            812_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_LEARN_AZERITE_ESSENCE_POWER,
        ),
        (
            813_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_SET_ITEM_BONUS_LIST_GROUP_ENTRY,
        ),
        (
            814_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_MOUNT_EQUIPMENT,
        ),
        (
            815_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_INCREASE_ITEM_BONUS_LIST_GROUP_STEP,
        ),
        (
            816_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_270,
        ),
        (
            817_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_SET_COVENANT,
        ),
        (
            818_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_CRAFT_RUNEFORGE_LEGENDARY,
        ),
        (
            819_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_SET_CHROMIE_TIME,
        ),
        (
            820_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_278,
        ),
        (
            821_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_LEARN_GARR_TALENT,
        ),
        (
            822_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_280,
        ),
        (
            823_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_LEARN_SOULBIND_CONDUIT,
        ),
        (
            824_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_CONVERT_ITEMS_TO_CURRENCY,
        ),
        (
            825_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_COMPLETE_CAMPAIGN,
        ),
        (
            826_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_MODIFY_KEYSTONE_2,
        ),
        (
            827_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_SET_GARRISON_FOLLOWER_LEVEL,
        ),
        (
            828_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_CRAFT_ITEM,
        ),
        (
            829_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_CRAFT_LOOT,
        ),
        (
            830_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_SALVAGE_ITEM,
        ),
        (
            831_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_CRAFT_SALVAGE_ITEM,
        ),
        (
            832_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_RECRAFT_ITEM,
        ),
        (
            833_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_CANCEL_ALL_PRIVATE_CONVERSATIONS,
        ),
        (
            834_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_299,
        ),
        (
            835_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_300,
        ),
        (
            836_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_CRAFT_ENCHANT,
        ),
        (
            837_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_GATHERING,
        ),
        (
            838_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_305,
        ),
        (
            839_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_UPDATE_INTERACTIONS,
        ),
        (
            840_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_307,
        ),
        (
            841_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_CANCEL_PRELOAD_WORLD,
        ),
        (
            842_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_PRELOAD_WORLD,
        ),
        (
            843_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_310,
        ),
        (
            844_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_ENSURE_WORLD_LOADED,
        ),
        (
            845_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_312,
        ),
        (
            846_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_CHANGE_ITEM_BONUSES_2,
        ),
        (
            847_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_ADD_SOCKET_BONUS,
        ),
        (
            848_i32,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_LEARN_TRANSMOG_APPEARANCE_FROM_ITEM_MOD_APPEARANCE_GROUP,
        ),
    ];
    for (spell_id, effect_type) in primary_noop_spells {
        spell_store.insert(
            spell_id,
            wow_data::SpellInfo {
                spell_id,
                cast_time_ms: 0,
                cooldown_ms: 0,
                recovery_time_ms: 0,
                effect_type,
                effect_base_points: 99,
                effect_bonus_coefficient: 0.0,
                aura_type: None,
                display_flags: 0,
                requires_spell_focus: 0,
                power_costs: Vec::new(),
                effects: Vec::new(),
            },
        );
    }
    session.set_spell_store(Arc::new(spell_store));

    let mut observed_opcodes = Vec::new();
    for (spell_id, _) in primary_noop_spells {
        session
            .execute_spell(spell_id, player_guid)
            .await
            .expect("represented C++ EffectNULL primary field should no-op");
        observed_opcodes.extend(drain_server_opcodes(&send_rx));
    }

    assert_eq!(session.player_health_like_cpp(), 88);
    assert_eq!(
        observed_opcodes,
        primary_noop_spells
            .iter()
            .flat_map(|_| [ServerOpcodes::SpellGo, ServerOpcodes::CooldownEvent])
            .collect::<Vec<_>>()
    );
}
#[tokio::test]
async fn spell_dual_wield_effect_row_sets_canonical_player_flag_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 751_i32;
    let player_guid = ObjectGuid::create_player(1, 68);
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 0,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "DualWield".to_string(),
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        1,
        1,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    assert_eq!(
        session
            .mutate_canonical_player_like_cpp(|player| { player.unit().can_dual_wield_like_cpp() }),
        Some(false)
    );

    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
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
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_DUAL_WIELD,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, player_guid)
        .await
        .expect("represented dual-wield spell row should execute");

    assert_eq!(
        session
            .mutate_canonical_player_like_cpp(|player| { player.unit().can_dual_wield_like_cpp() }),
        Some(true)
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::SpellGo, ServerOpcodes::CooldownEvent]
    );
}
#[tokio::test]
async fn spell_titan_grip_effect_row_sets_canonical_player_flag_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 853_i32;
    let penalty_spell_id = 49152_i32;
    let player_guid = ObjectGuid::create_player(1, 153);
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 0,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "TitanGrip".to_string(),
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        1,
        1,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    assert_eq!(
        session.mutate_canonical_player_like_cpp(|player| {
            (
                player.can_titan_grip(),
                player.titan_grip_penalty_spell_id(),
            )
        }),
        Some((false, 0))
    );

    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
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
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_TITAN_GRIP,
                effect_misc_value_1: penalty_spell_id,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, player_guid)
        .await
        .expect("represented titan-grip spell row should execute");

    assert_eq!(
        session.mutate_canonical_player_like_cpp(|player| {
            (
                player.can_titan_grip(),
                player.titan_grip_penalty_spell_id(),
            )
        }),
        Some((true, penalty_spell_id as u32)),
        "C++ Spell::EffectTitanGrip sets m_canTitanGrip and stores MiscValue as the penalty spell id"
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::SpellGo, ServerOpcodes::CooldownEvent]
    );
}
#[tokio::test]
async fn spell_dual_wield_effect_row_ignores_non_current_player_target_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 752_i32;
    let player_guid = ObjectGuid::create_player(1, 69);
    let other_guid = ObjectGuid::create_player(1, 70);
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 0,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "NoDualWield".to_string(),
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        1,
        1,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();

    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
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
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_DUAL_WIELD,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, other_guid)
        .await
        .expect("represented dual-wield non-current target should no-op");

    assert_eq!(
        session
            .mutate_canonical_player_like_cpp(|player| { player.unit().can_dual_wield_like_cpp() }),
        Some(false)
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::SpellGo, ServerOpcodes::CooldownEvent]
    );
}
#[tokio::test]
async fn spell_parry_effect_row_sets_canonical_caster_flag_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 753_i32;
    let player_guid = ObjectGuid::create_player(1, 71);
    let other_guid = ObjectGuid::create_player(1, 72);
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 0,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "ParryCaster".to_string(),
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        1,
        1,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    assert_eq!(
        session.mutate_canonical_player_like_cpp(|player| { player.unit().can_parry_like_cpp() }),
        Some(false)
    );

    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
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
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_PARRY,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, other_guid)
        .await
        .expect("represented parry spell row should set caster flag");

    assert_eq!(
        session.mutate_canonical_player_like_cpp(|player| { player.unit().can_parry_like_cpp() }),
        Some(true)
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::SpellGo, ServerOpcodes::CooldownEvent]
    );
}
#[tokio::test]
async fn spell_block_effect_row_sets_canonical_caster_flag_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 754_i32;
    let player_guid = ObjectGuid::create_player(1, 73);
    let other_guid = ObjectGuid::create_player(1, 74);
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 0,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "BlockCaster".to_string(),
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        1,
        1,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    assert_eq!(
        session.mutate_canonical_player_like_cpp(|player| { player.unit().can_block_like_cpp() }),
        Some(false)
    );

    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
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
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_BLOCK,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, other_guid)
        .await
        .expect("represented block spell row should set caster flag");

    assert_eq!(
        session.mutate_canonical_player_like_cpp(|player| { player.unit().can_block_like_cpp() }),
        Some(true)
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::SpellGo, ServerOpcodes::CooldownEvent]
    );
}
