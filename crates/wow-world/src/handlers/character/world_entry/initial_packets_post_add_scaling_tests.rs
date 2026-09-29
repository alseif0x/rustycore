use std::sync::Arc;

use crate::session::{SessionState, WorldSession};
use crate::test_fixtures::{
    CollectionLoadPortLikeCpp, install_canonical_player_owner_for_test,
    set_equipment_set_guid_generator_for_test, set_loaded_player_identity_like_cpp,
    set_player_position_for_test,
};
use wow_constants::ServerOpcodes;
use wow_core::guid::HighGuid;
use wow_core::{EquipmentSetGuidGeneratorLikeCpp, ObjectGuid, ObjectGuidGenerator, Position};
use wow_data::character_progression::{ChrClassesEntry, ChrClassesStore};
use wow_data::{
    MapEntry, MapStore, OctRegenHpGameTableLikeCpp, PlayerLevelStats, PlayerStatsStore,
    RegenGameTablesLikeCpp, RegenHpPerSptGameTableLikeCpp, RegenMpPerSptEntryLikeCpp,
    RegenMpPerSptGameTableLikeCpp,
};
use wow_packet::WorldPacket;
use wow_persistence::{
    PlayerInitialWorldStateRowsLikeCpp, PlayerInitialWorldStatesLoadOutcomeLikeCpp,
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
    session.set_item_guid_generator_like_cpp(Arc::new(ObjectGuidGenerator::new(HighGuid::Item, 1)));
    set_equipment_set_guid_generator_for_test(
        &mut session,
        Arc::new(EquipmentSetGuidGeneratorLikeCpp::new(1)),
    );
    (session, send_rx)
}

fn set_priest_level80_stats(session: &mut WorldSession, base_mana: u32, intellect: u16) {
    session.set_spell_store(Arc::new(wow_data::SpellStore::new()));
    session.set_player_stats(Arc::new(PlayerStatsStore::from_entries([(
        (1, 5, 80),
        PlayerLevelStats {
            strength: 10,
            agility: 10,
            stamina: 10,
            intellect,
            spirit: 30,
            base_mana,
        },
    )])));
    session.set_chr_classes_store(Arc::new(ChrClassesStore::from_entries([chr_class_entry(
        5, 0,
    )])));
    let mut regen_columns = [0.0; RegenMpPerSptGameTableLikeCpp::VALUE_COLUMN_COUNT];
    regen_columns[4] = 0.003345; // TrinityCore Data/gt/RegenMPPerSpt.txt, level 80 Priest
    let regen_row = RegenMpPerSptEntryLikeCpp::from_columns(regen_columns);
    let mut regen_rows = vec![RegenMpPerSptEntryLikeCpp::default(); 79];
    regen_rows.push(regen_row);
    session.set_regen_game_tables(Arc::new(RegenGameTablesLikeCpp::from_tables(
        RegenMpPerSptGameTableLikeCpp::from_rows(regen_rows),
        RegenHpPerSptGameTableLikeCpp::from_rows([]),
        OctRegenHpGameTableLikeCpp::from_rows([]),
    )));
}

