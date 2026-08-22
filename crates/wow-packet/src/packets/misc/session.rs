// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Session, account, client-state and hotfix packets.

use super::*;

/// C++ `WorldPackets::Misc::AddonList`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddonList {
    pub addons: Vec<String>,
}

impl ClientPacket for AddonList {
    const OPCODE: ClientOpcodes = ClientOpcodes::AddonList;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let count = pkt.read_uint32()?;
        let mut addons = Vec::new();

        for _ in 0..count {
            if pkt.remaining() == 0 {
                break;
            }

            let name_len = pkt.read_bits(10)? as usize;
            pkt.flush_bits();
            addons.push(pkt.read_string(name_len)?);
        }

        Ok(Self { addons })
    }
}

/// C++ `WorldPackets::Character::LoadingScreenNotify`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoadingScreenNotify {
    pub map_id: u32,
    pub showing: bool,
}

impl ClientPacket for LoadingScreenNotify {
    const OPCODE: ClientOpcodes = ClientOpcodes::LoadingScreenNotify;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            map_id: pkt.read_uint32()?,
            showing: pkt.read_bit()?,
        })
    }
}

/// C++ `WorldPackets::Misc::RandomRollClient`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RandomRollClient {
    pub min: i32,
    pub max: i32,
    pub party_index: Option<u8>,
}

impl ClientPacket for RandomRollClient {
    const OPCODE: ClientOpcodes = ClientOpcodes::RandomRoll;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let has_party_index = pkt.read_bit()?;
        let min = pkt.read_int32()?;
        let max = pkt.read_int32()?;
        let party_index = if has_party_index {
            Some(pkt.read_uint8()?)
        } else {
            None
        };
        Ok(Self {
            min,
            max,
            party_index,
        })
    }
}

/// C++ `WorldPackets::Misc::CloseInteraction`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CloseInteraction {
    pub source_guid: ObjectGuid,
}

impl ClientPacket for CloseInteraction {
    const OPCODE: ClientOpcodes = ClientOpcodes::CloseInteraction;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            source_guid: pkt.read_packed_guid()?,
        })
    }
}

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

// ── UpdateWorldState (SMSG 0x2748) ──────────────────────────────────

/// C++ `WorldPackets::Misc::SetAIAnimKit`: ObjectGuid + uint16 AnimKitID.
pub struct SetAiAnimKit {
    pub unit: ObjectGuid,
    pub anim_kit_id: u16,
}

impl ServerPacket for SetAiAnimKit {
    const OPCODE: ServerOpcodes = ServerOpcodes::SetAiAnimKit;

    fn write(&self, pkt: &mut WorldPacket) {
        for byte in self.unit.to_raw_bytes() {
            pkt.write_uint8(byte);
        }
        pkt.write_uint16(self.anim_kit_id);
    }
}

/// C++ `WorldPackets::Misc::SetMeleeAnimKit`: ObjectGuid + uint16 AnimKitID.
pub struct SetMeleeAnimKit {
    pub unit: ObjectGuid,
    pub anim_kit_id: u16,
}

impl ServerPacket for SetMeleeAnimKit {
    const OPCODE: ServerOpcodes = ServerOpcodes::SetMeleeAnimKit;

    fn write(&self, pkt: &mut WorldPacket) {
        for byte in self.unit.to_raw_bytes() {
            pkt.write_uint8(byte);
        }
        pkt.write_uint16(self.anim_kit_id);
    }
}

// ── UpdateCapturePoint (SMSG 0xbadd) ───────────────────────────────

/// Starts a cinematic sequence for the player.
pub struct TriggerCinematic {
    pub cinematic_id: u32,
    pub conversation_guid: ObjectGuid,
}

impl ServerPacket for TriggerCinematic {
    const OPCODE: ServerOpcodes = ServerOpcodes::TriggerCinematic;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.cinematic_id);
        for byte in self.conversation_guid.to_raw_bytes() {
            pkt.write_uint8(byte);
        }
    }
}

// ── TriggerMovie (SMSG 0x26cb) ──────────────────────────────────────

/// Feature system status sent AFTER entering the world.
/// This is the in-game variant; for the character select screen use
/// [`FeatureSystemStatusGlueScreen`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FeatureSystemConfigLikeCpp {
    pub support_tickets_enabled: bool,
    pub support_bugs_enabled: bool,
    pub support_complaints_enabled: bool,
    pub support_suggestions_enabled: bool,
    pub char_undelete_enabled: bool,
    pub bpay_store_enabled: bool,
}

