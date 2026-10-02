// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use wow_network::InstanceLink;
#[cfg(any(test, feature = "test-fixtures"))]
use wow_network::SocketWriteFenceLikeCpp;

impl crate::session::state::SessionCore {
    /// Get the instance server address.
    pub fn instance_address(&self) -> [u8; 4] {
        self.transport.connection.instance_address()
    }

    /// Get the instance server port.
    pub fn instance_port(&self) -> u16 {
        self.transport.connection.instance_port()
    }

    /// Set the ConnectTo key.
    pub fn set_connect_to_key(&mut self, key: Option<i64>) {
        self.transport.connection.set_connect_to_key(key);
    }

    /// Set the ConnectTo serial.
    pub fn set_connect_to_serial(
        &mut self,
        serial: Option<wow_packet::packets::auth::ConnectToSerial>,
    ) {
        self.transport.connection.set_connect_to_serial(serial);
    }

    /// Set the instance link receiver.
    pub fn set_instance_link_rx(
        &mut self,
        rx: Option<tokio::sync::oneshot::Receiver<InstanceLink>>,
    ) {
        self.transport.connection.set_instance_link_rx(rx);
    }

    /// The channel currently delivering client packets.
    pub fn packet_rx(&self) -> &flume::Receiver<wow_packet::WorldPacket> {
        self.transport.connection.packet_rx()
    }

    /// The pending ConnectTo key, if a redirect is in flight.
    /// Test-only, like the kernel query it forwards to: production reads
    /// transport state through the operations above, never field by field.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn connect_to_key(&self) -> Option<i64> {
        self.transport.connection.connect_to_key()
    }

    /// Whether a ConnectTo serial is still recorded.
    /// Test-only, like the kernel query it forwards to: production reads
    /// transport state through the operations above, never field by field.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn has_connect_to_serial(&self) -> bool {
        self.transport.connection.has_connect_to_serial()
    }

    /// Whether an instance link receiver is installed and still awaited.
    /// Test-only, like the kernel query it forwards to: production reads
    /// transport state through the operations above, never field by field.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn is_awaiting_instance_link(&self) -> bool {
        self.transport.connection.is_awaiting_instance_link()
    }

    /// Whether a realm send channel is parked.
    /// Test-only, like the kernel query it forwards to: production reads
    /// transport state through the operations above, never field by field.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn has_parked_realm_send_channel(&self) -> bool {
        self.transport.connection.has_parked_realm_send_channel()
    }

    /// The channel a realm-routed packet takes.
    pub fn realm_route_tx(&self) -> &flume::Sender<Vec<u8>> {
        self.transport.connection.realm_route_tx()
    }

    /// A clone of the parked realm receive channel.
    pub fn realm_packet_rx(&self) -> Option<flume::Receiver<wow_packet::WorldPacket>> {
        self.transport.connection.realm_packet_rx()
    }

    /// Replace the primary receive channel.
    /// Test-only, like the kernel query it forwards to: production reads
    /// transport state through the operations above, never field by field.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_packet_rx(&mut self, rx: flume::Receiver<wow_packet::WorldPacket>) {
        self.transport.connection.set_packet_rx(rx);
    }

    /// Park a realm receive channel directly.
    ///
    /// Test-only, like the kernel setter it forwards to: parking a channel
    /// outside the atomic instance-link transition is not a production state.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn install_realm_packet_channel(
        &mut self,
        rx: flume::Receiver<wow_packet::WorldPacket>,
    ) {
        self.transport.connection.install_realm_packet_channel(rx);
    }

    /// Drop the parked realm receive channel after its writer disconnected.
    pub fn clear_realm_packet_rx(&mut self) {
        self.transport.connection.clear_realm_packet_rx();
    }

    /// Send a server packet on the **realm** connection.
    pub fn send_packet_realm(&self, pkt: &impl wow_packet::ServerPacket) {
        self.transport
            .connection
            .send_realm_bytes(pkt.to_bytes(), self.account_id);
    }

    /// Send pre-serialized packet bytes on the realm connection.
    pub fn send_raw_packet_realm(&self, data: &[u8]) {
        self.transport
            .connection
            .send_raw_packet_realm(data, self.account_id);
    }

    /// Wait for instance packets to be written before emitting a realm packet.
    pub async fn wait_for_instance_send_before_realm_send_like_cpp(&self) -> bool {
        self.transport
            .connection
            .wait_for_instance_send_before_realm_send_like_cpp(self.account_id)
            .await
    }

    /// Wait for realm packets to be written before emitting an instance update.
    pub async fn wait_for_realm_send_before_instance_update_like_cpp(&self) -> bool {
        self.transport
            .connection
            .wait_for_realm_send_before_instance_update_like_cpp(self.account_id)
            .await
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn install_realm_send_channel_for_test(&mut self, tx: flume::Sender<Vec<u8>>) {
        self.transport.connection.install_realm_send_channel(tx);
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn install_realm_send_write_fence_for_test(
        &mut self,
        fence: SocketWriteFenceLikeCpp,
    ) {
        self.transport
            .connection
            .install_realm_send_write_fence(fence);
    }
}
