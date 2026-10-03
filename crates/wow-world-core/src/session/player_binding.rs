// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Player binding data shared with the WorldSession shell.

use wow_core::ObjectGuid;
use wow_entities::Player;

pub const PLAYER_LOCAL_FLAG_WAR_MODE_LIKE_CPP: u32 = 0x0000_0800;
pub const PLAYER_LOCAL_FLAG_OVERRIDE_TRANSPORT_SERVER_TIME_LIKE_CPP: u32 = 0x0000_8000;

#[cfg(any(test, feature = "test-fixtures"))]
pub struct PlayerTransportLoginStateLikeCpp {
    pub info: wow_packet::packets::movement::TransportInfo,
}

/// Login-only identity input consumed while the canonical Player is being
/// constructed. Once a generation-checked Player exists this value is retired;
/// it is not a second runtime identity authority.
#[derive(Debug, Clone, Default)]
pub struct PlayerIdentityBootstrapLikeCpp {
    pub name: Option<String>,
    pub race: u8,
    pub class: u8,
    pub level: u8,
    pub gender: u8,
}

impl crate::session::state::SessionCore {
    pub fn set_canonical_chosen_title_like_cpp(
        &mut self,
        title_id: i32,
    ) -> Option<wow_entities::PlayerValuesUpdate> {
        self.mutate_canonical_player_like_cpp(|player| {
            player.set_chosen_title_like_cpp(title_id);
            player.values_update(true)
        })
    }

    pub fn player_is_possessing_like_cpp(&self) -> bool {
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

    pub fn represented_player_charmed_guid_like_cpp(&self) -> ObjectGuid {
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

    pub fn player_liquid_status_like_cpp(&self) -> Option<u32> {
        self.canonical_player_snapshot_like_cpp(|player| player.gameplay_state().liquid_status)
    }

    /// Get the logged-in player GUID.
    pub fn player_guid(&self) -> Option<ObjectGuid> {
        self.player_guid
    }
}
impl crate::session::HubMut<'_> {
    /// C++ `Player::SetFactionForRace`: `Player::LoadFromDB` resolves the
    /// player's live faction template from `ChrRacesEntry::FactionID` before
    /// the player is added to the map or published through ObjectAccessor.
    pub fn set_player_faction_for_race_like_cpp(&mut self, race: u8) {
        let Some(chr_races_store) = self.catalogs.chr.races_store.as_ref() else {
            return;
        };
        let faction_template = chr_races_store
            .get(u32::from(race))
            .and_then(|entry| u32::try_from(entry.faction_id).ok())
            .filter(|faction_template| *faction_template != 0);
        let faction_template = faction_template.unwrap_or(0);
        let _canonical = self.core.with_owned_player_mut_like_cpp(|player| {
            player.unit_mut().set_faction(faction_template);
        });
        #[cfg(any(test, feature = "test-fixtures"))]
        if _canonical.is_some() || self.core.player_handle_like_cpp.is_none() {
            self.fixtures.identity.player_faction_template_like_cpp =
                (faction_template != 0).then_some(faction_template);
        }
    }

    pub fn set_player_create_mode_like_cpp(&mut self, create_mode: u8) -> bool {
        let _canonical = self
            .core
            .with_owned_player_mut_like_cpp(|player| player.set_create_mode_like_cpp(create_mode))
            .is_some();
        #[cfg(any(test, feature = "test-fixtures"))]
        if _canonical || self.core.player_handle_like_cpp.is_none() {
            self.fixtures.identity.player_create_mode_like_cpp = create_mode;
            return true;
        }
        _canonical
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_player_character_points_like_cpp(&mut self, points: i32) -> bool {
        let canonical = self
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player.set_character_points_like_cpp(points);
            })
            .is_some();
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical || self.core.player_handle_like_cpp.is_none() {
            self.fixtures.progression.player_character_points_like_cpp = points;
        }
        canonical
            || cfg!(any(test, feature = "test-fixtures"))
                && self.core.player_handle_like_cpp.is_none()
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_player_faction_template_like_cpp(&mut self, faction_template: u32) {
        self.fixtures.identity.player_faction_template_like_cpp =
            (faction_template != 0).then_some(faction_template);
        let _ = self.core.mutate_canonical_player_like_cpp(|player| {
            player.unit_mut().set_faction(faction_template);
        });
    }
}

impl crate::session::state::SessionCore {
    pub(crate) fn player_race_with_fixture_like_cpp(
        &self,
        #[cfg(any(test, feature = "test-fixtures"))] fixture_race: &u8,
    ) -> u8 {
        if let Some(race) = self.with_owned_player_like_cpp(|player| player.race_like_cpp()) {
            return race;
        }
        #[cfg(any(test, feature = "test-fixtures"))]
        {
            return *fixture_race;
        }
        #[cfg(not(any(test, feature = "test-fixtures")))]
        if self.player_handle_like_cpp.is_some() {
            return 0;
        }
        self.player_identity_bootstrap_like_cpp
            .as_ref()
            .map(|identity| identity.race)
            .unwrap_or_default()
    }

