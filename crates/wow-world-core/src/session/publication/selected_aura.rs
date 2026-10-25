// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Immutable publication-time ownership for selected Player AuraUpdates.

use crate::session::{directory::PlayerRegistry, mailbox::SendPlayerAuraUpdateCommand};
use wow_core::ObjectGuid;
use wow_map::{MapKey, MapManager, PlayerHandle, PlayerResidenceLikeCpp};

#[derive(Debug, Clone, Copy)]
struct Recipient {
    guid: ObjectGuid,
    handle: PlayerHandle,
    map_key: MapKey,
    residence_revision: u64,
}

/// Recipient decision captured while the canonical MapManager guard is held.
/// Delivery never reselects map membership or source visibility.
#[derive(Debug)]
pub struct PreparedSelectedAuraUpdate {
    source_handle: PlayerHandle,
    source_guid: ObjectGuid,
    source_residence: Option<(MapKey, u64)>,
    observers: Vec<Recipient>,
    packet_bytes: Vec<u8>,
}

impl PreparedSelectedAuraUpdate {
    pub fn packet_bytes(&self) -> &[u8] {
        &self.packet_bytes
    }

    /// Deliver through the established durable rails, with the source Player
    /// first and without requiring its own GUID in HaveAtClient.
    pub fn deliver_to_registry(&self, registry: &PlayerRegistry) {
        if let Some((map_key, residence_revision)) = self.source_residence {
            enqueue(
                registry,
                Recipient {
                    guid: self.source_guid,
                    handle: self.source_handle,
                    map_key,
                    residence_revision,
                },
                self.source_guid,
                &self.packet_bytes,
                false,
            );
        }
        self.deliver_observers_to_registry(registry);
    }

    pub(super) fn deliver_observers_to_registry(&self, registry: &PlayerRegistry) {
        for observer in self.observers.iter().copied() {
            enqueue(
                registry,
                observer,
                self.source_guid,
                &self.packet_bytes,
                true,
            );
        }
    }
}

/// Freeze the same selected source and shared-vision recipients used by the
/// session publisher. The caller owns the MapManager guard for the whole
/// producer operation and supplies already-serialized packet bytes.
pub fn prepare_selected_aura_update(
    manager: &MapManager,
    source_handle: PlayerHandle,
    packet_bytes: Vec<u8>,
) -> Option<PreparedSelectedAuraUpdate> {
    let source_guid = source_handle.guid();
    let residence = manager
        .checked_player_residence_like_cpp(source_handle)
        .ok()?;
    let (source_residence, observers) = match residence {
        PlayerResidenceLikeCpp::Detached => (None, Vec::new()),
        PlayerResidenceLikeCpp::Active(source_key) => {
            let (active_key, source_revision) =
                manager.player_active_residence_revision_like_cpp(source_handle)?;
            if active_key != source_key {
                return None;
            }
            let map = manager
                .find_map(source_key.map_id, source_key.instance_id)?
                .map();
            let candidates = map.selected_aura_update_observers(source_guid);
            let mut observers = Vec::with_capacity(candidates.len());
            for guid in candidates {
                if guid == source_guid {
                    continue;
                }
                let Some((handle, admitted_revision)) =
                    manager.current_player_admission_like_cpp(guid)
                else {
                    continue;
                };
                let Some((map_key, residence_revision)) =
                    manager.player_active_residence_revision_like_cpp(handle)
                else {
                    continue;
                };
                if map_key != source_key
                    || residence_revision != admitted_revision
                    || !map
                        .get_typed_player(guid)
                        .is_some_and(|player| player.unit().world().object().is_in_world())
                {
                    continue;
                }
                observers.push(Recipient {
                    guid,
                    handle,
                    map_key,
                    residence_revision,
                });
            }
            (Some((source_key, source_revision)), observers)
        }
    };

    Some(PreparedSelectedAuraUpdate {
        source_handle,
        source_guid,
        source_residence,
        observers,
        packet_bytes,
    })
}

fn enqueue(
    registry: &PlayerRegistry,
    recipient: Recipient,
    source_guid: ObjectGuid,
    packet_bytes: &[u8],
    require_source_visibility: bool,
) {
    let Some((registration, committed_visibility)) =
        registry.selected_aura_recipient(recipient.guid, recipient.handle, recipient.map_key)
    else {
        return;
    };
    if require_source_visibility && !committed_visibility.contains(&source_guid) {
        // The producer's shared-vision selection is keyed by the source GUID,
        // not the observer's own GUID. Check it after the map guard as before.
        return;
    }
    let command = SendPlayerAuraUpdateCommand {
        recipient_handle: recipient.handle,
        map_key: recipient.map_key,
        residence_revision: recipient.residence_revision,
        committed_visibility,
        packet_bytes: packet_bytes.to_vec(),
    };
    let _ = registry.publish_current_player_aura_update(registration, command);
}
