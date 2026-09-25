// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Player binding: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::PLAYER_LOCAL_FLAG_OVERRIDE_TRANSPORT_SERVER_TIME_LIKE_CPP;
use super::{Arc, DurableLootMoneyPersistenceTrackerLikeCpp, ObjectGuid};
use super::{PLAYER_LOCAL_FLAG_WAR_MODE_LIKE_CPP, Player, WorldSession};

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

#[cfg(test)]
pub(in crate::session) struct PlayerTransportLoginStateLikeCpp {
    pub(in crate::session) info: wow_packet::packets::movement::TransportInfo,
}

/// Login-only identity input consumed while the canonical Player is being
/// constructed. Once a generation-checked Player exists this value is retired;
/// it is not a second runtime identity authority.
#[derive(Debug, Clone, Default)]
pub(in crate::session) struct PlayerIdentityBootstrapLikeCpp {
    pub(in crate::session) name: Option<String>,
    pub(in crate::session) race: u8,
    pub(in crate::session) class: u8,
    pub(in crate::session) level: u8,
    pub(in crate::session) gender: u8,
}

impl WorldSession {
    pub(in crate::session) fn player_can_never_see_target_like_cpp(&self) -> bool {
        self.active_player_update_state_like_cpp()
            .map(|(flags, _, _)| {
                flags & PLAYER_LOCAL_FLAG_OVERRIDE_TRANSPORT_SERVER_TIME_LIKE_CPP == 0
            })
            .unwrap_or(true)
    }

    pub(crate) fn set_canonical_chosen_title_like_cpp(
        &mut self,
        title_id: i32,
    ) -> Option<wow_entities::PlayerValuesUpdate> {
        self.mutate_canonical_player_like_cpp(|player| {
            player.set_chosen_title_like_cpp(title_id);
            player.values_update(true)
        })
    }

