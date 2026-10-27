use super::*;
use wow_world_core::session::SessionHubHostLikeCpp;

async fn native_map_fixture(
    base: i64,
) -> (
    D3bWorldLikeCpp,
    wow_world::session::WorldSession,
    flume::Receiver<Vec<u8>>,
    flume::Sender<wow_packet::WorldPacket>,
    ObjectGuid,
) {
    let world = d3b_world_like_cpp(base);
    let player = ObjectGuid::create_player(1, base + 1);
    let (mut session, send, peer) = d3b_session_like_cpp(
        &world,
        player,
        "NativeMapDeath",
        Position::new(11.0, 10.0, 0.0, std::f32::consts::PI),
    )
    .await;
    d3b_player_attacks_like_cpp(&world, player);
    session.prepare_native_death_inputs_for_test_like_cpp();
    {
        let mut manager = world.canonical.lock().unwrap();
        let map = manager.find_map_mut(0, 0).unwrap().map_mut();
        let victim = map.get_typed_player_mut(player).unwrap();
        victim.unit_mut().attack_stop_like_cpp();
        victim.add_self_res_spell(12345);
        victim.unit_mut().set_emote_state_like_cpp(10);
        // Deterministic lethal damage even if the earlier Player pass regenerates.
        let attacker = map.get_typed_creature_mut(world.creature).unwrap();
        attacker.ai_ownership_mut().min_damage = 10_000;
        attacker.ai_ownership_mut().max_damage = 10_000;
    }
    for _ in send.try_iter() {}
    (world, session, send, peer, player)
}

fn player_state(
    world: &D3bWorldLikeCpp,
    guid: ObjectGuid,
) -> (
    u64,
    wow_constants::DeathState,
    bool,
    Option<wow_entities::PlayerDeathOperationPhase>,
    u32,
    Vec<i32>,
) {
    let manager = world.canonical.lock().unwrap();
    let player = manager
        .find_map(0, 0)
        .unwrap()
        .map()
        .get_typed_player(guid)
        .unwrap();
    (
        player.unit().data().health,
        player.unit().death_state(),
        player.data().player_flags & 0x10 != 0,
        player
            .native_death_operation_snapshot()
            .map(|operation| operation.phase()),
        player.unit().emote_state_like_cpp(),
        player.self_res_spells().to_vec(),
    )
}

async fn service_session_phase(session: &mut wow_world::session::WorldSession) {
    let receiver = session.session_phase_receiver_like_cpp();
    // Keep quiet sessions responsive to repeated primary presentation stages;
    // production retains its real 20ms admission deadline.
    if let Ok(Ok(request)) =
        tokio::time::timeout(Duration::from_millis(2), receiver.recv_async()).await
    {
        let catalogs = wow_world::session::SessionHandlerCatalogsLikeCpp::default();
        assert!(
            session
                .run_requested_session_phase_like_cpp(request, &catalogs)
                .await
                .is_none()
        );
    }
    session
        .run_session_commands_and_kills_for_test_like_cpp(|_| {})
        .await;
}

