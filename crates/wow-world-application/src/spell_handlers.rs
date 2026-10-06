// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Spell cancel handler family.
//!
//! C++ source of truth: `SpellHandler.cpp` (`HandleCancelCast`,
//! `HandleCancelAuraOpcode`, `HandleCancelAutoRepeatSpellOpcode`,
//! `HandleCancelChanneling`, `HandleCancelGrowthAuraOpcode`,
//! `HandleCancelMountAuraOpcode`, `HandleCancelQueuedSpellOpcode`,
//! `HandlePetCancelAuraOpcode`, `HandleTotemDestroyed`). The family owns the
//! packet bodies and the cast/aura cancellation transitions; the World session
//! only lends the aura-application participants and the hub (#1263 F5).
//! Bodies are moved unchanged from the World shell.

use tracing::{debug, warn};
use wow_constants::{ClientOpcodes, SpellCastResult};
use wow_core::ObjectGuid;
use wow_entities::RepresentedAuraEffectLikeCpp;
use wow_handler::{
    DuplicateHandlerRegistrationLikeCpp, HandlerFuture, PacketHandlerEntry, PacketProcessing,
    RegistryBuilder, SessionStatus,
};
use wow_packet::packets::pet::PetCancelAura;
use wow_packet::packets::spell::{
    CancelAura, CancelAutoRepeatSpell, CancelCast, CancelChannelling, CancelGrowthAura,
    CancelMountAura, CancelQueuedSpell,
};
use wow_packet::packets::totem::TotemDestroyed;
use wow_packet::{ClientPacket, WorldPacket};
use wow_world_core::session::HubMut;
use wow_world_inventory::InventoryState;
use wow_world_loot::LootState;
use wow_world_spell::SessionSpellState;

/// C++ `Unit` current-cast execution state, with the represented fixture
/// fallback while no Player handle is installed.
pub fn mutate_cast_execution_like_cpp<R>(
    hub: &mut HubMut<'_>,
    spell_state: &mut SessionSpellState,
    f: impl FnOnce(&mut wow_entities::CastExecutionStateLikeCpp) -> R,
) -> Option<R> {
    #[cfg(any(test, feature = "test-fixtures"))]
    if hub.shared().core.player_handle_like_cpp.is_none() {
        return spell_state.mutate_cast_execution_fixture_like_cpp(f);
    }
    #[cfg(not(any(test, feature = "test-fixtures")))]
    let _ = &spell_state;
    hub.core.with_owned_player_mut_like_cpp(|player| {
        f(&mut player.unit_mut().subsystems_mut().spells.execution)
    })
}

/// C++ `Player::CancelPendingCastRequest`: drop the queued request and report
/// it with `SPELL_FAILED_DONT_REPORT`.
pub fn cancel_pending_spell_cast_request_like_cpp(
    hub: &mut HubMut<'_>,
    spell_state: &mut SessionSpellState,
) -> bool {
    #[cfg(any(test, feature = "test-fixtures"))]
    let owned = if hub.shared().core.player_handle_like_cpp.is_none() {
        Some(spell_state.mutate_pending_spell_cast_fixture_like_cpp(Option::take))
    } else {
        hub.core.with_owned_player_mut_like_cpp(
            wow_entities::Player::cancel_pending_spell_cast_like_cpp,
        )
    };
    #[cfg(not(any(test, feature = "test-fixtures")))]
    let owned = {
        let _ = &spell_state;
        hub.core.with_owned_player_mut_like_cpp(
            wow_entities::Player::cancel_pending_spell_cast_like_cpp,
        )
    };
    let Some(request) = owned.flatten() else {
        return false;
    };

    hub.core
        .send_packet(&wow_packet::packets::spell::CastFailed {
            cast_id: request.cast_id,
            spell_id: request.spell_id,
            visual: Default::default(),
            reason: SpellCastResult::DontReport as i32,
            fail_arg1: 0,
            fail_arg2: 0,
        });
    true
}

/// Take the interrupted cast and publish its interruption frames.
pub fn interrupt_player_cast_like_cpp(
    hub: &mut HubMut<'_>,
    spell_state: &mut SessionSpellState,
    spell_id: Option<i32>,
) -> bool {
    let Some(cast) = mutate_cast_execution_like_cpp(hub, spell_state, |state| {
        state.take_interrupted_cast(spell_id)
    })
    .flatten() else {
        return false;
    };
    spell_state.publish_player_cast_interruption_like_cpp(hub, cast);
    true
}

