// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Character-related packet definitions: list, create, delete, and login.

use wow_constants::{ClientOpcodes, ServerOpcodes};
use wow_core::{ObjectGuid, Position};

use crate::{ClientPacket, PacketError, ServerPacket, WorldPacket};

/// C++ `WorldPackets::Character::SetTitle`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetTitle {
    pub title_id: i32,
}

impl ClientPacket for SetTitle {
    const OPCODE: ClientOpcodes = ClientOpcodes::SetTitle;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            title_id: pkt.read_int32()?,
        })
    }
}

// ── Visual item info (shared) ───────────────────────────────────────

/// Equipment visual info for a single slot in the character list.
#[derive(Debug, Clone, Copy, Default)]
pub struct VisualItemInfo {
    pub display_id: u32,
    pub display_enchant_id: u32,
    pub secondary_item_modified_appearance_id: i32,
    pub inv_type: u8,
    pub subclass: u8,
}

// ── Character info (shared) ─────────────────────────────────────────

/// Information about a single character in the character list.
#[derive(Debug, Clone)]
pub struct CharacterInfo {
    pub guid: ObjectGuid,
    pub guild_club_member_id: u64,
    pub name: String,
    pub list_position: u8,
    pub race_id: u8,
    pub class_id: u8,
    pub sex_id: u8,
    pub experience_level: u8,
    pub zone_id: i32,
    pub map_id: i32,
    pub position: Position,
    pub guild_guid: ObjectGuid,
    pub flags: u32,
    pub flags2: u32,
    pub flags3: u32,
    pub flags4: u32,
    pub first_login: bool,
    pub pet_display_id: u32,
    pub pet_level: u32,
    pub pet_family: u32,
    pub profession_ids: [u32; 2],
    pub equipment: [VisualItemInfo; 34],
    pub last_played_time: i64,
    pub spec_id: i16,
    pub last_login_version: i32,
    pub override_select_screen_file_data_id: u32,
}

impl Default for CharacterInfo {
    fn default() -> Self {
        Self {
            guid: ObjectGuid::EMPTY,
            guild_club_member_id: 0,
            name: String::new(),
            list_position: 0,
            race_id: 0,
            class_id: 0,
            sex_id: 0,
            experience_level: 0,
            zone_id: 0,
            map_id: 0,
            position: Position::ZERO,
            guild_guid: ObjectGuid::EMPTY,
            flags: 0,
            flags2: 0,
            flags3: 0,
            flags4: 0,
            first_login: false,
            pet_display_id: 0,
            pet_level: 0,
            pet_family: 0,
            profession_ids: [0; 2],
            equipment: [VisualItemInfo::default(); 34],
            last_played_time: 0,
            spec_id: 0,
            last_login_version: 54261,
            override_select_screen_file_data_id: 0,
        }
    }
}

// ── Race unlock data ────────────────────────────────────────────────

/// Tells the client whether a race is available for character creation/login.
#[derive(Debug, Clone)]
pub struct RaceUnlock {
    pub race_id: u8,
    pub has_expansion: bool,
    pub has_achievement: bool,
    pub has_heritage_armor: bool,
    pub is_locked: bool,
}

// ── Server: EnumCharactersResult (SMSG 0x2583) ──────────────────────

/// Response to the client's character list request.
pub struct EnumCharactersResult {
    pub success: bool,
    pub characters: Vec<CharacterInfo>,
    pub race_unlock_data: Vec<RaceUnlock>,
}

impl ServerPacket for EnumCharactersResult {
    const OPCODE: ServerOpcodes = ServerOpcodes::EnumCharactersResult;

