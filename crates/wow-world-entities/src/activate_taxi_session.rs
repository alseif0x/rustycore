// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! CMSG_ACTIVATE_TAXI (C++ `TaxiHandler.cpp:167` `HandleActivateTaxiOpcode`),
//! moved verbatim from the World `handlers/travel/travel.rs` (#1363 slice 4d).

use tracing::warn;
use wow_packet::ClientPacket;
use wow_packet::packets::misc::{ActivateTaxi, ActivateTaxiReply, ERR_TAXITOOFARAWAY_LIKE_CPP};
use wow_world_core::session::{RepresentedActivateTaxiLikeCpp, SessionHubHostLikeCpp};

/// The World operation the taxi activation still calls.
pub trait ActivateTaxiHostLikeCpp: SessionHubHostLikeCpp {
    /// `WorldSession::represented_taxi_usable_mount_displays_like_cpp`.
    fn represented_taxi_usable_mount_displays_like_cpp(&self, flying_mount_id: u32) -> Vec<i32>;
}

/// CMSG_ACTIVATE_TAXI.
///
/// C++ resolves `GetNPCIfCanInteractWith(Vendor, UNIT_NPC_FLAG_FLIGHTMASTER)`,
/// sends `ERR_TAXITOOFARAWAY` when that fails, then checks nearest taxi
/// node, known taximask nodes, preferred mount display, `TaxiPathGraph`,
/// and `Player::ActivateTaxiPathTo`.
///
/// Rust currently has represented NPC interaction and mount display filters,
/// but not `TaxiNodes.db2`, `TaxiPathGraph`, or live MotionMaster taxi
/// flight. This handler preserves packet/dispatch and the first C++ failure
/// reply, then records the accepted request for the future taxi runtime.
pub async fn handle_activate_taxi<H: ActivateTaxiHostLikeCpp>(
    host: &mut H,
    mut pkt: wow_packet::WorldPacket,
) {
    let activate = match ActivateTaxi::read(&mut pkt) {
        Ok(activate) => activate,
        Err(error) => {
            warn!("Bad ActivateTaxi: {error}");
            return;
        }
    };

    const NPC_FLAG_FLIGHT_MASTER: u32 = 0x2000;
    let can_interact = host
        .hub_ref()
        .represented_npc_can_interact_with_like_cpp(activate.vendor, NPC_FLAG_FLIGHT_MASTER, 0)
        .is_some()
        || host
            .core()
            .read_world_creature_like_cpp(activate.vendor, |creature| {
                creature.npc_flags() & NPC_FLAG_FLIGHT_MASTER != 0
            })
            .unwrap_or(false);

    if !can_interact {
        host.send_packet(&ActivateTaxiReply {
            reply: ERR_TAXITOOFARAWAY_LIKE_CPP,
        });
        return;
    }

    let preferred_mount_display = host
        .represented_taxi_usable_mount_displays_like_cpp(activate.flying_mount_id)
        .into_iter()
        .find_map(|display| u32::try_from(display).ok())
        .unwrap_or_default();

    host.hub_mut()
        .record_represented_activate_taxi_like_cpp(RepresentedActivateTaxiLikeCpp {
            vendor: activate.vendor,
            node: activate.node,
            ground_mount_id: activate.ground_mount_id,
            flying_mount_id: activate.flying_mount_id,
            preferred_mount_display,
        });
}