/// Remove every cancelable visible aura carrying one represented effect.
pub fn remove_represented_cancelable_auras_by_effect_like_cpp(
    mut hub: HubMut<'_>,
    spell_state: &mut SessionSpellState,
    inventory: &mut InventoryState,
    loot: &LootState,
    #[cfg(any(test, feature = "test-fixtures"))] quest_state: &crate::SessionQuestState,
    world_test_consumer: bool,
    represented_effect: RepresentedAuraEffectLikeCpp,
) -> usize {
    let no_aura_cancel = wow_data::spell::attributes::SPELL_ATTR0_NO_AURA_CANCEL;
    let Some(visible_auras) = hub.shared().resolved_player_visible_auras_like_cpp() else {
        return 0;
    };
    let slots: Vec<u8> = visible_auras
        .values()
        .filter_map(|aura| {
            // C++ removes SPELL_AURA_MOUNTED only when its SpellInfo is
            // cancelable, positive, and non-passive; the same predicate is
            // used for SPELL_AURA_MOD_SCALE in CancelGrowthAura. These
            // represented effects model positive player-cancelable paths;
            // SpellMisc attributes preserve the C++ no-player-cancel gate.
            if hub
                .catalogs
                .spell_catalogs
                .spell_store
                .as_ref()
                .is_some_and(|store| store.has_attribute0_like_cpp(aura.spell_id, no_aura_cancel))
            {
                return None;
            }
            (aura.represented_effect == Some(represented_effect)).then_some(aura.slot)
        })
        .collect();

    let removed = slots.len();
    for slot in slots {
        let _ = crate::player_aura_application_cx_like_cpp(
            hub.reborrow_like_cpp(),
            &mut *spell_state,
            &mut *inventory,
            loot,
            #[cfg(any(test, feature = "test-fixtures"))]
            quest_state,
            world_test_consumer,
        )
        .remove_aura_like_cpp(slot);
    }
    removed
}

/// Borrowed inputs of one spell cancel handler invocation: the hub plus the
/// participants of the application aura-application context.
pub struct SpellHandlerCxLikeCpp<'a> {
    hub: HubMut<'a>,
    spell_state: &'a mut SessionSpellState,
    inventory: &'a mut InventoryState,
    loot: &'a LootState,
    #[cfg(any(test, feature = "test-fixtures"))]
    quest_state: &'a crate::SessionQuestState,
    /// The host's World-test flag (World passes `cfg!(test)`).
    world_test_consumer: bool,
}

impl<'a> SpellHandlerCxLikeCpp<'a> {
    pub fn new(
        hub: HubMut<'a>,
        spell_state: &'a mut SessionSpellState,
        inventory: &'a mut InventoryState,
        loot: &'a LootState,
        #[cfg(any(test, feature = "test-fixtures"))] quest_state: &'a crate::SessionQuestState,
        world_test_consumer: bool,
    ) -> Self {
        Self {
            hub,
            spell_state,
            inventory,
            loot,
            #[cfg(any(test, feature = "test-fixtures"))]
            quest_state,
            world_test_consumer,
        }
    }