impl Default for FeatureSystemConfigLikeCpp {
    fn default() -> Self {
        Self {
            support_tickets_enabled: false,
            support_bugs_enabled: false,
            support_complaints_enabled: false,
            support_suggestions_enabled: false,
            char_undelete_enabled: false,
            bpay_store_enabled: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FeatureSystemStatus {
    pub cfg_realm_id: u32,
    pub cfg_realm_rec_id: i32,
    pub config: FeatureSystemConfigLikeCpp,
    pub is_muted: bool,
}

impl FeatureSystemStatus {
    pub fn default_wotlk() -> Self {
        Self::from_config_like_cpp(FeatureSystemConfigLikeCpp::default(), false)
    }

    pub fn from_config_like_cpp(config: FeatureSystemConfigLikeCpp, is_muted: bool) -> Self {
        Self {
            cfg_realm_id: 2,
            cfg_realm_rec_id: 0,
            config,
            is_muted,
        }
    }
}

impl ServerPacket for FeatureSystemStatus {
    const OPCODE: ServerOpcodes = ServerOpcodes::FeatureSystemStatus;

    fn write(&self, pkt: &mut WorldPacket) {
        // C++ `WorldSession::SendFeatureSystemStatus` dummy/config defaults.
        pkt.write_uint8(2); // ComplaintStatus
        pkt.write_uint32(self.cfg_realm_id); // CfgRealmID
        pkt.write_int32(self.cfg_realm_rec_id); // CfgRealmRecID

        // RAFSystem (5 fields)
        pkt.write_uint32(0); // RAFSystem.MaxRecruits
        pkt.write_uint32(0); // RAFSystem.MaxRecruitMonths
        pkt.write_uint32(0); // RAFSystem.MaxRecruitmentUses
        pkt.write_uint32(0); // RAFSystem.DaysInCycle
        pkt.write_uint32(0); // RAFSystem.Unknown1007

        // Token/Kiosk/Store
        pkt.write_uint32(300); // TokenPollTimeSeconds
        pkt.write_uint32(0); // KioskSessionMinutes
        pkt.write_int64(0); // TokenBalanceAmount
        pkt.write_uint32(0); // BpayStoreProductDeliveryDelay
        pkt.write_uint32(0); // ClubsPresenceUpdateTimer
        pkt.write_uint32(0); // HiddenUIClubsPresenceUpdateTimer

        // Season/Rules/Query
        pkt.write_int32(0); // ActiveSeason
        pkt.write_int32(0); // GameRuleValues.Count
        pkt.write_int16(50); // MaxPlayerNameQueriesPerPacket
        pkt.write_int16(600); // PlayerNameQueryTelemetryInterval
        pkt.write_uint32(10); // PlayerNameQueryInterval (seconds)

        // GameRuleValues (empty, count=0)

        // Bit flags in C++ `FeatureSystemStatus::Write` order.
        pkt.write_bit(false); // VoiceEnabled
        pkt.write_bit(true); // EuropaTicketSystemStatus.HasValue
        pkt.write_bit(self.config.bpay_store_enabled); // BpayStoreEnabled
        pkt.write_bit(false); // BpayStoreAvailable
        pkt.write_bit(false); // BpayStoreDisabledByParentalControls
        pkt.write_bit(false); // ItemRestorationButtonEnabled
        pkt.write_bit(false); // BrowserEnabled
        pkt.write_bit(false); // SessionAlert.HasValue
        pkt.write_bit(false); // RAFSystem.Enabled
        pkt.write_bit(false); // RAFSystem.RecruitingEnabled
        pkt.write_bit(self.config.char_undelete_enabled); // CharUndeleteEnabled
        pkt.write_bit(false); // RestrictedAccount
        pkt.write_bit(false); // CommerceSystemEnabled
        pkt.write_bit(true); // TutorialsEnabled
        pkt.write_bit(false); // Unk67
        pkt.write_bit(false); // WillKickFromWorld
        pkt.write_bit(false); // KioskModeEnabled
        pkt.write_bit(false); // CompetitiveModeEnabled
        pkt.write_bit(false); // TokenBalanceEnabled
        pkt.write_bit(true); // WarModeFeatureEnabled
        pkt.write_bit(false); // ClubsEnabled
        pkt.write_bit(false); // ClubsBattleNetClubTypeAllowed
        pkt.write_bit(false); // ClubsCharacterClubTypeAllowed
        pkt.write_bit(false); // ClubsPresenceUpdateEnabled
        pkt.write_bit(false); // VoiceChatDisabledByParentalControl
        pkt.write_bit(false); // VoiceChatMutedByParentalControl
        pkt.write_bit(false); // QuestSessionEnabled
        pkt.write_bit(self.is_muted); // IsMuted
        pkt.write_bit(false); // ClubFinderEnabled
        pkt.write_bit(false); // Unknown901CheckoutRelated
        pkt.write_bit(false); // TextToSpeechFeatureEnabled
        pkt.write_bit(false); // ChatDisabledByDefault
        pkt.write_bit(false); // ChatDisabledByPlayer
        pkt.write_bit(false); // LFGListCustomRequiresAuthenticator
        pkt.write_bit(false); // AddonsDisabled
        pkt.write_bit(false); // WarGamesEnabled
        pkt.write_bit(false); // ContentTrackingEnabled
        pkt.write_bit(false); // IsSellAllJunkEnabled
        pkt.write_bit(true); // IsGroupFinderEnabled
        pkt.write_bit(true); // IsLFDEnabled
        pkt.write_bit(true); // IsLFREnabled
        pkt.write_bit(true); // IsPremadeGroupEnabled
        pkt.flush_bits();

        // ── QuickJoinConfig ──
        pkt.write_bit(false); // QuickJoinConfig.ToastsDisabled
        pkt.write_float(0.0); // QuickJoinConfig.ToastDuration
        pkt.write_float(0.0); // QuickJoinConfig.DelayDuration
        pkt.write_float(0.0); // QuickJoinConfig.QueueMultiplier
        pkt.write_float(0.0); // QuickJoinConfig.PlayerMultiplier
        pkt.write_float(0.0); // QuickJoinConfig.PlayerFriendValue
        pkt.write_float(0.0); // QuickJoinConfig.PlayerGuildValue
        pkt.write_float(0.0); // QuickJoinConfig.ThrottleInitialThreshold
        pkt.write_float(0.0); // QuickJoinConfig.ThrottleDecayTime
        pkt.write_float(0.0); // QuickJoinConfig.ThrottlePrioritySpike
        pkt.write_float(0.0); // QuickJoinConfig.ThrottleMinThreshold
        pkt.write_float(0.0); // QuickJoinConfig.ThrottlePvPPriorityNormal
        pkt.write_float(0.0); // QuickJoinConfig.ThrottlePvPPriorityLow
        pkt.write_float(0.0); // QuickJoinConfig.ThrottlePvPHonorThreshold
        pkt.write_float(0.0); // QuickJoinConfig.ThrottleLfgListPriorityDefault
        pkt.write_float(0.0); // QuickJoinConfig.ThrottleLfgListPriorityAbove
        pkt.write_float(0.0); // QuickJoinConfig.ThrottleLfgListPriorityBelow
        pkt.write_float(0.0); // QuickJoinConfig.ThrottleLfgListIlvlScalingAbove
        pkt.write_float(0.0); // QuickJoinConfig.ThrottleLfgListIlvlScalingBelow
        pkt.write_float(0.0); // QuickJoinConfig.ThrottleRfPriorityAbove
        pkt.write_float(0.0); // QuickJoinConfig.ThrottleRfIlvlScalingAbove
        pkt.write_float(0.0); // QuickJoinConfig.ThrottleDfMaxItemLevel
        pkt.write_float(0.0); // QuickJoinConfig.ThrottleDfBestPriority

        // SessionAlert (optional — not present, bit was false)

        // Squelch
        pkt.write_bit(false); // Squelch.IsSquelched
        pkt.write_packed_guid(&ObjectGuid::EMPTY); // Squelch.BnetAccountGuid
        pkt.write_packed_guid(&ObjectGuid::EMPTY); // Squelch.GuildGuid

        // EuropaTicketSystemStatus (present in C++ login defaults).
        pkt.write_bit(self.config.support_tickets_enabled); // TicketsEnabled
        pkt.write_bit(self.config.support_bugs_enabled); // BugsEnabled
        pkt.write_bit(self.config.support_complaints_enabled); // ComplaintsEnabled
        pkt.write_bit(self.config.support_suggestions_enabled); // SuggestionsEnabled
        pkt.write_uint32(10); // ThrottleState.MaxTries
        pkt.write_uint32(60000); // ThrottleState.PerMilliseconds
        pkt.write_uint32(1); // ThrottleState.TryCount
        pkt.write_uint32(111111); // ThrottleState.LastResetTimeBeforeNow
    }
}

// ── FeatureSystemStatusGlueScreen (SMSG 0x25c0) — CHARACTER SELECT ──

/// Feature system status for the glue screen (character select).
/// This is the version sent during session init, BEFORE entering the world.
/// Different opcode and format from [`FeatureSystemStatus`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FeatureSystemStatusGlueScreen {
    pub max_characters_per_realm: i32,
    pub maximum_expansion_level: i32,
    pub config: FeatureSystemConfigLikeCpp,
}

impl FeatureSystemStatusGlueScreen {
    /// Default values matching C++ SendFeatureSystemStatusGlueScreen defaults.
    pub fn default_wotlk() -> Self {
        Self::from_config_like_cpp(FeatureSystemConfigLikeCpp::default(), 60, 2)
    }

    pub fn from_config_like_cpp(
        config: FeatureSystemConfigLikeCpp,
        max_characters_per_realm: i32,
        maximum_expansion_level: i32,
    ) -> Self {
        Self {
            max_characters_per_realm,
            maximum_expansion_level,
            config,
        }
    }
}

impl ServerPacket for FeatureSystemStatusGlueScreen {
    const OPCODE: ServerOpcodes = ServerOpcodes::FeatureSystemStatusGlueScreen;

    fn write(&self, pkt: &mut WorldPacket) {
        // ── 27 bit flags (exact C++ order) ──
        pkt.write_bit(self.config.bpay_store_enabled); // BpayStoreEnabled
        pkt.write_bit(false); // BpayStoreAvailable
        pkt.write_bit(false); // BpayStoreDisabledByParentalControls
        pkt.write_bit(self.config.char_undelete_enabled); // CharUndeleteEnabled
        pkt.write_bit(false); // CommerceSystemEnabled
        pkt.write_bit(false); // Unk14
        pkt.write_bit(false); // WillKickFromWorld
        pkt.write_bit(false); // IsExpansionPreorderInStore

        pkt.write_bit(false); // KioskModeEnabled
        pkt.write_bit(false); // CompetitiveModeEnabled
        pkt.write_bit(false); // unused 10.0.2
        pkt.write_bit(false); // TrialBoostEnabled
        pkt.write_bit(false); // TokenBalanceEnabled
        pkt.write_bit(false); // LiveRegionCharacterListEnabled
        pkt.write_bit(false); // LiveRegionCharacterCopyEnabled
        pkt.write_bit(false); // LiveRegionAccountCopyEnabled

        pkt.write_bit(false); // LiveRegionKeyBindingsCopyEnabled
        pkt.write_bit(false); // Unknown901CheckoutRelated
        pkt.write_bit(false); // unused 10.0.2
        pkt.write_bit(true); // EuropaTicketSystemStatus.HasValue (C# sets this!)
        pkt.write_bit(false); // unused 10.0.2
        pkt.write_bit(false); // LaunchETA.HasValue
        pkt.write_bit(false); // AddonsDisabled
        pkt.write_bit(false); // Unused1000

        pkt.write_bit(false); // AccountSaveDataExportEnabled
        pkt.write_bit(false); // AccountLockedByExport
        pkt.write_bit(false); // RealmHiddenAlert (not empty = false)

        // No RealmHiddenAlert bits (it's empty)
        pkt.flush_bits();

        // ── EuropaTicketSystemStatus (present — bit was true) ──
        // EuropaTicketConfig.Write():
        //   4 bits (TicketsEnabled, BugsEnabled, ComplaintsEnabled, SuggestionsEnabled)
        //   then SavedThrottleObjectState (4 × u32)
        pkt.write_bit(self.config.support_tickets_enabled); // TicketsEnabled
        pkt.write_bit(self.config.support_bugs_enabled); // BugsEnabled
        pkt.write_bit(self.config.support_complaints_enabled); // ComplaintsEnabled
        pkt.write_bit(self.config.support_suggestions_enabled); // SuggestionsEnabled
        // SavedThrottleObjectState — C++ sets these dummy values:
        pkt.write_uint32(10); // MaxTries
        pkt.write_uint32(60000); // PerMilliseconds
        pkt.write_uint32(1); // TryCount
        pkt.write_uint32(111111); // LastResetTimeBeforeNow

        // ── Sequential numeric fields (exact C# order) ──
        pkt.write_uint32(0); // TokenPollTimeSeconds
        pkt.write_uint32(0); // KioskSessionMinutes
        pkt.write_int64(0); // TokenBalanceAmount
        pkt.write_int32(self.max_characters_per_realm); // MaxCharactersPerRealm
        pkt.write_int32(0); // LiveRegionCharacterCopySourceRegions.Count
        pkt.write_uint32(0); // BpayStoreProductDeliveryDelay
        pkt.write_int32(0); // ActiveCharacterUpgradeBoostType
        pkt.write_int32(0); // ActiveClassTrialBoostType
        pkt.write_int32(0); // MinimumExpansionLevel (Classic=0)
        pkt.write_int32(self.maximum_expansion_level); // MaximumExpansionLevel
        pkt.write_int32(0); // ActiveSeason
        pkt.write_int32(0); // GameRuleValues.Count
        pkt.write_int16(50); // MaxPlayerNameQueriesPerPacket
        pkt.write_int16(600); // PlayerNameQueryTelemetryInterval (C# default=600)
        pkt.write_uint32(10); // PlayerNameQueryInterval (C# default=10 seconds)
        pkt.write_int32(0); // DebugTimeEvents.Count
        pkt.write_int32(0); // Unused1007

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

/// C++ `DB2Manager::HotfixId`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HotfixId {
    pub push_id: i32,
    pub unique_id: u32,
}

/// Available hotfixes sent during session init.
pub struct AvailableHotfixes {
    pub virtual_realm_address: u32,
    pub hotfixes: Vec<HotfixId>,
}

impl ServerPacket for AvailableHotfixes {
    const OPCODE: ServerOpcodes = ServerOpcodes::AvailableHotfixes;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.virtual_realm_address);
        pkt.write_uint32(self.hotfixes.len() as u32);
        for hotfix_id in &self.hotfixes {
            pkt.write_int32(hotfix_id.push_id);
            pkt.write_uint32(hotfix_id.unique_id);
        }
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

/// Response to ServerTimeOffsetRequest. Sends the current realm time.
pub struct ServerTimeOffset {
    pub time: i64,
}

impl ServerTimeOffset {
    /// Current time.
    pub fn now() -> Self {
        Self {
            time: unix_timestamp(),
        }
    }
}

impl ServerPacket for ServerTimeOffset {
    const OPCODE: ServerOpcodes = ServerOpcodes::ServerTimeOffset;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int64(self.time);
    }
}

// ── InitWorldStates (SMSG 0x2746) ─────────────────────────────────

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
        let sequence_index = packet.read_uint32()?;
        let client_time = packet.read_uint32()?;
        Ok(Self {
            client_time,
            sequence_index,
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

// ── AuraUpdate (SMSG 0x2c1f) ─────────────────────────────────────────

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
        Ok(Self {
            client_build,
            data_build,
            hotfixes,
        })
    }
}

// ── HotfixConnect (SMSG 0x2911) ───────────────────────────────────

/// One C++ `WorldPackets::Hotfix::HotfixConnect::HotfixData` header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HotfixConnectData {
    pub id: HotfixId,
    pub table_hash: u32,
    pub record_id: i32,
    pub size: u32,
    pub status: u8,
}

/// Response to [`HotfixRequest`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HotfixConnect {
    pub hotfixes: Vec<HotfixConnectData>,
    pub content: Vec<u8>,
}

