// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Player binding: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::WorldSession;
use super::{Arc, DurableLootMoneyPersistenceTrackerLikeCpp, ObjectGuid};
pub(in crate::session) use wow_world_core::session::PlayerIdentityBootstrapLikeCpp;

#[derive(Debug, Clone)]
pub(crate) struct SessionPlayerController {
    pub(in crate::session) guid: ObjectGuid,
    pub(in crate::session) name: String,
    pub(in crate::session) position: wow_core::Position,
    pub(in crate::session) map_id: u16,
    pub(in crate::session) race: u8,
    pub(in crate::session) class: u8,
    pub(in crate::session) level: u8,
    pub(in crate::session) gender: u8,
}

impl SessionPlayerController {
    pub(crate) fn new(
        guid: ObjectGuid,
        name: String,
        position: wow_core::Position,
        map_id: u16,
        race: u8,
        class: u8,
        level: u8,
        gender: u8,
    ) -> Self {
        Self {
            guid,
            name,
            position,
            map_id,
            race,
            class,
            level,
            gender,
        }
    }

    pub(crate) fn guid(&self) -> ObjectGuid {
        self.guid
    }

    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn position(&self) -> wow_core::Position {
        self.position
    }

    pub(crate) fn map_id(&self) -> u16 {
        self.map_id
    }

    pub(crate) fn race(&self) -> u8 {
        self.race
    }

    pub(crate) fn class(&self) -> u8 {
        self.class
    }

    pub(crate) fn level(&self) -> u8 {
        self.level
    }

    pub(crate) fn gender(&self) -> u8 {
        self.gender
    }
}

impl WorldSession {
    /// Set the logged-in player GUID.
    pub fn set_player_guid(&mut self, guid: Option<ObjectGuid>) {
        let previous_player_guid = self.core.player_guid;
        let player_changed = self.core.player_guid != guid;
        if player_changed {
            let _ = crate::session::hub_mut(self).set_player_zone_area_authority_like_cpp(false);
            self.core
                .invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        }
        self.core.player_guid = guid;
        if player_changed {
            self.view
                .last_presented_creature_melee_health_state_revision_like_cpp = 0;
            // Visible auras and their completeness proof belong to the C++
            // Player, not the authenticated WorldSession. Clear both at the
            // identity boundary so a later character cannot inherit positive
            // or negative aura-spell authority from the previous one.
            let _canonical = self
                .core
                .with_owned_player_mut_like_cpp(|player| {
                    player.reset_player_aura_source_authority_like_cpp();
                })
                .is_some();
            #[cfg(test)]
            if !_canonical && self.core.player_handle_like_cpp.is_none() {
                let _ = self.mutate_player_aura_subsystem_like_cpp(|auras| {
                    auras.clear_runtime_applications_like_cpp();
                    auras.reset_player_aura_source_authority_like_cpp();
                });
            }
            self.begin_player_equipment_inventory_authority_load_like_cpp();
            #[cfg(test)]
            {
                self.quest_state
                    .quest_test_fixture_like_cpp
                    .player_quest_status_authority_complete_like_cpp = false;
                self.quest_state
                    .quest_test_fixture_like_cpp
                    .represented_rewarded_quest_rows_like_cpp
                    .clear();
                self.lifecycle
                    .player_flags_test_fixture_like_cpp
                    .represented_loaded_player_flags_like_cpp = None;
                self.lifecycle
                    .player_flags_test_fixture_like_cpp
                    .represented_loaded_player_flags_ex_like_cpp = None;
                self.lifecycle
                    .player_flags_test_fixture_like_cpp
                    .represented_loaded_player_flags_applied_like_cpp = false;
            }
            #[cfg(test)]
            self.social.clear_represented_guild_identity_for_test_like_cpp();
            let _ = self.clear_represented_trait_config_rows_like_cpp();
            let _ =
                crate::session::hub_mut(self).update_player_pet_lifecycle_state_like_cpp(|state| {
                    state.character_rows_empty_authority_complete = false;
                });
            // C++ owns PlayerMenu (and therefore both InteractionData and its
            // menus) under Player. A WorldSession can survive character
            // logout, so no player-menu state may cross that lifetime here.
            {
                let (s, h) = crate::session::split_interaction(self);
                s.reset_player_interaction_data_like_cpp(h)
            };
            self.clear_player_gossip_options_like_cpp();
            #[cfg(test)]
            self.spell_state
                .clear_spell_acquisition_post_commit_actions_for_test_like_cpp();
            if previous_player_guid.is_some() {
                let _ = self.clear_represented_fallback_spell_rows_like_cpp();
            }
            // `_SaveSkills` tombstones belong to the current C++ Player's
            // update-field slots, not to the authenticated WorldSession.
            if previous_player_guid.is_some() {
                self.clear_player_skill_tombstones_like_cpp();
            }
        }
        if let Some(guid) = guid {
            self.core.account_state.recent_player_guid_low_like_cpp = guid.counter() as u64;
            self.visibility.last_observed_farsight_object_like_cpp = wow_core::ObjectGuid::EMPTY;
            #[cfg(test)]
            {
                self.visibility
                    .visibility_test_fixture_like_cpp
                    .represented_seer_guid_like_cpp = Some(guid);
            }
        }
        if guid.is_none() {
            self.core.player_identity_bootstrap_like_cpp = None;
            #[cfg(test)]
            {
                self.core.player_bootstrap_attached_like_cpp = false;
            }
            #[cfg(test)]
            {
                self.visibility
                    .visibility_test_fixture_like_cpp
                    .represented_seer_guid_like_cpp = None;
            }
            self.visibility.last_observed_farsight_object_like_cpp = wow_core::ObjectGuid::EMPTY;
            // Old registry clones remain permanently closed; a later character
            // selected on this authenticated session receives a fresh fence.
            self.lifecycle.durable_loot_money_persistence_like_cpp =
                Arc::new(DurableLootMoneyPersistenceTrackerLikeCpp::default());
        }
    }

