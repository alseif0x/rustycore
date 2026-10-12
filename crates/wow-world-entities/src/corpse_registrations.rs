// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Death, resurrection and corpse packet-handler registrations (#1263 F5,
//! deferred families).
//!
//! C++ source of truth: `src/server/game/Handlers/MiscHandler.cpp:447`
//! `HandleResurrectResponse`, `:59` `HandleRepopRequest`, `:416`
//! `HandleReclaimCorpse` and `:361` `HandleRequestCemeteryList`;
//! `src/server/game/Server/Protocol/Opcodes.cpp` fixes
//! `CMSG_RESURRECT_RESPONSE` `:855`, `CMSG_REPOP_REQUEST` `:811` and
//! `CMSG_RECLAIM_CORPSE` `:804` (`STATUS_LOGGEDIN`, `PROCESS_THREADUNSAFE`) and
//! `CMSG_REQUEST_CEMETERY_LIST` `:822` (`STATUS_LOGGEDIN`, `PROCESS_INPLACE`).
//!
//! The application crate owns this registration family because it already
//! coordinates the represented map transitions these opcodes end in
//! (`travel_handlers`) and the corpse queries (`character_query_handlers`).
//! Only the registration source moved: the four entries keep their opcode,
//! session status, packet processing and handler name exactly as the World
//! legacy inventory submitted them, and every packet is handed to the host
//! unread, as the legacy closures did.

use wow_constants::ClientOpcodes;
use wow_handler::{
    DuplicateHandlerRegistrationLikeCpp, HandlerFuture, PacketHandlerEntry, PacketProcessing,
    RegistryBuilder, SessionStatus,
};
use wow_packet::WorldPacket;

/// The adapter contract between the corpse registrations and the World
/// session that executes them.
///
/// **Death, resurrection and map transitions stay with the host.** C++ drives
/// them through the Player: `BuildPlayerRepop` + `RepopAtGraveyard`
/// (`MiscHandler.cpp:81-82`), `ResurrectUsingRequestData` with its teleport
/// (`:475`), and `ResurrectPlayer` + `SpawnCorpseBones` (`:441-444`); the
/// cemetery list reads `ObjectMgr::GraveyardStore` and its conditions
/// (`:367-372`). The host keeps the Rust equivalent it already owns: the
/// represented alive/ghost state, the pending resurrection request, the
/// teleport/far-transfer path and the deferred resurrection after it, the
/// graveyard catalog and condition evaluation, and every client publication.
/// This crate owns only the registration and dispatch of the four opcodes.
pub trait CorpseHandlerHostLikeCpp<C> {
    /// C++ `HandleResurrectResponse` (`MiscHandler.cpp:447`). The legacy
    /// closure did not read the catalog view.
    fn handle_resurrect_response_like_cpp<'a>(
        &'a mut self,
        pkt: WorldPacket,
    ) -> HandlerFuture<'a, ()>;

    /// C++ `HandleRepopRequest` (`MiscHandler.cpp:59`). The legacy closure did
    /// not read the catalog view.
    fn handle_repop_request_like_cpp<'a>(&'a mut self, pkt: WorldPacket) -> HandlerFuture<'a, ()>;

    /// C++ `HandleReclaimCorpse` (`MiscHandler.cpp:416`). The legacy closure did
    /// not read the catalog view.
    fn handle_reclaim_corpse_like_cpp<'a>(&'a mut self, pkt: WorldPacket) -> HandlerFuture<'a, ()>;

    /// C++ `HandleRequestCemeteryList` (`MiscHandler.cpp:361`). The legacy
    /// closure destructured the catalog view (`graveyards`), so the host
    /// receives that view.
    fn handle_request_cemetery_list_with_catalogs_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a C,
        pkt: WorldPacket,
    ) -> HandlerFuture<'a, ()>;
}

fn handle_resurrect_response_thunk<'a, S, C>(
    session: &'a mut S,
    _catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: CorpseHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    session.handle_resurrect_response_like_cpp(pkt)
}

fn handle_repop_request_thunk<'a, S, C>(
    session: &'a mut S,
    _catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: CorpseHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    session.handle_repop_request_like_cpp(pkt)
}

fn handle_reclaim_corpse_thunk<'a, S, C>(
    session: &'a mut S,
    _catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: CorpseHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    session.handle_reclaim_corpse_like_cpp(pkt)
}

fn handle_request_cemetery_list_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: CorpseHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    session.handle_request_cemetery_list_with_catalogs_like_cpp(catalogs, pkt)
}

/// Register the four C++ `MiscHandler.cpp` death/corpse opcodes.
pub fn register_corpse_handlers_like_cpp<S, C>(
    builder: &mut RegistryBuilder<S, C>,
) -> Result<(), DuplicateHandlerRegistrationLikeCpp>
where
    S: CorpseHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::ResurrectResponse,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_resurrect_response",
        handler: handle_resurrect_response_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::RepopRequest,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_repop_request",
        handler: handle_repop_request_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::ReclaimCorpse,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_reclaim_corpse",
        handler: handle_reclaim_corpse_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::RequestCemeteryList,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_request_cemetery_list",
        handler: handle_request_cemetery_list_thunk::<S, C>,
    })?;
    Ok(())
}