impl HotfixConnect {
    pub fn empty() -> Self {
        Self::default()
    }
}

impl ServerPacket for HotfixConnect {
    const OPCODE: ServerOpcodes = ServerOpcodes::HotfixConnect;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.hotfixes.len() as u32);
        for hotfix in &self.hotfixes {
            pkt.write_int32(hotfix.id.push_id);
            pkt.write_uint32(hotfix.id.unique_id);
            pkt.write_uint32(hotfix.table_hash);
            pkt.write_int32(hotfix.record_id);
            pkt.write_uint32(hotfix.size);
            pkt.write_bits(u32::from(hotfix.status), 3);
            pkt.flush_bits();
        }

        pkt.write_uint32(self.content.len() as u32);
        if !self.content.is_empty() {
            pkt.write_bytes(&self.content);
        }
    }
}

// ── MoveSetActiveMover (SMSG 0x2dd5) ───────────────────────────────

/// Sent on the instance connection after TransferPending.
/// Tells the client to pause movement processing during map transfer.
/// C# ref: MovementPackets.SuspendToken (ConnectionType.Instance)
pub struct SuspendToken {
    /// Movement counter (sequence index). Send 1 for simple teleports.
    pub sequence_index: u32,
    /// 1 = Normal teleport, 2 = Seamless teleport.
    pub reason: u32,
}

