use super::*;
use std::sync::Arc;
use wow_data::spell_db2::{
    SpellCategoriesEntry, SpellCooldownsEntry, SpellEffectDb2Entry, SpellMiscEntry,
    SpellReagentsEntry,
};
use wow_constants::ServerOpcodes;
use wow_persistence::PersistenceOutcomeLikeCpp;

const SELF_RES_TEST_SPELL: i32 = 61_043;
const REAGENT_ITEM: u32 = 17_030;
const DIFFICULTY: u8 = 3;

#[path = "registered_self_res_cast_item_provenance.rs"]
mod cast_item_provenance;

#[path = "registered_self_res/onpickup.rs"]
mod onpickup;

fn move_recovery_player_to_difficulty_three(session: &mut WorldSession) {
    let guid = session.player_guid().expect("recovery Player GUID");
    let handle = session
        .core
        .player_handle_like_cpp
        .expect("canonical Player owner");
    let position = session.player_position_like_cpp().expect("Player position");
    let manager = session
        .core
        .canonical_map_manager
        .as_ref()
        .cloned()
        .expect("canonical MapManager");
    let target = wow_map::MapKey::new(571, 10_000 + u32::from(DIFFICULTY));
    let mut manager = manager.lock().expect("canonical MapManager");
    manager
        .detach_player_like_cpp(handle)
        .expect("detach recovery Player from its initial map");
    manager.create_map_entry(
        target.map_id,
        target.instance_id,
        DIFFICULTY,
        wow_map::ManagedMapKind::Dungeon {
            has_reset_schedule: false,
        },
    );
    manager
        .attach_player_like_cpp(handle, target, position)
        .expect("attach recovery Player to difficulty-three map");
    assert_eq!(
        manager
            .find_map(target.map_id, target.instance_id)
            .expect("difficulty-three map")
            .map()
            .spawn_mode(),
        DIFFICULTY
    );
    drop(manager);
    assert_eq!(session.core.current_map_difficulty_id_like_cpp(), DIFFICULTY);
    assert_eq!(guid, handle.guid());
}

fn install_exact_self_res_spell(session: &mut WorldSession) {
    install_exact_test_spell(
        session,
        SELF_RES_TEST_SPELL,
        wow_data::spell::spell_effect_types::SPELL_EFFECT_SELF_RESURRECT,
        20,
        false,
    );
}

