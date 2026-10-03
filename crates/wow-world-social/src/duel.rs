//! Represented duel operations owned by the Social boundary.

use crate::{
    SPELL_DUEL_LIKE_CPP, SPELL_MOUNTED_DUEL_LIKE_CPP, SessionSocialLimits,
};
#[cfg(any(test, feature = "test-fixtures"))]
use crate::{
    RepresentedCanDuelSpellCastLikeCpp, RepresentedDuelAcceptedLikeCpp,
    RepresentedDuelCancelOutcomeLikeCpp, RepresentedDuelCancelledLikeCpp,
    RepresentedDuelRequestedLikeCpp,
    SPELL_DUEL_BEG_LIKE_CPP,
};
use std::sync::Arc;
use wow_core::ObjectGuid;
use wow_world_core::session::{mailbox::SessionCommand, HubMut, HubRef};

impl SessionSocialLimits {
    pub fn canonical_player_duel_in_progress_like_cpp(
        &self,
        hub: HubRef<'_>,
        guid: ObjectGuid,
        opponent: ObjectGuid,
    ) -> Option<bool> {
        let map_id = u32::from(hub.core.player_map_id_like_cpp());
        let manager = Arc::clone(hub.core.canonical_map_manager.as_ref()?);
        let manager = manager.lock().ok()?;
        let mut result = None;
        manager.do_for_all_maps_with_map_id(map_id, |managed| {
            if result.is_none() {
                result = managed
                    .map()
                    .get_typed_player(guid)
                    .map(|player| player.is_dueling_opponent_in_progress_like_cpp(opponent));
            }
        });
        result
    }

    fn represented_target_can_duel_like_cpp(
        &self,
        hub: HubRef<'_>,
        target_guid: ObjectGuid,
    ) -> Option<bool> {
        let manager = hub.core.canonical_map_manager.as_ref()?;
        let Ok(manager) = manager.lock() else {
            return None;
        };
        let mut result = None;
        manager.do_for_all_maps(|managed| {
            if result.is_none()
                && let Some(player) = managed.map().get_typed_player(target_guid)
            {
                result = Some(player.duel_info_like_cpp().is_none());
            }
        });
        result
    }

