//! Payload-only world packets for the Forever Classic client.
//!
//! These codecs intentionally do not implement [`crate::ServerPacket`].  That
//! trait prepends the legacy two-byte opcode; Forever's world transport owns
//! its 32-bit opcode/framing.  The constants below are the core opcode values
//! from `src/server/game/Server/Protocol/Opcodes.h` at
//! `02245dcd245e7433e524577656177723d3e4992e`.  A transport may translate
//! them through that fork's Classic opcode table before framing.
//!
//! `AuthenticationPackets.cpp:29-217`, `CharacterPackets.cpp:331-461`,
//! `ClientConfigPackets.cpp:23-37`, `HotfixPackets.cpp:79-89`,
//! `SystemPackets.cpp:247-279`, `MiscPackets.cpp:208-212`, and
//! `WorldSession.cpp:1431-1476` are the source anchors for the layouts.  The
//! glue-screen layout is explicitly annotated by that source as Classic
//! 1.60.1.70009; this module does not claim an unverified 70170 difference is
//! absent.

use std::convert::TryFrom;

use thiserror::Error;
use wow_core::ObjectGuid;

use crate::WorldPacket;

pub mod db_query;
pub mod hotfix;

/// Core opcode values (u32 metadata; no legacy u16 opcode is written here).
pub const AUTH_RESPONSE_OPCODE: u32 = 0x450001;
pub const AVAILABLE_HOTFIXES_OPCODE: u32 = 0x490001;
pub const BATTLE_NET_CONNECTION_STATUS_OPCODE: u32 = 0x4502B1;
pub const ACCOUNT_DATA_TIMES_OPCODE: u32 = 0x4501B6;
pub const CACHE_VERSION_OPCODE: u32 = 0x49000E;
pub const ENUM_CHARACTERS_RESULT_OPCODE: u32 = 0x450018;
pub const FEATURE_SYSTEM_STATUS_GLUE_SCREEN_OPCODE: u32 = 0x450064;
pub const RECENT_ALLY_DATA_RESPONSE_OPCODE: u32 = 0x450362;
pub const SET_TIME_ZONE_INFORMATION_OPCODE: u32 = 0x450123;
pub const TUTORIAL_FLAGS_OPCODE: u32 = 0x450268;

/// The Classic opcode-table override recorded in `ClassicOpcodes.cpp:45`.
pub const CLASSIC_70009_ACCOUNT_DATA_TIMES_OPCODE: u32 = 0x4601B5;

/// Errors caused by a payload that cannot be represented by the source layout.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ForeverPacketError {
    #[error("{field} has {actual} bytes; maximum is {maximum}")]
    Length {
        field: &'static str,
        actual: usize,
        maximum: usize,
    },
    #[error("{field} has {actual} elements; maximum is {maximum}")]
    Count {
        field: &'static str,
        actual: usize,
        maximum: usize,
    },
    #[error("{field} value {value} does not fit in {bits} bits")]
    Bits {
        field: &'static str,
        value: u64,
        bits: u8,
    },
}

fn count(value: usize, field: &'static str) -> Result<u32, ForeverPacketError> {
    u32::try_from(value).map_err(|_| ForeverPacketError::Count {
        field,
        actual: value,
        maximum: u32::MAX as usize,
    })
}

fn length(value: &[u8], maximum: usize, field: &'static str) -> Result<(), ForeverPacketError> {
    if value.len() > maximum {
        return Err(ForeverPacketError::Length {
            field,
            actual: value.len(),
            maximum,
        });
    }
    Ok(())
}

fn finish(mut packet: WorldPacket) -> Vec<u8> {
    packet.flush_bits();
    packet.into_data()
}

fn write_bounded_string(
    packet: &mut WorldPacket,
    value: &str,
    bits: u8,
    field: &'static str,
) -> Result<(), ForeverPacketError> {
    let bytes = value.as_bytes();
    let maximum = ((1u16 << bits) - 1) as usize;
    length(bytes, maximum, field)?;
    packet.write_bits(bytes.len() as u32, u32::from(bits));
    Ok(())
}