fn install_exact_test_spell(
    session: &mut WorldSession,
    spell_id: i32,
    spell_effect_type: u32,
    effect_base_points: i32,
    has_hit_delay: bool,
) {
    session.set_difficulty_store(Arc::new(wow_data::DifficultyStore::from_entries([
        wow_data::DifficultyEntry {
            id: u32::from(DIFFICULTY),
            instance_type: crate::session::MAP_RAID_LIKE_CPP,
            flags: 0,
            fallback_difficulty_id: 0,
            toggle_difficulty_id: 0,
        },
    ])));
    session.set_map_difficulty_store(Arc::new(
        wow_data::MapDifficultyStore::from_entries([wow_data::MapDifficultyEntry {
            id: 1,
            message: String::new(),
            map_id: 571,
            difficulty_id: DIFFICULTY,
            lock_id: 0,
            reset_interval: 0,
            max_players: 0,
            flags: 0,
        }]),
    ));
    session.set_spell_category_store(Arc::new(wow_data::SpellCategoryStore::from_entries([
        wow_data::SpellCategoryEntry {
            id: 3,
            name: "self-res test".into(),
            flags: 0,
            uses_per_week: 0,
            max_charges: 0,
            charge_recovery_time: 0,
            type_mask: 0,
        },
    ])));
    session.set_spell_class_options_store(Arc::new(
        wow_data::SpellClassOptionsStore::from_entries([]),
    ));

    let mut attributes = [0; 15];
    if spell_effect_type == wow_data::spell::spell_effect_types::SPELL_EFFECT_SELF_RESURRECT {
        attributes[0] = wow_data::spell::attributes::SPELL_ATTR0_ALLOW_CAST_WHILE_DEAD as i32;
    }
    let spell_effect = SpellEffectDb2Entry {
        id: 1,
        difficulty_id: i32::from(DIFFICULTY),
        effect_index: 0,
        effect: spell_effect_type,
        effect_amplitude: 0.0,
        effect_attributes: 0,
        effect_aura: 0,
        effect_aura_period: 0,
        effect_base_points,
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
        effect_misc_value: [0; 2],
        effect_radius_index: [0; 2],
        effect_spell_class_mask: [0; 4],
        implicit_target: [0; 2],
        spell_id: spell_id as u32,
    };
    let effective = wow_data::EffectiveCoreSpellDb2StoresLikeCpp::new(
        wow_data::spell_db2::SpellCategoriesStore::from_entries([SpellCategoriesEntry {
            id: 1,
            difficulty_id: DIFFICULTY,
            category: 3,
            defense_type: 0,
            dispel_type: 0,
            mechanic: 0,
            prevention_type: 0,
            start_recovery_category: 0,
            charge_category: 0,
            spell_id: spell_id as u32,
        }]),
        wow_data::spell_db2::SpellMiscStore::from_entries([SpellMiscEntry {
            id: 1,
            attributes,
            difficulty_id: DIFFICULTY,
            speed: if has_hit_delay { 10.0 } else { 0.0 },
            spell_id: spell_id as u32,
            ..Default::default()
        }]),
        wow_data::spell_db2::SpellEffectDb2Store::from_entries([spell_effect]),
        wow_data::spell_db2::SpellShapeshiftStore::from_entries([]),
        wow_data::spell_db2::SpellInterruptsStore::from_entries([]),
        wow_data::spell_db2::SpellCastTimesStore::from_entries([]),
        wow_data::spell_db2::SpellCooldownsStore::from_entries([SpellCooldownsEntry {
            id: 1,
            difficulty_id: DIFFICULTY,
            category_recovery_time: 10_000,
            recovery_time: 10_000,
            spell_id: spell_id as u32,
            ..Default::default()
        }]),
        wow_data::spell_db2::SpellCastingRequirementsStore::from_entries([]),
        wow_data::spell_db2::SpellPowerStore::from_entries([]),
        wow_data::spell_db2::SpellPowerDifficultyStore::from_entries([]),
    );
    let mut spells = crate::selected_spell_fixtures::canonical_spell_store([(
        spell_id as u32,
        DIFFICULTY,
    )])
    .hydrate_effective_core_db2_like_cpp(effective);
    assert!(spells.get(spell_id).is_none());
    spells.install_effective_cast_costs_like_cpp(wow_data::SpellCastCostStoreLikeCpp::from_effective_rows_like_cpp(
        [SpellReagentsEntry {
            id: 1,
            spell_id,
            reagent: [REAGENT_ITEM as i32, 0, 0, 0, 0, 0, 0, 0],
            reagent_count: [1, 0, 0, 0, 0, 0, 0, 0],
        }],
        [],
    ));
    assert!(spells.contains_spell_info_exact_like_cpp(spell_id as u32, DIFFICULTY));
    assert!(!spells.contains_spell_info_exact_like_cpp(spell_id as u32, 0));
    assert!(spells.get(spell_id).is_none());
    session.set_spell_store(Arc::new(spells));
    session.set_serverside_spell_store(Arc::new(
        wow_data::ServersideSpellStoreLikeCpp::default(),
    ));
    session.set_item_effect_store(Arc::new(wow_data::ItemEffectStore::from_entries([])));
    session.set_creature_aggro_config_like_cpp(CreatureAggroConfigLikeCpp {
        spell_x_spell_visual_store: Some(Arc::new(
            wow_data::SpellXSpellVisualStore::from_entries([]),
        )),
        ..Default::default()
    });
    session.mark_represented_character_spell_cooldowns_loaded_like_cpp();
}

fn install_reagent_item(session: &mut WorldSession, guid_counter: i64) -> ObjectGuid {
    crate::session::tests::install_stackable_test_item_template(session, REAGENT_ITEM, 20);
    let guid = ObjectGuid::create_item(1, guid_counter);
    let owner = session.player_guid().expect("recovery Player GUID");
    session.insert_inventory_item_like_cpp(
        23,
        crate::session::InventoryItem {
            guid,
            entry_id: REAGENT_ITEM,
            db_guid: guid.counter() as u64,
            inventory_type: None,
        },
    );
    let item = session.make_inventory_item_object(
        guid,
        REAGENT_ITEM,
        owner,
        1,
        0,
        ItemContext::None,
        23,
    );
    session.insert_inventory_item_object(item);
    session
        .with_owned_player_mut_like_cpp(|player| {
            player.store_top_level_item(23, guid).expect("backpack slot 23");
        })
        .expect("canonical Player owns the reagent");
    guid
}

fn registered_self_res_packet() -> wow_packet::WorldPacket {
    let mut bytes = (ClientOpcodes::SelfRes as u16).to_le_bytes().to_vec();
    bytes.extend_from_slice(&SELF_RES_TEST_SPELL.to_le_bytes());
    wow_packet::WorldPacket::from_bytes(&bytes)
}

