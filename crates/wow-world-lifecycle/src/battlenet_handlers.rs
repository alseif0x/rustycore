// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Battle.net service and realm-list ticket handlers.
//!
//! The client sends `BattlenetRequest` (CMSG 0x36FD) during character select
//! to invoke GameUtilitiesService RPCs. RustyCore answers `RpcNotImplemented`
//! for every request, matching the C# contract when no service handler is
//! registered, and mirrors `WorldSession::HandleBattlenetChangeRealmTicket`
//! for the realm-list ticket. The family owns the packet bodies; the World
//! session only builds the borrowed hub context (#1263 F5).

use tracing::debug;
use wow_constants::ClientOpcodes;
use wow_handler::{
    DuplicateHandlerRegistrationLikeCpp, HandlerFuture, PacketHandlerEntry, PacketProcessing,
    RegistryBuilder, SessionStatus,
};
use wow_packet::ClientPacket;
use wow_packet::WorldPacket;
use wow_packet::packets::battlenet::{
    BattlenetRequest, BattlenetResponse, BattlenetRpcErrorCode, ChangeRealmTicket,
    ChangeRealmTicketResponse,
};
use wow_world_core::session::{HubMut, PacketPublicationAccessLikeCpp};

/// Borrowed inputs of one battle.net handler invocation.
pub struct BattlenetHandlerCxLikeCpp<'a> {
    hub: HubMut<'a>,
}

impl<'a> BattlenetHandlerCxLikeCpp<'a> {
    pub fn new(hub: HubMut<'a>) -> Self {
        Self { hub }
    }

    fn publication_like_cpp(&self) -> PacketPublicationAccessLikeCpp<'_> {
        self.hub.shared().core.packet_publication_access_like_cpp()
    }

    /// C# dispatches these to GameUtilitiesService handlers. Since we don't
    /// implement any services yet, we always return RpcNotImplemented, which
    /// is exactly what C# does for unregistered service methods.
    pub async fn handle_battlenet_request(&mut self, req: BattlenetRequest) {
        debug!(
            "BattlenetRequest from account {}: service=0x{:08X} method={} token={}",
            self.hub.shared().core.account_id,
            req.method.service_hash(),
            req.method.method_id(),
            req.method.token,
        );

        self.publication_like_cpp()
            .send_packet(&BattlenetResponse::error(
                req.method.service_hash(),
                req.method.method_id(),
                req.method.token,
                BattlenetRpcErrorCode::RpcNotImplemented,
            ));
    }

    /// C++ `WorldSession::HandleBattlenetChangeRealmTicket`.
    pub async fn handle_change_realm_ticket(&mut self, ticket: ChangeRealmTicket) {
        self.hub.core.set_realm_list_secret_like_cpp(ticket.secret);
        self.publication_like_cpp().send_packet(
            &ChangeRealmTicketResponse::allow_worldserver_realm_list_ticket_like_cpp(ticket.token),
        );
    }
}

/// Builds a battle.net handler context from a host's hub.
pub trait BattlenetHandlerHostLikeCpp<C> {
    fn battlenet_handler_cx_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a C,
    ) -> BattlenetHandlerCxLikeCpp<'a>;
}

fn handle_battlenet_request_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    mut pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: BattlenetHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        match BattlenetRequest::read(&mut pkt) {
            Ok(req) => {
                session
                    .battlenet_handler_cx_like_cpp(catalogs)
                    .handle_battlenet_request(req)
                    .await;
            }
            Err(e) => tracing::warn!("Failed to read BattlenetRequest: {e}"),
        }
    })
}

fn handle_change_realm_ticket_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    mut pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: BattlenetHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        match ChangeRealmTicket::read(&mut pkt) {
            Ok(ticket) => {
                session
                    .battlenet_handler_cx_like_cpp(catalogs)
                    .handle_change_realm_ticket(ticket)
                    .await;
            }
            Err(e) => tracing::warn!("Failed to read ChangeRealmTicket: {e}"),
        }
    })
}

/// Registers the battle.net handlers on the packet registry.
pub fn register_battlenet_handlers_like_cpp<S, C>(
    builder: &mut RegistryBuilder<S, C>,
) -> Result<(), DuplicateHandlerRegistrationLikeCpp>
where
    S: BattlenetHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::BattlenetRequest,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_battlenet_request",
        handler: handle_battlenet_request_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::ChangeRealmTicket,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_change_realm_ticket",
        handler: handle_change_realm_ticket_thunk::<S, C>,
    })?;
    Ok(())
}