    fn write(&self, pkt: &mut WorldPacket) {
        // C++ `WorldPackets::Character::EnumCharactersResult::Write`
        // writes these presence/status bits before the fixed count block.
        pkt.write_bit(self.success);
        pkt.write_bit(false); // IsDeletedCharacters
        pkt.write_bit(false); // IsNewPlayerRestrictionSkipped
        pkt.write_bit(false); // IsNewPlayerRestricted
        pkt.write_bit(false); // IsNewPlayer
        pkt.write_bit(false); // IsTrialAccountRestricted
        pkt.write_bit(false); // HasDisabledClassesMask
        pkt.flush_bits();

        // Counts
        let max_level = self
            .characters
            .iter()
            .map(|c| c.experience_level)
            .max()
            .unwrap_or(0) as i32;
        pkt.write_int32(self.characters.len() as i32);
        pkt.write_int32(max_level); // MaxCharacterLevel
        pkt.write_int32(self.race_unlock_data.len() as i32);
        pkt.write_int32(0); // UnlockedConditionalAppearanceCount
        pkt.write_int32(0); // RaceLimitDisablesCount

        // No DisabledClassesMask (optional, we set bit to false)
        // No UnlockedConditionalAppearances (count=0)
        // No RaceLimitDisables (count=0)

        // Write each character like C++ `EnumCharactersResult::CharacterInfo`.
        for ch in &self.characters {
            pkt.write_packed_guid(&ch.guid);
            pkt.write_uint64(ch.guild_club_member_id);
            pkt.write_uint8(ch.list_position);
            pkt.write_uint8(ch.race_id);
            pkt.write_uint8(ch.class_id);
            pkt.write_uint8(ch.sex_id);
            pkt.write_int32(0); // Customizations.Count (no customizations)

            pkt.write_uint8(ch.experience_level);
            pkt.write_int32(ch.zone_id);
            pkt.write_int32(ch.map_id);
            pkt.write_float(ch.position.x);
            pkt.write_float(ch.position.y);
            pkt.write_float(ch.position.z);
            pkt.write_packed_guid(&ch.guild_guid);

            pkt.write_uint32(ch.flags);
            pkt.write_uint32(ch.flags2);
            pkt.write_uint32(ch.flags3);

            pkt.write_uint32(ch.pet_display_id);
            pkt.write_uint32(ch.pet_level);
            pkt.write_uint32(ch.pet_family);

            pkt.write_uint32(ch.profession_ids[0]);
            pkt.write_uint32(ch.profession_ids[1]);

            // Equipment (34 visual items)
            for item in &ch.equipment {
                pkt.write_uint32(item.display_id);
                pkt.write_uint32(item.display_enchant_id);
                pkt.write_int32(item.secondary_item_modified_appearance_id);
                pkt.write_uint8(item.inv_type);
                pkt.write_uint8(item.subclass);
            }

            pkt.write_int64(ch.last_played_time);
            pkt.write_int16(ch.spec_id);
            pkt.write_int32(0); // Unknown703
            pkt.write_int32(ch.last_login_version);
            pkt.write_uint32(ch.flags4);
            pkt.write_int32(0); // MailSenders.Count
            pkt.write_int32(0); // MailSenderTypes.Count
            pkt.write_uint32(ch.override_select_screen_file_data_id);

            // Customizations array (empty, count=0 above)
            // MailSenderTypes array (empty, count=0 above)

            // Bit-packed fields
            pkt.write_bits(ch.name.len() as u32, 6);
            pkt.write_bit(ch.first_login);
            pkt.write_bit(false); // BoostInProgress
            pkt.write_bits(0, 5); // unkWod61x
            pkt.write_bits(0, 2); // unknown
            pkt.write_bit(false); // RpeResetAvailable
            pkt.write_bit(false); // RpeResetQuestClearAvailable
            // MailSenders bit lengths (none, count=0)
            pkt.flush_bits();

            // MailSenders strings (none)
            // Character name
            pkt.write_string(&ch.name);
        }

        // C++ `EnumCharactersResult::RaceUnlock` writes RaceID followed by four bits.
        for ru in &self.race_unlock_data {
            pkt.write_int32(ru.race_id as i32);
            pkt.write_bit(ru.has_expansion);
            pkt.write_bit(ru.has_achievement);
            pkt.write_bit(ru.has_heritage_armor);
            pkt.write_bit(ru.is_locked);
            pkt.flush_bits();
        }
    }
}