    pub(crate) fn player_class_with_fixture_like_cpp(
        &self,
        #[cfg(any(test, feature = "test-fixtures"))] fixture_class: &u8,
    ) -> u8 {
        if let Some(class) = self.with_owned_player_like_cpp(|player| player.class_like_cpp()) {
            return class;
        }
        #[cfg(any(test, feature = "test-fixtures"))]
        {
            return *fixture_class;
        }
        #[cfg(not(any(test, feature = "test-fixtures")))]
        if self.player_handle_like_cpp.is_some() {
            return 0;
        }
        self.player_identity_bootstrap_like_cpp
            .as_ref()
            .map(|identity| identity.class)
            .unwrap_or_default()
    }

    pub(crate) fn player_level_with_fixture_like_cpp(
        &self,
        #[cfg(any(test, feature = "test-fixtures"))] fixture_level: &u8,
    ) -> u8 {
        if let Some(level) = self.with_owned_player_like_cpp(|player| player.level_like_cpp()) {
            return level;
        }
        #[cfg(any(test, feature = "test-fixtures"))]
        {
            return *fixture_level;
        }
        #[cfg(not(any(test, feature = "test-fixtures")))]
        self.player_level_without_owned_player_like_cpp()
    }

    pub(crate) fn player_level_without_owned_player_like_cpp(&self) -> u8 {
        if self.player_handle_like_cpp.is_some() {
            return 0;
        }
        self.player_identity_bootstrap_like_cpp
            .as_ref()
            .map(|identity| identity.level)
            .unwrap_or_default()
    }
}

impl crate::session::HubRef<'_> {
    pub fn player_can_never_see_target_like_cpp(&self) -> bool {
        self.active_player_update_state_like_cpp()
            .map(|(flags, _, _)| {
                flags & PLAYER_LOCAL_FLAG_OVERRIDE_TRANSPORT_SERVER_TIME_LIKE_CPP == 0
            })
            .unwrap_or(true)
    }

