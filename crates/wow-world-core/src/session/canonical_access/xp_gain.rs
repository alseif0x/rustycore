// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Selected canonical access for the complete Player XP grant operation.

use std::collections::HashMap;

use crate::session::state::{SessionCatalogs, SessionCore, SessionWorldConfig};
use wow_ai::max_level_for_expansion_like_cpp;
use wow_core::{ObjectGuid, Position};
use wow_entities::Player;

mod rest;

pub(super) fn represented_total_aura_modifier_from_snapshot_like_cpp(
    auras: Option<wow_entities::AuraSubsystem>,
    effect: wow_entities::RepresentedAuraEffectLikeCpp,
) -> Option<i32> {
    auras.map(|auras| {
        auras
            .runtime_applications_like_cpp()
            .values()
            .filter(|aura| aura.represented_effect == Some(effect))
            .map(|aura| aura.represented_amount)
            .sum()
    })
}

/// World-owned fallback references used only when the canonical owner is absent.
/// Every field is required at construction; no session fixture aggregate crosses this boundary.
#[cfg(any(test, feature = "test-fixtures"))]
pub struct CoreXPGainFixtureRefsLikeCpp<'a> {
    player_xp: &'a mut u32,
    player_next_level_xp: &'a mut u32,
    player_position: &'a Option<Position>,
    battleground_type_id: &'a Option<u32>,
    battleground_map_id: &'a Option<u32>,
    battleground_status: &'a Option<u8>,
    battleground_queue_slots: &'a Vec<wow_entities::PlayerBattlegroundQueueSlotLikeCpp>,
    battleground_arena_team_id_invited: &'a u32,
    rest: &'a mut crate::session::RestMgrTestFixtureLikeCpp,
    aura_authority_complete: &'a bool,
    aura_spell_hit_tombstoned: &'a bool,
    visible_auras: &'a HashMap<u8, wow_entities::AuraApplicationLikeCpp>,
    threat_aura_snapshots: &'a HashMap<u8, wow_entities::AuraThreatSnapshotLikeCpp>,
}

#[cfg(any(test, feature = "test-fixtures"))]
impl<'a> CoreXPGainFixtureRefsLikeCpp<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn new_like_cpp(
        player_xp: &'a mut u32,
        player_next_level_xp: &'a mut u32,
        player_position: &'a Option<Position>,
        battleground_type_id: &'a Option<u32>,
        battleground_map_id: &'a Option<u32>,
        battleground_status: &'a Option<u8>,
        battleground_queue_slots: &'a Vec<wow_entities::PlayerBattlegroundQueueSlotLikeCpp>,
        battleground_arena_team_id_invited: &'a u32,
        rest: &'a mut crate::session::RestMgrTestFixtureLikeCpp,
        aura_authority_complete: &'a bool,
        aura_spell_hit_tombstoned: &'a bool,
        visible_auras: &'a HashMap<u8, wow_entities::AuraApplicationLikeCpp>,
        threat_aura_snapshots: &'a HashMap<u8, wow_entities::AuraThreatSnapshotLikeCpp>,
    ) -> Self {
        Self {
            player_xp,
            player_next_level_xp,
            player_position,
            battleground_type_id,
            battleground_map_id,
            battleground_status,
            battleground_queue_slots,
            battleground_arena_team_id_invited,
            rest,
            aura_authority_complete,
            aura_spell_hit_tombstoned,
            visible_auras,
            threat_aura_snapshots,
        }
    }

    fn reborrow_like_cpp(&mut self) -> CoreXPGainFixtureRefsLikeCpp<'_> {
        CoreXPGainFixtureRefsLikeCpp {
            player_xp: &mut *self.player_xp,
            player_next_level_xp: &mut *self.player_next_level_xp,
            player_position: self.player_position,
            battleground_type_id: self.battleground_type_id,
            battleground_map_id: self.battleground_map_id,
            battleground_status: self.battleground_status,
            battleground_queue_slots: self.battleground_queue_slots,
            battleground_arena_team_id_invited: self.battleground_arena_team_id_invited,
            rest: &mut *self.rest,
            aura_authority_complete: self.aura_authority_complete,
            aura_spell_hit_tombstoned: self.aura_spell_hit_tombstoned,
            visible_auras: self.visible_auras,
            threat_aura_snapshots: self.threat_aura_snapshots,
        }
    }

    pub fn player_position_fixture_like_cpp(&self) -> &Option<Position> {
        self.player_position
    }

    pub fn player_xp_fixture_like_cpp(&self) -> &u32 {
        self.player_xp
    }

    pub fn rest_mgr_fixture_like_cpp(&self) -> &crate::session::RestMgrTestFixtureLikeCpp {
        self.rest
    }

    pub fn rest_transition_fixture_refs_like_cpp(
        &mut self,
    ) -> (
        &Option<Position>,
        &u32,
        &mut crate::session::RestMgrTestFixtureLikeCpp,
    ) {
        (self.player_position, &self.player_next_level_xp, self.rest)
    }
}