fn chr_class_entry(id: u32, cinematic_sequence_id: u16) -> ChrClassesEntry {
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

fn drain_server_opcodes(send_rx: &flume::Receiver<Vec<u8>>) -> Vec<ServerOpcodes> {
    let mut opcodes = Vec::new();
    while let Ok(bytes) = send_rx.try_recv() {
        let packet = WorldPacket::from_bytes(&bytes);
        if let Some(opcode) = packet.server_opcode() {
            opcodes.push(opcode);
        }
    }
    opcodes
}

// Output closure and cancellation cases are controlled Rust harness scenarios; they test
// Rust disconnect completion and do not establish matching C++ cancellation semantics.
#[derive(Clone, Copy, PartialEq)]
enum OutputClosure {
    Never,
    BeforeAck,
    DuringWorldStateRead,
    CancelDuringWorldStateRead,
    RetainedBeforeZone,
}

#[tokio::test]
async fn worldport_scales_items_only_after_post_add_initialization() {
    assert_post_add_scaling(true, OutputClosure::Never).await;
}

#[tokio::test]
async fn login_post_add_applies_destination_item_scaling() {
    assert_post_add_scaling(false, OutputClosure::Never).await;
}

#[tokio::test]
async fn worldport_does_not_report_logged_in_when_post_add_delivery_closes() {
    assert_post_add_scaling(true, OutputClosure::DuringWorldStateRead).await;
}

#[tokio::test]
async fn worldport_finishes_native_effects_when_self_create_delivery_is_closed() {
    assert_post_add_scaling(true, OutputClosure::BeforeAck).await;
}

#[tokio::test]
async fn cancelled_worldport_finishes_native_effects_before_disconnect_save() {
    assert_post_add_scaling(true, OutputClosure::CancelDuringWorldStateRead).await;
}

#[tokio::test]
async fn retained_worldport_before_zone_finishes_native_effects_before_disconnect_save() {
    assert_post_add_scaling(true, OutputClosure::RetainedBeforeZone).await;
}

async fn assert_post_add_scaling(worldport: bool, closure: OutputClosure) {
    let (mut session, send_rx) = make_session_with_send_capacity(64);
    let guid = ObjectGuid::create_player(1, 42);
    let position = Position::new(1.0, 2.0, 3.0, 0.5);
    session.set_map_store(Arc::new(MapStore::from_entries([MapEntry {
        id: 571,
        instance_type: wow_data::map::MAP_COMMON,
        expansion_id: 0,
        parent_map_id: -1,
        cosmetic_parent_map_id: -1,
        flags1: 0,
        flags2: 0x40,
    }])));
    session.set_player_guid(Some(guid));
    install_canonical_player_owner_for_test(&mut session, 571, 0);
    set_loaded_player_identity_like_cpp(&mut session, 571, 1, 5, 80, 0);
    set_player_position_for_test(&mut session, position);
    set_priest_level80_stats(&mut session, 1000, 20);
    assert!(session.player_stat_changes_like_cpp().is_some());
    assert!(session.complete_represented_trait_config_authority_load_like_cpp([], true));
    let canonical = session.canonical_map_manager.as_ref().unwrap().clone();
    let port = CollectionLoadPortLikeCpp::for_initial_world_states([]);
    let observed = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let observation = observed.clone();
    let mut send_rx = Some(send_rx);
    if closure == OutputClosure::BeforeAck {
        drop(send_rx.take());
    }
    let close_during_read = if closure == OutputClosure::DuringWorldStateRead {
        send_rx.take()
    } else {
        None
    };
    port.enqueue_initial_world_state_outcome(Box::pin(async move {
        {
            let manager = canonical.lock().unwrap();
            let player = manager
                .find_map(571, 0)
                .unwrap()
                .map()
                .get_typed_player(guid)
                .unwrap();
            assert!(
                !player.gameplay_state().using_pvp_item_levels,
                "scaling must not precede post-add InitWorldStates/auras/phase"
            );
        }
        observation.store(true, std::sync::atomic::Ordering::SeqCst);
        drop(close_during_read);
        if closure == OutputClosure::CancelDuringWorldStateRead {
            std::future::pending::<()>().await;
        }
        PlayerInitialWorldStatesLoadOutcomeLikeCpp {
            templates: PlayerInitialWorldStateRowsLikeCpp::Loaded(vec![]),
            saved_values: PlayerInitialWorldStateRowsLikeCpp::Loaded(vec![]),
        }
    }));
    session.set_player_lifecycle_port_like_cpp(port);
    if worldport {
        assert!(
            session.schedule_represented_resurrection_after_teleport_like_cpp(
                wow_entities::PlayerResurrectionRequestLikeCpp {
                    resurrecter: guid,
                    map_id: 571,
                    position,
                    health: 100,
                    mana: 50,
                    aura: 0,
                }
            )
        );
        assert!(session.set_pending_teleport_like_cpp(Some((571, position))));
        assert!(session.set_represented_far_teleport_pending_like_cpp(true));
        session.set_state(SessionState::Transfer);
        if closure == OutputClosure::RetainedBeforeZone {
            assert!(session.set_represented_far_teleport_pending_like_cpp(false));
            assert!(session.set_pending_teleport_like_cpp(None));
            assert!(session.begin_worldport_post_add_like_cpp(571, position));
            assert!(!session.begin_worldport_post_add_like_cpp(571, position));
            set_player_position_for_test(&mut session, Position::new(4.0, 5.0, 6.0, 0.0));
            assert!(!session.finish_worldport_native_before_disconnect_like_cpp());
            assert!(!session.represented_using_pvp_item_levels_like_cpp());
            assert!(
                session
                    .represented_delayed_resurrection_after_teleport_like_cpp()
                    .is_some()
            );
            set_player_position_for_test(&mut session, position);
            drop(send_rx.take());
            session.kick("controlled retained post-add before zone");
            session.save_disconnect_player_to_db_like_cpp().await;
        } else if closure == OutputClosure::CancelDuringWorldStateRead {
            assert!(
                tokio::time::timeout(
                    std::time::Duration::from_millis(50),
                    session.handle_world_port_response(WorldPacket::new_empty()),
                )
                .await
                .is_err()
            );
            assert!(observed.load(std::sync::atomic::Ordering::SeqCst));
            assert!(!session.represented_far_teleport_pending_like_cpp());
            assert!(session.pending_teleport_like_cpp().is_none());
            drop(send_rx.take());
            session.kick("controlled worldport cancellation before disconnect save");
            session.save_disconnect_player_to_db_like_cpp().await;
        } else {
            session
                .handle_world_port_response(WorldPacket::new_empty())
                .await;
        }
        assert_eq!(
            session.state(),
            if closure != OutputClosure::Never {
                SessionState::Disconnecting
            } else {
                SessionState::LoggedIn
            }
        );
        assert!(
            session
                .represented_delayed_resurrection_after_teleport_like_cpp()
                .is_none()
        );
        let pet_resummons = session.temporary_pet_resummon_requests_like_cpp();
        assert_eq!(pet_resummons, 1);
        assert!(session.finish_worldport_native_before_disconnect_like_cpp());
        assert_eq!(
            session.temporary_pet_resummon_requests_like_cpp(),
            pet_resummons
        );
    } else {
        session
            .send_initial_packets_after_add_to_map(guid, &position, 571, false)
            .await;
    }
    assert_eq!(
        observed.load(std::sync::atomic::Ordering::SeqCst),
        closure != OutputClosure::RetainedBeforeZone
    );
    assert!(
        session.represented_using_pvp_item_levels_like_cpp(),
        "the shared post-add phase must apply destination scaling for login and worldport"
    );
    if let Some(send_rx) = send_rx {
        assert!(drain_server_opcodes(&send_rx).contains(&ServerOpcodes::InitWorldStates));
    }
}
