//! Immediate object-only Corpse discovery, outside the MapManager borrow.

use wow_core::ObjectGuid;
use wow_entities::{CorpseType, PlayerResurrectionPhase};
use wow_map::{MapKey, MapManager, ObjectVisibilityDestroyRecipientsLikeCpp, PlayerHandle};
use wow_packet::{
    ServerPacket,
    packets::update::{CorpseCreateData, UpdateObject},
};
use wow_world_core::session::{SessionHubHostLikeCpp, SessionState};

#[derive(Clone, Copy)]
pub(crate) enum CorpseBirthKind {
    Release,
    Bones,
}

/// Detached visibility facts from the same Map mutation that replaces a corpse.
pub(crate) struct CorpseVisibilityPublicationLikeCpp {
    owner: PlayerHandle,
    key: MapKey,
    revision: u64,
    kind: CorpseBirthKind,
    destroyed: Option<(ObjectGuid, Vec<(PlayerHandle, u64)>)>,
    birth: Option<(CorpseCreateData, Vec<(PlayerHandle, u64)>)>,
}

fn recipient_receipts(
    manager: &MapManager,
    key: MapKey,
    guids: impl IntoIterator<Item = ObjectGuid>,
) -> Vec<(PlayerHandle, u64)> {
    guids
        .into_iter()
        .filter_map(|guid| {
            let (handle, _) = manager.current_player_admission_like_cpp(guid)?;
            let (residence, revision) =
                manager.player_active_residence_revision_like_cpp(handle)?;
            (residence == key).then_some((handle, revision))
        })
        .collect()
}

pub(crate) fn prepare_corpse_visibility_like_cpp(
    manager: &MapManager,
    owner: PlayerHandle,
    corpse_guid: Option<ObjectGuid>,
    destroyed: Option<ObjectVisibilityDestroyRecipientsLikeCpp>,
    kind: CorpseBirthKind,
) -> Option<CorpseVisibilityPublicationLikeCpp> {
    let (key, revision) = manager.player_active_residence_revision_like_cpp(owner)?;
    let (in_world, phase) = manager.with_player_like_cpp(owner, |player| {
        let world = player.unit().world();
        (
            player.guid() == owner.guid()
                && world.object().is_in_world()
                && world.has_current_map()
                && MapKey::new(world.map_id(), world.instance_id()) == key,
            player
                .resurrection_state_like_cpp()
                .delayed_after_teleport
                .map(|continuation| continuation.phase),
        )
    })?;
    if !in_world
        || !match kind {
            CorpseBirthKind::Release => phase == Some(PlayerResurrectionPhase::CorpsePublished),
            CorpseBirthKind::Bones => matches!(
                phase,
                None | Some(
                    PlayerResurrectionPhase::BonesInFlight
                        | PlayerResurrectionPhase::RecoveryCorpseBonesInFlight
                        | PlayerResurrectionPhase::CorpseBonesInFlight
                )
            ),
        }
    {
        return None;
    }
    let map = manager.find_map(key.map_id, key.instance_id)?.map();
    let birth = if let Some(guid) = corpse_guid {
        let corpse = map.get_typed_corpse(guid)?;
        if corpse.data().owner != owner.guid()
            || !corpse.world().object().is_in_world()
            || !match kind {
                CorpseBirthKind::Release => {
                    map.corpse_guid_by_player_like_cpp(owner.guid()) == Some(guid)
                }
                CorpseBirthKind::Bones => {
                    corpse.corpse_type() == CorpseType::Bones
                        && corpse.world().object().is_new_object()
                }
            }
        {
            return None;
        }
        let recipients = recipient_receipts(
            manager,
            key,
            map.corpse_birth_visibility_recipients_like_cpp(guid),
        );
        if matches!(kind, CorpseBirthKind::Release)
            && !recipients.iter().any(|(handle, _)| *handle == owner)
        {
            return None;
        }
        Some((
            wow_world_core::entity_update_bridge::corpse_create_data_from_entity_like_cpp(corpse),
            recipients,
        ))
    } else {
        None
    };
    let destroyed = destroyed.map(|intent| {
        (
            intent.object_guid,
            recipient_receipts(manager, key, intent.recipient_guids),
        )
    });
    Some(CorpseVisibilityPublicationLikeCpp {
        owner,
        key,
        revision,
        kind,
        destroyed,
        birth,
    })
}

