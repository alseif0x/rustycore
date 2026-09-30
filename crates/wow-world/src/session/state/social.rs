//! Existing Session substate definitions; authority and field contracts are unchanged.

use super::*;

/// Session social and chat policy: Recruit-A-Friend XP limits, chat anti-flood
/// state, and policy values used by chat and group handlers.
#[derive(Default)]
pub(in crate::session) struct SessionSocialLimits {
    /// C++ Recruit-A-Friend XP level gates used by `Player::GetsRecruitAFriendBonus(true)`.
    pub(in crate::session) max_recruit_a_friend_bonus_player_level_like_cpp: u32,
    pub(in crate::session) max_recruit_a_friend_bonus_player_level_difference_like_cpp: u32,
    /// C++ `WorldSession::m_chatFloodData` accumulators.
    pub(in crate::session) chat_flood_data_like_cpp: [ChatFloodThrottleDataLikeCpp; 2],
    /// C++ `CONFIG_ADDON_CHANNEL` represented switch.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(in crate::session) addon_channel_like_cpp: bool,
    /// C++ `CONFIG_CHAT_FAKE_MESSAGE_PREVENTING` represented switch for chat validation.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(in crate::session) chat_fake_message_preventing_like_cpp: bool,
    /// C++ `CONFIG_CHAT_PARTY_RAID_WARNINGS` represented switch.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(in crate::session) party_raid_warnings_like_cpp: bool,
    /// C++ `CONFIG_ALLOW_GM_GROUP` represented switch.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(in crate::session) allow_gm_group_like_cpp: bool,
    /// C++ `CONFIG_ALLOW_TWO_SIDE_INTERACTION_GROUP` represented switch.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(in crate::session) allow_two_side_interaction_group_like_cpp: bool,
    /// C++ `CONFIG_PARTY_LEVEL_REQ` represented gate.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(in crate::session) party_level_req_like_cpp: u32,
    /// C++ `CONFIG_CHAT_STRICT_LINK_CHECKING_KICK` represented switch.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(in crate::session) chat_strict_link_checking_kick_like_cpp: bool,
    /// C++ `CONFIG_CHAT_*_LEVEL_REQ` represented chat level gates.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(in crate::session) chat_level_requirements_like_cpp: ChatLevelRequirementsLikeCpp,
    /// C++ `CONFIG_LISTEN_RANGE_*` represented nearby-chat ranges.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(in crate::session) chat_listen_ranges_like_cpp: ChatListenRangesLikeCpp,
    /// C++ `CONFIG_CHATFLOOD_*` represented chat spam protection.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(in crate::session) chat_flood_config_like_cpp: ChatFloodConfigLikeCpp,
}

/// Addon chat filtering: C++ `WorldSession::_registeredAddonPrefixes` and
/// `_filterAddonMessages`. Read by the chat handlers, which is why the filter
/// keeps crate visibility instead of narrowing to the session tree.
#[derive(Default)]
pub(crate) struct SessionAddonFilter {
    pub(crate) registered_addon_prefixes: Vec<String>,
    pub(crate) filter_addon_messages: bool,
}
