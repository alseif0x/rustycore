//! Payload-only Forever local character-list responses.
//!
//! Layout anchors are `CharacterPackets.cpp:220-461` and
//! `CharacterPackets.h:116-300` at
//! `02245dcd245e7433e524577656177723d3e4992e`.  This projection includes
//! local `CharacterInfo`, race unlocks, conditional appearances and race-limit
//! rows.  Region-wide and warband rows remain zero-count because those are not
//! part of the current local Forever operation; the existing
//! `EmptyEnumCharactersResult` in `forever.rs` is intentionally unchanged.
//!
//! The 70170 wire has not been independently captured for every populated
//! character field.  This module therefore exposes the exact pinned source
//! layout, without claiming native 70170 parity or character admission.

use thiserror::Error;
use wow_core::{ObjectGuid, Position};

use super::{
    ClassUnlock, ForeverPacketError, RaceUnlock, character_create::CustomizationChoice, finish,
    length,
};
use crate::WorldPacket;

/// Core opcode for C++ `SMSG_ENUM_CHARACTERS_RESULT` (`Opcodes.h:1465`).
pub const ENUM_CHARACTERS_RESULT_CORE_OPCODE: u32 = 0x450018;

/// Classic group-shift translation of the core enum opcode.
pub const ENUM_CHARACTERS_RESULT_CLASSIC_OPCODE: u32 = 0x460018;

/// Source `SharedDefines.h` maximum local characters per realm.
const MAX_CHARACTERS_PER_REALM: usize = 200;
/// These source vectors have uint32 counts, not Create's fixed-array cap.
const MAX_CUSTOMIZATIONS: usize = u32::MAX as usize;
const MAX_VECTOR_ELEMENTS: usize = u32::MAX as usize;
const STRING_LENGTH_BITS: u32 = 6;
const MAX_STRING_BYTES: usize = (1 << STRING_LENGTH_BITS) - 1;
/// `SizedCString` writes the terminating NUL in the six-bit size field.
const MAX_CSTRING_BYTES: usize = (1 << STRING_LENGTH_BITS) - 2;
const VISUAL_ITEM_COUNT: usize = 19;

/// A visual item from `CharacterInfoBasic::VisualItems`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct VisualItemInfo {
    pub item_id: u32,
    pub transmogrified_item_id: u32,
    pub subclass: u8,
    pub inv_type: u8,
    pub display_id: u32,
    pub display_enchant_id: u32,
    pub secondary_item_modified_appearance_id: i32,
    pub sheathe_category: u8,
}

impl Default for VisualItemInfo {
    fn default() -> Self {
        Self {
            item_id: 0,
            transmogrified_item_id: 0,
            subclass: 0,
            inv_type: 0,
            display_id: 0,
            display_enchant_id: 0,
            secondary_item_modified_appearance_id: 0,
            sheathe_category: 0,
        }
    }
}

/// C++ `CustomTabardInfo`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct CustomTabardInfo {
    pub emblem_style: i32,
    pub emblem_color: i32,
    pub border_style: i32,
    pub border_color: i32,
    pub background_color: i32,
}

impl Default for CustomTabardInfo {
    fn default() -> Self {
        Self {
            emblem_style: -1,
            emblem_color: -1,
            border_style: -1,
            border_color: -1,
            background_color: -1,
        }
    }
}

/// The complete local `CharacterInfoBasic` projection, without Debug so names
/// and surnames are not accidentally rendered by diagnostics.
#[derive(Clone, PartialEq)]
pub struct CharacterInfoBasic {
    pub guid: ObjectGuid,
    pub virtual_realm_address: u32,
    pub guild_club_member_id: u64,
    name: String,
    surname: String,
    pub list_position: u16,
    pub race_id: u8,
    pub class_id: u8,
    pub sex_id: u8,
    pub customizations: Vec<CustomizationChoice>,
    pub experience_level: u8,
    pub zone_id: i32,
    pub map_id: i32,
    /// Source `TaggedPosition<Position::XYZ>`; `orientation` is not on this wire.
    pub preload_position: Position,
    pub guild_guid: ObjectGuid,
    pub flags: u32,
    pub flags2: u32,
    pub flags3: u32,
    pub flags4: u32,
    pub first_login: bool,
    pub cant_login_reason: u8,
    pub create_time: i64,
    pub last_active_time: i64,
    pub spec_id: u16,
    pub save_version: u32,
    pub last_login_version: u32,
    pub override_select_screen_file_data_id: u32,
    pub timerunning_season_id: i32,
    pub pet_creature_display_id: u32,
    pub pet_experience_level: u32,
    pub pet_creature_family_id: u32,
    pub profession_ids: [u32; 2],
    pub visual_items: [VisualItemInfo; VISUAL_ITEM_COUNT],
    pub personal_tabard: CustomTabardInfo,
    pub realm_queue: u32,
    /// Classic 1.60 source-specific field written before customizations.
    pub super_district_id: i32,
    pub realm_info_found: bool,
    pub is_realm_offline: bool,
}