// ── Server: CreateChar (SMSG 0x2701) ────────────────────────────────

/// Response to character creation.
pub struct CreateChar {
    pub code: u8,
    pub guid: ObjectGuid,
}

impl ServerPacket for CreateChar {
    const OPCODE: ServerOpcodes = ServerOpcodes::CreateChar;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint8(self.code);
        pkt.write_packed_guid(&self.guid);
    }
}

// ── Server: DeleteChar (SMSG 0x2702) ────────────────────────────────

/// Response to character deletion.
pub struct DeleteChar {
    pub code: u8,
}

impl ServerPacket for DeleteChar {
    const OPCODE: ServerOpcodes = ServerOpcodes::DeleteChar;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint8(self.code);
    }
}

// ── Server: LoginVerifyWorld (SMSG 0x2597) ──────────────────────────

/// Sent after PlayerLogin to confirm world entry.
pub struct LoginVerifyWorld {
    pub map_id: i32,
    pub position: Position,
    pub reason: u32,
}

impl ServerPacket for LoginVerifyWorld {
    const OPCODE: ServerOpcodes = ServerOpcodes::LoginVerifyWorld;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(self.map_id);
        pkt.write_float(self.position.x);
        pkt.write_float(self.position.y);
        pkt.write_float(self.position.z);
        pkt.write_float(self.position.orientation);
        pkt.write_uint32(self.reason);
    }
}

// ── Server: CharacterLoginFailed (SMSG 0x2705) ───────────────────────

/// C++ `WorldPackets::Character::LoginFailureReason`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum LoginFailureReasonLikeCpp {
    Failed = 0,
    NoWorld = 1,
    DuplicateCharacter = 2,
    NoInstances = 3,
    Disabled = 4,
    NoCharacter = 5,
    LockedForTransfer = 6,
    LockedByBilling = 7,
    LockedByMobileAh = 8,
    TemporaryGmLock = 9,
    LockedByCharacterUpgrade = 10,
    LockedByRevokedCharacterUpgrade = 11,
    LockedByRevokedVasTransaction = 17,
    LockedByRestriction = 19,
    LockedForRealmPlaytype = 23,
}

/// C++ `WorldPackets::Character::CharacterLoginFailed::Write`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CharacterLoginFailed {
    pub code: LoginFailureReasonLikeCpp,
}

impl ServerPacket for CharacterLoginFailed {
    const OPCODE: ServerOpcodes = ServerOpcodes::CharacterLoginFailed;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint8(self.code as u8);
    }
}

// ── Client: EnumCharacters (CMSG 0x35e9) ────────────────────────────

/// Client request to list characters.
pub struct EnumCharacters;

impl ClientPacket for EnumCharacters {
    const OPCODE: ClientOpcodes = ClientOpcodes::EnumCharacters;

    fn read(_packet: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(EnumCharacters)
    }
}

// ── Client: CreateCharacter (CMSG 0x3645) ───────────────────────────

/// Customization choice for character creation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChrCustomizationChoice {
    pub option_id: i32,
    pub choice_id: i32,
}

/// C++ `WorldPackets::Character::AlterApperance`.
#[derive(Debug, Clone)]
pub struct AlterAppearance {
    pub new_sex: u8,
    pub customizations: Vec<ChrCustomizationChoice>,
    pub customized_race: i32,
    pub customized_chr_model_id: i32,
}

impl ClientPacket for AlterAppearance {
    const OPCODE: ClientOpcodes = ClientOpcodes::AlterAppearance;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let customization_count = pkt.read_uint32()? as usize;
        let new_sex = pkt.read_uint8()?;
        let customized_race = pkt.read_int32()?;
        let customized_chr_model_id = pkt.read_int32()?;
        let mut customizations = Vec::with_capacity(customization_count);
        for _ in 0..customization_count {
            customizations.push(ChrCustomizationChoice {
                option_id: pkt.read_int32()?,
                choice_id: pkt.read_int32()?,
            });
        }
        customizations.sort_by_key(|choice| choice.option_id);

