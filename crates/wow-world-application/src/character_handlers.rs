// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Character handler family.
//!
//! C++ source of truth: `CharacterHandler.cpp` (`HandleSetPlayerDeclinedNames`,
//! `HandleAlterAppearance`, `HandleConfirmBarbersChoice`,
//! `HandleGetUndeleteCooldownStatus`, `HandleCharDeleteOpcode`,
//! `HandleCharRenameOpcode`, `HandleOpeningCinematic`). The family owns the
//! packet bodies, the barber-chair gate, the represented barber request records
//! and the opening-cinematic selection; the World session lends the hub,
//! inventory and world-entity state and keeps the shell-only capabilities the
//! host trait exposes (#1263 F5). Bodies are moved unchanged from the World
//! shell. Enumeration, creation, customize and login stay in the World shell
//! while they need its account, realm and connection orchestration.

use std::sync::Arc;

use tracing::{info, warn};
use wow_constants::ClientOpcodes;
use wow_constants::character::RESPONSE_SUCCESS_LIKE_CPP;
use wow_core::ObjectGuid;
use wow_handler::{
    DuplicateHandlerRegistrationLikeCpp, HandlerFuture, PacketHandlerEntry, PacketProcessing,
    RegistryBuilder, SessionStatus,
};
use wow_packet::packets::character::{
    AlterAppearance, BARBER_SHOP_RESULT_NOT_ON_CHAIR_LIKE_CPP, BARBER_SHOP_RESULT_SUCCESS_LIKE_CPP,
    BarberShopResult, CharDelete, CharacterRenameRequest, CharacterRenameResult,
    ConfirmBarbersChoice, DECLINED_NAMES_RESULT_ERROR_LIKE_CPP, DeleteChar, SetPlayerDeclinedNames,
    SetPlayerDeclinedNamesResult, response_codes,
};
use wow_packet::packets::misc::TriggerCinematic;
use wow_packet::{ClientPacket, WorldPacket};
use wow_persistence::CharacterAdministrationPersistencePortLikeCpp;
use wow_world_core::session::{
    HubMut, PacketPublicationAccessLikeCpp, RepresentedAlterAppearanceLikeCpp,
    RepresentedConfirmBarbersChoiceLikeCpp,
};
use wow_world_entities::{RepresentedGameObjectUseEffect, WorldEntitiesState};
use wow_world_inventory::InventoryState;

/// C++ `SharedDefines.h` `CHAR_CREATE_ERROR`, the failure result
/// `HandleCharRenameOpcode` publishes for a missing port or a refused submit.
const CHAR_CREATE_ERROR_LIKE_CPP: u8 = 25;

/// Host step that finishes one C++ `HandleCharDeleteOpcode` call.
///
/// The character-administration port and the login-DB `realmcharacters` refresh
/// live on the World session's lifecycle state, which the handler context does
/// not reach, so the thunk lends the port and runs the refresh where this step
/// says the C++ body runs it.
pub enum CharDeleteStepLikeCpp {
    /// Every packet of the C++ body is already published.
    Complete,
    /// C++ order: the host refreshes `realmcharacters`, then the owner
    /// publishes `CHAR_DELETE_SUCCESS`.
    RefreshRealmCharacters,
}