impl CharacterInfoBasic {
    pub fn new(name: impl Into<String>, surname: impl Into<String>) -> Self {
        Self {
            guid: ObjectGuid::EMPTY,
            virtual_realm_address: 0,
            guild_club_member_id: 0,
            name: name.into(),
            surname: surname.into(),
            list_position: 0,
            race_id: 0,
            class_id: 0,
            sex_id: 0,
            customizations: Vec::new(),
            experience_level: 0,
            zone_id: 0,
            map_id: 0,
            preload_position: Position::ZERO,
            guild_guid: ObjectGuid::EMPTY,
            flags: 0,
            flags2: 0,
            flags3: 0,
            flags4: 0,
            first_login: false,
            cant_login_reason: 0,
            create_time: 0,
            last_active_time: 0,
            spec_id: 0,
            save_version: 0,
            last_login_version: 0,
            override_select_screen_file_data_id: 0,
            timerunning_season_id: 0,
            pet_creature_display_id: 0,
            pet_experience_level: 0,
            pet_creature_family_id: 0,
            profession_ids: [0; 2],
            visual_items: [VisualItemInfo::default(); VISUAL_ITEM_COUNT],
            personal_tabard: CustomTabardInfo::default(),
            realm_queue: 0,
            super_district_id: 0,
            realm_info_found: false,
            is_realm_offline: false,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn surname(&self) -> &str {
        &self.surname
    }
}

/// C++ `CharacterRestrictionAndMailData`.
#[derive(Clone, PartialEq, Eq)]
pub struct CharacterRestrictionAndMailData {
    pub boost_in_progress: bool,
    pub restriction_flags: u32,
    mail_senders: Vec<String>,
    mail_sender_types: Vec<u32>,
    pub rpe_available: bool,
    pub no_rpe_reason: u32,
}

impl CharacterRestrictionAndMailData {
    pub fn new(mail_senders: Vec<String>, mail_sender_types: Vec<u32>) -> Self {
        Self {
            boost_in_progress: false,
            restriction_flags: 0,
            mail_senders,
            mail_sender_types,
            rpe_available: false,
            no_rpe_reason: 4,
        }
    }

    pub fn empty() -> Self {
        Self::new(Vec::new(), Vec::new())
    }

    pub fn mail_senders(&self) -> &[String] {
        &self.mail_senders
    }

    pub fn mail_sender_types(&self) -> &[u32] {
        &self.mail_sender_types
    }
}

/// C++ `EnumCharactersResult::CharacterInfo`.
#[derive(Clone, PartialEq)]
pub struct CharacterInfo {
    pub basic: CharacterInfoBasic,
    pub restrictions_and_mails: CharacterRestrictionAndMailData,
}

impl CharacterInfo {
    pub fn new(
        basic: CharacterInfoBasic,
        restrictions_and_mails: CharacterRestrictionAndMailData,
    ) -> Self {
        Self {
            basic,
            restrictions_and_mails,
        }
    }
}

/// C++ `EnumCharactersResult::UnlockedConditionalAppearance`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct UnlockedConditionalAppearance {
    pub achievement_id: i32,
    pub conditional_type: i32,
}

/// C++ `EnumCharactersResult::RaceLimitDisableInfo`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct RaceLimitDisableInfo {
    pub race_id: i8,
    pub reason: i8,
}

/// Local part of C++ `EnumCharactersResult`.
pub struct EnumCharactersResult {
    pub success: bool,
    pub realmless: bool,
    pub is_deleted_characters: bool,
    pub ignore_new_player_restrictions: bool,
    pub is_restricted_new_player: bool,
    pub is_newcomer_chat_completed: bool,
    pub is_restricted_trial: bool,
    pub is_account_lapsed_player: bool,
    pub force_character_list_sort: bool,
    pub max_character_level: i32,
    pub class_disable_mask: Option<u32>,
    pub characters: Vec<CharacterInfo>,
    pub race_unlock_data: Vec<RaceUnlock>,
    pub unlocked_conditional_appearances: Vec<UnlockedConditionalAppearance>,
    pub race_limit_disables: Vec<RaceLimitDisableInfo>,
}