        Ok(Self {
            new_sex,
            customizations,
            customized_race,
            customized_chr_model_id,
        })
    }
}

/// C++ `WorldPackets::Character::ConfirmBarbersChoice`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfirmBarbersChoice {
    pub customizations: Vec<ChrCustomizationChoice>,
}

impl ClientPacket for ConfirmBarbersChoice {
    const OPCODE: ClientOpcodes = ClientOpcodes::ConfirmBarbersChoice;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let customization_count = pkt.read_uint32()? as usize;
        let mut customizations = Vec::with_capacity(customization_count);
        for _ in 0..customization_count {
            customizations.push(ChrCustomizationChoice {
                option_id: pkt.read_uint32()? as i32,
                choice_id: pkt.read_uint32()? as i32,
            });
        }

        Ok(Self { customizations })
    }
}

pub const BARBER_SHOP_RESULT_SUCCESS_LIKE_CPP: i32 = 0;
pub const BARBER_SHOP_RESULT_NO_MONEY_LIKE_CPP: i32 = 1;
pub const BARBER_SHOP_RESULT_NOT_ON_CHAIR_LIKE_CPP: i32 = 2;

/// C++ `WorldPackets::Character::BarberShopResult`.
pub struct BarberShopResult {
    pub result: i32,
}

impl ServerPacket for BarberShopResult {
    const OPCODE: ServerOpcodes = ServerOpcodes::BarberShopResult;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(self.result);
    }
}

pub const MAX_DECLINED_NAME_CASES_LIKE_CPP: usize = 5;
pub const DECLINED_NAMES_RESULT_SUCCESS_LIKE_CPP: i32 = 0;
pub const DECLINED_NAMES_RESULT_ERROR_LIKE_CPP: i32 = 1;

/// C++ `DeclinedName` payload: genitive, dative, accusative,
/// instrumental, and prepositional cases.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclinedNameCasesLikeCpp {
    pub names: [String; MAX_DECLINED_NAME_CASES_LIKE_CPP],
}

/// C++ `WorldPackets::Character::SetPlayerDeclinedNames`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetPlayerDeclinedNames {
    pub player: ObjectGuid,
    pub declined_names: DeclinedNameCasesLikeCpp,
}

impl ClientPacket for SetPlayerDeclinedNames {
    const OPCODE: ClientOpcodes = ClientOpcodes::SetPlayerDeclinedNames;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let player = pkt.read_guid()?;
        let mut lengths = [0usize; MAX_DECLINED_NAME_CASES_LIKE_CPP];
        for length in &mut lengths {
            *length = pkt.read_bits(7)? as usize;
        }

        let names = [
            pkt.read_string(lengths[0])?,
            pkt.read_string(lengths[1])?,
            pkt.read_string(lengths[2])?,
            pkt.read_string(lengths[3])?,
            pkt.read_string(lengths[4])?,
        ];

        Ok(Self {
            player,
            declined_names: DeclinedNameCasesLikeCpp { names },
        })
    }
}

/// C++ `WorldPackets::Character::SetPlayerDeclinedNamesResult`.
pub struct SetPlayerDeclinedNamesResult {
    pub player: ObjectGuid,
    pub result_code: i32,
}

impl ServerPacket for SetPlayerDeclinedNamesResult {
    const OPCODE: ServerOpcodes = ServerOpcodes::SetPlayerDeclinedNamesResult;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(self.result_code);
        pkt.write_guid(&self.player);
    }
}

/// Client request to create a character.
#[derive(Debug, Clone)]
pub struct CreateCharacter {
    pub name: String,
    pub race: u8,
    pub class: u8,
    pub sex: i8,
    pub customizations: Vec<ChrCustomizationChoice>,
    pub template_set: Option<i32>,
    pub is_trial_boost: bool,
    pub use_npe: bool,
}

impl ClientPacket for CreateCharacter {
    const OPCODE: ClientOpcodes = ClientOpcodes::CreateCharacter;