fn outgoing_frames(send_rx: &flume::Receiver<Vec<u8>>) -> Vec<Vec<u8>> {
    std::iter::from_fn(|| send_rx.try_recv().ok()).collect()
}

fn outgoing_opcodes(send_rx: &flume::Receiver<Vec<u8>>) -> Vec<ServerOpcodes> {
    outgoing_frames(send_rx)
        .into_iter()
        .filter_map(|bytes| wow_packet::WorldPacket::from_bytes(&bytes).server_opcode())
        .collect()
}

fn cast_failed_reason(bytes: &[u8]) -> i32 {
    let mut packet = wow_packet::WorldPacket::from_bytes(bytes);
    assert_eq!(
        packet.read_uint16().expect("CastFailed opcode"),
        ServerOpcodes::CastFailed as u16
    );
    packet.read_packed_guid().expect("cast ID");
    packet.read_int32().expect("spell ID");
    wow_packet::packets::spell::SpellCastVisual::read(&mut packet).expect("cast visual");
    packet.read_int32().expect("CastFailed reason")
}

fn setup_registered_self_res(
    with_reagent: bool,
    guid_counter: i64,
) -> (
    WorldSession,
    flume::Sender<WorldPacket>,
    flume::Receiver<Vec<u8>>,
    Arc<RecordingPlayerLifecyclePort>,
    Arc<SelfResCorpsePersistence>,
    Option<ObjectGuid>,
) {
    let (mut session, _admission, packet_tx, send_rx) = logged_in_recovery_session();
    install_recovery_save_snapshot_inputs(&mut session);
    move_recovery_player_to_difficulty_three(&mut session);
    install_exact_self_res_spell(&mut session);
    crate::player_inventory_test_fixture::complete_known_empty_player_inventory_like_cpp(
        &mut session,
    );
    crate::player_inventory_test_fixture::complete_known_empty_player_currencies_like_cpp(
        &mut session,
    );
    assert!(wow_world_spell::set_player_aura_authority_complete_like_cpp(
        &mut session,
        true,
    ));
    session
        .with_owned_player_mut_like_cpp(|player| {
            let unit = player.unit_mut();
            unit.set_max_health(100);
            unit.set_health(0);
            unit.set_power_index(wow_constants::PowerType::Mana, Some(0));
            unit.set_power_index(wow_constants::PowerType::Rage, Some(1));
            unit.set_power_index(wow_constants::PowerType::Energy, Some(3));
            unit.set_power_index(wow_constants::PowerType::Focus, Some(4));
            unit.set_max_power(wow_constants::PowerType::Mana, 200);
            unit.set_power(wow_constants::PowerType::Mana, 25);
            unit.set_max_power(wow_constants::PowerType::Rage, 100);
            unit.set_power(wow_constants::PowerType::Rage, 50);
            unit.set_max_power(wow_constants::PowerType::Energy, 100);
            unit.set_power(wow_constants::PowerType::Energy, 30);
            unit.set_max_power(wow_constants::PowerType::Focus, 100);
            unit.set_power(wow_constants::PowerType::Focus, 40);
            unit.set_death_state(wow_constants::DeathState::Dead);
            player.add_self_res_spell(SELF_RES_TEST_SPELL);
        })
        .expect("canonical recovery Player remains installed");
    let reagent = with_reagent.then(|| install_reagent_item(&mut session, guid_counter));
    let save = RecordingPlayerLifecyclePort::new(PersistenceOutcomeLikeCpp::Applied { rows: 1 });
    let corpse = Arc::new(SelfResCorpsePersistence::default());
    session.set_player_lifecycle_port_like_cpp(save.clone());
    session.set_map_corpse_persistence_port_like_cpp(corpse.clone());
    (session, packet_tx, send_rx, save, corpse, reagent)
}

fn add_current_map_corpse(session: &mut WorldSession, counter: i64) -> ObjectGuid {
    let player_guid = session.player_guid().expect("recovery Player GUID");
    let (map_id, instance_id) = session.core.current_runtime_map_key_like_cpp();
    let corpse_guid = ObjectGuid::create_world_object(
        HighGuid::Corpse,
        0,
        1,
        map_id,
        instance_id,
        0,
        counter,
    );
    let mut corpse = wow_entities::Corpse::new_at(wow_entities::CorpseType::ResurrectablePve, 0);
    corpse.world_mut().object_mut().create(corpse_guid);
    corpse
        .world_mut()
        .set_map(u32::from(map_id), instance_id)
        .expect("corpse current map");
    corpse.world_mut().relocate(Position::ZERO);
    corpse.world_mut().object_mut().add_to_world();
    corpse.set_owner_guid(player_guid);
    session
        .core
        .canonical_map_manager
        .as_ref()
        .expect("canonical MapManager")
        .lock()
        .expect("canonical MapManager")
        .find_map_mut(u32::from(map_id), instance_id)
        .expect("current difficulty map")
        .map_mut()
        .add_map_object_record_to_map_like_cpp(
            wow_entities::MapObjectRecord::new_corpse(corpse).expect("corpse record"),
        )
        .expect("corpse inserted on the current map");
    corpse_guid
}