impl EnumCharactersResult {
    pub fn new() -> Self {
        Self {
            success: false,
            realmless: false,
            is_deleted_characters: false,
            ignore_new_player_restrictions: false,
            is_restricted_new_player: false,
            is_newcomer_chat_completed: false,
            is_restricted_trial: false,
            is_account_lapsed_player: false,
            force_character_list_sort: false,
            max_character_level: 1,
            class_disable_mask: None,
            characters: Vec::new(),
            race_unlock_data: Vec::new(),
            unlocked_conditional_appearances: Vec::new(),
            race_limit_disables: Vec::new(),
        }
    }

    pub fn encode_payload(&self) -> Result<Vec<u8>, CharacterListError> {
        bounded_count(
            self.characters.len(),
            MAX_CHARACTERS_PER_REALM,
            "characters",
        )?;
        bounded_count(
            self.race_unlock_data.len(),
            MAX_VECTOR_ELEMENTS,
            "race_unlock_data",
        )?;
        bounded_count(
            self.unlocked_conditional_appearances.len(),
            MAX_VECTOR_ELEMENTS,
            "unlocked_conditional_appearances",
        )?;
        bounded_count(
            self.race_limit_disables.len(),
            MAX_VECTOR_ELEMENTS,
            "race_limit_disables",
        )?;

        let mut packet = WorldPacket::new_empty();
        packet.write_bit(self.success);
        packet.write_bit(self.realmless);
        packet.write_bit(self.is_deleted_characters);
        packet.write_bit(self.ignore_new_player_restrictions);
        packet.write_bit(self.is_restricted_new_player);
        packet.write_bit(self.is_newcomer_chat_completed);
        packet.write_bit(self.is_restricted_trial);
        packet.write_bit(self.is_account_lapsed_player);
        packet.write_bit(self.class_disable_mask.is_some());
        packet.write_bit(self.force_character_list_sort);
        packet.flush_bits();

        packet.write_uint32(count(self.characters.len(), "characters")?);
        // RegionwideCharacters are deliberately not part of this local DTO.
        packet.write_uint32(0);
        packet.write_int32(self.max_character_level);
        packet.write_uint32(count(self.race_unlock_data.len(), "race_unlock_data")?);
        packet.write_uint32(count(
            self.unlocked_conditional_appearances.len(),
            "unlocked_conditional_appearances",
        )?);
        packet.write_uint32(count(
            self.race_limit_disables.len(),
            "race_limit_disables",
        )?);
        // WarbandGroups are deliberately not part of this local DTO.
        packet.write_uint32(0);

        if let Some(mask) = self.class_disable_mask {
            packet.write_uint32(mask);
        }
        for character in &self.characters {
            write_character(&mut packet, character)?;
        }
        for race in &self.race_unlock_data {
            write_race_unlock(&mut packet, race)?;
        }
        for appearance in &self.unlocked_conditional_appearances {
            packet.write_int32(appearance.achievement_id);
            packet.write_int32(appearance.conditional_type);
        }
        for limit in &self.race_limit_disables {
            packet.write_int8(limit.race_id);
            packet.write_int8(limit.reason);
        }

        Ok(finish(packet))
    }
}

