// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Mount and toy collection handlers that only need the hub.
//!
//! C++ source of truth: `WorldSession::HandleMountSetFavorite`,
//! `WorldSession::HandleMountSpecialAnimOpcode`,
//! `WorldSession::HandleMountClearFanfare` and
//! `WorldSession::HandleToyClearFanfare`
//! (`src/server/game/Handlers/MiscHandler.cpp`). The collection state, the
//! movement-recipient directory and packet delivery already live in the Core
//! hub, so the World session only builds the borrowed hub context (#1263 F5).
//! The handlers that still touch item/inventory or spell state
//! (`CollectionItemSetFavorite`, `AddToy`, `UseToy`) stay in the shell.

use tracing::{debug, warn};
use wow_constants::ClientOpcodes;
use wow_core::ObjectGuid;
use wow_handler::{
    DuplicateHandlerRegistrationLikeCpp, HandlerFuture, PacketHandlerEntry, PacketProcessing,
    RegistryBuilder, SessionStatus,
};
use wow_packet::packets::misc::{
    MountSetFavorite, MountSpecial, SpecialMountAnim, ToyClearFanfare,
};
use wow_packet::{ClientPacket, ServerPacket, WorldPacket};
use wow_world_core::session::HubMut;
use wow_world_core::session::mailbox::{SendIfVisibleLikeCppCommand, SessionCommand};

/// Borrowed inputs of one collection handler invocation.
pub struct CollectionsHandlerCxLikeCpp<'a> {
    hub: HubMut<'a>,
}

impl<'a> CollectionsHandlerCxLikeCpp<'a> {
    pub fn new(hub: HubMut<'a>) -> Self {
        Self { hub }
    }

    /// CMSG_MOUNT_SET_FAVORITE — toggle the account mount favorite flag.
    pub async fn handle_mount_set_favorite(&mut self, mut pkt: WorldPacket) {
        let request = match MountSetFavorite::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.hub.shared().core.account_id,
                    "MountSetFavorite parse failed: {error}"
                );
                return;
            }
        };

        self.hub
            .mount_set_favorite_like_cpp(request.mount_spell_id, request.is_favorite);
    }

    /// CMSG_MOUNT_SPECIAL_ANIM — forward the requested mount animation packet.
    pub async fn handle_mount_special_anim(&mut self, mut pkt: WorldPacket) {
        let request = match MountSpecial::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.hub.shared().core.account_id,
                    "MountSpecial parse failed: {error}"
                );
                return;
            }
        };
        let Some(unit_guid) = self.hub.shared().core.player_guid() else {
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

    /// C++ `MessageDistDeliverer` skips the source player
    /// (`player == i_source`) and then applies `HaveAtClient` for nearby
    /// receivers; Rust queues the packet to other sessions through the existing
    /// `SendIfVisibleLikeCpp` per-session gate.
    fn send_mount_special_anim_to_visible_set_like_cpp(
        &self,
        source_guid: ObjectGuid,
        packet_bytes: Vec<u8>,
    ) {
        let core = &self.hub.shared().core;
        let Some(registry) = core.player_registry() else {
            return;
        };
        let map_id = core.player_map_id_like_cpp();
        let instance_id = core
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);

        for registration in registry.same_map_movement_recipients(source_guid, map_id, instance_id)
        {
            let _ = registry.try_send_current_command(
                registration,
                SessionCommand::SendIfVisibleLikeCpp(SendIfVisibleLikeCppCommand {
                    queued_at: std::time::Instant::now(),
                    source_guid,
                    map_id,
                    instance_id,
                    packet_bytes: packet_bytes.clone(),
                }),
            );
        }
    }

    /// CMSG_MOUNT_CLEAR_FANFARE — C++ currently logs only.
    pub async fn handle_mount_clear_fanfare(&mut self, _pkt: WorldPacket) {
        debug!(
            account = self.hub.shared().core.account_id,
            "Mount fanfare cleared"
        );
    }

    /// CMSG_TOY_CLEAR_FANFARE — clear the account toy fanfare bit.
    pub async fn handle_toy_clear_fanfare(&mut self, mut pkt: WorldPacket) {
        let request = match ToyClearFanfare::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.hub.shared().core.account_id,
                    "ToyClearFanfare parse failed: {error}"
                );
                return;
            }
        };

        self.hub.toy_clear_fanfare_like_cpp(request.item_id);
    }
}

/// Builds a collections handler context from a host's hub.
pub trait CollectionsHandlerHostLikeCpp<C> {
    fn collections_handler_cx_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a C,
    ) -> CollectionsHandlerCxLikeCpp<'a>;
}

fn handle_mount_set_favorite_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: CollectionsHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .collections_handler_cx_like_cpp(catalogs)
            .handle_mount_set_favorite(pkt)
            .await;
    })
}

fn handle_mount_special_anim_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: CollectionsHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .collections_handler_cx_like_cpp(catalogs)
            .handle_mount_special_anim(pkt)
            .await;
    })
}

fn handle_mount_clear_fanfare_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: CollectionsHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .collections_handler_cx_like_cpp(catalogs)
            .handle_mount_clear_fanfare(pkt)
            .await;
    })
}

fn handle_toy_clear_fanfare_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: CollectionsHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .collections_handler_cx_like_cpp(catalogs)
            .handle_toy_clear_fanfare(pkt)
            .await;
    })
}

/// Registers the mount and toy collection handlers on the packet registry.
pub fn register_collections_handlers_like_cpp<S, C>(
    builder: &mut RegistryBuilder<S, C>,
) -> Result<(), DuplicateHandlerRegistrationLikeCpp>
where
    S: CollectionsHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::MountSetFavorite,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_mount_set_favorite",
        handler: handle_mount_set_favorite_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::MountSpecialAnim,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_mount_special_anim",
        handler: handle_mount_special_anim_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::MountClearFanfare,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_mount_clear_fanfare",
        handler: handle_mount_clear_fanfare_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::ToyClearFanfare,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_toy_clear_fanfare",
        handler: handle_toy_clear_fanfare_thunk::<S, C>,
    })?;
    Ok(())
}
