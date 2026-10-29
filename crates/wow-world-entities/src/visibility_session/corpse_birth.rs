//! Immediate object-only Corpse discovery, outside the MapManager borrow.

use wow_core::ObjectGuid;
use wow_entities::PlayerResurrectionPhase;
use wow_packet::{ServerPacket, packets::update::UpdateObject};
use wow_world_core::session::{SessionHubHostLikeCpp, SessionState};

pub fn publish_corpse_birth_visibility_like_cpp<H: SessionHubHostLikeCpp>(
    host: &mut H,
    corpse_guid: ObjectGuid,
) -> bool {
    let core = host.core();
    let Some(owner) = core.player_handle_like_cpp else {
        return false;
    };
    if core.player_guid() != Some(owner.guid()) || core.state == SessionState::Disconnecting {
        return false;
    }
    let Some(manager) = core.canonical_map_manager.as_ref() else {
        return false;
    };
    let (key, source_revision, data, recipients) = {
        let Ok(manager) = manager.lock() else {
            return false;
        };
        let Some((key, source_revision)) = manager.player_active_residence_revision_like_cpp(owner)
        else {
            return false;
        };
        let Some(map) = manager
            .find_map(key.map_id, key.instance_id)
            .map(|entry| entry.map())
        else {
            return false;
        };
        let Some(corpse) = map.get_typed_corpse(corpse_guid) else {
            return false;
        };
        if corpse.data().owner != owner.guid()
            || map.corpse_guid_by_player_like_cpp(owner.guid()) != Some(corpse_guid)
            || !manager
                .with_player_like_cpp(owner, |player| {
                    player.unit().world().object().is_in_world()
                        && player
                            .resurrection_state_like_cpp()
                            .delayed_after_teleport
                            .is_some_and(|continuation| {
                                continuation.phase == PlayerResurrectionPhase::CorpsePublished
                            })
                })
                .unwrap_or(false)
        {
            return false;
        }
        let recipients = map
            .corpse_birth_visibility_recipients_like_cpp(corpse_guid)
            .into_iter()
            .filter_map(|guid| {
                let (handle, _) = manager.current_player_admission_like_cpp(guid)?;
                let (residence, revision) =
                    manager.player_active_residence_revision_like_cpp(handle)?;
                (residence == key).then_some((handle, revision))
            })
            .collect::<Vec<_>>();
        if !recipients.iter().any(|(handle, _)| *handle == owner) {
            return false;
        }
        (
            key,
            source_revision,
            wow_world_core::entity_update_bridge::corpse_create_data_from_entity_like_cpp(corpse),
            recipients,
        )
    };
    let packet = UpdateObject::create_world_objects(
        vec![UpdateObject::create_corpse_birth_block(data)],
        key.map_id as u16,
    );
    let current = manager.lock().is_ok_and(|manager| {
        manager.player_active_residence_revision_like_cpp(owner) == Some((key, source_revision))
    });
    if !current
        || !core
            .client_visible_guids_like_cpp
            .publish_unseen_create_like_cpp(corpse_guid, || core.try_send_packet(&packet))
    {
        return false;
    }
    if let Some(registry) = core.player_registry() {
        for (handle, revision) in recipients {
            if handle == owner {
                continue;
            }
            let Some((registration, visible)) = registry.selected_packet_recipient_at_residence(
                handle.guid(),
                handle,
                key,
                revision,
                None,
            ) else {
                continue;
            };
            visible.publish_unseen_create_like_cpp(corpse_guid, || {
                registry
                    .try_send_current_packet(registration, packet.to_bytes())
                    .is_ok()
            });
        }
    }
    true
}
