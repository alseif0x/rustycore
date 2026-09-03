// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use super::{ClientOpcodes, SessionState, SessionStatus, WorldPacket, debug, info, warn};

impl super::WorldSession {
    /// Dispatch a single packet to its registered handler.
    pub(crate) async fn dispatch_packet(
        &mut self,
        catalogs: &super::ObjectMgrCatalogsLikeCpp,
        mut pkt: WorldPacket,
    ) {
        let opcode_raw = pkt.opcode_raw();
        let opcode: ClientOpcodes = match num_traits::FromPrimitive::from_u32(u32::from(opcode_raw))
        {
            Some(op) => op,
            None => {
                info!(
                    "Unknown client opcode 0x{opcode_raw:04X} from account {}",
                    self.account_id
                );
                return;
            }
        };

        let entry = match self.dispatch_table.get(&opcode) {
            Some(e) => *e,
            None => {
                info!(
                    "No handler for {:?} (0x{opcode_raw:04X}) from account {}",
                    opcode, self.account_id
                );
                return;
            }
        };

        // Check session status
        if std::env::var_os("RUSTYCORE_PACKET_SEQUENCE_TRACE").is_some()
            && opcode == ClientOpcodes::RequestCemeteryList
        {
            info!(
                account = self.account_id,
                state = ?self.state,
                required = ?entry.status,
                handler = entry.handler_name,
                "RUST_CEMETERY_TRACE dispatch reached status gate"
            );
        }
        if !self.is_status_allowed(entry.status) {
            warn!(
                "Handler {} rejected: session state {:?} doesn't match required {:?}",
                entry.handler_name, self.state, entry.status
            );
            return;
        }

        debug!(
            "Dispatching {:?} via {} for account {}",
            opcode, entry.handler_name, self.account_id
        );

        // Skip opcode before reading payload
        pkt.skip_opcode();
        if std::env::var_os("RUSTYCORE_PACKET_SEQUENCE_TRACE").is_some()
            && opcode == ClientOpcodes::RequestCemeteryList
        {
            info!(
                account = self.account_id,
                state = ?self.state,
                packet_size = pkt.size(),
                remaining = pkt.remaining(),
                read_position = pkt.read_position(),
                "RUST_CEMETERY_TRACE skipped opcode"
            );
        }

        let cemetery_trace = std::env::var_os("RUSTYCORE_PACKET_SEQUENCE_TRACE").is_some()
            && opcode == ClientOpcodes::RequestCemeteryList;
        if cemetery_trace {
            info!(
                account = self.account_id,
                state = ?self.state,
                "RUST_CEMETERY_TRACE before handler call"
            );
        }

        // One mechanism: the registration carries the call, so the dispatcher
        // never names a handler method (#359).
        (entry.handler)(self, catalogs, pkt).await;

        if cemetery_trace {
            info!(
                account = self.account_id,
                state = ?self.state,
                "RUST_CEMETERY_TRACE after handler call"
            );
        }
    }

    /// Check if the handler's required status matches the current session state.
    ///
    /// Matches C++ `WorldSession::Update` status gates:
    /// - `Authed` → allowed in ANY state (authenticated, in-world, or transferring)
    /// - `LoggedIn` → only when player is in-world
    /// - `Transfer` → only during map transfers
    /// - `LoggedInOrRecentlyLogout` → in-world or recently disconnected
    fn is_status_allowed(&self, required: SessionStatus) -> bool {
        match required {
            SessionStatus::Authed => true, // C++ STATUS_AUTHED
            SessionStatus::LoggedIn => self.state == SessionState::LoggedIn,
            SessionStatus::Transfer => self.state == SessionState::Transfer,
            SessionStatus::LoggedInOrRecentlyLogout => {
                self.state == SessionState::LoggedIn || self.state == SessionState::Disconnecting
            }
        }
    }
}