/// Operation-specific Player XP access. It borrows the canonical owner and catalogs;
/// fixture values are supplied only to the individual read or write that needs them.
pub struct CoreXPGainAccessLikeCpp<'a> {
    core: &'a SessionCore,
    catalogs: &'a SessionCatalogs,
    config: &'a SessionWorldConfig,
}

impl SessionCore {
    pub fn xp_gain_access_like_cpp<'a>(
        &'a self,
        catalogs: &'a SessionCatalogs,
        config: &'a SessionWorldConfig,
    ) -> CoreXPGainAccessLikeCpp<'a> {
        CoreXPGainAccessLikeCpp {
            core: self,
            catalogs,
            config,
        }
    }

    pub(crate) fn resolved_player_xp_with_fixture_like_cpp(
        &self,
        #[cfg(any(test, feature = "test-fixtures"))] fixture_xp: &u32,
    ) -> Option<u32> {
        let canonical =
            self.with_owned_player_like_cpp(|player| player.active_data().xp.max(0) as u32);
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(*fixture_xp);
        }
        canonical
    }

    pub(crate) fn player_active_max_level_with_config_like_cpp(
        &self,
        config: &SessionWorldConfig,
    ) -> u32 {
        let expansion_max = u32::from(max_level_for_expansion_like_cpp(self.expansion));
        let configured_max = config.max_player_level_config_like_cpp;
        if expansion_max == 80 || expansion_max >= configured_max {
            configured_max
        } else {
            expansion_max
        }
    }

    pub(crate) fn resolved_player_next_level_xp_with_fixture_like_cpp(
        &self,
        #[cfg(any(test, feature = "test-fixtures"))] fixture_next_level_xp: &u32,
    ) -> Option<u32> {
        let canonical = self
            .with_owned_player_like_cpp(|player| player.active_data().next_level_xp.max(0) as u32);
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(*fixture_next_level_xp);
        }
        canonical
    }

    pub(crate) fn resolved_player_xp_for_level_with_fixture_like_cpp(
        &self,
        catalogs: &SessionCatalogs,
        level: u8,
        #[cfg(any(test, feature = "test-fixtures"))] fixture_next_level_xp: &u32,
    ) -> Option<u32> {
        let canonical = self
            .with_owned_player_like_cpp(|player| player.player_xp_for_level_like_cpp(level))
            .flatten();
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() {
            return catalogs
                .player_xp_table
                .as_ref()
                .and_then(|table| table.get(usize::from(level)).copied())
                .or_else(|| {
                    self.resolved_player_next_level_xp_with_fixture_like_cpp(fixture_next_level_xp)
                });
        }
        canonical
    }

    pub(crate) fn set_player_xp_with_fixture_like_cpp(
        &self,
        xp: u32,
        #[cfg(any(test, feature = "test-fixtures"))] fixture_xp: &mut u32,
    ) -> bool {
        let canonical = self
            .with_owned_player_mut_like_cpp(|player| {
                let xp = xp.min(i32::MAX as u32) as i32;
                player.set_xp(xp);
                player.mark_xp_changed_like_cpp();
                let scaling_level_delta = if player.unit().data().level
                    < i32::from(crate::session::WRATH_OF_THE_LICH_KING_MAX_LEVEL_LIKE_CPP)
                    && xp < player.active_data().next_level_xp / 2
                {
                    -1
                } else {
                    0
                };
                player.set_scaling_player_level_delta_like_cpp(scaling_level_delta);
                player.mark_scaling_player_level_delta_changed_like_cpp();
            })
            .is_some();
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical || self.player_handle_like_cpp.is_none() {
            *fixture_xp = xp;
        }
        canonical
            || cfg!(any(test, feature = "test-fixtures")) && self.player_handle_like_cpp.is_none()
    }

    pub(crate) fn set_player_next_level_xp_with_fixture_like_cpp(
        &self,
        xp: u32,
        #[cfg(any(test, feature = "test-fixtures"))] fixture_next_level_xp: &mut u32,
    ) -> bool {
        let canonical = self
            .with_owned_player_mut_like_cpp(|player| {
                let next_level_xp = xp.min(i32::MAX as u32) as i32;
                player.set_next_level_xp(next_level_xp);
                // Rust hydrates the Character row before its XP table refresh,
                // while C++ has NextLevelXP ready before `SetXP`. Recompute the
                // dependent SetXP field here so the final canonical value is
                // independent of that transitional load ordering.
                let scaling_level_delta = if player.unit().data().level
                    < i32::from(crate::session::WRATH_OF_THE_LICH_KING_MAX_LEVEL_LIKE_CPP)
                    && player.active_data().xp < next_level_xp / 2
                {
                    -1
                } else {
                    0
                };
                player.set_scaling_player_level_delta_like_cpp(scaling_level_delta);
            })
            .is_some();
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical || self.player_handle_like_cpp.is_none() {
            *fixture_next_level_xp = xp;
        }
        canonical
            || cfg!(any(test, feature = "test-fixtures")) && self.player_handle_like_cpp.is_none()
    }
}

