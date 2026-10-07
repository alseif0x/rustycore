// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! NPC handlers for the C++ `NPCHandler.cpp` family.
//!
//! C++ source of truth: `src/server/game/Handlers/NPCHandler.cpp`
//! (`HandleTabardVendorActivateOpcode`, `HandleSpiritHealerActivate` and
//! `HandleRequestStabledPets`, the handlers behind
//! `CMSG_TABARD_VENDOR_ACTIVATE`, `CMSG_SPIRIT_HEALER_ACTIVATE` and
//! `CMSG_REQUEST_STABLED_PETS`). The family owns the packet bodies and the
//! represented interaction gate they enforce. More of the NPC family lands
//! later; the account-scoped session core and the represented NPC interaction
//! access already live in the Core hub, so the World session only builds the
//! borrowed context (#1263 F5).

use tracing::{debug, info, warn};
use wow_constants::ClientOpcodes;
use wow_constants::unit::NPCFlags1;
use wow_handler::{
    DuplicateHandlerRegistrationLikeCpp, HandlerFuture, PacketHandlerEntry, PacketProcessing,
    RegistryBuilder, SessionStatus,
};
use wow_packet::packets::misc::{RequestStabledPets, SpiritHealerActivate};
use wow_packet::{ClientPacket, WorldPacket};
use wow_world_core::session::HubMut;

/// Borrowed inputs of one NPC handler invocation.
pub struct NpcHandlerCxLikeCpp<'a> {
    hub: HubMut<'a>,
}

impl<'a> NpcHandlerCxLikeCpp<'a> {
    pub fn new(hub: HubMut<'a>) -> Self {
        Self { hub }
    }

    /// CMSG_TABARD_VENDOR_ACTIVATE — player talks to a tabard designer.
    /// C++ refs: `HandleTabardVendorActivateOpcode` /
    /// `SendTabardVendorActivate` (`Handlers/NPCHandler.cpp:49-91`).
    pub async fn handle_tabard_vendor_activate(&mut self, mut pkt: wow_packet::WorldPacket) {
        use wow_packet::packets::misc::NpcInteractionOpenResult;
        let guid = pkt
            .read_packed_guid()
            .unwrap_or(wow_core::ObjectGuid::EMPTY);
        info!(
            "TabardVendorActivate {:?} account {}",
            guid,
            self.hub.shared().core.account_id
        );
        self.hub
            .core
            .send_packet(&NpcInteractionOpenResult::new(guid, 14)); // GuildTabardVendor
    }

    /// CMSG_SPIRIT_HEALER_ACTIVATE — ghost uses spirit healer.
    /// C++ ref: `WorldSession::HandleSpiritHealerActivate`.
    pub async fn handle_spirit_healer_activate(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match SpiritHealerActivate::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.hub.shared().core.account_id,
                    "SpiritHealerActivate parse failed: {error}"
                );
                return;
            }
        };

        let Some(_healer) = self
            .hub
            .shared()
            .represented_npc_can_interact_with_like_cpp(
                request.healer,
                NPCFlags1::SPIRIT_HEALER.bits(),
                0,
            )
        else {
            debug!(
                account = self.hub.shared().core.account_id,
                healer = ?request.healer,
                "SpiritHealerActivate ignored without represented spirit healer"
            );
            return;
        };

        // C++ continues into SendSpiritResurrect here: resurrect 50%, durability
        // loss, corpse-bones spawn, and possible graveyard teleport. That player
        // corpse/death runtime is not represented in this handler yet.
        debug!(
            account = self.hub.shared().core.account_id,
            healer = ?request.healer,
            "SpiritHealerActivate validated; resurrection runtime pending"
        );
    }

    /// CMSG_REQUEST_STABLED_PETS — player opens stable master UI.
    /// C++ ref: `WorldSession::HandleRequestStabledPets`.
    pub async fn handle_request_stabled_pets(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match RequestStabledPets::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.hub.shared().core.account_id,
                    "RequestStabledPets parse failed: {error}"
                );
                return;
            }
        };

        // C++ returns before sending anything when CheckStableMaster fails.
        // The live stable-master validation and Player::SetStableMaster update
        // fields are not ported here yet, so preserve that observable branch.
        debug!(
            account = self.hub.shared().core.account_id,
            stable_master = ?request.stable_master,
            "RequestStabledPets ignored without represented stable-master runtime"
        );
    }
}

/// Builds an NPC handler context from a host's hub.
pub trait NpcHandlerHostLikeCpp<C> {
    fn npc_handler_cx_like_cpp<'a>(&'a mut self, catalogs: &'a C) -> NpcHandlerCxLikeCpp<'a>;
}

fn handle_tabard_vendor_activate_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: NpcHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .npc_handler_cx_like_cpp(catalogs)
            .handle_tabard_vendor_activate(pkt)
            .await;
    })
}

fn handle_spirit_healer_activate_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: NpcHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .npc_handler_cx_like_cpp(catalogs)
            .handle_spirit_healer_activate(pkt)
            .await;
    })
}

fn handle_request_stabled_pets_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: NpcHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .npc_handler_cx_like_cpp(catalogs)
            .handle_request_stabled_pets(pkt)
            .await;
    })
}

pub fn register_npc_handlers_like_cpp<S, C>(
    builder: &mut RegistryBuilder<S, C>,
) -> Result<(), DuplicateHandlerRegistrationLikeCpp>
where
    S: NpcHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::TabardVendorActivate,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_tabard_vendor_activate",
        handler: handle_tabard_vendor_activate_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::SpiritHealerActivate,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_spirit_healer_activate",
        handler: handle_spirit_healer_activate_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::RequestStabledPets,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_request_stabled_pets",
        handler: handle_request_stabled_pets_thunk::<S, C>,
    })?;
    Ok(())
}