    fn read(packet: &mut WorldPacket) -> Result<Self, PacketError> {
        let name_len = packet.read_bits(6)? as usize;
        let has_template_set = packet.read_bit()?;
        let is_trial_boost = packet.read_bit()?;
        let use_npe = packet.read_bit()?;

        let race = packet.read_uint8()?;
        let class = packet.read_uint8()?;
        let sex = packet.read_int8()?;
        let customization_count = packet.read_uint32()? as usize;

        let name = packet.read_string(name_len)?;

        let template_set = if has_template_set {
            Some(packet.read_int32()?)
        } else {
            None
        };

        let mut customizations = Vec::with_capacity(customization_count);
        for _ in 0..customization_count {
            customizations.push(ChrCustomizationChoice {
                option_id: packet.read_int32()?,
                choice_id: packet.read_int32()?,
            });
        }
        customizations.sort_by_key(|choice| choice.option_id);

        Ok(CreateCharacter {
            name,
            race,
            class,
            sex,
            customizations,
            template_set,
            is_trial_boost,
            use_npe,
        })
    }
}

// ── Client: CharDelete (CMSG 0x369d) ────────────────────────────────

/// Client request to delete a character.
pub struct CharDelete {
    pub guid: ObjectGuid,
}

impl ClientPacket for CharDelete {
    const OPCODE: ClientOpcodes = ClientOpcodes::CharDelete;

    fn read(packet: &mut WorldPacket) -> Result<Self, PacketError> {
        let guid = packet.read_packed_guid()?;
        Ok(CharDelete { guid })
    }
}

// ── Client: PlayerLogin (CMSG 0x35eb) ───────────────────────────────

/// Client request to log in with a specific character.
pub struct PlayerLogin {
    pub guid: ObjectGuid,
    pub far_clip: f32,
}

impl ClientPacket for PlayerLogin {
    const OPCODE: ClientOpcodes = ClientOpcodes::PlayerLogin;

    fn read(packet: &mut WorldPacket) -> Result<Self, PacketError> {
        let guid = packet.read_packed_guid()?;
        let far_clip = packet.read_float()?;
        Ok(PlayerLogin { guid, far_clip })
    }
}

/// C++ `WorldPackets::Character::CharacterRenameRequest`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterRenameRequest {
    pub guid: ObjectGuid,
    pub new_name: String,
}

impl ClientPacket for CharacterRenameRequest {
    const OPCODE: ClientOpcodes = ClientOpcodes::CharacterRenameRequest;

    fn read(packet: &mut WorldPacket) -> Result<Self, PacketError> {
        let guid = packet.read_guid()?;
        let new_name_len = packet.read_bits(6)? as usize;
        let new_name = packet.read_string(new_name_len)?;
        Ok(Self { guid, new_name })
    }
}

/// C++ `WorldPackets::Character::CharacterRenameResult`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterRenameResult {
    pub result: u8,
    pub name: String,
    pub guid: Option<ObjectGuid>,
}

impl ServerPacket for CharacterRenameResult {
    const OPCODE: ServerOpcodes = ServerOpcodes::CharacterRenameResult;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint8(self.result);
        pkt.write_bit(self.guid.is_some());
        pkt.write_bits(self.name.len() as u32, 6);
        pkt.flush_bits();

        if let Some(guid) = self.guid {
            pkt.write_guid(&guid);
        }

        pkt.write_string(&self.name);
    }
}

/// C++ `WorldPackets::Character::CharCustomize`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharCustomize {
    pub guid: ObjectGuid,
    pub sex_id: u8,
    pub customizations: Vec<ChrCustomizationChoice>,
    pub name: String,
}

impl ClientPacket for CharCustomize {
    const OPCODE: ClientOpcodes = ClientOpcodes::CharCustomize;