    pub(in crate::session) fn player_world_local_state_like_cpp(
        &self,
    ) -> Option<wow_entities::PlayerWorldLocalState> {
        let canonical =
            self.with_owned_player_like_cpp(|player| player.gameplay_state().world_local);
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(
                wow_entities::PlayerWorldLocalState::from_represented_parts_like_cpp(
                    self.player_zone_id_like_cpp,
                    self.player_area_id_like_cpp,
                    self.player_zone_area_authority_complete_like_cpp,
                    self.player_pvp_hostile_like_cpp,
                    self.player_pvp_end_timer_like_cpp,
                    self.player_contested_pvp_timer_like_cpp,
                    self.represented_is_outdoors_like_cpp,
                ),
            );
        }
        canonical
    }

    pub(in crate::session) fn player_war_mode_local_active_like_cpp(&self) -> bool {
        self.active_player_update_state_like_cpp()
            .is_some_and(|(flags, _, _)| flags & PLAYER_LOCAL_FLAG_WAR_MODE_LIKE_CPP != 0)
    }

    pub(crate) fn player_is_possessing_like_cpp(&self) -> bool {
        let Some(player_guid) = self.player_guid else {
            return false;
        };
        let map_id = u32::from(self.player_map_id_like_cpp());
        let Some(manager) = self.canonical_map_manager.as_ref() else {
            return false;
        };
        let Ok(manager) = manager.lock() else {
            return false;
        };

        let mut result = None;
        manager.do_for_all_maps_with_map_id(map_id, |managed| {
            if result.is_some() {
                return;
            }

            let map = managed.map();
            let Some(player) = map.get_typed_player(player_guid) else {
                return;
            };
            let Some(charmed_guid) = player.unit().subsystems().control.charmed_guid else {
                result = Some(false);
                return;
            };

            let target_possessed_by_player = map
                .get_typed_player(charmed_guid)
                .map(|target| {
                    let control = &target.unit().subsystems().control;
                    control.charmer_guid == Some(player_guid) && control.is_possessed()
                })
                .or_else(|| {
                    map.with_creature_like_cpp(charmed_guid, |target| {
                        let control = &target.unit().subsystems().control;
                        control.charmer_guid == Some(player_guid) && control.is_possessed()
                    })
                })
                .unwrap_or(false);

            result = Some(target_possessed_by_player);
        });

        result.unwrap_or(false)
    }

    pub(crate) fn represented_player_charmed_guid_like_cpp(&self) -> ObjectGuid {
        let Some(player_guid) = self.player_guid else {
            return ObjectGuid::EMPTY;
        };
        let map_id = u32::from(self.player_map_id_like_cpp());
        let Some(manager) = self.canonical_map_manager.as_ref() else {
            return ObjectGuid::EMPTY;
        };
        let Ok(manager) = manager.lock() else {
            return ObjectGuid::EMPTY;
        };

        let mut result = None;
        manager.do_for_all_maps_with_map_id(map_id, |managed| {
            if result.is_some() {
                return;
            }

            let Some(player) = managed.map().get_typed_player(player_guid) else {
                return;
            };
            result = Some(
                player
                    .unit()
                    .subsystems()
                    .control
                    .charmed_guid
                    .unwrap_or(ObjectGuid::EMPTY),
            );
        });

        result.unwrap_or(ObjectGuid::EMPTY)
    }

    /// Set the logged-in player GUID.
    pub fn set_player_guid(&mut self, guid: Option<ObjectGuid>) {
        let previous_player_guid = self.player_guid;
        let player_changed = self.player_guid != guid;
        if player_changed {
            let _ = self.set_player_zone_area_authority_like_cpp(false);
            self.invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        }
        self.player_guid = guid;
        if player_changed {
            self.last_presented_creature_melee_health_state_revision_like_cpp = 0;
            // Visible auras and their completeness proof belong to the C++
            // Player, not the authenticated WorldSession. Clear both at the
            // identity boundary so a later character cannot inherit positive
            // or negative aura-spell authority from the previous one.
            let _canonical = self
                .with_owned_player_mut_like_cpp(|player| {
                    player.reset_player_aura_source_authority_like_cpp();
                })
                .is_some();
            #[cfg(test)]
            if !_canonical && self.player_handle_like_cpp.is_none() {
                let _ = self.mutate_player_aura_subsystem_like_cpp(|auras| {
                    auras.clear_runtime_applications_like_cpp();
                    auras.reset_player_aura_source_authority_like_cpp();
                });
            }
            self.begin_player_equipment_inventory_authority_load_like_cpp();
            #[cfg(test)]
            {
                self.quest_test_fixture_like_cpp
                    .player_quest_status_authority_complete_like_cpp = false;
                self.quest_test_fixture_like_cpp
                    .represented_rewarded_quest_rows_like_cpp
                    .clear();
                self.player_flags_test_fixture_like_cpp
                    .represented_loaded_player_flags_like_cpp = None;
                self.player_flags_test_fixture_like_cpp
                    .represented_loaded_player_flags_ex_like_cpp = None;
                self.player_flags_test_fixture_like_cpp
                    .represented_loaded_player_flags_applied_like_cpp = false;
            }
            #[cfg(test)]
            {
                self.guild_test_fixture_like_cpp
                    .represented_guild_id_like_cpp = 0;
                self.guild_test_fixture_like_cpp
                    .represented_guild_id_authority_complete_like_cpp = false;
            }
            let _ = self.clear_represented_trait_config_rows_like_cpp();
            let _ = self.update_player_pet_lifecycle_state_like_cpp(|state| {
                state.character_rows_empty_authority_complete = false;
            });
            // C++ owns PlayerMenu (and therefore both InteractionData and its
            // menus) under Player. A WorldSession can survive character
            // logout, so no player-menu state may cross that lifetime here.
            self.reset_player_interaction_data_like_cpp();
            self.clear_player_gossip_options_like_cpp();
            #[cfg(test)]
            self.represented_spell_acquisition_post_commit_actions_like_cpp
                .clear();
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
            self.recent_player_guid_low_like_cpp = guid.counter() as u64;
            self.last_observed_farsight_object_like_cpp = wow_core::ObjectGuid::EMPTY;
            #[cfg(test)]
            {
                self.visibility_test_fixture_like_cpp
                    .represented_seer_guid_like_cpp = Some(guid);
            }
        }
        if guid.is_none() {
            self.player_identity_bootstrap_like_cpp = None;
            #[cfg(test)]
            {
                self.player_bootstrap_attached_like_cpp = false;
            }
            #[cfg(test)]
            {
                self.visibility_test_fixture_like_cpp
                    .represented_seer_guid_like_cpp = None;
            }
            self.last_observed_farsight_object_like_cpp = wow_core::ObjectGuid::EMPTY;
            // Old registry clones remain permanently closed; a later character
            // selected on this authenticated session receives a fresh fence.
            self.durable_loot_money_persistence_like_cpp =
                Arc::new(DurableLootMoneyPersistenceTrackerLikeCpp::default());
        }
    }

    /// C++ `Player::SetFactionForRace`: `Player::LoadFromDB` resolves the
    /// player's live faction template from `ChrRacesEntry::FactionID` before
    /// the player is added to the map or published through ObjectAccessor.
    pub(in crate::session) fn set_player_faction_for_race_like_cpp(&mut self, race: u8) {
        let Some(chr_races_store) = self.chr.races_store.as_ref() else {
            return;
        };
        let faction_template = chr_races_store
            .get(u32::from(race))
            .and_then(|entry| u32::try_from(entry.faction_id).ok())
            .filter(|faction_template| *faction_template != 0);
        let faction_template = faction_template.unwrap_or(0);
        let _canonical = self.with_owned_player_mut_like_cpp(|player| {
            player.unit_mut().set_faction(faction_template);
        });
        #[cfg(test)]
        if _canonical.is_some() || self.player_handle_like_cpp.is_none() {
            self.player_faction_template_like_cpp =
                (faction_template != 0).then_some(faction_template);
        }
    }

    pub(crate) fn attach_player_controller_like_cpp(
        &mut self,
        controller: SessionPlayerController,
    ) {
        let controller_position = controller.position();
        self.set_player_guid(Some(controller.guid()));
        self.player_identity_bootstrap_like_cpp = Some(PlayerIdentityBootstrapLikeCpp {
            name: Some(controller.name().to_string()),
            race: controller.race(),
            class: controller.class(),
            level: controller.level(),
            gender: controller.gender(),
        });
        #[cfg(test)]
        {
            self.player_name = Some(controller.name().to_string());
            self.player_position = Some(controller_position);
        }
        self.current_map_id = controller.map_id();
        #[cfg(test)]
        {
            self.player_race = controller.race();
            self.player_class = controller.class();
            self.player_level = controller.level();
            self.player_gender = controller.gender();
        }
        #[cfg(test)]
        {
            self.visibility_test_fixture_like_cpp
                .represented_seer_guid_like_cpp = Some(controller.guid());
        }
        self.last_observed_farsight_object_like_cpp = wow_core::ObjectGuid::EMPTY;
        #[cfg(test)]
        {
            self.player_bootstrap_attached_like_cpp = true;
        }
        self.initialize_reputation_mgr_like_cpp();
        self.set_fall_information_like_cpp(0, controller_position.z);
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
        let gray_level = self.gray_level(level);
        let canonical = self
            .with_owned_player_mut_like_cpp(|player| {
                player.set_level_and_gray_level_like_cpp(level, gray_level);
            })
            .is_some();
        if !canonical {
            #[cfg(not(test))]
            if self.player_handle_like_cpp.is_some() {
                return;
            }
            self.player_identity_bootstrap_like_cpp
                .get_or_insert_default()
                .level = level;
            #[cfg(test)]
            {
                self.player_level = level;
            }
        }
        self.refresh_represented_talent_points_like_cpp();
    }

    #[cfg(test)]
    pub(crate) fn set_player_class_like_cpp(&mut self, class: u8) {
        if self.player_class_like_cpp() != class {
            self.invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        }
        let canonical = self
            .with_owned_player_mut_like_cpp(|player| {
                player.unit_mut().set_class(class);
                player.unit_mut().set_player_class(class);
            })
            .is_some();
        if !canonical {
            self.player_identity_bootstrap_like_cpp
                .get_or_insert_default()
                .class = class;
            self.player_class = class;
        }
        self.refresh_represented_talent_points_like_cpp();
    }

    pub(crate) fn set_player_create_mode_like_cpp(&mut self, create_mode: u8) -> bool {
        let _canonical = self
            .with_owned_player_mut_like_cpp(|player| player.set_create_mode_like_cpp(create_mode))
            .is_some();
        #[cfg(test)]
        if _canonical || self.player_handle_like_cpp.is_none() {
            self.player_create_mode_like_cpp = create_mode;
            return true;
        }
        _canonical
    }

    pub(crate) fn set_player_gold_like_cpp(&mut self, gold: u64) -> bool {
        let canonical = self
            .with_owned_player_mut_like_cpp(|player| player.set_money(gold))
            .is_some();
        #[cfg(test)]
        if canonical || self.player_handle_like_cpp.is_none() {
            self.player_gold = gold;
        }
        canonical || cfg!(test) && self.player_handle_like_cpp.is_none()
    }

    #[cfg(test)]
    pub(crate) fn set_player_character_points_like_cpp(&mut self, points: i32) -> bool {
        let canonical = self
            .with_owned_player_mut_like_cpp(|player| {
                player.set_character_points_like_cpp(points);
            })
            .is_some();
        #[cfg(test)]
        if canonical || self.player_handle_like_cpp.is_none() {
            self.player_character_points_like_cpp = points;
        }
        canonical || cfg!(test) && self.player_handle_like_cpp.is_none()
    }

    pub(crate) fn player_name_like_cpp(&self) -> Option<String> {
        if let Some(name) =
            self.with_owned_player_like_cpp(|player| player.unit().world().name().to_owned())
        {
            return Some(name);
        }
        #[cfg(test)]
        {
            return self.player_name.clone();
        }
        if self.player_handle_like_cpp.is_some() {
            return None;
        }
        self.player_identity_bootstrap_like_cpp
            .as_ref()
            .and_then(|identity| identity.name.clone())
    }

    pub(crate) fn player_faction_template_id_like_cpp(&self) -> Option<u32> {
        let canonical = self.with_owned_player_like_cpp(|player| {
            u32::try_from(player.unit().data().faction_template)
                .ok()
                .filter(|faction| *faction != 0)
        });
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return self.player_faction_template_like_cpp;
        }
        canonical.flatten()
    }

    #[cfg(test)]
    pub(crate) fn represented_can_swim_to_fly_transition_like_cpp(&self) -> bool {
        self.resolved_can_swim_to_fly_transition_like_cpp()
            .expect("test Player movement owner must resolve")
    }

    pub(in crate::session) fn resolved_can_swim_to_fly_transition_like_cpp(&self) -> Option<bool> {
        let canonical = self.with_owned_player_like_cpp(|player| {
            player
                .gameplay_state()
                .movement_control
                .can_swim_to_fly_transition
        });
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(self.represented_can_swim_to_fly_transition_like_cpp);
        }
        canonical
    }

    pub(in crate::session) fn resolved_player_scale_duration_like_cpp(&self) -> Option<i32> {
        let canonical = self.with_owned_player_like_cpp(|player| {
            player.gameplay_state().movement_control.scale_duration
        });
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(self.player_scale_duration_like_cpp);
        }
        canonical
    }

    pub(crate) fn player_liquid_status_like_cpp(&self) -> Option<u32> {
        self.canonical_player_snapshot_like_cpp(|player| player.gameplay_state().liquid_status)
    }

    pub(crate) fn player_race_like_cpp(&self) -> u8 {
        if let Some(race) = self.with_owned_player_like_cpp(|player| player.race_like_cpp()) {
            return race;
        }
        #[cfg(test)]
        {
            return self.player_race;
        }
        #[cfg(not(test))]
        if self.player_handle_like_cpp.is_some() {
            return 0;
        }
        self.player_identity_bootstrap_like_cpp
            .as_ref()
            .map(|identity| identity.race)
            .unwrap_or_default()
    }

    pub(crate) fn player_class_like_cpp(&self) -> u8 {
        if let Some(class) = self.with_owned_player_like_cpp(|player| player.class_like_cpp()) {
            return class;
        }
        #[cfg(test)]
        {
            return self.player_class;
        }
        #[cfg(not(test))]
        if self.player_handle_like_cpp.is_some() {
            return 0;
        }
        self.player_identity_bootstrap_like_cpp
            .as_ref()
            .map(|identity| identity.class)
            .unwrap_or_default()
    }

    pub(crate) fn player_create_mode_like_cpp(&self) -> Option<u8> {
        let canonical = self.with_owned_player_like_cpp(Player::create_mode_like_cpp);
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(self.player_create_mode_like_cpp);
        }
        canonical
    }

    pub(crate) fn player_level_like_cpp(&self) -> u8 {
        if let Some(level) = self.with_owned_player_like_cpp(|player| player.level_like_cpp()) {
            return level;
        }
        #[cfg(test)]
        {
            return self.player_level;
        }
        #[cfg(not(test))]
        if self.player_handle_like_cpp.is_some() {
            return 0;
        }
        self.player_identity_bootstrap_like_cpp
            .as_ref()
            .map(|identity| identity.level)
            .unwrap_or_default()
    }

    pub(crate) fn player_gender_like_cpp(&self) -> u8 {
        if let Some(gender) = self.with_owned_player_like_cpp(|player| player.gender_like_cpp()) {
            return gender;
        }
        #[cfg(test)]
        {
            return self.player_gender;
        }
        #[cfg(not(test))]
        if self.player_handle_like_cpp.is_some() {
            return 0;
        }
        self.player_identity_bootstrap_like_cpp
            .as_ref()
            .map(|identity| identity.gender)
            .unwrap_or_default()
    }

    #[cfg(test)]
    pub(crate) fn set_player_faction_template_like_cpp(&mut self, faction_template: u32) {
        self.player_faction_template_like_cpp = (faction_template != 0).then_some(faction_template);
        let _ = self.mutate_canonical_player_like_cpp(|player| {
            player.unit_mut().set_faction(faction_template);
        });
    }

    /// Get the logged-in player GUID.
    pub fn player_guid(&self) -> Option<ObjectGuid> {
        self.player_guid
    }
}
