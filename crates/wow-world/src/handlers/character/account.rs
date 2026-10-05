// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Character enumeration, account-session operations, and account-scoped packet registrations.

use wow_packet::ClientPacket;

use super::*;

mod collections;
mod enumeration;
mod registrations;

impl WorldSession {
    /// Build and send SMSG_CONNECT_TO to the client.
    pub(super) fn send_connect_to(&mut self, serial: ConnectToSerial) {
        let session_mgr = match self.session_mgr() {
            Some(mgr) => Arc::clone(mgr),
            None => {
                warn!(
                    "No session manager for ConnectTo flow (account {}), sending login directly",
                    self.core.account_id
                );
                self.fallback_direct_login();
                return;
            }
        };

        // Generate ConnectToKey
        let key = ConnectToKey {
            account_id: self.core.account_id,
            connection_type: 1, // Instance
            key: rand::thread_rng().gen_range(0..0x7FFF_FFFF_u32),
        };
        let key_raw = key.raw();
        self.set_connect_to_key(Some(key_raw));
        self.set_connect_to_serial(Some(serial));

        // Register in SessionManager — returns oneshot receiver for instance link
        let rx = session_mgr.register(
            self.core.account_id,
            key_raw,
            self.core.transport.session_key.clone(),
        );
        self.set_instance_link_rx(Some(rx));

        // Build the ConnectTo payload
        let addr = self.instance_address();
        let port = self.instance_port();

        // Build where_buffer for RSA signature: [type(1B)][ip(4B)]
        let mut where_buffer = Vec::with_capacity(5);
        where_buffer.push(1u8); // IPv4
        where_buffer.extend_from_slice(&addr);

        let signature = rsa_sign_connect_to(&where_buffer, 1, port);

        let connect_to = ConnectTo {
            signature,
            address: ConnectToAddress::IPv4(addr),
            port,
            serial,
            con: 1, // Instance
            key: key_raw,
        };

        info!(
            "Sending ConnectTo (serial={:?}) to account {} for instance {}:{port}",
            serial,
            self.core.account_id,
            format!("{}.{}.{}.{}", addr[0], addr[1], addr[2], addr[3])
        );

        self.send_packet(&connect_to);
    }

    /// Handle CMSG_REQUEST_PLAYED_TIME (0x327A).
    ///
    /// C# ref: `MiscHandler.HandlePlayedTime`.
    /// Client sends this when the player types `/played`.
    /// We respond with total and level played time in seconds.
    /// `trigger_event` mirrors the client flag (TriggerScriptEvent).
    /// Handle CMSG_HOTFIX_REQUEST — client requests hotfix data.
    /// Borrows C++ `sDB2Manager.GetHotfixData()`; Session owns no catalog.
    /// C++ `Handlers/HotfixHandler.cpp:77-135`.
    /// Test-only entry point for the hotfix handler owned by
    /// `wow-world-application` (#1263 F5).
    #[cfg(test)]
    pub async fn handle_hotfix_request(
        &mut self,
        cache: &wow_data::HotfixBlobCache,
        req: wow_packet::packets::misc::HotfixRequest,
    ) {
        let tact_keys = self
            .tact_key_store_for_test_like_cpp()
            .cloned()
            .unwrap_or_else(|| Arc::new(wow_data::TactKeyStore::from_entries([])));
        wow_world_application::DataServiceHandlerCxLikeCpp::new(
            crate::session::hub_mut(self),
            cache,
            tact_keys.as_ref(),
        )
        .handle_hotfix_request(req)
        .await;
    }

    /// Handle ConnectToFailed — client couldn't connect to instance port.
    ///
    /// Retry with the next serial, or fall back to direct login if all retries
    /// are exhausted.
    pub async fn handle_connect_to_failed(&mut self, pkt: ConnectToFailed) {
        warn!(
            "ConnectToFailed (serial={:?}) from account {}",
            pkt.serial, self.core.account_id
        );

        // Clean up the pending entry from SessionManager
        if let Some(mgr) = self.session_mgr() {
            mgr.remove(self.core.account_id);
        }
        self.set_instance_link_rx(None);

        // Try next serial
        if let Some(next_serial) = pkt.serial.next() {
            info!("Retrying ConnectTo with serial {:?}", next_serial);
            self.send_connect_to(next_serial);
        } else {
            warn!(
                "All ConnectTo retries exhausted for account {}, aborting login like C++",
                self.core.account_id
            );
            self.set_player_loading(None);
            self.lifecycle.release_character_login_claim_like_cpp();
            self.set_connect_to_key(None);
            self.set_connect_to_serial(None);
            self.send_packet(&CharacterLoginFailed {
                code: LoginFailureReasonLikeCpp::NoWorld,
            });
        }
    }
}