    fn remove_aura_like_cpp(&mut self, slot: u8) -> Result<(), &'static str> {
        crate::player_aura_application_cx_like_cpp(
            self.hub.reborrow_like_cpp(),
            &mut *self.spell_state,
            &mut *self.inventory,
            self.loot,
            #[cfg(any(test, feature = "test-fixtures"))]
            self.quest_state,
            self.world_test_consumer,
        )
        .remove_aura_like_cpp(slot)
    }

    fn remove_represented_cancelable_auras_by_effect_like_cpp(
        &mut self,
        represented_effect: RepresentedAuraEffectLikeCpp,
    ) -> usize {
        remove_represented_cancelable_auras_by_effect_like_cpp(
            self.hub.reborrow_like_cpp(),
            &mut *self.spell_state,
            &mut *self.inventory,
            self.loot,
            #[cfg(any(test, feature = "test-fixtures"))]
            self.quest_state,
            self.world_test_consumer,
            represented_effect,
        )
    }

    pub fn remove_represented_growth_auras_cancelable_like_cpp(&mut self) -> usize {
        self.remove_represented_cancelable_auras_by_effect_like_cpp(
            RepresentedAuraEffectLikeCpp::ModScale,
        )
    }

    pub fn remove_represented_mount_auras_cancelable_like_cpp(&mut self) -> usize {
        self.remove_represented_cancelable_auras_by_effect_like_cpp(
            RepresentedAuraEffectLikeCpp::Mounted,
        )
    }

    pub fn remove_represented_cancelable_owned_aura_like_cpp(
        &mut self,
        spell_id: i32,
        caster_guid: ObjectGuid,
    ) -> usize {
        let Some(spell_store) = self.hub.catalogs.spell_catalogs.spell_store.as_ref() else {
            return 0;
        };
        if spell_store.get(spell_id).is_none()
            || spell_store.has_attribute0_like_cpp(
                spell_id,
                wow_data::spell::attributes::SPELL_ATTR0_NO_AURA_CANCEL,
            )
            || spell_store.is_channeled_like_cpp(spell_id)
            || spell_store.is_passive_like_cpp(spell_id)
        {
            return 0;
        }

        let Some(visible_auras) = self.hub.shared().resolved_player_visible_auras_like_cpp() else {
            return 0;
        };
        let slots: Vec<u8> = visible_auras
            .values()
            .filter_map(|aura| {
                if aura.spell_id != spell_id {
                    return None;
                }
                if !caster_guid.is_empty() && aura.caster_guid != caster_guid {
                    return None;
                }
                // C++ checks SpellInfo before RemoveOwnedAura: no
                // SPELL_ATTR0_NO_AURA_CANCEL, positive, and non-passive.
                // Full SpellInfo::IsPositive is not represented yet; allow
                // the locally materialized positive/cancelable aura shapes,
                // including the single-effect generic represented aura.
                (aura.represented_effect.is_none()
                    || matches!(
                        aura.represented_effect,
                        Some(
                            RepresentedAuraEffectLikeCpp::Mounted
                                | RepresentedAuraEffectLikeCpp::ModScale
                                | RepresentedAuraEffectLikeCpp::ModSpeedNoControl
                        )
                    ))
                .then_some(aura.slot)
            })
            .collect();

        let removed = slots.len();
        for slot in slots {
            let _ = self.remove_aura_like_cpp(slot);
        }
        removed
    }

    fn interrupt_current_channeled_spell_like_cpp(&mut self, spell_id: i32) -> bool {
        let Ok(spell_id) = u32::try_from(spell_id) else {
            return false;
        };
        if spell_id == 0 {
            return false;
        }

        let interrupted = self
            .hub
            .core
            .mutate_canonical_player_like_cpp(|player| {
                let unit = player.unit_mut();
                if unit
                    .current_spell(wow_entities::CurrentSpellSlot::Channeled)
                    .is_none_or(|current| current.spell_id != spell_id)
                {
                    return false;
                }
                unit.interrupt_spell(wow_entities::CurrentSpellSlot::Channeled, true, true)
                    .is_some()
            })
            .unwrap_or(false);

        if interrupted {
            let _ = interrupt_player_cast_like_cpp(
                &mut self.hub,
                self.spell_state,
                Some(spell_id as i32),
            );
        }

        interrupted
    }

    pub fn cancel_client_cast_request_like_cpp(&mut self, spell_id: Option<i32>) {
        let Some((had_active, cast)) =
            mutate_cast_execution_like_cpp(&mut self.hub, self.spell_state, |state| {
                (
                    state.active.is_some(),
                    state.take_interrupted_cast(spell_id),
                )
            })
        else {
            return;
        };
        if let Some(cast) = cast {
            self.spell_state
                .publish_player_cast_interruption_like_cpp(&mut self.hub, cast);
        }
        if had_active {
            cancel_pending_spell_cast_request_like_cpp(&mut self.hub, self.spell_state);
        }
    }

    /// Handle `CMSG_CANCEL_CAST` — player cancels an in-progress cast.
    pub async fn handle_cancel_cast(&mut self, mut pkt: WorldPacket) {
        let request = match CancelCast::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.hub.shared().core.account_id,
                    "CancelCast parse failed: {error}"
                );
                return;
            }
        };

        self.cancel_client_cast_request_like_cpp(
            (request.spell_id != 0).then_some(request.spell_id as i32),
        );
    }

    /// Handle `CMSG_CANCEL_AURA` — player requests removing a cancelable owned aura.
    pub async fn handle_cancel_aura(&mut self, mut pkt: WorldPacket) {
        let request = match CancelAura::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.hub.shared().core.account_id,
                    "CancelAura parse failed: {error}"
                );
                return;
            }
        };

        debug!(
            account = self.hub.shared().core.account_id,
            spell_id = request.spell_id,
            caster_guid = ?request.caster_guid,
            "CMSG_CANCEL_AURA parsed"
        );
        let Some(spell_store) = self.hub.catalogs.spell_store() else {
            return;
        };
        if spell_store.get(request.spell_id).is_none()
            || spell_store.has_attribute0_like_cpp(
                request.spell_id,
                wow_data::spell::attributes::SPELL_ATTR0_NO_AURA_CANCEL,
            )
        {
            return;
        }
        if spell_store.is_channeled_like_cpp(request.spell_id) {
            self.interrupt_current_channeled_spell_like_cpp(request.spell_id);
            return;
        }
        if spell_store.is_passive_like_cpp(request.spell_id) {
            return;
        }
        self.remove_represented_cancelable_owned_aura_like_cpp(
            request.spell_id,
            request.caster_guid,
        );
    }

    /// Handle `CMSG_CANCEL_AUTO_REPEAT_SPELL`.
    pub async fn handle_cancel_auto_repeat_spell(&mut self, mut pkt: WorldPacket) {
        if let Err(error) = CancelAutoRepeatSpell::read(&mut pkt) {
            warn!(
                account = self.hub.shared().core.account_id,
                "CancelAutoRepeatSpell parse failed: {error}"
            );
        }
        // C++ interrupts CURRENT_AUTOREPEAT_SPELL. Rust does not yet represent
        // a separate auto-repeat current-spell slot, so this remains silent.
    }

    /// Handle `CMSG_CANCEL_CHANNELLING` — player stops a channelled spell.
    pub async fn handle_cancel_channelling(&mut self, mut pkt: WorldPacket) {
        let request = match CancelChannelling::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.hub.shared().core.account_id,
                    "CancelChannelling parse failed: {error}"
                );
                return;
            }
        };

        let Some(spell_store) = self.hub.catalogs.spell_store() else {
            return;
        };

        if spell_store.get(request.channel_spell).is_none()
            || spell_store.has_attribute0_like_cpp(
                request.channel_spell,
                wow_data::spell::attributes::SPELL_ATTR0_NO_AURA_CANCEL,
            )
        {
            return;
        }

        debug!(
            account = self.hub.shared().core.account_id,
            channel_spell = request.channel_spell,
            reason = request.reason,
            "CMSG_CANCEL_CHANNELLING parsed"
        );
        self.interrupt_current_channeled_spell_like_cpp(request.channel_spell);
    }

    /// Handle `CMSG_CANCEL_GROWTH_AURA`.
    pub async fn handle_cancel_growth_aura(&mut self, mut pkt: WorldPacket) {
        if let Err(error) = CancelGrowthAura::read(&mut pkt) {
            warn!(
                account = self.hub.shared().core.account_id,
                "CancelGrowthAura parse failed: {error}"
            );
        }
        self.remove_represented_growth_auras_cancelable_like_cpp();
    }

    /// Handle `CMSG_CANCEL_MOUNT_AURA`.
    pub async fn handle_cancel_mount_aura(&mut self, mut pkt: WorldPacket) {
        if let Err(error) = CancelMountAura::read(&mut pkt) {
            warn!(
                account = self.hub.shared().core.account_id,
                "CancelMountAura parse failed: {error}"
            );
        }
        self.remove_represented_mount_auras_cancelable_like_cpp();
    }

    /// Handle `CMSG_CANCEL_QUEUED_SPELL`.
    pub async fn handle_cancel_queued_spell(&mut self, mut pkt: WorldPacket) {
        if let Err(error) = CancelQueuedSpell::read(&mut pkt) {
            warn!(
                account = self.hub.shared().core.account_id,
                "CancelQueuedSpell parse failed: {error}"
            );
            return;
        }
        // C++ cancels `Player::CancelPendingCastRequest`, not the current
        // non-melee spell. The represented queue is separate from
        // `active_spell_cast`, so this keeps casts already in progress alive.
        cancel_pending_spell_cast_request_like_cpp(&mut self.hub, self.spell_state);
    }

    /// Handle `CMSG_PET_CANCEL_AURA`.
    pub async fn handle_pet_cancel_aura(&mut self, mut pkt: WorldPacket) {
        let request = match PetCancelAura::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.hub.shared().core.account_id,
                    "PetCancelAura parse failed: {error}"
                );
                return;
            }
        };

        debug!(
            account = self.hub.shared().core.account_id,
            pet_guid = ?request.pet_guid,
            spell_id = request.spell_id,
            "CMSG_PET_CANCEL_AURA parsed"
        );
        self.hub
            .cancel_represented_pet_aura_like_cpp(request.pet_guid, request.spell_id);
    }

    /// Handle `CMSG_TOTEM_DESTROYED`.
    pub async fn handle_totem_destroyed(&mut self, mut pkt: WorldPacket) {
        let request = match TotemDestroyed::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.hub.shared().core.account_id,
                    "TotemDestroyed parse failed: {error}"
                );
                return;
            }
        };

        debug!(
            account = self.hub.shared().core.account_id,
            slot = request.slot,
            totem_guid = ?request.totem_guid,
            "CMSG_TOTEM_DESTROYED parsed"
        );
        self.hub
            .destroy_represented_totem_like_cpp(request.slot, request.totem_guid);
    }
}

