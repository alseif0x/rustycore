// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1363 slice 1: production-dead `state` entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl WorldSession {
    /// Parse the unresolved-placeholder CMSG_LEARN_TALENTS payload.
    ///
    /// C++ expands each `uint16` talent id into a `LearnTalent` request with
    /// `RequestedRank = 0` and delegates to `HandleLearnTalentOpcode`.
    /// The inspected C++ opcode table still uses the shared `0xBADD`
    /// placeholder, so this handler is deliberately not registered for live
    /// dispatch until the real client opcode is resolved.
    pub(crate) async fn handle_learn_talents(
        &mut self,
        talent_tabs: &wow_data::TalentTabStore,
        mut packet: WorldPacket,
    ) {
        let request = match LearnTalents::read(&mut packet) {
            Ok(request) => request,
            Err(error) => {
                warn!("Bad LearnTalents: {error}");
                return;
            }
        };

        for talent_id in request.talent_ids {
            if self.learn_represented_talent_like_cpp(talent_tabs, u32::from(talent_id), 0) {
                if let Some(talent_data) =
                    crate::session::hub_ref(self).resolved_update_talent_data_packet_like_cpp()
                {
                    self.send_packet(&talent_data);
                }
            }
        }
    }
}
