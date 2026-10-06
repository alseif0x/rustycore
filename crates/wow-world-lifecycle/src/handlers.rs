// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Account-data, addon-list, CUF-profile and tutorial packet handlers.
//!
//! C++ source of truth: `WorldSession::HandleRequestAccountData`,
//! `HandleUpdateAccountData`, `HandleAddonList`, `HandleSaveCUFProfiles` and
//! `HandleTutorial` (`src/server/game/Handlers/MiscHandler.cpp`). The handlers
//! own the session lifecycle state and the represented durability operation;
//! the World session only splits its state into the borrowed context (#1263 F5).

use tracing::{debug, info, warn};
use wow_constants::ClientOpcodes;
use wow_core::ObjectGuid;
use wow_handler::{
    DuplicateHandlerRegistrationLikeCpp, HandlerFuture, PacketHandlerEntry, PacketProcessing,
    RegistryBuilder, SessionStatus,
};
use wow_packet::ClientPacket;
use wow_packet::WorldPacket;
use wow_packet::packets::misc::{
    AddonList, MAX_ACCOUNT_DATA_SIZE_LIKE_CPP, NUM_ACCOUNT_DATA_TYPES, RequestAccountData,
    SaveCufProfiles, TUTORIAL_ACTION_CLEAR_LIKE_CPP, TUTORIAL_ACTION_RESET_LIKE_CPP,
    TUTORIAL_ACTION_UPDATE_LIKE_CPP, TutorialSetFlag, UpdateAccountData,
    UserClientUpdateAccountData, compress_account_data_like_cpp, decompress_account_data_like_cpp,
};
use wow_world_core::session::HubMut;

use crate::SessionLifecycleState;

/// Borrowed inputs of one account-data handler invocation.
pub struct AccountDataHandlerCxLikeCpp<'a> {
    lifecycle: &'a mut SessionLifecycleState,
    hub: HubMut<'a>,
}

impl<'a> AccountDataHandlerCxLikeCpp<'a> {
    pub fn new(lifecycle: &'a mut SessionLifecycleState, hub: HubMut<'a>) -> Self {
        Self { lifecycle, hub }
    }

    fn player_guid_like_cpp(&self) -> ObjectGuid {
        self.hub.core.player_guid().unwrap_or(ObjectGuid::EMPTY)
    }

    pub async fn handle_request_account_data(&mut self, mut pkt: WorldPacket) {
        let packet = match RequestAccountData::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.hub.core.account_id,
                    "RequestAccountData parse failed: {error}"
                );
                return;
            }
        };

        if usize::from(packet.data_type) >= NUM_ACCOUNT_DATA_TYPES {
            return;
        }

        let Some(account_data) = self.lifecycle.account_data_like_cpp(packet.data_type) else {
            return;
        };
        let data = account_data.data.clone();
        let time = account_data.time;
        let compressed_data = match compress_account_data_like_cpp(&data) {
            Ok(compressed_data) => compressed_data,
            Err(error) => {
                warn!(
                    account = self.hub.core.account_id,
                    "RequestAccountData compression failed: {error}"
                );
                return;
            }
        };

        self.hub
            .core
            .packet_publication_access_like_cpp()
            .send_packet_realm(&UpdateAccountData {
                player_guid: self.player_guid_like_cpp(),
                time,
                size: data.len() as u32,
                data_type: packet.data_type,
                compressed_data,
            });
    }

    pub async fn handle_update_account_data(&mut self, mut pkt: WorldPacket) {
        let packet = match UserClientUpdateAccountData::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.hub.core.account_id,
                    "UpdateAccountData parse failed: {error}"
                );
                return;
            }
        };

        if usize::from(packet.data_type) >= NUM_ACCOUNT_DATA_TYPES {
            return;
        }

        if packet.size == 0 {
            self.lifecycle
                .set_account_data_persisted_like_cpp(
                    &mut self.hub,
                    packet.data_type,
                    0,
                    String::new(),
                )
                .await;
            return;
        }

        if packet.size > MAX_ACCOUNT_DATA_SIZE_LIKE_CPP {
            warn!(
                account = self.hub.core.account_id,
                data_type = packet.data_type,
                size = packet.size,
                "UpdateAccountData rejected oversized payload like C++"
            );
            return;
        }

        let data = match decompress_account_data_like_cpp(&packet.compressed_data, packet.size) {
            Ok(data) => data,
            Err(error) => {
                warn!(
                    account = self.hub.core.account_id,
                    data_type = packet.data_type,
                    "UpdateAccountData decompression failed: {error}"
                );
                return;
            }
        };

        self.lifecycle
            .set_account_data_persisted_like_cpp(&mut self.hub, packet.data_type, packet.time, data)
            .await;
    }

    pub fn handle_addon_list(&self, mut pkt: WorldPacket) {
        let packet = match AddonList::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.hub.core.account_id,
                    "AddonList parse failed: {error}"
                );
                return;
            }
        };

        debug!(
            account = self.hub.core.account_id,
            addon_count = packet.addons.len(),
            "HandleAddonList consumed addon list like C++"
        );
    }

    pub fn handle_save_cuf_profiles(&mut self, mut pkt: WorldPacket) {
        let packet = match SaveCufProfiles::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.hub.core.account_id,
                    "SaveCufProfiles parse failed: {error}"
                );
                return;
            }
        };

        if !self
            .lifecycle
            .represented_save_cuf_profiles_like_cpp(&mut self.hub, packet.profiles)
        {
            warn!(
                account = self.hub.core.account_id,
                max_profiles = wow_packet::packets::misc::MAX_CUF_PROFILES_LIKE_CPP,
                "SaveCufProfiles ignored profile count above C++ MAX_CUF_PROFILES"
            );
        }
    }

    pub fn handle_tutorial(&mut self, mut pkt: WorldPacket) {
        let packet = match TutorialSetFlag::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.hub.core.account_id,
                    "Tutorial parse failed: {error}"
                );
                return;
            }
        };

        if !self.apply_tutorial_action_like_cpp(packet.action, packet.tutorial_bit) {
            warn!(
                account = self.hub.core.account_id,
                action = packet.action,
                tutorial_bit = packet.tutorial_bit,
                "CMSG_TUTORIAL ignored invalid action or TutorialBit like C++"
            );
        }
    }

    fn apply_tutorial_action_like_cpp(&mut self, action: u8, tutorial_bit: Option<u32>) -> bool {
        match action {
            TUTORIAL_ACTION_UPDATE_LIKE_CPP => {
                let Some(tutorial_bit) = tutorial_bit else {
                    return false;
                };
                let index = (tutorial_bit >> 5) as usize;
                if index >= self.lifecycle.tutorial_values_like_cpp().len() {
                    return false;
                }
                let flag = self.lifecycle.tutorial_values_like_cpp()[index]
                    | (1u32 << (tutorial_bit & 0x1F));
                self.lifecycle.set_tutorial_int_like_cpp(index, flag)
            }
            TUTORIAL_ACTION_CLEAR_LIKE_CPP => {
                for index in 0..self.lifecycle.tutorial_values_like_cpp().len() {
                    self.lifecycle.set_tutorial_int_like_cpp(index, u32::MAX);
                }
                true
            }
            TUTORIAL_ACTION_RESET_LIKE_CPP => {
                for index in 0..self.lifecycle.tutorial_values_like_cpp().len() {
                    self.lifecycle.set_tutorial_int_like_cpp(index, 0);
                }
                true
            }
            _ => false,
        }
    }
    /// C++ `WorldSession::HandleRequestPlayedTime`.
    pub async fn handle_request_played_time(&mut self, trigger_event: bool) {
        use wow_packet::packets::misc::PlayedTime;

        // Session time elapsed since login (seconds).
        let session_secs: u32 = self
            .lifecycle
            .login_time_like_cpp()
            .map(|t| t.elapsed().as_secs() as u32)
            .unwrap_or(0);

        // Add session time on top of DB-loaded base values.
        let total_time = self
            .lifecycle
            .total_played_time_like_cpp()
            .saturating_add(session_secs);
        let level_time = self
            .lifecycle
            .level_played_time_like_cpp()
            .saturating_add(session_secs);

        self.hub
            .core
            .packet_publication_access_like_cpp()
            .send_packet(&PlayedTime {
                total_time,
                level_time,
                trigger_event,
            });
    }

    /// C++ `WorldSession::HandleLogoutCancel`.
    pub async fn handle_logout_cancel(&mut self) {
        info!("LogoutCancel from account {}", self.hub.core.account_id);
        self.lifecycle.clear_logout_time();
        self.hub
            .core
            .packet_publication_access_like_cpp()
            .send_packet(&wow_packet::packets::misc::LogoutCancelAck);
    }
}

