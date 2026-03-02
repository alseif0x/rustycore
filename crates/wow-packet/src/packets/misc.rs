// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Miscellaneous login-sequence packets sent to the client during character login.

use wow_constants::{ClientOpcodes, ServerOpcodes};
use wow_core::ObjectGuid;

use crate::{ClientPacket, ServerPacket, WorldPacket};
use crate::world_packet::PacketError;

// ── AccountDataTimes (SMSG 0x270a) ──────────────────────────────────

/// Number of AccountDataTypes (from C# AccountDataTypes.Max = 15).
pub const NUM_ACCOUNT_DATA_TYPES: usize = 15;

/// Account data cache timestamps. Sent twice during login:
/// once with a global (empty) guid and once with the player's guid.
pub struct AccountDataTimes {
    pub player_guid: ObjectGuid,
    pub server_time: i64,
    pub account_times: [i64; NUM_ACCOUNT_DATA_TYPES],
}

impl AccountDataTimes {
    /// Global account data (no player).
    pub fn global() -> Self {
        Self {
            player_guid: ObjectGuid::EMPTY,
            server_time: unix_timestamp(),
            account_times: [0i64; NUM_ACCOUNT_DATA_TYPES],
        }
    }

    /// Per-character account data.
    pub fn for_player(guid: ObjectGuid) -> Self {
        Self {
            player_guid: guid,
            server_time: unix_timestamp(),
            account_times: [0i64; NUM_ACCOUNT_DATA_TYPES],
        }
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

// ── TutorialFlags (SMSG 0x27be) ─────────────────────────────────────

/// Tutorial flags. All 0xFFFFFFFF means all tutorials are shown/completed.
pub struct TutorialFlags {
    pub tutorial_data: [u32; 8],
}

impl TutorialFlags {
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

// ── FeatureSystemStatus (SMSG 0x25bf) — IN-GAME version ─────────────

/// Feature system status sent AFTER entering the world.
/// This is the in-game variant; for the character select screen use
/// [`FeatureSystemStatusGlueScreen`].
pub struct FeatureSystemStatus {
    pub cfg_realm_id: u32,
    pub cfg_realm_rec_id: i32,
}

impl FeatureSystemStatus {
    pub fn default_wotlk() -> Self {
        Self {
            cfg_realm_id: 1,
            cfg_realm_rec_id: 0,
        }
    }
}

impl ServerPacket for FeatureSystemStatus {
    const OPCODE: ServerOpcodes = ServerOpcodes::FeatureSystemStatus;

    fn write(&self, pkt: &mut WorldPacket) {
        // ── Fixed-size fields (exact C# order) ──
        pkt.write_uint8(2);                     // ComplaintStatus
        pkt.write_uint32(self.cfg_realm_id);    // CfgRealmID
        pkt.write_int32(self.cfg_realm_rec_id); // CfgRealmRecID

        // RAFSystem (5 fields)
        pkt.write_uint32(0);     // RAFSystem.MaxRecruits
        pkt.write_uint32(0);     // RAFSystem.MaxRecruitMonths
        pkt.write_uint32(0);     // RAFSystem.MaxRecruitmentUses
        pkt.write_uint32(0);     // RAFSystem.DaysInCycle
        pkt.write_uint32(0);     // RAFSystem.Unknown1007

        // Token/Kiosk/Store
        pkt.write_uint32(0);     // TokenPollTimeSeconds
        pkt.write_uint32(0);     // KioskSessionMinutes
        pkt.write_int64(0);      // TokenBalanceAmount
        pkt.write_uint32(0);     // BpayStoreProductDeliveryDelay
        pkt.write_uint32(0);     // ClubsPresenceUpdateTimer
        pkt.write_uint32(0);     // HiddenUIClubsPresenceUpdateTimer

        // Season/Rules/Query
        pkt.write_int32(0);      // ActiveSeason
        pkt.write_int32(0);      // GameRuleValues.Count
        pkt.write_int16(50);     // MaxPlayerNameQueriesPerPacket
        pkt.write_int16(0);      // PlayerNameQueryTelemetryInterval
        pkt.write_uint32(60);    // PlayerNameQueryInterval (seconds)

        // GameRuleValues (empty, count=0)

        // ── Bit flags (42 boolean fields — exact C# order) ──
        pkt.write_bit(false);    // VoiceEnabled
        pkt.write_bit(false);    // EuropaTicketSystemStatus.HasValue
        pkt.write_bit(false);    // BpayStoreEnabled
        pkt.write_bit(false);    // BpayStoreAvailable
        pkt.write_bit(false);    // BpayStoreDisabledByParentalControls
        pkt.write_bit(false);    // ItemRestorationButtonEnabled
        pkt.write_bit(false);    // BrowserEnabled
        pkt.write_bit(false);    // SessionAlert.HasValue
        pkt.write_bit(false);    // RAFSystem.Enabled
        pkt.write_bit(false);    // RAFSystem.RecruitingEnabled
        pkt.write_bit(false);    // CharUndeleteEnabled
        pkt.write_bit(false);    // RestrictedAccount
        pkt.write_bit(false);    // CommerceSystemEnabled
        pkt.write_bit(true);     // TutorialsEnabled
        pkt.write_bit(false);    // Unk67
        pkt.write_bit(false);    // WillKickFromWorld
        pkt.write_bit(false);    // KioskModeEnabled
        pkt.write_bit(false);    // CompetitiveModeEnabled
        pkt.write_bit(false);    // TokenBalanceEnabled
        pkt.write_bit(false);    // WarModeFeatureEnabled
        pkt.write_bit(false);    // ClubsEnabled
        pkt.write_bit(false);    // ClubsBattleNetClubTypeAllowed
        pkt.write_bit(false);    // ClubsCharacterClubTypeAllowed
        pkt.write_bit(false);    // ClubsPresenceUpdateEnabled
        pkt.write_bit(false);    // VoiceChatDisabledByParentalControl
        pkt.write_bit(false);    // VoiceChatMutedByParentalControl
        pkt.write_bit(false);    // QuestSessionEnabled
        pkt.write_bit(false);    // IsMuted
        pkt.write_bit(false);    // ClubFinderEnabled
        pkt.write_bit(false);    // Unknown901CheckoutRelated
        pkt.write_bit(false);    // TextToSpeechFeatureEnabled
        pkt.write_bit(false);    // ChatDisabledByDefault
        pkt.write_bit(false);    // ChatDisabledByPlayer
        pkt.write_bit(false);    // LFGListCustomRequiresAuthenticator
        pkt.write_bit(false);    // AddonsDisabled
        pkt.write_bit(false);    // WarGamesEnabled
        pkt.write_bit(false);    // ContentTrackingEnabled
        pkt.write_bit(false);    // IsSellAllJunkEnabled
        pkt.write_bit(false);    // IsGroupFinderEnabled
        pkt.write_bit(false);    // IsLFDEnabled
        pkt.write_bit(false);    // IsLFREnabled
        pkt.write_bit(false);    // IsPremadeGroupEnabled
        pkt.flush_bits();

        // ── QuickJoinConfig ──
        pkt.write_bit(false);    // QuickJoinConfig.ToastsDisabled
        pkt.write_float(0.0);    // QuickJoinConfig.ToastDuration
        pkt.write_float(0.0);    // QuickJoinConfig.DelayDuration
        pkt.write_float(0.0);    // QuickJoinConfig.QueueMultiplier
        pkt.write_float(0.0);    // QuickJoinConfig.PlayerMultiplier
        pkt.write_float(0.0);    // QuickJoinConfig.PlayerFriendValue
        pkt.write_float(0.0);    // QuickJoinConfig.PlayerGuildValue
        pkt.write_float(0.0);    // QuickJoinConfig.ThrottleInitialThreshold
        pkt.write_float(0.0);    // QuickJoinConfig.ThrottleDecayTime
        pkt.write_float(0.0);    // QuickJoinConfig.ThrottlePrioritySpike
        pkt.write_float(0.0);    // QuickJoinConfig.ThrottleMinThreshold
        pkt.write_float(0.0);    // QuickJoinConfig.ThrottlePvPPriorityNormal
        pkt.write_float(0.0);    // QuickJoinConfig.ThrottlePvPPriorityLow
        pkt.write_float(0.0);    // QuickJoinConfig.ThrottlePvPHonorThreshold
        pkt.write_float(0.0);    // QuickJoinConfig.ThrottleLfgListPriorityDefault
        pkt.write_float(0.0);    // QuickJoinConfig.ThrottleLfgListPriorityAbove
        pkt.write_float(0.0);    // QuickJoinConfig.ThrottleLfgListPriorityBelow
        pkt.write_float(0.0);    // QuickJoinConfig.ThrottleLfgListIlvlScalingAbove
        pkt.write_float(0.0);    // QuickJoinConfig.ThrottleLfgListIlvlScalingBelow
        pkt.write_float(0.0);    // QuickJoinConfig.ThrottleRfPriorityAbove
        pkt.write_float(0.0);    // QuickJoinConfig.ThrottleRfIlvlScalingAbove
        pkt.write_float(0.0);    // QuickJoinConfig.ThrottleDfMaxItemLevel
        pkt.write_float(0.0);    // QuickJoinConfig.ThrottleDfBestPriority

        // SessionAlert (optional — not present, bit was false)

        // Squelch
        pkt.write_bit(false);                           // Squelch.IsSquelched
        pkt.write_packed_guid(&ObjectGuid::EMPTY);      // Squelch.BnetAccountGuid
        pkt.write_packed_guid(&ObjectGuid::EMPTY);      // Squelch.GuildGuid

        // EuropaTicketSystemStatus (optional — not present, bit was false)
    }
}

// ── FeatureSystemStatusGlueScreen (SMSG 0x25c0) — CHARACTER SELECT ──

/// Feature system status for the glue screen (character select).
/// This is the version sent during session init, BEFORE entering the world.
/// Different opcode and format from [`FeatureSystemStatus`].
pub struct FeatureSystemStatusGlueScreen {
    pub max_characters_per_realm: i32,
}

impl FeatureSystemStatusGlueScreen {
    /// Default values matching C# SendFeatureSystemStatusGlueScreen.
    pub fn default_wotlk() -> Self {
        Self {
            max_characters_per_realm: 60,
        }
    }
}

impl ServerPacket for FeatureSystemStatusGlueScreen {
    const OPCODE: ServerOpcodes = ServerOpcodes::FeatureSystemStatusGlueScreen;

    fn write(&self, pkt: &mut WorldPacket) {
        // ── 27 bit flags (exact C# order) ──
        pkt.write_bit(false);    // BpayStoreEnabled
        pkt.write_bit(false);    // BpayStoreAvailable
        pkt.write_bit(false);    // BpayStoreDisabledByParentalControls
        pkt.write_bit(false);    // CharUndeleteEnabled
        pkt.write_bit(false);    // CommerceSystemEnabled
        pkt.write_bit(false);    // Unk14
        pkt.write_bit(false);    // WillKickFromWorld
        pkt.write_bit(false);    // IsExpansionPreorderInStore

        pkt.write_bit(false);    // KioskModeEnabled
        pkt.write_bit(false);    // CompetitiveModeEnabled
        pkt.write_bit(false);    // unused 10.0.2
        pkt.write_bit(false);    // TrialBoostEnabled
        pkt.write_bit(false);    // TokenBalanceEnabled
        pkt.write_bit(false);    // LiveRegionCharacterListEnabled
        pkt.write_bit(false);    // LiveRegionCharacterCopyEnabled
        pkt.write_bit(false);    // LiveRegionAccountCopyEnabled

        pkt.write_bit(false);    // LiveRegionKeyBindingsCopyEnabled
        pkt.write_bit(false);    // Unknown901CheckoutRelated
        pkt.write_bit(false);    // unused 10.0.2
        pkt.write_bit(true);     // EuropaTicketSystemStatus.HasValue (C# sets this!)
        pkt.write_bit(false);    // unused 10.0.2
        pkt.write_bit(false);    // LaunchETA.HasValue
        pkt.write_bit(false);    // AddonsDisabled
        pkt.write_bit(false);    // Unused1000

        pkt.write_bit(false);    // AccountSaveDataExportEnabled
        pkt.write_bit(false);    // AccountLockedByExport
        pkt.write_bit(false);    // RealmHiddenAlert (not empty = false)

        // No RealmHiddenAlert bits (it's empty)
        pkt.flush_bits();

        // ── EuropaTicketSystemStatus (present — bit was true) ──
        // EuropaTicketConfig.Write():
        //   4 bits (TicketsEnabled, BugsEnabled, ComplaintsEnabled, SuggestionsEnabled)
        //   then SavedThrottleObjectState (4 × u32)
        pkt.write_bit(false);    // TicketsEnabled (SupportTicketsEnabled config, default false)
        pkt.write_bit(false);    // BugsEnabled (SupportBugsEnabled config, default false)
        pkt.write_bit(false);    // ComplaintsEnabled (SupportComplaintsEnabled config, default false)
        pkt.write_bit(false);    // SuggestionsEnabled (SupportSuggestionsEnabled config, default false)
        // SavedThrottleObjectState — C# hardcodes these in SendFeatureSystemStatusGlueScreen:
        pkt.write_uint32(10);    // MaxTries
        pkt.write_uint32(60000); // PerMilliseconds
        pkt.write_uint32(1);     // TryCount
        pkt.write_uint32(111111); // LastResetTimeBeforeNow

        // ── Sequential numeric fields (exact C# order) ──
        pkt.write_uint32(0);                             // TokenPollTimeSeconds
        pkt.write_uint32(0);                             // KioskSessionMinutes
        pkt.write_int64(0);                              // TokenBalanceAmount
        pkt.write_int32(self.max_characters_per_realm);  // MaxCharactersPerRealm
        pkt.write_int32(0);                              // LiveRegionCharacterCopySourceRegions.Count
        pkt.write_uint32(0);                             // BpayStoreProductDeliveryDelay
        pkt.write_int32(0);                              // ActiveCharacterUpgradeBoostType
        pkt.write_int32(0);                              // ActiveClassTrialBoostType
        pkt.write_int32(0);                              // MinimumExpansionLevel (Classic=0)
        pkt.write_int32(2);                              // MaximumExpansionLevel (WotLK=2)
        pkt.write_int32(0);                              // ActiveSeason
        pkt.write_int32(0);                              // GameRuleValues.Count
        pkt.write_int16(50);                             // MaxPlayerNameQueriesPerPacket
        pkt.write_int16(600);                            // PlayerNameQueryTelemetryInterval (C# default=600)
        pkt.write_uint32(10);                            // PlayerNameQueryInterval (C# default=10 seconds)
        pkt.write_int32(0);                              // DebugTimeEvents.Count
        pkt.write_int32(0);                              // Unused1007

        // LaunchETA (optional — not present)
        // RealmHiddenAlert (optional — empty)
        // LiveRegionCharacterCopySourceRegions (empty, count=0)
        // GameRuleValues (empty, count=0)
        // DebugTimeEvents (empty, count=0)
    }
}

// ── ClientCacheVersion (SMSG 0x291c) ────────────────────────────────

/// Client cache version sent during session init.
pub struct ClientCacheVersion {
    pub cache_version: u32,
}

impl ServerPacket for ClientCacheVersion {
    const OPCODE: ServerOpcodes = ServerOpcodes::CacheVersion;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.cache_version);
    }
}

// ── AvailableHotfixes (SMSG 0x290f) ────────────────────────────────

/// Available hotfixes sent during session init. Empty for minimal login.
pub struct AvailableHotfixes {
    pub virtual_realm_address: u32,
}

impl ServerPacket for AvailableHotfixes {
    const OPCODE: ServerOpcodes = ServerOpcodes::AvailableHotfixes;