/// C++ `WorldPackets::Auth::ClassAvailability`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClassAvailability {
    pub class_id: u8,
    pub active_expansion_level: u8,
    pub account_expansion_level: u8,
    pub min_active_expansion_level: u8,
}

/// C++ `RaceClassAvailability` row supplied by the account/race catalog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RaceClassAvailability {
    pub race_id: u8,
    pub classes: Vec<ClassAvailability>,
}

/// C++ `WorldPackets::Auth::VirtualRealmNameInfo` and `VirtualRealmInfo`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VirtualRealmInfo {
    pub realm_address: u32,
    pub is_local: bool,
    pub is_internal_realm: bool,
    pub realm_name_actual: String,
    pub realm_name_normalized: String,
}

/// C++ `CharacterTemplateClass`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CharacterTemplateClass {
    pub class_id: u8,
    pub faction_group: u8,
}

/// C++ `CharacterTemplate` projection used by `AuthSuccessInfo`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterTemplate {
    pub template_set_id: u32,
    pub classes: Vec<CharacterTemplateClass>,
    pub name: String,
    pub description: String,
}

/// C++ `WorldPackets::Auth::GameTime`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameTime {
    pub billing_type: u32,
    pub minutes_remaining: u32,
    pub real_billing_type: u32,
    pub is_in_igr: bool,
    pub is_paid_for_by_igr: bool,
    pub is_cais_enabled: bool,
}

/// C++ `WorldPackets::Auth::BaseBuildKey`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct BaseBuildKey {
    pub build_key: [u8; 16],
    pub config_key: [u8; 16],
}

/// Success payload fields from `AuthenticationPackets.h:168-192`.
#[derive(Clone, PartialEq, Eq)]
pub struct AuthSuccessInfo {
    pub active_expansion_level: u8,
    pub account_expansion_level: u8,
    pub time_rested: u32,
    pub virtual_realm_address: u32,
    pub time_seconds_until_pc_kick: u32,
    pub currency_id: u32,
    pub time: i64,
    pub game_time: GameTime,
    pub virtual_realms: Vec<VirtualRealmInfo>,
    pub available_classes: Vec<RaceClassAvailability>,
    pub templates: Vec<CharacterTemplate>,
    pub is_expansion_trial: bool,
    pub force_character_template: bool,
    pub num_players_horde: Option<u16>,
    pub num_players_alliance: Option<u16>,
    pub expansion_trial_expiration: Option<i64>,
    pub current_build: Option<BaseBuildKey>,
}

/// C++ `WorldPackets::Auth::AuthWaitInfo`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuthWaitInfo {
    pub wait_count: u32,
    pub wait_time: u32,
    pub allowed_faction_group_for_character_create: u8,
    pub has_fcm: bool,
    pub can_create_only_if_existing: bool,
}

/// C++ `WorldPackets::Auth::AuthResponse`, without its opcode.
#[derive(Clone, PartialEq, Eq)]
pub struct AuthResponsePayload {
    pub result: u32,
    pub success_info: Option<AuthSuccessInfo>,
    pub wait_info: Option<AuthWaitInfo>,
}

impl AuthResponsePayload {
    pub fn denied(result: u32) -> Self {
        Self {
            result,
            success_info: None,
            wait_info: None,
        }
    }

    pub fn encode_payload(&self) -> Result<Vec<u8>, ForeverPacketError> {
        let mut packet = WorldPacket::new_empty();
        packet.write_uint32(self.result);
        packet.write_bit(self.success_info.is_some());
        packet.write_bit(self.wait_info.is_some());
        packet.flush_bits();

        if let Some(success) = &self.success_info {
            write_success_info(&mut packet, success)?;
        }
        if let Some(wait) = self.wait_info {
            packet.write_uint32(wait.wait_count);
            packet.write_uint32(wait.wait_time);
            packet.write_uint8(wait.allowed_faction_group_for_character_create);
            packet.write_bit(wait.has_fcm);
            packet.write_bit(wait.can_create_only_if_existing);
            packet.flush_bits();
        }

        Ok(finish(packet))
    }
}