    pub(crate) fn attach_player_controller_like_cpp(
        &mut self,
        controller: SessionPlayerController,
    ) {
        let controller_position = controller.position();
        self.set_player_guid(Some(controller.guid()));
        self.core.player_identity_bootstrap_like_cpp = Some(PlayerIdentityBootstrapLikeCpp {
            name: Some(controller.name().to_string()),
            race: controller.race(),
            class: controller.class(),
            level: controller.level(),
            gender: controller.gender(),
        });
        #[cfg(any(test, feature = "test-fixtures"))]
        {
            self.fixtures.identity.player_name = Some(controller.name().to_string());
            self.fixtures.movement.player_position = Some(controller_position);
        }
        self.core.current_map_id = controller.map_id();
        #[cfg(any(test, feature = "test-fixtures"))]
        {
            self.fixtures.identity.player_race = controller.race();
            self.fixtures.identity.player_class = controller.class();
            self.fixtures.identity.player_level = controller.level();
            self.fixtures.identity.player_gender = controller.gender();
        }
        #[cfg(test)]
        {
            self.visibility
                .visibility_test_fixture_like_cpp
                .represented_seer_guid_like_cpp = Some(controller.guid());
        }
        self.visibility.last_observed_farsight_object_like_cpp = wow_core::ObjectGuid::EMPTY;
        #[cfg(test)]
        {
            self.core.player_bootstrap_attached_like_cpp = true;
        }
        crate::session::hub_mut(self).initialize_reputation_mgr_like_cpp();
        crate::session::hub_mut(self).set_fall_information_like_cpp(0, controller_position.z);
        // Production receives MapManager at session construction, so consume
        // the login bootstrap immediately. Unit fixtures historically inject
        // or replace their synthetic manager after attachment; they exercise
        // the same ownership transition through
        // `ensure_canonical_player_owner_for_map_like_cpp` instead.
        #[cfg(not(test))]
        let _ = self.install_detached_canonical_player_from_session_like_cpp(controller_position);
        self.set_player_moved_unit_guid_like_cpp(controller.guid());
    }

    pub(crate) fn set_player_liquid_status_like_cpp(&mut self, status: u32) {
        crate::canonical_player_sync::sync_player_liquid_status_like_cpp(self, status);
    }

    pub(crate) fn set_player_level_like_cpp(&mut self, level: u8) {
        let gray_level = crate::session::hub_ref(self).gray_level(level);
        let canonical = self
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player.set_level_and_gray_level_like_cpp(level, gray_level);
            })
            .is_some();
        if !canonical {
            #[cfg(not(test))]
            if self.core.player_handle_like_cpp.is_some() {
                return;
            }
            self.core
                .player_identity_bootstrap_like_cpp
                .get_or_insert_default()
                .level = level;
            #[cfg(any(test, feature = "test-fixtures"))]
            {
                self.fixtures.identity.player_level = level;
            }
        }
        self.refresh_represented_talent_points_like_cpp();
    }

    #[cfg(test)]
    pub(crate) fn set_player_class_like_cpp(&mut self, class: u8) {
        if crate::session::hub_ref(self).player_class_like_cpp() != class {
            self.core
                .invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        }
        let canonical = self
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player.unit_mut().set_class(class);
                player.unit_mut().set_player_class(class);
            })
            .is_some();
        if !canonical {
            self.core
                .player_identity_bootstrap_like_cpp
                .get_or_insert_default()
                .class = class;
            self.fixtures.identity.player_class = class;
        }
        self.refresh_represented_talent_points_like_cpp();
    }

    pub(crate) fn set_player_gold_like_cpp(&mut self, gold: u64) -> bool {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.set_player_gold_like_cpp(&mut hub, gold)
    }

    #[inline]
    pub fn player_guid(&self) -> Option<ObjectGuid> {
        self.core.player_guid()
    }
}

impl crate::session::state::InventoryState {
    pub(crate) fn set_player_gold_like_cpp(
        &mut self,
        hub: &mut crate::session::HubMut<'_>,
        gold: u64,
    ) -> bool {
        let canonical = hub
            .core
            .with_owned_player_mut_like_cpp(|player| player.set_money(gold))
            .is_some();
        #[cfg(test)]
        if canonical || hub.core.player_handle_like_cpp.is_none() {
            self.player_gold = gold;
        }
        canonical || cfg!(test) && hub.core.player_handle_like_cpp.is_none()
    }
}

#[cfg(test)]
#[path = "../../unit_tests/session/player_binding/f3_shims.rs"]
mod f3_shims;