    fn write(&self, pkt: &mut WorldPacket) {
        // C# order: VRA first, then Count (HotfixPackets.cs:AvailableHotfixes.Write)
        pkt.write_uint32(self.virtual_realm_address);
        pkt.write_int32(0); // Hotfixes.Count = 0
    }
}

// ── ConnectionStatus (SMSG 0x2809) ─────────────────────────────────

/// BattleNet connection status sent at end of session init.
pub struct ConnectionStatus {
    pub state: u8,
    pub suppress_notification: bool,
}

impl ServerPacket for ConnectionStatus {
    const OPCODE: ServerOpcodes = ServerOpcodes::BattleNetConnectionStatus;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_bits(u32::from(self.state), 2);
        pkt.write_bit(self.suppress_notification);
        pkt.flush_bits();
    }
}

// ── SetTimeZoneInformation (SMSG 0x2677) ────────────────────────────

/// Time zone info sent to the client.
pub struct SetTimeZoneInformation {
    pub server_timezone: String,
    pub game_timezone: String,
    pub server_regional_timezone: String,
}

impl SetTimeZoneInformation {
    pub fn utc() -> Self {
        Self {
            server_timezone: "Etc/UTC".into(),
            game_timezone: "Etc/UTC".into(),
            server_regional_timezone: "Etc/UTC".into(),
        }
    }
}

impl ServerPacket for SetTimeZoneInformation {
    const OPCODE: ServerOpcodes = ServerOpcodes::SetTimeZoneInformation;

    fn write(&self, pkt: &mut WorldPacket) {
        // 7-bit length-prefixed strings
        pkt.write_bits(self.server_timezone.len() as u32, 7);
        pkt.write_bits(self.game_timezone.len() as u32, 7);
        pkt.write_bits(self.server_regional_timezone.len() as u32, 7);
        pkt.flush_bits();

        pkt.write_string(&self.server_timezone);
        pkt.write_string(&self.game_timezone);
        pkt.write_string(&self.server_regional_timezone);
    }
}

// ── LoginSetTimeSpeed (SMSG 0x270d) ─────────────────────────────────

/// Set game time and speed at login.
pub struct LoginSetTimeSpeed {
    pub server_time: i32,
    pub game_time: i32,
    pub new_speed: f32,
    pub server_time_holiday_offset: i32,
    pub game_time_holiday_offset: i32,
}

impl LoginSetTimeSpeed {
    /// Current time with standard speed (1/24 = real-time game day).
    pub fn now() -> Self {
        let t = wow_core::GameTime::now().to_packed() as i32;
        Self {
            server_time: t,
            game_time: t,
            new_speed: 1.0 / 24.0,
            server_time_holiday_offset: 0,
            game_time_holiday_offset: 0,
        }
    }
}

impl ServerPacket for LoginSetTimeSpeed {
    const OPCODE: ServerOpcodes = ServerOpcodes::LoginSetTimeSpeed;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(self.server_time);
        pkt.write_int32(self.game_time);
        pkt.write_float(self.new_speed);
        pkt.write_int32(self.server_time_holiday_offset);
        pkt.write_int32(self.game_time_holiday_offset);
    }
}

// ── SetupCurrency (SMSG 0x2573) ─────────────────────────────────────

/// Currency setup (empty for minimal login).
pub struct SetupCurrency {
    pub count: i32,
}

impl SetupCurrency {
    pub fn empty() -> Self {
        Self { count: 0 }
    }
}

impl ServerPacket for SetupCurrency {
    const OPCODE: ServerOpcodes = ServerOpcodes::SetupCurrency;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(self.count);
    }
}

// ── UndeleteCooldownStatusResponse (SMSG 0x27ce) ────────────────────

/// Response to GetUndeleteCharacterCooldownStatus.
/// Tells the client whether character undelete is on cooldown.
pub struct UndeleteCooldownStatusResponse {
    pub on_cooldown: bool,
    pub max_cooldown: i32,
    pub current_cooldown: i32,
}

impl UndeleteCooldownStatusResponse {
    /// No cooldown — character undelete is available.
    pub fn no_cooldown() -> Self {
        Self {
            on_cooldown: false,
            max_cooldown: 0,
            current_cooldown: 0,
        }
    }
}

impl ServerPacket for UndeleteCooldownStatusResponse {
    const OPCODE: ServerOpcodes = ServerOpcodes::UndeleteCooldownStatusResponse;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_bit(self.on_cooldown);
        pkt.write_int32(self.max_cooldown);
        pkt.write_int32(self.current_cooldown);
    }
}

// ── ServerTimeOffset (SMSG 0x2714) ───────────────────────────────────

/// Response to ServerTimeOffsetRequest. Sends the current realm time.
pub struct ServerTimeOffset {
    pub time: i64,
}

impl ServerTimeOffset {
    /// Current time.
    pub fn now() -> Self {
        Self { time: unix_timestamp() }
    }
}

impl ServerPacket for ServerTimeOffset {
    const OPCODE: ServerOpcodes = ServerOpcodes::ServerTimeOffset;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int64(self.time);
    }
}

// ── InitWorldStates (SMSG 0x2746) ─────────────────────────────────

/// World state variables for the current zone. Sent after UpdateObject.
/// For a minimal login, we send an empty list.
pub struct InitWorldStates {
    pub map_id: i32,
    pub area_id: i32,
    pub subarea_id: i32,
}

impl InitWorldStates {
    pub fn new(map_id: i32, zone_id: i32) -> Self {
        Self {
            map_id,
            area_id: zone_id,
            subarea_id: 0,
        }
    }
}

impl ServerPacket for InitWorldStates {
    const OPCODE: ServerOpcodes = ServerOpcodes::InitWorldStates;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(self.map_id);
        pkt.write_int32(self.area_id);
        pkt.write_int32(self.subarea_id);
        pkt.write_int32(0); // Worldstates.Count = 0
    }
}

// ── UpdateTalentData (SMSG 0x25d7) ──────────────────────────────────

/// Talent data sent during login. Empty for fresh characters.
pub struct UpdateTalentData;

impl ServerPacket for UpdateTalentData {
    const OPCODE: ServerOpcodes = ServerOpcodes::UpdateTalentData;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(0);    // UnspentTalentPoints
        pkt.write_uint8(0);    // ActiveGroup
        pkt.write_int32(1);    // TalentGroupInfos.Count (1 spec group)

        // TalentGroupInfo[0] — C# writes count twice (uint8 + uint32):
        pkt.write_uint8(0);    // (byte)Talents.Count
        pkt.write_uint32(0);   // (uint)Talents.Count
        pkt.write_uint8(6);    // (byte)MaxGlyphSlotIndex
        pkt.write_uint32(6);   // (uint)MaxGlyphSlotIndex
        pkt.write_uint8(0);    // SpecID = 0 (no spec)
        // 0 talent entries
        // 6 glyph entries (all 0):
        for _ in 0..6 {
            pkt.write_uint16(0);
        }

