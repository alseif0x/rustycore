use super::*;
use wow_world::session::mailbox::SessionPhaseRequestLikeCpp;
use wow_world_core::session::SessionHubHostLikeCpp;

const GHOST: i32 = 8_326;
const GHOST_EFFECTS: [(u32, i32); 3] = [
    (0b001, wow_data::spell::aura_types::SPELL_AURA_GHOST),
    (
        0b010,
        wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SPEED,
    ),
    (
        0b100,
        wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SWIM_SPEED,
    ),
];

fn spawn_selected_recovery_root(world: &D3bWorldLikeCpp) -> tokio::task::JoinHandle<()> {
    let guards = D3bLoopGuardsLikeCpp::default();
    crate::runtime::spawn_canonical_map_update_loop(
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
        None,
        crate::module_hooks::WorldModuleHooksLikeCpp::new(Arc::new(
            wow_module_api::ModuleRegistry::new(),
        )),
    )
}

fn install_selected_ghost(session: &mut wow_world::session::WorldSession) -> (u8, Instant) {
    let player_guid = session.player_guid().expect("entered Player GUID");
    let race_id = session
        .core()
        .with_owned_player_like_cpp(|player| player.race_like_cpp())
        .expect("canonical Player owner");
    session.set_chr_races_store(Arc::new(
        wow_data::character_progression::ChrRacesStore::from_entries([
            wow_data::character_progression::ChrRacesEntry {
                id: u32::from(race_id),
                res_sickness_spell_id: 0,
                client_prefix: String::new(),
                client_file_string: String::new(),
                name: "Selected recovery test race".into(),
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
                helmet_anim_scaling_race_id: 0,
                transmogrify_disabled_slot_mask: 0,
                faction_id: 0,
                cinematic_sequence_id: 0,
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
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert_spell_misc_attributes_like_cpp(GHOST, [0; 15]);
    session.set_spell_store(Arc::new(spell_store));
    session
        .set_spell_aura_options_store(Arc::new(wow_data::SpellAuraOptionsStore::from_entries([])));
    session.set_difficulty_store(Arc::new(wow_data::DifficultyStore::from_ids([0])));

    let slot = 4;
    let stale = Instant::now() - Duration::from_secs(3_600);
    let snapshot = wow_entities::AuraThreatSnapshotLikeCpp::new(
        [0; 2],
        GHOST_EFFECTS
            .into_iter()
            .map(|(bit, aura_type)| (bit, aura_type, 0, 0))
            .collect(),
    );
    let application = wow_entities::AuraApplicationLikeCpp {
        spell_id: GHOST,
        difficulty_id: 0,
        caster_guid: player_guid,
        slot,
        duration_total: 60_000,
        duration_remaining: 60_000,
        stack_count: 1,
        aura_flags: 1,
        effect_mask: 0b111,
        aura_interrupt_flags: 0,
        aura_interrupt_flags2: 0,
        represented_effect: None,
        represented_amount: 0,
        represented_effect_amounts: Vec::new(),
        represented_misc_value: None,
        represented_multiplier: 1.0,
        applied_at: stale,
    };
    session
        .core_mut()
        .with_owned_player_mut_like_cpp(|player| {
            player.apply_player_threat_aura_like_cpp(
                GHOST as u32,
                player_guid,
                slot,
                snapshot.clone(),
            );
            player.install_player_threat_aura_like_cpp(slot, snapshot, application);
            let auras = &mut player.unit_mut().subsystems_mut().auras;
            auras.set_aura_cast_provenance_like_cpp(
                slot,
                wow_entities::AuraCastProvenanceLikeCpp {
                    cast_id: ObjectGuid::create_world_object(
                        wow_core::guid::HighGuid::Cast,
                        3,
                        1,
                        571,
                        0,
                        GHOST as u32,
                        96_392,
                    ),
                    spell_visual_id: 0,
                    caster_level: Some(80),
                },
            );
            assert!(auras.set_aura_persistence_metadata(
                slot,
                wow_entities::AuraPersistenceMetadata {
                    item_guid: ObjectGuid::EMPTY,
                    cast_item_id: 0,
                    cast_item_level: 80,
                    recalculate_mask: 0b111,
                    charges: 0,
                    base_amounts: vec![(0, 0), (1, 0), (2, 0)],
                },
            ));
            auras.visible_auras_to_update.insert(slot);
        })
        .expect("canonical Player receives the selected Ghost");
    (slot, stale)
}

fn selected_updates(packets: &[Vec<u8>]) -> Vec<(ObjectGuid, i32, Option<u32>)> {
    let mut updates = Vec::new();
    for bytes in packets {
        let mut packet = wow_packet::WorldPacket::from_bytes(bytes);
        if packet.server_opcode() != Some(ServerOpcodes::AuraUpdate) {
            continue;
        }
        packet.skip_opcode();
        let _update_all = packet.read_bit().unwrap();
        let count = packet.read_bits(9).unwrap();
        let mut packet_updates = Vec::new();
        for _ in 0..count {
            let _slot = packet.read_uint8().unwrap();
            if !packet.read_bit().unwrap() {
                packet.reset_bits();
                continue;
            }
            let _cast_id = packet.read_packed_guid().unwrap();
            let spell_id = packet.read_int32().unwrap();
            let _visual_id = packet.read_int32().unwrap();
            let _flags = packet.read_uint16().unwrap();
            let _active_flags = packet.read_uint32().unwrap();
            let _cast_level = packet.read_uint16().unwrap();
            let _applications = packet.read_uint8().unwrap();
            let _content_tuning = packet.read_int32().unwrap();
            let has_cast_unit = packet.read_bit().unwrap();
            let has_duration = packet.read_bit().unwrap();
            let has_remaining = packet.read_bit().unwrap();
            let _has_time_mod = packet.read_bit().unwrap();
            let points = packet.read_bits(6).unwrap();
            let _estimated_points = packet.read_bits(6).unwrap();
            let _has_content_tuning = packet.read_bit().unwrap();
            packet.reset_bits();
            if has_cast_unit {
                let _caster = packet.read_packed_guid().unwrap();
            }
            if has_duration {
                let _duration = packet.read_uint32().unwrap();
            }
            let remaining = has_remaining.then(|| packet.read_uint32().unwrap());
            for _ in 0..points {
                let _point = packet.read_float().unwrap();
            }
            packet_updates.push((spell_id, remaining));
        }
        let target = packet.read_packed_guid().unwrap();
        for (spell_id, remaining) in packet_updates {
            updates.push((target, spell_id, remaining));
        }
    }
    updates
}

#[tokio::test]
async fn selected_recovery_from_the_real_map_root_publishes_before_tick_tail() {
    let world = d3b_world_like_cpp(96_390);
    let player_guid = ObjectGuid::create_player(1, 96_391);
    let (mut session, send_rx, _input_peer) = d3b_session_like_cpp(
        &world,
        player_guid,
        "SelectedRecovery",
        Position::new(10.0, 10.0, 0.0, 0.0),
    )
    .await;
    let _ = send_rx.try_iter().collect::<Vec<_>>();
    let (slot, stale) = install_selected_ghost(&mut session);
    let handle = session
        .core()
        .player_handle_like_cpp
        .expect("Player handle");
    let loop_handle = spawn_selected_recovery_root(&world);
    let phase_rx = session.session_phase_receiver_like_cpp();
    let started = Instant::now();
    let mut recovery_count = 0;
    let mut observed_diff = 0;
    let mut observed_map_epoch = 0;

    while started.elapsed() < Duration::from_secs(5) && recovery_count == 0 {
        let request = tokio::time::timeout(Duration::from_secs(1), phase_rx.recv_async())
            .await
            .expect("canonical root requests a session phase")
            .expect("session phase rail remains connected");
        if let SessionPhaseRequestLikeCpp::PlayerRecovery(recovery) = &request {
            recovery_count += 1;
            observed_diff = recovery.admission.diff_ms;
            observed_map_epoch = recovery.admission.map_tick_epoch;
            assert!(
                observed_diff > 0,
                "the root's Recovery request carries its tick diff"
            );
            let state = world.canonical.lock().unwrap().tick_coordination_like_cpp();
            assert_eq!(
                state,
                wow_map::MapTickCoordinationStateLikeCpp::AwaitingSessions(
                    recovery.admission.map_tick_epoch
                ),
                "the real producer is paused at the Player recovery barrier"
            );
        }

        let catalogs = wow_world::session::SessionHandlerCatalogsLikeCpp::default();
        assert!(
            session
                .run_requested_session_phase_like_cpp(request, &catalogs)
                .await
                .is_none()
        );

        if recovery_count != 0 {
            let remaining = world
                .canonical
                .lock()
                .unwrap()
                .with_player_like_cpp(handle, |player| {
                    let auras = &player.unit().subsystems().auras;
                    let application = auras.runtime_application_like_cpp(slot).unwrap();
                    assert_eq!(application.applied_at, stale);
                    assert!(!auras.visible_auras_to_update.contains(&slot));
                    application.duration_remaining
                })
                .unwrap();
            assert_eq!(remaining, 60_000 - observed_diff as i32);
            let packets = send_rx.try_iter().collect::<Vec<_>>();
            assert_eq!(
                selected_updates(&packets),
                [(player_guid, GHOST, Some((60_000 - observed_diff) as u32))],
                "the actual Recovery callee publishes one direct source AuraUpdate before replying"
            );
            assert_eq!(
                world.canonical.lock().unwrap().tick_coordination_like_cpp(),
                wow_map::MapTickCoordinationStateLikeCpp::AwaitingSessions(observed_map_epoch),
                "the root has not resumed its map tail before source publication is observed"
            );
        }
    }

    assert_eq!(
        recovery_count, 1,
        "one selected Player recovery request ran"
    );
    assert!(observed_diff > 0);

    let second_started = Instant::now();
    let mut second_diff = 0;
    let mut second_epoch = 0;
    while second_started.elapsed() < Duration::from_secs(5) && second_diff == 0 {
        let request = tokio::time::timeout(Duration::from_secs(1), phase_rx.recv_async())
            .await
            .expect("the canonical root requests the next map pass")
            .expect("session phase rail remains connected");
        if let SessionPhaseRequestLikeCpp::PlayerRecovery(recovery) = &request {
            second_diff = recovery.admission.diff_ms;
            second_epoch = recovery.admission.map_tick_epoch;
            assert!(second_diff > 0);
            assert_eq!(
                world.canonical.lock().unwrap().tick_coordination_like_cpp(),
                wow_map::MapTickCoordinationStateLikeCpp::AwaitingSessions(second_epoch),
                "the actual producer waits at the following Recovery barrier"
            );
        }
        let catalogs = wow_world::session::SessionHandlerCatalogsLikeCpp::default();
        assert!(
            session
                .run_requested_session_phase_like_cpp(request, &catalogs)
                .await
                .is_none()
        );
    }
    assert!(second_diff > 0, "a following Recovery pass ran");
    let remaining = world
        .canonical
        .lock()
        .unwrap()
        .with_player_like_cpp(handle, |player| {
            player
                .unit()
                .subsystems()
                .auras
                .runtime_application_like_cpp(slot)
                .unwrap()
                .duration_remaining
        })
        .unwrap();
    assert_eq!(
        remaining,
        60_000 - observed_diff as i32 - second_diff as i32
    );
    assert!(
        selected_updates(&send_rx.try_iter().collect::<Vec<_>>()).is_empty(),
        "the next positive countdown has no pending aura delta and sends no frame"
    );
    assert_eq!(
        world.canonical.lock().unwrap().tick_coordination_like_cpp(),
        wow_map::MapTickCoordinationStateLikeCpp::AwaitingSessions(second_epoch),
        "the second Recovery consumer returns before the map tail"
    );
    loop_handle.abort();
    let _ = loop_handle.await;
}
