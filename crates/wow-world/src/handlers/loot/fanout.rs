// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Loot packet fanout to viewers and nearby sessions.

use super::*;

impl WorldSession {
    /// Shared per-session visibility gate for one or more packet frames.
    ///
    /// Mirrors C++ `GridNotifiers.h : MessageDistDeliverer::SendPacket` and
    /// `GridNotifiersImpl.h : MessageDistDeliverer::Visit(PlayerMapType&)`:
    /// `MessageDistDeliverer::Visit` rechecks phase/distance against the
    /// current source object, then `SendPacket` applies HaveAtClient.
    pub(super) fn send_if_visible_like_cpp_gate_passes_like_cpp(
        &mut self,
        queued_at: Instant,
        source_guid: ObjectGuid,
        map_id: u16,
        instance_id: u32,
        representative_packet_bytes: &[u8],
        allow_legacy_creature_source: bool,
    ) -> bool {
        let is_monster_move = representative_packet_bytes
            .get(0..2)
            .and_then(|bytes| bytes.try_into().ok())
            .map(u16::from_le_bytes)
            == Some(wow_constants::ServerOpcodes::OnMonsterMove as u16);
        // Gate 1: session must be fully logged in (player object loaded).
        if self.state() != crate::session::SessionState::LoggedIn {
            if is_monster_move {
                tracing::info!(
                    account = self.core.account_id,
                    source_guid = ?source_guid,
                    "RUST_MONSTER_MOVE_DELIVERY rejected: session not logged in"
                );
            }
            return false;
        }
        // Gate 1b: C++ does not deliver SMSG_ON_MONSTER_MOVE during the
        // initial enter-world packet burst. Rust queues fan-out commands from
        // a sessionless world tick, so drop only movement commands that were
        // queued before the login burst completed.
        if is_monster_move {
            if let Some(cutoff) = self
                .world_entities
                .suppress_creature_movement_queued_at_or_before_like_cpp
            {
                if queued_at <= cutoff {
                    tracing::info!(
                        account = self.core.account_id,
                        source_guid = ?source_guid,
                        queued_before_cutoff_ms =
                            cutoff.saturating_duration_since(queued_at).as_millis(),
                        "RUST_MONSTER_MOVE_DELIVERY rejected: queued before enter-world movement cutoff"
                    );
                    return false;
                }
            }
        }
        // Gate 2: map must match.
        if self.core.player_map_id_like_cpp() != map_id {
            if is_monster_move {
                tracing::info!(
                    account = self.core.account_id,
                    source_guid = ?source_guid,
                    player_map = self.core.player_map_id_like_cpp(),
                    command_map = map_id,
                    "RUST_MONSTER_MOVE_DELIVERY rejected: wrong map"
                );
            }
            return false;
        }
        // Gate 3: instance must match.
        let session_instance_id = self
            .core
            .current_canonical_player_map_key_like_cpp()
            .map(|k| k.instance_id)
            .unwrap_or(0);
        if session_instance_id != instance_id {
            if is_monster_move {
                tracing::info!(
                    account = self.core.account_id,
                    source_guid = ?source_guid,
                    session_instance_id,
                    command_instance_id = instance_id,
                    "RUST_MONSTER_MOVE_DELIVERY rejected: wrong instance"
                );
            }
            return false;
        }
        // Gate 4: source GUID must be in client's visible set (HaveAtClient).
        if !self
            .core
            .client_visible_guids_like_cpp
            .contains(&source_guid)
        {
            if is_monster_move {
                tracing::info!(
                    account = self.core.account_id,
                    source_guid = ?source_guid,
                    visible_count = self.core.client_visible_guids_like_cpp.len(),
                    "RUST_MONSTER_MOVE_DELIVERY rejected: source not visible"
                );
            }
            return false;
        }
        // Gate 5: for creature-backed MessageDistDeliverer packets, re-read
        // the current source object and apply C++ Visit(PlayerMapType&): same
        // phase and exact 2D visibility range before SendPacket.
        if source_guid.is_creature() {
            match self
                .represented_can_receive_creature_message_to_set_by_guid_with_legacy_fallback_like_cpp(
                    source_guid,
                    map_id,
                    instance_id,
                    false,
                    allow_legacy_creature_source,
                )
            {
                Some(true) => {}
                Some(false) => {
                    if is_monster_move {
                        tracing::info!(
                            account = self.core.account_id,
                            source_guid = ?source_guid,
                            visible_count = self.core.client_visible_guids_like_cpp.len(),
                            "RUST_MONSTER_MOVE_DELIVERY rejected: source failed current creature phase/range gate"
                        );
                    }
                    return false;
                }
                None => {
                    if is_monster_move {
                        tracing::info!(
                            account = self.core.account_id,
                            source_guid = ?source_guid,
                            visible_count = self.core.client_visible_guids_like_cpp.len(),
                            "RUST_MONSTER_MOVE_DELIVERY rejected: source creature missing"
                        );
                    }
                    return false;
                }
            }
        }
        true
    }

