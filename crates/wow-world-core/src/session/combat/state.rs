// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Canonical combat state adapters shared with World.

use crate::session::begin_combat_ref_on_map_like_cpp;
use std::sync::{Arc, atomic::Ordering};
use wow_constants::UnitPvpFlags;
use wow_core::ObjectGuid;

pub const PLAYER_FLAGS_IN_PVP_LIKE_CPP: u32 = 0x0000_0200;
pub const SPELL_PVP_RULES_ENABLED_LIKE_CPP: i32 = 134_735;

impl crate::session::HubMut<'_> {
    pub fn update_player_pvp_like_cpp(
        &mut self,
        state: bool,
        override_state: bool,
    ) {
        // C++ `Player::UpdatePvP` (Player.cpp:22663) is a Player transition:
        // the flag and the timer move together and this session only asks for
        // it.
        let now_secs = wow_entities::game_time_secs_like_cpp();
        #[cfg_attr(not(any(test, feature = "test-fixtures")), allow(unused_mut))]
        let mut mutated = self.core.with_owned_player_mut_like_cpp(|player| {
            player.update_pvp_like_cpp(state, now_secs, override_state);
        });
        #[cfg(any(test, feature = "test-fixtures"))]
        if mutated.is_none()
            && self.core.player_handle_like_cpp.is_none()
            && let Some(guid) = self.core.player_guid()
        {
            mutated = self
                .core
                .mutate_canonical_player_by_guid_like_cpp(guid, |player| {
                    player.update_pvp_like_cpp(state, now_secs, override_state);
                });
        }
        #[cfg(any(test, feature = "test-fixtures"))]
        if self.core.player_handle_like_cpp.is_none() {
            self.fixtures.combat.player_pvp_end_timer_like_cpp = if !state || override_state {
                None
            } else {
                Some(now_secs)
            };
            self.fixtures.combat.player_pvp_enabled_like_cpp = state;
        }
        let _ = mutated;
    }

    pub fn begin_canonical_player_combat_ref_like_cpp(
        &mut self,
        attacker_guid: ObjectGuid,
        victim_guid: ObjectGuid,
        relation_represented: bool,
        attacker_is_friendly_to_victim: bool,
        victim_is_friendly_to_attacker: bool,
    ) -> bool {
        let Some(map_key) = self.core.current_canonical_player_map_key_like_cpp() else {
            return false;
        };
        let Some(manager) = self.core.canonical_map_manager.as_ref().cloned() else {
            return false;
        };
        let Ok(mut manager) = manager.lock() else {
            return false;
        };
        let Some(managed) = manager.find_map_mut(map_key.map_id, map_key.instance_id) else {
            return false;
        };
        begin_combat_ref_on_map_like_cpp(
            managed.map_mut(),
            attacker_guid,
            victim_guid,
            relation_represented,
            attacker_is_friendly_to_victim,
            victim_is_friendly_to_attacker,
        )
    }

    pub fn revalidate_canonical_player_combat_refs_like_cpp(
        &mut self,
        player_guid: ObjectGuid,
    ) {
        let Some(map_key) = self.core.current_canonical_player_map_key_like_cpp() else {
            return;
        };
        let Some(manager) = self.core.canonical_map_manager.as_ref().cloned() else {
            return;
        };
        let Ok(mut manager) = manager.lock() else {
            return;
        };
        let Some(managed) = manager.find_map_mut(map_key.map_id, map_key.instance_id) else {
            return;
        };
        if managed.map().get_typed_player(player_guid).is_some() {
            managed.map_mut().revalidate_all_combat_refs_like_cpp();
        }
    }

    pub fn represented_set_advanced_combat_logging_like_cpp(&mut self, enable: bool) {
        self.core
            .flags
            .advanced_combat_logging_enabled_like_cpp
            .store(enable, Ordering::Relaxed);
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    fn set_player_pvp_hostile_fixture_like_cpp(&mut self, hostile: bool) {
        let _ = self.mutate_player_world_local_state_like_cpp(|state| {
            state.set_pvp_hostile_like_cpp(hostile);
        });
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_player_pvp_state_like_cpp(
        &mut self,
        hostile: bool,
        pvp_enabled: bool,
        in_pvp_flag: bool,
    ) {
        self.set_player_pvp_hostile_fixture_like_cpp(hostile);
        self.update_player_pvp_like_cpp(pvp_enabled, true);
        if let Some(guid) = self.core.player_guid() {
            let _ = self.core.with_owned_player_mut_like_cpp(|player| {
                if in_pvp_flag {
                    player.set_player_flag(PLAYER_FLAGS_IN_PVP_LIKE_CPP);
                } else {
                    player.remove_player_flag(PLAYER_FLAGS_IN_PVP_LIKE_CPP);
                }
            });
            let _ = guid;
        }
        if self.core.player_handle_like_cpp.is_none() {
            self.fixtures.combat.player_pvp_enabled_like_cpp = pvp_enabled;
            self.fixtures.combat.player_in_pvp_flag_like_cpp = in_pvp_flag;
        }
    }
}

impl crate::session::HubRef<'_> {
    pub fn player_is_pvp_like_cpp(&self, guid: ObjectGuid) -> Option<bool> {
        if self.core.player_guid() != Some(guid) {
            return None;
        }
        let canonical = self.core.with_owned_player_like_cpp(|player| {
            player
                .unit()
                .pvp_flags_like_cpp()
                .contains(UnitPvpFlags::PVP)
        });
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            if let Some(flags) = self.canonical_player_pvp_flags_like_cpp(guid) {
                return Some(flags.contains(UnitPvpFlags::PVP));
            }
            return Some(self.fixtures.combat.player_pvp_enabled_like_cpp);
        }
        canonical
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn canonical_player_pvp_flags_like_cpp(
        &self,
        guid: ObjectGuid,
    ) -> Option<UnitPvpFlags> {
        if self.core.player_guid() == Some(guid)
            && let Some(flags) = self
                .core
                .with_owned_player_like_cpp(|player| player.unit().pvp_flags_like_cpp())
        {
            return Some(flags);
        }
        let map_id = u32::from(self.core.player_map_id_like_cpp());
        let manager = Arc::clone(self.core.canonical_map_manager.as_ref()?);
        let manager = manager.lock().ok()?;
        let mut result = None;
        manager.do_for_all_maps_with_map_id(map_id, |managed| {
            if result.is_none() {
                result = managed
                    .map()
                    .get_typed_player(guid)
                    .map(|player| player.unit().pvp_flags_like_cpp());
            }
        });
        result
    }

    pub fn player_has_in_pvp_flag_like_cpp(
        &self,
        guid: ObjectGuid,
    ) -> Option<bool> {
        if self.core.player_guid() != Some(guid) {
            return None;
        }
        let canonical = self.core.with_owned_player_like_cpp(|player| {
            player.has_player_flag(PLAYER_FLAGS_IN_PVP_LIKE_CPP)
        });
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            if let Some(value) = self
                .core
                .canonical_player_has_player_flag_like_cpp(guid, PLAYER_FLAGS_IN_PVP_LIKE_CPP)
            {
                return Some(value);
            }
            return Some(self.fixtures.combat.player_in_pvp_flag_like_cpp);
        }
        canonical
    }

    pub fn represented_has_pvp_rules_enabled_like_cpp(&self) -> bool {
        self.player_has_visible_aura_spell_like_cpp(SPELL_PVP_RULES_ENABLED_LIKE_CPP)
            .unwrap_or(false)
    }

    pub fn resolved_combat_target_like_cpp(&self) -> Option<Option<ObjectGuid>> {
        let canonical = self
            .core
            .with_owned_player_like_cpp(|player| player.unit().attacking());
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            return Some(self.fixtures.combat.combat_target);
        }
        canonical
    }

    pub fn represented_advanced_combat_logging_enabled_like_cpp(&self) -> bool {
        self.core
            .flags
            .advanced_combat_logging_enabled_like_cpp
            .load(Ordering::Relaxed)
    }
}

