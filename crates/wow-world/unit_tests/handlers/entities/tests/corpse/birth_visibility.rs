//! Corpse birth discovery, failure fences and ordinary rediscovery.

use super::*;
use wow_entities::{
    PlayerResurrectionContinuation, PlayerResurrectionOperation, PlayerResurrectionPhase,
};

#[tokio::test]
async fn corpse_birth_full_queue_stale_handle_and_wrong_instance_never_discover_or_finish() {
    for (index, mode) in ["full", "stale", "instance"].into_iter().enumerate() {
        let (mut session, output, _realm, owner, _) =
            dead_player_session_with_graveyard_like_cpp(9_280 + index as i64);
        let generators = session.id_generators_for_test_like_cpp();
        let corpse =
            wow_world_entities::create_corpse_like_cpp(&mut session, generators.item.as_ref())
                .await
                .unwrap();
        session
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player
                    .resurrection_state_mut_like_cpp()
                    .delayed_after_teleport = Some(PlayerResurrectionContinuation {
                    operation: PlayerResurrectionOperation::Repop,
                    phase: PlayerResurrectionPhase::CorpsePublished,
                });
            })
            .unwrap();
        let handle = session.core.player_handle_like_cpp.unwrap();
        let retired = if mode == "stale" {
            Some(
                session
                    .core
                    .canonical_map_manager
                    .as_ref()
                    .unwrap()
                    .lock()
                    .unwrap()
                    .retire_player_like_cpp(handle)
                    .unwrap(),
            )
        } else if mode == "instance" {
            let mut manager = session
                .core
                .canonical_map_manager
                .as_ref()
                .unwrap()
                .lock()
                .unwrap();
            manager.detach_player_like_cpp(handle).unwrap();
            manager.create_world_map(571, 1);
            manager
                .attach_player_like_cpp(
                    handle,
                    wow_map::MapKey::new(571, 1),
                    Position::new(1.0, 2.0, 3.0, 0.0),
                )
                .unwrap();
            None
        } else {
            while session.core.send_tx().try_send(vec![0xFF]).is_ok() {}
            None
        };
        assert!(
            !wow_world_entities::publish_corpse_birth_visibility_like_cpp(&mut session, corpse)
        );
        assert!(!session.core.client_visible_guids_like_cpp.contains(&corpse));
        assert_eq!(session.core.player_handle_like_cpp, Some(handle));
        assert_eq!(session.player_guid(), Some(owner));
        assert!(output.try_iter().all(|bytes| bytes == [0xFF]));
        let phase = if let Some(retired) = retired {
            retired.resurrection_state_like_cpp().delayed_after_teleport
        } else {
            session
                .core
                .with_owned_player_like_cpp(|player| {
                    player.resurrection_state_like_cpp().delayed_after_teleport
                })
                .unwrap()
        };
        assert_eq!(
            phase.unwrap().phase,
            PlayerResurrectionPhase::CorpsePublished
        );
    }
}

#[tokio::test]
async fn ordinary_corpse_scan_uses_birth_only_while_its_owner_is_publishing() {
    let (mut session, output, _realm, _, _) = dead_player_session_with_graveyard_like_cpp(9_282);
    let generators = session.id_generators_for_test_like_cpp();
    let corpse = wow_world_entities::create_corpse_like_cpp(&mut session, generators.item.as_ref())
        .await
        .unwrap();
    let data = {
        let manager = session
            .core
            .canonical_map_manager
            .as_ref()
            .unwrap()
            .lock()
            .unwrap();
        wow_world_core::entity_update_bridge::corpse_create_data_from_entity_like_cpp(
            manager
                .find_map(571, 0)
                .unwrap()
                .map()
                .get_typed_corpse(corpse)
                .unwrap(),
        )
    };
    let creatures = session.creature_spawn_catalogs_for_test_like_cpp();
    for birth in [true, false] {
        session
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player
                    .resurrection_state_mut_like_cpp()
                    .delayed_after_teleport = birth.then_some(PlayerResurrectionContinuation {
                    operation: PlayerResurrectionOperation::Repop,
                    phase: PlayerResurrectionPhase::CorpsePublished,
                });
            })
            .unwrap();
        session.core.client_visible_guids_like_cpp.remove(&corpse);
        wow_world_entities::force_update_visibility_with_catalogs_like_cpp(
            &mut session,
            &creatures,
        )
        .await;
        let expected = wow_packet::packets::update::UpdateObject::create_world_objects(
            vec![if birth {
                wow_packet::packets::update::UpdateObject::create_corpse_birth_block(data.clone())
            } else {
                wow_packet::packets::update::UpdateObject::create_corpse_block(data.clone())
            }],
            571,
        )
        .to_bytes();
        assert_eq!(output.try_iter().collect::<Vec<_>>(), vec![expected]);
        assert!(session.core.client_visible_guids_like_cpp.contains(&corpse));
        if birth {
            assert!(
                wow_world_entities::publish_corpse_birth_visibility_like_cpp(&mut session, corpse)
            );
            assert!(
                output.is_empty(),
                "a concurrent scanner already published the birth"
            );
        }
    }
}