/// Builds a spell handler context from a host's spell, inventory, loot and
/// quest state.
pub trait SpellHandlerHostLikeCpp<C> {
    fn spell_handler_cx_like_cpp<'a>(&'a mut self, catalogs: &'a C) -> SpellHandlerCxLikeCpp<'a>;
}

fn handle_cancel_cast_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: SpellHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .spell_handler_cx_like_cpp(catalogs)
            .handle_cancel_cast(pkt)
            .await;
    })
}

fn handle_cancel_aura_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: SpellHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .spell_handler_cx_like_cpp(catalogs)
            .handle_cancel_aura(pkt)
            .await;
    })
}

fn handle_cancel_auto_repeat_spell_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: SpellHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .spell_handler_cx_like_cpp(catalogs)
            .handle_cancel_auto_repeat_spell(pkt)
            .await;
    })
}

fn handle_cancel_channelling_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: SpellHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .spell_handler_cx_like_cpp(catalogs)
            .handle_cancel_channelling(pkt)
            .await;
    })
}

fn handle_cancel_growth_aura_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: SpellHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .spell_handler_cx_like_cpp(catalogs)
            .handle_cancel_growth_aura(pkt)
            .await;
    })
}

fn handle_cancel_mount_aura_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: SpellHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .spell_handler_cx_like_cpp(catalogs)
            .handle_cancel_mount_aura(pkt)
            .await;
    })
}

