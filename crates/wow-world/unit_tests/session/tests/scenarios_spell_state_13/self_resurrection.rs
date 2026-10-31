//! Native SelfRes spell-effect scenarios.

use super::*;

#[derive(Default)]
struct SelfResFailureCorpsePersistence {
    deletes: std::sync::Mutex<Vec<u64>>,
}

impl wow_persistence::MapCorpsePersistencePortLikeCpp for SelfResFailureCorpsePersistence {
    fn load_map_corpses_like_cpp<'a>(
        &'a self,
        _request: wow_persistence::MapCorpseLoadRequestLikeCpp,
    ) -> wow_persistence::PersistenceFutureLikeCpp<'a, wow_persistence::MapCorpseLoadOutcomeLikeCpp>
    {
        Box::pin(async {
            wow_persistence::MapCorpseLoadOutcomeLikeCpp::Failed {
                reason: "SelfRes failure case does not load corpses".into(),
            }
        })
    }

    fn save_corpse_like_cpp<'a>(
        &'a self,
        _request: wow_persistence::CorpseSaveRequestLikeCpp,
    ) -> wow_persistence::PersistenceFutureLikeCpp<
        'a,
        wow_persistence::CorpsePersistenceMutationOutcomeLikeCpp,
    > {
        Box::pin(async { wow_persistence::CorpsePersistenceMutationOutcomeLikeCpp::Committed })
    }

    fn delete_corpse_like_cpp<'a>(
        &'a self,
        owner_guid: u64,
    ) -> wow_persistence::PersistenceFutureLikeCpp<
        'a,
        wow_persistence::CorpsePersistenceMutationOutcomeLikeCpp,
    > {
        self.deletes.lock().unwrap().push(owner_guid);
        Box::pin(async { wow_persistence::CorpsePersistenceMutationOutcomeLikeCpp::Committed })
    }
}

fn configure_native_self_resurrect_player_like_cpp(
    session: &mut WorldSession,
    guid: ObjectGuid,
    health: u32,
    max_health: u32,
) {
    configure_self_resurrect_canonical_player_like_cpp(session, guid, health, max_health);
    crate::player_inventory_test_fixture::complete_known_empty_player_inventory_like_cpp(session);
    crate::player_inventory_test_fixture::complete_known_empty_player_currencies_like_cpp(session);
    assert!(wow_world_spell::set_player_aura_authority_complete_like_cpp(session, true));
    session
        .mutate_canonical_player_like_cpp(|player| {
            player.add_self_res_spell(21_169);
            player.clear_data_changes();
        })
        .expect("canonical Player owns the SelfRes field");
    assert_eq!(session.visibility.last_visibility_pos_like_cpp(), None);
}

