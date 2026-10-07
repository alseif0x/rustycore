// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Loot handlers that only need the loot state and the hub.
//!
//! C++ source of truth: `src/server/game/Handlers/LootHandler.cpp`. The loot
//! specialization transition lives in the loot domain owner and the
//! specialization/class catalogs are reached through the hub, so the World
//! session only builds the borrowed context (#1263 F5). `CMSG_LOOT_UNIT` also
//! lives here as the C++ `WorldSession::HandleLootOpcode` body; the shell-only
//! capabilities it needs (spell/aura interruption, the represented AE-loot
//! search, the represented loot request/open transitions and the release
//! transition) stay behind the owner's host trait. The loot-item and money
//! bodies stay in the World shell while they need the item, quest and creature
//! orchestration.

use std::future::Future;

use tracing::{debug, warn};
use wow_constants::ClientOpcodes;
use wow_core::ObjectGuid;
use wow_handler::{
    DuplicateHandlerRegistrationLikeCpp, HandlerFuture, PacketHandlerEntry, PacketProcessing,
    RegistryBuilder, SessionStatus,
};
use wow_packet::packets::loot::{
    AELootTargets, AELootTargetsAck, LootResponse, LootUnit, SetLootSpecialization,
};
use wow_packet::{ClientPacket, WorldPacket};
use wow_world_core::session::{HubMut, HubRef, ItemValuationCatalogsLikeCpp};
use wow_world_loot::{LootState, RepresentedCreatureLootStateLikeCpp};

/// Borrowed inputs of one loot handler invocation.
pub struct LootHandlerCxLikeCpp<'a> {
    hub: HubMut<'a>,
    loot: &'a mut LootState,
}

impl<'a> LootHandlerCxLikeCpp<'a> {
    pub fn new(hub: HubMut<'a>, loot: &'a mut LootState) -> Self {
        Self { hub, loot }
    }

    /// CMSG_SET_LOOT_SPECIALIZATION — select or clear the loot specialization.
    ///
    /// C++ accepts non-zero values only when `sChrSpecializationStore` has the
    /// row and its `ClassID` matches the player's class; `SpecID == 0` clears.
    pub async fn handle_set_loot_specialization(&mut self, packet: SetLootSpecialization) {
        if self.hub.shared().core.player_guid().is_none() {
            return;
        }

        if packet.spec_id == 0 {
            self.loot
                .set_loot_specialization_id_like_cpp(&mut self.hub, 0);
            return;
        }

        let Some(store) = self.hub.catalogs.chr_specialization_store() else {
            return;
        };
        let Some(spec) = store.get(packet.spec_id) else {
            return;
        };
        if spec.class_id != self.hub.shared().player_class_like_cpp() {
            return;
        }

        self.loot
            .set_loot_specialization_id_like_cpp(&mut self.hub, packet.spec_id);
    }
}

/// Builds a loot handler context from a host's loot state and hub, and lends the
/// World shell capabilities the moved `CMSG_LOOT_UNIT` body still needs.
///
/// Every method delegates to the existing World operation at the exact point the
/// World body invoked it, so no World body is duplicated and no step value or
/// resumption is needed.
pub trait LootHandlerHostLikeCpp<C> {
    fn loot_handler_cx_like_cpp<'a>(&'a mut self, catalogs: &'a C) -> LootHandlerCxLikeCpp<'a>;

