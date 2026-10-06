use std::collections::HashMap;

#[cfg(any(test, feature = "test-fixtures"))]
use crate::SupportFeatureTestFixtureLikeCpp;
use wow_core::ObjectGuid;
#[cfg(any(test, feature = "test-fixtures"))]
use wow_entities::PlayerGossipOptionLikeCpp;
#[cfg(any(test, feature = "test-fixtures"))]
use wow_entities::PlayerInteractionDataLikeCpp;

/// Current finite stock for a vendor item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VendorItemCount {
    pub count: u32,
    pub last_increment_time: u64,
}

#[cfg(any(test, feature = "test-fixtures"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VendorBuyItemTestOverrideLikeCpp {
    pub item_id: u32,
    pub item_type: i32,
    pub max_count: u32,
    pub incr_time: u32,
    pub player_condition_id: u32,
    pub has_vendor_conditions: bool,
    pub extended_cost: u32,
    pub buy_price: u64,
    pub max_durability: u32,
    pub buy_count: u32,
}

/// NPC interaction: C++ `PlayerInteractionData`, gossip options, vendor stock and the support-
/// feature fixture.
pub struct InteractionState {
    /// Per-session finite vendor stock state, mirroring Creature::m_vendorItemCounts
    /// until vendor ownership moves into the shared creature model.
    pub(crate) vendor_item_counts: HashMap<(ObjectGuid, u32), VendorItemCount>,
    /// Test-only replacement for one resolved `VendorItem` row. Production
    /// always resolves the row through CharacterHandler's WorldDB query.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) vendor_buy_item_test_override_like_cpp: Option<VendorBuyItemTestOverrideLikeCpp>,
    /// Detached support feature configuration used only by tests.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) support_feature_test_fixture_like_cpp: SupportFeatureTestFixtureLikeCpp,

    // ── Player-menu interaction state ─────────────────────────────
    /// The represented subset of C++ `PlayerMenu::InteractionData`.
    ///
    /// `PlayerChoiceId` remains unrepresented until the corresponding
    /// player-choice runtime lands. Gossip options deliberately remain
    /// separate because C++ `InteractionData::Reset` and
    /// `PlayerMenu::ClearMenus` are different operations.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) player_interaction_data_like_cpp: PlayerInteractionDataLikeCpp,
    /// Active gossip options for the NPC the player is talking to.
    /// Stored when SMSG_GOSSIP_MESSAGE is sent, used when CMSG_GOSSIP_SELECT_OPTION arrives.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) gossip_options: Vec<PlayerGossipOptionLikeCpp>,
}

impl InteractionState {
    pub fn new_like_cpp() -> Self {
        Self {
            vendor_item_counts: HashMap::new(),
            #[cfg(any(test, feature = "test-fixtures"))]
            vendor_buy_item_test_override_like_cpp: None,
            #[cfg(any(test, feature = "test-fixtures"))]
            support_feature_test_fixture_like_cpp: SupportFeatureTestFixtureLikeCpp::default(),
            #[cfg(any(test, feature = "test-fixtures"))]
            player_interaction_data_like_cpp: PlayerInteractionDataLikeCpp::default(),
            #[cfg(any(test, feature = "test-fixtures"))]
            gossip_options: Vec::new(),
        }
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_vendor_buy_item_test_override_like_cpp(
        &mut self,
        item: VendorBuyItemTestOverrideLikeCpp,
    ) {
        self.vendor_buy_item_test_override_like_cpp = Some(item);
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn vendor_buy_item_test_override_like_cpp(
        &self,
    ) -> Option<VendorBuyItemTestOverrideLikeCpp> {
        self.vendor_buy_item_test_override_like_cpp
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn gossip_options_for_test_like_cpp(&self) -> &[PlayerGossipOptionLikeCpp] {
        &self.gossip_options
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn gossip_options_for_test_mut_like_cpp(&mut self) -> &mut Vec<PlayerGossipOptionLikeCpp> {
        &mut self.gossip_options
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn clear_gossip_options_for_test_like_cpp(&mut self) {
        self.gossip_options.clear();
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn replace_gossip_options_for_test_like_cpp(
        &mut self,
        options: Vec<PlayerGossipOptionLikeCpp>,
    ) {
        self.gossip_options = options;
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn vendor_item_last_increment_time_for_test_like_cpp(
        &mut self,
        vendor_guid: ObjectGuid,
        item_id: u32,
        last_increment_time: u64,
    ) -> bool {
        let Some(count) = self.vendor_item_counts.get_mut(&(vendor_guid, item_id)) else {
            return false;
        };
        count.last_increment_time = last_increment_time;
        true
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn has_vendor_item_count_for_test_like_cpp(
        &self,
        vendor_guid: ObjectGuid,
        item_id: u32,
    ) -> bool {
        self.vendor_item_counts
            .contains_key(&(vendor_guid, item_id))
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn support_feature_values_for_test_like_cpp(&self) -> (bool, bool, bool, bool, bool) {
        let fixture = &self.support_feature_test_fixture_like_cpp;
        (
            fixture.represented_support_enabled_like_cpp,
            fixture.represented_support_tickets_enabled_like_cpp,
            fixture.represented_support_bugs_enabled_like_cpp,
            fixture.represented_support_complaints_enabled_like_cpp,
            fixture.represented_support_suggestions_enabled_like_cpp,
        )
    }
}