fn publish_native_resurrection_values_from_map_like_cpp(
    session: &mut WorldSession,
    player_guid: ObjectGuid,
    send_rx: &flume::Receiver<Vec<u8>>,
    expected: (u64, i32, i32, i32, i32),
) {
    let manager = session
        .core
        .canonical_map_manager
        .as_ref()
        .expect("canonical MapManager")
        .clone();
    {
        let mut manager = manager.lock().unwrap();
        let power_indexes = {
            let player = manager
                .find_map_mut(0, 0)
                .expect("current World map")
                .map_mut()
                .get_typed_player_mut(player_guid)
                .expect("native Player is owned by the current Map");
            let object = player.unit().world().object();
            assert!(object.is_in_world());
            assert!(
                object.is_object_updated(),
                "native setters mark the Map owner"
            );
            assert_eq!(
                session.visibility.last_visibility_pos_like_cpp(),
                Some(player.unit().world().position()),
                "C++ forced visibility completes before the Map publishes native fields"
            );
            [
                PowerType::Mana,
                PowerType::Rage,
                PowerType::Energy,
                PowerType::Focus,
            ]
            .map(|power| {
                player
                    .unit()
                    .get_power_index(power)
                    .expect("native Player power index")
            })
        };
        let _ = manager.update(1);
        let map = manager.find_map(0, 0).expect("current World map");
        let summary = map.last_send_object_updates_summary_like_cpp();
        let updates = summary.player_values_updates.as_slice();
        assert_eq!(updates.len(), 1, "Map captures one final Player snapshot");
        assert_eq!(updates[0].guid, player_guid);
        let values_update = &updates[0].values_update;
        let unit = values_update
            .unit_data
            .as_ref()
            .expect("Map snapshot contains native Unit fields");
        assert_eq!(unit.values.health, expected.0);
        for (index, value) in power_indexes
            .into_iter()
            .zip([expected.1, expected.2, expected.3, expected.4])
        {
            assert_eq!(unit.values.power[index], value);
        }
        let active = values_update
            .active_player_data
            .as_ref()
            .expect("Map snapshot carries the cleared SelfRes list");
        assert!(active.values.self_res_spells.is_empty());
        assert!(active.values.self_res_spells_update_mask.is_some());
        assert!(
            !map.map()
                .get_typed_player(player_guid)
                .unwrap()
                .unit()
                .world()
                .object()
                .is_object_updated(),
            "Map consumes native update masks once"
        );
    }

    assert_eq!(
        session
            .send_represented_player_unit_values_updates_from_last_map_send_object_updates_like_cpp(
            ),
        1,
        "the existing World consumer sends the Map-owned snapshot"
    );
    assert_eq!(
        drain_server_opcodes(send_rx),
        vec![ServerOpcodes::UpdateObject]
    );
    assert_eq!(
        session
            .send_represented_player_unit_values_updates_from_last_map_send_object_updates_like_cpp(
            ),
        0,
        "a second consumer pass cannot duplicate the consumed snapshot"
    );
}

fn configure_self_resurrect_without_inventory_authority_like_cpp(
    session: &mut WorldSession,
    guid: ObjectGuid,
) {
    configure_self_resurrect_canonical_player_like_cpp(session, guid, 0, 100);
    assert!(wow_world_spell::set_player_aura_authority_complete_like_cpp(session, true));
    session
        .mutate_canonical_player_like_cpp(|player| player.add_self_res_spell(21_169))
        .expect("canonical Player owns the SelfRes field");
}

fn self_resurrect_spell(spell_id: i32, damage: i32, misc_value: i32) -> wow_data::SpellInfo {
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
            effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_SELF_RESURRECT,
            effect_base_points: damage,
            effect_misc_value_1: misc_value,
            ..Default::default()
        }],
    }
}

fn install_self_resurrect_spell_like_cpp(
    session: &mut WorldSession,
    spell_id: i32,
    damage: i32,
    misc_value: i32,
) {
    let mut attributes = [0; 15];
    attributes[0] = wow_data::spell::attributes::SPELL_ATTR0_ALLOW_CAST_WHILE_DEAD as i32;
    let misc = wow_data::spell_db2::SpellMiscEntry {
        id: spell_id as u32,
        spell_id: spell_id as u32,
        attributes,
        ..Default::default()
    };
    let stores = wow_data::EffectiveCoreSpellDb2StoresLikeCpp::new(
        wow_data::spell_db2::SpellCategoriesStore::from_entries([]),
        wow_data::spell_db2::SpellMiscStore::from_entries([misc]),
        wow_data::spell_db2::SpellEffectDb2Store::from_entries([]),
        wow_data::spell_db2::SpellShapeshiftStore::from_entries([]),
        wow_data::spell_db2::SpellInterruptsStore::from_entries([]),
        wow_data::spell_db2::SpellCastTimesStore::from_entries([]),
        wow_data::spell_db2::SpellCooldownsStore::from_entries([]),
        wow_data::spell_db2::SpellCastingRequirementsStore::from_entries([]),
        wow_data::spell_db2::SpellPowerStore::from_entries([]),
        wow_data::spell_db2::SpellPowerDifficultyStore::from_entries([]),
    );
    let mut spell_store =
        crate::selected_spell_fixtures::canonical_spell_store([(spell_id as u32, 0)])
            .hydrate_effective_core_db2_like_cpp(stores);
    spell_store.insert(spell_id, self_resurrect_spell(spell_id, damage, misc_value));
    session.set_spell_store(Arc::new(spell_store));
}

