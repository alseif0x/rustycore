// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    #[cfg(test)]
    pub(crate) fn connect_to_key(&self) -> Option<i64> {
        self.core.connect_to_key()
    }
    #[cfg(test)]
    pub(crate) fn has_connect_to_serial(&self) -> bool {
        self.core.has_connect_to_serial()
    }
    #[cfg(test)]
    pub(crate) fn is_awaiting_instance_link(&self) -> bool {
        self.core.is_awaiting_instance_link()
    }
    #[cfg(test)]
    pub(crate) fn has_parked_realm_send_channel(&self) -> bool {
        self.core.has_parked_realm_send_channel()
    }
    #[cfg(test)]
    pub(crate) fn set_packet_rx(&mut self, rx: flume::Receiver<wow_packet::WorldPacket>) {
        self.core.set_packet_rx(rx)
    }
    #[cfg(test)]
    pub(crate) fn install_realm_packet_channel(
        &mut self,
        rx: flume::Receiver<wow_packet::WorldPacket>,
    ) {
        self.core.install_realm_packet_channel(rx)
    }
    #[cfg(test)]
    pub(crate) fn install_realm_send_channel_for_test(&mut self, tx: flume::Sender<Vec<u8>>) {
        self.core.install_realm_send_channel_for_test(tx)
    }
    #[cfg(test)]
    pub(crate) fn install_realm_send_write_fence_for_test(
        &mut self,
        fence: SocketWriteFenceLikeCpp,
    ) {
        self.core.install_realm_send_write_fence_for_test(fence)
    }
    pub(crate) async fn wait_for_instance_send_before_realm_send_like_cpp(&self) -> bool {
        self.core
            .wait_for_instance_send_before_realm_send_like_cpp()
            .await
    }
    pub(crate) async fn wait_for_realm_send_before_instance_update_like_cpp(&self) -> bool {
        self.core
            .wait_for_realm_send_before_instance_update_like_cpp()
            .await
    }
}