fn write_success_info(
    packet: &mut WorldPacket,
    value: &AuthSuccessInfo,
) -> Result<(), ForeverPacketError> {
    packet.write_uint32(value.virtual_realm_address);
    packet.write_uint32(count(value.virtual_realms.len(), "virtual_realms")?);
    packet.write_uint32(value.time_rested);
    packet.write_uint8(value.active_expansion_level);
    packet.write_uint8(value.account_expansion_level);
    packet.write_uint32(value.time_seconds_until_pc_kick);
    packet.write_uint32(count(value.available_classes.len(), "available_classes")?);
    packet.write_uint32(count(value.templates.len(), "templates")?);
    packet.write_uint32(value.currency_id);
    write_game_time(packet, value.game_time);
    packet.write_int64(value.time);

    for realm in &value.virtual_realms {
        packet.write_uint32(realm.realm_address);
        packet.write_bit(realm.is_local);
        packet.write_bit(realm.is_internal_realm);
        length(
            realm.realm_name_actual.as_bytes(),
            u8::MAX as usize,
            "realm_name_actual",
        )?;
        length(
            realm.realm_name_normalized.as_bytes(),
            u8::MAX as usize,
            "realm_name_normalized",
        )?;
        packet.write_bits(realm.realm_name_actual.len() as u32, 8);
        packet.write_bits(realm.realm_name_normalized.len() as u32, 8);
        packet.flush_bits();
        packet.write_string(&realm.realm_name_actual);
        packet.write_string(&realm.realm_name_normalized);
    }

    for race in &value.available_classes {
        packet.write_uint8(race.race_id);
        packet.write_uint32(count(race.classes.len(), "class_availability")?);
        for class in &race.classes {
            packet.write_uint8(class.class_id);
            packet.write_uint8(class.active_expansion_level);
            packet.write_uint8(class.account_expansion_level);
            packet.write_uint8(class.min_active_expansion_level);
        }
    }

    for template in &value.templates {
        packet.write_uint32(template.template_set_id);
        packet.write_uint32(count(template.classes.len(), "template_classes")?);
        for class in &template.classes {
            packet.write_uint8(class.class_id);
            packet.write_uint8(class.faction_group);
        }
        write_bounded_string(packet, &template.name, 7, "template_name")?;
        write_bounded_string(packet, &template.description, 10, "template_description")?;
        packet.flush_bits();
        packet.write_string(&template.name);
        packet.write_string(&template.description);
    }

    packet.write_bit(value.is_expansion_trial);
    packet.write_bit(value.force_character_template);
    packet.write_bit(value.num_players_horde.is_some());
    packet.write_bit(value.num_players_alliance.is_some());
    packet.write_bit(value.expansion_trial_expiration.is_some());
    packet.write_bit(value.current_build.is_some());
    packet.flush_bits();

    if let Some(players) = value.num_players_horde {
        packet.write_uint16(players);
    }
    if let Some(players) = value.num_players_alliance {
        packet.write_uint16(players);
    }
    if let Some(expiration) = value.expansion_trial_expiration {
        packet.write_int64(expiration);
    }
    if let Some(build) = value.current_build {
        for index in 0..16 {
            packet.write_uint8(build.build_key[index]);
            packet.write_uint8(build.config_key[index]);
        }
    }
    Ok(())
}

fn write_game_time(packet: &mut WorldPacket, value: GameTime) {
    packet.write_uint32(value.billing_type);
    packet.write_uint32(value.minutes_remaining);
    packet.write_uint32(value.real_billing_type);
    packet.write_bit(value.is_in_igr);
    packet.write_bit(value.is_paid_for_by_igr);
    packet.write_bit(value.is_cais_enabled);
    packet.flush_bits();
}