impl Default for EnumCharactersResult {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Error)]
pub enum CharacterListError {
    #[error(transparent)]
    Packet(#[from] ForeverPacketError),
    #[error("character-list position contains a non-finite coordinate")]
    NonFinitePosition,
    #[error("mail sender and sender-type counts differ")]
    MailCountMismatch,
}

fn count(value: usize, field: &'static str) -> Result<u32, ForeverPacketError> {
    super::count(value, field)
}

fn bounded_count(
    value: usize,
    maximum: usize,
    field: &'static str,
) -> Result<u32, CharacterListError> {
    if value > maximum {
        return Err(CharacterListError::Packet(ForeverPacketError::Count {
            field,
            actual: value,
            maximum,
        }));
    }
    Ok(count(value, field)?)
}

fn validate_name(value: &str, field: &'static str) -> Result<(), CharacterListError> {
    length(value.as_bytes(), MAX_STRING_BYTES, field).map_err(CharacterListError::Packet)
}

fn write_name_length(
    packet: &mut WorldPacket,
    value: &str,
    field: &'static str,
) -> Result<(), CharacterListError> {
    validate_name(value, field)?;
    packet.write_bits(value.len() as u32, STRING_LENGTH_BITS);
    Ok(())
}

fn validate_cstring(value: &str, field: &'static str) -> Result<(), CharacterListError> {
    length(value.as_bytes(), MAX_CSTRING_BYTES, field).map_err(CharacterListError::Packet)
}

fn write_cstring_length(
    packet: &mut WorldPacket,
    value: &str,
    field: &'static str,
) -> Result<(), CharacterListError> {
    validate_cstring(value, field)?;
    // PacketOperators::SizedCString::SizeWriter counts the NUL even when the
    // string is empty.  DataWriter emits the NUL only for non-empty values.
    packet.write_bits((value.len() + 1) as u32, STRING_LENGTH_BITS);
    Ok(())
}

fn write_character(
    packet: &mut WorldPacket,
    character: &CharacterInfo,
) -> Result<(), CharacterListError> {
    write_basic(packet, &character.basic)?;
    write_restrictions(packet, &character.restrictions_and_mails)
}

fn write_basic(
    packet: &mut WorldPacket,
    value: &CharacterInfoBasic,
) -> Result<(), CharacterListError> {
    validate_name(value.name(), "character_name")?;
    validate_name(value.surname(), "character_surname")?;
    bounded_count(
        value.customizations.len(),
        MAX_CUSTOMIZATIONS,
        "customizations",
    )?;
    if !value.preload_position.x.is_finite()
        || !value.preload_position.y.is_finite()
        || !value.preload_position.z.is_finite()
    {
        return Err(CharacterListError::NonFinitePosition);
    }

    packet.write_packed_guid(&value.guid);
    packet.write_uint32(value.virtual_realm_address);
    packet.write_uint16(value.list_position);
    packet.write_uint8(value.race_id);
    packet.write_uint8(value.sex_id);
    packet.write_uint8(value.class_id);
    packet.write_int16(value.spec_id as i16);
    packet.write_uint32(count(value.customizations.len(), "customizations")?);
    packet.write_uint8(value.experience_level);
    packet.write_int32(value.map_id);
    packet.write_int32(value.zone_id);
    packet.write_float(value.preload_position.x);
    packet.write_float(value.preload_position.y);
    packet.write_float(value.preload_position.z);
    packet.write_uint64(value.guild_club_member_id);
    packet.write_packed_guid(&value.guild_guid);
    packet.write_uint32(value.flags);
    packet.write_uint32(value.flags2);
    packet.write_uint32(value.flags3);
    packet.write_uint32(value.flags4);
    packet.write_uint8(value.cant_login_reason);
    packet.write_uint32(value.pet_creature_display_id);
    packet.write_uint32(value.pet_experience_level);
    packet.write_uint32(value.pet_creature_family_id);

    for item in value.visual_items {
        packet.write_uint32(item.item_id);
        packet.write_uint32(item.transmogrified_item_id);
        packet.write_uint8(item.subclass);
        packet.write_uint8(item.inv_type);
        packet.write_uint32(item.display_id);
        packet.write_uint32(item.display_enchant_id);
        packet.write_int32(item.secondary_item_modified_appearance_id);
        packet.write_uint8(item.sheathe_category);
    }

    packet.write_int32(value.save_version as i32);
    packet.write_int64(value.create_time);
    packet.write_int64(value.last_active_time);
    packet.write_int32(value.last_login_version as i32);
    packet.write_int32(value.personal_tabard.emblem_style);
    packet.write_int32(value.personal_tabard.emblem_color);
    packet.write_int32(value.personal_tabard.border_style);
    packet.write_int32(value.personal_tabard.border_color);
    packet.write_int32(value.personal_tabard.background_color);
    packet.write_uint32(value.profession_ids[0]);
    packet.write_uint32(value.profession_ids[1]);
    packet.write_int32(value.timerunning_season_id);
    packet.write_uint32(value.override_select_screen_file_data_id);
    packet.write_uint32(value.realm_queue);
    packet.write_int32(value.super_district_id);

    for customization in &value.customizations {
        packet.write_uint32(customization.option_id);
        packet.write_uint32(customization.choice_id);
    }

    write_name_length(packet, value.name(), "character_name")?;
    write_name_length(packet, value.surname(), "character_surname")?;
    packet.write_bit(value.first_login);
    packet.write_bit(value.realm_info_found);
    packet.write_bit(value.is_realm_offline);
    packet.flush_bits();
    packet.write_string(value.name());
    packet.write_string(value.surname());
    Ok(())
}

fn write_restrictions(
    packet: &mut WorldPacket,
    value: &CharacterRestrictionAndMailData,
) -> Result<(), CharacterListError> {
    if value.mail_senders.len() != value.mail_sender_types.len() {
        return Err(CharacterListError::MailCountMismatch);
    }
    bounded_count(
        value.mail_senders.len(),
        MAX_VECTOR_ELEMENTS,
        "mail_senders",
    )?;
    for sender in &value.mail_senders {
        validate_cstring(sender, "mail_sender")?;
    }

    packet.write_bit(value.boost_in_progress);
    packet.write_bit(value.rpe_available);
    packet.flush_bits();
    packet.write_uint32(value.restriction_flags);
    packet.write_uint32(count(value.mail_senders.len(), "mail_senders")?);
    packet.write_uint32(count(value.mail_sender_types.len(), "mail_sender_types")?);
    packet.write_uint32(value.no_rpe_reason);
    for sender_type in &value.mail_sender_types {
        packet.write_uint32(*sender_type);
    }
    for sender in &value.mail_senders {
        write_cstring_length(packet, sender, "mail_sender")?;
    }
    packet.flush_bits();
    for sender in &value.mail_senders {
        if !sender.is_empty() {
            packet.write_string(sender);
            packet.write_uint8(0);
        }
    }
    Ok(())
}

fn write_race_unlock(
    packet: &mut WorldPacket,
    value: &RaceUnlock,
) -> Result<(), CharacterListError> {
    bounded_count(
        value.class_unlocks.len(),
        MAX_VECTOR_ELEMENTS,
        "class_unlocks",
    )?;
    packet.write_int8(value.race_id);
    packet.write_uint32(count(value.class_unlocks.len(), "class_unlocks")?);
    for class in &value.class_unlocks {
        packet.write_int8(class.class_id);
        packet.write_uint32(class.achievement_id);
        packet.write_bit(class.has_expansion);
        packet.write_bit(class.has_unlocked_achievement);
        packet.write_bit(class.has_entitlement);
        packet.flush_bits();
    }
    packet.write_bit(value.has_unlocked_license);
    packet.write_bit(value.has_unlocked_achievement);
    packet.write_bit(value.has_heritage_armor_unlock_achievement);
    packet.write_bit(value.has_entitlement);
    packet.write_bit(value.hide_race_on_client);
    packet.write_bit(value.faction_balance_disabled);
    packet.write_bit(value.does_not_have_available_classes);
    packet.flush_bits();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn character(name: &str, surname: &str) -> CharacterInfo {
        let mut basic = CharacterInfoBasic::new(name, surname);
        basic.guid = ObjectGuid::create_player(1, 7);
        basic.virtual_realm_address = 0x0201_0001;
        basic.list_position = 2;
        basic.race_id = 1;
        basic.class_id = 1;
        basic.sex_id = 0;
        basic.experience_level = 1;
        basic.map_id = 0;
        basic.zone_id = 12;
        basic.preload_position = Position::xyz(1.0, 2.0, 3.0);
        basic.super_district_id = 1;
        basic.realm_info_found = true;
        CharacterInfo::new(basic, CharacterRestrictionAndMailData::empty())
    }

    #[test]
    fn local_enum_header_and_character_layout_are_source_ordered() {
        let mut result = EnumCharactersResult::new();
        result.success = true;
        result.max_character_level = 1;
        result.characters.push(character("Human", "Surname"));
        let payload = result.encode_payload().unwrap();

        assert_eq!(&payload[..2], &[0x80, 0]);
        assert_eq!(&payload[2..6], &[1, 0, 0, 0]); // local characters
        assert_eq!(&payload[6..10], &[0, 0, 0, 0]); // region-wide omitted
        assert_eq!(&payload[10..14], &[1, 0, 0, 0]); // max level

        // CharacterInfoBasic's name/surname precede the separate restriction
        // block; the packet therefore does not end with the surname.
        let restriction_start = payload.len() - 17; // 2 bits + four u32s
        assert_eq!(
            &payload[restriction_start - b"HumanSurname".len()..restriction_start],
            b"HumanSurname"
        );
        assert_eq!(
            &payload[restriction_start..],
            &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0]
        );
    }

