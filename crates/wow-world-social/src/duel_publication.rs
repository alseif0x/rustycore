use crate::SessionSocialLimits;
use wow_world_core::session::HubMut;
use wow_world_core::session::mailbox::SendRepresentedDuelRequestedLikeCppCommand;

impl SessionSocialLimits {
    pub fn handle_send_represented_duel_requested_command_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        command: SendRepresentedDuelRequestedLikeCppCommand,
    ) {
        self.set_represented_duel_arbiter_guid_like_cpp(hub, Some(command.arbiter_guid));
        hub.core.send_raw_packet(&command.packet_bytes);
    }
}