/// Payload-only Classic 1.60.1.70009 glue-screen layout from
/// `SystemPackets.cpp:247-279`.  The source does not prove that every field is
/// unchanged in 70170; the explicit suffix keeps that uncertainty visible.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassicGlueScreen70009 {
    pub commerce_price_poll_time_seconds: u32,
    pub redeem_for_balance_amount: i64,
    pub max_characters_on_realm: i32,
    pub active_boost_type: i32,
    pub trial_boost_type: i32,
    pub minimum_expansion_level: i32,
    pub maximum_expansion_level: i32,
    pub content_set_id: i32,
    /// `SystemPackets.h:265` declares `vector<int32>`.  The Classic writer
    /// at this SHA emits a zero count, while `AuthHandler.cpp:143-145`
    /// populates the source vector with mode 8; keep the typed field so the
    /// 70170 integration can make that unresolved choice explicitly.
    pub available_game_mode_ids: Vec<i32>,
    pub active_timerunning_season_id: i32,
    pub remaining_timerunning_season_seconds: i32,
    pub timerunning_conversion_min_character_age: i32,
    pub timerunning_conversion_max_season_id: i32,
    pub max_player_guid_lookups_per_request: i16,
    pub name_lookup_telemetry_interval: i16,
    pub not_found_cache_time_seconds: u32,
    pub most_recent_time_event_id: i32,
    pub event_realm_queues: u32,
}

impl ClassicGlueScreen70009 {
    pub fn encode_payload(&self) -> Result<Vec<u8>, ForeverPacketError> {
        let mut packet = WorldPacket::new_empty();
        packet.write_bytes(&[0; 6]);
        packet.write_uint32(self.commerce_price_poll_time_seconds);
        packet.write_int64(self.redeem_for_balance_amount);
        packet.write_int32(self.max_characters_on_realm);
        packet.write_uint32(0); // LiveRegionCharacterCopySourceRegions
        packet.write_int32(self.active_boost_type);
        packet.write_int32(self.trial_boost_type);
        packet.write_int32(self.minimum_expansion_level);
        packet.write_int32(self.maximum_expansion_level);
        packet.write_int32(self.content_set_id);
        packet.write_uint32(0); // DisabledGameModes
        packet.write_uint32(0); // GameRules
        packet.write_uint32(count(
            self.available_game_mode_ids.len(),
            "available_game_mode_ids",
        )?);
        for game_mode_id in &self.available_game_mode_ids {
            packet.write_int32(*game_mode_id);
        }
        packet.write_int32(self.active_timerunning_season_id);
        packet.write_int32(self.remaining_timerunning_season_seconds);
        packet.write_int32(self.timerunning_conversion_min_character_age);
        packet.write_int32(self.timerunning_conversion_max_season_id);
        packet.write_int16(self.max_player_guid_lookups_per_request);
        packet.write_int16(self.name_lookup_telemetry_interval);
        packet.write_uint32(self.not_found_cache_time_seconds);
        packet.write_uint32(0); // DebugTimeEvents
        packet.write_int32(self.most_recent_time_event_id);
        packet.write_uint32(self.event_realm_queues);
        Ok(finish(packet))
    }
}

/// Three 7-bit sized strings from `SystemPackets.cpp:415-426`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetTimeZoneInformation {
    pub server_time_tz: String,
    pub game_time_tz: String,
    pub server_regional_time_tz: String,
}

impl SetTimeZoneInformation {
    pub fn encode_payload(&self) -> Result<Vec<u8>, ForeverPacketError> {
        let mut packet = WorldPacket::new_empty();
        write_bounded_string(&mut packet, &self.server_time_tz, 7, "server_time_tz")?;
        write_bounded_string(&mut packet, &self.game_time_tz, 7, "game_time_tz")?;
        write_bounded_string(
            &mut packet,
            &self.server_regional_time_tz,
            7,
            "server_regional_time_tz",
        )?;
        packet.flush_bits();
        packet.write_string(&self.server_time_tz);
        packet.write_string(&self.game_time_tz);
        packet.write_string(&self.server_regional_time_tz);
        Ok(finish(packet))
    }
}

/// C++ `ClientConfig::ClientCacheVersion`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClientCacheVersion {
    pub cache_version: u32,
}

