// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Battlenet service request handler.
//!
//! The client sends BattlenetRequest (CMSG 0x36FD) during character select
//! to invoke GameUtilitiesService RPCs. We respond with RpcNotImplemented
//! for all requests, matching C# behavior when no service handler is registered.

use tracing::debug;
use wow_constants::ClientOpcodes;
use wow_handler::{PacketHandlerEntry, PacketProcessing, SessionStatus};
use wow_packet::packets::battlenet::*;

use crate::session::WorldSession;

// ── Handler registration ────────────────────────────────────────────

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::BattlenetRequest,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_battlenet_request",
    }
}

// ── Handler implementation ──────────────────────────────────────────

impl WorldSession {
    /// Handle CMSG_BATTLENET_REQUEST — respond with RpcNotImplemented.
    ///
    /// C# dispatches these to GameUtilitiesService handlers. Since we don't
    /// implement any services yet, we always return RpcNotImplemented,
    /// which is exactly what C# does for unregistered service methods.
    pub async fn handle_battlenet_request(&mut self, req: BattlenetRequest) {
        debug!(
            "BattlenetRequest from account {}: service=0x{:08X} method={} token={}",
            self.account_id,
            req.method.service_hash(),
            req.method.method_id(),
            req.method.token,
        );

        self.send_packet(&BattlenetResponse::error(
            req.method.service_hash(),
            req.method.method_id(),
            req.method.token,
            BattlenetRpcErrorCode::RpcNotImplemented,
        ));
    }
}
