// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Private account_data capability handlers extracted from the legacy misc owner.

use tracing::{debug, warn};
use wow_constants::ClientOpcodes;
use wow_core::ObjectGuid;
use wow_handler::{PacketProcessing, SessionStatus};

use crate::session::registry::PacketHandlerEntry;
use wow_packet::ClientPacket;
use wow_packet::packets::misc::{
    AddonList, MAX_ACCOUNT_DATA_SIZE_LIKE_CPP, NUM_ACCOUNT_DATA_TYPES, RequestAccountData,
    SaveCufProfiles, TutorialSetFlag, UpdateAccountData, UserClientUpdateAccountData,
    compress_account_data_like_cpp, decompress_account_data_like_cpp,
};

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::AddonList,
        status: SessionStatus::Authed,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_addon_list",
        handler: |session, pkt| Box::pin(async move { session.handle_addon_list(pkt).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::RequestAccountData,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_request_account_data",
        handler: |session, pkt| {
            Box::pin(async move { session.handle_request_account_data(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::UpdateAccountData,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_update_account_data",
        handler: |session, pkt| {
            Box::pin(async move { session.handle_update_account_data(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SaveCufProfiles,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_save_cuf_profiles",
        handler: |session, pkt| {
            Box::pin(async move { session.handle_save_cuf_profiles(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::Tutorial,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_tutorial",
        handler: |session, pkt| Box::pin(async move { session.handle_tutorial(pkt).await }),
    }
}

impl crate::session::WorldSession {
    pub async fn handle_request_account_data(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match RequestAccountData::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "RequestAccountData parse failed: {error}"
                );
                return;
            }
        };

        if usize::from(packet.data_type) >= NUM_ACCOUNT_DATA_TYPES {
            return;
        }

        let Some(account_data) = self.account_data_like_cpp(packet.data_type) else {
            return;
        };
        let data = account_data.data.clone();
        let time = account_data.time;
        let compressed_data = match compress_account_data_like_cpp(&data) {
            Ok(compressed_data) => compressed_data,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "RequestAccountData compression failed: {error}"
                );
                return;
            }
        };

        self.send_packet_realm(&UpdateAccountData {
            player_guid: self.player_guid().unwrap_or(ObjectGuid::EMPTY),
            time,
            size: data.len() as u32,
            data_type: packet.data_type,
            compressed_data,
        });
    }

    pub async fn handle_update_account_data(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match UserClientUpdateAccountData::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "UpdateAccountData parse failed: {error}"
                );
                return;
            }
        };

        if usize::from(packet.data_type) >= NUM_ACCOUNT_DATA_TYPES {
            return;
        }

        if packet.size == 0 {
            self.set_account_data_persisted_like_cpp(packet.data_type, 0, String::new())
                .await;
            return;
        }

        if packet.size > MAX_ACCOUNT_DATA_SIZE_LIKE_CPP {
            warn!(
                account = self.account_id,
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
                    account = self.account_id,
                    data_type = packet.data_type,
                    "UpdateAccountData decompression failed: {error}"
                );
                return;
            }
        };

        self.set_account_data_persisted_like_cpp(packet.data_type, packet.time, data)
            .await;
    }

    pub async fn handle_addon_list(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match AddonList::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(account = self.account_id, "AddonList parse failed: {error}");
                return;
            }
        };

        debug!(
            account = self.account_id,
            addon_count = packet.addons.len(),
            "HandleAddonList consumed addon list like C++"
        );
    }

    pub async fn handle_save_cuf_profiles(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match SaveCufProfiles::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "SaveCufProfiles parse failed: {error}"
                );
                return;
            }
        };

        if !self.represented_save_cuf_profiles_like_cpp(packet.profiles) {
            warn!(
                account = self.account_id,
                max_profiles = wow_packet::packets::misc::MAX_CUF_PROFILES_LIKE_CPP,
                "SaveCufProfiles ignored profile count above C++ MAX_CUF_PROFILES"
            );
        }
    }

    pub async fn handle_tutorial(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match TutorialSetFlag::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(account = self.account_id, "Tutorial parse failed: {error}");
                return;
            }
        };

        if !self.apply_tutorial_action_like_cpp(packet.action, packet.tutorial_bit) {
            warn!(
                account = self.account_id,
                action = packet.action,
                tutorial_bit = packet.tutorial_bit,
                "CMSG_TUTORIAL ignored invalid action or TutorialBit like C++"
            );
        }
    }
}