async fn apply_native_self_resurrect_with_caster(
    session: &mut WorldSession,
    caster_guid: ObjectGuid,
) -> Result<(), &'static str> {
    let player_guid = session.player_guid().expect("fixture Player identity");
    let spell_id = 78_338_i32;
    install_self_resurrect_spell_like_cpp(session, spell_id, -35, 77);
    session
        .execute_spell_with_visual_and_target_data_with_metadata(
            spell_id,
            player_guid,
            ObjectGuid::EMPTY,
            wow_packet::packets::spell::SpellCastVisual::default(),
            wow_packet::packets::spell::SpellTargetData {
                flags: 0x2,
                unit: player_guid,
                ..Default::default()
            },
            wow_entities::SpellCastMetadata {
                actual_spell_difficulty_id_like_cpp: Some(0),
                caster_guid_override: Some(caster_guid),
                ..Default::default()
            },
        )
        .await
}

#[tokio::test]
async fn spell_self_resurrect_flat_case_sets_health_and_powers_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let (realm_tx, realm_rx) = flume::unbounded();
    session.install_realm_send_channel_for_test(realm_tx);
    let spell_id = 780_i32;
    let player_guid = ObjectGuid::create_player(1, 780);
    configure_native_self_resurrect_player_like_cpp(&mut session, player_guid, 0, 100);
    let save_port = crate::session::RecordingPlayerLifecyclePort::new(
        wow_persistence::PersistenceOutcomeLikeCpp::Applied { rows: 1 },
    );
    session.set_player_lifecycle_port_like_cpp(save_port.clone());
    install_self_resurrect_spell_like_cpp(&mut session, spell_id, -35, 77);

    session
        .execute_spell_with_visual_and_target_data(
            spell_id,
            player_guid,
            ObjectGuid::EMPTY,
            wow_packet::packets::spell::SpellCastVisual::default(),
            wow_packet::packets::spell::SpellTargetData::default(),
        )
        .await
        .expect("represented self-resurrect flat effect should execute");

    assert!(session.player_is_alive_like_cpp());
    assert_eq!(session.player_health_like_cpp(), 35);
    let powers = session
        .mutate_canonical_player_like_cpp(|player| {
            (
                player.unit().data().health,
                player.get_power(PowerType::Mana),
                player.get_power(PowerType::Rage),
                player.get_power(PowerType::Energy),
                player.get_power(PowerType::Focus),
                player.self_res_spells().to_vec(),
            )
        })
        .unwrap();
    assert_eq!(powers, (35, 77, 0, 100, 0, Vec::<i32>::new()));
    let packets = drain_server_packet_bytes(&send_rx);
    let opcodes = packets
        .iter()
        .filter_map(|bytes| WorldPacket::from_bytes(bytes).server_opcode())
        .collect::<Vec<_>>();
    assert_eq!(
        opcodes,
        vec![
            ServerOpcodes::SpellGo,
            ServerOpcodes::MoveSetLandWalk,
            ServerOpcodes::MoveUnroot,
        ],
        "native field changes remain pending until Map::SendObjectUpdates"
    );
    let spell_go = packets
        .iter()
        .find(|bytes| {
            WorldPacket::from_bytes(bytes).server_opcode() == Some(ServerOpcodes::SpellGo)
        })
        .expect("self-resurrection publishes SpellGo");
    let target = decode_spell_go_target_data_like_cpp(spell_go, spell_id);
    assert_eq!(
        target.flags, 0,
        "the caster-only cast has no explicit target flags"
    );
    assert!(
        target.unit.is_empty(),
        "the caster-only cast has no explicit Unit"
    );
    assert_eq!(
        decode_spell_go_hit_target_count_like_cpp(spell_go, spell_id),
        0,
        "caster-only SELF_RESURRECT publishes an empty SpellGo hit list"
    );
    publish_native_resurrection_values_from_map_like_cpp(
        &mut session,
        player_guid,
        &send_rx,
        (35, 77, 0, 100, 0),
    );
    let realm = realm_rx.try_iter().collect::<Vec<_>>();
    assert_eq!(realm.len(), 5);
    assert_eq!(
        u16::from_le_bytes([realm[0][0], realm[0][1]]),
        ServerOpcodes::DeathReleaseLoc as u16
    );
    for (packet, (power, value)) in realm[1..].iter().zip([
        (PowerType::Mana, 77),
        (PowerType::Rage, 0),
        (PowerType::Energy, 100),
        (PowerType::Focus, 0),
    ]) {
        assert_eq!(
            *packet,
            wow_packet::packets::combat::PowerUpdate {
                guid: player_guid,
                powers: vec![(value, power as u8)],
            }
            .to_bytes()
        );
    }
    assert_eq!(
        save_port.character_save_count(),
        0,
        "NoCorpse skips SaveToDB"
    );
    assert_eq!(
        session.with_owned_player_like_cpp(|player| player
            .resurrection_state_like_cpp()
            .delayed_after_teleport),
        Some(None),
        "the completed native operation clears its same-owner continuation"
    );
    let generators = session.id_generators_for_test_like_cpp();
    let creatures = session.creature_spawn_catalogs_for_test_like_cpp();
    assert!(
        wow_world_entities::continue_player_resurrection_after_teleport(
            &mut session,
            wow_world_entities::ResurrectionOperationCatalogs {
                item_guid_generator: generators.item.as_ref(),
                creature_spawns: &creatures,
                graveyards: None,
            },
        )
        .await
    );
    assert!(
        realm_rx.try_recv().is_err(),
        "completed SelfRes must not republish powers"
    );
}