impl ServerPacket for SuspendToken {
    const OPCODE: ServerOpcodes = ServerOpcodes::SuspendToken;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.sequence_index);
        pkt.write_bits(self.reason, 2);
        pkt.flush_bits();
    }
}

// ── ResumeToken (SMSG 0x25a9) ────────────────────────────────────────

/// Sent after WorldPortResponse to resume movement processing.
/// C# ref: MovementPackets.ResumeToken (ConnectionType.Instance)
pub struct ResumeToken {
    pub sequence_index: u32,
    /// 1 = Normal, 2 = Seamless.
    pub reason: u32,
}

impl ServerPacket for ResumeToken {
    const OPCODE: ServerOpcodes = ServerOpcodes::ResumeToken;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.sequence_index);
        pkt.write_bits(self.reason, 2);
        pkt.flush_bits();
    }
}

// ── NewWorld (SMSG 0x2594) ────────────────────────────────────────────

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

/// SMSG_NPC_INTERACTION_OPEN_RESULT — opens an NPC interaction UI on client.
/// C++ `WorldPackets::NPC::NPCInteractionOpenResult::Write`
/// (`Server/Packets/NPCPackets.cpp:96-104`).
/// PlayerInteractionType values: Banker=8, Binder=20, Auctioneer=21,
/// StableMaster=22, GuildTabardVendor=14, TaxiNode=6, Merchant=5, Trainer=7.
pub struct NpcInteractionOpenResult {
    pub npc: wow_core::ObjectGuid,
    pub interaction_type: i32,
    pub success: bool,
}