    pub fn player_world_local_state_like_cpp(&self) -> Option<wow_entities::PlayerWorldLocalState> {
        let canonical = self
            .core
            .with_owned_player_like_cpp(|player| player.gameplay_state().world_local);
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            return Some(
                wow_entities::PlayerWorldLocalState::from_represented_parts_like_cpp(
                    self.fixtures.identity.player_zone_id_like_cpp,
                    self.fixtures.identity.player_area_id_like_cpp,
                    self.fixtures
                        .identity
                        .player_zone_area_authority_complete_like_cpp,
                    self.fixtures.combat.player_pvp_hostile_like_cpp,
                    self.fixtures.combat.player_pvp_end_timer_like_cpp,
                    self.fixtures.combat.player_contested_pvp_timer_like_cpp,
                    self.fixtures.identity.represented_is_outdoors_like_cpp,
                ),
            );
        }
        canonical
    }

    pub fn player_war_mode_local_active_like_cpp(&self) -> bool {
        self.active_player_update_state_like_cpp()
            .is_some_and(|(flags, _, _)| flags & PLAYER_LOCAL_FLAG_WAR_MODE_LIKE_CPP != 0)
    }

    pub fn player_name_like_cpp(&self) -> Option<String> {
        if let Some(name) = self
            .core
            .with_owned_player_like_cpp(|player| player.unit().world().name().to_owned())
        {
            return Some(name);
        }
        #[cfg(any(test, feature = "test-fixtures"))]
        {
            return self.fixtures.identity.player_name.clone();
        }
        if self.core.player_handle_like_cpp.is_some() {
            return None;
        }
        self.core
            .player_identity_bootstrap_like_cpp
            .as_ref()
            .and_then(|identity| identity.name.clone())
    }

    pub fn player_faction_template_id_like_cpp(&self) -> Option<u32> {
        let canonical = self.core.with_owned_player_like_cpp(|player| {
            u32::try_from(player.unit().data().faction_template)
                .ok()
                .filter(|faction| *faction != 0)
        });
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            return self.fixtures.identity.player_faction_template_like_cpp;
        }
        canonical.flatten()
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_can_swim_to_fly_transition_like_cpp(&self) -> bool {
        self.resolved_can_swim_to_fly_transition_like_cpp()
            .expect("test Player movement owner must resolve")
    }

    pub fn resolved_can_swim_to_fly_transition_like_cpp(&self) -> Option<bool> {
        let canonical = self.core.with_owned_player_like_cpp(|player| {
            player
                .gameplay_state()
                .movement_control
                .can_swim_to_fly_transition
        });
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            return Some(
                self.fixtures
                    .movement
                    .represented_can_swim_to_fly_transition_like_cpp,
            );
        }
        canonical
    }

    pub fn resolved_player_scale_duration_like_cpp(&self) -> Option<i32> {
        let canonical = self.core.with_owned_player_like_cpp(|player| {
            player.gameplay_state().movement_control.scale_duration
        });
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            return Some(self.fixtures.identity.player_scale_duration_like_cpp);
        }
        canonical
    }

    pub fn player_race_like_cpp(&self) -> u8 {
        #[cfg(any(test, feature = "test-fixtures"))]
        {
            self.core
                .player_race_with_fixture_like_cpp(&self.fixtures.identity.player_race)
        }
        #[cfg(not(any(test, feature = "test-fixtures")))]
        {
            self.core.player_race_with_fixture_like_cpp()
        }
    }

    pub fn player_class_like_cpp(&self) -> u8 {
        #[cfg(any(test, feature = "test-fixtures"))]
        {
            self.core
                .player_class_with_fixture_like_cpp(&self.fixtures.identity.player_class)
        }
        #[cfg(not(any(test, feature = "test-fixtures")))]
        {
            self.core.player_class_with_fixture_like_cpp()
        }
    }

    pub fn player_create_mode_like_cpp(&self) -> Option<u8> {
        let canonical = self
            .core
            .with_owned_player_like_cpp(Player::create_mode_like_cpp);
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            return Some(self.fixtures.identity.player_create_mode_like_cpp);
        }
        canonical
    }

    pub fn player_level_like_cpp(&self) -> u8 {
        #[cfg(any(test, feature = "test-fixtures"))]
        {
            self.core
                .player_level_with_fixture_like_cpp(&self.fixtures.identity.player_level)
        }
        #[cfg(not(any(test, feature = "test-fixtures")))]
        {
            self.core.player_level_with_fixture_like_cpp()
        }
    }

    pub fn player_gender_like_cpp(&self) -> u8 {
        if let Some(gender) = self
            .core
            .with_owned_player_like_cpp(|player| player.gender_like_cpp())
        {
            return gender;
        }
        #[cfg(any(test, feature = "test-fixtures"))]
        {
            return self.fixtures.identity.player_gender;
        }
        #[cfg(not(any(test, feature = "test-fixtures")))]
        if self.core.player_handle_like_cpp.is_some() {
            return 0;
        }
        self.core
            .player_identity_bootstrap_like_cpp
            .as_ref()
            .map(|identity| identity.gender)
            .unwrap_or_default()
    }
}
