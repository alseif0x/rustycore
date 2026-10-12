// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Dungeon-finder status and blacklist handlers that only need the hub.
//!
//! C++ source of truth: `src/server/game/Handlers/LFGHandler.cpp` and
//! `MiscHandler.cpp`. Most of these opcodes answer the client from the
//! represented session state and packet publication only, so the World session
//! just builds the borrowed hub context (#1263 F5). `#1263 F5 remaining
//! families` moved the `DfGetSystemInfo` registration here as well
//! (`LFGHandler.cpp:107` `HandleDFGetSystemInfo`): its body stays in the World
//! shell while it needs the LFG dungeon catalog and the shell reward
//! projection, and the host lends that operation.

use tracing::warn;
use wow_constants::ClientOpcodes;
use wow_handler::{
    DuplicateHandlerRegistrationLikeCpp, HandlerFuture, PacketHandlerEntry, PacketProcessing,
    RegistryBuilder, SessionStatus,
};
use wow_packet::packets::misc::{DfGetJoinStatus, LfgListBlacklist, LfgUpdateStatus};
use wow_packet::{ClientPacket, ServerPacket, WorldPacket};
use wow_world_core::session::{HubMut, PacketPublicationAccessLikeCpp};

/// Borrowed inputs of one dungeon-finder handler invocation.
pub struct DungeonFindingHandlerCxLikeCpp<'a> {
    hub: HubMut<'a>,
}

impl<'a> DungeonFindingHandlerCxLikeCpp<'a> {
    pub fn new(hub: HubMut<'a>) -> Self {
        Self { hub }
    }

    fn publication_like_cpp(&self) -> PacketPublicationAccessLikeCpp<'_> {
        self.hub.shared().core.packet_publication_access_like_cpp()
    }

    /// CMSG_DF_GET_JOIN_STATUS — C++ returns before sending anything when
    /// `Player::isUsingLfg()` is false; Rust has no represented active LFG
    /// join state in this handler yet, so that observable branch is preserved.
    pub async fn handle_df_get_join_status(&mut self, mut pkt: WorldPacket) {
        if let Err(error) = DfGetJoinStatus::read(&mut pkt) {
            warn!(
                account = self.hub.shared().core.account_id,
                "DFGetJoinStatus parse failed: {error}"
            );
        }
    }

    /// CMSG_REQUEST_CONQUEST_FORMULA_CONSTANTS — C++ registers it as
    /// STATUS_UNHANDLED/Handle_NULL.
    pub async fn handle_request_conquest_formula_constants(&mut self, _pkt: WorldPacket) {}

    /// CMSG_REQUEST_LFG_LIST_BLACKLIST — C++ builds it from
    /// `sLFGMgr->GetLockedDungeons(playerGuid)`; Rust represents the
    /// well-defined no-locks response until that manager state is ported.
    pub async fn handle_request_lfg_list_blacklist(&mut self, _pkt: WorldPacket) {
        self.publication_like_cpp()
            .send_packet_realm(&LfgListBlacklist::empty());
    }

    /// CMSG_LFG_LIST_GET_STATUS — C++ always sends LFGUpdateStatus for a live
    /// player; Rust represents the well-defined no-ticket/no-queue branch
    /// until `sLFGMgr` state is ported.
    pub async fn handle_lfg_list_get_status(&mut self, _pkt: WorldPacket) {
        self.publication_like_cpp()
            .send_packet_realm(&LfgUpdateStatus::removed_from_queue());
    }
}

/// Builds a dungeon-finder handler context from a host's hub.
pub trait DungeonFindingHandlerHostLikeCpp<C> {
    fn dungeon_finding_handler_cx_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a C,
    ) -> DungeonFindingHandlerCxLikeCpp<'a>;

    /// C++ `LFGHandler.cpp:107` `WorldSession::HandleDFGetSystemInfo`.
    ///
    /// `#1263 F5 remaining families`: the legacy registration closure
    /// destructured the session catalog view (`lfg_dungeons`), so the host
    /// receives that view here. The body stays in the World session.
    fn handle_df_get_system_info_with_catalog_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a C,
        pkt: WorldPacket,
    ) -> HandlerFuture<'a, ()>;
}

fn handle_df_get_join_status_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: DungeonFindingHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .dungeon_finding_handler_cx_like_cpp(catalogs)
            .handle_df_get_join_status(pkt)
            .await;
    })
}

fn handle_request_conquest_formula_constants_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: DungeonFindingHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .dungeon_finding_handler_cx_like_cpp(catalogs)
            .handle_request_conquest_formula_constants(pkt)
            .await;
    })
}

fn handle_request_lfg_list_blacklist_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: DungeonFindingHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .dungeon_finding_handler_cx_like_cpp(catalogs)
            .handle_request_lfg_list_blacklist(pkt)
            .await;
    })
}

fn handle_lfg_list_get_status_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: DungeonFindingHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .dungeon_finding_handler_cx_like_cpp(catalogs)
            .handle_lfg_list_get_status(pkt)
            .await;
    })
}

/// `#1263 F5 remaining families`: the system-info entry that lived in the World
/// shell's `handlers/dungeon_finding/mod.rs`.
fn handle_df_get_system_info_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: DungeonFindingHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .handle_df_get_system_info_with_catalog_like_cpp(catalogs, pkt)
            .await
    })
}

/// Registers the dungeon-finder status handlers on the packet registry.
pub fn register_dungeon_finding_handlers_like_cpp<S, C>(
    builder: &mut RegistryBuilder<S, C>,
) -> Result<(), DuplicateHandlerRegistrationLikeCpp>
where
    S: DungeonFindingHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::DfGetJoinStatus,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadSafe,
        handler_name: "handle_df_get_join_status",
        handler: handle_df_get_join_status_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::RequestConquestFormulaConstants,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_request_conquest_formula_constants",
        handler: handle_request_conquest_formula_constants_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::RequestLfgListBlacklist,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_request_lfg_list_blacklist",
        handler: handle_request_lfg_list_blacklist_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::LfgListGetStatus,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_lfg_list_get_status",
        handler: handle_lfg_list_get_status_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::DfGetSystemInfo,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadSafe,
        handler_name: "handle_df_get_system_info",
        handler: handle_df_get_system_info_thunk::<S, C>,
    })?;
    Ok(())
}