impl ClientCacheVersion {
    pub fn encode_payload(self) -> Vec<u8> {
        let mut packet = WorldPacket::new_empty();
        packet.write_uint32(self.cache_version);
        finish(packet)
    }
}

/// C++ `DB2Manager::HotfixId`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HotfixId {
    pub push_id: i32,
    pub unique_id: u32,
}

/// C++ `Hotfix::AvailableHotfixes`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AvailableHotfixes {
    pub virtual_realm_address: i32,
    pub hotfixes: Vec<HotfixId>,
}

impl AvailableHotfixes {
    pub fn encode_payload(&self) -> Result<Vec<u8>, ForeverPacketError> {
        let mut packet = WorldPacket::new_empty();
        packet.write_int32(self.virtual_realm_address);
        packet.write_uint32(count(self.hotfixes.len(), "hotfixes")?);
        for hotfix in &self.hotfixes {
            packet.write_int32(hotfix.push_id);
            packet.write_uint32(hotfix.unique_id);
        }
        Ok(finish(packet))
    }
}

/// `NUM_ACCOUNT_DATA_TYPES` is 20 at the pinned source's WorldSession.h:888.
pub const NUM_ACCOUNT_DATA_TYPES: usize = 20;

/// C++ `ClientConfig::AccountDataTimes`.  `ObjectGuid` is the normal packed
/// GUID encoding; an empty GUID is therefore two mask bytes, not sixteen zeroes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountDataTimes {
    pub player_guid: ObjectGuid,
    pub server_time: i64,
    pub account_times: [i64; NUM_ACCOUNT_DATA_TYPES],
}

impl AccountDataTimes {
    pub fn encode_payload(&self) -> Vec<u8> {
        let mut packet = WorldPacket::new_empty();
        packet.write_packed_guid(&self.player_guid);
        packet.write_int64(self.server_time);
        for timestamp in self.account_times {
            packet.write_int64(timestamp);
        }
        finish(packet)
    }
}

/// C++ `Misc::TutorialFlags`; `MAX_ACCOUNT_TUTORIAL_VALUES` is eight.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TutorialFlags {
    pub values: [u32; 8],
}

impl TutorialFlags {
    pub fn encode_payload(self) -> Vec<u8> {
        let mut packet = WorldPacket::new_empty();
        for value in self.values {
            packet.write_uint32(value);
        }
        finish(packet)
    }
}

/// C++ `Battlenet::ConnectionStatus`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConnectionStatus {
    pub state: u8,
    pub suppress_notification: bool,
}

impl ConnectionStatus {
    pub fn encode_payload(self) -> Result<Vec<u8>, ForeverPacketError> {
        if self.state > 3 {
            return Err(ForeverPacketError::Bits {
                field: "connection_status.state",
                value: self.state as u64,
                bits: 2,
            });
        }
        let mut packet = WorldPacket::new_empty();
        packet.write_bits(u32::from(self.state), 2);
        packet.write_bit(self.suppress_notification);
        Ok(finish(packet))
    }
}

/// C++ `EnumCharactersResult::ClassUnlock`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClassUnlock {
    pub class_id: i8,
    pub achievement_id: u32,
    pub has_expansion: bool,
    pub has_unlocked_achievement: bool,
    pub has_entitlement: bool,
}

/// C++ `EnumCharactersResult::RaceUnlock`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RaceUnlock {
    pub race_id: i8,
    pub has_unlocked_license: bool,
    pub has_unlocked_achievement: bool,
    pub has_heritage_armor_unlock_achievement: bool,
    pub has_entitlement: bool,
    pub hide_race_on_client: bool,
    pub faction_balance_disabled: bool,
    pub does_not_have_available_classes: bool,
    pub class_unlocks: Vec<ClassUnlock>,
}

