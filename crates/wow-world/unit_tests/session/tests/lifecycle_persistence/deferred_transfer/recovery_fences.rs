use super::*;
use std::sync::Arc;
use wow_core::Position;

pub(crate) fn install_worldport_ack_inputs(session: &mut WorldSession) {
    let identity = session
        .core
        .with_owned_player_like_cpp(|player| {
            (
                player.race_like_cpp(),
                player.class_like_cpp(),
                player.level_like_cpp(),
            )
        })
        .expect("worldport ACK fixture has its canonical Player");
    session.set_chr_classes_store(Arc::new(
        wow_data::character_progression::ChrClassesStore::from_entries([
            wow_data::character_progression::ChrClassesEntry {
                id: u32::from(identity.1),
                starting_level: 1,
                ..Default::default()
            },
        ]),
    ));
    session.set_player_stats(Arc::new(wow_data::PlayerStatsStore::from_entries([(
        identity,
        wow_data::PlayerLevelStats {
            strength: 10,
            agility: 10,
            stamina: 10,
            intellect: 10,
            spirit: 10,
            base_mana: 100,
        },
    )])));
    assert!(session.complete_represented_trait_config_authority_load_like_cpp([], true));
}

fn recovery_handoff_session() -> (
    WorldSession,
    flume::Sender<WorldPacket>,
    flume::Receiver<Vec<u8>>,
) {
    let (mut session, input, output) = make_session();
    let guid = ObjectGuid::create_player(1, 43_260);
    let old_position = Position::new(4.0, 5.0, 6.0, 0.5);
    session.set_player_guid(Some(guid));
    install_canonical_player_owner_for_test(&mut session, 571, 0);
    session.set_player_position_like_cpp(old_position);
    let maps = Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 571,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
        wow_data::MapEntry {
            id: 1,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ]));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    session.set_map_store(Arc::clone(&maps));
    session.set_spell_store(Arc::new(wow_data::SpellStore::new()));
    install_worldport_ack_inputs(&mut session);

    let (safe_locs, _) = wow_data::WorldSafeLocStore::from_rows_like_cpp(
        [wow_data::WorldSafeLocRow {
            id: 900,
            map_id: 1,
            x: 90.0,
            y: 91.0,
            z: 92.0,
            facing_degrees: 0.0,
        }],
        &maps,
    );
    session.set_world_safe_loc_store_like_cpp(Arc::new(safe_locs));
    session.set_graveyard_store(Arc::new(wow_data::GraveyardStore::default()));

    session
        .with_owned_player_mut_like_cpp(|player| {
            assert!(player.begin_resurrection_operation(
                wow_entities::PlayerResurrectionOperation::SpiritHealer {
                    corpse_graveyard: Some(900),
                },
                wow_entities::PlayerResurrectionPhase::PlayerSaveApplied,
            ));
        })
        .unwrap();
    assert!(session.set_pending_teleport_like_cpp(Some((571, old_position))));
    assert!(session.set_represented_far_teleport_pending_like_cpp(true));
    session
        .with_owned_player_mut_like_cpp(|player| {
            assert_eq!(player.defer_save_if_transfer_pending_like_cpp(), Some(true));
        })
        .unwrap();
    session.set_state(SessionState::Transfer);
    assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
    (session, input, output)
}

#[tokio::test]
async fn recovery_fences_registered_worldport_leaves_new_postsave_far_transfer_pending() {
    let (mut session, _input, output) = recovery_handoff_session();
    let catalogs = session.session_handler_catalogs_for_test_like_cpp();
    let response = crate::session::registry::get_handler(ClientOpcodes::WorldPortResponse)
        .expect("registered WorldPortResponse handler");

    (response.handler)(&mut session, &catalogs, WorldPacket::new_empty()).await;

    assert_eq!(session.state(), SessionState::Transfer);
    let transfer = crate::session::hub_ref(&session)
        .player_teleport_state_snapshot_like_cpp()
        .expect("new post-save transfer state");
    assert!(
        transfer.far_pending,
        "the new destination needs its own ACK"
    );
    assert_eq!(
        transfer.far_destination.map(|destination| destination.0),
        Some(1)
    );
    assert!(transfer.post_add.is_none());
    assert_eq!(
        session.pending_teleport_like_cpp().map(|pending| pending.0),
        Some(1)
    );
    assert!(
        output.try_recv().is_ok(),
        "the live transport receives the new far transfer"
    );
    assert_eq!(
        session.with_owned_player_like_cpp(|player| {
            player.resurrection_state_like_cpp().delayed_after_teleport
        }),
        Some(None),
        "the healer operation returned from TeleportTo and retained native transfer ownership"
    );
    assert_eq!(
        session.with_owned_player_like_cpp(|player| player.has_deferred_player_save_like_cpp()),
        Some(true),
        "the old pending save intent waits for the new far transfer's ACK"
    );
}

#[tokio::test]
async fn recovery_fences_disconnect_completes_new_postsave_far_transfer_before_save_owner() {
    let (mut session, input, output) = recovery_handoff_session();
    drop(input);
    drop(output);
    let catalogs = session.session_handler_catalogs_for_test_like_cpp();

    assert!(
        wow_world_character::finish_worldport_native_before_disconnect_like_cpp(
            &mut session,
            wow_world_entities::ResurrectionOperationCatalogs {
                item_guid_generator: catalogs.id_generators.item.as_ref(),
                creature_spawns: catalogs.creature_spawns.as_ref(),
                graveyards: Some(catalogs.graveyards.as_ref()),
            },
        )
        .await
    );

    let transfer = crate::session::hub_ref(&session)
        .player_teleport_state_snapshot_like_cpp()
        .expect("native transfer state after disconnect completion");
    assert!(!transfer.far_pending);
    assert!(transfer.post_add.is_none());
    assert_eq!(session.pending_teleport_like_cpp(), None);
    assert_eq!(
        session.with_owned_player_like_cpp(|player| player.unit().world().map_id()),
        Some(1),
        "the disconnect loop completed the far transfer admitted by healer recovery"
    );
    assert_eq!(
        session.with_owned_player_like_cpp(|player| {
            player.resurrection_state_like_cpp().delayed_after_teleport
        }),
        Some(None)
    );
    assert_eq!(
        session.with_owned_player_like_cpp(|player| player.has_deferred_player_save_like_cpp()),
        Some(true),
        "native disconnect completion leaves the durable save intent to its established owner"
    );
}
