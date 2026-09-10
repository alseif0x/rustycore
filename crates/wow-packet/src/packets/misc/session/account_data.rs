//! Account data packets.
//!
//! Separated from session.rs under #689.

use super::*;

// ── AccountDataTimes (SMSG 0x270a) ──────────────────────────────────

/// C++ `WorldPackets::ClientConfig::RequestAccountData`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RequestAccountData {
    pub player_guid: ObjectGuid,
    pub data_type: u8,
}

impl ClientPacket for RequestAccountData {
    const OPCODE: ClientOpcodes = ClientOpcodes::RequestAccountData;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            player_guid: pkt.read_packed_guid()?,
            data_type: pkt.read_bits(4)? as u8,
        })
    }
}

/// C++ `WorldPackets::ClientConfig::UserClientUpdateAccountData`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserClientUpdateAccountData {
    pub player_guid: ObjectGuid,
    pub time: i64,
    pub size: u32,
    pub data_type: u8,
    pub compressed_data: Vec<u8>,
}

impl ClientPacket for UserClientUpdateAccountData {
    const OPCODE: ClientOpcodes = ClientOpcodes::UpdateAccountData;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let player_guid = pkt.read_packed_guid()?;
        let time = pkt.read_int64()?;
        let size = pkt.read_uint32()?;
        let data_type = pkt.read_bits(4)? as u8;
        let compressed_size = pkt.read_uint32()? as usize;
        let compressed_data = pkt.read_bytes(compressed_size)?;

        Ok(Self {
            player_guid,
            time,
            size,
            data_type,
            compressed_data,
        })
    }
}

/// C++ `WorldPackets::ClientConfig::UpdateAccountData`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateAccountData {
    pub player_guid: ObjectGuid,
    pub time: i64,
    pub size: u32,
    pub data_type: u8,
    pub compressed_data: Vec<u8>,
}

impl ServerPacket for UpdateAccountData {
    const OPCODE: ServerOpcodes = ServerOpcodes::UpdateAccountData;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.player_guid);
        pkt.write_int64(self.time);
        pkt.write_uint32(self.size);
        pkt.write_bits(u32::from(self.data_type & 0x0F), 4);
        pkt.write_uint32(self.compressed_data.len() as u32);
        pkt.write_bytes(&self.compressed_data);
    }
}

/// Account data cache timestamps. Sent twice during login:
/// once with a global (empty) guid and once with the player's guid.
pub struct AccountDataTimes {
    pub player_guid: ObjectGuid,
    pub server_time: i64,
    pub account_times: [i64; NUM_ACCOUNT_DATA_TYPES],
}

impl AccountDataTimes {
    pub fn for_times(
        player_guid: ObjectGuid,
        account_times: [i64; NUM_ACCOUNT_DATA_TYPES],
    ) -> Self {
        Self {
            player_guid,
            server_time: unix_timestamp(),
            account_times,
        }
    }

    /// Global account data (no player).
    pub fn global() -> Self {
        Self::for_times(ObjectGuid::EMPTY, [0i64; NUM_ACCOUNT_DATA_TYPES])
    }

    /// Per-character account data.
    pub fn for_player(guid: ObjectGuid) -> Self {
        Self::for_times(guid, [0i64; NUM_ACCOUNT_DATA_TYPES])
    }
}

impl ServerPacket for AccountDataTimes {
    const OPCODE: ServerOpcodes = ServerOpcodes::AccountDataTimes;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.player_guid);
        pkt.write_int64(self.server_time);
        for t in &self.account_times {
            pkt.write_int64(*t);
        }
    }
}

// ── Tutorial (CMSG 0x36e4 / SMSG 0x27be) ────────────────────────────

/// Tutorial flags. All 0xFFFFFFFF means all tutorials are shown/completed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TutorialFlags {
    pub tutorial_data: [u32; 8],
}

impl TutorialFlags {
    /// C++ `WorldSession::LoadTutorialsData` defaults to zeroes when no
    /// account_tutorial row exists.
    pub fn none_shown() -> Self {
        Self {
            tutorial_data: [0; 8],
        }
    }

    /// All tutorials shown (client won't display any tutorial pop-ups).
    pub fn all_shown() -> Self {
        Self {
            tutorial_data: [0xFFFFFFFF; 8],
        }
    }
}

