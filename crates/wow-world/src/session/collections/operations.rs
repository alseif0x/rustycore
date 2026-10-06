//! Represented mail, auction and achievement operations.
//!
//! Moved out of the Session root under #632. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(crate) fn access_requirement_leader_has_achievement_like_cpp(
        &self,
        achievement_id: u32,
    ) -> bool {
        if achievement_id == 0 {
            return true;
        }

        let Some(player_guid) = self.core.player_guid else {
            return false;
        };
        let leader_guid = self
            .resolved_group_guid_like_cpp()
            .and_then(|group_guid| {
                self.core
                    .directory
                    .group_registry
                    .as_ref()?
                    .get(&group_guid)
            })
            .map(|group| group.leader_guid)
            .unwrap_or(player_guid);
        if leader_guid == player_guid {
            return crate::session::hub_ref(self)
                .completed_achievement_ids_snapshot_like_cpp()
                .is_some_and(|achievements| achievements.contains(&achievement_id));
        }

        self.core.player_registry.as_ref().is_some_and(|registry| {
            registry.connected_player_has_achievement(leader_guid, achievement_id)
        })
    }
    #[cfg_attr(not(test), allow(unused_variables))]
    pub(crate) fn record_represented_auction_place_bid_like_cpp(
        &mut self,
        bid: RepresentedAuctionPlaceBidLikeCpp,
    ) {
        #[cfg(test)]
        self.inventory
            .record_represented_auction_place_bid_like_cpp(bid);
    }
    #[cfg(test)]
    pub(crate) fn represented_auction_place_bids_like_cpp(
        &self,
    ) -> &[RepresentedAuctionPlaceBidLikeCpp] {
        self.inventory.represented_auction_place_bids_like_cpp()
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/collections/operations/f3_shims.rs"]
mod f3_shims;