        pkt.write_bit(false);  // IsPetTalents
        pkt.flush_bits();
    }
}

// ── SendKnownSpells (SMSG 0x2c27) ──────────────────────────────────

/// Known spells list sent during login.
///
/// C# format:
/// ```text
/// [bit]  InitialLogin
/// [i32]  KnownSpells.Count
/// [i32]  FavoriteSpells.Count
/// [i32 × N] KnownSpells (spell IDs)
/// [i32 × M] FavoriteSpells (spell IDs)
/// ```
pub struct SendKnownSpells {
    pub initial_login: bool,
    pub known_spells: Vec<i32>,
    pub favorite_spells: Vec<i32>,
}

impl SendKnownSpells {
    /// Empty spell list for fresh characters.
    pub fn empty() -> Self {
        Self {
            initial_login: true,
            known_spells: Vec::new(),
            favorite_spells: Vec::new(),
        }
    }
}

impl ServerPacket for SendKnownSpells {
    const OPCODE: ServerOpcodes = ServerOpcodes::SendKnownSpells;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_bit(self.initial_login);
        pkt.write_int32(self.known_spells.len() as i32);
        pkt.write_int32(self.favorite_spells.len() as i32);
        for &spell_id in &self.known_spells {
            pkt.write_int32(spell_id);
        }
        for &spell_id in &self.favorite_spells {
            pkt.write_int32(spell_id);
        }
    }
}

// ── SendUnlearnSpells (SMSG 0x2c2b) ────────────────────────────────

/// Unlearned spells list. Empty for fresh characters.
pub struct SendUnlearnSpells;

impl ServerPacket for SendUnlearnSpells {
    const OPCODE: ServerOpcodes = ServerOpcodes::SendUnlearnSpells;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(0);    // Spells.Count
    }
}

// ── SendSpellHistory (SMSG 0x2c28) ──────────────────────────────────

/// Spell cooldown history. Empty for fresh characters.
pub struct SendSpellHistory;

impl ServerPacket for SendSpellHistory {
    const OPCODE: ServerOpcodes = ServerOpcodes::SendSpellHistory;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(0);    // Entries.Count
    }
}

// ── SendSpellCharges (SMSG 0x2c2a) ──────────────────────────────────

/// Spell charges. Empty for fresh characters.
pub struct SendSpellCharges;

impl ServerPacket for SendSpellCharges {
    const OPCODE: ServerOpcodes = ServerOpcodes::SendSpellCharges;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(0);    // Entries.Count
    }
}

// ── UpdateActionButtons (SMSG 0x25e0) ───────────────────────────────

/// Maximum number of action bar buttons.
pub const MAX_ACTION_BUTTONS: usize = 180;

/// Action bar buttons. 180 slots (MaxActionButtons).
///
/// Each slot is a packed i64:
/// - Bits [0:22] = action ID (spell ID, item ID, macro ID)
/// - Bits [23:30] = ActionButtonType (0=Spell, 1=Macro, 2=Item, etc.)
/// - Bits [31:63] = unused (0)
///
/// Reason: 0=Initialization, 1=AfterSpecSwap, 2=SpecSwap
pub struct UpdateActionButtons {
    pub buttons: [i64; MAX_ACTION_BUTTONS],
    pub reason: u8,
}

impl UpdateActionButtons {
    /// All slots empty (fresh character or initialization).
    pub fn empty() -> Self {
        Self {
            buttons: [0i64; MAX_ACTION_BUTTONS],
            reason: 0,
        }
    }

    /// Pack an action + type into the button format.
    ///
    /// C# packing: `action | (type << 23)` in lower 32 bits of i64.
    pub fn pack_button(action: i32, button_type: u8) -> i64 {
        let packed = (action & 0x007F_FFFF) | ((button_type as i32) << 23);
        packed as u32 as i64
    }
}

impl ServerPacket for UpdateActionButtons {
    const OPCODE: ServerOpcodes = ServerOpcodes::UpdateActionButtons;

    fn write(&self, pkt: &mut WorldPacket) {
        for &btn in &self.buttons {
            pkt.write_int64(btn);
        }
        pkt.write_uint8(self.reason);
    }
}

// ── InitializeFactions (SMSG 0x2724) ────────────────────────────────

/// Faction reputation standings. 1000 factions, all neutral for fresh chars.
/// C# interleaves flags+standings per faction, then 1000 bonus bits.
pub struct InitializeFactions;

impl ServerPacket for InitializeFactions {
    const OPCODE: ServerOpcodes = ServerOpcodes::InitializeFactions;

    fn write(&self, pkt: &mut WorldPacket) {
        // Interleaved: [flags, standing] per faction
        for _ in 0..1000 {
            pkt.write_uint16(0);   // FactionFlags
            pkt.write_int32(0);    // FactionStandings
        }
        // Then 1000 bits for FactionHasBonus
        for _ in 0..1000 {
            pkt.write_bit(false);
        }
        pkt.flush_bits();
    }
}

// ── BindPointUpdate (SMSG 0x257d) ───────────────────────────────────

/// Hearthstone bind point. Sent during login.
pub struct BindPointUpdate {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub map_id: i32,
    pub area_id: i32,
}

impl ServerPacket for BindPointUpdate {
    const OPCODE: ServerOpcodes = ServerOpcodes::BindPointUpdate;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_float(self.x);
        pkt.write_float(self.y);
        pkt.write_float(self.z);
        pkt.write_int32(self.map_id);
        pkt.write_int32(self.area_id);
    }
}

// ── WorldServerInfo (SMSG 0x25ad) ───────────────────────────────────

/// World server info sent during login.
pub struct WorldServerInfo {
    pub difficulty_id: i32,
}

impl WorldServerInfo {
    pub fn default_open_world() -> Self {
        Self { difficulty_id: 0 }
    }
}

impl ServerPacket for WorldServerInfo {
    const OPCODE: ServerOpcodes = ServerOpcodes::WorldServerInfo;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(self.difficulty_id);
        pkt.write_bit(false);  // IsTournamentRealm
        pkt.write_bit(false);  // XRealmPvpAlert
        pkt.write_bit(false);  // RestrictedAccountMaxLevel.HasValue
        pkt.write_bit(false);  // RestrictedAccountMaxMoney.HasValue
        pkt.write_bit(false);  // InstanceGroupSize.HasValue
        pkt.flush_bits();
        // No optional fields written (all HasValue=false)
    }
}

// ── InitialSetup (SMSG 0x2580) ─────────────────────────────────────

/// Expansion level info sent during login.
pub struct InitialSetup {
    pub server_expansion_level: u8,
    pub server_expansion_tier: u8,
}

impl InitialSetup {
    pub fn wotlk() -> Self {
        Self {
            server_expansion_level: 2, // WotLK
            server_expansion_tier: 0,
        }
    }
}

impl ServerPacket for InitialSetup {
    const OPCODE: ServerOpcodes = ServerOpcodes::InitialSetup;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint8(self.server_expansion_level);
        pkt.write_uint8(self.server_expansion_tier);
    }
}

// ── TimeSyncRequest (SMSG 0x2dd2) ────────────────────────────────────

/// Time synchronization request. The client uses this to sync its clock.
/// Critical for loading — client expects this before it can finish.
pub struct TimeSyncRequest {
    pub sequence_index: u32,
}

impl ServerPacket for TimeSyncRequest {
    const OPCODE: ServerOpcodes = ServerOpcodes::TimeSyncRequest;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.sequence_index);
    }
}

// ── TimeSyncResponse (CMSG 0x3a3d) ──────────────────────────────────

/// Client response to a TimeSyncRequest. Contains the client's time
/// at the moment it received the request, plus the server's sequence index.
///
/// The server must keep sending periodic TimeSyncRequests (every 5-10s)
/// or the client's internal time sync state becomes inconsistent and crashes.
pub struct TimeSyncResponse {
    pub client_time: u32,
    pub sequence_index: u32,
}

impl ClientPacket for TimeSyncResponse {
    const OPCODE: ClientOpcodes = ClientOpcodes::TimeSyncResponse;

    fn read(packet: &mut WorldPacket) -> Result<Self, PacketError> {
        let client_time = packet.read_uint32()?;
        let sequence_index = packet.read_uint32()?;
        Ok(Self {
            client_time,
            sequence_index,
        })
    }
}

// ── ContactList (SMSG 0x278c) ────────────────────────────────────────

/// Social/Friends list. Sent during login with SocialFlag::All (0x07).
pub struct ContactList {
    pub flags: u32,
}

impl ContactList {
    /// All social flags (Friend | Ignored | Muted).
    pub fn all() -> Self {
        Self { flags: 7 }
    }
}

impl ServerPacket for ContactList {
    const OPCODE: ServerOpcodes = ServerOpcodes::ContactList;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.flags);
        pkt.write_bits(0u32, 8); // Contacts.Count
        pkt.flush_bits();
    }
}

// ── ActiveGlyphs (SMSG 0x2c51) ──────────────────────────────────────

/// Active glyphs. Sent during login with IsFullUpdate=true.
pub struct ActiveGlyphs {
    pub is_full_update: bool,
}

impl ServerPacket for ActiveGlyphs {
    const OPCODE: ServerOpcodes = ServerOpcodes::ActiveGlyphs;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(0); // Glyphs.Count
        pkt.write_bit(self.is_full_update);
        pkt.flush_bits();
    }
}

// ── LoadEquipmentSet (SMSG 0x270e) ───────────────────────────────────

/// Equipment set list. Empty for fresh characters.
pub struct LoadEquipmentSet;

impl ServerPacket for LoadEquipmentSet {
    const OPCODE: ServerOpcodes = ServerOpcodes::LoadEquipmentSet;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(0); // SetData.Count
    }
}

// ── AllAccountCriteria (SMSG 0x2571) ─────────────────────────────────

/// Account-wide achievement criteria. Empty for fresh accounts.
pub struct AllAccountCriteria;

impl ServerPacket for AllAccountCriteria {
    const OPCODE: ServerOpcodes = ServerOpcodes::AllAccountCriteria;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(0); // Progress.Count
    }
}

// ── AllAchievementData (SMSG 0x2570) ─────────────────────────────────

/// Account-wide achievements. Empty for fresh accounts.
pub struct AllAchievementData;

impl ServerPacket for AllAchievementData {
    const OPCODE: ServerOpcodes = ServerOpcodes::AllAchievementData;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(0); // Earned.Count
        pkt.write_int32(0); // Progress.Count
    }
}

// ── AccountMountUpdate (SMSG 0x25ae) ─────────────────────────────────

/// Account-wide mount collection. Sent with IsFullUpdate=true on login.
pub struct AccountMountUpdate;

impl ServerPacket for AccountMountUpdate {
    const OPCODE: ServerOpcodes = ServerOpcodes::AccountMountUpdate;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_bit(true); // IsFullUpdate
        // write_int32 auto-flushes pending bits
        pkt.write_int32(0);  // Mounts.Count
        // No mount entries (each would be: i32 SpellID + 4 bits Flags)
        pkt.flush_bits();
    }
}