fn damage_aura_spell_catalog(
    rows: &[(i32, i32, i32)],
) -> (
    Arc<wow_data::SpellStore>,
    Arc<wow_data::SpellClassOptionsStore>,
) {
    let misc_entries = rows
        .iter()
        .enumerate()
        .map(
            |(index, (spell_id, _, _))| wow_data::spell_db2::SpellMiscEntry {
                id: u32::try_from(index + 1).expect("fixture SpellMisc id fits u32"),
                attributes: [0; 15],
                difficulty_id: 0,
                spell_id: *spell_id as u32,
                ..Default::default()
            },
        )
        .collect::<Vec<_>>();
    let effect_entries = rows
        .iter()
        .enumerate()
        .map(
            |(index, (spell_id, aura_type, amount))| wow_data::spell_db2::SpellEffectDb2Entry {
                id: u32::try_from(index + 1).expect("fixture SpellEffect id fits u32"),
                difficulty_id: 0,
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_amplitude: 0.0,
                effect_attributes: 0,
                effect_aura: i16::try_from(*aura_type).expect("fixture Aura type fits i16"),
                effect_aura_period: 0,
                effect_base_points: *amount - 1,
                effect_bonus_coefficient: 0.0,
                effect_chain_amplitude: 0.0,
                effect_chain_targets: 0,
                effect_die_sides: 0,
                effect_item_type: 0,
                effect_mechanic: 0,
                effect_points_per_resource: 0.0,
                effect_pos_facing: 0.0,
                effect_real_points_per_level: 0.0,
                effect_trigger_spell: 0,
                bonus_coefficient_from_ap: 0.0,
                pvp_multiplier: 0.0,
                coefficient: 0.0,
                variance: 0.0,
                resource_coefficient: 0.0,
                group_size_base_points_coefficient: 0.0,
                effect_misc_value: [1, 0],
                effect_radius_index: [0; 2],
                effect_spell_class_mask: [0; 4],
                implicit_target: [0; 2],
                spell_id: *spell_id as u32,
            },
        )
        .collect::<Vec<_>>();
    let class_options_entries = rows
        .iter()
        .enumerate()
        .map(
            |(index, (spell_id, _, _))| wow_data::SpellClassOptionsEntry {
                id: u32::try_from(index + 1).expect("fixture SpellClassOptions id fits u32"),
                spell_id: *spell_id,
                modal_next_spell: 0,
                spell_class_set: 0,
                spell_class_mask: [0; 4],
            },
        )
        .collect::<Vec<_>>();
    let stores = wow_data::EffectiveCoreSpellDb2StoresLikeCpp::new(
        wow_data::spell_db2::SpellCategoriesStore::from_entries([]),
        wow_data::spell_db2::SpellMiscStore::from_entries(misc_entries),
        wow_data::spell_db2::SpellEffectDb2Store::from_entries(effect_entries),
        wow_data::spell_db2::SpellShapeshiftStore::from_entries([]),
        wow_data::spell_db2::SpellInterruptsStore::from_entries([]),
        wow_data::spell_db2::SpellCastTimesStore::from_entries([]),
        wow_data::spell_db2::SpellCooldownsStore::from_entries([]),
        wow_data::spell_db2::SpellCastingRequirementsStore::from_entries([]),
        wow_data::spell_db2::SpellPowerStore::from_entries([]),
        wow_data::spell_db2::SpellPowerDifficultyStore::from_entries([]),
    );
    let spells = wow_world_test_support::spell_catalog::canonical_spell_store(
        rows.iter().map(|(spell_id, _, _)| (*spell_id as u32, 0)),
    )
    .hydrate_effective_core_db2_like_cpp(stores);
    (
        Arc::new(spells),
        Arc::new(wow_data::SpellClassOptionsStore::from_entries(
            class_options_entries,
        )),
    )
}

fn install_player_damage_aura(
    world: &D3bWorldLikeCpp,
    target: ObjectGuid,
    caster: ObjectGuid,
    slot: u8,
    spell_id: i32,
    aura_type: i32,
    amount: i32,
) {
    let snapshot =
        wow_entities::AuraThreatSnapshotLikeCpp::new([0; 2], vec![(1, aura_type, amount, 1)]);
    let mut manager = world.canonical.lock().unwrap();
    let player = manager
        .find_map_mut(0, 0)
        .unwrap()
        .map_mut()
        .get_typed_player_mut(target)
        .unwrap();
    player.apply_player_threat_aura_like_cpp(spell_id as u32, caster, slot, snapshot.clone());
    player.install_player_threat_aura_like_cpp(
        slot,
        snapshot,
        wow_entities::AuraApplicationLikeCpp {
            spell_id,
            difficulty_id: 0,
            caster_guid: caster,
            slot,
            duration_total: 60_000,
            duration_remaining: 60_000,
            stack_count: 1,
            aura_flags: 1,
            effect_mask: 1,
            aura_interrupt_flags: 0,
            aura_interrupt_flags2: 0,
            represented_effect: None,
            represented_amount: amount,
            represented_effect_amounts: vec![wow_entities::RepresentedAuraEffectAmountLikeCpp {
                effect_index: 0,
                amount,
            }],
            represented_misc_value: Some(1),
            represented_multiplier: 1.0,
            applied_at: Instant::now(),
        },
    );
    player
        .unit_mut()
        .subsystems_mut()
        .auras
        .set_aura_cast_provenance_like_cpp(
            slot,
            wow_entities::AuraCastProvenanceLikeCpp {
                cast_id: ObjectGuid::create_world_object(
                    wow_core::guid::HighGuid::Cast,
                    3,
                    1,
                    571,
                    0,
                    spell_id as u32,
                    i64::from(spell_id) + i64::from(slot),
                ),
                spell_visual_id: 0,
                caster_level: Some(80),
            },
        );
}