#[tokio::test]
async fn spell_self_resurrect_percent_case_uses_max_health_and_mana_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let (realm_tx, realm_rx) = flume::unbounded();
    session.install_realm_send_channel_for_test(realm_tx);
    let spell_id = 781_i32;
    let player_guid = ObjectGuid::create_player(1, 781);
    configure_native_self_resurrect_player_like_cpp(&mut session, player_guid, 0, 400);
    install_self_resurrect_spell_like_cpp(&mut session, spell_id, 25, 999);

    session
        .execute_spell(spell_id, player_guid)
        .await
        .expect("represented self-resurrect percent effect should execute");

    assert!(session.player_is_alive_like_cpp());
    assert_eq!(session.player_health_like_cpp(), 100);
    let powers = session
        .mutate_canonical_player_like_cpp(|player| {
            (
                player.unit().data().health,
                player.get_power(PowerType::Mana),
                player.get_power(PowerType::Rage),
                player.get_power(PowerType::Energy),
                player.get_power(PowerType::Focus),
                player.self_res_spells().to_vec(),
            )
        })
        .unwrap();
    assert_eq!(powers, (100, 50, 0, 100, 0, Vec::<i32>::new()));
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![
            ServerOpcodes::SpellGo,
            ServerOpcodes::MoveSetLandWalk,
            ServerOpcodes::MoveUnroot,
        ],
        "no intermediate HP-zero VALUES snapshot is sent before Map::SendObjectUpdates"
    );
    publish_native_resurrection_values_from_map_like_cpp(
        &mut session,
        player_guid,
        &send_rx,
        (100, 50, 0, 100, 0),
    );
    let realm = realm_rx.try_iter().collect::<Vec<_>>();
    assert_eq!(realm.len(), 5);
    assert_eq!(
        u16::from_le_bytes([realm[0][0], realm[0][1]]),
        ServerOpcodes::DeathReleaseLoc as u16
    );
    for (packet, (power, value)) in realm[1..].iter().zip([
        (PowerType::Mana, 50),
        (PowerType::Rage, 0),
        (PowerType::Energy, 100),
        (PowerType::Focus, 0),
    ]) {
        assert_eq!(
            *packet,
            wow_packet::packets::combat::PowerUpdate {
                guid: player_guid,
                powers: vec![(value, power as u8)],
            }
            .to_bytes()
        );
    }
}

