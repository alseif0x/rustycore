// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The CMSG_LEARN_TALENT handler, moved from the World
//! `handlers/talent/state.rs` (#1363 slice 9b). The registration stays in
//! `wow_world_application::register_talent_handlers_like_cpp`; the World
//! adapter `handlers/talent/host.rs` calls it with the session as host.

use tracing::warn;
use wow_packet::packets::talent::LearnTalent;
use wow_packet::{ClientPacket, WorldPacket};

use super::{TalentHostLikeCpp, learn_represented_talent_like_cpp};

/// Handle CMSG_LEARN_TALENT.
///
/// C++ calls `Player::LearnTalent(TalentID, RequestedRank)` and sends
/// `SendTalentsInfoData()` only on success. Rust currently has the
/// represented talent snapshot plus DB2 spell-rank and talent-tab class
/// validation, but not the complete C++ point/prerequisite/tier runtime.
pub async fn handle_learn_talent<H: TalentHostLikeCpp>(
    host: &mut H,
    talent_tabs: &wow_data::TalentTabStore,
    mut packet: WorldPacket,
) {
    let request = match LearnTalent::read(&mut packet) {
        Ok(request) => request,
        Err(error) => {
            warn!("Bad LearnTalent: {error}");
            return;
        }
    };

    if request.talent_id < 0 {
        return;
    }

    if learn_represented_talent_like_cpp(
        host,
        talent_tabs,
        request.talent_id as u32,
        request.requested_rank,
    ) && let Some(talent_data) = host.hub_ref().resolved_update_talent_data_packet_like_cpp()
    {
        host.send_packet(&talent_data);
    }
}