fn set_player_health(world: &D3bWorldLikeCpp, guid: ObjectGuid, health: u64) {
    let mut manager = world.canonical.lock().unwrap();
    let player = manager
        .find_map_mut(0, 0)
        .unwrap()
        .map_mut()
        .get_typed_player_mut(guid)
        .unwrap();
    player.unit_mut().set_max_health(health.max(1_000));
    player.unit_mut().set_health(health);
    player
        .unit_mut()
        .set_death_state(wow_constants::DeathState::Alive);
}

fn spawn_canonical_loop_with_spell_store(
    world: &D3bWorldLikeCpp,
    spell_store: Arc<wow_data::SpellStore>,
) -> (
    tokio::task::JoinHandle<()>,
    Arc<CanonicalCreatureRuntimeProbeLikeCpp>,
) {
    let mut runtime = d3b_canonical_runtime_like_cpp();
    runtime.aggro_config.spell_store = Some(spell_store);
    let probe = Arc::clone(&runtime.probe);
    let guards = D3bLoopGuardsLikeCpp::default();
    let task = crate::runtime::spawn_canonical_map_update_loop(
        Arc::clone(&world.canonical),
        D3B_TICK_MS_LIKE_CPP,
        60_000,
        Arc::clone(&guards.spawn_metadata),
        Arc::new(wow_data::ConditionEntriesByTypeStore::default()),
        Arc::clone(&world.map_store),
        Arc::new(FakeGameEventPersistencePortLikeCpp::default()),
        world.writer.clone(),
        Arc::clone(&guards.respawn_db_mutation_order),
        Arc::new(std::sync::atomic::AtomicBool::new(false)),
        empty_loaded_grid_creature_respawn_caches_like_cpp(),
        Arc::new(wow_data::AreaTriggerTemplateStore::default()),
        CanonicalGameEventSchedulerLikeCpp::start_system(3_600_000),
        Arc::clone(&world.registry),
        Arc::new(ActiveWorldSessionRegistryLikeCpp::new()),
        Arc::new(wow_data::BattlemasterListStore::from_entries([])),
        Arc::clone(&guards.world_state_mgr),
        Some(runtime),
        crate::module_hooks::WorldModuleHooksLikeCpp::new(Arc::new(
            wow_module_api::ModuleRegistry::new(),
        )),
    );
    (task, probe)
}

async fn add_native_death_participant(
    world: &D3bWorldLikeCpp,
    guid: ObjectGuid,
    name: &str,
    position: Position,
    spell_store: &Arc<wow_data::SpellStore>,
    spell_class_options_store: &Arc<wow_data::SpellClassOptionsStore>,
) -> (
    wow_world::session::WorldSession,
    flume::Receiver<Vec<u8>>,
    flume::Sender<wow_packet::WorldPacket>,
) {
    let (mut session, send, peer) = d3b_session_like_cpp(world, guid, name, position).await;
    install_aura_lifetime_human_race(&mut session);
    session.set_spell_store(Arc::clone(spell_store));
    session.set_spell_class_options_store(Arc::clone(spell_class_options_store));
    session.prepare_native_death_inputs_for_test_like_cpp();
    for _ in send.try_iter() {}
    (session, send, peer)
}

fn split_damage_log_target(packet_bytes: &[u8]) -> Option<ObjectGuid> {
    let mut packet = wow_packet::WorldPacket::from_bytes(packet_bytes);
    if packet.server_opcode() != Some(ServerOpcodes::SpellNonMeleeDamageLog) {
        return None;
    }
    packet.skip_opcode();
    packet.read_packed_guid().ok()
}

fn creature_attacker_state_packet(packet_bytes: &[u8], creature: ObjectGuid) -> bool {
    let mut packet = wow_packet::WorldPacket::from_bytes(packet_bytes);
    if packet.server_opcode() != Some(ServerOpcodes::AttackerStateUpdate) {
        return false;
    }
    packet.skip_opcode();
    let _log_data = packet.read_uint8().unwrap();
    let _round_size = packet.read_uint32().unwrap();
    let _hit_info = packet.read_uint32().unwrap();
    packet.read_packed_guid().unwrap() == creature
}

fn health_update_packet(packet_bytes: &[u8], player: ObjectGuid, health: i64) -> bool {
    let mut packet = wow_packet::WorldPacket::from_bytes(packet_bytes);
    if packet.server_opcode() != Some(ServerOpcodes::HealthUpdate) {
        return false;
    }
    packet.skip_opcode();
    packet.read_packed_guid().ok() == Some(player) && packet.read_int64().ok() == Some(health)
}