    #[test]
    fn rejects_name_mail_and_character_count_bounds() {
        let mut result = EnumCharactersResult::new();
        result.characters.push(character(&"x".repeat(64), "S"));
        assert!(matches!(
            result.encode_payload(),
            Err(CharacterListError::Packet(
                ForeverPacketError::Length { .. }
            ))
        ));

        let mut mismatch = character("Human", "Surname");
        mismatch.restrictions_and_mails =
            CharacterRestrictionAndMailData::new(vec!["sender".to_owned()], Vec::new());
        result.characters.clear();
        result.characters.push(mismatch);
        assert!(matches!(
            result.encode_payload(),
            Err(CharacterListError::MailCountMismatch)
        ));

        result.characters = (0..=MAX_CHARACTERS_PER_REALM)
            .map(|index| character(&format!("C{index}"), "S"))
            .collect();
        assert!(matches!(
            result.encode_payload(),
            Err(CharacterListError::Packet(ForeverPacketError::Count { .. }))
        ));
    }

    #[test]
    fn rejects_non_finite_preload_position() {
        let mut value = character("Human", "Surname");
        value.basic.preload_position.x = f32::NAN;
        let mut result = EnumCharactersResult::new();
        result.characters.push(value);
        assert!(matches!(
            result.encode_payload(),
            Err(CharacterListError::NonFinitePosition)
        ));
    }