#[tokio::test]
async fn spell_self_resurrect_skips_alive_player_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 782_i32;
    let player_guid = ObjectGuid::create_player(1, 782);
    configure_native_self_resurrect_player_like_cpp(&mut session, player_guid, 40, 100);
    install_self_resurrect_spell_like_cpp(&mut session, spell_id, -35, 77);

    session
        .execute_spell(spell_id, player_guid)
        .await
        .expect("alive player self-resurrect should be a C++ no-op");

    assert!(session.player_is_alive_like_cpp());
    assert_eq!(session.player_health_like_cpp(), 40);
    assert_eq!(
        session.with_owned_player_like_cpp(|player| player.self_res_spells().to_vec()),
        Some(vec![21_169]),
        "an alive no-op does not enter ResurrectPlayer or clear SelfRes values"
    );
    let powers = session
        .mutate_canonical_player_like_cpp(|player| {
            (
                player.unit().data().health,
                player.get_power(PowerType::Mana),
                player.get_power(PowerType::Rage),
                player.get_power(PowerType::Energy),
                player.get_power(PowerType::Focus),
            )
        })
        .unwrap();
    assert_eq!(powers, (40, 25, 50, 30, 40));
    assert_eq!(drain_server_opcodes(&send_rx), vec![ServerOpcodes::SpellGo]);
}

#[tokio::test]
async fn native_self_resurrect_actual_creature_caster_is_a_noop() {
    let (mut session, _, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 7831);
    configure_native_self_resurrect_player_like_cpp(&mut session, player_guid, 0, 100);
    let canonical = session.core.canonical_map_manager.as_ref().unwrap().clone();
    let creature_guid = test_creature_guid(78_311);
    add_canonical_test_creature_on_map(
        &canonical,
        creature_guid,
        78_312,
        Position::new(10.0, 20.0, 30.0, 0.0),
        0,
        0,
        0,
    );
    let _ = drain_server_opcodes(&send_rx);

    apply_native_self_resurrect_with_caster(&mut session, creature_guid)
        .await
        .expect("Source ignores a non-Player caster");

    assert!(!session.player_is_alive_like_cpp());
    assert_eq!(session.player_health_like_cpp(), 0);
    assert_eq!(
        session.with_owned_player_like_cpp(|player| player
            .resurrection_state_like_cpp()
            .delayed_after_teleport),
        Some(None)
    );
    assert_eq!(drain_server_opcodes(&send_rx), vec![ServerOpcodes::SpellGo]);
}

#[tokio::test]
async fn native_self_resurrect_player_out_of_world_is_a_noop() {
    let (mut session, _, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 7832);
    configure_native_self_resurrect_player_like_cpp(&mut session, player_guid, 0, 100);
    session
        .mutate_canonical_player_like_cpp(|player| {
            let _ = player.unit_mut().remove_from_world_like_cpp();
        })
        .expect("out-of-world Player remains the exact canonical owner");
    let _ = drain_server_opcodes(&send_rx);

    apply_native_self_resurrect_with_caster(&mut session, player_guid)
        .await
        .expect("Source ignores a Player that is not in world");

    assert!(!session.player_is_alive_like_cpp());
    assert_eq!(session.player_health_like_cpp(), 0);
    assert_eq!(
        session.with_owned_player_like_cpp(|player| player
            .resurrection_state_like_cpp()
            .delayed_after_teleport),
        Some(None)
    );
    assert_eq!(drain_server_opcodes(&send_rx), vec![ServerOpcodes::SpellGo]);
    assert!(
        !session
            .with_owned_player_like_cpp(|player| player
                .unit()
                .world()
                .object()
                .is_need_notify(wow_entities::ObjectNotifyFlags::VISIBILITY_CHANGED))
            .unwrap()
    );
}