fn attach_test_spell_catalog(
    session: &mut wow_world::session::WorldSession,
    spell_store: &Arc<wow_data::SpellStore>,
    spell_class_options_store: &Arc<wow_data::SpellClassOptionsStore>,
) {
    install_aura_lifetime_human_race(session);
    session.set_spell_store(Arc::clone(spell_store));
    session.set_spell_class_options_store(Arc::clone(spell_class_options_store));
    session.prepare_native_death_inputs_for_test_like_cpp();
}

fn install_aura_lifetime_human_race(session: &mut wow_world::session::WorldSession) {
    // Selected Aura ticking reads the current race row before it filters Aura
    // slots. These canonical D3b sessions represent Human Players on map 0.
    let race_id = session
        .core()
        .with_owned_player_like_cpp(|player| player.race_like_cpp())
        .expect("canonical Player has a race identity");
    assert_eq!(race_id, 1, "the Aura fixture uses the Human race");
    session.set_chr_races_store(Arc::new(
        wow_data::character_progression::ChrRacesStore::from_entries([
            wow_data::character_progression::ChrRacesEntry {
                id: u32::from(race_id),
                res_sickness_spell_id: 0,
                client_prefix: String::new(),
                client_file_string: String::new(),
                name: "Human Aura lifetime test race".into(),
                flags: 0,
                male_display_id: 0,
                female_display_id: 0,
                high_res_male_display_id: 0,
                high_res_female_display_id: 0,
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
                cinematic_sequence_id: 0,
                helmet_anim_scaling_race_id: 0,
                transmogrify_disabled_slot_mask: 0,
                faction_id: 0,
                base_language: 0,
                creature_type: 0,
                alliance: 0,
                race_related: 0,
                unaltered_visual_race_id: 0,
                default_class_id: 0,
                neutral_race_id: 0,
            },
        ]),
    ));
}