impl CoreXPGainAccessLikeCpp<'_> {
    pub fn player_guid_like_cpp(&self) -> Option<ObjectGuid> {
        self.core.player_guid()
    }

    pub fn session_is_logged_in_like_cpp(&self) -> bool {
        self.core.state == crate::session::SessionState::LoggedIn
    }

    pub fn account_id_like_cpp(&self) -> u32 {
        self.core.account_id
    }

    pub fn recruiter_id_like_cpp(&self) -> u32 {
        self.core.recruiter_id_like_cpp()
    }

    pub fn player_group_owner_access_like_cpp(
        &self,
    ) -> crate::session::PlayerGroupOwnerAccessLikeCpp<'_> {
        self.core.player_group_owner_access_like_cpp()
    }

    pub fn player_map_id_like_cpp(&self) -> u16 {
        self.core.player_map_id_like_cpp()
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn owner_handle_absent_like_cpp(&self) -> bool {
        self.core.player_handle_like_cpp.is_none()
    }

    pub fn player_level_like_cpp(
        &self,
        #[cfg(any(test, feature = "test-fixtures"))] fixture_level: &u8,
    ) -> u8 {
        self.core.player_level_with_fixture_like_cpp(
            #[cfg(any(test, feature = "test-fixtures"))]
            fixture_level,
        )
    }

    pub fn player_is_max_level_like_cpp(
        &self,
        #[cfg(any(test, feature = "test-fixtures"))] fixture_level: &u8,
    ) -> bool {
        let player_level = self.player_level_like_cpp(
            #[cfg(any(test, feature = "test-fixtures"))]
            fixture_level,
        );
        u32::from(player_level)
            >= self
                .core
                .player_active_max_level_with_config_like_cpp(self.config)
    }

    pub fn player_is_at_configured_max_level_like_cpp(
        &self,
        #[cfg(any(test, feature = "test-fixtures"))] fixture_level: &u8,
    ) -> bool {
        let max_level = self.config.max_player_level_config_like_cpp;
        max_level != 0
            && u32::from(self.player_level_like_cpp(
                #[cfg(any(test, feature = "test-fixtures"))]
                fixture_level,
            )) >= max_level
    }

    pub fn resolved_player_is_alive_like_cpp(
        &self,
        #[cfg(any(test, feature = "test-fixtures"))] vitals: (&u32, &u32, &bool),
    ) -> Option<bool> {
        self.core
            .resolved_player_vitals_with_fixture_like_cpp(
                #[cfg(any(test, feature = "test-fixtures"))]
                vitals.0,
                #[cfg(any(test, feature = "test-fixtures"))]
                vitals.1,
                #[cfg(any(test, feature = "test-fixtures"))]
                vitals.2,
            )
            .map(|(_, _, alive)| alive)
    }

    pub fn player_in_represented_battleground_like_cpp(
        &self,
        #[cfg(any(test, feature = "test-fixtures"))] fixtures: &CoreXPGainFixtureRefsLikeCpp<'_>,
    ) -> bool {
        let canonical = self
            .core
            .with_owned_player_like_cpp(|player| player.battleground_state_like_cpp())
            .is_some_and(|state| state.in_battleground_like_cpp());
        #[cfg(any(test, feature = "test-fixtures"))]
        if !canonical && self.core.player_handle_like_cpp.is_none() {
            return wow_entities::PlayerBattlegroundState::from_represented_parts_like_cpp(
                *fixtures.battleground_type_id,
                *fixtures.battleground_map_id,
                *fixtures.battleground_status,
                fixtures.battleground_queue_slots.clone(),
                *fixtures.battleground_arena_team_id_invited,
            )
            .in_battleground_like_cpp();
        }
        canonical
    }

    pub fn player_position_like_cpp(
        &self,
        #[cfg(any(test, feature = "test-fixtures"))] fixture_position: &Option<Position>,
    ) -> Option<Position> {
        self.core.player_position_with_fixture_like_cpp(
            #[cfg(any(test, feature = "test-fixtures"))]
            fixture_position,
        )
    }

    pub fn player_instance_id_like_cpp(&self) -> Option<u32> {
        self.core
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
    }

    pub fn group_members_for_player_like_cpp(
        &self,
        group_guid: u64,
        player_guid: ObjectGuid,
    ) -> Option<Vec<ObjectGuid>> {
        let group_registry = self.core.directory.group_registry.as_ref()?;
        let _player_registry = self.core.player_registry.as_ref()?;
        let group = group_registry.get(&group_guid)?;
        if !group.members.contains(&player_guid) {
            return None;
        }
        let members = group.members.clone();
        drop(group);
        Some(members)
    }

    pub fn group_member_presence_like_cpp(
        &self,
        member_guid: ObjectGuid,
    ) -> Option<crate::session::directory::PlayerGroupPresenceSnapshot> {
        self.core
            .player_registry
            .as_ref()?
            .group_presence(member_guid)
    }

    pub fn represented_creature_has_loot_recipient_like_cpp(
        &self,
        creature_guid: ObjectGuid,
    ) -> Option<bool> {
        if let Some(manager) = self.core.map_manager.as_ref() {
            let manager = manager
                .read()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if let Some(creature) =
                manager.find_creature(self.core.player_map_id_like_cpp(), 0, creature_guid)
            {
                return Some(creature.creature.has_loot_recipient());
            }
        }

        let key = self
            .core
            .current_canonical_player_map_key_like_cpp()
            .unwrap_or(wow_map::MapKey::new(
                u32::from(self.core.player_map_id_like_cpp()),
                0,
            ));
        let manager = self.core.canonical_map_manager.as_ref()?.lock().ok()?;
        manager
            .find_map(key.map_id, key.instance_id)?
            .map()
            .with_creature_like_cpp(creature_guid, |creature| creature.has_loot_recipient())
    }

    pub fn resolved_player_xp_like_cpp(
        &self,
        #[cfg(any(test, feature = "test-fixtures"))] fixture_xp: &u32,
    ) -> Option<u32> {
        self.core.resolved_player_xp_with_fixture_like_cpp(
            #[cfg(any(test, feature = "test-fixtures"))]
            fixture_xp,
        )
    }

    pub fn resolved_player_next_level_xp_like_cpp(
        &self,
        #[cfg(any(test, feature = "test-fixtures"))] fixtures: &CoreXPGainFixtureRefsLikeCpp<'_>,
    ) -> Option<u32> {
        self.core
            .resolved_player_next_level_xp_with_fixture_like_cpp(
                #[cfg(any(test, feature = "test-fixtures"))]
                &fixtures.player_next_level_xp,
            )
    }

    pub fn resolved_player_xp_for_level_like_cpp(
        &self,
        level: u8,
        #[cfg(any(test, feature = "test-fixtures"))] fixtures: &CoreXPGainFixtureRefsLikeCpp<'_>,
    ) -> Option<u32> {
        self.core
            .resolved_player_xp_for_level_with_fixture_like_cpp(
                self.catalogs,
                level,
                #[cfg(any(test, feature = "test-fixtures"))]
                &fixtures.player_next_level_xp,
            )
    }

    pub fn set_player_xp_like_cpp(
        &self,
        xp: u32,
        #[cfg(any(test, feature = "test-fixtures"))] fixtures: &mut CoreXPGainFixtureRefsLikeCpp<
            '_,
        >,
    ) -> bool {
        self.core.set_player_xp_with_fixture_like_cpp(
            xp,
            #[cfg(any(test, feature = "test-fixtures"))]
            &mut fixtures.player_xp,
        )
    }

    pub fn set_player_next_level_xp_like_cpp(
        &self,
        xp: u32,
        #[cfg(any(test, feature = "test-fixtures"))] fixtures: &mut CoreXPGainFixtureRefsLikeCpp<
            '_,
        >,
    ) -> bool {
        self.core.set_player_next_level_xp_with_fixture_like_cpp(
            xp,
            #[cfg(any(test, feature = "test-fixtures"))]
            &mut fixtures.player_next_level_xp,
        )
    }

    pub fn resolved_total_represented_aura_modifier_like_cpp(
        &self,
        effect: wow_entities::RepresentedAuraEffectLikeCpp,
        #[cfg(any(test, feature = "test-fixtures"))] fixtures: &CoreXPGainFixtureRefsLikeCpp<'_>,
    ) -> Option<i32> {
        represented_total_aura_modifier_from_snapshot_like_cpp(
            self.core
                .player_aura_subsystem_snapshot_with_fixture_refs_like_cpp(
                    #[cfg(any(test, feature = "test-fixtures"))]
                    fixtures.aura_authority_complete,
                    #[cfg(any(test, feature = "test-fixtures"))]
                    fixtures.aura_spell_hit_tombstoned,
                    #[cfg(any(test, feature = "test-fixtures"))]
                    fixtures.visible_auras,
                    #[cfg(any(test, feature = "test-fixtures"))]
                    fixtures.threat_aura_snapshots,
                ),
            effect,
        )
    }

    pub fn resolved_player_flags_for_rest_state_save_like_cpp(&self) -> Option<u32> {
        self.core.with_owned_player_for_rest_like_cpp(|player| {
            let mut player_flags = player.data().player_flags;
            let rest = player.rest_state_like_cpp();
            if rest.is_location_initialized_like_cpp() {
                if rest.is_resting_by_flag_like_cpp() {
                    player_flags |= crate::session::PLAYER_FLAGS_RESTING_LIKE_CPP;
                } else {
                    player_flags &= !crate::session::PLAYER_FLAGS_RESTING_LIKE_CPP;
                }
            }
            player_flags
        })
    }

    pub fn take_xp_rest_bonus_like_cpp(
        &self,
        xp: u32,
        rested_consumption_modifier: i32,
        at_max_level: bool,
        recruit_a_friend: bool,
    ) -> (u32, u8) {
        self.core
            .with_owned_player_mut_like_cpp(|player| {
                player.take_xp_rest_bonus_like_cpp(
                    xp,
                    rested_consumption_modifier,
                    at_max_level,
                    recruit_a_friend,
                )
            })
            .unwrap_or((0, 0))
    }

    pub fn sync_represented_xp_level_to_canonical_and_client_like_cpp(
        &self,
        level_changed: bool,
        rest_info_mask: u8,
        #[cfg(any(test, feature = "test-fixtures"))] fixture_level: &u8,
        #[cfg(any(test, feature = "test-fixtures"))] fixtures: &CoreXPGainFixtureRefsLikeCpp<'_>,
    ) {
        if self.core.player_guid().is_none() {
            return;
        }

        let level = self.player_level_like_cpp(
            #[cfg(any(test, feature = "test-fixtures"))]
            fixture_level,
        );
        let (Some(xp), Some(next_level_xp), Some(scaling_player_level_delta)) = (
            self.resolved_player_xp_like_cpp(
                #[cfg(any(test, feature = "test-fixtures"))]
                &fixtures.player_xp,
            ),
            self.resolved_player_next_level_xp_like_cpp(
                #[cfg(any(test, feature = "test-fixtures"))]
                fixtures,
            ),
            self.resolved_player_scaling_level_delta_like_cpp(
                #[cfg(any(test, feature = "test-fixtures"))]
                fixture_level,
                #[cfg(any(test, feature = "test-fixtures"))]
                fixtures,
            ),
        ) else {
            return;
        };
        let xp = xp.min(i32::MAX as u32) as i32;
        let next_level_xp = next_level_xp.min(i32::MAX as u32) as i32;
        let owner_marked = self
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player.mark_xp_changed_like_cpp();
                player.mark_scaling_player_level_delta_changed_like_cpp();
            })
            .is_some();
        #[cfg(not(any(test, feature = "test-fixtures")))]
        if !owner_marked {
            return;
        }
        #[cfg(any(test, feature = "test-fixtures"))]
        if !owner_marked && self.core.player_handle_like_cpp.is_some() {
            return;
        }

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
                self.resolved_xp_rest_threshold_like_cpp(
                    #[cfg(any(test, feature = "test-fixtures"))]
                    &fixtures.rest,
                ),
                self.resolved_xp_rest_state_like_cpp(
                    #[cfg(any(test, feature = "test-fixtures"))]
                    &fixtures.rest,
                ),
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
        self.core.send_player_values_update_like_cpp(&update);
    }

    fn resolved_player_scaling_level_delta_like_cpp(
        &self,
        #[cfg(any(test, feature = "test-fixtures"))] fixture_level: &u8,
        #[cfg(any(test, feature = "test-fixtures"))] fixtures: &CoreXPGainFixtureRefsLikeCpp<'_>,
    ) -> Option<i32> {
        let canonical = self
            .core
            .with_owned_player_like_cpp(|player| player.active_data().scaling_player_level_delta);
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            let level = self.player_level_like_cpp(fixture_level);
            return Some(
                if level < crate::session::WRATH_OF_THE_LICH_KING_MAX_LEVEL_LIKE_CPP
                    && *fixtures.player_xp < *fixtures.player_next_level_xp / 2
                {
                    -1
                } else {
                    0
                },
            );
        }
        canonical
    }

    pub fn send_packet_realm_like_cpp(&self, packet: &impl wow_packet::ServerPacket) {
        self.core.send_packet_realm(packet);
    }
}