// ── AccountToyUpdate (SMSG 0x25b0) ───────────────────────────────────

/// Account-wide toy collection. Sent with IsFullUpdate=true on login.
pub struct AccountToyUpdate;

impl ServerPacket for AccountToyUpdate {
    const OPCODE: ServerOpcodes = ServerOpcodes::AccountToyUpdate;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_bit(true);  // IsFullUpdate
        // write_int32 auto-flushes the pending bit
        pkt.write_int32(0);   // ToyItemIDs.Count
        pkt.write_int32(0);   // IsToyFavorite.Count (same)
        pkt.write_int32(0);   // HasFanfare.Count (same)
        // No entries — each would have i32 key, then per-item bits
        pkt.flush_bits();
    }
}

// ── LoadCufProfiles (SMSG 0x25bc) ────────────────────────────────────

/// Compact Unit Frame profiles. Empty for fresh characters.
pub struct LoadCufProfiles;

impl ServerPacket for LoadCufProfiles {
    const OPCODE: ServerOpcodes = ServerOpcodes::LoadCufProfiles;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(0); // CUFProfiles.Count
    }
}

// ── AuraUpdate (SMSG 0x2c1f) ─────────────────────────────────────────

/// Aura update for a unit. On login, sent with UpdateAll=true and no auras.
pub struct AuraUpdate {
    pub unit_guid: ObjectGuid,
    pub update_all: bool,
}

impl AuraUpdate {
    /// Full aura update with no auras (fresh login).
    pub fn empty_for(guid: ObjectGuid) -> Self {
        Self {
            unit_guid: guid,
            update_all: true,
        }
    }
}

impl ServerPacket for AuraUpdate {
    const OPCODE: ServerOpcodes = ServerOpcodes::AuraUpdate;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_bit(self.update_all);
        pkt.write_bits(0u32, 9); // Auras.Count
        // No aura entries
        // write_packed_guid auto-flushes the 10 pending bits
        pkt.write_packed_guid(&self.unit_guid);
    }
}

// ── BattlePetJournalLockAcquired (SMSG 0x25ed) ──────────────────────

/// Tells the client that the battle pet journal lock has been acquired.
/// Empty packet (opcode only, no payload).
pub struct BattlePetJournalLockAcquired;

impl ServerPacket for BattlePetJournalLockAcquired {
    const OPCODE: ServerOpcodes = ServerOpcodes::BattlePetJournalLockAcquired;

    fn write(&self, _pkt: &mut WorldPacket) {
        // Empty packet — no payload
    }
}

// ── DungeonDifficultySet (SMSG 0x26a4) ───────────────────────────────

/// Sets the current dungeon difficulty. Sent BEFORE LoginVerifyWorld.
/// C# sends this via `Player.SendDungeonDifficulty()` during HandlePlayerLogin.
pub struct DungeonDifficultySet {
    pub difficulty_id: i32,
}

impl DungeonDifficultySet {
    /// Normal dungeon difficulty (default for fresh characters).
    pub fn normal() -> Self {
        Self { difficulty_id: 0 }
    }
}

impl ServerPacket for DungeonDifficultySet {
    const OPCODE: ServerOpcodes = ServerOpcodes::SetDungeonDifficulty;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(self.difficulty_id);
    }
}

// ── DbQueryBulk (CMSG 0x35e5) ─────────────────────────────────────

/// Client request for DB2 records. The server must respond with one
/// [`DBReply`] per requested record, even if the record doesn't exist.
pub struct DbQueryBulk {
    pub table_hash: u32,
    pub queries: Vec<i32>,
}

impl ClientPacket for DbQueryBulk {
    const OPCODE: ClientOpcodes = ClientOpcodes::DbQueryBulk;

    fn read(packet: &mut WorldPacket) -> Result<Self, PacketError> {
        let table_hash = packet.read_uint32()?;
        let count = packet.read_bits(13)? as usize;
        let mut queries = Vec::with_capacity(count.min(8192));
        for _ in 0..count {
            queries.push(packet.read_int32()?);
        }
        Ok(Self { table_hash, queries })
    }
}

// ── DBReply (SMSG 0x290e) ──────────────────────────────────────────

/// Response to a single [`DbQueryBulk`] record request.
/// Status: 0=NotSet, 1=Valid, 2=RecordRemoved, 3=Invalid.
pub struct DBReply {
    pub table_hash: u32,
    pub record_id: i32,
    pub timestamp: i32,
    pub status: u8,
    pub data: Vec<u8>,
}

impl DBReply {
    /// Reply with Status::Invalid (no data). The client will use its local DB2.
    pub fn not_found(table_hash: u32, record_id: i32) -> Self {
        Self {
            table_hash,
            record_id,
            timestamp: unix_timestamp() as i32,
            status: 3, // HotfixRecord.Status.Invalid
            data: Vec::new(),
        }
    }

    /// Reply with Status::RecordRemoved (2) — record is not on the server;
    /// client should use its local DB2 copy and NOT retry.
    pub fn record_removed(table_hash: u32, record_id: i32) -> Self {
        Self {
            table_hash,
            record_id,
            timestamp: unix_timestamp() as i32,
            status: 2, // HotfixRecord.Status.RecordRemoved
            data: Vec::new(),
        }
    }

    /// Reply with Status::Valid and raw blob data from hotfix_blob table.
    pub fn found(table_hash: u32, record_id: i32, data: Vec<u8>) -> Self {
        Self {
            table_hash,
            record_id,
            timestamp: unix_timestamp() as i32,
            status: 1, // HotfixRecord.Status.Valid
            data,
        }
    }
}

impl ServerPacket for DBReply {
    const OPCODE: ServerOpcodes = ServerOpcodes::DbReply;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.table_hash);
        pkt.write_int32(self.record_id);
        pkt.write_int32(self.timestamp);
        pkt.write_bits(u32::from(self.status), 3);
        // write_uint32 auto-flushes the 3 pending bits
        pkt.write_uint32(self.data.len() as u32);
        if !self.data.is_empty() {
            pkt.write_bytes(&self.data);
        }
    }
}

// ── HotfixRequest (CMSG 0x35e6) ───────────────────────────────────

/// Client request for hotfix data after receiving [`AvailableHotfixes`].
pub struct HotfixRequest {
    pub client_build: u32,
    pub data_build: u32,
    pub hotfixes: Vec<i32>,
}

impl ClientPacket for HotfixRequest {
    const OPCODE: ClientOpcodes = ClientOpcodes::HotfixRequest;

    fn read(packet: &mut WorldPacket) -> Result<Self, PacketError> {
        let client_build = packet.read_uint32()?;
        let data_build = packet.read_uint32()?;
        let count = packet.read_uint32()? as usize;
        let mut hotfixes = Vec::with_capacity(count.min(8192));
        for _ in 0..count {
            hotfixes.push(packet.read_int32()?);
        }
        Ok(Self { client_build, data_build, hotfixes })
    }
}

// ── HotfixConnect (SMSG 0x2911) ───────────────────────────────────

/// Response to [`HotfixRequest`]. Empty when server has no hotfix data.
pub struct HotfixConnect;

impl ServerPacket for HotfixConnect {
    const OPCODE: ServerOpcodes = ServerOpcodes::HotfixConnect;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(0);  // Hotfixes.Count
        pkt.write_uint32(0); // HotfixContent size
    }
}

// ── MoveSetActiveMover (SMSG 0x2dd5) ───────────────────────────────

/// Tells the client which unit it controls for movement input.
///
/// **Critical**: Without this packet the client's `m_mover` pointer is null.
/// Any camera/movement processing will dereference null → ACCESS_VIOLATION.
///
/// C# format: just a single PackedGuid.
pub struct MoveSetActiveMover {
    pub mover_guid: ObjectGuid,
}

impl ServerPacket for MoveSetActiveMover {
    const OPCODE: ServerOpcodes = ServerOpcodes::MoveSetActiveMover;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.mover_guid);
    }
}

// ── SetSpellModifier (SMSG 0x2c33 / 0x2c34) ───────────────────────

/// Spell modifier data: empty for fresh characters with no talents/auras.
///
/// The same struct is used for both `SetFlatSpellModifier` (0x2c33) and
/// `SetPctSpellModifier` (0x2c34) — only the opcode differs.
///
/// C# format:
/// ```text
/// [i32] Modifiers.Count
/// for each SpellModifierInfo:
///     [u8]  ModIndex
///     [i32] ModifierData.Count
///     for each SpellModifierData:
///         [f32] ModifierValue
///         [u8]  ClassIndex
/// ```
pub struct SetSpellModifier {
    /// Which opcode to use (Flat or Pct).
    opcode: ServerOpcodes,
}

impl SetSpellModifier {
    /// Empty flat spell modifiers (no modifier entries).
    pub fn flat_empty() -> Self {
        Self {
            opcode: ServerOpcodes::SetFlatSpellModifier,
        }
    }

    /// Empty percent spell modifiers (no modifier entries).
    pub fn pct_empty() -> Self {
        Self {
            opcode: ServerOpcodes::SetPctSpellModifier,
        }
    }

    /// Build the packet bytes (custom opcode, can't use the trait const).
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut pkt = WorldPacket::new_server(self.opcode);
        pkt.write_int32(0); // Modifiers.Count = 0
        pkt.data().to_vec()
    }
}

// ── SetProficiency (SMSG 0x2735) ───────────────────────────────────

/// Tells the client what weapon/armor types the player can use.
///
/// C# format:
/// ```text
/// [i32] ProficiencyMask  (bitmask of sub-classes)
/// [u8]  ProficiencyClass (ItemClass enum: 2=Weapon, 4=Armor)
/// ```
pub struct SetProficiency {
    pub proficiency_mask: u32,
    pub proficiency_class: u8,
}

impl ServerPacket for SetProficiency {
    const OPCODE: ServerOpcodes = ServerOpcodes::SetProficiency;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.proficiency_mask);
        pkt.write_uint8(self.proficiency_class);
    }
}

impl SetProficiency {
    /// Default weapon proficiency for a given class.
    ///
    /// Masks from C# InitDataForForm() / proficiency spell effects.
    /// Class 2 = Weapon (ItemClass.Weapon).
    pub fn default_weapons(class_id: u8) -> Self {
        // Weapon subclass bit positions (1 << subclass):
        //  0=Axe1H     0x00001   7=Sword1H   0x00080   15=Dagger    0x08000
        //  1=Axe2H     0x00002   8=Sword2H   0x00100   16=Thrown    0x10000
        //  2=Bow       0x00004  10=Staff     0x00400   18=Crossbow  0x40000
        //  3=Gun       0x00008  13=Fist      0x02000   19=Wand      0x80000
        //  4=Mace1H    0x00010
        //  5=Mace2H    0x00020
        //  6=Polearm   0x00040
        let mask = match class_id {
            1 => 0x0005_A5FF, // Warrior: Axe12,Bow,Gun,Mace12,Polearm,Sword12,Staff,Fist,Dagger,Thrown,Xbow
            2 => 0x0000_01F3, // Paladin: Axe12,Mace12,Polearm,Sword12
            3 => 0x0005_A5CF, // Hunter: Axe12,Bow,Gun,Polearm,Sword12,Staff,Fist,Dagger,Thrown,Xbow
            4 => 0x0005_A09C, // Rogue: Bow,Gun,Mace1H,Sword1H,Fist,Dagger,Thrown,Xbow
            5 => 0x0008_8410, // Priest: Mace1H,Staff,Dagger,Wand
            6 => 0x0000_01F3, // DK: Axe12,Mace12,Polearm,Sword12
            7 => 0x0000_A433, // Shaman: Axe12,Mace12,Staff,Fist,Dagger
            8 => 0x0008_8480, // Mage: Sword1H,Staff,Dagger,Wand
            9 => 0x0008_8480, // Warlock: Sword1H,Staff,Dagger,Wand
            11 => 0x0000_A470, // Druid: Mace12,Polearm,Staff,Fist,Dagger
            _ => 0x0000_2000, // Fists only
        };
        Self {
            proficiency_mask: mask,
            proficiency_class: 2, // Weapon
        }
    }