#[tokio::test]
async fn native_self_resurrect_missing_owner_fails_stopped_without_bones_or_save() {
    let (mut session, _, send_rx) = make_session();
    let corpse_port = std::sync::Arc::new(SelfResFailureCorpsePersistence::default());
    session.set_map_corpse_persistence_port_like_cpp(corpse_port.clone());
    let save_port = crate::session::RecordingPlayerLifecyclePort::new(
        wow_persistence::PersistenceOutcomeLikeCpp::Applied { rows: 1 },
    );
    session.set_player_lifecycle_port_like_cpp(save_port.clone());
    let generators = session.id_generators_for_test_like_cpp();
    let creature_spawns = session.creature_spawn_catalogs_for_test_like_cpp();

    assert!(
        wow_world_entities::self_resurrect_player_like_cpp(
            &mut session,
            wow_world_entities::ResurrectionOperationCatalogs {
                item_guid_generator: generators.item.as_ref(),
                creature_spawns: &creature_spawns,
                graveyards: None,
            },
            35,
            77,
        )
        .await
        .is_err()
    );

    assert_eq!(session.core.state, SessionState::Disconnecting);
    assert!(session.with_owned_player_like_cpp(|_| ()).is_none());
    assert!(corpse_port.deletes.lock().unwrap().is_empty());
    assert_eq!(save_port.character_save_count(), 0);
    assert!(send_rx.is_empty());
}