#[tokio::test]
async fn secondary_split_deaths_finish_before_each_log_and_primary_hit() {
    let _tracing = tracing::subscriber::set_default(
        tracing_subscriber::fmt()
            .with_test_writer()
            .with_max_level(tracing::Level::ERROR)
            .finish(),
    );
    const SPLIT_ONE: i32 = 96_481;
    const SPLIT_TWO: i32 = 96_482;
    let split = wow_data::spell::aura_types::SPELL_AURA_SPLIT_DAMAGE_PCT;
    let (spell_store, spell_class_options_store) =
        damage_aura_spell_catalog(&[(SPLIT_ONE, split, 50), (SPLIT_TWO, split, 50)]);
    let (world, mut primary_session, primary_send, _peer, primary) =
        native_map_fixture(96_510).await;
    attach_test_spell_catalog(
        &mut primary_session,
        &spell_store,
        &spell_class_options_store,
    );
    let secondary_one = ObjectGuid::create_player(1, 96_512);
    let secondary_two = ObjectGuid::create_player(1, 96_513);
    let (mut secondary_one_session, secondary_one_send, _secondary_one_peer) =
        add_native_death_participant(
            &world,
            secondary_one,
            "SplitOne",
            Position::new(12.0, 10.0, 0.0, 0.0),
            &spell_store,
            &spell_class_options_store,
        )
        .await;
    let (mut secondary_two_session, secondary_two_send, _secondary_two_peer) =
        add_native_death_participant(
            &world,
            secondary_two,
            "SplitTwo",
            Position::new(13.0, 10.0, 0.0, 0.0),
            &spell_store,
            &spell_class_options_store,
        )
        .await;
    set_player_health(&world, primary, 50_000);
    set_player_health(&world, secondary_one, 1);
    set_player_health(&world, secondary_two, 1);
    install_player_damage_aura(&world, primary, secondary_one, 10, SPLIT_ONE, split, 50);
    install_player_damage_aura(&world, primary, secondary_two, 11, SPLIT_TWO, split, 50);
    primary_session.refresh_visibility_for_test_like_cpp().await;
    assert!(primary_session.client_sees_guid_for_test_like_cpp(secondary_one));
    assert!(primary_session.client_sees_guid_for_test_like_cpp(secondary_two));
    for _ in primary_send.try_iter() {}
    let (task, probe) = spawn_canonical_loop_with_spell_store(&world, spell_store);
    let started = Instant::now();
    let mut primary_packets = Vec::new();
    let mut secondary_one_packets = Vec::new();
    let mut secondary_two_packets = Vec::new();
    while started.elapsed() < Duration::from_secs(7) {
        tokio::join!(
            service_session_phase(&mut primary_session),
            service_session_phase(&mut secondary_one_session),
            service_session_phase(&mut secondary_two_session),
        );
        primary_packets.extend(primary_send.try_iter());
        secondary_one_packets.extend(secondary_one_send.try_iter());
        secondary_two_packets.extend(secondary_two_send.try_iter());
        let first = player_state(&world, secondary_one);
        let second = player_state(&world, secondary_two);
        let primary_state = player_state(&world, primary);
        if first.0 == 0
            && first.3.is_none()
            && second.0 == 0
            && second.3.is_none()
            && primary_state.0 == 47_500
            && probe.executed_ticks.load(Ordering::Relaxed) > 0
            && primary_packets
                .iter()
                .any(|packet| health_update_packet(packet, primary, 47_500))
        {
            break;
        }
    }
    let producer_finished = task.is_finished();
    let refused_ticks = probe.refused_ticks.load(Ordering::Relaxed);
    task.abort();
    let _ = task.await;

    for guid in [secondary_one, secondary_two] {
        let state = player_state(&world, guid);
        assert_eq!(
            state.0,
            0,
            "each split caster completed native death; state={state:?}, primary_health={}, ticks={}, refused_ticks={refused_ticks}, producer_finished={producer_finished}, primary_packets={}, secondary_packets={}",
            player_state(&world, primary).0,
            probe.executed_ticks.load(Ordering::Relaxed),
            primary_packets.len(),
            secondary_one_packets.len() + secondary_two_packets.len(),
        );
        assert!(matches!(
            state.1,
            wow_constants::DeathState::JustDied | wow_constants::DeathState::Corpse
        ));
        assert_eq!(state.3, None, "the exact native operation was acknowledged");
    }
    assert_eq!(player_state(&world, primary).0, 47_500);
    let split_logs = primary_packets
        .iter()
        .enumerate()
        .filter_map(|(index, packet)| split_damage_log_target(packet).map(|target| (index, target)))
        .collect::<Vec<_>>();
    assert_eq!(
        split_logs
            .iter()
            .map(|(_, target)| *target)
            .collect::<Vec<_>>(),
        vec![secondary_one, secondary_two],
        "each split combat log targets the Aura participant in entry order"
    );
    let attacker_state = primary_packets
        .iter()
        .position(|packet| creature_attacker_state_packet(packet, world.creature))
        .expect("primary AttackerStateUpdate follows all split bodies and logs");
    assert!(split_logs.iter().all(|(index, _)| *index < attacker_state));
    for (guid, packets) in [
        (secondary_one, &secondary_one_packets),
        (secondary_two, &secondary_two_packets),
    ] {
        let body_packet = packets
            .iter()
            .position(|packet| {
                wow_packet::WorldPacket::from_bytes(packet).server_opcode()
                    == Some(ServerOpcodes::StandStateUpdate)
            })
            .expect("the native body sent its death presentation");
        let split_log = packets
            .iter()
            .position(|packet| split_damage_log_target(packet) == Some(guid))
            .expect("the source Player received its split damage log");
        assert!(
            body_packet < split_log,
            "the split log is enqueued after the source Player's native death body"
        );
    }
}

