use crate::contracts::ChatFloodThrottleDataLikeCpp;
#[cfg(any(test, feature = "test-fixtures"))]
use crate::contracts::{
    RepresentedCalendarAddEventLikeCpp, RepresentedCalendarCommunityInviteLikeCpp,
    RepresentedCalendarRemoveEventLikeCpp, RepresentedCanDuelSpellCastLikeCpp,
    RepresentedDeclinePetitionLikeCpp, RepresentedDuelAcceptedLikeCpp,
    RepresentedDuelCancelledLikeCpp, RepresentedDuelRequestedLikeCpp,
    RepresentedForceDeselectLikeCpp, RepresentedQueryPetitionLikeCpp,
    RepresentedSignPetitionLikeCpp, RepresentedSilencePartyTalkerLikeCpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
use crate::test_support::{
    CalendarTestFixtureLikeCpp, DuelTestFixtureLikeCpp, GuildTestFixtureLikeCpp,
    TradeTestFixtureLikeCpp,
};

/// Social admission and request evidence owned by a world session.
#[derive(Default)]
pub struct SessionSocialLimits {
    /// C++ Recruit-A-Friend XP level gates used by `Player::GetsRecruitAFriendBonus(true)`.
    pub(crate) max_recruit_a_friend_bonus_player_level_like_cpp: u32,
    pub(crate) max_recruit_a_friend_bonus_player_level_difference_like_cpp: u32,
    /// C++ `Player::m_chatFloodData`, charged by `Player::UpdateSpeakTime`.
    /// Rust retains the session-scoped accumulator pending the F6 lifetime cut.
    pub(crate) chat_flood_data_like_cpp: [ChatFloodThrottleDataLikeCpp; 2],
    pub(crate) addon_filter: SessionAddonFilter,

    // Test-only compatibility for pre-#578 fixtures. Production group
    // membership and Player-owned update sequences live on canonical Player.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) group_guid: Option<u64>,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) represented_subgroup_like_cpp: Option<u8>,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub represented_group_update_sequences_like_cpp:
        [wow_entities::PlayerGroupUpdateSequenceLikeCpp;
            wow_social::group::MAX_GROUP_CATEGORY_LIKE_CPP as usize],

    /// Detached guild membership and invitation state used only by tests.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) guild_test_fixture_like_cpp: GuildTestFixtureLikeCpp,
    /// Calendar request evidence used only by detached Session tests.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) calendar_test_fixture_like_cpp: CalendarTestFixtureLikeCpp,
    /// Detached trade inputs used only by Session tests.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) trade_test_fixture_like_cpp: TradeTestFixtureLikeCpp,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) represented_sign_petitions_like_cpp: Vec<RepresentedSignPetitionLikeCpp>,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) represented_decline_petitions_like_cpp: Vec<RepresentedDeclinePetitionLikeCpp>,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) represented_query_petitions_like_cpp: Vec<RepresentedQueryPetitionLikeCpp>,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) represented_silence_party_talker_like_cpp: Vec<RepresentedSilencePartyTalkerLikeCpp>,
    /// Detached duel state and evidence used only by tests.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) duel_test_fixture_like_cpp: DuelTestFixtureLikeCpp,
}

impl SessionSocialLimits {
    pub fn with_recruit_a_friend_limits_like_cpp(
        max_bonus_player_level: u32,
        max_bonus_player_level_difference: u32,
    ) -> Self {
        Self {
            max_recruit_a_friend_bonus_player_level_like_cpp: max_bonus_player_level,
            max_recruit_a_friend_bonus_player_level_difference_like_cpp:
                max_bonus_player_level_difference,
            chat_flood_data_like_cpp: [ChatFloodThrottleDataLikeCpp::default(); 2],
            addon_filter: SessionAddonFilter::default(),
            #[cfg(any(test, feature = "test-fixtures"))]
            group_guid: None,
            #[cfg(any(test, feature = "test-fixtures"))]
            represented_subgroup_like_cpp: None,
            #[cfg(any(test, feature = "test-fixtures"))]
            represented_group_update_sequences_like_cpp: std::array::from_fn(|_| {
                Default::default()
            }),
            #[cfg(any(test, feature = "test-fixtures"))]
            guild_test_fixture_like_cpp: GuildTestFixtureLikeCpp::default(),
            #[cfg(any(test, feature = "test-fixtures"))]
            calendar_test_fixture_like_cpp: CalendarTestFixtureLikeCpp::default(),
            #[cfg(any(test, feature = "test-fixtures"))]
            trade_test_fixture_like_cpp: TradeTestFixtureLikeCpp::default(),
            #[cfg(any(test, feature = "test-fixtures"))]
            represented_sign_petitions_like_cpp: Vec::new(),
            #[cfg(any(test, feature = "test-fixtures"))]
            represented_decline_petitions_like_cpp: Vec::new(),
            #[cfg(any(test, feature = "test-fixtures"))]
            represented_query_petitions_like_cpp: Vec::new(),
            #[cfg(any(test, feature = "test-fixtures"))]
            represented_silence_party_talker_like_cpp: Vec::new(),
            #[cfg(any(test, feature = "test-fixtures"))]
            duel_test_fixture_like_cpp: DuelTestFixtureLikeCpp::default(),
        }
    }

    pub fn recruit_a_friend_xp_limits_like_cpp(&self) -> (u32, u32) {
        (
            self.max_recruit_a_friend_bonus_player_level_like_cpp,
            self.max_recruit_a_friend_bonus_player_level_difference_like_cpp,
        )
    }

    pub fn set_recruit_a_friend_xp_limits_like_cpp(
        &mut self,
        max_bonus_player_level: u32,
        max_bonus_player_level_difference: u32,
    ) {
        self.max_recruit_a_friend_bonus_player_level_like_cpp = max_bonus_player_level;
        self.max_recruit_a_friend_bonus_player_level_difference_like_cpp =
            max_bonus_player_level_difference;
    }

    pub fn register_addon_prefixes_like_cpp(
        &mut self,
        prefixes: Vec<String>,
        max_prefixes: usize,
    ) -> (usize, bool) {
        self.addon_filter.registered_addon_prefixes.extend(prefixes);
        self.addon_filter.filter_addon_messages =
            self.addon_filter.registered_addon_prefixes.len() <= max_prefixes;
        (
            self.addon_filter.registered_addon_prefixes.len(),
            self.addon_filter.filter_addon_messages,
        )
    }

    pub fn clear_registered_addon_prefixes_like_cpp(&mut self) {
        self.addon_filter.registered_addon_prefixes.clear();
    }

    pub fn is_addon_registered_like_cpp(&self, prefix: &str) -> bool {
        // C++ WorldSession::IsAddonRegistered: if the registration filter is
        // disabled (initial state or softcap exceeded), all prefixes pass.
        if !self.addon_filter.filter_addon_messages {
            return true;
        }

        !self.addon_filter.registered_addon_prefixes.is_empty()
            && self
                .addon_filter
                .registered_addon_prefixes
                .iter()
                .any(|registered| registered == prefix)
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_group_guid_for_test_like_cpp(&mut self, group_guid: Option<u64>) {
        self.group_guid = group_guid;
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn group_guid_for_test_like_cpp(&self) -> Option<u64> {
        self.group_guid
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_addon_filter_for_test_like_cpp(
        &mut self,
        registered_prefixes: Vec<String>,
        filter_addon_messages: bool,
    ) {
        self.addon_filter.registered_addon_prefixes = registered_prefixes;
        self.addon_filter.filter_addon_messages = filter_addon_messages;
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn registered_addon_prefixes_for_test_like_cpp(&self) -> &[String] {
        &self.addon_filter.registered_addon_prefixes
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn addon_message_filter_enabled_for_test_like_cpp(&self) -> bool {
        self.addon_filter.filter_addon_messages
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn record_represented_sign_petition_for_test_like_cpp(
        &mut self,
        petition: RepresentedSignPetitionLikeCpp,
    ) {
        self.represented_sign_petitions_like_cpp.push(petition);
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_sign_petitions_for_test_like_cpp(
        &self,
    ) -> &[RepresentedSignPetitionLikeCpp] {
        &self.represented_sign_petitions_like_cpp
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn record_represented_decline_petition_for_test_like_cpp(
        &mut self,
        petition: RepresentedDeclinePetitionLikeCpp,
    ) {
        self.represented_decline_petitions_like_cpp.push(petition);
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_decline_petitions_for_test_like_cpp(
        &self,
    ) -> &[RepresentedDeclinePetitionLikeCpp] {
        &self.represented_decline_petitions_like_cpp
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn record_represented_query_petition_for_test_like_cpp(
        &mut self,
        petition: RepresentedQueryPetitionLikeCpp,
    ) {
        self.represented_query_petitions_like_cpp.push(petition);
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_query_petitions_for_test_like_cpp(
        &self,
    ) -> &[RepresentedQueryPetitionLikeCpp] {
        &self.represented_query_petitions_like_cpp
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn record_calendar_community_invite_for_test_like_cpp(
        &mut self,
        invite: RepresentedCalendarCommunityInviteLikeCpp,
    ) {
        self.calendar_test_fixture_like_cpp
            .represented_calendar_community_invites_like_cpp
            .push(invite);
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn record_calendar_add_event_for_test_like_cpp(
        &mut self,
        event: RepresentedCalendarAddEventLikeCpp,
    ) {
        self.calendar_test_fixture_like_cpp
            .represented_calendar_add_events_like_cpp
            .push(event);
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn record_calendar_remove_event_for_test_like_cpp(&mut self, event_id: u64) {
        self.calendar_test_fixture_like_cpp
            .represented_calendar_remove_events_like_cpp
            .push(RepresentedCalendarRemoveEventLikeCpp { event_id });
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_calendar_remove_events_for_test_like_cpp(
        &self,
    ) -> &[RepresentedCalendarRemoveEventLikeCpp] {
        &self
            .calendar_test_fixture_like_cpp
            .represented_calendar_remove_events_like_cpp
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_duel_requests_for_test_like_cpp(
        &self,
    ) -> &[RepresentedDuelRequestedLikeCpp] {
        &self
            .duel_test_fixture_like_cpp
            .represented_duel_requests_like_cpp
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_can_duel_spell_casts_for_test_like_cpp(
        &self,
    ) -> &[RepresentedCanDuelSpellCastLikeCpp] {
        &self
            .duel_test_fixture_like_cpp
            .represented_can_duel_spell_casts_like_cpp
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn record_represented_force_deselect_for_test_like_cpp(
        &mut self,
        force_deselect: RepresentedForceDeselectLikeCpp,
    ) {
        self.duel_test_fixture_like_cpp
            .represented_force_deselects_like_cpp
            .push(force_deselect);
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_force_deselects_for_test_like_cpp(
        &self,
    ) -> &[RepresentedForceDeselectLikeCpp] {
        &self
            .duel_test_fixture_like_cpp
            .represented_force_deselects_like_cpp
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn clear_represented_guild_identity_for_test_like_cpp(&mut self) {
        self.guild_test_fixture_like_cpp
            .represented_guild_id_like_cpp = 0;
        self.guild_test_fixture_like_cpp
            .represented_guild_id_authority_complete_like_cpp = false;
    }
}

/// Addon chat filtering: C++ `WorldSession::_registeredAddonPrefixes` and
/// `_filterAddonMessages`. The state is private to this owner crate.
#[derive(Default)]
pub(crate) struct SessionAddonFilter {
    pub(crate) registered_addon_prefixes: Vec<String>,
    pub(crate) filter_addon_messages: bool,
}