#[tokio::test]
async fn registered_self_res_casts_with_cost_and_saves_restored_native_state_before_ack() {
    let (mut session, packet_tx, send_rx, save, corpse_port, reagent) =
        setup_registered_self_res(true, 61_044);
    assert!(!packet_tx.is_disconnected());
    let reagent = reagent.expect("positive case has a physical reagent");
    let player_guid = session.player_guid().expect("recovery Player GUID");
    let corpse_guid = add_current_map_corpse(&mut session, 61_045);
    save.hold_character_save();
    let manager = session
        .core
        .canonical_map_manager
        .as_ref()
        .expect("canonical MapManager")
        .clone();
    let handle = session.core.player_handle_like_cpp.expect("Player handle");
    let captured_now_secs = (wow_world_core::session::unix_now_ms_like_cpp() / 1_000) as i64;
    let catalogs = SessionHandlerCatalogsLikeCpp::default();
    let mut dispatch = Box::pin(session.dispatch_packet(
        &catalogs,
        registered_self_res_packet(),
    ));
    tokio::select! {
        result = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            save.wait_for_character_save(),
        ) => assert!(result.is_ok(), "registered SelfRes did not reach the held full-save port"),
        _ = &mut dispatch => panic!("registered SelfRes completed before the held full-save ACK"),
    }

    assert_eq!(save.character_save_count(), 1);
    let requests = save.character_save_requests();
    assert_eq!(requests.len(), 1);
    let request = &requests[0];
    assert_eq!(request.character.health, 20);
    assert_eq!(request.character.powers.as_ref().unwrap()[0], 40);
    let cooldown = request
        .spell_cooldowns
        .as_ref()
        .and_then(|rows| {
            rows.iter()
                .find(|row| row.spell_id == SELF_RES_TEST_SPELL as u32)
        })
        .expect("the same full-save DTO includes the SelfRes cooldown");
    assert_eq!(cooldown.category_id, 3);
    assert!(cooldown.cooldown_end_unix_secs > captured_now_secs);
    assert!(request.inventory_items.as_ref().is_some_and(|rows| {
        rows.iter().any(|row| {
            row.item_guid == reagent.counter() as u64
                && row.state == wow_persistence::PlayerItemSaveStateLikeCpp::Removed
        })
    }));
    let (health, mana, item_state) = manager
        .lock()
        .expect("canonical MapManager")
        .with_player_like_cpp(handle, |player| {
            (
                player.unit().data().health,
                player.unit().get_power(wow_constants::PowerType::Mana),
                player
                    .inventory_runtime_like_cpp()
                    .item_objects()
                    .get(&reagent)
                    .map(wow_entities::Item::update_state),
            )
        })
        .expect("canonical Player remains alive during the held save");
    assert_eq!((health, mana), (20, 40));
    assert_eq!(item_state, Some(wow_constants::ItemUpdateState::Removed));
    assert_eq!(
        corpse_port.deletes.lock().unwrap().as_slice(),
        &[player_guid.counter() as u64]
    );
    assert!(manager
        .lock()
        .unwrap()
        .find_map(571, 10_000 + u32::from(DIFFICULTY))
        .unwrap()
        .map()
        .get_typed_corpse(corpse_guid)
        .is_none());

    save.release_character_save();
    dispatch.as_mut().await;
    drop(dispatch);
    assert_eq!(save.character_save_count(), 1);
    assert!(session.player_is_alive_like_cpp());
    assert!(session
        .with_owned_player_like_cpp(|player| player.self_res_spells().is_empty())
        .expect("canonical Player remains installed"));
    assert_eq!(
        session.with_owned_player_like_cpp(|player| {
            player
                .resurrection_state_like_cpp()
                .delayed_after_teleport
        }),
        Some(None),
        "the full-save ACK clears the exact SelfRes continuation"
    );
    assert!(manager
        .lock()
        .unwrap()
        .with_player_like_cpp(handle, |player| {
            !player
                .inventory_runtime_like_cpp()
                .item_objects()
                .contains_key(&reagent)
        })
        .expect("canonical Player remains installed"));
    assert!(outgoing_opcodes(&send_rx).contains(&ServerOpcodes::SpellGo));
}

