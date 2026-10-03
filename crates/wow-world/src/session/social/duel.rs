//! Represented duels and their lifecycle.
//!
//! Moved out of the Session root under #615. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(crate) fn set_represented_duel_arbiter_guid_like_cpp(&mut self, guid: Option<ObjectGuid>) {
        let (state, mut hub) = crate::session::split_social_mut(self);
        state.set_represented_duel_arbiter_guid_like_cpp(&mut hub, guid)
    }
    pub(crate) fn resolved_represented_duel_arbiter_guid_like_cpp(
        &self,
    ) -> Option<Option<ObjectGuid>> {
        let (state, hub) = crate::session::split_social_ref(self);
        state.resolved_represented_duel_arbiter_guid_like_cpp(hub)
    }
    fn set_represented_duel_state_like_cpp(
        &mut self,
        player_guid: ObjectGuid,
        opponent_guid: ObjectGuid,
        state: wow_entities::PlayerDuelStateLikeCpp,
    ) {
        let _ = self
            .core
            .mutate_canonical_player_by_guid_like_cpp(player_guid, |player| {
                player.set_duel_info_like_cpp(Some(wow_entities::PlayerDuelInfoLikeCpp {
                    opponent: opponent_guid,
                    state,
                }));
            });
    }
    /// C++ `Spell::EffectDuel`.
    ///
    /// Represented boundary: canonical connected players only. This creates the
    /// duel request packet, represented arbiter GUID and challenged duel state;
    /// the actual duel-flag GameObject, area/social ignore checks, phasing,
    /// script hook and full duel lifecycle remain outside this bounded slice.
    pub(in crate::session) fn apply_duel_effect_like_cpp(
        &mut self,
        spell_id: i32,
        gameobject_entry: i32,
        target_guid: ObjectGuid,
    ) -> bool {
        let Some(player_guid) = self.player_guid() else {
            return false;
        };
        if target_guid == player_guid {
            return false;
        }
        if gameobject_entry <= 0 {
            return false;
        }
        if self
            .core
            .mutate_canonical_player_by_guid_like_cpp(player_guid, |player| {
                player.duel_info_like_cpp()
            })
            .flatten()
            .is_some()
        {
            return false;
        }
        let Some(target_duel) = self
            .core
            .mutate_canonical_player_by_guid_like_cpp(target_guid, |player| {
                player.duel_info_like_cpp()
            })
        else {
            return false;
        };
        if target_duel.is_some() {
            return false;
        }

        let map_id = self.core.player_map_id_like_cpp();
        let arbiter_guid = ObjectGuid::create_world_object(
            HighGuid::GameObject,
            0,
            1,
            map_id,
            0,
            gameobject_entry as u32,
            i64::from(spell_id.max(0) as u32),
        );
        let requested_by_wow_account =
            ObjectGuid::create_global(HighGuid::WowAccount, 0, self.core.account_id as i64);

        self.set_represented_duel_state_like_cpp(
            player_guid,
            target_guid,
            wow_entities::PlayerDuelStateLikeCpp::Challenged,
        );
        self.set_represented_duel_state_like_cpp(
            target_guid,
            player_guid,
            wow_entities::PlayerDuelStateLikeCpp::Challenged,
        );
        self.set_represented_duel_arbiter_guid_like_cpp(Some(arbiter_guid));

        use wow_packet::ServerPacket;
        let packet = wow_packet::packets::misc::DuelRequested {
            arbiter_guid,
            requested_by_guid: player_guid,
            requested_by_wow_account,
            to_the_death: false,
        };
        let packet_bytes = packet.to_bytes();
        self.send_raw_packet(&packet_bytes);
        self.core.try_send_connected_player_command_like_cpp(
            target_guid,
            SessionCommand::SendRepresentedDuelRequestedLikeCpp(
                crate::session::mailbox::SendRepresentedDuelRequestedLikeCppCommand {
                    arbiter_guid,
                    packet_bytes,
                },
            ),
        );

        #[cfg(test)]
        self.social
            .record_represented_duel_request_for_test_like_cpp(
                RepresentedDuelRequestedLikeCpp {
                    target_guid,
                    arbiter_guid,
                    gameobject_entry: gameobject_entry as u32,
                    to_the_death: false,
                },
            );
        true
    }
    fn handle_duel_accepted_like_cpp(&mut self, arbiter_guid: ObjectGuid) -> bool {
        let Some(player_guid) = self.player_guid() else {
            return false;
        };
        if self.resolved_represented_duel_arbiter_guid_like_cpp() != Some(Some(arbiter_guid)) {
            return false;
        }

        let Some(duel) = ({
            let (s, mut h) = crate::session::split_social_mut(self);
            s.represented_current_duel_info_like_cpp(&mut h)
        }) else {
            return false;
        };
        if duel.state != wow_entities::PlayerDuelStateLikeCpp::Challenged {
            return false;
        }

        let opponent_guid = duel.opponent;
        let Some(opponent_duel) = ({
            let (s, mut h) = crate::session::split_social_mut(self);
            s.represented_duel_opponent_info_like_cpp(&mut h, opponent_guid)
        }) else {
            return false;
        };
        if opponent_duel.opponent != player_guid {
            return false;
        }

        self.set_represented_duel_state_like_cpp(
            player_guid,
            opponent_guid,
            wow_entities::PlayerDuelStateLikeCpp::Countdown,
        );
        self.set_represented_duel_state_like_cpp(
            opponent_guid,
            player_guid,
            wow_entities::PlayerDuelStateLikeCpp::Countdown,
        );

        use wow_packet::ServerPacket;
        let packet = wow_packet::packets::misc::DuelCountdown {
            countdown_ms: DUEL_COUNTDOWN_MS_LIKE_CPP,
        };
        let packet_bytes = packet.to_bytes();
        self.send_raw_packet(&packet_bytes);
        {
            let (s, h) = crate::session::split_social_ref(self);
            s.send_represented_duel_countdown_to_opponent_like_cpp(h, opponent_guid, packet_bytes)
        };
        #[cfg(test)]
        self.social
            .record_represented_duel_accept_for_test_like_cpp(
                RepresentedDuelAcceptedLikeCpp {
                    opponent_guid,
                    arbiter_guid,
                    countdown_ms: DUEL_COUNTDOWN_MS_LIKE_CPP,
                },
            );
        true
    }
    pub(crate) fn handle_duel_response_like_cpp(
        &mut self,
        arbiter_guid: ObjectGuid,
        accepted: bool,
        forfeited: bool,
    ) -> bool {
        if accepted && !forfeited {
            self.handle_duel_accepted_like_cpp(arbiter_guid)
        } else {
            let (s, mut h) = crate::session::split_social_mut(self);
            s.handle_duel_cancelled_like_cpp(&mut h)
        }
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/social/duel/f3_shims.rs"]
mod f3_shims;
