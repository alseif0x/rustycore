// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Xp grants: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::{Arc, PLAYER_FLAGS_NO_XP_GAIN_LIKE_CPP, Player, WorldSession, info, warn};

impl WorldSession {
    /// Apply XP to the live session state, leveling up if threshold reached.
    /// C++ `Player::GiveXP` visible side effects; persistence is handled by async wrappers.
    /// `group_rate` is packet metadata only: C++ `KillRewarder::_RewardXP`
    /// scales `xp` before calling this method and passes `_groupRate` separately.
    pub(crate) fn give_xp_runtime_like_cpp(
        &mut self,
        mut xp: u32,
        victim: wow_core::ObjectGuid,
        group_rate: f32,
    ) -> bool {
        use wow_packet::packets::misc::{LevelUpInfo, LogXpGain};

        if xp == 0 {
            return false;
        }
        let Some(player_is_alive) = self.resolved_player_is_alive_like_cpp() else {
            return false;
        };
        if !player_is_alive && !self.player_in_represented_battleground_like_cpp() {
            return false;
        }
        if self.represented_player_has_flag_like_cpp(PLAYER_FLAGS_NO_XP_GAIN_LIKE_CPP) {
            return false;
        }
        if victim.is_any_type_creature()
            && !self
                .represented_creature_has_loot_recipient_like_cpp(victim)
                .unwrap_or(false)
        {
            return false;
        }

        // C++ captures the pre-hook level, dispatches the mutable PlayerScript
        // amount, and only then checks max level. Do not reapply the xp == 0
        // guard after dispatch: C++ continues when a hook changes the amount
        // to zero.
        let old_level = self.player_level_like_cpp();
        let script_context = wow_script::player::GivePlayerXpContextLikeCpp {
            player_guid: self.player_guid().unwrap_or(wow_core::ObjectGuid::EMPTY),
            victim_guid: victim,
        };
        #[cfg(test)]
        if let Some(dispatcher) = &self.give_player_xp_script_dispatcher_like_cpp {
            dispatcher(script_context, &mut xp);
        } else {
            let _ = wow_script::player::on_give_player_xp_like_cpp(script_context, &mut xp);
        }
        #[cfg(not(test))]
        let _ = wow_script::player::on_give_player_xp_like_cpp(script_context, &mut xp);
        if self.player_is_max_level_like_cpp() {
            return false;
        } // max level

        // Resolve the generation-checked owner before consuming rest state or
        // publishing LogXPGain. A stale session must produce no side effect.
        let (Some(current_xp), Some(_next_level_xp)) = (
            self.resolved_player_xp_like_cpp(),
            self.resolved_player_next_level_xp_like_cpp(),
        ) else {
            return false;
        };

        // C++ `Player::GiveXP`: Recruit-A-Friend is mutually exclusive with
        // rested XP and contributes 2 * base XP (3x total).
        let recruit_a_friend = self.gets_recruit_a_friend_xp_bonus_like_cpp();
        let (bonus_xp, rest_info_mask) = if recruit_a_friend {
            (xp.saturating_mul(2), 0)
        } else {
            self.take_represented_xp_rest_bonus_for_gain_like_cpp(xp, victim)
        };
        let total_xp = xp.saturating_add(bonus_xp);

        // Send floating XP text — C++ `WorldPackets::Character::LogXPGain`.
        // C++ opcode registration routes SMSG_LOG_XP_GAIN through
        // CONNECTION_TYPE_REALM, even after the session has switched its
        // default channel to the instance connection.
        self.send_packet_realm(&LogXpGain {
            victim,
            original: total_xp.min(i32::MAX as u32) as i32,
            reason: if victim.is_empty() { 1 } else { 0 },
            amount: xp.min(i32::MAX as u32) as i32,
            group_bonus: group_rate,
        });
        if !self.set_player_xp_like_cpp(current_xp.saturating_add(total_xp)) {
            return false;
        }

        // C++ `Player::GiveXP`: while (newXP >= nextLvlXP && !IsMaxLevel()).
        loop {
            let (Some(current_xp), Some(next_level_xp)) = (
                self.resolved_player_xp_like_cpp(),
                self.resolved_player_next_level_xp_like_cpp(),
            ) else {
                return false;
            };
            if current_xp < next_level_xp || self.player_is_max_level_like_cpp() {
                break;
            }
            if !self.set_player_xp_like_cpp(current_xp - next_level_xp) {
                return false;
            }
            let new_level = self.player_level_like_cpp() + 1;

            info!(account = self.account_id, new_level, "Player leveled up");

            // C++ `Player::GiveLevel` computes these deltas from
            // player_classlevelstats + player_racestats and GtBaseMP before
            // updating the live player level.
            let (base_mana_delta, stat_delta) = self
                .level_up_stat_deltas_like_cpp(new_level)
                .unwrap_or((0, [0; 5]));
            let mut power_delta = [0i32; 10];
            power_delta[0] = base_mana_delta;

            let Some(next_level_xp) = self.resolved_player_xp_for_level_like_cpp(new_level) else {
                return false;
            };

            // Send SMSG_LEVELUP_INFO — "Ding!" popup.
            // C++ registers SMSG_LEVEL_UP_INFO on CONNECTION_TYPE_REALM too.
            self.send_packet_realm(&LevelUpInfo {
                level: new_level as i32,
                health_delta: 0,
                power_delta,
                stat_delta,
                num_new_talents: 0,
            });

            self.set_player_level_like_cpp(new_level);
            self.set_player_next_level_xp_like_cpp(next_level_xp);
            self.send_level_up_stat_update_like_cpp();
        }

        self.sync_represented_xp_level_to_canonical_and_client_like_cpp(
            self.player_level_like_cpp() != old_level,
            rest_info_mask,
        );

        true
    }