#[tokio::test]
async fn registered_self_res_missing_reagent_rejects_before_go_and_consumes_list_entry() {
    let (mut session, packet_tx, send_rx, save, _corpse, _reagent) =
        setup_registered_self_res(false, 61_046);
    assert!(!packet_tx.is_disconnected());
    session
        .dispatch_packet(
            &SessionHandlerCatalogsLikeCpp::default(),
            registered_self_res_packet(),
        )
        .await;
    let frames = outgoing_frames(&send_rx);
    assert_eq!(
        frames
            .iter()
            .filter_map(|frame| wow_packet::WorldPacket::from_bytes(frame).server_opcode())
            .collect::<Vec<_>>(),
        vec![ServerOpcodes::CastFailed]
    );
    assert_eq!(
        cast_failed_reason(&frames[0]),
        wow_constants::SpellCastResult::Reagents as i32
    );
    assert_eq!(save.character_save_count(), 0);
    assert!(session
        .with_owned_player_like_cpp(|player| player.self_res_spells().is_empty())
        .unwrap());
    assert_eq!(session.player_health_like_cpp(), 0);
}

#[tokio::test]
async fn registered_self_res_active_cooldown_rejects_before_effect_and_consumption() {
    let (mut session, packet_tx, send_rx, save, _corpse, reagent) =
        setup_registered_self_res(true, 61_047);
    assert!(!packet_tx.is_disconnected());
    let reagent = reagent.expect("cooldown case has a physical reagent");
    session
        .with_owned_player_mut_like_cpp(|player| {
            let history = &mut player.unit_mut().subsystems_mut().spells.history;
            history.start_cooldown(
                wow_world_core::session::unix_now_ms_like_cpp(),
                SELF_RES_TEST_SPELL as u32,
                0,
                10_000,
                3,
                10_000,
                false,
            );
        })
        .expect("canonical spell history remains installed");
    session
        .dispatch_packet(
            &SessionHandlerCatalogsLikeCpp::default(),
            registered_self_res_packet(),
        )
        .await;
    let frames = outgoing_frames(&send_rx);
    assert_eq!(
        frames
            .iter()
            .filter_map(|frame| wow_packet::WorldPacket::from_bytes(frame).server_opcode())
            .collect::<Vec<_>>(),
        vec![ServerOpcodes::CastFailed]
    );
    assert_eq!(
        cast_failed_reason(&frames[0]),
        wow_constants::SpellCastResult::NotReady as i32
    );
    assert_eq!(save.character_save_count(), 0);
    assert_eq!(session.player_health_like_cpp(), 0);
    assert!(session
        .with_owned_player_like_cpp(|player| {
            player.self_res_spells().is_empty()
                && player
                    .inventory_runtime_like_cpp()
                    .item_objects()
                    .contains_key(&reagent)
        })
        .unwrap());
}

#[tokio::test]
async fn registered_self_res_without_a_corpse_keeps_cost_and_cooldown_dirty_without_saving() {
    let (mut session, packet_tx, send_rx, save, corpse, reagent) =
        setup_registered_self_res(true, 61_048);
    assert!(!packet_tx.is_disconnected());
    let reagent = reagent.expect("no-corpse case has a physical reagent");
    session
        .dispatch_packet(
            &SessionHandlerCatalogsLikeCpp::default(),
            registered_self_res_packet(),
        )
        .await;
    assert_eq!(save.character_save_count(), 0);
    assert!(corpse.deletes.lock().unwrap().is_empty());
    assert!(session.player_is_alive_like_cpp());
    assert_eq!(session.player_health_like_cpp(), 20);
    assert!(session
        .with_owned_player_like_cpp(|player| {
            player.self_res_spells().is_empty()
                && player.unit().get_power(wow_constants::PowerType::Mana) == 40
                && player
                    .inventory_runtime_like_cpp()
                    .item_objects()
                    .get(&reagent)
                    .is_some_and(|item| {
                        item.update_state() == wow_constants::ItemUpdateState::Removed
                    })
                && player.unit().subsystems().spells.history.cooldowns.contains_key(
                    &(SELF_RES_TEST_SPELL as u32)
                )
        })
        .unwrap());
    let opcodes = outgoing_opcodes(&send_rx);
    assert!(opcodes.contains(&ServerOpcodes::SpellGo));
}