#[tokio::test]
async fn secondary_split_death_failure_fences_next_aura_and_primary_mutation() {
    const SPLIT_ONE: i32 = 96_483;
    const SPLIT_TWO: i32 = 96_484;
    let split = wow_data::spell::aura_types::SPELL_AURA_SPLIT_DAMAGE_PCT;
    let (spell_store, spell_class_options_store) =
        damage_aura_spell_catalog(&[(SPLIT_ONE, split, 50), (SPLIT_TWO, split, 50)]);
    let (world, mut primary_session, primary_send, _peer, primary) =
        native_map_fixture(96_520).await;
    attach_test_spell_catalog(
        &mut primary_session,
        &spell_store,
        &spell_class_options_store,
    );
    let failing_secondary = ObjectGuid::create_player(1, 96_522);
    let later_secondary = ObjectGuid::create_player(1, 96_523);
    let (mut failing_session, failing_send, _failing_peer) = add_native_death_participant(
        &world,
        failing_secondary,
        "SplitFailure",
        Position::new(12.0, 10.0, 0.0, 0.0),
        &spell_store,
        &spell_class_options_store,
    )
    .await;
    let (mut later_session, _later_send, _later_peer) = add_native_death_participant(
        &world,
        later_secondary,
        "SplitLater",
        Position::new(13.0, 10.0, 0.0, 0.0),
        &spell_store,
        &spell_class_options_store,
    )
    .await;
    set_player_health(&world, primary, 50_000);
    set_player_health(&world, failing_secondary, 1);
    set_player_health(&world, later_secondary, 100);
    install_player_damage_aura(&world, primary, failing_secondary, 10, SPLIT_ONE, split, 50);
    install_player_damage_aura(&world, primary, later_secondary, 11, SPLIT_TWO, split, 50);
    {
        let mut manager = world.canonical.lock().unwrap();
        let player = manager
            .find_map_mut(0, 0)
            .unwrap()
            .map_mut()
            .get_typed_player_mut(failing_secondary)
            .unwrap();
        let mut spells = player.spell_runtime_like_cpp().clone();
        spells.set_row_authority_like_cpp(false, false);
        player.replace_spell_runtime_like_cpp(spells);
    }
    let (task, probe) = spawn_canonical_loop_with_spell_store(&world, spell_store);
    let started = Instant::now();
    let mut primary_packets = Vec::new();
    let mut failing_packets = Vec::new();
    while started.elapsed() < Duration::from_secs(7) {
        tokio::join!(
            service_session_phase(&mut primary_session),
            service_session_phase(&mut failing_session),
            service_session_phase(&mut later_session),
        );
        primary_packets.extend(primary_send.try_iter());
        failing_packets.extend(failing_send.try_iter());
        let state = player_state(&world, failing_secondary);
        if state.3 == Some(wow_entities::PlayerDeathOperationPhase::InFlight) {
            break;
        }
    }
    tokio::task::yield_now().await;
    let completed_ticks = probe.executed_ticks.load(Ordering::Relaxed);
    for _ in 0..3 {
        tokio::join!(
            service_session_phase(&mut primary_session),
            service_session_phase(&mut failing_session),
            service_session_phase(&mut later_session),
        );
        primary_packets.extend(primary_send.try_iter());
        failing_packets.extend(failing_send.try_iter());
    }
    task.abort();
    let _ = task.await;
    let failed = player_state(&world, failing_secondary);
    assert_eq!(
        failed.0, 1,
        "the failing native body did not write health zero"
    );
    assert_eq!(
        failed.3,
        Some(wow_entities::PlayerDeathOperationPhase::InFlight),
        "the unresolved operation retains its fence"
    );
    assert_eq!(player_state(&world, later_secondary).0, 100);
    assert_eq!(player_state(&world, primary).0, 50_000);
    assert_eq!(
        probe.executed_ticks.load(Ordering::Relaxed),
        completed_ticks
    );
    assert!(
        !primary_packets
            .iter()
            .any(|packet| creature_attacker_state_packet(packet, world.creature))
    );
    assert!(!failing_packets.iter().any(|packet| {
        wow_packet::WorldPacket::from_bytes(packet).server_opcode()
            == Some(ServerOpcodes::StandStateUpdate)
    }));
}

