// Copyright (c) 2026 alseif0x
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

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ChangeRealmTicket,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_change_realm_ticket",
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

    /// Handle CMSG_CHANGE_REALM_TICKET like C++
    /// `WorldSession::HandleBattlenetChangeRealmTicket`.
    pub async fn handle_change_realm_ticket(&mut self, ticket: ChangeRealmTicket) {
        self.set_realm_list_secret_like_cpp(ticket.secret);
        self.send_packet(
            &ChangeRealmTicketResponse::allow_worldserver_realm_list_ticket_like_cpp(ticket.token),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wow_constants::ServerOpcodes;
    use wow_packet::WorldPacket;

    fn make_session() -> (WorldSession, flume::Receiver<Vec<u8>>) {
        let (_pkt_tx, pkt_rx) = flume::bounded::<WorldPacket>(8);
        let (send_tx, send_rx) = flume::bounded::<Vec<u8>>(8);
        (
            WorldSession::new(
                1,
                "tester".to_string(),
                0,
                0,
                0,
                0,
                vec![],
                "enUS".to_string(),
                pkt_rx,
                send_tx,
            ),
            send_rx,
        )
    }

    #[tokio::test]
    async fn change_realm_ticket_sets_secret_and_sends_cpp_response() {
        let (mut session, send_rx) = make_session();
        let secret = [0x5Au8; 32];

        session
            .handle_change_realm_ticket(ChangeRealmTicket {
                token: 0xCAFE_BABE,
                secret,
            })
            .await;

        assert_eq!(session.realm_list_secret_like_cpp(), &secret);

        let bytes = send_rx.try_recv().expect("change realm ticket response");
        let mut pkt = WorldPacket::from_bytes(&bytes);
        assert_eq!(
            pkt.read_uint16().unwrap(),
            ServerOpcodes::ChangeRealmTicketResponse as u16
        );
        assert_eq!(pkt.read_uint32().unwrap(), 0xCAFE_BABE);
        assert!(pkt.read_bit().unwrap());
        assert_eq!(
            pkt.read_uint32().unwrap(),
            "WorldserverRealmListTicket".len() as u32
        );
        assert_eq!(
            pkt.read_string("WorldserverRealmListTicket".len()).unwrap(),
            "WorldserverRealmListTicket"
        );
    }

    #[test]
    fn change_realm_ticket_handler_metadata_matches_cpp() {
        let entry = inventory::iter::<PacketHandlerEntry>
            .into_iter()
            .find(|entry| entry.opcode == ClientOpcodes::ChangeRealmTicket)
            .expect("ChangeRealmTicket handler entry");

        assert_eq!(entry.status, SessionStatus::Authed);
        assert_eq!(entry.processing, PacketProcessing::ThreadUnsafe);
        assert_eq!(entry.handler_name, "handle_change_realm_ticket");
    }
}