    #[test]
    fn orientation_is_not_part_of_tagged_position_wire_data() {
        let mut value = character("Human", "Surname");
        value.basic.preload_position.orientation = f32::NAN;
        let mut result = EnumCharactersResult::new();
        result.characters.push(value);

        assert!(result.encode_payload().is_ok());
    }

    #[test]
    fn populated_character_has_independent_cursor_golden() {
        let mut value = character("Hero", "Wolf");
        value.basic.guid = ObjectGuid::new(0x0102_0304_0506_0708, 0x1112_1314_1516_1718);
        value.basic.guild_guid = ObjectGuid::new(0x0100_0000_0000_0000, 0x11);
        value.basic.virtual_realm_address = 0xA1B2_C3D4;
        value.basic.list_position = 0x1234;
        value.basic.race_id = 7;
        value.basic.class_id = 4;
        value.basic.sex_id = 2;
        value.basic.spec_id = 0x1357;
        value.basic.customizations = vec![
            CustomizationChoice {
                option_id: 0x1011_1213,
                choice_id: 0x2122_2324,
            },
            CustomizationChoice {
                option_id: 0x3132_3334,
                choice_id: 0x4142_4344,
            },
        ];
        value.basic.experience_level = 8;
        value.basic.map_id = -10;
        value.basic.zone_id = 0x1234_5678;
        value.basic.preload_position = Position::new(1.25, -2.5, 3.75, f32::NAN);
        value.basic.guild_club_member_id = 0x0102_0304_0506_0708;
        value.basic.flags = 0x0102_0304;
        value.basic.flags2 = 0x1112_1314;
        value.basic.flags3 = 0x2122_2324;
        value.basic.flags4 = 0x3132_3334;
        value.basic.first_login = true;
        value.basic.cant_login_reason = 9;
        value.basic.pet_creature_display_id = 0xAABB_CCDD;
        value.basic.pet_experience_level = 0x0102_0304;
        value.basic.pet_creature_family_id = 0x1122_3344;
        for (index, item) in value.basic.visual_items.iter_mut().enumerate() {
            let index = index as u32;
            item.item_id = 0x1000_0000 + index;
            item.transmogrified_item_id = 0x2000_0000 + index;
            item.subclass = (index + 1) as u8;
            item.inv_type = (0x80 + index) as u8;
            item.display_id = 0x3000_0000 + index;
            item.display_enchant_id = 0x4000_0000 + index;
            item.secondary_item_modified_appearance_id = -100 - index as i32;
            item.sheathe_category = (0x40 + index) as u8;
        }
        value.basic.save_version = 0x5566_7788;
        value.basic.create_time = 0x0102_0304_0506_0708;
        value.basic.last_active_time = 0x1112_1314_1516_1718;
        value.basic.last_login_version = 0x2233_4455;
        value.basic.personal_tabard = CustomTabardInfo {
            emblem_style: -1,
            emblem_color: 2,
            border_style: -3,
            border_color: 4,
            background_color: -5,
        };
        value.basic.profession_ids = [0xDEAD_BEEF, 0xFEED_FACE];
        value.basic.timerunning_season_id = -6;
        value.basic.override_select_screen_file_data_id = 0x8765_4321;
        value.basic.realm_queue = 0x1020_3040;
        value.basic.super_district_id = -7;
        value.restrictions_and_mails = CharacterRestrictionAndMailData::new(
            vec!["Aé".to_owned(), String::new()],
            vec![0x1122_3344, 0x5566_7788],
        );
        value.restrictions_and_mails.boost_in_progress = true;
        value.restrictions_and_mails.restriction_flags = 0xABCD_EF01;
        value.restrictions_and_mails.rpe_available = true;
        value.restrictions_and_mails.no_rpe_reason = 0x1234_5678;

        let mut result = EnumCharactersResult::new();
        result.success = true;
        result.realmless = true;
        result.is_deleted_characters = false;
        result.ignore_new_player_restrictions = true;
        result.is_restricted_new_player = false;
        result.is_newcomer_chat_completed = true;
        result.is_restricted_trial = false;
        result.is_account_lapsed_player = true;
        result.force_character_list_sort = true;
        result.max_character_level = 60;
        result.class_disable_mask = Some(0xCAFE_BABE);
        result.characters.push(value);

        let payload = result.encode_payload().unwrap();
        assert_eq!(&payload[..2], &[0xD5, 0xC0]);
        assert_eq!(&payload[2..6], &[1, 0, 0, 0]);
        assert_eq!(&payload[10..14], &[60, 0, 0, 0]);
        assert_eq!(&payload[30..34], &0xCAFE_BABEu32.to_le_bytes());

        let mut cursor = 34;
        assert_eq!(
            &payload[cursor..cursor + 18],
            &[
                0xFF, 0xFF, 0x18, 0x17, 0x16, 0x15, 0x14, 0x13, 0x12, 0x11, 0x08, 0x07, 0x06, 0x05,
                0x04, 0x03, 0x02, 0x01,
            ]
        );
        cursor += 18;
        assert_eq!(&payload[cursor..cursor + 4], &0xA1B2_C3D4u32.to_le_bytes());
        cursor += 4;
        assert_eq!(&payload[cursor..cursor + 2], &0x1234u16.to_le_bytes());
        cursor += 2;
        assert_eq!(&payload[cursor..cursor + 3], &[7, 2, 4]);
        cursor += 3;
        assert_eq!(&payload[cursor..cursor + 2], &0x1357u16.to_le_bytes());
        cursor += 2;
        assert_eq!(&payload[cursor..cursor + 4], &[2, 0, 0, 0]);
        cursor += 4;
        assert_eq!(&payload[cursor], &8);
        cursor += 1;
        assert_eq!(&payload[cursor..cursor + 4], &(-10i32).to_le_bytes());
        cursor += 4;
        assert_eq!(&payload[cursor..cursor + 4], &0x1234_5678u32.to_le_bytes());
        cursor += 4;
        for expected in [1.25f32, -2.5, 3.75] {
            assert_eq!(&payload[cursor..cursor + 4], &expected.to_le_bytes());
            cursor += 4;
        }
        assert_eq!(
            &payload[cursor..cursor + 8],
            &0x0102_0304_0506_0708u64.to_le_bytes()
        );
        cursor += 8;
        assert_eq!(&payload[cursor..cursor + 4], &[1, 0x80, 0x11, 1]);
        cursor += 4;
        for expected in [0x0102_0304u32, 0x1112_1314, 0x2122_2324, 0x3132_3334] {
            assert_eq!(&payload[cursor..cursor + 4], &expected.to_le_bytes());
            cursor += 4;
        }
        assert_eq!(&payload[cursor], &9);
        cursor += 1;
        for expected in [0xAABB_CCDDu32, 0x0102_0304, 0x1122_3344] {
            assert_eq!(&payload[cursor..cursor + 4], &expected.to_le_bytes());
            cursor += 4;
        }

        // Each source VisualItemInfo is 23 bytes; checking every item keeps
        // the 19-element order independent from the encoder implementation.
        for index in 0..VISUAL_ITEM_COUNT {
            let index = index as u32;
            assert_eq!(
                &payload[cursor..cursor + 4],
                &(0x1000_0000 + index).to_le_bytes()
            );
            assert_eq!(
                &payload[cursor + 4..cursor + 8],
                &(0x2000_0000 + index).to_le_bytes()
            );
            assert_eq!(
                &payload[cursor + 8..cursor + 10],
                &[(index + 1) as u8, (0x80 + index) as u8]
            );
            assert_eq!(
                &payload[cursor + 10..cursor + 14],
                &(0x3000_0000 + index).to_le_bytes()
            );
            assert_eq!(
                &payload[cursor + 14..cursor + 18],
                &(0x4000_0000 + index).to_le_bytes()
            );
            assert_eq!(
                &payload[cursor + 18..cursor + 22],
                &(-100i32 - index as i32).to_le_bytes()
            );
            assert_eq!(&payload[cursor + 22], &((0x40 + index) as u8));
            cursor += 23;
        }

        for expected in [0x5566_7788u32] {
            assert_eq!(&payload[cursor..cursor + 4], &expected.to_le_bytes());
            cursor += 4;
        }
        for expected in [0x0102_0304_0506_0708u64, 0x1112_1314_1516_1718u64] {
            assert_eq!(&payload[cursor..cursor + 8], &expected.to_le_bytes());
            cursor += 8;
        }
        assert_eq!(&payload[cursor..cursor + 4], &0x2233_4455u32.to_le_bytes());
        cursor += 4;
        for expected in [-1i32, 2, -3, 4, -5] {
            assert_eq!(&payload[cursor..cursor + 4], &expected.to_le_bytes());
            cursor += 4;
        }
        for expected in [0xDEAD_BEEFu32, 0xFEED_FACE] {
            assert_eq!(&payload[cursor..cursor + 4], &expected.to_le_bytes());
            cursor += 4;
        }
        assert_eq!(&payload[cursor..cursor + 4], &(-6i32).to_le_bytes());
        cursor += 4;
        for expected in [0x8765_4321u32, 0x1020_3040u32] {
            assert_eq!(&payload[cursor..cursor + 4], &expected.to_le_bytes());
            cursor += 4;
        }
        assert_eq!(&payload[cursor..cursor + 4], &(-7i32).to_le_bytes());
        cursor += 4;
        assert_eq!(
            &payload[cursor..cursor + 16],
            &[
                0x13, 0x12, 0x11, 0x10, 0x24, 0x23, 0x22, 0x21, 0x34, 0x33, 0x32, 0x31, 0x44, 0x43,
                0x42, 0x41,
            ]
        );
        cursor += 16;
        assert_eq!(&payload[cursor..cursor + 2], &[0x10, 0x4C]);
        cursor += 2;
        assert_eq!(&payload[cursor..cursor + 8], b"HeroWolf");
        cursor += 8;

        assert_eq!(&payload[cursor..cursor + 1], &[0xC0]);
        cursor += 1;
        for expected in [0xABCD_EF01u32, 2, 2, 0x1234_5678, 0x1122_3344, 0x5566_7788] {
            assert_eq!(&payload[cursor..cursor + 4], &expected.to_le_bytes());
            cursor += 4;
        }
        assert_eq!(&payload[cursor..cursor + 2], &[0x10, 0x10]);
        cursor += 2;
        assert_eq!(&payload[cursor..], &[b'A', 0xC3, 0xA9, 0]);
        cursor += 4;
        assert_eq!(cursor, payload.len());
    }

