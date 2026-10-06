// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Character handler family.
//!
//! C++ source of truth: `CharacterHandler.cpp` (`HandleSetPlayerDeclinedNames`,
//! `HandleAlterAppearance`, `HandleConfirmBarbersChoice`,
//! `HandleGetUndeleteCooldownStatus`). The family owns the packet bodies, the
//! barber-chair gate and the represented barber request records; the World
//! session only lends the hub, inventory and world-entity state (#1263 F5).
//! Bodies are moved unchanged from the World shell. Enumeration, creation,
//! deletion, rename, customize and login stay in the World shell while they
//! need its account, realm and connection orchestration.

use tracing::warn;
use wow_constants::ClientOpcodes;
use wow_handler::{
    DuplicateHandlerRegistrationLikeCpp, HandlerFuture, PacketHandlerEntry, PacketProcessing,
    RegistryBuilder, SessionStatus,
};
use wow_packet::packets::character::{
    AlterAppearance, BARBER_SHOP_RESULT_NOT_ON_CHAIR_LIKE_CPP, BARBER_SHOP_RESULT_SUCCESS_LIKE_CPP,
    BarberShopResult, ConfirmBarbersChoice, DECLINED_NAMES_RESULT_ERROR_LIKE_CPP,
    SetPlayerDeclinedNames, SetPlayerDeclinedNamesResult,
};
use wow_packet::{ClientPacket, WorldPacket};
use wow_world_core::session::{
    HubMut, PacketPublicationAccessLikeCpp, RepresentedAlterAppearanceLikeCpp,
    RepresentedConfirmBarbersChoiceLikeCpp,
};
use wow_world_entities::{RepresentedGameObjectUseEffect, WorldEntitiesState};
use wow_world_inventory::InventoryState;

/// Borrowed inputs of one character handler invocation.
pub struct CharacterHandlerCxLikeCpp<'a> {
    hub: HubMut<'a>,
    inventory: &'a mut InventoryState,
    world_entities: &'a WorldEntitiesState,
}

impl<'a> CharacterHandlerCxLikeCpp<'a> {
    pub fn new(
        hub: HubMut<'a>,
        inventory: &'a mut InventoryState,
        world_entities: &'a WorldEntitiesState,
    ) -> Self {
        Self {
            hub,
            inventory,
            world_entities,
        }
    }

