// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Complete Player XP grant operation and its persistence wrapper.

use std::sync::Arc;

use wow_core::ObjectGuid;
use wow_packet::packets::misc::{LevelUpInfo, LogXpGain};
use wow_world_core::session::CoreXPGainAccessLikeCpp;
#[cfg(any(test, feature = "test-fixtures"))]
use wow_world_core::session::{CoreXPGainFixtureRefsLikeCpp, StatsFixtureRefs};
use wow_world_lifecycle::SessionLifecycleState;

use super::QuestRewardCx;

mod raf;
mod rest;

impl QuestRewardCx<'_> {
    /// Apply XP to canonical player state, including the visible level-up phases.
    /// C++ `Player::GiveXP`; the async wrapper persists the final projection.
    /// `group_rate` is packet metadata only; `KillRewarder::_RewardXP` has
    /// already scaled `xp` before this operation is called.
    pub fn give_xp_runtime_like_cpp(
        &mut self,
        mut xp: u32,
        victim: ObjectGuid,
        group_rate: f32,
        #[cfg(any(test, feature = "test-fixtures"))]
        fixtures: &mut QuestXpGainFixtureRefsLikeCpp<'_>,
    ) -> bool {
        if xp == 0 {
            return false;
        }

        let player_is_alive = {
            let player = self.xp_gain_access_like_cpp();
            player.resolved_player_is_alive_like_cpp(
                #[cfg(any(test, feature = "test-fixtures"))]
                fixtures.stats.vitals_fixture_refs_like_cpp(),
            )
        };
        let Some(player_is_alive) = player_is_alive else {
            return false;
        };
        if !player_is_alive {
            let in_battleground = {
                let player = self.xp_gain_access_like_cpp();
                player.player_in_represented_battleground_like_cpp(
                    #[cfg(any(test, feature = "test-fixtures"))]
                    &fixtures.core,
                )
            };
            if !in_battleground {
                return false;
            }
        }
        if self.represented_player_has_flag_like_cpp(
            wow_constants::player_flags::PLAYER_FLAGS_NO_XP_GAIN_LIKE_CPP,
        ) {
            return false;
        }
        if victim.is_any_type_creature() {
            let has_loot_recipient = {
                let player = self.xp_gain_access_like_cpp();
                player.represented_creature_has_loot_recipient_like_cpp(victim)
            };
            if !has_loot_recipient.unwrap_or(false) {
                return false;
            }
        }

        // Capture the old level, dispatch PlayerScript, then check max level.
        // C++ does not repeat the initial xp==0 guard after the hook.
        let old_level = {
            let player = self.xp_gain_access_like_cpp();
            player.player_level_like_cpp(
                #[cfg(any(test, feature = "test-fixtures"))]
                &*self.player_level,
            )
        };
        let player_guid = {
            let player = self.xp_gain_access_like_cpp();
            player.player_guid_like_cpp()
        };
        let script_context = wow_script::player::GivePlayerXpContextLikeCpp {
            player_guid: player_guid.unwrap_or(ObjectGuid::EMPTY),
            victim_guid: victim,
        };
        #[cfg(any(test, feature = "test-fixtures"))]
        if self.world_test_consumer {
            if let Some(dispatcher) = &self.config.give_player_xp_script_dispatcher_like_cpp {
                dispatcher(script_context, &mut xp);
            } else {
                let _ = wow_script::player::on_give_player_xp_like_cpp(script_context, &mut xp);
            }
        } else {
            let _ = wow_script::player::on_give_player_xp_like_cpp(script_context, &mut xp);
        }
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let _ = wow_script::player::on_give_player_xp_like_cpp(script_context, &mut xp);

        let is_max_level = {
            let player = self.xp_gain_access_like_cpp();
            player.player_is_max_level_like_cpp(
                #[cfg(any(test, feature = "test-fixtures"))]
                &*self.player_level,
            )
        };
        if is_max_level {
            return false;
        }

        // Resolve both canonical values before RAF/rest mutation or LogXPGain.
        let (current_xp, next_level_xp) = {
            let player = self.xp_gain_access_like_cpp();
            (
                player.resolved_player_xp_like_cpp(
                    #[cfg(any(test, feature = "test-fixtures"))]
                    fixtures.core.player_xp_fixture_like_cpp(),
                ),
                player.resolved_player_next_level_xp_like_cpp(
                    #[cfg(any(test, feature = "test-fixtures"))]
                    &fixtures.core,
                ),
            )
        };
        let (Some(current_xp), Some(_next_level_xp)) = (current_xp, next_level_xp) else {
            return false;
        };

        // RAF is mutually exclusive with rested XP and contributes 2 * base XP.
        let recruit_a_friend = self.gets_recruit_a_friend_xp_bonus_like_cpp(
            #[cfg(any(test, feature = "test-fixtures"))]
            fixtures,
        );
        let (bonus_xp, rest_info_mask) = if recruit_a_friend {
            (xp.saturating_mul(2), 0)
        } else {
            self.take_represented_xp_rest_bonus_for_gain_like_cpp(
                xp,
                victim,
                #[cfg(any(test, feature = "test-fixtures"))]
                fixtures,
            )
        };
        let total_xp = xp.saturating_add(bonus_xp);

        // SMSG_LOG_XP_GAIN is registered on CONNECTION_TYPE_REALM.
        let packet = LogXpGain {
            victim,
            original: total_xp.min(i32::MAX as u32) as i32,
            reason: if victim.is_empty() { 1 } else { 0 },
            amount: xp.min(i32::MAX as u32) as i32,
            group_bonus: group_rate,
        };
        {
            let player = self.xp_gain_access_like_cpp();
            player.send_packet_realm_like_cpp(&packet);
        }
        let installed = {
            let mut player = self.xp_gain_access_like_cpp();
            player.set_player_xp_like_cpp(
                current_xp.saturating_add(total_xp),
                #[cfg(any(test, feature = "test-fixtures"))]
                &mut fixtures.core,
            )
        };
        if !installed {
            return false;
        }

        // C++ `Player::GiveXP`: while (newXP >= nextLvlXP && !IsMaxLevel()).
        loop {
            let (current_xp, next_level_xp) = {
                let player = self.xp_gain_access_like_cpp();
                (
                    player.resolved_player_xp_like_cpp(
                        #[cfg(any(test, feature = "test-fixtures"))]
                        fixtures.core.player_xp_fixture_like_cpp(),
                    ),
                    player.resolved_player_next_level_xp_like_cpp(
                        #[cfg(any(test, feature = "test-fixtures"))]
                        &fixtures.core,
                    ),
                )
            };
            let (Some(current_xp), Some(next_level_xp)) = (current_xp, next_level_xp) else {
                return false;
            };
            if current_xp < next_level_xp {
                break;
            }
            let is_max_level = {
                let player = self.xp_gain_access_like_cpp();
                player.player_is_max_level_like_cpp(
                    #[cfg(any(test, feature = "test-fixtures"))]
                    &*self.player_level,
                )
            };
            if is_max_level {
                break;
            }
            let removed = {
                let mut player = self.xp_gain_access_like_cpp();
                player.set_player_xp_like_cpp(
                    current_xp - next_level_xp,
                    #[cfg(any(test, feature = "test-fixtures"))]
                    &mut fixtures.core,
                )
            };
            if !removed {
                return false;
            }
            let new_level = {
                let player = self.xp_gain_access_like_cpp();
                player.player_level_like_cpp(
                    #[cfg(any(test, feature = "test-fixtures"))]
                    &*self.player_level,
                ) + 1
            };

            tracing::info!(
                account = self.player.account_id_like_cpp(),
                new_level,
                "Player leveled up"
            );

            let (base_mana_delta, stat_delta) = self
                .level_up_stat_deltas_like_cpp(
                    new_level,
                    #[cfg(any(test, feature = "test-fixtures"))]
                    fixtures,
                )
                .unwrap_or((0, [0; 5]));
            let mut power_delta = [0i32; 10];
            power_delta[0] = base_mana_delta;

            let next_level_xp = {
                let player = self.xp_gain_access_like_cpp();
                player.resolved_player_xp_for_level_like_cpp(
                    new_level,
                    #[cfg(any(test, feature = "test-fixtures"))]
                    &fixtures.core,
                )
            };
            let Some(next_level_xp) = next_level_xp else {
                return false;
            };

            let packet = LevelUpInfo {
                level: new_level as i32,
                health_delta: 0,
                power_delta,
                stat_delta,
                num_new_talents: 0,
            };
            {
                let player = self.xp_gain_access_like_cpp();
                player.send_packet_realm_like_cpp(&packet);
            }

            self.apply_xp_level_transition_like_cpp(
                new_level,
                #[cfg(any(test, feature = "test-fixtures"))]
                fixtures,
            );
            {
                let mut player = self.xp_gain_access_like_cpp();
                player.set_player_next_level_xp_like_cpp(
                    next_level_xp,
                    #[cfg(any(test, feature = "test-fixtures"))]
                    &mut fixtures.core,
                );
            }
            Self::send_level_up_stat_update_like_cpp(
                &self.player,
                #[cfg(any(test, feature = "test-fixtures"))]
                self.player_level,
                self.catalogs,
                self.config,
                self.inventory,
                #[cfg(any(test, feature = "test-fixtures"))]
                fixtures.stats.reborrow_like_cpp(),
            );
        }

        let level_changed = {
            let player = self.xp_gain_access_like_cpp();
            player.player_level_like_cpp(
                #[cfg(any(test, feature = "test-fixtures"))]
                &*self.player_level,
            ) != old_level
        };
        {
            let mut player = self.xp_gain_access_like_cpp();
            player.sync_represented_xp_level_to_canonical_and_client_like_cpp(
                level_changed,
                rest_info_mask,
                #[cfg(any(test, feature = "test-fixtures"))]
                &*self.player_level,
                #[cfg(any(test, feature = "test-fixtures"))]
                &fixtures.core,
            );
        }
        true
    }

    /// C++ `Player::GiveXP(xp, victim, group_rate)` plus the existing XP save.
    pub async fn give_xp_like_cpp(
        &mut self,
        xp: u32,
        victim: ObjectGuid,
        group_rate: f32,
        #[cfg(any(test, feature = "test-fixtures"))]
        fixtures: &mut QuestXpGainFixtureRefsLikeCpp<'_>,
    ) {
        let old_level = {
            let player = self.xp_gain_access_like_cpp();
            player.player_level_like_cpp(
                #[cfg(any(test, feature = "test-fixtures"))]
                &*self.player_level,
            )
        };
        let (old_rest_bonus, old_rest_state) = {
            let player = self.xp_gain_access_like_cpp();
            (
                player.resolved_xp_rest_bonus_like_cpp(
                    #[cfg(any(test, feature = "test-fixtures"))]
                    fixtures.core.rest_mgr_fixture_like_cpp(),
                ),
                player.resolved_xp_rest_state_like_cpp(
                    #[cfg(any(test, feature = "test-fixtures"))]
                    fixtures.core.rest_mgr_fixture_like_cpp(),
                ),
            )
        };
        let (Some(old_rest_bonus), Some(old_rest_state)) = (old_rest_bonus, old_rest_state) else {
            return;
        };
        if !self.give_xp_runtime_like_cpp(
            xp,
            victim,
            group_rate,
            #[cfg(any(test, feature = "test-fixtures"))]
            fixtures,
        ) {
            return;
        }

        let (Some(guid), Some(port)) = (
            {
                let player = self.xp_gain_access_like_cpp();
                player.player_guid_like_cpp()
            },
            self.lifecycle
                .player_lifecycle_port_like_cpp()
                .map(Arc::clone),
        ) else {
            return;
        };
        let (level_changed, rest_info_changed) = {
            let player = self.xp_gain_access_like_cpp();
            (
                player.player_level_like_cpp(
                    #[cfg(any(test, feature = "test-fixtures"))]
                    &*self.player_level,
                ) != old_level,
                player.represented_xp_rest_info_changed_since_like_cpp(
                    old_rest_bonus,
                    old_rest_state,
                    #[cfg(any(test, feature = "test-fixtures"))]
                    fixtures.core.rest_mgr_fixture_like_cpp(),
                ),
            )
        };
        let Some(request) = self.resolved_current_player_xp_persistence_request_like_cpp(
            level_changed,
            rest_info_changed,
            guid.counter() as u64,
            #[cfg(any(test, feature = "test-fixtures"))]
            fixtures,
        ) else {
            return;
        };
        match port.persist_xp_like_cpp(request).await {
            wow_persistence::PersistenceOutcomeLikeCpp::Applied { .. } => {}
            wow_persistence::PersistenceOutcomeLikeCpp::Failed { reason }
            | wow_persistence::PersistenceOutcomeLikeCpp::Unknown { reason } => {
                tracing::warn!(
                    guid = guid.counter(),
                    "Failed to atomically persist represented XP/rest state: {reason}"
                );
            }
        }
    }

    fn xp_gain_access_like_cpp(&self) -> CoreXPGainAccessLikeCpp<'_> {
        self.player
            .xp_gain_access_like_cpp(self.catalogs, self.config)
    }

    fn represented_player_has_flag_like_cpp(&self, flag: u32) -> bool {
        let canonical = self
            .player
            .player_guid_like_cpp()
            .and_then(|guid| self.player.canonical_player_has_player_flag_like_cpp(guid, flag));
        if let Some(value) = canonical {
            return value;
        }
        #[cfg(any(test, feature = "test-fixtures"))]
        if self.world_test_consumer && self.xp_gain_access_like_cpp().owner_handle_absent_like_cpp() {
            return self
                .lifecycle
                .represented_loaded_player_flags_for_test_like_cpp()
                .is_some_and(|flags| (flags & flag) != 0);
        }
        false
    }

    fn level_up_stat_deltas_like_cpp(
        &self,
        new_level: u8,
        #[cfg(any(test, feature = "test-fixtures"))]
        fixtures: &mut QuestXpGainFixtureRefsLikeCpp<'_>,
    ) -> Option<(i32, [i32; 5])> {
        let stats = self.player.stats_access_like_cpp(
            self.catalogs,
            self.config,
            #[cfg(any(test, feature = "test-fixtures"))]
            &*self.player_level,
            #[cfg(any(test, feature = "test-fixtures"))]
            fixtures.stats.reborrow_like_cpp(),
        );
        let publication = self.player.packet_publication_access_like_cpp();
        let application = crate::CharacterStatsApplicationCxLikeCpp::new(
            stats,
            self.inventory,
            publication,
        );
        application.level_up_stat_deltas_like_cpp(new_level)
    }

    fn apply_xp_level_transition_like_cpp(
        &mut self,
        level: u8,
        #[cfg(any(test, feature = "test-fixtures"))]
        fixtures: &mut QuestXpGainFixtureRefsLikeCpp<'_>,
    ) {
        #[cfg(any(test, feature = "test-fixtures"))]
        Self::set_player_level_and_refresh_talent_points_like_cpp(
            &mut self.player,
            level,
            self.catalogs,
            self.quest_state,
            self.world_test_consumer,
            self.player_level,
            fixtures.gray_level_overrides,
            fixtures.talent_groups,
            fixtures.active_talent_group,
            fixtures.player_character_points,
        );
        #[cfg(not(any(test, feature = "test-fixtures")))]
        Self::set_player_level_and_refresh_talent_points_like_cpp(
            &mut self.player,
            level,
            self.catalogs,
            self.world_test_consumer,
        );
    }

    fn resolved_current_player_xp_persistence_request_like_cpp(
        &self,
        level_changed: bool,
        rest_info_changed: bool,
        guid_counter: u64,
        #[cfg(any(test, feature = "test-fixtures"))]
        fixtures: &QuestXpGainFixtureRefsLikeCpp<'_>,
    ) -> Option<wow_persistence::PlayerXpPersistenceRequestLikeCpp> {
        Self::resolved_current_player_xp_persistence_request_from_access_like_cpp(
            self.xp_gain_access_like_cpp(),
            self.lifecycle,
            self.world_test_consumer,
            level_changed,
            rest_info_changed,
            guid_counter,
            #[cfg(any(test, feature = "test-fixtures"))]
            &*self.player_level,
            #[cfg(any(test, feature = "test-fixtures"))]
            fixtures.core.player_xp_fixture_like_cpp(),
            #[cfg(any(test, feature = "test-fixtures"))]
            fixtures.core.rest_mgr_fixture_like_cpp(),
        )
    }

    /// Read the XP persistence projection through selected immutable owners.
    #[allow(clippy::too_many_arguments)]
    pub fn resolved_current_player_xp_persistence_request_from_access_like_cpp(
        player: CoreXPGainAccessLikeCpp<'_>,
        lifecycle: &SessionLifecycleState,
        world_test_consumer: bool,
        level_changed: bool,
        rest_info_changed: bool,
        guid_counter: u64,
        #[cfg(any(test, feature = "test-fixtures"))] fixture_level: &u8,
        #[cfg(any(test, feature = "test-fixtures"))] fixture_xp: &u32,
        #[cfg(any(test, feature = "test-fixtures"))]
        fixture_rest: &wow_world_core::session::RestMgrTestFixtureLikeCpp,
    ) -> Option<wow_persistence::PlayerXpPersistenceRequestLikeCpp> {
        let rest = if rest_info_changed {
            Some(wow_persistence::PlayerXpRestStateSaveLikeCpp {
                rest_state: player.resolved_xp_rest_state_like_cpp(
                    #[cfg(any(test, feature = "test-fixtures"))]
                    fixture_rest,
                )?,
                player_flags: Self::resolved_player_flags_for_rest_state_save_from_access_like_cpp(
                    &player,
                    lifecycle,
                    world_test_consumer,
                    #[cfg(any(test, feature = "test-fixtures"))]
                    fixture_rest,
                )?,
                rest_bonus: wow_entities::sanitize_rest_bonus_like_cpp(
                    player.resolved_xp_rest_bonus_like_cpp(
                        #[cfg(any(test, feature = "test-fixtures"))]
                        fixture_rest,
                    )?,
                ),
            })
        } else {
            None
        };
        Some(wow_persistence::PlayerXpPersistenceRequestLikeCpp {
            player_guid: guid_counter,
            level_changed,
            level: player.player_level_like_cpp(
                #[cfg(any(test, feature = "test-fixtures"))]
                fixture_level,
            ),
            xp: player.resolved_player_xp_like_cpp(
                #[cfg(any(test, feature = "test-fixtures"))]
                fixture_xp,
            )?,
            rest,
        })
    }

    pub fn resolved_player_flags_for_rest_state_save_from_access_like_cpp(
        player: &CoreXPGainAccessLikeCpp<'_>,
        lifecycle: &SessionLifecycleState,
        world_test_consumer: bool,
        #[cfg(any(test, feature = "test-fixtures"))]
        fixture_rest: &wow_world_core::session::RestMgrTestFixtureLikeCpp,
    ) -> Option<u32> {
        let canonical = player.resolved_player_flags_for_rest_state_save_like_cpp();
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none()
            && world_test_consumer
            && player.owner_handle_absent_like_cpp()
        {
            let mut flags = lifecycle
                .represented_loaded_player_flags_for_test_like_cpp()
                .unwrap_or(0);
            let rest = player.player_rest_state_snapshot_like_cpp(fixture_rest)?;
            if rest.is_location_initialized_like_cpp() {
                if rest.is_resting_by_flag_like_cpp() {
                    flags |= wow_world_core::session::PLAYER_FLAGS_RESTING_LIKE_CPP;
                } else {
                    flags &= !wow_world_core::session::PLAYER_FLAGS_RESTING_LIKE_CPP;
                }
            }
            return Some(flags);
        }
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let _ = (lifecycle, world_test_consumer);
        canonical
    }
}