/// Empty-character projection of `EnumCharactersResult`.
///
/// The pinned handler sends this packet after account queries, with character,
/// region-wide character, conditional-appearance, race-limit and warband rows
/// supplied by the session.  This bounded codec intentionally supports only
/// the empty-character path; it does not fabricate character records.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmptyEnumCharactersResult {
    pub success: bool,
    pub realmless: bool,
    pub is_deleted_characters: bool,
    pub ignore_new_player_restrictions: bool,
    pub is_restricted_new_player: bool,
    pub is_newcomer_chat_completed: bool,
    pub is_restricted_trial: bool,
    pub is_account_lapsed_player: bool,
    pub force_character_list_sort: bool,
    pub class_disable_mask: Option<u32>,
    pub max_character_level: i32,
    pub race_unlock_data: Vec<RaceUnlock>,
}

impl EmptyEnumCharactersResult {
    pub fn encode_payload(&self) -> Result<Vec<u8>, ForeverPacketError> {
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
        packet.write_uint32(0); // Characters
        packet.write_uint32(0); // RegionwideCharacters
        packet.write_int32(self.max_character_level);
        packet.write_uint32(count(self.race_unlock_data.len(), "race_unlock_data")?);
        packet.write_uint32(0); // UnlockedConditionalAppearances
        packet.write_uint32(0); // RaceLimitDisables
        packet.write_uint32(0); // WarbandGroups

        if let Some(mask) = self.class_disable_mask {
            packet.write_uint32(mask);
        }
        for race in &self.race_unlock_data {
            packet.write_int8(race.race_id);
            packet.write_uint32(count(race.class_unlocks.len(), "class_unlocks")?);
            for class in &race.class_unlocks {
                packet.write_int8(class.class_id);
                packet.write_uint32(class.achievement_id);
                packet.write_bit(class.has_expansion);
                packet.write_bit(class.has_unlocked_achievement);
                packet.write_bit(class.has_entitlement);
                packet.flush_bits();
            }
            packet.write_bit(race.has_unlocked_license);
            packet.write_bit(race.has_unlocked_achievement);
            packet.write_bit(race.has_heritage_armor_unlock_achievement);
            packet.write_bit(race.has_entitlement);
            packet.write_bit(race.hide_race_on_client);
            packet.write_bit(race.faction_balance_disabled);
            packet.write_bit(race.does_not_have_available_classes);
            packet.flush_bits();
        }
        Ok(finish(packet))
    }
}

/// Empty release packet sent by the pinned Classic handler after enum.  Its
/// handler/layout is annotated as 1.60.1.70009 in `CharacterHandler.cpp:508-517`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EmptyRecentAllyDataResponse70009 {
    pub leading_value: u32,
    pub mode: u8,
    pub entry_count: u32,
}

