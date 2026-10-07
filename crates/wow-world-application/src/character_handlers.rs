// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Character handler family.
//!
//! C++ source of truth: `CharacterHandler.cpp` (`HandleSetPlayerDeclinedNames`,
//! `HandleAlterAppearance`, `HandleConfirmBarbersChoice`,
//! `HandleGetUndeleteCooldownStatus`, `HandleCharDeleteOpcode`,
//! `HandleCharRenameOpcode`, `HandleOpeningCinematic`, `HandleCharEnumOpcode`,
//! `HandleCharCreateOpcode`, `HandleCharCustomizeOpcode`). The family owns the
//! packet bodies, the barber-chair gate, the represented barber request records,
//! the opening-cinematic selection, the character-list projection and the
//! create/customize persistence flow; the World session lends the hub, inventory
//! and world-entity state and keeps the shell-only capabilities the host trait
//! exposes (#1263 F5). Bodies are moved unchanged from the World shell. Login
//! stays in the World shell while it needs its realm and connection
//! orchestration.
//!
//! The packet bodies live in the private sibling module
//! `character_handler_families` (`undelete`, `barber`, `declined_names`,
//! `char_delete`, `rename`, `cinematic`, `enumeration`, `creation`,
//! `customize`); `barber` and `creation` also own their private helpers. The
//! bodies are moved unchanged, so this owner module keeps the facades, the host
//! trait, the thunks and the registrar and declares no descendant module.

use std::sync::Arc;

use wow_constants::ClientOpcodes;
use wow_constants::rest::{REST_STATE_NORMAL_LIKE_CPP, REST_STATE_RAF_LINKED_LIKE_CPP};
use wow_core::{ObjectGuid, ObjectGuidGenerator};
use wow_handler::{
    DuplicateHandlerRegistrationLikeCpp, HandlerFuture, PacketHandlerEntry, PacketProcessing,
    RegistryBuilder, SessionStatus,
};
use wow_packet::packets::character::{
    CharCustomize, CharDelete, CharacterRenameRequest, CreateCharacter, VisualItemInfo,
};
use wow_packet::packets::misc::TriggerCinematic;
use wow_packet::{ClientPacket, WorldPacket};
use wow_persistence::{
    CharacterAdministrationPersistencePortLikeCpp, CharacterEnumerationPersistencePortLikeCpp,
};
use wow_world_core::session::{
    HubMut, PacketPublicationAccessLikeCpp, SupportFeaturePolicyLikeCpp,
};
use wow_world_entities::WorldEntitiesState;
use wow_world_inventory::InventoryState;

/// C++ `SharedDefines.h` `CHAR_CREATE_ERROR`, the failure result
/// `HandleCharRenameOpcode` publishes for a missing port or a refused submit.
pub(crate) const CHAR_CREATE_ERROR_LIKE_CPP: u8 = 25;

/// C++ `SharedDefines.h` `CHAR_CREATE_NAME_IN_USE`, the failure result
/// `HandleCharCustomizeOpcode` publishes for a name another character holds.
pub(crate) const CHAR_CREATE_NAME_IN_USE_LIKE_CPP: u8 = 27;

/// Maximum characters per account.
pub(crate) const MAX_CHARACTERS_PER_ACCOUNT: u32 = 10;

/// C++ `Player::Create` seeds the represented RAF rest state from the account's
/// recruiter role.
pub fn initial_character_rest_state_like_cpp(is_a_recruiter: bool, recruiter_id: u32) -> u8 {
    if is_a_recruiter || recruiter_id != 0 {
        REST_STATE_RAF_LINKED_LIKE_CPP
    } else {
        REST_STATE_NORMAL_LIKE_CPP
    }
}

/// Parse a space-separated equipment cache string into VisualItemInfo array.
///
/// C++ `EnumCharactersResult::CharacterInfo` parses `equipmentCache` as five
/// fields per slot: InvType, DisplayID, DisplayEnchantID, Subclass, and
/// SecondaryItemModifiedAppearanceID.
pub fn parse_equipment_cache(cache: &str) -> [VisualItemInfo; 34] {
    let mut equipment = [VisualItemInfo::default(); 34];
    if cache.is_empty() {
        return equipment;
    }

    let parts: Vec<&str> = cache.split_whitespace().collect();
    let fields_per_slot = 5;

    for slot in 0..34 {
        let base = slot * fields_per_slot;
        if base + fields_per_slot > parts.len() {
            break;
        }
        equipment[slot] = VisualItemInfo {
            inv_type: parts[base].parse().unwrap_or(0),
            display_id: parts[base + 1].parse().unwrap_or(0),
            display_enchant_id: parts[base + 2].parse().unwrap_or(0),
            subclass: parts[base + 3].parse().unwrap_or(0),
            secondary_item_modified_appearance_id: parts[base + 4].parse().unwrap_or(0),
        };
    }

    equipment
}

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