    fn read(packet: &mut WorldPacket) -> Result<Self, PacketError> {
        let guid = packet.read_guid()?;
        let sex_id = packet.read_uint8()?;
        let customization_count = packet.read_uint32()? as usize;
        let mut customizations = Vec::with_capacity(customization_count);
        for _ in 0..customization_count {
            customizations.push(ChrCustomizationChoice {
                option_id: packet.read_int32()?,
                choice_id: packet.read_int32()?,
            });
        }
        customizations.sort_by_key(|choice| choice.option_id);
        let name_len = packet.read_bits(6)? as usize;
        let name = packet.read_string(name_len)?;

        Ok(Self {
            guid,
            sex_id,
            customizations,
            name,
        })
    }
}

/// C++ `WorldPackets::Character::CharCustomizeSuccess`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharCustomizeSuccess {
    pub guid: ObjectGuid,
    pub sex_id: u8,
    pub customizations: Vec<ChrCustomizationChoice>,
    pub name: String,
}

impl ServerPacket for CharCustomizeSuccess {
    const OPCODE: ServerOpcodes = ServerOpcodes::CharCustomizeSuccess;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_guid(&self.guid);
        pkt.write_uint8(self.sex_id);
        pkt.write_uint32(self.customizations.len() as u32);
        for customization in &self.customizations {
            pkt.write_int32(customization.option_id);
            pkt.write_int32(customization.choice_id);
        }
        pkt.write_bits(self.name.len() as u32, 6);
        pkt.flush_bits();
        pkt.write_string(&self.name);
    }
}

/// C++ `WorldPackets::Character::CharCustomizeFailure`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharCustomizeFailure {
    pub result: u8,
    pub guid: ObjectGuid,
}

impl ServerPacket for CharCustomizeFailure {
    const OPCODE: ServerOpcodes = ServerOpcodes::CharCustomizeFailure;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint8(self.result);
        pkt.write_guid(&self.guid);
    }
}

// ── Response codes ──────────────────────────────────────────────────

/// Result codes for character operations (`SharedDefines.h` `ResponseCodes`).
#[allow(dead_code)]
pub mod response_codes {
    // Character creation results
    pub const CHAR_CREATE_SUCCESS: u8 = 24;
    pub const CHAR_CREATE_ERROR: u8 = 25;
    pub const CHAR_CREATE_FAILED: u8 = 26;
    pub const CHAR_CREATE_NAME_IN_USE: u8 = 27;
    pub const CHAR_CREATE_DISABLED: u8 = 28;
    pub const CHAR_CREATE_PVP_TEAMS_VIOLATION: u8 = 29;
    pub const CHAR_CREATE_SERVER_LIMIT: u8 = 30;
    pub const CHAR_CREATE_ACCOUNT_LIMIT: u8 = 31;
    pub const CHAR_CREATE_EXPANSION: u8 = 34;
    pub const CHAR_CREATE_EXPANSION_CLASS: u8 = 35;
    pub const CHAR_CREATE_NEW_PLAYER: u8 = 51;

    // Character deletion results
    pub const CHAR_DELETE_SUCCESS: u8 = 63;
    pub const CHAR_DELETE_FAILED: u8 = 64;
    pub const CHAR_DELETE_FAILED_LOCKED_FOR_TRANSFER: u8 = 65;
    pub const CHAR_DELETE_FAILED_GUILD_LEADER: u8 = 66;
    pub const CHAR_DELETE_FAILED_ARENA_CAPTAIN: u8 = 67;
    pub const CHAR_DELETE_FAILED_HAS_HEIRLOOM_OR_MAIL: u8 = 68;

    // Character login results
    pub const CHAR_LOGIN_SUCCESS: u8 = 0;
    pub const CHAR_LOGIN_NO_WORLD: u8 = 2;
    pub const CHAR_LOGIN_FAILED: u8 = 17;
    pub const CHAR_LOGIN_DISABLED: u8 = 18;
    pub const CHAR_LOGIN_NO_CHARACTER: u8 = 19;
    pub const CHAR_LOGIN_LOCKED_FOR_TRANSFER: u8 = 20;
    pub const CHAR_LOGIN_LOCKED_BY_BILLING: u8 = 21;
}

#[cfg(test)]
#[path = "character/tests/mod.rs"]
mod tests;