    /// Default armor proficiency for a given class.
    ///
    /// Class 4 = Armor (ItemClass.Armor).
    /// Subclass bit positions: Cloth=1(0x02), Leather=2(0x04), Mail=3(0x08),
    /// Plate=4(0x10), Shield=6(0x40).
    pub fn default_armor(class_id: u8) -> Self {
        let mask = match class_id {
            1 => 0x5E,  // Warrior: Cloth+Leather+Mail+Plate+Shield
            2 => 0x5E,  // Paladin: Cloth+Leather+Mail+Plate+Shield
            3 => 0x0E,  // Hunter: Cloth+Leather+Mail
            4 => 0x06,  // Rogue: Cloth+Leather
            5 => 0x02,  // Priest: Cloth
            6 => 0x1E,  // DK: Cloth+Leather+Mail+Plate
            7 => 0x4E,  // Shaman: Cloth+Leather+Mail+Shield
            8 => 0x02,  // Mage: Cloth
            9 => 0x02,  // Warlock: Cloth
            11 => 0x06, // Druid: Cloth+Leather
            _ => 0x02,  // Cloth
        };
        Self {
            proficiency_mask: mask,
            proficiency_class: 4, // Armor
        }
    }
}

// ── LogoutRequest (CMSG 0x34d6) ─────────────────────────────────────

/// Client requests to log out.
pub struct LogoutRequest {
    pub idle_logout: bool,
}

impl ClientPacket for LogoutRequest {
    const OPCODE: ClientOpcodes = ClientOpcodes::LogoutRequest;

    fn read(packet: &mut WorldPacket) -> Result<Self, PacketError> {
        let idle_logout = packet.read_bit()?;
        Ok(Self { idle_logout })
    }
}

// ── LogoutCancel (CMSG 0x34d8) ──────────────────────────────────────

/// Client cancels a pending logout.
pub struct LogoutCancel;

impl ClientPacket for LogoutCancel {
    const OPCODE: ClientOpcodes = ClientOpcodes::LogoutCancel;

    fn read(_packet: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self)
    }
}

// ── LogoutResponse (SMSG 0x2683) ────────────────────────────────────

/// Server responds to a logout request.
pub struct LogoutResponse {
    pub logout_result: i32,
    pub instant: bool,
}

impl LogoutResponse {
    /// Successful instant logout.
    pub fn instant_ok() -> Self {
        Self {
            logout_result: 0,
            instant: true,
        }
    }

    /// Successful delayed logout (20s timer).
    pub fn delayed_ok() -> Self {
        Self {
            logout_result: 0,
            instant: false,
        }
    }
}

impl ServerPacket for LogoutResponse {
    const OPCODE: ServerOpcodes = ServerOpcodes::LogoutResponse;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(self.logout_result);
        pkt.write_bit(self.instant);
        pkt.flush_bits();
    }
}

// ── TransferPending (SMSG 0x25cd) ────────────────────────────────────

/// Sent when the player is being teleported to a new map.
/// C# ref: MovePackets.cs - TransferPending
pub struct TransferPending {
    pub map_id: u32,
    pub old_map_position: wow_core::Position,
    pub ship: Option<ShipTransferPending>,
    pub transfer_spell_id: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct ShipTransferPending {
    pub id: u32,
    pub origin_map_id: u32,
}

impl ServerPacket for TransferPending {
    const OPCODE: ServerOpcodes = ServerOpcodes::TransferPending;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.map_id);
        pkt.write_float(self.old_map_position.x);
        pkt.write_float(self.old_map_position.y);
        pkt.write_float(self.old_map_position.z);
        pkt.write_bit(self.ship.is_some());
        pkt.write_bit(self.transfer_spell_id.is_some());
        pkt.flush_bits();
        
        if let Some(ref ship) = self.ship {
            pkt.write_uint32(ship.id);
            pkt.write_uint32(ship.origin_map_id);
        }
        
        if let Some(spell_id) = self.transfer_spell_id {
            pkt.write_uint32(spell_id);
        }
    }
}

// ── LogoutComplete (SMSG 0x2684) ────────────────────────────────────

/// Server tells client logout is complete — return to character select.
pub struct LogoutComplete;

impl ServerPacket for LogoutComplete {
    const OPCODE: ServerOpcodes = ServerOpcodes::LogoutComplete;

    fn write(&self, _pkt: &mut WorldPacket) {}
}

// ── LogoutCancelAck (SMSG 0x2685) ───────────────────────────────────

/// Server acknowledges logout cancellation.
pub struct LogoutCancelAck;

impl ServerPacket for LogoutCancelAck {
    const OPCODE: ServerOpcodes = ServerOpcodes::LogoutCancelAck;

    fn write(&self, _pkt: &mut WorldPacket) {}
}

// ── Helper ──────────────────────────────────────────────────────────

fn unix_timestamp() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

// ── ShowTradeSkill (client → server) ────────────────────────────────────────
// Sent when the player opens their own profession window from the spellbook,
// or when clicking a trade skill link to view another player's profession.

/// Parsed `CMSG_SHOW_TRADE_SKILL` (0x36CA).
#[derive(Debug, Clone)]
pub struct ShowTradeSkill {
    pub caster_guid: wow_core::ObjectGuid,
    pub spell_id: i32,
    pub skill_id: u16,
}

impl crate::ClientPacket for ShowTradeSkill {
    const OPCODE: wow_constants::ClientOpcodes = wow_constants::ClientOpcodes::ShowTradeSkill;

    fn read(packet: &mut crate::WorldPacket) -> Result<Self, crate::world_packet::PacketError> {
        let caster_guid = packet.read_packed_guid()?;
        let spell_id = packet.read_int32()?;
        let skill_id = packet.read_int32()? as u16;
        Ok(Self { caster_guid, spell_id, skill_id })
    }
}

// ── ShowTradeSkillResponse (server → client) ─────────────────────────────────
// Response to ShowTradeSkill — tells the client which recipes the player knows.

/// `SMSG_SHOW_TRADE_SKILL_RESPONSE` (0x2774).
///
/// C# struct: CasterGUID, SpellId, SkillLineId, SkillRank, SkillMaxRank,
///            SkillLineIDs[], SkillRanks[], SkillMaxRanks[], KnownAbilitySpellIDs[]
pub struct ShowTradeSkillResponse {
    pub caster_guid: wow_core::ObjectGuid,
    pub spell_id: i32,
    pub skill_line_id: u16,
    pub skill_rank: i32,
    pub skill_max_rank: i32,
    /// Known recipe/ability spell IDs for this profession.
    pub known_ability_spell_ids: Vec<i32>,
}

impl ShowTradeSkillResponse {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = crate::WorldPacket::new_server(ServerOpcodes::ShowTradeSkillResponse);
        buf.write_packed_guid(&self.caster_guid);
        buf.write_int32(self.spell_id);

        // SkillLineIDs[] — secondary lines (default [0])
        buf.write_int32(1);
        // SkillRanks[] — secondary ranks (default [0])
        buf.write_int32(1);
        // SkillMaxRanks[] — secondary max ranks (default [0])
        buf.write_int32(1);
        // KnownAbilitySpellIDs count
        buf.write_int32(self.known_ability_spell_ids.len() as i32);

        // secondary lists (each 1 entry = 0)
        buf.write_int32(0); // SkillLineIDs[0]
        buf.write_int32(0); // SkillRanks[0]
        buf.write_int32(0); // SkillMaxRanks[0]

        buf.write_int32(self.skill_line_id as i32); // SkillLineId
        buf.write_int32(self.skill_rank);           // SkillRank
        buf.write_int32(self.skill_max_rank);        // SkillMaxRank

        for spell_id in &self.known_ability_spell_ids {
            buf.write_int32(*spell_id);
        }

        buf.into_data()
    }
}

// ── PhaseShiftChange (SMSG 0x2578) ───────────────────────────────────────────
//
// Sent after AddToMap so the client knows which phases the player is in.
// Without this, the client may not render any world objects.
//
// C# ref: PhasingHandler.SendToPlayer → PhaseShiftChange.Write()
// Format:
//   WritePackedGuid(Client)         — player GUID
//   Phaseshift.Write():
//     WriteUInt32(PhaseShiftFlags)  — 0x08 = Unphased (default, no special phase)
//     WriteInt32(Phases.Count)      — 0
//     WritePackedGuid(PersonalGUID) — empty
//   WriteInt32(VisibleMapIDs * 2)  — 0 (size in bytes)
//   WriteInt32(PreloadMapIDs * 2)  — 0
//   WriteInt32(UiMapPhaseIDs * 2)  — 0

pub struct PhaseShiftChange {
    pub player_guid: ObjectGuid,
}

impl PhaseShiftChange {
    pub fn default_for(player_guid: ObjectGuid) -> Self {
        Self { player_guid }
    }
}

impl ServerPacket for PhaseShiftChange {
    const OPCODE: ServerOpcodes = ServerOpcodes::PhaseShiftChange;

    fn write(&self, pkt: &mut crate::WorldPacket) {
        // Client GUID
        pkt.write_packed_guid(&self.player_guid);
        // Phaseshift block: flags + phases count + personal guid
        pkt.write_uint32(0x08); // PhaseShiftFlags::Unphased
        pkt.write_int32(0);     // Phases.Count = 0
        pkt.write_packed_guid(&ObjectGuid::EMPTY); // PersonalGUID = empty
        // VisibleMapIDs count * 2 (size in bytes)
        pkt.write_int32(0);
        // PreloadMapIDs count * 2
        pkt.write_int32(0);
        // UiMapPhaseIDs count * 2
        pkt.write_int32(0);
    }
}

// ── Vendor packets ───────────────────────────────────────────────────────────
//
// C# ref: NpcPackets.cs — VendorInventory, BuyItem, BuySucceeded, BuyFailed, SellItem

/// One item in the vendor's inventory list.
/// C#: VendorItemPkt
#[derive(Debug, Clone)]
pub struct VendorItem {
    pub muid: i32,         // slot/muid index
    pub item_id: i32,
    pub item_type: i32,    // 1 = item, 2 = currency
    pub quantity: i32,     // max stack on vendor (-1 = unlimited)
    pub price: u64,        // buy price (copper)
    pub durability: i32,
    pub stack_count: i32,  // VendorStackCount from item_sparse
    pub extended_cost: i32,
    pub player_condition_failed: i32,
    pub locked: bool,
    pub do_not_filter: bool,
    pub refundable: bool,
}