/// Host step that finishes one C++ `HandleCharRenameOpcode` call.
///
/// The rename read is submitted on the World session's rename callback rail,
/// which the handler context does not reach.
pub enum CharRenameStepLikeCpp {
    /// Every packet of the C++ body is already published.
    Complete,
    /// C++ order: the host submits the read; when it is refused, the owner
    /// publishes the `CHAR_CREATE_ERROR` result.
    SubmitCharacterRename {
        port: Arc<dyn CharacterAdministrationPersistencePortLikeCpp>,
        guid: ObjectGuid,
        new_name: String,
    },
}

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

    /// Handle CMSG_CHAR_DELETE — delete a character.
    ///
    /// C++ `HandleCharDeleteOpcode` resolves the character-administration port,
    /// refuses a character the account does not own, deletes the row, refreshes
    /// the login-DB `realmcharacters` count and answers. The port and the
    /// refresh belong to the World session's lifecycle state, so the thunk
    /// lends the port and runs the refresh where [`CharDeleteStepLikeCpp`]
    /// names it.
    pub async fn handle_char_delete(
        &mut self,
        port: Option<Arc<dyn CharacterAdministrationPersistencePortLikeCpp>>,
        pkt: CharDelete,
    ) -> CharDeleteStepLikeCpp {
        let port = match port {
            Some(port) => port,
            None => {
                self.hub.core.send_packet(&DeleteChar {
                    code: response_codes::CHAR_DELETE_FAILED,
                });
                return CharDeleteStepLikeCpp::Complete;
            }
        };

        // Verify the character belongs to this account
        if !self.hub.shared().core.is_legit_character(&pkt.guid) {
            warn!(
                "Account {} tried to delete non-owned character {:?}",
                self.hub.shared().core.account_id,
                pkt.guid
            );
            self.hub.core.send_packet(&DeleteChar {
                code: response_codes::CHAR_DELETE_FAILED,
            });
            return CharDeleteStepLikeCpp::Complete;
        }

        let account_id = self.hub.shared().core.account_id;
        match port
            .delete_owned_character_like_cpp(pkt.guid.counter() as u64, account_id)
            .await
        {
            wow_persistence::CharacterAdministrationMutationOutcomeLikeCpp::Applied => {
                info!(
                    "Character {:?} deleted for account {}",
                    pkt.guid, account_id
                );
                self.hub.remove_legit_character(&pkt.guid);

                // Update realmcharacters count in login DB
                CharDeleteStepLikeCpp::RefreshRealmCharacters
            }
            wow_persistence::CharacterAdministrationMutationOutcomeLikeCpp::Failed { reason } => {
                warn!("Failed to delete character: {reason}");
                self.hub.core.send_packet(&DeleteChar {
                    code: response_codes::CHAR_DELETE_FAILED,
                });
                CharDeleteStepLikeCpp::Complete
            }
        }
    }

    /// C++ `HandleCharDeleteOpcode` tail: the host refreshed the login-DB
    /// `realmcharacters` count, so the success result may be published.
    pub fn publish_char_delete_success_like_cpp(&mut self) {
        self.hub.core.send_packet(&DeleteChar {
            code: response_codes::CHAR_DELETE_SUCCESS,
        });
    }

    fn send_character_rename_like_cpp(
        &self,
        result: u8,
        guid: ObjectGuid,
        new_name: impl Into<String>,
    ) {
        let name = new_name.into();
        self.hub.core.send_packet(&CharacterRenameResult {
            result,
            name,
            guid: (result == RESPONSE_SUCCESS_LIKE_CPP).then_some(guid),
        });
    }

    /// Handle CMSG_CHARACTER_RENAME_REQUEST.
    ///
    /// C++ `HandleCharRenameOpcode` refuses a character the account does not
    /// own, validates the new name, resolves the character-administration port
    /// and submits the read for a commit admitted only in its ready callback.
    /// The port and the callback rail belong to the World session's lifecycle
    /// state, so the thunk lends the port and submits where
    /// [`CharRenameStepLikeCpp`] names it.
    pub async fn handle_character_rename_request(
        &mut self,
        port: Option<Arc<dyn CharacterAdministrationPersistencePortLikeCpp>>,
        pkt: CharacterRenameRequest,
    ) -> CharRenameStepLikeCpp {
        if !self.hub.shared().core.is_legit_character(&pkt.guid) {
            warn!(
                "Account {} tried to rename non-owned character {:?}",
                self.hub.shared().core.account_id,
                pkt.guid
            );
            self.hub.core.kick(
                "WorldSession::HandleCharRenameOpcode rename character from a different account",
            );
            return CharRenameStepLikeCpp::Complete;
        }

        let name_result =
            wow_entities::represented_character_rename_name_result_like_cpp(&pkt.new_name);
        if name_result != RESPONSE_SUCCESS_LIKE_CPP {
            self.send_character_rename_like_cpp(name_result, pkt.guid, pkt.new_name);
            return CharRenameStepLikeCpp::Complete;
        }

        let Some(port) = port else {
            self.send_character_rename_like_cpp(CHAR_CREATE_ERROR_LIKE_CPP, pkt.guid, pkt.new_name);
            return CharRenameStepLikeCpp::Complete;
        };

        CharRenameStepLikeCpp::SubmitCharacterRename {
            port,
            guid: pkt.guid,
            new_name: pkt.new_name,
        }
    }

    /// C++ `HandleCharRenameOpcode` tail: the session's rename callback rail
    /// refused the read, so the `CHAR_CREATE_ERROR` result is published.
    pub fn publish_character_rename_refusal_like_cpp(
        &mut self,
        guid: ObjectGuid,
        new_name: String,
    ) {
        self.send_character_rename_like_cpp(CHAR_CREATE_ERROR_LIKE_CPP, guid, new_name);
    }

    /// Handle CMSG_OPENING_CINEMATIC.
    pub async fn handle_opening_cinematic(&mut self, _pkt: WorldPacket) {
        let _ = self.opening_cinematic_like_cpp();
    }

    /// C++ `HandleOpeningCinematic`: a character that has never gained
    /// experience plays its class cinematic, or its race cinematic when the
    /// class has none.
    fn opening_cinematic_like_cpp(&mut self) -> Option<u32> {
        if self.hub.shared().resolved_player_xp_like_cpp()? != 0 {
            return None;
        }

        let class_store = self.hub.catalogs.chr.classes_store.as_ref()?;
        let class_entry = class_store.get(u32::from(self.hub.shared().player_class_like_cpp()))?;
        let cinematic_id = if class_entry.cinematic_sequence_id != 0 {
            u32::from(class_entry.cinematic_sequence_id)
        } else {
            let race_store = self.hub.catalogs.chr.races_store.as_ref()?;
            race_store
                .get(u32::from(self.hub.shared().player_race_like_cpp()))
                .map(|race_entry| race_entry.cinematic_sequence_id as u32)?
        };

        send_represented_cinematic_start_like_cpp(&mut self.hub, cinematic_id);
        Some(cinematic_id)
    }
}

