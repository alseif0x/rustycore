// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Travel handlers that only need the hub and the lifecycle state.
//!
//! C++ source of truth: `WorldSession::HandleSuspendTokenResponse`,
//! `WorldSession::HandleTaxiNodeStatusQuery` and the STATUS_UNHANDLED
//! `CMSG_UPDATE_AREA_TRIGGER_VISUAL`
//! (`src/server/game/Handlers/MovementHandler.cpp`). The far-transfer
//! destination, the canonical creature registry and packet delivery already
//! live in the hub/lifecycle owners, so the World session only splits its
//! lifecycle state and builds the borrowed context (#1263 F5). The taxi
//! activation, area-trigger and world-port bodies stay in the shell while they
//! call shell-owned seams.

use tracing::{debug, info, warn};
use wow_constants::ClientOpcodes;
use wow_handler::{
    DuplicateHandlerRegistrationLikeCpp, HandlerFuture, PacketHandlerEntry, PacketProcessing,
    RegistryBuilder, SessionStatus,
};
use wow_packet::packets::misc::{NewWorld, TaxiNodeStatusPkt};
use wow_packet::{ServerPacket, WorldPacket};
use wow_world_core::session::{HubMut, SessionState};
use wow_world_lifecycle::SessionLifecycleState;

/// Borrowed inputs of one travel handler invocation.
pub struct TravelHandlerCxLikeCpp<'a> {
    hub: HubMut<'a>,
    lifecycle: &'a mut SessionLifecycleState,
}

impl<'a> TravelHandlerCxLikeCpp<'a> {
    pub fn new(hub: HubMut<'a>, lifecycle: &'a mut SessionLifecycleState) -> Self {
        Self { hub, lifecycle }
    }

    /// CMSG_SUSPEND_TOKEN_RESPONSE — the client confirms the suspended token
    /// while a far teleport is pending.
    ///
    /// C++ `WorldSession::HandleSuspendTokenResponse` (MovementHandler.cpp:239)
    /// replies with SMSG_NEW_WORLD so the client loads the destination map; only
    /// then does the client send CMSG_WORLD_PORT_RESPONSE. Without this step the
    /// client sits on the loading screen at 0%.
    pub async fn handle_suspend_token_response(&mut self, _pkt: WorldPacket) {
        if self.hub.shared().core.state == SessionState::Disconnecting {
            return;
        }
        if !self
            .hub
            .shared()
            .represented_far_teleport_pending_like_cpp()
        {
            return;
        }
        let Some((new_map, new_pos)) = self.lifecycle.pending_teleport_like_cpp(self.hub.shared())
        else {
            return;
        };
        let packet = NewWorld {
            map_id: new_map,
            pos: new_pos,
            reason: 16, // C++ Player.h NEW_WORLD_NORMAL (not the seamless value 21).
        };
        if self
            .hub
            .shared()
            .core
            .realm_route_tx()
            .send(ServerPacket::to_bytes(&packet))
            .is_err()
        {
            self.hub.core.kick("worldport NewWorld could not be queued");
            return;
        }
        self.lifecycle
            .recovery_new_world_sent_like_cpp(&mut self.hub);
        info!(
            account = self.hub.shared().core.account_id,
            map = new_map,
            "[FAR_TELEPORT] SuspendTokenResponse -> sent SMSG_NEW_WORLD (client now loads destination map)"
        );
    }

    /// CMSG_TAXI_NODE_STATUS_QUERY — answer the flight-master status of a unit.
    pub async fn handle_taxi_node_status_query(&mut self, mut pkt: WorldPacket) {
        let unit_guid = match pkt.read_packed_guid() {
            Ok(guid) => guid,
            Err(_) => {
                warn!("TaxiNodeStatusQuery: failed to read unit GUID");
                return;
            }
        };

        const NPC_FLAG_FLIGHT_MASTER: u32 = 0x2000;
        let is_flight_master = self
            .hub
            .core
            .mutate_world_creature(unit_guid, |creature| {
                creature.npc_flags() & NPC_FLAG_FLIGHT_MASTER != 0
            })
            .unwrap_or(false);

        // TaxiNodeStatus: 0=None, 1=Learned, 2=Unlearned, 3=NotEligible
        let status: u8 = if is_flight_master { 2 } else { 0 };

        debug!(
            account = self.hub.shared().core.account_id,
            ?unit_guid,
            status,
            "TaxiNodeStatusQuery"
        );
        self.hub
            .shared()
            .core
            .packet_publication_access_like_cpp()
            .send_packet(&TaxiNodeStatusPkt { unit_guid, status });
    }

    /// CMSG_UPDATE_AREA_TRIGGER_VISUAL — C++ registers it as
    /// STATUS_UNHANDLED/Handle_NULL.
    pub async fn handle_update_area_trigger_visual(&mut self, _pkt: WorldPacket) {}
}

/// Builds a travel handler context from a host's lifecycle state and hub.
pub trait TravelHandlerHostLikeCpp<C> {
    fn travel_handler_cx_like_cpp<'a>(&'a mut self, catalogs: &'a C) -> TravelHandlerCxLikeCpp<'a>;
}

fn handle_suspend_token_response_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: TravelHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .travel_handler_cx_like_cpp(catalogs)
            .handle_suspend_token_response(pkt)
            .await;
    })
}

fn handle_taxi_node_status_query_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: TravelHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .travel_handler_cx_like_cpp(catalogs)
            .handle_taxi_node_status_query(pkt)
            .await;
    })
}

fn handle_update_area_trigger_visual_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: TravelHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .travel_handler_cx_like_cpp(catalogs)
            .handle_update_area_trigger_visual(pkt)
            .await;
    })
}

/// Registers the travel handlers on the packet registry.
pub fn register_travel_handlers_like_cpp<S, C>(
    builder: &mut RegistryBuilder<S, C>,
) -> Result<(), DuplicateHandlerRegistrationLikeCpp>
where
    S: TravelHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::SuspendTokenResponse,
        status: SessionStatus::Transfer,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_suspend_token_response",
        handler: handle_suspend_token_response_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::TaxiNodeStatusQuery,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadSafe,
        handler_name: "handle_taxi_node_status_query",
        handler: handle_taxi_node_status_query_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::UpdateAreaTriggerVisual,
        status: SessionStatus::Authed,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_update_area_trigger_visual",
        handler: handle_update_area_trigger_visual_thunk::<S, C>,
    })?;
    Ok(())
}