/// Builds the account-data handler context from a host's session state.
pub trait AccountDataHandlerHostLikeCpp<C> {
    fn account_data_handler_cx_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a C,
    ) -> AccountDataHandlerCxLikeCpp<'a>;
}

fn handle_addon_list_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: AccountDataHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .account_data_handler_cx_like_cpp(catalogs)
            .handle_addon_list(pkt);
    })
}

fn handle_request_account_data_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: AccountDataHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .account_data_handler_cx_like_cpp(catalogs)
            .handle_request_account_data(pkt)
            .await;
    })
}

fn handle_update_account_data_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: AccountDataHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .account_data_handler_cx_like_cpp(catalogs)
            .handle_update_account_data(pkt)
            .await;
    })
}

fn handle_save_cuf_profiles_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: AccountDataHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .account_data_handler_cx_like_cpp(catalogs)
            .handle_save_cuf_profiles(pkt);
    })
}

fn handle_tutorial_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: AccountDataHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .account_data_handler_cx_like_cpp(catalogs)
            .handle_tutorial(pkt);
    })
}

fn handle_request_played_time_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    mut pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: AccountDataHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        let trigger = pkt.read_uint8().unwrap_or(0) != 0;
        session
            .account_data_handler_cx_like_cpp(catalogs)
            .handle_request_played_time(trigger)
            .await;
    })
}

fn handle_logout_cancel_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    _pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: AccountDataHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .account_data_handler_cx_like_cpp(catalogs)
            .handle_logout_cancel()
            .await;
    })
}

/// Register the account-data packet entries through their lifecycle owner.
pub fn register_account_data_handlers_like_cpp<S, C>(
    builder: &mut RegistryBuilder<S, C>,
) -> Result<(), DuplicateHandlerRegistrationLikeCpp>
where
    S: AccountDataHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::AddonList,
        status: SessionStatus::Authed,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_addon_list",
        handler: handle_addon_list_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::RequestAccountData,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_request_account_data",
        handler: handle_request_account_data_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::UpdateAccountData,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_update_account_data",
        handler: handle_update_account_data_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::SaveCufProfiles,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_save_cuf_profiles",
        handler: handle_save_cuf_profiles_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::Tutorial,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_tutorial",
        handler: handle_tutorial_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::RequestPlayedTime,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_request_played_time",
        handler: handle_request_played_time_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::LogoutCancel,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_logout_cancel",
        handler: handle_logout_cancel_thunk::<S, C>,
    })?;
    Ok(())
}