/// C++ `WorldSession::SendCinematicStart` over the represented cinematic state:
/// publish `SMSG_TRIGGER_CINEMATIC` and begin the sequence when it is known.
///
/// The application owner keeps the body (#1263 F5); the World session delegates
/// its remaining gameobject-use caller to this helper.
pub fn send_represented_cinematic_start_like_cpp(hub: &mut HubMut<'_>, cinematic_id: u32) {
    if hub
        .shared()
        .player_cinematic_state_snapshot_like_cpp()
        .is_none()
    {
        return;
    }
    hub.core.send_packet(&TriggerCinematic {
        cinematic_id,
        conversation_guid: ObjectGuid::EMPTY,
    });
    if let Some(sequence) = hub
        .catalogs
        .cinematic_sequences_store
        .as_ref()
        .and_then(|store| store.get(cinematic_id))
    {
        let camera_ids = sequence.camera;
        let _ = hub.with_player_cinematic_state_like_cpp(|state| {
            state.begin_cinematic_like_cpp(cinematic_id, camera_ids);
        });
    }
}

/// Builds a character handler context from a host's state.
pub trait CharacterHandlerHostLikeCpp<C> {
    fn character_handler_cx_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a C,
    ) -> CharacterHandlerCxLikeCpp<'a>;

    /// C++ `HandleCharDeleteOpcode`/`HandleCharRenameOpcode` resolve the
    /// character-administration port from the session's lifecycle state, which
    /// the handler context cannot reach.
    fn character_administration_persistence_port_like_cpp(
        &mut self,
    ) -> Option<Arc<dyn CharacterAdministrationPersistencePortLikeCpp>>;

    /// C++ `HandleCharDeleteOpcode` refreshes the login-DB `realmcharacters`
    /// count between the delete commit and `CHAR_DELETE_SUCCESS`; the refresh
    /// needs the session's player-lifecycle port.
    fn update_realm_characters_like_cpp(&mut self) -> HandlerFuture<'_, ()>;

    /// C++ `HandleCharRenameOpcode` submits the read on the session's rename
    /// callback rail, which admits the commit only in its ready callback.
    fn submit_character_rename_like_cpp(
        &mut self,
        port: Arc<dyn CharacterAdministrationPersistencePortLikeCpp>,
        guid: ObjectGuid,
        name: String,
    ) -> bool;
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

fn handle_char_delete_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    mut pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: CharacterHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        match CharDelete::read(&mut pkt) {
            Ok(del) => {
                let port = session.character_administration_persistence_port_like_cpp();
                let step = session
                    .character_handler_cx_like_cpp(catalogs)
                    .handle_char_delete(port, del)
                    .await;
                if matches!(step, CharDeleteStepLikeCpp::RefreshRealmCharacters) {
                    session.update_realm_characters_like_cpp().await;
                    session
                        .character_handler_cx_like_cpp(catalogs)
                        .publish_char_delete_success_like_cpp();
                }
            }
            Err(e) => tracing::warn!("Failed to read CharDelete: {e}"),
        }
    })
}

fn handle_character_rename_request_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    mut pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: CharacterHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        match CharacterRenameRequest::read(&mut pkt) {
            Ok(rename) => {
                let port = session.character_administration_persistence_port_like_cpp();
                let step = session
                    .character_handler_cx_like_cpp(catalogs)
                    .handle_character_rename_request(port, rename)
                    .await;
                if let CharRenameStepLikeCpp::SubmitCharacterRename {
                    port,
                    guid,
                    new_name,
                } = step
                {
                    if !session.submit_character_rename_like_cpp(port, guid, new_name.clone()) {
                        session
                            .character_handler_cx_like_cpp(catalogs)
                            .publish_character_rename_refusal_like_cpp(guid, new_name);
                    }
                }
            }
            Err(e) => tracing::warn!("Failed to read CharacterRenameRequest: {e}"),
        }
    })
}

fn handle_opening_cinematic_thunk<'a, S, C>(
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
            .handle_opening_cinematic(pkt)
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
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::CharDelete,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_char_delete",
        handler: handle_char_delete_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::CharacterRenameRequest,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_character_rename_request",
        handler: handle_character_rename_request_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::OpeningCinematic,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_opening_cinematic",
        handler: handle_opening_cinematic_thunk::<S, C>,
    })?;
    Ok(())
}