    pub(super) fn represented_notify_loot_list_like_cpp(&self, owner_guid: ObjectGuid) {
        if self.resolved_group_guid_like_cpp().is_none() {
            return;
        }

        let Some(loot) = self.loot.loot_table.get(&owner_guid) else {
            return;
        };

        let master = if loot.loot_method == LOOT_METHOD_MASTER_LIKE_CPP
            && loot_has_over_threshold_item_like_cpp(loot)
        {
            (!loot.loot_master.is_empty()).then_some(loot.loot_master)
        } else {
            None
        };

        let packet = LootList {
            owner: owner_guid,
            loot_obj: loot.loot_guid,
            master,
            round_robin_winner: (!loot.round_robin_player.is_empty())
                .then_some(loot.round_robin_player),
        };
        let bytes = packet.to_bytes();
        let instance_id = self
            .core
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);

        for allowed_looter in &loot.allowed_looters {
            if Some(*allowed_looter) == self.player_guid() {
                self.send_packet(&packet);
                continue;
            }

            let Some(registry) = self.player_registry() else {
                continue;
            };
            let Some(registration) = registry.loot_delivery_recipient(
                *allowed_looter,
                self.core.player_map_id_like_cpp(),
                instance_id,
            ) else {
                continue;
            };

            let _ = registry.send_current_packet(registration, bytes.clone());
        }
    }

    pub(super) fn represented_notify_loot_item_removed_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
        loot_list_id: u8,
    ) {
        let (state, mut hub) = crate::session::split_loot_mut(self);
        state.represented_notify_loot_item_removed_like_cpp(&mut hub, owner_guid, loot_list_id)
    }

    pub(super) fn send_loot_error_like_cpp(
        &self,
        loot_obj: ObjectGuid,
        owner: ObjectGuid,
        error: u8,
    ) {
        let (state, hub) = crate::session::split_loot_ref(self);
        state.send_loot_error_like_cpp(hub, loot_obj, owner, error)
    }

    pub(super) fn send_loot_item_push_result(
        &self,
        player_guid: ObjectGuid,
        item_guid: ObjectGuid,
        loot_entry: &LootEntry,
        random_properties_id: i32,
        random_properties_seed: i32,
        slot: u8,
        quantity: u32,
        quantity_in_inventory: u32,
        created: bool,
        dungeon_encounter_id: u32,
    ) {
        let (state, hub) = crate::session::split_loot_ref(self);
        state.send_loot_item_push_result(
            hub,
            player_guid,
            item_guid,
            loot_entry,
            random_properties_id,
            random_properties_seed,
            slot,
            quantity,
            quantity_in_inventory,
            created,
            dungeon_encounter_id,
        )
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/handlers/loot/fanout/f3_shims.rs"]
mod f3_shims;
