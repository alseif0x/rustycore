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
                    account = self.account_id,
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
            if let Some(cutoff) = self.suppress_creature_movement_queued_at_or_before_like_cpp {
                if queued_at <= cutoff {
                    tracing::info!(
                        account = self.account_id,
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
        if self.player_map_id_like_cpp() != map_id {
            if is_monster_move {
                tracing::info!(
                    account = self.account_id,
                    source_guid = ?source_guid,
                    player_map = self.player_map_id_like_cpp(),
                    command_map = map_id,
                    "RUST_MONSTER_MOVE_DELIVERY rejected: wrong map"
                );
            }
            return false;
        }
        // Gate 3: instance must match.
        let session_instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|k| k.instance_id)
            .unwrap_or(0);
        if session_instance_id != instance_id {
            if is_monster_move {
                tracing::info!(
                    account = self.account_id,
                    source_guid = ?source_guid,
                    session_instance_id,
                    command_instance_id = instance_id,
                    "RUST_MONSTER_MOVE_DELIVERY rejected: wrong instance"
                );
            }
            return false;
        }
        // Gate 4: source GUID must be in client's visible set (HaveAtClient).
        if !self.client_visible_guids_like_cpp.contains(&source_guid) {
            if is_monster_move {
                tracing::info!(
                    account = self.account_id,
                    source_guid = ?source_guid,
                    visible_count = self.client_visible_guids_like_cpp.len(),
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
                            account = self.account_id,
                            source_guid = ?source_guid,
                            visible_count = self.client_visible_guids_like_cpp.len(),
                            "RUST_MONSTER_MOVE_DELIVERY rejected: source failed current creature phase/range gate"
                        );
                    }
                    return false;
                }
                None => {
                    if is_monster_move {
                        tracing::info!(
                            account = self.account_id,
                            source_guid = ?source_guid,
                            visible_count = self.client_visible_guids_like_cpp.len(),
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
        if self.group_guid.is_none() {
            return;
        }

        let Some(loot) = self.loot_table.get(&owner_guid) else {
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
                self.player_map_id_like_cpp(),
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
        let Some(loot) = self.loot_table.get(&owner_guid).cloned() else {
            return;
        };
        let snapshot = wow_loot::OwnedLootSnapshot {
            generation: self
                .represented_loot_cache_generations_like_cpp
                .get(&owner_guid)
                .copied()
                .unwrap_or(0),
            scope: wow_loot::OwnedLootScope::Shared,
            loot,
        };
        self.represented_notify_loot_item_removed_from_snapshot_like_cpp(
            owner_guid,
            None,
            &snapshot,
            loot_list_id,
        );
    }

    pub(super) fn represented_notify_loot_item_removed_from_snapshot_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
        authority: Option<&OwnedLootAuthority>,
        snapshot: &wow_loot::OwnedLootSnapshot,
        loot_list_id: u8,
    ) {
        let loot = &snapshot.loot;
        let Some(entry) = loot
            .items
            .iter()
            .find(|entry| entry.loot_list_id == loot_list_id)
        else {
            return;
        };

        let packet = LootRemoved {
            owner: owner_guid,
            loot_obj: loot.loot_guid,
            loot_list_id,
        };
        let bytes = packet.to_bytes();
        let players_looting = loot.players_looting.clone();
        let allowed_looters = entry.allowed_looters.clone();
        let current_player = self.player_guid();
        let current_map = self.player_map_id_like_cpp();
        let current_instance = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        let registry = self.player_registry().cloned();
        let mut stale_looters = Vec::new();

        for looter in &players_looting {
            if !allowed_looters.contains(looter) {
                continue;
            }

            if Some(*looter) == current_player {
                self.send_packet(&packet);
                continue;
            }

            let Some(registry) = registry.as_ref() else {
                stale_looters.push(*looter);
                continue;
            };
            let Some(registration) =
                registry.loot_delivery_recipient(*looter, current_map, current_instance)
            else {
                stale_looters.push(*looter);
                continue;
            };
            if registry
                .send_current_packet(registration, bytes.clone())
                .is_err()
            {
                stale_looters.push(*looter);
            }
        }

        if !stale_looters.is_empty()
            && let Some(loot) = self.loot_table.get_mut(&owner_guid)
        {
            loot.players_looting
                .retain(|looter| !stale_looters.contains(looter));
        }
        if !stale_looters.is_empty() {
            if let Some(authority) = authority {
                for looter in stale_looters {
                    let _ =
                        authority.remove_viewer_if_generation_like_cpp(snapshot.generation, looter);
                }
            }
        }
    }

    pub(super) fn send_loot_error_like_cpp(
        &self,
        loot_obj: ObjectGuid,
        owner: ObjectGuid,
        error: u8,
    ) {
        self.send_packet(&LootResponse {
            owner,
            loot_obj,
            failure_reason: error,
            acquire_reason: 0,
            loot_method: 0,
            threshold: LOOT_RESPONSE_DEFAULT_THRESHOLD_LIKE_CPP,
            coins: 0,
            items: vec![],
            currencies: vec![],
            acquired: false,
            ae_looting: false,
        });
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
        let is_encounter_loot = dungeon_encounter_id != 0;
        self.send_packet_realm(&ItemPushResult {
            player_guid,
            slot: u8::from(INVENTORY_SLOT_BAG_0),
            slot_in_bag: i32::from(slot),
            item: ItemInstance {
                item_id: loot_entry.item_id as i32,
                random_properties_seed,
                random_properties_id,
                item_bonus: None,
                modifications: ItemModList { values: Vec::new() },
            },
            quest_log_item_id: 0,
            quantity: quantity as i32,
            quantity_in_inventory: quantity_in_inventory as i32,
            dungeon_encounter_id: dungeon_encounter_id as i32,
            battle_pet_species_id: 0,
            battle_pet_breed_id: 0,
            battle_pet_breed_quality: 0,
            battle_pet_level: 0,
            item_guid,
            pushed: false,
            display_text: if is_encounter_loot {
                ItemPushResultDisplayType::EncounterLoot
            } else {
                ItemPushResultDisplayType::Normal
            },
            created,
            is_bonus_roll: false,
            is_encounter_loot,
        });
    }
}
