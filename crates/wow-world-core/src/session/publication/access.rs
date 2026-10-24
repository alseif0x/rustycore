// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use std::time::Instant;

use crate::entity_update_bridge::player_values_update_to_update_object;
use crate::session::SessionCore;
use crate::session::mailbox::{SendIfVisibleLikeCppCommand, SessionCommand};
use wow_core::{ObjectGuid, Position};
use wow_entities::PlayerValuesUpdate;
use wow_entities::VISIBILITY_RADIUS_LIKE_CPP as VISIBILITY_RADIUS;

/// Borrowed access to the session's established packet publication channel.
pub struct PacketPublicationAccessLikeCpp<'a> {
    core: &'a SessionCore,
}

impl SessionCore {
    /// Build a borrowed capability for this session's packet channel.
    pub fn packet_publication_access_like_cpp(&self) -> PacketPublicationAccessLikeCpp<'_> {
        PacketPublicationAccessLikeCpp { core: self }
    }
}

impl PacketPublicationAccessLikeCpp<'_> {
    pub fn reborrow_like_cpp(&self) -> PacketPublicationAccessLikeCpp<'_> {
        PacketPublicationAccessLikeCpp { core: self.core }
    }

    /// Project and send Item values using the current map at publication time.
    pub fn publish_item_values_update_like_cpp(
        &self,
        guid: ObjectGuid,
        update: &wow_entities::ItemValuesUpdate,
    ) -> bool {
        let Some(packet) = crate::entity_update_bridge::item_values_update_to_update_object(
            guid,
            self.core.player_map_id_like_cpp(),
            update,
        ) else {
            return false;
        };
        self.core.send_packet(&packet)
    }

    /// Project and send container values using the current map at publication time.
    pub fn publish_bag_values_update_like_cpp(
        &self,
        guid: ObjectGuid,
        update: &wow_entities::BagValuesUpdate,
    ) -> bool {
        let Some(packet) = crate::entity_update_bridge::bag_values_update_to_update_object(
            guid,
            self.core.player_map_id_like_cpp(),
            update,
        ) else {
            return false;
        };
        self.core.send_packet(&packet)
    }

    /// Publish one packet through the session's established send operation.
    pub fn send_packet<P: wow_packet::ServerPacket>(&self, packet: &P) -> bool {
        self.core.send_packet(packet)
    }

    /// C++ `ChatHandler::SendSysMessage` to this session's own player, through
    /// the session's established send operation. The trusted-module adapter
    /// applies validated self-message effects through this capability (#1444).
    pub fn send_system_message_like_cpp(&self, text: &str) {
        self.core.send_system_message_like_cpp(text);
    }

    /// Build and publish player values through Core's map-addressed update path.
    pub fn publish_player_values_update_like_cpp(
        &self,
        guid: ObjectGuid,
        update: &PlayerValuesUpdate,
    ) -> bool {
        let Some(packet) =
            player_values_update_to_update_object(guid, self.core.player_map_id_like_cpp(), update)
        else {
            return false;
        };
        self.core.send_packet(&packet)
    }

    /// Publish one selected Ghost/Wisp AuraUpdate to its Player first, then
    /// commit the map-selected observer packets to the existing durable rails.
    /// Source and recipient ownership are validated together under MapManager;
    /// directory access and packet enqueue happen only after that guard drops.
    pub fn publish_selected_aura_update(
        &self,
        source_guid: ObjectGuid,
        packet_bytes: &[u8],
    ) -> bool {
        let Some(handle) = self.core.player_handle_like_cpp else {
            #[cfg(any(test, feature = "test-fixtures"))]
            if self.core.player_guid() == Some(source_guid) {
                self.core.send_raw_packet(packet_bytes);
                return true;
            }
            return false;
        };
        if handle.guid() != source_guid || self.core.player_guid() != Some(source_guid) {
            return false;
        }
        let Some(manager) = self.core.canonical_map_manager.as_ref().cloned() else {
            return false;
        };

        let selected = manager.lock().ok().and_then(|manager| {
            let residence = manager.checked_player_residence_like_cpp(handle).ok()?;
            match residence {
                wow_map::PlayerResidenceLikeCpp::Detached => Some(None),
                wow_map::PlayerResidenceLikeCpp::Active(source_key) => {
                    let (active_key, _source_revision) =
                        manager.player_active_residence_revision_like_cpp(handle)?;
                    if active_key != source_key {
                        return None;
                    }
                    let managed = manager.find_map(source_key.map_id, source_key.instance_id)?;
                    let map = managed.map();
                    let observers = map.selected_aura_update_observers(source_guid);
                    let mut recipients = Vec::with_capacity(observers.len());
                    for guid in observers {
                        if guid == source_guid {
                            // C++ processes shared vision for the source
                            // Player, then excludes it at final delivery.
                            continue;
                        }
                        let Some((recipient_handle, residence_revision)) =
                            manager.current_player_admission_like_cpp(guid)
                        else {
                            continue;
                        };
                        let Some((map_key, current_revision)) =
                            manager.player_active_residence_revision_like_cpp(recipient_handle)
                        else {
                            continue;
                        };
                        if map_key != source_key || current_revision != residence_revision {
                            continue;
                        }
                        if !map
                            .get_typed_player(guid)
                            .is_some_and(|player| player.unit().world().object().is_in_world())
                        {
                            continue;
                        }
                        recipients.push((guid, recipient_handle, map_key, current_revision));
                    }
                    Some(Some(recipients))
                }
            }
        });

        let Some(recipients) = selected else {
            return false;
        };

        // Match Player::SendMessageToSet: the source's direct self packet is
        // enqueued before any observer handoff. The selected recipients are
        // already committed and are not re-evaluated at drain time.
        self.core.send_raw_packet(packet_bytes);
        let Some(recipients) = recipients else {
            return true;
        };
        let Some(registry) = self.core.player_registry() else {
            return true;
        };
        for (guid, recipient_handle, map_key, residence_revision) in recipients {
            let Some((registration, committed_visibility)) =
                registry.selected_aura_recipient(guid, recipient_handle, map_key)
            else {
                continue;
            };
            if !committed_visibility.contains(&source_guid) {
                continue;
            }
            let command = crate::session::mailbox::SendPlayerAuraUpdateCommand {
                recipient_handle,
                map_key,
                residence_revision,
                committed_visibility,
                packet_bytes: packet_bytes.to_vec(),
            };
            let _ = registry.publish_current_player_aura_update(registration, command);
        }
        true
    }

    /// Publish one packet through the established realm connection.
    pub fn send_packet_realm(&self, packet: &impl wow_packet::ServerPacket) {
        self.core.send_packet_realm(packet);
    }

    /// Wait for instance packets to be written before emitting a realm packet.
    pub async fn wait_for_instance_send_before_realm_send_like_cpp(&self) -> bool {
        self.core
            .wait_for_instance_send_before_realm_send_like_cpp()
            .await
    }

    /// Wait for realm packets to be written before emitting an instance update.
    pub async fn wait_for_realm_send_before_instance_update_like_cpp(&self) -> bool {
        self.core
            .wait_for_realm_send_before_instance_update_like_cpp()
            .await
    }

    /// Queue a source packet for nearby runtime recipients using the session's
    /// established visibility and connection routing.
    pub fn broadcast_from_position_to_visible_set_and_connection_like_cpp(
        &self,
        source_guid: ObjectGuid,
        source_position: Position,
        bytes: Vec<u8>,
        realm_connection: bool,
    ) {
        let Some(registry) = self.core.player_registry() else {
            return;
        };
        let player_guid = self.core.player_guid().unwrap_or(ObjectGuid::EMPTY);
        let map_id = self.core.player_map_id_like_cpp();
        let instance_id = self
            .core
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        let range_sq = VISIBILITY_RADIUS * VISIBILITY_RADIUS;

        let candidates: Vec<_> = registry
            .runtime_recipients()
            .into_iter()
            .filter_map(|recipient| {
                if recipient.guid == player_guid
                    || !recipient.is_in_world
                    || recipient.map_id != map_id
                    || recipient.instance_id != instance_id
                {
                    return None;
                }
                let dx = recipient.position.x - source_position.x;
                let dy = recipient.position.y - source_position.y;
                if dx * dx + dy * dy > range_sq {
                    return None;
                }
                Some(recipient.registration)
            })
            .collect();

        for registration in candidates {
            let command = SendIfVisibleLikeCppCommand {
                queued_at: Instant::now(),
                source_guid,
                map_id,
                instance_id,
                packet_bytes: bytes.clone(),
            };
            let command = if realm_connection {
                SessionCommand::SendRealmIfVisibleLikeCpp(command)
            } else {
                SessionCommand::SendIfVisibleLikeCpp(command)
            };
            let _ = registry.try_send_current_command(registration, command);
        }
    }

    /// Queue packets for the owner's current movement set over the selected
    /// connection, preserving the session's fixture-backed position view.
    pub fn broadcast_to_movement_set_in_range_and_connection_like_cpp(
        &self,
        bytes: Vec<u8>,
        range: f32,
        realm_connection: bool,
        #[cfg(any(test, feature = "test-fixtures"))] player_position_fixture: &Option<Position>,
    ) {
        let (Some(guid), Some(registry)) = (self.core.player_guid(), self.core.player_registry())
        else {
            return;
        };
        #[cfg(any(test, feature = "test-fixtures"))]
        let source_position = self
            .core
            .player_position_with_fixture_like_cpp(player_position_fixture);
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let source_position = self.core.player_position_with_fixture_like_cpp();
        let Some(source_position) = source_position else {
            return;
        };
        let map_id = self.core.player_map_id_like_cpp();
        let instance_id = self
            .core
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        for registration in registry.movement_recipients_within_range(
            guid,
            map_id,
            instance_id,
            source_position,
            range,
        ) {
            let command = SendIfVisibleLikeCppCommand {
                queued_at: Instant::now(),
                source_guid: guid,
                map_id,
                instance_id,
                packet_bytes: bytes.clone(),
            };
            let command = if realm_connection {
                SessionCommand::SendRealmIfVisibleLikeCpp(command)
            } else {
                SessionCommand::SendIfVisibleLikeCpp(command)
            };
            let _ = registry.try_send_current_command(registration, command);
        }
    }
}
