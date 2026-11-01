// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Battle.net request regressions.
//!
//! The battle.net service and realm-list ticket handlers moved to
//! `wow-world-lifecycle` in #1263 F5; this module keeps the cfg(test)
//! delegates the in-file regressions drive.

#[cfg(test)]
mod test_shims;

#[cfg(test)]
mod tests {
    use wow_constants::{ClientOpcodes, ServerOpcodes};
    use wow_handler::{PacketProcessing, SessionStatus};
    use wow_packet::WorldPacket;
    use wow_packet::packets::battlenet::ChangeRealmTicket;

    use crate::session::WorldSession;

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
                crate::session::registry::build_dispatch_table(),
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
        let entry = crate::session::registry::registered_handler_entries_like_cpp()
            .into_iter()
            .find(|entry| entry.opcode == ClientOpcodes::ChangeRealmTicket)
            .expect("ChangeRealmTicket handler entry");

        assert_eq!(entry.status, SessionStatus::Authed);
        assert_eq!(entry.processing, PacketProcessing::ThreadUnsafe);
        assert_eq!(entry.handler_name, "handle_change_realm_ticket");
    }
}