impl NpcInteractionOpenResult {
    pub fn new(npc: wow_core::ObjectGuid, interaction_type: i32) -> Self {
        Self {
            npc,
            interaction_type,
            success: true,
        }
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

/// C++ `WorldPackets::Token::CommerceTokenGetLog`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommerceTokenGetLog {
    pub unk_int: u32,
}

impl ClientPacket for CommerceTokenGetLog {
    const OPCODE: ClientOpcodes = ClientOpcodes::CommerceTokenGetLog;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            unk_int: pkt.read_uint32()?,
        })
    }
}

/// C++ `WorldPackets::Token::CommerceTokenGetLogResponse`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CommerceTokenGetLogResponse {
    pub unk_int: u32,
    pub result: u32,
    /// Auctionable token rows are unimplemented in this C++ branch too; the
    /// handler sends a success response with an empty list.
    pub auctionable_token_count: u32,
}

impl CommerceTokenGetLogResponse {
    pub fn success_empty(unk_int: u32) -> Self {
        Self {
            unk_int,
            result: TOKEN_RESULT_SUCCESS_LIKE_CPP,
            auctionable_token_count: 0,
        }
    }
}

impl ServerPacket for CommerceTokenGetLogResponse {
    const OPCODE: ServerOpcodes = ServerOpcodes::CommerceTokenGetLogResponse;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.unk_int);
        pkt.write_uint32(self.result);
        pkt.write_uint32(self.auctionable_token_count);
    }
}

// ── RatedPvpInfo ─────────────────────────────────────────────────────────────

/// Floating text "+XP" on screen when player earns experience.
/// C++ `WorldPackets::Character::LogXPGain::Write`.
pub struct LogXpGain {
    pub victim: ObjectGuid,
    pub original: i32, // base XP plus represented bonuses
    pub reason: u8,    // 0=Kill, 1=NoKill(quest/explore)
    pub amount: i32,   // base XP amount
    pub group_bonus: f32,
}

impl ServerPacket for LogXpGain {
    const OPCODE: ServerOpcodes = ServerOpcodes::LogXpGain;
    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.victim);
        pkt.write_int32(self.original);
        pkt.write_uint8(self.reason);
        pkt.write_int32(self.amount);
        pkt.write_float(self.group_bonus);
    }
}

// ── SMSG_EXPLORATION_EXPERIENCE ─────────────────────────────────────────────
