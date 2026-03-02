// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Character-related packet definitions: list, create, delete, and login.

use wow_constants::{ClientOpcodes, ServerOpcodes};
use wow_core::{ObjectGuid, Position};

use crate::{ClientPacket, PacketError, ServerPacket, WorldPacket};

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
        // Bit flags (from C# EnumCharactersResult.Write)
        pkt.write_bit(self.success);
        pkt.write_bit(false); // IsDeletedCharacters
        pkt.write_bit(false); // IsNewPlayerRestrictionSkipped
        pkt.write_bit(false); // IsNewPlayerRestricted
        pkt.write_bit(false); // IsNewPlayer
        pkt.write_bit(false); // IsTrialAccountRestricted
        pkt.write_bit(false); // HasDisabledClassesMask
        pkt.flush_bits();

        // Counts
        let max_level = self.characters.iter().map(|c| c.experience_level).max().unwrap_or(0) as i32;
        pkt.write_int32(self.characters.len() as i32);
        pkt.write_int32(max_level); // MaxCharacterLevel
        pkt.write_int32(self.race_unlock_data.len() as i32);
        pkt.write_int32(0); // UnlockedConditionalAppearanceCount
        pkt.write_int32(0); // RaceLimitDisablesCount

        // No DisabledClassesMask (optional, we set bit to false)
        // No UnlockedConditionalAppearances (count=0)
        // No RaceLimitDisables (count=0)

        // Write each character (from C# CharacterInfo.Write)
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

        // RaceUnlockData (C# CharacterPackets.cs RaceUnlock.Write)
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
#[derive(Debug, Clone)]
pub struct ChrCustomizationChoice {
    pub option_id: i32,
    pub choice_id: i32,
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

// ── Response codes ──────────────────────────────────────────────────

/// Result codes for character operations (values from ResponseCodes enum in C#).
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
mod tests {
    use super::*;
    #[test]
    fn enum_characters_result_empty_list() {
        let pkt_data = EnumCharactersResult {
            success: true,
            characters: vec![],
            race_unlock_data: vec![],
        };
        let bytes = pkt_data.to_bytes();
        // Should have at least opcode + bits + counts
        assert!(bytes.len() > 2);
    }

    #[test]
    fn create_char_response_roundtrip() {
        let resp = CreateChar {
            code: response_codes::CHAR_CREATE_SUCCESS,
            guid: ObjectGuid::create_player(1, 100),
        };
        let bytes = resp.to_bytes();
        assert!(bytes.len() > 2);
    }

    #[test]
    fn delete_char_response_roundtrip() {
        let resp = DeleteChar {
            code: response_codes::CHAR_DELETE_SUCCESS,
        };
        let bytes = resp.to_bytes();
        assert_eq!(bytes.len(), 3); // opcode(2) + code(1)
    }

    #[test]
    fn login_verify_world_serialization() {
        let pkt = LoginVerifyWorld {
            map_id: 0,
            position: Position::new(-8949.95, -132.493, 83.5312, 0.0),
            reason: 0,
        };
        let bytes = pkt.to_bytes();
        // opcode(2) + map_id(4) + x(4) + y(4) + z(4) + o(4) + reason(4) = 26
        assert_eq!(bytes.len(), 26);
    }

    #[test]
    fn enum_characters_empty_read() {
        let mut pkt = WorldPacket::from_bytes(&[0x00, 0x00]);
        pkt.skip_opcode();
        let result = EnumCharacters::read(&mut pkt);
        assert!(result.is_ok());
    }

    #[test]
    fn char_delete_read_roundtrip() {
        let guid = ObjectGuid::create_player(1, 42);
        let mut pkt = WorldPacket::new_empty();
        pkt.write_packed_guid(&guid);

        pkt.reset_read();
        let result = CharDelete::read(&mut pkt).unwrap();
        assert_eq!(result.guid, guid);
    }

    #[test]
    fn player_login_read_roundtrip() {
        let guid = ObjectGuid::create_player(1, 99);
        let mut pkt = WorldPacket::new_empty();
        pkt.write_packed_guid(&guid);
        pkt.write_float(1000.0);

        pkt.reset_read();
        let result = PlayerLogin::read(&mut pkt).unwrap();
        assert_eq!(result.guid, guid);
        assert!((result.far_clip - 1000.0).abs() < 0.001);
    }

    #[test]
    fn create_character_read() {
        let mut pkt = WorldPacket::new_empty();
        // Name "Test" = 4 chars
        pkt.write_bits(4, 6);
        pkt.write_bit(false); // has_template_set
        pkt.write_bit(false); // is_trial_boost
        pkt.write_bit(false); // use_npe

        pkt.write_uint8(1); // race: Human
        pkt.write_uint8(1); // class: Warrior
        pkt.write_int8(0);  // sex: Male
        pkt.write_uint32(0); // customization_count

        pkt.write_string("Test");

        pkt.reset_read();
        let result = CreateCharacter::read(&mut pkt).unwrap();
        assert_eq!(result.name, "Test");
        assert_eq!(result.race, 1);
        assert_eq!(result.class, 1);
        assert_eq!(result.sex, 0);
        assert!(result.template_set.is_none());
    }

    #[test]
    fn enum_characters_with_one_character() {
        let char_info = CharacterInfo {
            guid: ObjectGuid::create_player(1, 42),
            name: "TestChar".into(),
            list_position: 0,
            race_id: 1,
            class_id: 1,
            sex_id: 0,
            experience_level: 1,
            zone_id: 12,
            map_id: 0,
            position: Position::new(-8949.95, -132.493, 83.5312, 0.0),
            guild_guid: ObjectGuid::EMPTY,
            first_login: true,
            ..CharacterInfo::default()
        };

        let pkt = EnumCharactersResult {
            success: true,
            characters: vec![char_info],
            race_unlock_data: vec![
                RaceUnlock { race_id: 1, has_expansion: true, has_achievement: false, has_heritage_armor: false, is_locked: false },
            ],
        };
        let bytes = pkt.to_bytes();
        // Should be a reasonably sized packet
        assert!(bytes.len() > 50);
    }
}