    /// Mirror C++ update-field side effects from `SetXP` / `GiveLevel` until
    /// canonical map-owned `SendObjectUpdates` has complete session fanout.
    pub(in crate::session) fn sync_represented_xp_level_to_canonical_and_client_like_cpp(
        &mut self,
        level_changed: bool,
        rest_info_mask: u8,
    ) {
        if self.player_guid().is_none() {
            return;
        }

        let level = self.player_level_like_cpp();
        let (Some(xp), Some(next_level_xp), Some(scaling_player_level_delta)) = (
            self.resolved_player_xp_like_cpp(),
            self.resolved_player_next_level_xp_like_cpp(),
            self.resolved_player_scaling_level_delta_like_cpp(),
        ) else {
            return;
        };
        let xp = xp.min(i32::MAX as u32) as i32;
        let next_level_xp = next_level_xp.min(i32::MAX as u32) as i32;
        // `GiveLevel` may publish and clear its stat delta before C++'s final
        // `SetXP(newXP)`. Preserve that final unconditional ModifyValue mark
        // without writing a second progression value back into the owner.
        let owner_marked = self
            .with_owned_player_mut_like_cpp(|player| {
                player.mark_xp_changed_like_cpp();
                player.mark_scaling_player_level_delta_changed_like_cpp();
            })
            .is_some();
        #[cfg(not(test))]
        if !owner_marked {
            return;
        }
        #[cfg(test)]
        if !owner_marked && self.player_handle_like_cpp.is_some() {
            // A stale handle is an unknown owner in tests too. Only legacy
            // handle-less fixtures may exercise the isolated packet adapter.
            return;
        }

        // C++ mutates RestInfo and XP on the same Player update mask before
        // `Map::SendObjectUpdates`. Build one isolated transitional delta so
        // the explicit session fanout neither splits nor duplicates RestInfo,
        // and does not leak unrelated canonical dirty fields.
        let mut delta = Player::new(None, false);
        delta.clear_data_changes();
        if level_changed {
            delta.unit_mut().set_level(level);
            delta.set_next_level_xp(next_level_xp);
        }
        delta.set_xp(xp);
        delta.mark_xp_changed_like_cpp();
        delta.set_scaling_player_level_delta_like_cpp(scaling_player_level_delta);
        delta.mark_scaling_player_level_delta_changed_like_cpp();
        if rest_info_mask != 0 {
            let (Some(rest_threshold), Some(rest_state)) = (
                self.resolved_xp_rest_threshold_like_cpp(),
                self.resolved_xp_rest_state_like_cpp(),
            ) else {
                return;
            };
            delta.prepare_rest_info_values_update_like_cpp(
                0,
                rest_threshold,
                rest_state,
                rest_info_mask,
            );
        }
        let update = delta.values_update(true);
        self.send_player_values_update_like_cpp(&update);
    }

    /// Give XP to the player, leveling up if threshold reached.
    /// C++ `Player::GiveXP(xp, victim, group_rate)`.
    pub(crate) async fn give_xp(&mut self, xp: u32, victim: wow_core::ObjectGuid, group_rate: f32) {
        let old_level = self.player_level_like_cpp();
        let (Some(old_rest_bonus), Some(old_rest_state)) = (
            self.resolved_xp_rest_bonus_like_cpp(),
            self.resolved_xp_rest_state_like_cpp(),
        ) else {
            return;
        };
        if !self.give_xp_runtime_like_cpp(xp, victim, group_rate) {
            return;
        }

        let (Some(guid), Some(port)) = (
            self.player_guid(),
            self.player_lifecycle_port_like_cpp().map(Arc::clone),
        ) else {
            return;
        };
        let Some(request) = self.resolved_current_player_xp_persistence_request_like_cpp(
            self.player_level_like_cpp() != old_level,
            self.represented_xp_rest_info_changed_since_like_cpp(old_rest_bonus, old_rest_state),
            guid.counter() as u64,
        ) else {
            return;
        };
        match port.persist_xp_like_cpp(request).await {
            wow_persistence::PersistenceOutcomeLikeCpp::Applied { .. } => {}
            wow_persistence::PersistenceOutcomeLikeCpp::Failed { reason }
            | wow_persistence::PersistenceOutcomeLikeCpp::Unknown { reason } => {
                warn!(
                    guid = guid.counter(),
                    "Failed to atomically persist represented XP/rest state: {reason}"
                );
            }
        }
    }
}