#[tokio::test]
async fn lethal_self_share_runs_one_native_body_after_attacker_state() {
    let _tracing = tracing::subscriber::set_default(
        tracing_subscriber::fmt()
            .with_test_writer()
            .with_max_level(tracing::Level::ERROR)
            .finish(),
    );
    const SHARE: i32 = 96_485;
    let share = wow_data::spell::aura_types::SPELL_AURA_SHARE_DAMAGE_PCT;
    let (spell_store, spell_class_options_store) =
        damage_aura_spell_catalog(&[(SHARE, share, 100)]);
    let (world, mut session, send, _peer, player) = native_map_fixture(96_530).await;
    attach_test_spell_catalog(&mut session, &spell_store, &spell_class_options_store);
    set_player_health(&world, player, 1);
    install_player_damage_aura(&world, player, player, 10, SHARE, share, 100);
    let (task, probe) = spawn_canonical_loop_with_spell_store(&world, spell_store);
    let started = Instant::now();
    let mut packets = Vec::new();
    while started.elapsed() < Duration::from_secs(5) {
        service_session_phase(&mut session).await;
        packets.extend(send.try_iter());
        let state = player_state(&world, player);
        if state.0 == 0 && state.3.is_none() && probe.executed_ticks.load(Ordering::Relaxed) > 0 {
            break;
        }
    }
    task.abort();
    let _ = task.await;
    let state = player_state(&world, player);
    assert_eq!(
        state.0,
        0,
        "self-share did not complete; state={state:?}, ticks={}, refused_ticks={}",
        probe.executed_ticks.load(Ordering::Relaxed),
        probe.refused_ticks.load(Ordering::Relaxed),
    );
    assert_eq!(
        state.3, None,
        "self-share's native body is acknowledged once"
    );
    let attacker_state = packets
        .iter()
        .position(|packet| creature_attacker_state_packet(packet, world.creature))
        .expect("pre-share AttackerStateUpdate is published");
    let death_presentation = packets
        .iter()
        .position(|packet| {
            wow_packet::WorldPacket::from_bytes(packet).server_opcode()
                == Some(ServerOpcodes::StandStateUpdate)
        })
        .expect("the existing native death body publishes once");
    assert!(attacker_state < death_presentation);
    assert_eq!(
        packets
            .iter()
            .filter(
                |packet| wow_packet::WorldPacket::from_bytes(packet).server_opcode()
                    == Some(ServerOpcodes::StandStateUpdate)
            )
            .count(),
        1,
        "the primary was not killed a second time after self-share"
    );
    assert!(!packets.iter().any(|packet| {
        wow_packet::WorldPacket::from_bytes(packet).server_opcode()
            == Some(ServerOpcodes::HealthUpdate)
    }));
}

#[tokio::test]
async fn native_map_combat_finishes_player_death_before_resuming_the_producer() {
    let (world, mut session, send, _peer, player) = native_map_fixture(96_450).await;
    let (task, probe) = d3b_spawn_canonical_loop_like_cpp(&world);
    let started = Instant::now();
    let mut opcodes = Vec::new();
    while started.elapsed() < Duration::from_secs(5) {
        service_session_phase(&mut session).await;
        opcodes.extend(d3b_server_opcodes_like_cpp(&send));
        let state = player_state(&world, player);
        if state.0 == 0 && state.3.is_none() && probe.executed_ticks.load(Ordering::Relaxed) > 0 {
            break;
        }
    }
    task.abort();
    let _ = task.await;
    let state = player_state(&world, player);
    assert_eq!(state.0, 0);
    assert!(matches!(
        state.1,
        wow_constants::DeathState::JustDied | wow_constants::DeathState::Corpse
    ));
    assert!(!state.2, "death alone must not release the spirit");
    assert_eq!(
        state.3, None,
        "the combat future acknowledged the exact completed operation"
    );
    assert_eq!(state.4, 0);
    assert!(
        state.5.is_empty(),
        "the native SelfRes initializer replaced the stale list"
    );
    assert!(probe.executed_ticks.load(Ordering::Relaxed) > 0);
    assert!(opcodes.contains(&ServerOpcodes::AttackerStateUpdate));
    assert!(
        opcodes
            .iter()
            .position(|opcode| *opcode == ServerOpcodes::AttackStart)
            .unwrap()
            < opcodes
                .iter()
                .position(|opcode| *opcode == ServerOpcodes::AttackerStateUpdate)
                .unwrap(),
        "the newly engaged creature starts its attack before the lethal hit"
    );
    assert!(opcodes.contains(&ServerOpcodes::StandStateUpdate));
    assert!(
        !opcodes.contains(&ServerOpcodes::HealthUpdate),
        "lethal SetHealth uses the Map field publisher"
    );
}