impl EmptyRecentAllyDataResponse70009 {
    pub fn encode_payload(self) -> Vec<u8> {
        let mut packet = WorldPacket::new_empty();
        packet.write_uint32(self.leading_value);
        packet.write_uint8(self.mode);
        packet.write_uint32(self.entry_count);
        finish(packet)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_success() -> AuthSuccessInfo {
        AuthSuccessInfo {
            active_expansion_level: 0,
            account_expansion_level: 0,
            time_rested: 0,
            virtual_realm_address: 0,
            time_seconds_until_pc_kick: 0,
            currency_id: 0,
            time: 0,
            game_time: GameTime {
                billing_type: 0,
                minutes_remaining: 0,
                real_billing_type: 0,
                is_in_igr: false,
                is_paid_for_by_igr: false,
                is_cais_enabled: false,
            },
            virtual_realms: Vec::new(),
            available_classes: Vec::new(),
            templates: Vec::new(),
            is_expansion_trial: false,
            force_character_template: false,
            num_players_horde: None,
            num_players_alliance: None,
            expansion_trial_expiration: None,
            current_build: None,
        }
    }

    #[test]
    fn denied_auth_is_result_and_two_presence_bits() {
        assert_eq!(
            AuthResponsePayload::denied(0x12_34_56_78)
                .encode_payload()
                .unwrap(),
            vec![0x78, 0x56, 0x34, 0x12, 0]
        );
    }

    #[test]
    fn auth_success_uses_source_order_and_empty_arrays() {
        let payload = AuthResponsePayload {
            result: 0,
            success_info: Some(empty_success()),
            wait_info: None,
        }
        .encode_payload()
        .unwrap();
        // ByteBuffer writes bits MSB-first; source/native AuthResponse uses
        // bit 7 for SuccessInfo, not the least significant bit.
        assert_eq!(&payload[..5], &[0, 0, 0, 0, 0x80]);
        assert_eq!(&payload[5..9], &[0, 0, 0, 0]); // realm address
        assert_eq!(payload.len(), 57);
        // GameTime is before Time (three u32 + one bit byte, then i64).
        assert_eq!(&payload[35..47], &[0; 12]);
    }

    #[test]
    fn auth_rejects_oversized_realm_name() {
        let mut info = empty_success();
        info.virtual_realms.push(VirtualRealmInfo {
            realm_address: 1,
            is_local: true,
            is_internal_realm: false,
            realm_name_actual: "x".repeat(256),
            realm_name_normalized: String::new(),
        });
        let response = AuthResponsePayload {
            result: 0,
            success_info: Some(info),
            wait_info: None,
        };
        assert!(matches!(
            response.encode_payload(),
            Err(ForeverPacketError::Length { .. })
        ));
    }

    #[test]
    fn timezone_has_three_lengths_then_data() {
        let value = SetTimeZoneInformation {
            server_time_tz: "A".into(),
            game_time_tz: "BC".into(),
            server_regional_time_tz: "D".into(),
        };
        assert_eq!(
            value.encode_payload().unwrap(),
            vec![2, 8, 8, b'A', b'B', b'C', b'D']
        );
    }

    #[test]
    fn account_data_uses_twenty_timestamps() {
        let packet = AccountDataTimes {
            player_guid: ObjectGuid::EMPTY,
            server_time: 1,
            account_times: [2; NUM_ACCOUNT_DATA_TYPES],
        }
        .encode_payload();
        assert_eq!(packet.len(), 2 + 8 + 8 * NUM_ACCOUNT_DATA_TYPES);
    }

    #[test]
    fn connection_status_rejects_three_bit_state() {
        assert!(matches!(
            (ConnectionStatus {
                state: 4,
                suppress_notification: false
            })
            .encode_payload(),
            Err(ForeverPacketError::Bits { .. })
        ));
    }

    #[test]
    fn empty_enum_header_and_race_unlock_are_ordered() {
        let value = EmptyEnumCharactersResult {
            success: true,
            realmless: false,
            is_deleted_characters: false,
            ignore_new_player_restrictions: false,
            is_restricted_new_player: false,
            is_newcomer_chat_completed: false,
            is_restricted_trial: false,
            is_account_lapsed_player: false,
            force_character_list_sort: false,
            class_disable_mask: None,
            max_character_level: 1,
            race_unlock_data: vec![RaceUnlock {
                race_id: 1,
                has_unlocked_license: true,
                has_unlocked_achievement: false,
                has_heritage_armor_unlock_achievement: false,
                has_entitlement: true,
                hide_race_on_client: false,
                faction_balance_disabled: false,
                does_not_have_available_classes: false,
                class_unlocks: vec![ClassUnlock {
                    class_id: 2,
                    achievement_id: 3,
                    has_expansion: true,
                    has_unlocked_achievement: true,
                    has_entitlement: true,
                }],
            }],
        };
        let packet = value.encode_payload().unwrap();
        assert_eq!(&packet[..2], &[0x80, 0]);
        assert_eq!(&packet[2..6], &[0, 0, 0, 0]); // Characters
        assert_eq!(&packet[6..10], &[0, 0, 0, 0]); // RegionwideCharacters
        assert_eq!(&packet[10..14], &[1, 0, 0, 0]); // MaxCharacterLevel
        assert_eq!(&packet[14..18], &[1, 0, 0, 0]); // RaceUnlockData
        assert_eq!(packet.len(), 42);
    }

    #[test]
    fn classic_release_payload_is_exact_empty_shape() {
        assert_eq!(
            EmptyRecentAllyDataResponse70009 {
                leading_value: 0,
                mode: 0,
                entry_count: 0
            }
            .encode_payload(),
            vec![0; 9]
        );
    }
}