/// SMSG_VENDOR_INVENTORY — list of items a vendor is selling.
/// C#: VendorInventory
pub struct VendorInventory {
    pub vendor_guid: ObjectGuid,
    pub reason: u8,  // 0 = ok, non-0 = error (no items etc)
    pub items: Vec<VendorItem>,
}

impl ServerPacket for VendorInventory {
    const OPCODE: ServerOpcodes = ServerOpcodes::VendorInventory;

    fn write(&self, pkt: &mut crate::WorldPacket) {
        pkt.write_packed_guid(&self.vendor_guid);
        pkt.write_uint8(self.reason);
        pkt.write_int32(self.items.len() as i32);

        for (i, item) in self.items.iter().enumerate() {
            pkt.write_uint64(item.price);
            pkt.write_int32(item.muid);
            pkt.write_int32(item.item_type);
            pkt.write_int32(item.durability);
            pkt.write_int32(item.stack_count);
            pkt.write_int32(item.quantity);
            pkt.write_int32(item.extended_cost);
            pkt.write_int32(item.player_condition_failed);
            // 3 bits: Locked, DoNotFilterOnVendor, Refundable
            pkt.write_bit(item.locked);
            pkt.write_bit(item.do_not_filter);
            pkt.write_bit(item.refundable);
            pkt.flush_bits();
            // ItemInstance inline:
            //   ItemID (i32), RandomPropertiesSeed (i32), RandomPropertiesID (i32)
            //   bit(ItemBonus != null) = false, FlushBits
            //   ItemModList: WriteBits(0, 6) + FlushBits  (no mods)
            pkt.write_int32(item.item_id);
            pkt.write_int32(0i32); // RandomPropertiesSeed
            pkt.write_int32(0i32); // RandomPropertiesID
            pkt.write_bit(false);  // has ItemBonus = false
            pkt.flush_bits();
            pkt.write_bits(0u32, 6); // ItemModList count = 0
            pkt.flush_bits();
            // no ItemMod entries, no ItemBonus
            let _ = i; // suppress unused
        }
    }
}

/// CMSG_BUY_ITEM — client wants to buy an item from a vendor.
/// C#: BuyItem
#[derive(Debug)]
pub struct BuyItem {
    pub vendor_guid: ObjectGuid,
    pub container_guid: ObjectGuid,
    pub quantity: i32,
    pub muid: i32,
    pub slot: i32,
    pub item_type: i32,
    pub item_id: i32,
}

impl ClientPacket for BuyItem {
    const OPCODE: wow_constants::ClientOpcodes = wow_constants::ClientOpcodes::BuyItem;

    fn read(pkt: &mut crate::WorldPacket) -> Result<Self, PacketError> {
        let vendor_guid = pkt.read_packed_guid()?;
        let container_guid = pkt.read_packed_guid()?;
        let quantity = pkt.read_int32()?;
        let muid = pkt.read_int32()?;
        let slot = pkt.read_int32()?;
        let item_type = pkt.read_int32()?;
        // ItemInstance.Read: ItemID, RandomPropertiesSeed, RandomPropertiesID, bit(hasBonus), FlushBits, ItemModList
        let item_id = pkt.read_int32()?;
        let _seed = pkt.read_int32()?;
        let _rand_prop = pkt.read_int32()?;
        let has_bonus = pkt.read_bit()?;
        let mod_count = pkt.read_bits(6)? as u32;
        for _ in 0..mod_count {
            let _val = pkt.read_int32()?;
            let _ty  = pkt.read_uint8()?;
        }
        if has_bonus {
            // ItemBonuses: Context (u8) + BonusListIDs count + entries
            let _ctx = pkt.read_uint8()?;
            let bonus_count = pkt.read_uint32()?;
            for _ in 0..bonus_count {
                let _bid = pkt.read_uint16()?;
            }
        }
        Ok(Self { vendor_guid, container_guid, quantity, muid, slot, item_type, item_id })
    }
}

/// SMSG_BUY_SUCCEEDED — item bought successfully.
/// C#: BuySucceeded
pub struct BuySucceeded {
    pub vendor_guid: ObjectGuid,
    pub muid: i32,
    pub new_quantity: i32,
    pub quantity_bought: i32,
}

impl ServerPacket for BuySucceeded {
    const OPCODE: ServerOpcodes = ServerOpcodes::BuySucceeded;

    fn write(&self, pkt: &mut crate::WorldPacket) {
        pkt.write_packed_guid(&self.vendor_guid);
        pkt.write_int32(self.muid);
        pkt.write_int32(self.new_quantity);
        pkt.write_int32(self.quantity_bought);
    }
}

/// SMSG_BUY_FAILED — buy failed with reason code.
/// C#: BuyFailed
pub struct BuyFailed {
    pub vendor_guid: ObjectGuid,
    pub muid: i32,
    pub reason: u8,
}

impl ServerPacket for BuyFailed {
    const OPCODE: ServerOpcodes = ServerOpcodes::BuyFailed;

    fn write(&self, pkt: &mut crate::WorldPacket) {
        pkt.write_packed_guid(&self.vendor_guid);
        pkt.write_int32(self.muid);
        pkt.write_uint8(self.reason);
    }
}

/// CMSG_SELL_ITEM — client wants to sell an item to a vendor.
/// C#: SellItem
#[derive(Debug)]
pub struct SellItem {
    pub vendor_guid: ObjectGuid,
    pub item_guid: ObjectGuid,
    pub amount: i32,
}

impl ClientPacket for SellItem {
    const OPCODE: wow_constants::ClientOpcodes = wow_constants::ClientOpcodes::SellItem;

    fn read(pkt: &mut crate::WorldPacket) -> Result<Self, PacketError> {
        let vendor_guid = pkt.read_packed_guid()?;
        let item_guid = pkt.read_packed_guid()?;
        let amount = pkt.read_int32()?;
        Ok(Self { vendor_guid, item_guid, amount })
    }
}

/// SMSG_SELL_RESPONSE — result of a sell operation.
/// C#: SellResponse
pub struct SellResponse {
    pub vendor_guid: ObjectGuid,
    pub item_guid: ObjectGuid,
    pub reason: u8, // 0 = ok
}

impl ServerPacket for SellResponse {
    const OPCODE: ServerOpcodes = ServerOpcodes::SellResponse;

    fn write(&self, pkt: &mut crate::WorldPacket) {
        pkt.write_packed_guid(&self.vendor_guid);
        pkt.write_packed_guid(&self.item_guid);
        pkt.write_uint8(self.reason);
    }
}

// ── PlayedTime (SMSG 0x26d5) ─────────────────────────────────────────────────

/// Server response to CMSG_REQUEST_PLAYED_TIME.
///
/// C# ref: `MiscHandler.HandlePlayedTime` → `PlayedTime` packet.
/// Fields: TotalTime (u32), LevelTime (u32), TriggerEvent (bool).
pub struct PlayedTime {
    /// Total time the character has been played (seconds).
    pub total_time: u32,
    /// Time played at the current level (seconds).
    pub level_time: u32,
    /// Mirror of the client's TriggerScriptEvent flag.
    pub trigger_event: bool,
}

impl ServerPacket for PlayedTime {
    const OPCODE: ServerOpcodes = ServerOpcodes::PlayedTime;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.total_time);
        pkt.write_uint32(self.level_time);
        pkt.write_bit(self.trigger_event);
        pkt.flush_bits();
    }
}

// ── TaxiNodeStatusPkt (SMSG 0x267C) ─────────────────────────────────────────
/// Response to CMSG_TAXI_NODE_STATUS_QUERY.
/// C# ref: TaxiPackets.TaxiNodeStatusPkt
/// Status bits: 0=None, 1=Learned, 2=Unlearned, 3=NotEligible
pub struct TaxiNodeStatusPkt {
    pub unit_guid: wow_core::ObjectGuid,
    /// 2-bit field: 0=None 1=Learned 2=Unlearned 3=NotEligible
    pub status: u8,
}

impl ServerPacket for TaxiNodeStatusPkt {
    const OPCODE: ServerOpcodes = ServerOpcodes::TaxiNodeStatus;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.unit_guid);
        pkt.write_bits(self.status as u32, 2);
        pkt.flush_bits();
    }
}

// ── RequestCemeteryListResponse (SMSG 0x258F) ────────────────────────────────
/// Response to CMSG_REQUEST_CEMETERY_LIST.
/// C# ref: MiscPackets.RequestCemeteryListResponse (ConnectionType.Instance)
pub struct RequestCemeteryListResponse {
    pub is_gossip_triggered: bool,
    pub cemetery_ids: Vec<u32>,
}

impl RequestCemeteryListResponse {
    /// Empty response — no graveyards in this zone.
    pub fn empty(is_gossip_triggered: bool) -> Self {
        Self { is_gossip_triggered, cemetery_ids: vec![] }
    }
}

impl ServerPacket for RequestCemeteryListResponse {
    const OPCODE: ServerOpcodes = ServerOpcodes::RequestCemeteryListResponse;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_bit(self.is_gossip_triggered);
        pkt.flush_bits();
        pkt.write_uint32(self.cemetery_ids.len() as u32);
        for id in &self.cemetery_ids {
            pkt.write_uint32(*id);
        }
    }
}

// ── AuctionHelloResponse ─────────────────────────────────────────────────────

/// SMSG_AUCTION_HELLO_RESPONSE — opens the auction house UI on the client.
/// C# ref: AuctionHousePackets.AuctionHelloResponse
pub struct AuctionHelloResponse {
    /// GUID of the auctioneer NPC.
    pub auctioneer_guid: wow_core::ObjectGuid,
    /// AuctionHouse.db2 entry id (1=Alliance, 2=Horde, 7=Neutral).
    pub auction_house_id: i32,
    /// Delay in ms before purchased items are delivered.
    pub purchased_item_delivery_delay: i32,
    /// Delay in ms before cancelled items are returned.
    pub cancelled_item_delivery_delay: i32,
    /// Whether the auction house is currently open for business.
    pub open_for_business: bool,
}

impl AuctionHelloResponse {
    /// Convenience: open neutral auction house for a given NPC guid.
    pub fn open(auctioneer_guid: wow_core::ObjectGuid) -> Self {
        Self {
            auctioneer_guid,
            auction_house_id: 7, // neutral
            purchased_item_delivery_delay: 3_600_000,  // 1 hour
            cancelled_item_delivery_delay: 3_600_000,
            open_for_business: true,
        }
    }
}

impl ServerPacket for AuctionHelloResponse {
    const OPCODE: ServerOpcodes = ServerOpcodes::AuctionHelloResponse;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.auctioneer_guid);
        pkt.write_int32(self.purchased_item_delivery_delay);
        pkt.write_int32(self.cancelled_item_delivery_delay);
        pkt.write_int32(self.auction_house_id);
        pkt.write_bit(self.open_for_business);
        pkt.flush_bits();
    }
}

// ── NpcInteractionOpenResult ──────────────────────────────────────────────────

