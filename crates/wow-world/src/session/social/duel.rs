//! Represented duels and their lifecycle.
//!
//! Moved out of the Session root under #615. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(in crate::session) fn canonical_player_duel_in_progress_like_cpp(
        &self,
        guid: ObjectGuid,
        opponent: ObjectGuid,
    ) -> Option<bool> {
        let map_id = u32::from(self.player_map_id_like_cpp());
        let manager = Arc::clone(self.canonical_map_manager.as_ref()?);
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
    fn represented_target_can_duel_like_cpp(&self, target_guid: ObjectGuid) -> Option<bool> {
        let manager = self.canonical_map_manager.as_ref()?;
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
    pub(crate) fn handle_can_duel_like_cpp(&mut self, target_guid: ObjectGuid, to_the_death: bool) {
        let Some(result) = self.represented_target_can_duel_like_cpp(target_guid) else {
            return;
        };

        self.send_packet(&wow_packet::packets::misc::CanDuelResult {
            target_guid,
            result,
        });

        if result {
            let Some(mounted) = self.resolved_player_mounted_like_cpp() else {
                return;
            };
            let spell_id = if mounted {
                SPELL_MOUNTED_DUEL_LIKE_CPP
            } else {
                SPELL_DUEL_LIKE_CPP
            };
            #[cfg(test)]
            self.represented_can_duel_spell_casts_like_cpp.push(
                RepresentedCanDuelSpellCastLikeCpp {
                    target_guid,
                    spell_id,
                    to_the_death,
                },
            );
            #[cfg(not(test))]
            let _ = (spell_id, to_the_death);
        }
    }
    pub(crate) fn set_represented_duel_arbiter_guid_like_cpp(&mut self, guid: Option<ObjectGuid>) {
        let canonical = self
            .with_owned_player_mut_like_cpp(|player| player.set_duel_arbiter_like_cpp(guid))
            .is_some();
        #[cfg(test)]
        if !canonical && self.player_handle_like_cpp.is_none() {
            self.represented_duel_arbiter_guid_like_cpp = guid;
        }
        #[cfg(not(test))]
        let _ = canonical;
    }
    pub(crate) fn resolved_represented_duel_arbiter_guid_like_cpp(
        &self,
    ) -> Option<Option<ObjectGuid>> {
        let canonical = self.with_owned_player_like_cpp(|player| player.duel_arbiter_like_cpp());
        if canonical.is_some() {
            return canonical;
        }
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            return Some(self.represented_duel_arbiter_guid_like_cpp);
        }
        None
    }
    fn represented_current_duel_info_like_cpp(
        &mut self,
    ) -> Option<wow_entities::PlayerDuelInfoLikeCpp> {
        self.mutate_canonical_player_like_cpp(|player| player.duel_info_like_cpp())
            .flatten()
    }
    fn represented_duel_opponent_info_like_cpp(
        &mut self,
        opponent_guid: ObjectGuid,
    ) -> Option<wow_entities::PlayerDuelInfoLikeCpp> {
        self.mutate_canonical_player_by_guid_like_cpp(opponent_guid, |player| {
            player.duel_info_like_cpp()
        })
        .flatten()
    }
    fn set_represented_duel_state_like_cpp(
        &mut self,
        player_guid: ObjectGuid,
        opponent_guid: ObjectGuid,
        state: wow_entities::PlayerDuelStateLikeCpp,
    ) {
        let _ = self.mutate_canonical_player_by_guid_like_cpp(player_guid, |player| {
            player.set_duel_info_like_cpp(Some(wow_entities::PlayerDuelInfoLikeCpp {
                opponent: opponent_guid,
                state,
            }));
        });
    }
    fn clear_represented_duel_like_cpp(&mut self, player_guid: ObjectGuid) {
        let _ = self.mutate_canonical_player_by_guid_like_cpp(player_guid, |player| {
            player.clear_duel_like_cpp();
        });
    }
    fn send_represented_duel_countdown_to_opponent_like_cpp(
        &self,
        opponent_guid: ObjectGuid,
        packet_bytes: Vec<u8>,
    ) {
        self.try_send_connected_player_command_like_cpp(
            opponent_guid,
            SessionCommand::SendRepresentedDuelCountdownLikeCpp(
                crate::session::mailbox::SendRepresentedDuelCountdownLikeCppCommand {
                    packet_bytes,
                },
            ),
        );
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
            .mutate_canonical_player_by_guid_like_cpp(player_guid, |player| {
                player.duel_info_like_cpp()
            })
            .flatten()
            .is_some()
        {
            return false;
        }
        let Some(target_duel) = self
            .mutate_canonical_player_by_guid_like_cpp(target_guid, |player| {
                player.duel_info_like_cpp()
            })
        else {
            return false;
        };
        if target_duel.is_some() {
            return false;
        }

        let map_id = self.player_map_id_like_cpp();
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
            ObjectGuid::create_global(HighGuid::WowAccount, 0, self.account_id as i64);

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
        self.send_represented_duel_requested_to_opponent_like_cpp(
            target_guid,
            arbiter_guid,
            packet_bytes,
        );

        #[cfg(test)]
        {
            self.represented_duel_requests_like_cpp
                .push(RepresentedDuelRequestedLikeCpp {
                    target_guid,
                    arbiter_guid,
                    gameobject_entry: gameobject_entry as u32,
                    to_the_death: false,
                });
        }
        true
    }
    fn handle_duel_accepted_like_cpp(&mut self, arbiter_guid: ObjectGuid) -> bool {
        let Some(player_guid) = self.player_guid() else {
            return false;
        };
        if self.resolved_represented_duel_arbiter_guid_like_cpp() != Some(Some(arbiter_guid)) {
            return false;
        }

        let Some(duel) = self.represented_current_duel_info_like_cpp() else {
            return false;
        };
        if duel.state != wow_entities::PlayerDuelStateLikeCpp::Challenged {
            return false;
        }

        let opponent_guid = duel.opponent;
        let Some(opponent_duel) = self.represented_duel_opponent_info_like_cpp(opponent_guid)
        else {
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
        self.send_represented_duel_countdown_to_opponent_like_cpp(opponent_guid, packet_bytes);
        #[cfg(test)]
        self.represented_duel_accepts_like_cpp
            .push(RepresentedDuelAcceptedLikeCpp {
                opponent_guid,
                arbiter_guid,
                countdown_ms: DUEL_COUNTDOWN_MS_LIKE_CPP,
            });
        true
    }
    fn handle_duel_cancelled_like_cpp(&mut self) -> bool {
        let Some(player_guid) = self.player_guid() else {
            return false;
        };
        let Some(duel) = self.represented_current_duel_info_like_cpp() else {
            return false;
        };
        if duel.state == wow_entities::PlayerDuelStateLikeCpp::Completed {
            return false;
        }

        let opponent_guid = duel.opponent;
        #[cfg(test)]
        let outcome = if duel.state == wow_entities::PlayerDuelStateLikeCpp::InProgress {
            RepresentedDuelCancelOutcomeLikeCpp::Surrendered
        } else {
            RepresentedDuelCancelOutcomeLikeCpp::Interrupted
        };
        #[cfg(test)]
        let beg_spell_id = (outcome == RepresentedDuelCancelOutcomeLikeCpp::Surrendered)
            .then_some(SPELL_DUEL_BEG_LIKE_CPP);

        self.clear_represented_duel_like_cpp(player_guid);
        self.clear_represented_duel_like_cpp(opponent_guid);
        #[cfg(test)]
        self.represented_duel_cancels_like_cpp
            .push(RepresentedDuelCancelledLikeCpp {
                opponent_guid,
                outcome,
                beg_spell_id,
            });
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
            self.handle_duel_cancelled_like_cpp()
        }
    }
    #[cfg(test)]
    pub(crate) fn represented_duel_accepts_like_cpp(&self) -> &[RepresentedDuelAcceptedLikeCpp] {
        &self.represented_duel_accepts_like_cpp
    }
    #[cfg(test)]
    pub(crate) fn represented_duel_cancels_like_cpp(&self) -> &[RepresentedDuelCancelledLikeCpp] {
        &self.represented_duel_cancels_like_cpp
    }
}