    pub fn handle_can_duel_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        target_guid: ObjectGuid,
        to_the_death: bool,
    ) {
        let Some(result) = self.represented_target_can_duel_like_cpp(hub.shared(), target_guid)
        else {
            return;
        };

        hub.core
            .send_packet(&wow_packet::packets::misc::CanDuelResult {
                target_guid,
                result,
            });

        if result {
            let Some(mounted) = hub.shared().resolved_player_mounted_like_cpp() else {
                return;
            };
            let spell_id = if mounted {
                SPELL_MOUNTED_DUEL_LIKE_CPP
            } else {
                SPELL_DUEL_LIKE_CPP
            };
            #[cfg(any(test, feature = "test-fixtures"))]
            self.duel_test_fixture_like_cpp
                .represented_can_duel_spell_casts_like_cpp
                .push(RepresentedCanDuelSpellCastLikeCpp {
                    target_guid,
                    spell_id,
                    to_the_death,
                });
            #[cfg(not(any(test, feature = "test-fixtures")))]
            let _ = (spell_id, to_the_death);
        }
    }

    pub fn set_represented_duel_arbiter_guid_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        guid: Option<ObjectGuid>,
    ) {
        let canonical = hub
            .core
            .with_owned_player_mut_like_cpp(|player| player.set_duel_arbiter_like_cpp(guid))
            .is_some();
        #[cfg(any(test, feature = "test-fixtures"))]
        if !canonical && hub.core.player_handle_like_cpp.is_none() {
            self.duel_test_fixture_like_cpp
                .represented_duel_arbiter_guid_like_cpp = guid;
        }
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let _ = canonical;
    }

    pub fn resolved_represented_duel_arbiter_guid_like_cpp(
        &self,
        hub: HubRef<'_>,
    ) -> Option<Option<ObjectGuid>> {
        let canonical = hub
            .core
            .with_owned_player_like_cpp(|player| player.duel_arbiter_like_cpp());
        if canonical.is_some() {
            return canonical;
        }
        #[cfg(any(test, feature = "test-fixtures"))]
        if hub.core.player_handle_like_cpp.is_none() {
            return Some(
                self.duel_test_fixture_like_cpp
                    .represented_duel_arbiter_guid_like_cpp,
            );
        }
        None
    }

    pub fn represented_current_duel_info_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
    ) -> Option<wow_entities::PlayerDuelInfoLikeCpp> {
        hub.core
            .mutate_canonical_player_like_cpp(|player| player.duel_info_like_cpp())
            .flatten()
    }

    pub fn represented_duel_opponent_info_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        opponent_guid: ObjectGuid,
    ) -> Option<wow_entities::PlayerDuelInfoLikeCpp> {
        hub.core
            .mutate_canonical_player_by_guid_like_cpp(opponent_guid, |player| {
                player.duel_info_like_cpp()
            })
            .flatten()
    }

    fn clear_represented_duel_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        player_guid: ObjectGuid,
    ) {
        let _ = hub
            .core
            .mutate_canonical_player_by_guid_like_cpp(player_guid, |player| {
                player.clear_duel_like_cpp();
            });
    }

    pub fn send_represented_duel_countdown_to_opponent_like_cpp(
        &self,
        hub: HubRef<'_>,
        opponent_guid: ObjectGuid,
        packet_bytes: Vec<u8>,
    ) {
        hub.core.try_send_connected_player_command_like_cpp(
            opponent_guid,
            SessionCommand::SendRepresentedDuelCountdownLikeCpp(
                wow_world_core::session::mailbox::SendRepresentedDuelCountdownLikeCppCommand {
                    packet_bytes,
                },
            ),
        );
    }

    pub fn handle_duel_cancelled_like_cpp(&mut self, hub: &mut HubMut<'_>) -> bool {
        let Some(player_guid) = hub.core.player_guid() else {
            return false;
        };
        let Some(duel) = self.represented_current_duel_info_like_cpp(hub) else {
            return false;
        };
        if duel.state == wow_entities::PlayerDuelStateLikeCpp::Completed {
            return false;
        }

        let opponent_guid = duel.opponent;
        #[cfg(any(test, feature = "test-fixtures"))]
        let outcome = if duel.state == wow_entities::PlayerDuelStateLikeCpp::InProgress {
            RepresentedDuelCancelOutcomeLikeCpp::Surrendered
        } else {
            RepresentedDuelCancelOutcomeLikeCpp::Interrupted
        };
        #[cfg(any(test, feature = "test-fixtures"))]
        let beg_spell_id = (outcome == RepresentedDuelCancelOutcomeLikeCpp::Surrendered)
            .then_some(SPELL_DUEL_BEG_LIKE_CPP);

        self.clear_represented_duel_like_cpp(hub, player_guid);
        self.clear_represented_duel_like_cpp(hub, opponent_guid);
        #[cfg(any(test, feature = "test-fixtures"))]
        self.duel_test_fixture_like_cpp
            .represented_duel_cancels_like_cpp
            .push(RepresentedDuelCancelledLikeCpp {
                opponent_guid,
                outcome,
                beg_spell_id,
            });
        true
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn record_represented_duel_request_for_test_like_cpp(
        &mut self,
        request: RepresentedDuelRequestedLikeCpp,
    ) {
        self.duel_test_fixture_like_cpp
            .represented_duel_requests_like_cpp
            .push(request);
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn record_represented_duel_accept_for_test_like_cpp(
        &mut self,
        accepted: RepresentedDuelAcceptedLikeCpp,
    ) {
        self.duel_test_fixture_like_cpp
            .represented_duel_accepts_like_cpp
            .push(accepted);
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_duel_accepts_like_cpp(&self) -> &[RepresentedDuelAcceptedLikeCpp] {
        &self
            .duel_test_fixture_like_cpp
            .represented_duel_accepts_like_cpp
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_duel_cancels_like_cpp(&self) -> &[RepresentedDuelCancelledLikeCpp] {
        &self
            .duel_test_fixture_like_cpp
            .represented_duel_cancels_like_cpp
    }
}
