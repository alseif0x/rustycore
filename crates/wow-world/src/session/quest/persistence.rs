//! Save and load plans for represented quest state.
//!
//! Moved out of the Session root under #605. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub fn set_quest_poi_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::QuestPoiPersistencePortLikeCpp>,
    ) {
        self.persistence_ports_like_cpp.catalogs.quest_poi = Some(port);
    }
    pub(crate) fn quest_poi_persistence_port_like_cpp(
        &self,
    ) -> Option<Arc<dyn wow_persistence::QuestPoiPersistencePortLikeCpp>> {
        self.persistence_ports_like_cpp.catalogs.quest_poi.clone()
    }
    pub(in crate::session) fn resolved_current_player_xp_persistence_request_like_cpp(
        &self,
        level_changed: bool,
        rest_info_changed: bool,
        guid_counter: u64,
    ) -> Option<wow_persistence::PlayerXpPersistenceRequestLikeCpp> {
        let rest = if rest_info_changed {
            Some(wow_persistence::PlayerXpRestStateSaveLikeCpp {
                rest_state: self.resolved_xp_rest_state_like_cpp()?,
                player_flags: self.resolved_player_flags_for_rest_state_save_like_cpp()?,
                rest_bonus: Self::sanitize_rest_bonus_like_cpp(
                    self.resolved_xp_rest_bonus_like_cpp()?,
                ),
            })
        } else {
            None
        };
        Some(wow_persistence::PlayerXpPersistenceRequestLikeCpp {
            player_guid: guid_counter,
            level_changed,
            level: self.player_level_like_cpp(),
            xp: self.resolved_player_xp_like_cpp()?,
            rest,
        })
    }
    #[cfg(test)]
    pub(in crate::session) fn current_player_xp_persistence_request_like_cpp(
        &self,
        level_changed: bool,
        rest_info_changed: bool,
        guid_counter: u64,
    ) -> wow_persistence::PlayerXpPersistenceRequestLikeCpp {
        wow_persistence::PlayerXpPersistenceRequestLikeCpp {
            player_guid: guid_counter,
            level_changed,
            level: self.player_level_like_cpp(),
            xp: self.player_xp_like_cpp(),
            rest: rest_info_changed.then(|| wow_persistence::PlayerXpRestStateSaveLikeCpp {
                rest_state: self.represented_xp_rest_state_like_cpp(),
                player_flags: self.represented_player_flags_for_rest_state_save_like_cpp(),
                rest_bonus: Self::sanitize_rest_bonus_like_cpp(
                    self.represented_xp_rest_bonus_like_cpp(),
                ),
            }),
        }
    }
}