    fn publication_like_cpp(&self) -> PacketPublicationAccessLikeCpp<'_> {
        self.hub.shared().core.packet_publication_access_like_cpp()
    }

    /// C++ `Player::IsSittingOnBarberChair` over the represented chair-use
    /// effects: the latest chair use by this Player at its current stand state.
    fn represented_is_on_barber_chair_like_cpp(&self) -> bool {
        let Some(player_guid) = self.hub.shared().core.player_guid() else {
            return false;
        };
        // `UnitStandStateType` is a fieldless `#[repr(u8)]` enum, so the
        // discriminant cast equals the former `ToPrimitive::to_u32` result.
        let Some(current_stand_state) = self
            .hub
            .shared()
            .resolved_player_stand_state_like_cpp()
            .map(|state| state as u32)
        else {
            return false;
        };

        self.world_entities
            .represented_gameobject_use_effects_since_like_cpp(0)
            .iter()
            .rev()
            .any(|effect| {
                matches!(
                    effect,
                    RepresentedGameObjectUseEffect::BarberChairUsed {
                        player_guid: effect_player_guid,
                        stand_state,
                        ..
                    } if *effect_player_guid == player_guid && *stand_state == current_stand_state
                )
            })
    }

    /// Handle CMSG_GET_UNDELETE_CHARACTER_COOLDOWN_STATUS.
    ///
    /// The client sends this when it wants to know if character undelete is
    /// available. We always respond with "no cooldown" (undelete available).
    pub async fn handle_get_undelete_cooldown_status(&mut self) {
        self.publication_like_cpp()
            .send_packet(&wow_packet::packets::misc::UndeleteCooldownStatusResponse::no_cooldown());
    }

    /// Handle CMSG_ALTER_APPEARANCE.
    ///
    /// C++ `HandleAlterAppearance` validates customization DB2 requirements,
    /// requires the player to be sitting on a nearby barber chair, checks
    /// `GetBarberShopCost`, sends `SMSG_BARBER_SHOP_RESULT`, then mutates
    /// player gender/customizations and criteria.
    ///
    /// Rust currently represents barber-chair use and stand-state, but does
    /// not yet own the full ChrCustomization/BarberShop cost/runtime mutation.
    /// This seam preserves packet/dispatch, the C++ not-on-chair result, and
    /// records accepted requests without fabricating the full appearance change.
    pub async fn handle_alter_appearance(&mut self, mut pkt: WorldPacket) {
        let request = match AlterAppearance::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!("Bad AlterAppearance: {error}");
                return;
            }
        };

        if !self.represented_is_on_barber_chair_like_cpp() {
            self.publication_like_cpp().send_packet(&BarberShopResult {
                result: BARBER_SHOP_RESULT_NOT_ON_CHAIR_LIKE_CPP,
            });
            return;
        }

        let cost = 0;
        self.publication_like_cpp().send_packet(&BarberShopResult {
            result: BARBER_SHOP_RESULT_SUCCESS_LIKE_CPP,
        });
        self.inventory.record_represented_alter_appearance_like_cpp(
            &mut self.hub,
            RepresentedAlterAppearanceLikeCpp {
                new_sex: request.new_sex,
                customizations: request.customizations,
                customized_race: request.customized_race,
                customized_chr_model_id: request.customized_chr_model_id,
                cost,
            },
        );
    }

    /// Handle CMSG_CONFIRM_BARBERS_CHOICE.
    ///
    /// C++ `HandleConfirmBarbersChoice` converts the barber rows into
    /// `ChrCustomizationChoice`, checks `GetBarberShopCost`, sends only the
    /// no-money failure, and otherwise mutates money/customizations/criteria
    /// without a success packet. Rust records the accepted request until the
    /// Player customization/cost/criteria runtime is canonical.
    pub async fn handle_confirm_barbers_choice(&mut self, mut pkt: WorldPacket) {
        let request = match ConfirmBarbersChoice::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!("Bad ConfirmBarbersChoice: {error}");
                return;
            }
        };

        let cost = 0;
        self.hub.record_represented_confirm_barbers_choice_like_cpp(
            RepresentedConfirmBarbersChoiceLikeCpp {
                customizations: request.customizations,
                cost,
            },
        );
    }

    /// Handle CMSG_SET_PLAYER_DECLINED_NAMES.
    ///
    /// C++ resolves the target character through `sCharacterCache`, requires a
    /// Cyrillic base name, normalizes all five declined forms, validates them
    /// with `ObjectMgr::CheckDeclinedNames`, then replaces the
    /// `character_declinedname` row and returns success. Rust does not yet
    /// carry that character-cache / locale-validation runtime through this
    /// session path, so this bounded seam preserves the parse/dispatch and the
    /// C++ error-result branch instead of fabricating persisted declined names.
    pub async fn handle_set_player_declined_names(&mut self, mut pkt: WorldPacket) {
        let request = match SetPlayerDeclinedNames::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!("Bad SetPlayerDeclinedNames: {error}");
                return;
            }
        };

        self.publication_like_cpp()
            .send_packet(&SetPlayerDeclinedNamesResult {
                player: request.player,
                result_code: DECLINED_NAMES_RESULT_ERROR_LIKE_CPP,
            });
    }
}

/// Builds a character handler context from a host's state.
pub trait CharacterHandlerHostLikeCpp<C> {
    fn character_handler_cx_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a C,
    ) -> CharacterHandlerCxLikeCpp<'a>;
}

fn handle_get_undelete_cooldown_status_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    _pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: CharacterHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .character_handler_cx_like_cpp(catalogs)
            .handle_get_undelete_cooldown_status()
            .await;
    })
}

fn handle_alter_appearance_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: CharacterHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .character_handler_cx_like_cpp(catalogs)
            .handle_alter_appearance(pkt)
            .await;
    })
}

fn handle_confirm_barbers_choice_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: CharacterHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .character_handler_cx_like_cpp(catalogs)
            .handle_confirm_barbers_choice(pkt)
            .await;
    })
}

fn handle_set_player_declined_names_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: CharacterHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .character_handler_cx_like_cpp(catalogs)
            .handle_set_player_declined_names(pkt)
            .await;
    })
}

/// Registers the character handlers on the packet registry.
pub fn register_character_handlers_like_cpp<S, C>(
    builder: &mut RegistryBuilder<S, C>,
) -> Result<(), DuplicateHandlerRegistrationLikeCpp>
where
    S: CharacterHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::GetUndeleteCharacterCooldownStatus,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_get_undelete_cooldown_status",
        handler: handle_get_undelete_cooldown_status_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::AlterAppearance,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_alter_appearance",
        handler: handle_alter_appearance_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::ConfirmBarbersChoice,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_confirm_barbers_choice",
        handler: handle_confirm_barbers_choice_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::SetPlayerDeclinedNames,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_set_player_declined_names",
        handler: handle_set_player_declined_names_thunk::<S, C>,
    })?;
    Ok(())
}
