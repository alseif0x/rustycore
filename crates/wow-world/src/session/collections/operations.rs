//! Represented mail, auction and achievement operations.
//!
//! Moved out of the Session root under #632. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(crate) fn replace_owned_player_mails_like_cpp(
        &self,
        mails: Vec<wow_entities::PlayerMailRecord>,
    ) -> bool {
        self.with_owned_player_mut_like_cpp(|player| {
            player.hydrate_mails_like_cpp(mails);
        })
        .is_some()
    }
    pub(crate) fn owned_player_mails_like_cpp(
        &self,
    ) -> Option<Vec<wow_entities::PlayerMailRecord>> {
        self.with_owned_player_like_cpp(|player| player.gameplay_state().mails.clone())
    }
    pub(in crate::session) fn completed_achievement_ids_snapshot_like_cpp(
        &self,
    ) -> Option<HashSet<u32>> {
        let canonical = self.with_owned_player_like_cpp(|player| {
            player
                .gameplay_state()
                .achievements
                .iter()
                .map(|achievement| achievement.achievement_id)
                .collect()
        });
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(self.represented_completed_achievements_like_cpp.clone());
        }
        canonical
    }
    pub(in crate::session) fn replace_completed_achievement_ids_like_cpp(
        &mut self,
        achievement_ids: impl IntoIterator<Item = u32>,
    ) -> bool {
        let achievement_ids: HashSet<_> = achievement_ids
            .into_iter()
            .filter(|achievement_id| *achievement_id != 0)
            .collect();
        let achievements = achievement_ids
            .iter()
            .copied()
            .map(|achievement_id| wow_entities::PlayerAchievementRecord {
                achievement_id,
                completed_at: None,
            })
            .collect::<Vec<_>>();
        let canonical = self
            .with_owned_player_mut_like_cpp(|player| {
                player.hydrate_completed_achievements_like_cpp(achievements);
            })
            .is_some();
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            self.represented_completed_achievements_like_cpp = achievement_ids;
            return true;
        }
        canonical
    }
    pub(crate) fn access_requirement_leader_has_achievement_like_cpp(
        &self,
        achievement_id: u32,
    ) -> bool {
        if achievement_id == 0 {
            return true;
        }

        let Some(player_guid) = self.player_guid else {
            return false;
        };
        let leader_guid = self
            .resolved_group_guid_like_cpp()
            .and_then(|group_guid| self.group_registry.as_ref()?.get(&group_guid))
            .map(|group| group.leader_guid)
            .unwrap_or(player_guid);
        if leader_guid == player_guid {
            return self
                .completed_achievement_ids_snapshot_like_cpp()
                .is_some_and(|achievements| achievements.contains(&achievement_id));
        }

        self.player_registry.as_ref().is_some_and(|registry| {
            registry.connected_player_has_achievement(leader_guid, achievement_id)
        })
    }
    #[cfg_attr(not(test), allow(unused_variables))]
    pub(crate) fn record_represented_auction_place_bid_like_cpp(
        &mut self,
        bid: RepresentedAuctionPlaceBidLikeCpp,
    ) {
        #[cfg(test)]
        self.represented_auction_place_bids_like_cpp.push(bid);
    }
    #[cfg(test)]
    pub(crate) fn represented_auction_place_bids_like_cpp(
        &self,
    ) -> &[RepresentedAuctionPlaceBidLikeCpp] {
        &self.represented_auction_place_bids_like_cpp
    }
}