/// Selected XP fixture references. Player level and vitals are borrowed from
/// their existing owners at the phase that reads them, so this bundle cannot
/// shadow those authorities.
#[cfg(any(test, feature = "test-fixtures"))]
pub struct QuestXpGainFixtureRefsLikeCpp<'a> {
    core: CoreXPGainFixtureRefsLikeCpp<'a>,
    player_character_points: &'a mut i32,
    gray_level_overrides: &'a std::collections::HashMap<u8, u8>,
    talent_groups:
        &'a [std::collections::BTreeMap<u32, u8>; wow_world_core::session::MAX_SPECIALIZATIONS_LIKE_CPP],
    active_talent_group: &'a u8,
    stats: StatsFixtureRefs<'a>,
}

#[cfg(any(test, feature = "test-fixtures"))]
impl<'a> QuestXpGainFixtureRefsLikeCpp<'a> {
    pub(super) fn vitals_fixture_refs_like_cpp(&self) -> (&u32, &u32, &bool) {
        self.stats.vitals_fixture_refs_like_cpp()
    }

    pub(super) fn stats_fixture_refs_like_cpp(&mut self) -> StatsFixtureRefs<'_> {
        self.stats.reborrow_like_cpp()
    }

    pub fn new_like_cpp(
        core: CoreXPGainFixtureRefsLikeCpp<'a>,
        player_character_points: &'a mut i32,
        gray_level_overrides: &'a std::collections::HashMap<u8, u8>,
        talent_groups:
            &'a [std::collections::BTreeMap<u32, u8>; wow_world_core::session::MAX_SPECIALIZATIONS_LIKE_CPP],
        active_talent_group: &'a u8,
        stats: StatsFixtureRefs<'a>,
    ) -> Self {
        Self {
            core,
            player_character_points,
            gray_level_overrides,
            talent_groups,
            active_talent_group,
            stats,
        }
    }
}
