//! Represented friend and ignore lists.
//!
//! Moved out of the Session root under #615. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub fn set_friendship_rep_reaction_store(&mut self, store: Arc<FriendshipRepReactionStore>) {
        self.friendship_rep_reaction_store = Some(store);
    }
    pub(crate) fn friendship_rep_reaction_store(&self) -> Option<&Arc<FriendshipRepReactionStore>> {
        self.friendship_rep_reaction_store.as_ref()
    }
    pub(in crate::session) fn represented_recruit_a_friend_xp_rest_state_applies_like_cpp(
        &self,
    ) -> bool {
        self.gets_recruit_a_friend_xp_bonus_like_cpp()
    }
    pub(in crate::session) fn gets_recruit_a_friend_xp_bonus_like_cpp(&self) -> bool {
        self.gets_recruit_a_friend_bonus_like_cpp(true)
    }
    pub(in crate::session) fn gets_recruit_a_friend_bonus_like_cpp(&self, for_xp: bool) -> bool {
        // C++ `WorldObject::IsInMap` requires both players to be in world.
        // In particular, offline rest accrual runs during LoadFromDB before
        // `AddPlayerToMap` and must not normalize the state as RAF-linked.
        if self.state != SessionState::LoggedIn {
            return false;
        }
        let player_level = u32::from(self.player_level_like_cpp());
        if for_xp && player_level > self.max_recruit_a_friend_bonus_player_level_like_cpp {
            return false;
        }
        let (Some(player_guid), Some(group_guid), Some(group_registry), Some(player_registry)) = (
            self.player_guid(),
            self.resolved_group_guid_like_cpp(),
            self.group_registry.as_ref(),
            self.player_registry.as_ref(),
        ) else {
            return false;
        };
        let Some(group) = group_registry.get(&group_guid) else {
            return false;
        };
        if !group.members.contains(&player_guid) {
            return false;
        }
        let group_members = group.members.clone();
        drop(group);

        let Some(player_position) = self.player_position_like_cpp() else {
            return false;
        };
        let player_map_id = self.player_map_id_like_cpp();
        let player_instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        let max_distance = self.reputation_rates_like_cpp().recruit_a_friend_distance;

        for member_guid in group_members {
            if member_guid == player_guid {
                continue;
            }
            let Some(member) = player_registry.group_presence(member_guid) else {
                continue;
            };
            if member.map_id != player_map_id {
                continue;
            }
            if member.instance_id != player_instance_id {
                continue;
            }
            if !member.is_in_world {
                continue;
            }
            // C++ measures a dead member from their corpse. The shared registry
            // does not yet publish corpse location, so fail closed instead of
            // granting RAF from a stale live-player position.
            if !member.is_alive {
                continue;
            }
            if member.position.distance(&player_position) > max_distance {
                continue;
            }
            if for_xp {
                let member_level = u32::from(member.level);
                if member_level > self.max_recruit_a_friend_bonus_player_level_like_cpp {
                    continue;
                }
                if member_level < player_level
                    && player_level - member_level
                        > self.max_recruit_a_friend_bonus_player_level_difference_like_cpp
                {
                    continue;
                }
            }

            let member_recruited_self = member.recruiter_id == self.account_id;
            let self_recruited_member = self.recruiter_id_like_cpp() == member.account_id;
            if member_recruited_self || self_recruited_member {
                return true;
            }
        }

        false
    }
    pub fn set_recruit_a_friend_xp_config_like_cpp(
        &mut self,
        max_bonus_level: u32,
        max_level_difference: u32,
    ) {
        self.max_recruit_a_friend_bonus_player_level_like_cpp = max_bonus_level;
        self.max_recruit_a_friend_bonus_player_level_difference_like_cpp = max_level_difference;
    }
}