#[tokio::test]
async fn native_self_resurrect_obtain_failure_kicks_and_retains_before_bones_or_save() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 78_333_i32;
    let player_guid = ObjectGuid::create_player(1, 7833);
    configure_self_resurrect_without_inventory_authority_like_cpp(&mut session, player_guid);
    let corpse_guid =
        ObjectGuid::create_world_object(wow_core::guid::HighGuid::Corpse, 0, 1, 0, 0, 0, 78_337);
    let mut corpse = wow_entities::Corpse::new_at(wow_entities::CorpseType::ResurrectablePve, 0);
    corpse.world_mut().object_mut().create(corpse_guid);
    corpse.world_mut().set_map(0, 0).expect("corpse map");
    corpse.world_mut().relocate(Position::ZERO);
    corpse.world_mut().object_mut().add_to_world();
    corpse.set_owner_guid(player_guid);
    session
        .core
        .canonical_map_manager
        .as_ref()
        .unwrap()
        .lock()
        .unwrap()
        .find_map_mut(0, 0)
        .unwrap()
        .map_mut()
        .add_map_object_record_to_map_like_cpp(
            wow_entities::MapObjectRecord::new_corpse(corpse).expect("corpse record"),
        )
        .expect("corpse inserted on the Player's active map");
    let corpse_port = std::sync::Arc::new(SelfResFailureCorpsePersistence::default());
    session.set_map_corpse_persistence_port_like_cpp(corpse_port.clone());
    let save_port = crate::session::RecordingPlayerLifecyclePort::new(
        wow_persistence::PersistenceOutcomeLikeCpp::Applied { rows: 1 },
    );
    session.set_player_lifecycle_port_like_cpp(save_port.clone());
    install_self_resurrect_spell_like_cpp(&mut session, spell_id, -35, 77);
    let (cast_id, residence_revision) =
        wow_world_spell::allocate_player_cast_identity_like_cpp(&session, spell_id)
            .expect("the admitted Player allocates its canonical Cast GUID and residence stamp");
    let mut metadata = wow_entities::SpellCastMetadata::default();
    metadata.prepared_residence_revision = residence_revision;
    assert!(
        session.set_active_spell_cast_like_cpp(Some(wow_entities::SpellCastState {
            spell_id,
            target_guid: player_guid,
            target_data: Default::default(),
            cast_id,
            cast_start_time: Instant::now(),
            cast_time_ms: 0,
            spell_visual: Default::default(),
            metadata,
        }))
    );

    session.tick_active_spell_cast().await;

    assert_eq!(session.core.state, SessionState::Disconnecting);
    let native_player_state = session
        .with_owned_player_like_cpp(|player| {
            (
                player.unit().death_state(),
                player.unit().is_alive(),
                player.unit().data().health,
            )
        })
        .expect("native resurrection retains the canonical Player");
    assert_eq!(
        native_player_state,
        (wow_constants::DeathState::Alive, true, 0),
        "ResurrectPlayer(0.0) establishes Source Alive while deferred restoration keeps health zero"
    );
    let continuation = session
        .with_owned_player_like_cpp(|player| {
            player.resurrection_state_like_cpp().delayed_after_teleport
        })
        .flatten()
        .expect("the failed native operation retains its exact Player owner");
    assert_eq!(
        continuation.operation,
        wow_entities::PlayerResurrectionOperation::SelfRes {
            health: 35,
            mana: 77,
        }
    );
    assert_eq!(
        continuation.phase,
        wow_entities::PlayerResurrectionPhase::ObtainSpellsInFlight
    );
    assert!(
        corpse_port.deletes.lock().unwrap().is_empty(),
        "an obtain failure cannot enter the dependent corpse-bones phase"
    );
    assert_eq!(save_port.character_save_count(), 0);
    assert!(
        !drain_server_opcodes(&send_rx).contains(&ServerOpcodes::CastFailed),
        "Disconnecting suppresses the driver's synthetic NotKnown CastFailed"
    );
}

#[tokio::test]
async fn native_self_resurrect_current_native_setters_cap_flat_values() {
    let (mut session, _, _) = make_session();
    let spell_id = 78_335_i32;
    let player_guid = ObjectGuid::create_player(1, 7835);
    configure_native_self_resurrect_player_like_cpp(&mut session, player_guid, 0, 100);
    session
        .mutate_canonical_player_like_cpp(|player| {
            player.unit_mut().set_max_power(PowerType::Mana, 60);
        })
        .unwrap();
    install_self_resurrect_spell_like_cpp(&mut session, spell_id, -200, 150);

    session
        .execute_spell(spell_id, player_guid)
        .await
        .expect("flat values are applied by the current native setters");

    let (health, mana) = session
        .with_owned_player_like_cpp(|player| {
            (
                player.unit().data().health,
                player.get_power(PowerType::Mana),
            )
        })
        .unwrap();
    assert_eq!((health, mana), (100, 60));
}

#[tokio::test]
async fn native_self_resurrect_int_min_stops_before_resurrection_mutation() {
    let (mut session, _, _) = make_session();
    let spell_id = 78_336_i32;
    let player_guid = ObjectGuid::create_player(1, 7836);
    configure_native_self_resurrect_player_like_cpp(&mut session, player_guid, 0, 100);
    install_self_resurrect_spell_like_cpp(&mut session, spell_id, i32::MIN, 77);

    assert!(session.execute_spell(spell_id, player_guid).await.is_err());

    assert!(!session.player_is_alive_like_cpp());
    assert_eq!(session.player_health_like_cpp(), 0);
    assert_eq!(
        session.with_owned_player_like_cpp(|player| player
            .resurrection_state_like_cpp()
            .delayed_after_teleport),
        Some(None),
        "undefined Source negation is rejected before beginning the operation"
    );
}
