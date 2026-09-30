// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! `WorldSession::interaction` sub-state (#1241 F2): moved fields, no logic.

use super::*;

/// NPC interaction: C++ `PlayerInteractionData`, gossip options, vendor stock and the support-
/// feature fixture.
pub(crate) struct InteractionState {
    /// Per-session finite vendor stock state, mirroring Creature::m_vendorItemCounts
    /// until vendor ownership moves into the shared creature model.
    pub(crate) vendor_item_counts: HashMap<(wow_core::ObjectGuid, u32), VendorItemCount>,
    /// Test-only replacement for one resolved `VendorItem` row. Production
    /// always resolves the row through CharacterHandler's WorldDB query.
    #[cfg(test)]
    pub(in crate::session) vendor_buy_item_test_override_like_cpp:
        Option<VendorBuyItemTestOverrideLikeCpp>,
    /// Detached support feature configuration used only by tests.
    #[cfg(test)]
    pub(in crate::session) support_feature_test_fixture_like_cpp: SupportFeatureTestFixtureLikeCpp,

    // ── Player-menu interaction state ─────────────────────────────
    /// The represented subset of C++ `PlayerMenu::InteractionData`.
    ///
    /// `PlayerChoiceId` remains unrepresented until the corresponding
    /// player-choice runtime lands. Gossip options deliberately remain
    /// separate because C++ `InteractionData::Reset` and
    /// `PlayerMenu::ClearMenus` are different operations.
    #[cfg(test)]
    pub(in crate::session) player_interaction_data_like_cpp: PlayerInteractionDataLikeCpp,
    /// Active gossip options for the NPC the player is talking to.
    /// Stored when SMSG_GOSSIP_MESSAGE is sent, used when CMSG_GOSSIP_SELECT_OPTION arrives.
    #[cfg(test)]
    pub(crate) gossip_options: Vec<GossipOptionInfo>,
}