impl ServerPacket for TutorialFlags {
    const OPCODE: ServerOpcodes = ServerOpcodes::TutorialFlags;

    fn write(&self, pkt: &mut WorldPacket) {
        for val in &self.tutorial_data {
            pkt.write_uint32(*val);
        }
    }
}

/// C++ `WorldPackets::Misc::TutorialSetFlag`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TutorialSetFlag {
    pub action: u8,
    pub tutorial_bit: Option<u32>,
}

impl ClientPacket for TutorialSetFlag {
    const OPCODE: ClientOpcodes = ClientOpcodes::Tutorial;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        pkt.skip_opcode();
        let action = pkt.read_bits(2)? as u8;
        let tutorial_bit = if action == TUTORIAL_ACTION_UPDATE_LIKE_CPP {
            Some(pkt.read_uint32()?)
        } else {
            None
        };

        Ok(Self {
            action,
            tutorial_bit,
        })
    }
}

// ── ContactList (SMSG 0x278c) ────────────────────────────────────────

/// C++ `CUFProfile`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CufProfile {
    pub profile_name: String,
    pub frame_height: u16,
    pub frame_width: u16,
    pub sort_by: u8,
    pub health_text: u8,
    pub top_point: u8,
    pub bottom_point: u8,
    pub left_point: u8,
    pub top_offset: u16,
    pub bottom_offset: u16,
    pub left_offset: u16,
    pub bool_options: u32,
}

/// C++ `WorldPackets::Misc::SaveCUFProfiles`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveCufProfiles {
    pub profiles: Vec<CufProfile>,
}

impl ClientPacket for SaveCufProfiles {
    const OPCODE: ClientOpcodes = ClientOpcodes::SaveCufProfiles;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        pkt.skip_opcode();
        let count = pkt.read_uint32()? as usize;
        let mut profiles = Vec::with_capacity(count);
        for _ in 0..count {
            let name_len = pkt.read_bits(7)? as usize;
            let mut bool_options = 0u32;
            for option in 0..CUF_BOOL_OPTIONS_COUNT_LIKE_CPP {
                if pkt.read_bit()? {
                    bool_options |= 1 << option;
                }
            }

            profiles.push(CufProfile {
                frame_height: pkt.read_uint16()?,
                frame_width: pkt.read_uint16()?,
                sort_by: pkt.read_uint8()?,
                health_text: pkt.read_uint8()?,
                top_point: pkt.read_uint8()?,
                bottom_point: pkt.read_uint8()?,
                left_point: pkt.read_uint8()?,
                top_offset: pkt.read_uint16()?,
                bottom_offset: pkt.read_uint16()?,
                left_offset: pkt.read_uint16()?,
                profile_name: pkt.read_string(name_len)?,
                bool_options,
            });
        }

        Ok(Self { profiles })
    }
}

/// C++ `WorldPackets::Misc::LoadCUFProfiles`.
pub struct LoadCufProfiles {
    pub profiles: Vec<CufProfile>,
}

impl LoadCufProfiles {
    pub fn empty() -> Self {
        Self {
            profiles: Vec::new(),
        }
    }
}

impl Default for LoadCufProfiles {
    fn default() -> Self {
        Self::empty()
    }
}

impl ServerPacket for LoadCufProfiles {
    const OPCODE: ServerOpcodes = ServerOpcodes::LoadCufProfiles;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.profiles.len() as u32);
        for profile in &self.profiles {
            pkt.write_bits(profile.profile_name.len() as u32, 7);
            for option in 0..CUF_BOOL_OPTIONS_COUNT_LIKE_CPP {
                pkt.write_bit(profile.bool_options & (1 << option) != 0);
            }

            pkt.write_uint16(profile.frame_height);
            pkt.write_uint16(profile.frame_width);
            pkt.write_uint8(profile.sort_by);
            pkt.write_uint8(profile.health_text);
            pkt.write_uint8(profile.top_point);
            pkt.write_uint8(profile.bottom_point);
            pkt.write_uint8(profile.left_point);
            pkt.write_uint16(profile.top_offset);
            pkt.write_uint16(profile.bottom_offset);
            pkt.write_uint16(profile.left_offset);
            pkt.write_string(&profile.profile_name);
        }
    }
}
