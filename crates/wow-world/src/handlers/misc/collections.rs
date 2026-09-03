// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Private collections capability handlers extracted from the legacy misc owner.

use tracing::{debug, info, warn};
use wow_constants::{ClientOpcodes, InventoryResult, SpellCastResult};
use wow_core::ObjectGuid;
use wow_handler::{PacketProcessing, SessionStatus};

use crate::session::registry::PacketHandlerEntry;
use wow_packet::packets::collection::{
    COLLECTION_TYPE_APPEARANCE_LIKE_CPP, COLLECTION_TYPE_TOYBOX_LIKE_CPP,
    CollectionItemSetFavorite, TransmogrifyItems,
};
use wow_packet::packets::item::InventoryChangeFailure;
use wow_packet::packets::misc::{
    AddToy, MountSetFavorite, MountSpecial, SpecialMountAnim, ToyClearFanfare, UseToy,
};
use wow_packet::packets::spell::{CastFailed, SpellCastVisual, SpellPreparePkt, SpellStartPkt};
use wow_packet::{ClientPacket, ServerPacket};

use crate::entity_update_bridge::player_values_update_to_update_object;
use crate::session::{CAST_FLAG_EX_USE_TOY_SPELL_LIKE_CPP, SpellCastMetadata};

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::MountSetFavorite,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_mount_set_favorite",
        handler: |session, pkt| {
            Box::pin(async move { session.handle_mount_set_favorite(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::MountSpecialAnim,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_mount_special_anim",
        handler: |session, pkt| {
            Box::pin(async move { session.handle_mount_special_anim(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::CollectionItemSetFavorite,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_collection_item_set_favorite",
        handler: |session, pkt| {
            Box::pin(async move { session.handle_collection_item_set_favorite(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::MountClearFanfare,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_mount_clear_fanfare",
        handler: |session, pkt| {
            Box::pin(async move { session.handle_mount_clear_fanfare(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::AddToy,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_add_toy",
        handler: |session, pkt| Box::pin(async move { session.handle_add_toy(pkt).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ToyClearFanfare,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_toy_clear_fanfare",
        handler: |session, pkt| {
            Box::pin(async move { session.handle_toy_clear_fanfare(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::UseToy,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_use_toy",
        handler: |session, pkt| Box::pin(async move { session.handle_use_toy(pkt).await }),
    }
}

impl crate::session::WorldSession {
    /// CMSG_MOUNT_SET_FAVORITE — toggle the favorite bit on a known account mount.
    ///
    /// C++ ref: `WorldSession::HandleMountSetFavorite` delegates to
    /// `CollectionMgr::MountSetFavorite`, which silently ignores unknown mounts
    /// and sends a partial `SMSG_ACCOUNT_MOUNT_UPDATE` for the changed mount.
    pub async fn handle_mount_set_favorite(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match MountSetFavorite::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "MountSetFavorite parse failed: {error}"
                );
                return;
            }
        };

        self.mount_set_favorite_like_cpp(request.mount_spell_id, request.is_favorite);
    }

    /// CMSG_MOUNT_SPECIAL_ANIM — forward the requested mount animation packet.
    ///
    /// C++ ref: `WorldSession::HandleMountSpecialAnimOpcode` copies the
    /// client-provided visual kit ids and sequence variation into
    /// `SMSG_SPECIAL_MOUNT_ANIM`, sets `UnitGUID` to the player, and calls
    /// `SendMessageToSet(..., false)`. C++ `MessageDistDeliverer` still skips
    /// the source player (`player == i_source`) and then applies `HaveAtClient`
    /// for nearby receivers, so Rust queues the packet to other sessions via
    /// the existing `SendIfVisibleLikeCpp` per-session gate.

    pub async fn handle_mount_special_anim(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match MountSpecial::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "MountSpecial parse failed: {error}"
                );
                return;
            }
        };
        let Some(unit_guid) = self.player_guid() else {
            return;
        };

        let packet_bytes = SpecialMountAnim {
            unit_guid,
            spell_visual_kit_ids: request.spell_visual_kit_ids,
            sequence_variation: request.sequence_variation,
        }
        .to_bytes();

        self.send_mount_special_anim_to_visible_set_like_cpp(unit_guid, packet_bytes);
    }

    fn send_mount_special_anim_to_visible_set_like_cpp(
        &self,
        source_guid: ObjectGuid,
        packet_bytes: Vec<u8>,
    ) {
        let Some(registry) = self.player_registry() else {
            return;
        };
        let map_id = self.player_map_id_like_cpp();
        let instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);

        for registration in registry.same_map_movement_recipients(source_guid, map_id, instance_id)
        {
            let _ = registry.try_send_current_command(
                registration,
                crate::session::mailbox::SessionCommand::SendIfVisibleLikeCpp(
                    crate::session::mailbox::SendIfVisibleLikeCppCommand {
                        queued_at: std::time::Instant::now(),
                        source_guid,
                        map_id,
                        instance_id,
                        packet_bytes: packet_bytes.clone(),
                    },
                ),
            );
        }
    }

    /// CMSG_COLLECTION_ITEM_SET_FAVORITE — toggle favorite state for supported collections.
    ///
    /// C++ ref: `WorldSession::HandleCollectionItemSetFavorite` forwards TOYBOX
    /// ids to `CollectionMgr::ToySetFavorite`, and only forwards APPEARANCE ids
    /// when `CollectionMgr::HasItemAppearance(id)` returns a permanent
    /// appearance. Temporary appearances, unknown ids, and unsupported collection
    /// types are ignored.

    pub async fn handle_collection_item_set_favorite(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match CollectionItemSetFavorite::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "CollectionItemSetFavorite parse failed: {error}"
                );
                return;
            }
        };

        match request.collection_type {
            COLLECTION_TYPE_TOYBOX_LIKE_CPP => {
                self.toy_set_favorite_like_cpp(request.id, request.is_favorite);
            }
            COLLECTION_TYPE_APPEARANCE_LIKE_CPP => {
                let (has_appearance, is_temporary) = self.has_item_appearance_like_cpp(request.id);
                if !has_appearance || is_temporary {
                    return;
                }

                self.set_appearance_is_favorite_like_cpp(request.id, request.is_favorite);
            }
            _ => {}
        }
    }

    /// CMSG_TRANSMOGRIFY_ITEMS — parsed only; full C++ handler is not ported yet.
    ///
    /// C++ `WorldSession::HandleTransmogrifyItems` also validates the NPC
    /// interaction, inventory items, appearances, costs, modifiers, and reset
    /// paths before applying changes. This Rust slice only represents the
    /// client packet and keeps gameplay state unchanged.

    pub async fn handle_transmogrify_items(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match TransmogrifyItems::read_like_cpp(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "TransmogrifyItems parse failed: {error}"
                );
                return;
            }
        };

        debug!(
            account = self.account_id,
            npc = ?request.npc,
            item_count = request.items.len(),
            current_spec_only = request.current_spec_only,
            "TransmogrifyItems parsed; full C++ transmogrification application is pending"
        );
    }

    /// CMSG_MOUNT_CLEAR_FANFARE — C++ currently logs only.

    pub async fn handle_mount_clear_fanfare(&mut self, _pkt: wow_packet::WorldPacket) {
        debug!(account = self.account_id, "Mount fanfare cleared");
    }

    /// CMSG_TOY_CLEAR_FANFARE — clear the account toy fanfare bit.
    ///
    /// C++ ref: `WorldSession::HandleToyClearFanfare` forwards only the item id
    /// to `CollectionMgr::ToyClearFanfare`, which silently ignores unknown toys.

    pub async fn handle_toy_clear_fanfare(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match ToyClearFanfare::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "ToyClearFanfare parse failed: {error}"
                );
                return;
            }
        };

        self.toy_clear_fanfare_like_cpp(request.item_id);
    }

    /// CMSG_USE_TOY — bounded C++ guard path before spell execution.
    ///
    /// C++ `HandleUseToy` validates item template, `CollectionMgr::HasToy`,
    /// item effect spell membership, `SpellMgr::GetSpellInfo`, possession, and
    /// then creates/prepares a `Spell` with toy-specific flags. Rust still uses
    /// the represented spell executor, but preserves the C++ toy metadata that
    /// must reach `SpellCastData`.

    pub async fn handle_use_toy(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match UseToy::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(account = self.account_id, "UseToy parse failed: {error}");
                return;
            }
        };

        let item_id = match u32::try_from(request.cast.misc[0]) {
            Ok(item_id) if item_id != 0 => item_id,
            _ => return,
        };

        if self.item_storage_template(item_id).is_none() {
            return;
        }

        if !self.has_account_toy_like_cpp(item_id) {
            return;
        }

        if !self.toy_item_has_spell_effect_like_cpp(item_id, request.cast.spell_id) {
            return;
        }

        let Some(spell_store) = self.spell_store() else {
            return;
        };
        let Some(spell_info) = spell_store.get(request.cast.spell_id).cloned() else {
            warn!(
                account = self.account_id,
                spell_id = request.cast.spell_id,
                item_id,
                "HandleUseToy: unknown spell id used by toy item"
            );
            return;
        };

        if self.player_is_possessing_like_cpp() {
            return;
        }

        let toy_cooldown_ms =
            self.toy_item_spell_cooldown_ms_like_cpp(item_id, request.cast.spell_id, &spell_info);
        if let Some(remaining_ms) = self.represented_spell_cooldown_remaining_ms_like_cpp(
            request.cast.spell_id,
            toy_cooldown_ms,
        ) {
            debug!(
                account = self.account_id,
                item_id,
                spell_id = request.cast.spell_id,
                remaining_ms,
                "UseToy rejected by represented item-backed cooldown"
            );
            self.send_packet(&CastFailed {
                cast_id: request.cast.cast_id,
                spell_id: request.cast.spell_id,
                visual: request.cast.visual.clone(),
                reason: SpellCastResult::NotReady as i32,
                fail_arg1: 0,
                fail_arg2: 0,
            });
            return;
        }

        let Some(player_guid) = self.player_guid() else {
            return;
        };

        let server_cast_id = self.next_represented_spell_cast_guid_like_cpp(request.cast.spell_id);
        self.send_packet(&SpellPreparePkt {
            client_cast_id: request.cast.cast_id,
            server_cast_id,
        });

        let metadata = SpellCastMetadata {
            from_client: true,
            misc: request.cast.misc,
            cast_item_entry: Some(item_id),
            cast_item_battle_pet_modifiers: None,
            cast_flags_ex: CAST_FLAG_EX_USE_TOY_SPELL_LIKE_CPP,
            original_cast_id: request.cast.cast_id,
            unit_target_battle_pet_companion_guid: None,
            ..SpellCastMetadata::default()
        };

        let mut spell_target = request.cast.target.clone();
        let target_guid = if !spell_target.unit.is_empty() {
            spell_target.unit
        } else {
            spell_target.flags |= 0x2; // SpellCastTargetFlags::Unit
            spell_target.unit = player_guid;
            player_guid
        };

        let spell_visual = SpellCastVisual {
            spell_visual_id: request.cast.visual.spell_visual_id,
            script_visual_id: 0,
        };

        if spell_info.has_cast_time() {
            let start_pkt = SpellStartPkt {
                caster: player_guid,
                cast_id: server_cast_id,
                original_cast_id: request.cast.cast_id,
                spell_id: request.cast.spell_id,
                visual: spell_visual.clone(),
                cast_flags: 0x0000_0002,
                cast_flags_ex: CAST_FLAG_EX_USE_TOY_SPELL_LIKE_CPP,
                cast_time_ms: spell_info.cast_time_ms,
                target: spell_target.clone(),
            };
            self.send_packet(&start_pkt);

            self.active_spell_cast = Some(crate::session::SpellCastState {
                spell_id: request.cast.spell_id,
                target_guid,
                target_data: spell_target,
                cast_id: server_cast_id,
                cast_start_time: std::time::Instant::now(),
                cast_time_ms: spell_info.cast_time_ms,
                spell_visual,
                metadata,
            });
        } else if let Err(error) = self
            .execute_spell_with_visual_and_target_data_with_metadata(
                request.cast.spell_id,
                target_guid,
                server_cast_id,
                spell_visual,
                spell_target,
                metadata,
            )
            .await
        {
            warn!(
                account = self.account_id,
                spell_id = request.cast.spell_id,
                item_id,
                "UseToy represented spell execution failed: {error}"
            );
        }

        debug!(
            account = self.account_id,
            item_id,
            spell_id = request.cast.spell_id,
            "UseToy executed through represented spell path"
        );
    }

    /// CMSG_ADD_TOY — learn a Toy.db2 item and consume the inventory item.
    ///
    /// C++ ref: `WorldSession::HandleAddToy` validates the item guid, checks
    /// `sDB2Manager.IsToyItem(item->GetEntry())`, calls
    /// `CollectionMgr::AddToy(item->GetEntry(), false, false)`, which inserts
    /// the account row and calls `Player::AddToy`, then destroys the item only
    /// when the account toy was newly inserted.

    pub async fn handle_add_toy(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match AddToy::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(account = self.account_id, "AddToy parse failed: {error}");
                return;
            }
        };

        if request.item_guid == wow_core::ObjectGuid::EMPTY {
            return;
        }

        let Some((bag, slot, item)) = self.get_inventory_item_by_guid_like_cpp(request.item_guid)
        else {
            self.send_packet_realm(&InventoryChangeFailure::error(
                InventoryResult::ItemNotFound,
            ));
            return;
        };

        if !self.is_toy_item_like_cpp(item.entry_id) {
            return;
        }

        let runtime_item = self
            .resolved_inventory_item_objects_like_cpp()
            .and_then(|items| items.get(&item.guid).cloned());
        let can_use_result =
            self.can_use_inventory_item_represented_like_cpp(&item, runtime_item.as_ref());
        if can_use_result != InventoryResult::Ok {
            self.send_equip_error(can_use_result, Some(item.guid), None, 0, 0);
            return;
        }

        if !self.add_account_toy_like_cpp(item.entry_id, false, false) {
            return;
        }

        let destroyed_entry_id = item.entry_id;
        if self
            .destroy_inventory_full_stack_by_pos_like_cpp(bag, slot, item, runtime_item, "AddToy")
            .await
        {
            if let Some(update) = self.add_player_toy_dynamic_field_like_cpp(destroyed_entry_id) {
                if let Some(guid) = self.player_guid() {
                    if let Some(packet) = player_values_update_to_update_object(
                        guid,
                        self.player_map_id_like_cpp(),
                        &update,
                    ) {
                        self.send_packet(&packet);
                    }
                }
            }
            info!(
                "Added toy item={} from bag {} slot {} for account {}",
                destroyed_entry_id, bag, slot, self.account_id
            );
        } else {
            self.remove_account_toy_like_cpp(destroyed_entry_id);
        }
    }
}