fn handle_cancel_queued_spell_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: SpellHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .spell_handler_cx_like_cpp(catalogs)
            .handle_cancel_queued_spell(pkt)
            .await;
    })
}

fn handle_pet_cancel_aura_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: SpellHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .spell_handler_cx_like_cpp(catalogs)
            .handle_pet_cancel_aura(pkt)
            .await;
    })
}

fn handle_totem_destroyed_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: SpellHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .spell_handler_cx_like_cpp(catalogs)
            .handle_totem_destroyed(pkt)
            .await;
    })
}

/// Registers the spell cancel handlers on the packet registry.
pub fn register_spell_handlers_like_cpp<S, C>(
    builder: &mut RegistryBuilder<S, C>,
) -> Result<(), DuplicateHandlerRegistrationLikeCpp>
where
    S: SpellHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::CancelCast,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadSafe,
        handler_name: "handle_cancel_cast",
        handler: handle_cancel_cast_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::CancelAura,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_cancel_aura",
        handler: handle_cancel_aura_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::CancelAutoRepeatSpell,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_cancel_auto_repeat_spell",
        handler: handle_cancel_auto_repeat_spell_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::CancelChannelling,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_cancel_channelling",
        handler: handle_cancel_channelling_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::CancelGrowthAura,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_cancel_growth_aura",
        handler: handle_cancel_growth_aura_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::CancelMountAura,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_cancel_mount_aura",
        handler: handle_cancel_mount_aura_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::CancelQueuedSpell,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_cancel_queued_spell",
        handler: handle_cancel_queued_spell_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::PetCancelAura,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_pet_cancel_aura",
        handler: handle_pet_cancel_aura_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::TotemDestroyed,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_totem_destroyed",
        handler: handle_totem_destroyed_thunk::<S, C>,
    })?;
    Ok(())
}
