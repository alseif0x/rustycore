//! Feature system packets.
//!
//! Separated from session.rs under #689.

use super::*;

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