#[tokio::test]
async fn native_map_combat_ignores_disconnected_observer_packet_queue() {
    let (world, mut primary_session, primary_send, _primary_peer, primary) =
        native_map_fixture(96_540).await;
    let (spell_store, spell_class_options_store) = damage_aura_spell_catalog(&[]);
    let observer = ObjectGuid::create_player(1, 96_542);
    let (mut observer_session, observer_send, observer_peer) = add_native_death_participant(
        &world,
        observer,
        "DisconnectedOutputObserver",
        Position::new(12.0, 10.0, 0.0, 0.0),
        &spell_store,
        &spell_class_options_store,
    )
    .await;
    observer_session
        .refresh_visibility_for_test_like_cpp()
        .await;
    assert!(observer_session.client_sees_guid_for_test_like_cpp(world.creature));
    drop(observer_send);
    assert!(
        !observer_peer.is_disconnected(),
        "the observer input peer stays connected while its output receiver is closed"
    );
    set_player_health(&world, primary, 1);

    let (task, probe) = d3b_spawn_canonical_loop_like_cpp(&world);
    let started = Instant::now();
    let mut primary_opcodes = Vec::new();
    while started.elapsed() < Duration::from_secs(7) {
        tokio::join!(
            service_session_phase(&mut primary_session),
            service_session_phase(&mut observer_session),
        );
        primary_opcodes.extend(d3b_server_opcodes_like_cpp(&primary_send));
        let state = player_state(&world, primary);
        if state.0 == 0 && state.3.is_none() && probe.executed_ticks.load(Ordering::Relaxed) > 0 {
            break;
        }
    }
    task.abort();
    let _ = task.await;

    let state = player_state(&world, primary);
    assert_eq!(
        state.0, 0,
        "the disconnected observer did not cancel lethal combat"
    );
    assert!(matches!(
        state.1,
        wow_constants::DeathState::JustDied | wow_constants::DeathState::Corpse
    ));
    assert_eq!(state.3, None, "the primary native death operation retired");
    assert!(probe.executed_ticks.load(Ordering::Relaxed) > 0);
    assert!(primary_opcodes.contains(&ServerOpcodes::AttackStart));
    assert!(primary_opcodes.contains(&ServerOpcodes::AttackerStateUpdate));
    assert!(primary_opcodes.contains(&ServerOpcodes::StandStateUpdate));
    assert!(player_state(&world, observer).0 > 0);
    assert!(!observer_peer.is_disconnected());
}

#[tokio::test]
async fn native_map_combat_failure_retains_player_and_holds_later_ticks() {
    let (world, mut session, send, _peer, player) = native_map_fixture(96_460).await;
    {
        let mut manager = world.canonical.lock().unwrap();
        let player = manager
            .find_map_mut(0, 0)
            .unwrap()
            .map_mut()
            .get_typed_player_mut(player)
            .unwrap();
        let mut spells = player.spell_runtime_like_cpp().clone();
        spells.set_row_authority_like_cpp(false, false);
        player.replace_spell_runtime_like_cpp(spells);
    }
    let (task, probe) = d3b_spawn_canonical_loop_like_cpp(&world);
    let started = Instant::now();
    let mut opcodes = Vec::new();
    while started.elapsed() < Duration::from_secs(5) {
        service_session_phase(&mut session).await;
        opcodes.extend(d3b_server_opcodes_like_cpp(&send));
        if player_state(&world, player).3 == Some(wow_entities::PlayerDeathOperationPhase::InFlight)
        {
            break;
        }
    }
    tokio::task::yield_now().await;
    let completed = probe.executed_ticks.load(Ordering::Relaxed);
    for _ in 0..3 {
        service_session_phase(&mut session).await;
        opcodes.extend(d3b_server_opcodes_like_cpp(&send));
    }
    task.abort();
    let _ = task.await;
    let state = player_state(&world, player);
    assert!(state.0 > 0);
    assert_eq!(state.1, wow_constants::DeathState::Alive);
    assert_eq!(
        state.3,
        Some(wow_entities::PlayerDeathOperationPhase::InFlight)
    );
    assert_eq!(probe.executed_ticks.load(Ordering::Relaxed), completed);
    assert!(!opcodes.contains(&ServerOpcodes::StandStateUpdate));
}

#[tokio::test]
async fn native_map_combat_pending_session_death_fences_recovery_and_the_tick_tail() {
    let (world, mut session, send, _peer, player) = native_map_fixture(96_470).await;
    {
        let mut manager = world.canonical.lock().unwrap();
        let player = manager
            .find_map_mut(0, 0)
            .unwrap()
            .map_mut()
            .get_typed_player_mut(player)
            .unwrap();
        player
            .begin_native_death_operation(wow_entities::PlayerDeathOperationContext::new(
                player.guid(),
                false,
            ))
            .unwrap();
    }
    let (task, probe) = d3b_spawn_canonical_loop_like_cpp(&world);
    for _ in 0..5 {
        service_session_phase(&mut session).await;
    }
    task.abort();
    let _ = task.await;
    let state = player_state(&world, player);
    assert!(state.0 > 0);
    assert_eq!(state.1, wow_constants::DeathState::Alive);
    assert_eq!(
        state.3,
        Some(wow_entities::PlayerDeathOperationPhase::Ready)
    );
    assert_eq!(probe.executed_ticks.load(Ordering::Relaxed), 0);
    assert!(!d3b_server_opcodes_like_cpp(&send).contains(&ServerOpcodes::AttackerStateUpdate));
}