    #[test]
    fn sized_cstring_mail_uses_nul_in_length_and_data() {
        let mut value = character("Human", "Surname");
        value.restrictions_and_mails =
            CharacterRestrictionAndMailData::new(vec!["".to_owned(), "é".to_owned()], vec![7, 8]);
        let mut result = EnumCharactersResult::new();
        result.characters.push(value);
        let payload = result.encode_payload().unwrap();

        // The last block is: bit flags, four u32s, two types, two six-bit
        // SizedCString lengths (1 and 3), then only the non-empty string+NUL.
        let suffix = [
            0, 0, 0, 0, 0, 2, 0, 0, 0, 2, 0, 0, 0, 4, 0, 0, 0, 7, 0, 0, 0, 8, 0, 0, 0, 0x04, 0x30,
            0xC3, 0xA9, 0,
        ];
        assert!(payload.ends_with(&suffix));
    }

    #[test]
    fn sized_cstring_mail_accepts_62_bytes_but_not_63() {
        let mut accepted = character("Human", "Surname");
        accepted.restrictions_and_mails =
            CharacterRestrictionAndMailData::new(vec!["é".repeat(31)], vec![1]);
        let mut result = EnumCharactersResult::new();
        result.characters.push(accepted);
        assert!(result.encode_payload().is_ok());

        let mut rejected = character("Human", "Surname");
        rejected.restrictions_and_mails =
            CharacterRestrictionAndMailData::new(vec!["x".repeat(63)], vec![1]);
        let mut result = EnumCharactersResult::new();
        result.characters.push(rejected);
        assert!(matches!(
            result.encode_payload(),
            Err(CharacterListError::Packet(ForeverPacketError::Length {
                field: "mail_sender",
                actual: 63,
                maximum: 62,
            }))
        ));
    }
}
