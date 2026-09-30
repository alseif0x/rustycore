use super::{
    Creature, HighGuid, MapObjectRecord, ObjectGuid, PathBuf, Player,
    PlayerDirectoryIdentityLikeCpp, PlayerDirectoryPlacementLikeCpp, PlayerRegistry,
    PlayerSessionRegistrationLikeCpp, Position, SessionCommand, env, fs,
};

pub(super) fn legacy_runtime_world_map_store_like_cpp() -> wow_data::MapStore {
    wow_data::MapStore::from_entries([wow_data::MapEntry {
        id: 0,
        instance_type: wow_data::map::MAP_COMMON,
        expansion_id: 0,
        parent_map_id: -1,
        cosmetic_parent_map_id: -1,
        flags1: 0,
        flags2: 0,
    }])
}

pub(super) fn player_registration_fixture_like_cpp(
    send_tx: flume::Sender<Vec<u8>>,
    command_tx: flume::Sender<SessionCommand>,
    player_name: &str,
) -> PlayerSessionRegistrationLikeCpp {
    PlayerSessionRegistrationLikeCpp {
        identity: PlayerDirectoryIdentityLikeCpp {
            player_name: player_name.to_string(),
            account_id: 1,
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
            position: wow_core::Position::ZERO,
            is_in_world: true,
            level: 1,
            is_alive: true,
        },
        active_loot_rolls: Vec::new(),
        realm_send_tx: send_tx.clone(),
        send_tx,
        command_tx,
        session_phase_tx: wow_world::session::directory::detached_session_phase_rail_like_cpp(),
        durable_creature_runtime_commands_like_cpp: Default::default(),
        client_visible_guids_like_cpp: Default::default(),
        client_visible_transports_like_cpp: Default::default(),
        advanced_combat_logging_enabled_like_cpp: Default::default(),
        visibility_refresh_pending_like_cpp: Default::default(),
    }
}

pub(super) fn drain_durable_creature_runtime_commands_like_cpp(
    registry: &PlayerRegistry,
    player_guid: ObjectGuid,
) -> Vec<SessionCommand> {
    let durable = registry
        .fixture_durable_creature_runtime_commands_like_cpp(player_guid)
        .expect("registered player");
    durable
        .lock()
        .expect("durable creature-runtime command lock")
        .drain_like_cpp()
}

pub(super) fn insert_player_registration_fixture_with_in_world_like_cpp(
    registry: &PlayerRegistry,
    counter: u64,
    send_tx: flume::Sender<Vec<u8>>,
    command_tx: flume::Sender<SessionCommand>,
    is_in_world: bool,
) {
    let mut info =
        player_registration_fixture_like_cpp(send_tx, command_tx, &format!("Player{counter}"));
    info.placement.is_in_world = is_in_world;
    registry.register_or_replace(
        ObjectGuid::create_player(1, counter as i64),
        info,
        Default::default(),
    );
}

pub(super) fn insert_player_registration_fixture_like_cpp(
    registry: &PlayerRegistry,
    counter: u64,
    send_tx: flume::Sender<Vec<u8>>,
    command_tx: flume::Sender<SessionCommand>,
) {
    insert_player_registration_fixture_with_in_world_like_cpp(
        registry, counter, send_tx, command_tx, true,
    );
}

pub(super) fn unique_temp_dir(name: &str) -> PathBuf {
    let mut path = env::temp_dir();
    path.push(format!(
        "rustycore_world_server_{name}_{}",
        std::process::id()
    ));

    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).expect("temp dir failed");
    path
}

// ── Slice 4A.1b: routing tests ───────────────────────────────────────────
// C++ anchors:
//   Object.cpp : WorldObject::SendMessageToSet (~1746-1764)
//   GridNotifiersImpl.h : MessageDistDeliverer::Visit(PlayerMapType&) (~43-46)
//   GridNotifiers.h : MessageDistDeliverer::SendPacket

pub(super) fn make_source_guid() -> ObjectGuid {
    ObjectGuid::create_world_object(wow_core::guid::HighGuid::Creature, 0, 1, 571, 0, 1, 1)
}

pub(super) fn make_nearby_visible_event_like_cpp(
    map_id: u16,
    instance_id: u32,
    source_position: Position,
    range: f32,
    required_3d: bool,
) -> wow_world::map_manager::RuntimeEvent {
    wow_world::map_manager::RuntimeEvent {
        source_guid: make_source_guid(),
        recipients: wow_world::map_manager::RecipientRule::NearbyVisible {
            source_guid: make_source_guid(),
            map_id,
            instance_id,
            source_position,
            range,
            required_3d,
        },
        packet_bytes: vec![0xAA, 0xBB],
    }
}