    /// Builds the application loot-release context from the host's disjoint state.
    fn loot_release_handler_cx_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a C,
    ) -> crate::LootReleaseCxLikeCpp<'a>;

    /// The process-owned item valuation catalogs of C++'s static item helpers
    /// (`Entities/Item/Item.cpp`). The World dispatch carries them in the
    /// handler-catalogs bundle this crate cannot name, so the host projects the
    /// capability the moved `HandleLootOpcode` body reads.
    fn loot_unit_item_valuation_like_cpp<'c>(
        &self,
        catalogs: &'c C,
    ) -> &'c ItemValuationCatalogsLikeCpp;

    /// The session hub, which the moved body uses for the player guid, the
    /// account id, `Player::IsAlive`, the player position and the packets.
    fn loot_unit_hub_ref_like_cpp(&self) -> HubRef<'_>;

    /// The loot state owner: the moved body reads the active non-item views and
    /// sets the active loot guid and the AE loot view owners on it.
    fn loot_unit_loot_ref_like_cpp(&self) -> &LootState;
    fn loot_unit_loot_mut_like_cpp(&mut self) -> &mut LootState;

    /// C++ `ObjectAccessor::GetCreature` plus the `AELootCreatureCheck` state of
    /// the corpse the body is about to open.
    fn loot_unit_represented_creature_loot_state_like_cpp(
        &mut self,
        guid: ObjectGuid,
    ) -> Option<RepresentedCreatureLootStateLikeCpp>;

    /// C++ `Player::InterruptNonMeleeSpells(false)` reached by `SendLoot`; the
    /// represented equivalent owns the session cast slots and the canonical Unit
    /// spell slots, which the application owner does not reach.
    fn loot_unit_interrupt_non_melee_spell_cast_like_cpp(&mut self);

    /// C++ `RemoveAurasWithInterruptFlags(SpellAuraInterruptFlags::Looting)`
    /// reads the session's aura container.
    fn loot_unit_remove_auras_with_looting_interrupt_flags_like_cpp(&mut self);

    /// C++ `Trinity::CreatureListSearcher<AELootCreatureCheck>`: the represented
    /// grid search, corpse-pool reconciliation and `isAllowedToLoot` reads live
    /// in the World loot tree.
    fn loot_unit_ae_loot_creature_targets_like_cpp<'a>(
        &'a mut self,
        main_loot_target: ObjectGuid,
        player_guid: ObjectGuid,
    ) -> impl Future<Output = Vec<ObjectGuid>> + Send + 'a;

    /// C++ `Player::SendLoot` request half: `GetLootForPlayer` plus
    /// `Player::isAllowedToLoot` over the represented loot authority.
    fn loot_unit_loot_response_for_owner_like_cpp<'a>(
        &'a mut self,
        owner_guid: ObjectGuid,
        player_guid: ObjectGuid,
        ae_looting: bool,
    ) -> impl Future<Output = Option<LootResponse>> + Send + 'a;

    /// The close-out half of C++ `Player::SendLoot` when the session still holds
    /// other loot views; it owns the application release context.
    fn loot_unit_release_all_like_cpp<'a>(
        &'a mut self,
        player_guid: ObjectGuid,
    ) -> impl Future<Output = ()> + Send + 'a;

    /// C++ `Player::SendLoot` publication: the response, the loot-list
    /// notification and the first-open loot generation stay in the World loot
    /// tree, which owns the represented loot authority.
    fn loot_unit_on_loot_opened_with_catalogs_like_cpp(
        &mut self,
        item_valuation: &ItemValuationCatalogsLikeCpp,
        owner_guid: ObjectGuid,
        player_guid: ObjectGuid,
        response: LootResponse,
    );
}

