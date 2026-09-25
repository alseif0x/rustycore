// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Player presentation: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

#[cfg(test)]
use super::RepresentedForceDeselectLikeCpp;
use super::debug;
use super::{LIQUID_MAP_IN_WATER_LIKE_CPP, LIQUID_MAP_UNDER_WATER_LIKE_CPP, MovementFlag};
use super::{ObjectGuid, Player, SKILL_RIDING_LIKE_CPP, SpellCastResult, UnitFlags, WorldSession};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::session) enum RepresentedMountSpellCheckOutcomeLikeCpp {
    CastFailed(SpellCastResult),
    DontReport,
}

impl WorldSession {
    pub(crate) fn player_is_game_master_like_cpp(&self) -> Option<bool> {
        let canonical = self.with_owned_player_like_cpp(Player::is_game_master_like_cpp);
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(self.player_game_master_like_cpp);
        }
        canonical
    }

    pub(in crate::session) fn player_unit_presentation_snapshot_like_cpp(
        &self,
    ) -> Option<(UnitFlags, i32, f32)> {
        let canonical = self.with_owned_player_like_cpp(|player| {
            (
                player.unit().unit_flags_like_cpp(),
                player.unit().data().mount_display_id,
                player.unit().world().object().scale(),
            )
        });
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some((
                self.player_unit_flags_like_cpp,
                self.player_mount_display_id_like_cpp,
                self.player_object_scale_like_cpp,
            ));
        }
        canonical
    }

    pub(in crate::session) fn set_player_mount_presentation_like_cpp(
        &mut self,
        display_id: i32,
        mounted: bool,
    ) -> bool {
        let mut canonical = self
            .with_owned_player_mut_like_cpp(|player| {
                player.set_mount_presentation_like_cpp(
                    u32::try_from(display_id).unwrap_or(0),
                    mounted,
                );
            })
            .is_some();
        #[cfg(test)]
        if !canonical
            && self.player_handle_like_cpp.is_none()
            && let Some(guid) = self.player_guid()
        {
            canonical = self
                .mutate_canonical_player_by_guid_like_cpp(guid, |player| {
                    player.set_mount_presentation_like_cpp(
                        u32::try_from(display_id).unwrap_or(0),
                        mounted,
                    );
                })
                .is_some();
        }
        #[cfg(test)]
        if canonical || self.player_handle_like_cpp.is_none() {
            self.player_mount_display_id_like_cpp = display_id;
            self.player_mounted_like_cpp = mounted;
            if mounted {
                self.player_unit_flags_like_cpp.insert(UnitFlags::MOUNT);
            } else {
                self.player_unit_flags_like_cpp.remove(UnitFlags::MOUNT);
            }
            return true;
        }
        canonical
    }

    pub(in crate::session) fn update_player_collision_height_like_cpp(&mut self) {
        let Some((_, mount_display_id, object_scale)) =
            self.player_unit_presentation_snapshot_like_cpp()
        else {
            return;
        };
        let computed_height = if let (Some(display_store), Some(model_store)) = (
            self.creatures.display_info_store.as_ref(),
            self.creatures.model_data_store.as_ref(),
        ) {
            let native_display_id = crate::handlers::character::default_display_id(
                self.player_race_like_cpp(),
                self.player_gender_like_cpp(),
            );
            let mount_display_id = u32::try_from(mount_display_id).ok().filter(|id| *id != 0);
            wow_data::unit_collision_height_like_cpp(
                object_scale,
                native_display_id,
                mount_display_id,
                display_store,
                model_store,
            )
        } else {
            None
        };

        let mount_display_id = u32::try_from(mount_display_id).unwrap_or(0);
        let _canonical_height = self.with_owned_player_mut_like_cpp(|player| {
            let unit = player.unit_mut();
            unit.set_mount_display_id(mount_display_id);
            if let Some(height) = computed_height {
                unit.set_collision_height_like_cpp(height);
            }
            unit.collision_height_like_cpp()
        });
        #[cfg(test)]
        if let Some(height) = _canonical_height.or(computed_height)
            && (_canonical_height.is_some() || self.player_handle_like_cpp.is_none())
        {
            self.player_collision_height_like_cpp = height;
        }
    }

    /// C++ `Unit::GetShapeshiftForm`: the canonical `UNIT_FIELD_BYTES_2` byte
    /// owned by the Unit, with the transitional Player gameplay projection as
    /// the fallback for fixtures that only seed it.
    pub(crate) fn represented_shapeshift_form_like_cpp(&self) -> Option<u32> {
        let canonical = self.with_owned_player_like_cpp(|player| {
            let form_id = u32::from(player.unit().shapeshift_form_id_like_cpp());
            if form_id != 0 {
                form_id
            } else {
                player.shapeshift_form_id_like_cpp()
            }
        });
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(self.represented_shapeshift_form_like_cpp);
        }
        canonical
    }

    /// C++ `Unit::SetShapeshiftForm`: write the canonical Unit field and keep
    /// the transitional Player gameplay projection in sync for the fallback
    /// readers.
    pub(crate) fn set_represented_shapeshift_form_like_cpp(&mut self, form_id: u32) -> bool {
        let canonical = self
            .with_owned_player_mut_like_cpp(|player| {
                player
                    .unit_mut()
                    .set_shapeshift_form_id_like_cpp(u8::try_from(form_id).unwrap_or(0));
                player.set_shapeshift_form_id_like_cpp(form_id);
            })
            .is_some();
        #[cfg(test)]
        if canonical || self.player_handle_like_cpp.is_none() {
            self.represented_shapeshift_form_like_cpp = form_id;
            return true;
        }
        canonical
    }

    pub(crate) fn represented_primary_specialization_id_like_cpp(&self) -> Option<u32> {
        let canonical = self.with_owned_player_like_cpp(Player::primary_specialization_id_like_cpp);
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(self.represented_primary_specialization_id_like_cpp);
        }
        canonical
    }

    pub(crate) fn set_represented_primary_specialization_id_like_cpp(
        &mut self,
        spec_id: u32,
    ) -> bool {
        let canonical = self
            .with_owned_player_mut_like_cpp(|player| player.set_primary_specialization(spec_id))
            .is_some();
        #[cfg(test)]
        if canonical || self.player_handle_like_cpp.is_none() {
            self.represented_primary_specialization_id_like_cpp = spec_id;
            return true;
        }
        canonical
    }

    #[allow(dead_code)]
    pub(crate) fn represented_mount_x_display_usable_like_cpp(
        &self,
        player_condition_id: u32,
    ) -> bool {
        self.represented_meets_player_condition_id_like_cpp(player_condition_id)
    }

    #[allow(dead_code)]
    pub(in crate::session) fn represented_mount_capability_selection_for_type_like_cpp(
        &self,
        mount_type_id: u16,
        riding_skill: u32,
        mount_restriction_flags: Option<u8>,
        is_submerged: bool,
        is_in_water: bool,
    ) -> Result<wow_data::MountCapabilityEntry, wow_data::MountCapabilityRejectLikeCpp> {
        let capability_store = self
            .mount_capability_store
            .as_ref()
            .ok_or(wow_data::MountCapabilityRejectLikeCpp::MissingCapabilityRow)?;
        let type_store = self
            .mount_type_x_capability_store
            .as_ref()
            .ok_or(wow_data::MountCapabilityRejectLikeCpp::MissingMountTypeCapabilities)?;
        let area_store = self
            .area_table_store
            .as_ref()
            .ok_or(wow_data::MountCapabilityRejectLikeCpp::Area)?;

        let map_id = u32::from(self.player_map_id_like_cpp());
        let map = self.maps.store.as_ref().and_then(|store| store.get(map_id));
        let (_, area_id) = self
            .player_zone_area_like_cpp()
            .ok_or(wow_data::MountCapabilityRejectLikeCpp::Area)?;
        let mount_flags = mount_restriction_flags.unwrap_or_else(|| {
            area_store
                .get(area_id)
                .map(|area| area.mount_flags as u8)
                .unwrap_or_else(|| {
                    if area_id == 0 {
                        // C++ reaches this check after TerrainMgr resolved the
                        // player's area. Until Rust has that full terrain bridge,
                        // an area 0 placeholder should not make ordinary ground
                        // mounts fail SPELL_FAILED_NOT_HERE.
                        wow_data::AREA_MOUNT_FLAG_ALLOW_GROUND_MOUNTS
                    } else {
                        0
                    }
                })
        });
        let context = wow_data::MountCapabilityContextLikeCpp {
            riding_skill,
            mount_flags,
            is_submerged,
            is_in_water,
            map_id: map_id as i32,
            cosmetic_parent_map_id: map
                .map(|entry| i32::from(entry.cosmetic_parent_map_id))
                .unwrap_or(-1),
            parent_map_id: map
                .map(|entry| i32::from(entry.parent_map_id))
                .unwrap_or(-1),
        };
        let visible_auras = self
            .resolved_player_visible_auras_like_cpp()
            .ok_or(wow_data::MountCapabilityRejectLikeCpp::Aura)?;

        capability_store
            .select_for_mount_type_with_reject_like_cpp(
                type_store,
                mount_type_id,
                &context,
                |required_area_id| {
                    area_store.is_in_area_like_cpp(area_id, u32::from(required_area_id))
                },
                |aura_id| {
                    visible_auras
                        .values()
                        .any(|aura| u32::try_from(aura.spell_id).ok() == Some(aura_id))
                },
                |spell_id| self.known_spells_like_cpp().contains(&spell_id),
            )
            .copied()
    }

    pub(crate) fn represented_mount_capability_for_type_like_cpp(
        &self,
        mount_type_id: u16,
        riding_skill: u32,
        mount_restriction_flags: Option<u8>,
        is_submerged: bool,
        is_in_water: bool,
    ) -> Option<wow_data::MountCapabilityEntry> {
        self.represented_mount_capability_selection_for_type_like_cpp(
            mount_type_id,
            riding_skill,
            mount_restriction_flags,
            is_submerged,
            is_in_water,
        )
        .ok()
    }

    #[allow(dead_code)]
    pub(crate) fn represented_mount_capability_for_type_from_session_like_cpp(
        &self,
        mount_type_id: u16,
        mount_restriction_flags: Option<u8>,
    ) -> Option<wow_data::MountCapabilityEntry> {
        let (is_submerged, is_in_water) = self.represented_player_mount_liquid_state_like_cpp()?;
        self.represented_mount_capability_for_type_like_cpp(
            mount_type_id,
            u32::from(self.resolved_player_skill_value_like_cpp(SKILL_RIDING_LIKE_CPP)?),
            mount_restriction_flags,
            is_submerged,
            is_in_water,
        )
    }

    pub(crate) fn represented_player_mount_liquid_state_like_cpp(&self) -> Option<(bool, bool)> {
        let liquid_status = self.player_liquid_status_like_cpp()?;
        let is_submerged = liquid_status & LIQUID_MAP_UNDER_WATER_LIKE_CPP != 0
            || self
                .resolved_player_movement_flags_like_cpp()?
                .contains(MovementFlag::SWIMMING);
        let is_in_water =
            liquid_status & (LIQUID_MAP_IN_WATER_LIKE_CPP | LIQUID_MAP_UNDER_WATER_LIKE_CPP) != 0;
        Some((is_submerged, is_in_water))
    }

    #[cfg(test)]
    pub(crate) fn represented_force_deselects_like_cpp(
        &self,
    ) -> &[RepresentedForceDeselectLikeCpp] {
        &self
            .duel_test_fixture_like_cpp
            .represented_force_deselects_like_cpp
    }

    pub(crate) fn represented_eject_passenger_like_cpp(
        &mut self,
        passenger_guid: ObjectGuid,
    ) -> bool {
        if !passenger_guid.is_unit() {
            return false;
        }

        self.eject_player_mount_vehicle_passenger_like_cpp(passenger_guid)
    }

    pub(crate) fn apply_far_sight_like_cpp(&mut self, enable: bool) {
        if !enable {
            #[cfg(test)]
            if let Some(player_guid) = self.player_guid() {
                self.visibility_test_fixture_like_cpp
                    .represented_seer_guid_like_cpp = Some(player_guid);
            }
            return;
        }

        let Some(target) = self.current_canonical_farsight_object_like_cpp() else {
            debug!("CMSG_FAR_SIGHT enable requested with no current viewpoint");
            return;
        };
        if self.canonical_map_has_seer_like_object_like_cpp(target) {
            #[cfg(test)]
            {
                self.visibility_test_fixture_like_cpp
                    .represented_seer_guid_like_cpp = Some(target);
            }
        } else {
            debug!("CMSG_FAR_SIGHT enable target {:?} is not resoluble", target);
        }
    }
}