/// SMSG_NPC_INTERACTION_OPEN_RESULT — opens an NPC interaction UI on client.
/// C# ref: NPCPackets.NPCInteractionOpenResult
/// PlayerInteractionType values: Banker=8, Binder=20, Auctioneer=21,
/// StableMaster=22, GuildTabardVendor=14, TaxiNode=6, Merchant=5, Trainer=7.
pub struct NpcInteractionOpenResult {
    pub npc: wow_core::ObjectGuid,
    pub interaction_type: i32,
    pub success: bool,
}

impl NpcInteractionOpenResult {
    pub fn new(npc: wow_core::ObjectGuid, interaction_type: i32) -> Self {
        Self { npc, interaction_type, success: true }
    }
}

impl ServerPacket for NpcInteractionOpenResult {
    const OPCODE: ServerOpcodes = ServerOpcodes::NpcInteractionOpenResult;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.npc);
        pkt.write_int32(self.interaction_type);
        pkt.write_bit(self.success);
        pkt.flush_bits();
    }
}

// ── Auction empty results ─────────────────────────────────────────────────────

/// SMSG_AUCTION_LIST_BIDDER_ITEMS_RESULT — empty bidder list.
pub struct AuctionListBidderItemsResult;
impl ServerPacket for AuctionListBidderItemsResult {
    const OPCODE: ServerOpcodes = ServerOpcodes::AuctionListBidderItemsResult;
    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(0); // Items.Count
        pkt.write_int32(0); // TotalCount
        pkt.write_int32(0); // DesiredDelay (ms)
    }
}

/// SMSG_AUCTION_LIST_OWNER_ITEMS_RESULT — empty owner list.
pub struct AuctionListOwnerItemsResult;
impl ServerPacket for AuctionListOwnerItemsResult {
    const OPCODE: ServerOpcodes = ServerOpcodes::AuctionListOwnerItemsResult;
    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(0); // Items.Count
        pkt.write_int32(0); // TotalCount
        pkt.write_int32(0); // DesiredDelay
    }
}

/// SMSG_AUCTION_LIST_PENDING_SALES_RESULT — empty pending sales.
pub struct AuctionListPendingSalesResult;
impl ServerPacket for AuctionListPendingSalesResult {
    const OPCODE: ServerOpcodes = ServerOpcodes::AuctionListPendingSalesResult;
    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(0); // Mails.Count
        pkt.write_int32(0); // TotalNumRecords
    }
}

// ── QueryTimeResponse ────────────────────────────────────────────────────────

/// SMSG_QUERY_TIME_RESPONSE — server time response to CMSG_QUERY_TIME.
/// C# ref: QueryPackets.QueryTimeResponse → WriteInt64(CurrentTime)
pub struct QueryTimeResponse {
    /// Current server Unix timestamp (seconds).
    pub current_time: i64,
}

impl ServerPacket for QueryTimeResponse {
    const OPCODE: ServerOpcodes = ServerOpcodes::QueryTimeResponse;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int64(self.current_time);
    }
}

// ── MailQueryNextTimeResult ──────────────────────────────────────────────────

/// SMSG_MAIL_QUERY_NEXT_TIME_RESULT — tells client when next mail arrives.
/// C# ref: MailPackets.MailQueryNextTimeResult
/// next_mail_time = -1.0 means "no mail pending".
pub struct MailQueryNextTimeResult {
    /// -1.0 = no mail, 0.0 = mail now, >0 = seconds until delivery.
    pub next_mail_time: f32,
}

impl MailQueryNextTimeResult {
    /// Convenience: "no mail pending" response.
    pub fn no_mail() -> Self {
        Self { next_mail_time: -1.0 }
    }
}