pub(super) fn make_creature_spell_runtime_plan_like_cpp(
    target_guid: ObjectGuid,
) -> (
    wow_world::map_manager::RuntimePlan,
    Vec<u8>,
    Vec<u8>,
    Vec<u8>,
) {
    use wow_packet::packets::spell::{
        SpellCastLogData, SpellCastVisual, SpellGoPkt, SpellLogPowerData, SpellStartPkt,
        SpellTargetData,
    };

    let caster_guid = make_source_guid();
    let cast_id = ObjectGuid::create_world_object(HighGuid::Cast, 3, 1, 571, 0, 12_345, 77);
    let visual = SpellCastVisual {
        spell_visual_id: 987,
        script_visual_id: 0,
    };
    let target = SpellTargetData {
        flags: 0x2,
        unit: target_guid,
        item: ObjectGuid::EMPTY,
        ..Default::default()
    };
    let start_bytes = SpellStartPkt {
        cast_data: Default::default(),
        caster: caster_guid,
        cast_id,
        original_cast_id: ObjectGuid::EMPTY,
        spell_id: 12_345,
        visual: visual.clone(),
        cast_flags: 0x0000_0002,
        cast_flags_ex: 0,
        cast_time_ms: 0,
        target: target.clone(),
    }
    .to_bytes();
    let go = SpellGoPkt {
        cast_data: Default::default(),
        caster: caster_guid,
        cast_id,
        original_cast_id: ObjectGuid::EMPTY,
        spell_id: 12_345,
        visual,
        cast_flags: 0x0004_0100,
        cast_flags_ex: 0,
        cast_time_ms: 123,
        target,
        hit_targets: vec![target_guid],
        miss_targets: Vec::new(),
    };
    let basic_go_bytes = go.to_bytes();
    let full_go_bytes = go.to_full_log_bytes_like_cpp(&SpellCastLogData {
        health: 321,
        attack_power: 45,
        spell_power: 0,
        armor: 67,
        power_data: vec![SpellLogPowerData {
            power_type: 0,
            amount: 89,
            cost: 0,
        }],
    });
    let plan = wow_world::map_manager::RuntimePlan {
        events: vec![wow_world::map_manager::RuntimeEvent {
            source_guid: caster_guid,
            recipients: wow_world::map_manager::RecipientRule::NearbyVisibleDurableSpellCast {
                source_guid: caster_guid,
                map_id: 571,
                instance_id: 0,
                source_position: Position::ZERO,
                range: 100.0,
                required_3d: false,
                basic_go_packet_bytes: basic_go_bytes.clone(),
                full_go_packet_bytes: full_go_bytes.clone(),
            },
            packet_bytes: start_bytes.clone(),
        }],
    };
    (plan, start_bytes, basic_go_bytes, full_go_bytes)
}

pub(super) fn make_registry_player_like_cpp(
    map_id: u16,
    instance_id: u32,
    position: Position,
    is_in_world: bool,
) -> (
    PlayerSessionRegistrationLikeCpp,
    flume::Receiver<SessionCommand>,
) {
    let (send_tx, _send_rx) = flume::bounded(4);
    let (command_tx, command_rx) = flume::bounded(4);
    let mut info = player_registration_fixture_like_cpp(send_tx, command_tx, "Tester");
    info.placement.map_id = map_id;
    info.placement.instance_id = instance_id;
    info.placement.position = position;
    info.placement.is_in_world = is_in_world;
    (info, command_rx)
}

pub(super) fn add_canonical_test_player_on_map_like_cpp(
    canonical: &wow_world::SharedCanonicalMapManager,
    guid: ObjectGuid,
    position: Position,
    map_id: u32,
    instance_id: u32,
    health: u64,
) {
    let mut player = Player::new(Some(1), false);
    player.unit_mut().world_mut().object_mut().create(guid);
    player.unit_mut().world_mut().set_name("RuntimeVictim");
    player
        .unit_mut()
        .world_mut()
        .set_map(map_id, instance_id)
        .unwrap();
    player.unit_mut().world_mut().relocate(position);
    player.unit_mut().world_mut().object_mut().add_to_world();
    player.unit_mut().set_level(80);
    player.unit_mut().set_faction(1);
    player.unit_mut().set_max_health(health);
    player.unit_mut().set_health(health);

    canonical
        .lock()
        .unwrap()
        .create_world_map(map_id, instance_id)
        .map_mut()
        .insert_map_object_record(MapObjectRecord::new_player(player).unwrap())
        .unwrap();
}

pub(super) fn add_canonical_test_creature_on_map_like_cpp(
    canonical: &wow_world::SharedCanonicalMapManager,
    guid: ObjectGuid,
    position: Position,
    map_id: u32,
    instance_id: u32,
    health: u64,
) {
    let mut creature = Creature::new(false);
    creature.unit_mut().world_mut().object_mut().create(guid);
    creature.unit_mut().world_mut().object_mut().set_entry(9002);
    creature
        .unit_mut()
        .world_mut()
        .set_map(map_id, instance_id)
        .unwrap();
    creature.unit_mut().world_mut().relocate(position);
    creature.unit_mut().world_mut().set_combat_reach(1.0);
    creature.unit_mut().world_mut().object_mut().add_to_world();
    creature.unit_mut().set_level(80);
    creature.unit_mut().set_max_health(health);
    creature.unit_mut().set_health(health);

    canonical
        .lock()
        .unwrap()
        .create_world_map(map_id, instance_id)
        .map_mut()
        .insert_map_object_record(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();
}

pub(super) fn mirror_canonical_melee_test_creature_like_cpp(
    canonical: &wow_world::SharedCanonicalMapManager,
    guid: ObjectGuid,
    map_id: u32,
    instance_id: u32,
) -> wow_world::map_manager::WorldCreature {
    let mut creature = canonical
        .lock()
        .unwrap()
        .find_map(map_id, instance_id)
        .unwrap()
        .map()
        .with_creature_like_cpp(guid, Clone::clone)
        .expect("canonical test creature");
    creature.set_ai_home_position(creature.position());
    creature.set_ai_identity_runtime(100, 14, 0, 0);
    creature
        .unit_mut()
        .set_weapon_damage(wow_constants::WeaponAttackType::BaseAttack, 3.0, 5.0);
    {
        let ai = creature.ai_ownership_mut();
        ai.aggro_radius = 20.0;
        ai.min_damage = 3;
        ai.max_damage = 5;
    }
    let create_data =
        wow_world::map_manager::WorldCreature::create_data_from_canonical_like_cpp(&creature);
    wow_world::map_manager::WorldCreature::from_canonical(creature, create_data)
}