/// Host step that finishes one C++ `HandleCharCreateOpcode` call.
///
/// The login-DB `realmcharacters` refresh runs on the World session's
/// lifecycle state, which the handler context does not reach, so the thunk
/// runs it where this step says the C++ body runs it.
pub enum CreateCharacterStepLikeCpp {
    /// Every packet of the C++ body is already published.
    Complete,
    /// C++ order: the host refreshes `realmcharacters`, then the owner
    /// publishes `CHAR_CREATE_SUCCESS` for the created character.
    RefreshRealmCharacters { guid: ObjectGuid },
}

/// Borrowed inputs of one character handler invocation.
pub struct CharacterHandlerCxLikeCpp<'a> {
    pub(crate) hub: HubMut<'a>,
    pub(crate) inventory: &'a mut InventoryState,
    pub(crate) world_entities: &'a WorldEntitiesState,
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

    pub(crate) fn publication_like_cpp(&self) -> PacketPublicationAccessLikeCpp<'_> {
        self.hub.shared().core.packet_publication_access_like_cpp()
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

    /// C++ `HandleCharEnumOpcode` resolves the character-enumeration port from
    /// the session's lifecycle state, which the handler context cannot reach.
    fn character_enumeration_persistence_port_like_cpp(
        &mut self,
    ) -> Option<Arc<dyn CharacterEnumerationPersistencePortLikeCpp>>;

    /// C++ `HandleCharEnumOpcode` reads `sWorld->getBoolConfig(
    /// CONFIG_DECLINED_NAMES_USED)`; the support-feature policy lives on the
    /// shell catalogs, which the generic thunk cannot name.
    fn support_feature_policy_like_cpp(catalogs: &C) -> &SupportFeaturePolicyLikeCpp;

    /// C++ `sObjectMgr->GenerateCharacterGuid()`: the player GUID generator
    /// lives on the shell catalogs, which the generic thunk cannot name.
    fn character_guid_generator_like_cpp(catalogs: &C) -> &ObjectGuidGenerator;
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

fn handle_enum_characters_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    _pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: CharacterHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        let port = session.character_enumeration_persistence_port_like_cpp();
        session
            .character_handler_cx_like_cpp(catalogs)
            .handle_enum_characters_with_policy_like_cpp(
                port,
                S::support_feature_policy_like_cpp(catalogs),
            )
            .await;
    })
}

fn handle_create_character_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    mut pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: CharacterHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        match CreateCharacter::read(&mut pkt) {
            Ok(create) => {
                let port = session.character_administration_persistence_port_like_cpp();
                let generator = S::character_guid_generator_like_cpp(catalogs);
                let step = session
                    .character_handler_cx_like_cpp(catalogs)
                    .handle_create_character_with_generator_like_cpp(port, generator, create)
                    .await;
                if let CreateCharacterStepLikeCpp::RefreshRealmCharacters { guid } = step {
                    session.update_realm_characters_like_cpp().await;
                    session
                        .character_handler_cx_like_cpp(catalogs)
                        .publish_create_character_success_like_cpp(guid);
                }
            }
            Err(e) => tracing::warn!("Failed to read CreateCharacter: {e}"),
        }
    })
}

fn handle_char_customize_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    mut pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: CharacterHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        match CharCustomize::read(&mut pkt) {
            Ok(customize) => {
                let port = session.character_administration_persistence_port_like_cpp();
                session
                    .character_handler_cx_like_cpp(catalogs)
                    .handle_char_customize(port, customize)
                    .await;
            }
            Err(e) => tracing::warn!("Failed to read CharCustomize: {e}"),
        }
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
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::EnumCharacters,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_enum_characters",
        handler: handle_enum_characters_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::CreateCharacter,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_create_character",
        handler: handle_create_character_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::CharCustomize,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_char_customize",
        handler: handle_char_customize_thunk::<S, C>,
    })?;
    Ok(())
}