impl ServerPacket for MailQueryNextTimeResult {
    const OPCODE: ServerOpcodes = ServerOpcodes::MailQueryNextTimeResult;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_float(self.next_mail_time);
        pkt.write_int32(0); // Next.Count = 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn account_data_times_global() {
        let pkt = AccountDataTimes::global();
        let bytes = pkt.to_bytes();
        // opcode(2) + packed_guid(2 for empty) + server_time(8) + 15*i64(120) = 132
        assert_eq!(bytes.len(), 132);
    }

    #[test]
    fn account_data_times_player() {
        let guid = ObjectGuid::create_player(1, 42);
        let pkt = AccountDataTimes::for_player(guid);
        let bytes = pkt.to_bytes();
        assert!(bytes.len() > 76); // Bigger than empty GUID version
    }

    #[test]
    fn tutorial_flags_all_shown() {
        let pkt = TutorialFlags::all_shown();
        let bytes = pkt.to_bytes();
        // opcode(2) + 8*u32(32) = 34
        assert_eq!(bytes.len(), 34);
    }

    #[test]
    fn feature_system_status_serializes() {
        let pkt = FeatureSystemStatus::default_wotlk();
        let bytes = pkt.to_bytes();
        assert!(bytes.len() > 20);
        // Verify opcode is FeatureSystemStatus (0x25bf)
        let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
        assert_eq!(opcode, 0x25bf);
    }

    #[test]
    fn feature_system_status_glue_screen_serializes() {
        let pkt = FeatureSystemStatusGlueScreen::default_wotlk();
        let bytes = pkt.to_bytes();
        assert!(bytes.len() > 20);
        // Verify opcode is FeatureSystemStatusGlueScreen (0x25c0)
        let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
        assert_eq!(opcode, 0x25c0);
    }

    #[test]
    fn client_cache_version_serializes() {
        let pkt = ClientCacheVersion { cache_version: 42 };
        let bytes = pkt.to_bytes();
        // opcode(2) + uint32(4) = 6
        assert_eq!(bytes.len(), 6);
        let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
        assert_eq!(opcode, 0x291c);
    }

    #[test]
    fn available_hotfixes_empty_serializes() {
        let pkt = AvailableHotfixes { virtual_realm_address: 1 };
        let bytes = pkt.to_bytes();
        // opcode(2) + uint32(4) + int32(4) = 10
        assert_eq!(bytes.len(), 10);
        let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
        assert_eq!(opcode, 0x290f);
    }

    #[test]
    fn connection_status_serializes() {
        let pkt = ConnectionStatus { state: 1, suppress_notification: true };
        let bytes = pkt.to_bytes();
        // opcode(2) + 3 bits flushed to 1 byte = 3
        assert_eq!(bytes.len(), 3);
        let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
        assert_eq!(opcode, 0x2809);
    }

    #[test]
    fn set_timezone_utc() {
        let pkt = SetTimeZoneInformation::utc();
        let bytes = pkt.to_bytes();
        // Should contain "Etc/UTC" x3
        assert!(bytes.len() > 20);
    }

    #[test]
    fn login_set_time_speed_now() {
        let pkt = LoginSetTimeSpeed::now();
        let bytes = pkt.to_bytes();
        // opcode(2) + 4*i32(16) + float(4) = 22
        assert_eq!(bytes.len(), 22);
    }

    #[test]
    fn setup_currency_empty() {
        let pkt = SetupCurrency::empty();
        let bytes = pkt.to_bytes();
        // opcode(2) + i32(4) = 6
        assert_eq!(bytes.len(), 6);
    }

    #[test]
    fn init_world_states_empty() {
        let pkt = InitWorldStates::new(0, 12);
        let bytes = pkt.to_bytes();
        // opcode(2) + 4*i32(16) = 18
        assert_eq!(bytes.len(), 18);
        let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
        assert_eq!(opcode, 0x2746);
    }

    #[test]
    fn update_talent_data_empty() {
        let pkt = UpdateTalentData;
        let bytes = pkt.to_bytes();
        // opcode(2) + int32(4) + uint8(1) + int32(4) +
        // TalentGroupInfo: uint8(1)+uint32(4)+uint8(1)+uint32(4)+uint8(1)+6*uint16(12) +
        // bit(IsPetTalents) flushed to 1 byte = 2+4+1+4+1+4+1+4+1+12+1 = 35
        assert_eq!(bytes.len(), 35);
        let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
        assert_eq!(opcode, 0x25d7);
    }

    #[test]
    fn send_known_spells_empty() {
        let pkt = SendKnownSpells::empty();
        let bytes = pkt.to_bytes();
        // opcode(2) + bit(flush)+int32(4)+int32(4) = 2+1+4+4 = 11
        assert_eq!(bytes.len(), 11);
    }

    #[test]
    fn send_known_spells_with_data() {
        let pkt = SendKnownSpells {
            initial_login: true,
            known_spells: vec![6603, 78, 2457],
            favorite_spells: vec![],
        };
        let bytes = pkt.to_bytes();
        // opcode(2) + bit(flush)(1) + count(4) + fav_count(4) + 3*i32(12) = 23
        assert_eq!(bytes.len(), 23);
    }

    #[test]
    fn send_spell_history_empty() {
        let pkt = SendSpellHistory;
        let bytes = pkt.to_bytes();
        // opcode(2) + int32(4) = 6
        assert_eq!(bytes.len(), 6);
    }

    #[test]
    fn update_action_buttons_empty() {
        let pkt = UpdateActionButtons::empty();
        let bytes = pkt.to_bytes();
        // opcode(2) + 180*i64(1440) + uint8(1) = 1443
        assert_eq!(bytes.len(), 1443);
        let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
        assert_eq!(opcode, 0x25e0);
    }

    #[test]
    fn update_action_buttons_pack() {
        // Spell 6603 (Auto Attack) as type 0 (Spell)
        let packed = UpdateActionButtons::pack_button(6603, 0);
        assert_eq!(packed, 6603);

        // Spell 78 (Heroic Strike) as type 0
        let packed = UpdateActionButtons::pack_button(78, 0);
        assert_eq!(packed, 78);

        // Item action as type 2
        let packed = UpdateActionButtons::pack_button(12345, 2);
        // 12345 | (2 << 23) = 12345 | 16777216 = 16789561
        assert_eq!(packed, 12345 | (2i64 << 23));
    }

    #[test]
    fn initialize_factions_empty() {
        let pkt = InitializeFactions;
        let bytes = pkt.to_bytes();
        // opcode(2) + 1000*(uint16+int32) + ceil(1000/8) = 2 + 6000 + 125 = 6127
        assert_eq!(bytes.len(), 6127);
        let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
        assert_eq!(opcode, 0x2724);
    }

    #[test]
    fn bind_point_update_serializes() {
        let pkt = BindPointUpdate {
            x: 1.0, y: 2.0, z: 3.0,
            map_id: 0, area_id: 12,
        };
        let bytes = pkt.to_bytes();
        // opcode(2) + 3*f32(12) + 2*i32(8) = 22
        assert_eq!(bytes.len(), 22);
        let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
        assert_eq!(opcode, 0x257d);
    }

    #[test]
    fn world_server_info_serializes() {
        let pkt = WorldServerInfo::default_open_world();
        let bytes = pkt.to_bytes();
        // opcode(2) + int32(4) + 5 bits flushed to 1 byte = 7
        assert_eq!(bytes.len(), 7);
    }

    #[test]
    fn initial_setup_wotlk() {
        let pkt = InitialSetup::wotlk();
        let bytes = pkt.to_bytes();
        // opcode(2) + uint8(1) + uint8(1) = 4
        assert_eq!(bytes.len(), 4);
        let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
        assert_eq!(opcode, 0x2580);
    }

    #[test]
    fn time_sync_request_serializes() {
        let pkt = TimeSyncRequest { sequence_index: 0 };
        let bytes = pkt.to_bytes();
        // opcode(2) + u32(4) = 6
        assert_eq!(bytes.len(), 6);
        let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
        assert_eq!(opcode, 0x2dd2);
    }

    #[test]
    fn contact_list_empty() {
        let pkt = ContactList::all();
        let bytes = pkt.to_bytes();
        // opcode(2) + u32(4) + bits(8→1 byte) = 7
        assert_eq!(bytes.len(), 7);
        let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
        assert_eq!(opcode, 0x278c);
        // Flags = 7 (All)
        let flags = u32::from_le_bytes([bytes[2], bytes[3], bytes[4], bytes[5]]);
        assert_eq!(flags, 7);
    }

    #[test]
    fn active_glyphs_empty() {
        let pkt = ActiveGlyphs { is_full_update: true };
        let bytes = pkt.to_bytes();
        // opcode(2) + i32(4) + 1 bit flushed to 1 byte = 7
        assert_eq!(bytes.len(), 7);
        let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
        assert_eq!(opcode, 0x2c51);
    }

    #[test]
    fn load_equipment_set_empty() {
        let pkt = LoadEquipmentSet;
        let bytes = pkt.to_bytes();
        // opcode(2) + i32(4) = 6
        assert_eq!(bytes.len(), 6);
        let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
        assert_eq!(opcode, 0x270e);
    }

    #[test]
    fn all_account_criteria_empty() {
        let pkt = AllAccountCriteria;
        let bytes = pkt.to_bytes();
        // opcode(2) + i32(4) = 6
        assert_eq!(bytes.len(), 6);
        let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
        assert_eq!(opcode, 0x2571);
    }

    #[test]
    fn all_achievement_data_empty() {
        let pkt = AllAchievementData;
        let bytes = pkt.to_bytes();
        // opcode(2) + i32(4) + i32(4) = 10
        assert_eq!(bytes.len(), 10);
        let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
        assert_eq!(opcode, 0x2570);
    }

    #[test]
    fn account_mount_update_empty() {
        let pkt = AccountMountUpdate;
        let bytes = pkt.to_bytes();
        // opcode(2) + 1 bit(padded to 1 byte) + i32(4) = 7
        // wait: write_bit(true) → 1 bit buffered, then write_int32(0)
        // auto-flushes → 1 byte (bit), then 4 bytes (i32), then flush_bits (no-op) = 7
        assert_eq!(bytes.len(), 7);
        let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
        assert_eq!(opcode, 0x25ae);
    }

    #[test]
    fn account_toy_update_empty() {
        let pkt = AccountToyUpdate;
        let bytes = pkt.to_bytes();
        // opcode(2) + 1 bit(padded to 1 byte) + 3*i32(12) = 15
        assert_eq!(bytes.len(), 15);
        let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
        assert_eq!(opcode, 0x25b0);
    }

    #[test]
    fn load_cuf_profiles_empty() {
        let pkt = LoadCufProfiles;
        let bytes = pkt.to_bytes();
        // opcode(2) + i32(4) = 6
        assert_eq!(bytes.len(), 6);
        let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
        assert_eq!(opcode, 0x25bc);
    }

    #[test]
    fn aura_update_empty() {
        let guid = ObjectGuid::create_player(1, 42);
        let pkt = AuraUpdate::empty_for(guid);
        let bytes = pkt.to_bytes();
        // opcode(2) + 10 bits(padded to 2 bytes) + packed_guid(variable)
        assert!(bytes.len() > 4);
        let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
        assert_eq!(opcode, 0x2c1f);
        // Byte 2: UpdateAll=1(MSB) + first 7 bits of count(0) = 0x80
        assert_eq!(bytes[2], 0x80);
    }

    #[test]
    fn battle_pet_journal_lock_acquired_empty() {
        let pkt = BattlePetJournalLockAcquired;
        let bytes = pkt.to_bytes();
        // opcode(2) + no payload = 2
        assert_eq!(bytes.len(), 2);
        let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
        assert_eq!(opcode, 0x25ed);
    }

    #[test]
    fn db_reply_not_found() {
        let pkt = DBReply::not_found(0xDF2F53CF, 42);
        let bytes = pkt.to_bytes();
        // opcode(2) + u32(4) + i32(4) + i32(4) + 3 bits flushed(1) + u32(4) = 19
        assert_eq!(bytes.len(), 19);
        let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
        assert_eq!(opcode, 0x290e);
        // table_hash
        let th = u32::from_le_bytes([bytes[2], bytes[3], bytes[4], bytes[5]]);
        assert_eq!(th, 0xDF2F53CF);
        // record_id
        let rid = i32::from_le_bytes([bytes[6], bytes[7], bytes[8], bytes[9]]);
        assert_eq!(rid, 42);
        // status byte: 3 bits MSB-first for value 3 = 0b011 → in MSB-first bit layout: 0_1_1_00000 = 0x60
        assert_eq!(bytes[14], 0x60);
        // data size = 0
        let ds = u32::from_le_bytes([bytes[15], bytes[16], bytes[17], bytes[18]]);
        assert_eq!(ds, 0);
    }

    #[test]
    fn db_query_bulk_roundtrip() {
        // Build a DbQueryBulk packet manually with 13-bit count.
        // Use a WorldPacket's bit writer to produce correctly-encoded bits.
        let mut writer = WorldPacket::new_server(ServerOpcodes::DbReply);
        // Overwrite opcode with client opcode (we'll skip it anyway)
        // Just append the payload fields after a dummy 2-byte opcode:
        writer.write_uint32(0xAABBCCDD);  // table_hash
        writer.write_bits(3, 13);         // count = 3 (13 bits)
        writer.flush_bits();
        writer.write_int32(100);
        writer.write_int32(200);
        writer.write_int32(300);

        // Read it back: from_bytes includes the 2-byte opcode from new_server
        let mut reader = WorldPacket::from_bytes(writer.data());
        reader.skip_opcode(); // skip the 2-byte dummy opcode
        let parsed = DbQueryBulk::read(&mut reader).unwrap();
        assert_eq!(parsed.table_hash, 0xAABBCCDD);
        assert_eq!(parsed.queries, vec![100, 200, 300]);
    }

    #[test]
    fn hotfix_connect_empty() {
        let pkt = HotfixConnect;
        let bytes = pkt.to_bytes();
        // opcode(2) + i32(4) + u32(4) = 10
        assert_eq!(bytes.len(), 10);
        let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
        assert_eq!(opcode, 0x2911);
        // count = 0
        let count = i32::from_le_bytes([bytes[2], bytes[3], bytes[4], bytes[5]]);
        assert_eq!(count, 0);
        // content size = 0
        let size = u32::from_le_bytes([bytes[6], bytes[7], bytes[8], bytes[9]]);
        assert_eq!(size, 0);
    }

    #[test]
    fn dungeon_difficulty_set_normal() {
        let pkt = DungeonDifficultySet::normal();
        let bytes = pkt.to_bytes();
        // opcode(2) + i32(4) = 6
        assert_eq!(bytes.len(), 6);
        let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
        assert_eq!(opcode, 0x26a4);
        let difficulty = i32::from_le_bytes([bytes[2], bytes[3], bytes[4], bytes[5]]);
        assert_eq!(difficulty, 0);
    }

    #[test]
    fn move_set_active_mover() {
        let guid = ObjectGuid::create_player(1, 42);
        let pkt = MoveSetActiveMover { mover_guid: guid };
        let bytes = pkt.to_bytes();
        let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
        assert_eq!(opcode, 0x2dd5);
        // After opcode: packed guid (variable length, but > 0)
        assert!(bytes.len() > 2);
    }

    #[test]
    fn set_spell_modifier_flat_empty() {
        let bytes = SetSpellModifier::flat_empty().to_bytes();
        let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
        assert_eq!(opcode, 0x2c33);
        // opcode(2) + i32(4) = 6
        assert_eq!(bytes.len(), 6);
        let count = i32::from_le_bytes([bytes[2], bytes[3], bytes[4], bytes[5]]);
        assert_eq!(count, 0);
    }

    #[test]
    fn set_spell_modifier_pct_empty() {
        let bytes = SetSpellModifier::pct_empty().to_bytes();
        let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
        assert_eq!(opcode, 0x2c34);
        assert_eq!(bytes.len(), 6);
    }

    #[test]
    fn set_proficiency_weapon() {
        let pkt = SetProficiency::default_weapons(1); // Warrior
        let bytes = pkt.to_bytes();
        let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
        assert_eq!(opcode, 0x2735);
        // opcode(2) + u32(4) + u8(1) = 7
        assert_eq!(bytes.len(), 7);
        // Class byte = 2 (Weapon)
        assert_eq!(bytes[6], 2);
    }

    #[test]
    fn logout_request_read() {
        let mut writer = WorldPacket::new_server(ServerOpcodes::DbReply); // dummy opcode
        writer.write_bit(true); // idle_logout
        writer.flush_bits();
        let mut reader = WorldPacket::from_bytes(writer.data());
        reader.skip_opcode();
        let req = LogoutRequest::read(&mut reader).unwrap();
        assert!(req.idle_logout);
    }

    #[test]
    fn logout_response_instant_ok() {
        let pkt = LogoutResponse::instant_ok();
        let bytes = pkt.to_bytes();
        let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
        assert_eq!(opcode, 0x2683);
        // i32(4) + 1 bit flushed(1) = 7 total
        assert_eq!(bytes.len(), 7);
        // result = 0
        let result = i32::from_le_bytes([bytes[2], bytes[3], bytes[4], bytes[5]]);
        assert_eq!(result, 0);
        // instant = true → MSB bit set
        assert_eq!(bytes[6], 0x80);
    }

    #[test]
    fn logout_response_delayed_ok() {
        let pkt = LogoutResponse::delayed_ok();
        let bytes = pkt.to_bytes();
        assert_eq!(bytes.len(), 7);
        // instant = false → 0x00
        assert_eq!(bytes[6], 0x00);
    }

    #[test]
    fn logout_complete_empty() {
        let pkt = LogoutComplete;
        let bytes = pkt.to_bytes();
        assert_eq!(bytes.len(), 2); // opcode only
        let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
        assert_eq!(opcode, 0x2684);
    }

    #[test]
    fn logout_cancel_ack_empty() {
        let pkt = LogoutCancelAck;
        let bytes = pkt.to_bytes();
        assert_eq!(bytes.len(), 2); // opcode only
        let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
        assert_eq!(opcode, 0x2685);
    }

    #[test]
    fn set_proficiency_armor() {
        let pkt = SetProficiency::default_armor(1); // Warrior
        let bytes = pkt.to_bytes();
        assert_eq!(bytes.len(), 7);
        // Class byte = 4 (Armor)
        assert_eq!(bytes[6], 4);
        // Mask = 0x5E for warrior (Cloth+Leather+Mail+Plate+Shield)
        let mask = u32::from_le_bytes([bytes[2], bytes[3], bytes[4], bytes[5]]);
        assert_eq!(mask, 0x5E);
    }
}
