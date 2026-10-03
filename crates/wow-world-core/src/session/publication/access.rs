// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use crate::session::SessionCore;

/// Borrowed access to the session's established packet publication channel.
pub struct PacketPublicationAccessLikeCpp<'a> {
    core: &'a SessionCore,
}

impl SessionCore {
    /// Build a borrowed capability for this session's packet channel.
    pub fn packet_publication_access_like_cpp(&self) -> PacketPublicationAccessLikeCpp<'_> {
        PacketPublicationAccessLikeCpp { core: self }
    }
}

impl PacketPublicationAccessLikeCpp<'_> {
    /// Publish one packet through the session's established send operation.
    pub fn send_packet<P: wow_packet::ServerPacket>(&self, packet: &P) -> bool {
        self.core.send_packet(packet)
    }
}
