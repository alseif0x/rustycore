//! Quest giver menus, status and offer details seen from the Session.
//!
//! Moved out of the Session root under #605. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(crate) fn invalidate_player_quest_status_authority_like_cpp(&mut self) {
        let _ = self.mutate_player_quest_gameplay_like_cpp(|state| {
            state.set_status_authority_complete_like_cpp(false);
        });
        self.core
            .invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
    }
}

impl crate::session::QuestStateCxRef<'_> {
    pub(crate) fn represented_quest_giver_involved_source_allows_quest_like_cpp(
        &self,
        source_guid: ObjectGuid,
        quest_id: u32,
        quest_store: &wow_data::quest::QuestStore,
    ) -> bool {
        if source_guid.is_any_type_creature() {
            let Some(access) = self.hub.represented_npc_can_interact_with_like_cpp(
                source_guid,
                NPCFlags1::QUEST_GIVER.bits(),
                0,
            ) else {
                debug!(
                    account = self.hub.core.account_id,
                    ?source_guid,
                    quest_id,
                    "QuestGiverCompleteReward: represented Creature source missing or not interactable"
                );
                return false;
            };

            return quest_store.creature_has_ender_relation_like_cpp(access.entry, quest_id);
        }

        if source_guid.is_game_object() {
            let Some(access) =
                self.represented_gameobject_questgiver_can_interact_with_like_cpp(source_guid)
            else {
                debug!(
                    account = self.hub.core.account_id,
                    ?source_guid,
                    quest_id,
                    "QuestGiverCompleteReward: represented GameObject source missing or not interactable"
                );
                return false;
            };

            return quest_store.gameobject_has_ender_relation_like_cpp(access.entry, quest_id);
        }

        // Player and Item questgiver branches are not involved-quest sources in this represented slice.
        // Match the C++ early-return shape by failing closed with no packet and no mutation.
        debug!(
            account = self.hub.core.account_id,
            ?source_guid,
            quest_id,
            "QuestGiverCompleteReward: unsupported represented source type"
        );
        false
    }

    pub(crate) fn represented_quest_giver_accept_source_allows_quest_like_cpp(
        &self,
        source_guid: ObjectGuid,
        quest_id: u32,
        quest_store: &wow_data::quest::QuestStore,
    ) -> bool {
        if source_guid.is_any_type_creature() {
            let Some(access) = self.hub.represented_npc_can_interact_with_like_cpp(
                source_guid,
                NPCFlags1::QUEST_GIVER.bits(),
                0,
            ) else {
                debug!(
                    account = self.hub.core.account_id,
                    ?source_guid,
                    quest_id,
                    "QuestGiverAcceptQuest: represented Creature source missing or not interactable"
                );
                return false;
            };

            return quest_store.creature_has_starter_relation_like_cpp(access.entry, quest_id);
        }

        if source_guid.is_game_object() {
            let Some(access) =
                self.represented_gameobject_questgiver_can_interact_with_like_cpp(source_guid)
            else {
                debug!(
                    account = self.hub.core.account_id,
                    ?source_guid,
                    quest_id,
                    "QuestGiverAcceptQuest: represented GameObject source missing or not interactable"
                );
                return false;
            };

            return quest_store.gameobject_has_starter_relation_like_cpp(access.entry, quest_id);
        }

        // Player quest sharing and Item questgiver branches are not represented in this slice.
        // Match the C++ early-return shape by failing closed with no packet and no mutation.
        debug!(
            account = self.hub.core.account_id,
            ?source_guid,
            quest_id,
            "QuestGiverAcceptQuest: unsupported represented source type"
        );
        false
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/quest/giver/f3_shims.rs"]
mod f3_shims;