/// CMSG_LOOT_UNIT — player right-clicks a dead creature to loot it.
///
/// C++ `WorldSession::HandleLootOpcode` (`Handlers/LootHandler.cpp:216`,
/// registered `Opcodes.cpp:590` as `STATUS_LOGGEDIN`/`PROCESS_THREADUNSAFE`).
/// The body keeps its original parse, gates, order, log strings and packets;
/// only the access path changed.
async fn handle_loot_unit_with_catalogs_like_cpp<H, C>(
    host: &mut H,
    item_valuation: &ItemValuationCatalogsLikeCpp,
    mut pkt: WorldPacket,
) where
    H: LootHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    let req = match LootUnit::read(&mut pkt) {
        Ok(r) => r,
        Err(e) => {
            warn!("Bad LootUnit: {e}");
            return;
        }
    };

    let player_guid = match host.loot_unit_hub_ref_like_cpp().core.player_guid() {
        Some(g) => g,
        None => return,
    };

    debug!(account = host.loot_unit_hub_ref_like_cpp().core.account_id, target = ?req.unit, "CMSG_LOOT_UNIT");

    if host
        .loot_unit_hub_ref_like_cpp()
        .resolved_player_is_alive_like_cpp()
        != Some(true)
    {
        return;
    }

    if !req.unit.is_creature_or_vehicle() {
        return;
    }

    // Check creature exists and is dead.
    let creature_state = match host.loot_unit_represented_creature_loot_state_like_cpp(req.unit) {
        Some(state) => state,
        None => {
            warn!("LootUnit: creature {:?} not found", req.unit);
            return;
        }
    };

    if creature_state.is_alive {
        return;
    }

    if host
        .loot_unit_hub_ref_like_cpp()
        .player_position_like_cpp()
        .is_some_and(|player| !player.is_within_dist(&creature_state.position, 30.0))
    {
        return;
    }

    host.loot_unit_interrupt_non_melee_spell_cast_like_cpp();
    host.loot_unit_remove_auras_with_looting_interrupt_flags_like_cpp();

    let ae_owner_guids = if host
        .loot_unit_hub_ref_like_cpp()
        .config
        .enable_ae_loot_like_cpp()
    {
        host.loot_unit_ae_loot_creature_targets_like_cpp(req.unit, player_guid)
            .await
    } else {
        Vec::new()
    };

    if !ae_owner_guids.is_empty() {
        host.loot_unit_hub_ref_like_cpp()
            .core
            .send_packet(&AELootTargets {
                count: ae_owner_guids.len() as u32 + 1,
            });
    }

    let Some(response) = host
        .loot_unit_loot_response_for_owner_like_cpp(req.unit, player_guid, false)
        .await
    else {
        return;
    };
    if host
        .loot_unit_loot_ref_like_cpp()
        .has_active_non_item_loot_views_like_cpp()
    {
        host.loot_unit_release_all_like_cpp(player_guid).await;
    }
    host.loot_unit_loot_mut_like_cpp()
        .set_active_loot_guid(req.unit);
    host.loot_unit_on_loot_opened_with_catalogs_like_cpp(
        item_valuation,
        req.unit,
        player_guid,
        response,
    );

    if !ae_owner_guids.is_empty() {
        host.loot_unit_hub_ref_like_cpp()
            .core
            .send_packet(&AELootTargetsAck);

        for owner_guid in ae_owner_guids {
            if let Some(response) = host
                .loot_unit_loot_response_for_owner_like_cpp(owner_guid, player_guid, true)
                .await
            {
                host.loot_unit_loot_mut_like_cpp()
                    .add_active_loot_view_owner_like_cpp(owner_guid);
                host.loot_unit_on_loot_opened_with_catalogs_like_cpp(
                    item_valuation,
                    owner_guid,
                    player_guid,
                    response,
                );
                host.loot_unit_hub_ref_like_cpp()
                    .core
                    .send_packet(&AELootTargetsAck);
            }
        }
    }
}

fn handle_set_loot_specialization_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: LootHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        let mut pkt = pkt;
        match SetLootSpecialization::read(&mut pkt) {
            Ok(packet) => {
                session
                    .loot_handler_cx_like_cpp(catalogs)
                    .handle_set_loot_specialization(packet)
                    .await;
            }
            Err(e) => tracing::warn!("Failed to read SetLootSpecialization: {e}"),
        }
    })
}

fn handle_loot_release_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: LootHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .loot_release_handler_cx_like_cpp(catalogs)
            .handle_loot_release(pkt)
            .await;
    })
}

fn handle_loot_unit_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: LootHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        let item_valuation = session.loot_unit_item_valuation_like_cpp(catalogs);
        handle_loot_unit_with_catalogs_like_cpp::<S, C>(session, item_valuation, pkt).await;
    })
}

/// Registers the loot handlers on the packet registry.
pub fn register_loot_handlers_like_cpp<S, C>(
    builder: &mut RegistryBuilder<S, C>,
) -> Result<(), DuplicateHandlerRegistrationLikeCpp>
where
    S: LootHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::SetLootSpecialization,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_set_loot_specialization",
        handler: handle_set_loot_specialization_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::LootRelease,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_loot_release",
        handler: handle_loot_release_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::LootUnit,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_loot_unit",
        handler: handle_loot_unit_thunk::<S, C>,
    })?;
    Ok(())
}