impl crate::session::HubRef<'_> {
    pub fn resolved_in_combat_like_cpp(&self) -> Option<bool> {
        let canonical = self
            .core
            .with_owned_player_like_cpp(|player| player.unit().subsystems().combat.has_combat());
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            return Some(self.fixtures.combat.in_combat);
        }
        canonical
    }
}

impl crate::session::HubMut<'_> {
    pub fn set_combat_target_like_cpp(&mut self, target: Option<ObjectGuid>) -> bool {
        let canonical = self
            .core
            .with_owned_player_mut_like_cpp(|player| player.unit_mut().set_attacking(target))
            .is_some();
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical || self.core.player_handle_like_cpp.is_none() {
            self.fixtures.combat.combat_target = target;
        }
        canonical
            || cfg!(any(test, feature = "test-fixtures"))
                && self.core.player_handle_like_cpp.is_none()
    }

    /// Publish C++ `CombatManager::HasCombat` from the canonical Player to the
    /// bounded directory view. The argument remains only for pre-owner tests;
    /// production never manufactures combat state outside `CombatSubsystem`.
    pub fn set_in_combat_like_cpp(&mut self, in_combat: bool) {
        #[cfg(any(test, feature = "test-fixtures"))]
        if self.core.player_handle_like_cpp.is_none() {
            self.fixtures.combat.in_combat = in_combat;
            if let (Some(guid), Some(registry)) =
                (self.core.player_guid(), &self.core.player_registry)
            {
                registry.publish_in_combat_for_control_channel(
                    guid,
                    &self.core.session_command_tx,
                    in_combat,
                );
            }
            return;
        }
        let canonical = self.shared().resolved_in_combat_like_cpp();
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let _ = in_combat;
        let Some(in_combat) = canonical else {
            return;
        };
        if let (Some(guid), Some(registry)) = (self.core.player_guid(), &self.core.player_registry)
        {
            registry.publish_in_combat_for_control_channel(
                guid,
                &self.core.session_command_tx,
                in_combat,
            );
        }
    }
}

impl crate::session::state::SessionCatalogs {
    pub fn combat_rating_multiplier_like_cpp(&self, level: u8, rating: u32) -> f32 {
        self.combat_ratings_game_table
            .as_ref()
            .map(|table| table.rating_multiplier_like_cpp(u16::from(level), rating))
            .unwrap_or(1.0)
    }

    pub fn mana_regen_ratio_like_cpp(&self, level: u8, class: u8) -> f32 {
        self.regen_game_tables
            .as_ref()
            .map(|tables| tables.mana_regen_ratio_like_cpp(u16::from(level), class))
            .unwrap_or(0.0)
    }
}