pub(crate) fn publish_corpse_visibility_like_cpp<H: SessionHubHostLikeCpp>(
    host: &H,
    prepared: &CorpseVisibilityPublicationLikeCpp,
) -> bool {
    let core = host.core();
    let Some(manager) = core.canonical_map_manager.as_ref() else {
        return false;
    };
    if core.player_handle_like_cpp != Some(prepared.owner)
        || core.player_guid() != Some(prepared.owner.guid())
        || !manager.lock().is_ok_and(|manager| {
            manager.player_active_residence_revision_like_cpp(prepared.owner)
                == Some((prepared.key, prepared.revision))
        })
    {
        return false;
    }
    let mut published = true;
    let mut failed_destroys = Vec::new();
    // Map::RemoveCorpse precedes AddToMap(bones). Consume its old intent before
    // a same-GUID birth can become known, so the deferred rail cannot erase it.
    if let Some((guid, recipients)) = &prepared.destroyed {
        let packet = UpdateObject::out_of_range_objects(vec![*guid], prepared.key.map_id as u16);
        for (handle, revision) in recipients {
            if !publish_to_recipient(host, prepared, *handle, *revision, *guid, &packet, false) {
                failed_destroys.push((*handle, *revision));
                published = false;
            }
        }
    }
    if let Some((data, recipients)) = &prepared.birth {
        let packet = UpdateObject::create_world_objects(
            vec![UpdateObject::create_corpse_birth_block(data.clone())],
            prepared.key.map_id as u16,
        );
        for (handle, revision) in recipients {
            // Retaining an old known GUID after a failed destroy cannot
            // authorize skipping that receiver's replacement CREATE.
            if failed_destroys.contains(&(*handle, *revision)) {
                continue;
            }
            let delivered =
                publish_to_recipient(host, prepared, *handle, *revision, data.guid, &packet, true);
            if matches!(prepared.kind, CorpseBirthKind::Bones) || *handle == prepared.owner {
                published &= delivered;
            }
        }
    }
    if published && matches!(prepared.kind, CorpseBirthKind::Bones) {
        let Ok(mut manager) = manager.lock() else {
            return false;
        };
        if manager.player_active_residence_revision_like_cpp(prepared.owner)
            != Some((prepared.key, prepared.revision))
        {
            return false;
        }
        if let Some((data, _)) = &prepared.birth {
            let Some(corpse) = manager
                .find_map_mut(prepared.key.map_id, prepared.key.instance_id)
                .and_then(|map| map.map_mut().get_typed_corpse_mut(data.guid))
            else {
                return false;
            };
            if corpse.corpse_type() != CorpseType::Bones
                || corpse.data().owner != prepared.owner.guid()
            {
                return false;
            }
            // Map.cpp:573-575 ends the birth scope after immediate fanout;
            // ordinary rediscovery and relogin must use CREATE1 again.
            corpse.world_mut().object_mut().set_is_new_object(false);
        }
    }
    published
}

fn publish_to_recipient<H: SessionHubHostLikeCpp>(
    host: &H,
    source: &CorpseVisibilityPublicationLikeCpp,
    handle: PlayerHandle,
    revision: u64,
    guid: ObjectGuid,
    packet: &UpdateObject,
    birth: bool,
) -> bool {
    let core = host.core();
    if handle == source.owner {
        // A kicked C++ socket skips self delivery; nearby observers still see
        // the native transition. Closed self delivery is not a save receipt.
        if core.state == SessionState::Disconnecting {
            return true;
        }
        let visible = &core.client_visible_guids_like_cpp;
        let enqueue = || core.try_send_packet(packet);
        return if birth {
            visible.publish_unseen_create_like_cpp(guid, enqueue)
        } else {
            visible.publish_known_destroy_like_cpp(guid, enqueue)
        };
    }
    let Some(registry) = core.player_registry() else {
        return true;
    };
    let Some((registration, visible)) = registry.selected_packet_recipient_at_residence(
        handle.guid(),
        handle,
        source.key,
        revision,
        None,
    ) else {
        return true;
    };
    let enqueue = || {
        registry
            .try_send_current_packet(registration, packet.to_bytes())
            .is_ok()
    };
    if birth {
        visible.publish_unseen_create_like_cpp(guid, enqueue)
    } else {
        visible.publish_known_destroy_like_cpp(guid, enqueue)
    }
}

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
    let prepared = {
        let Ok(manager) = manager.lock() else {
            return false;
        };
        prepare_corpse_visibility_like_cpp(
            &manager,
            owner,
            Some(corpse_guid),
            None,
            CorpseBirthKind::Release,
        )
    };
    prepared.is_some_and(|prepared| publish_corpse_visibility_like_cpp(host, &prepared))
}
