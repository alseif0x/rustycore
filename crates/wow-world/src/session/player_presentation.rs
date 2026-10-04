// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Player presentation: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

#[cfg(any(test, feature = "test-fixtures"))]
use super::RepresentedForceDeselectLikeCpp;
use super::debug;
use super::{Player, SKILL_RIDING_LIKE_CPP, SpellCastResult, WorldSession};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::session) enum RepresentedMountSpellCheckOutcomeLikeCpp {
    CastFailed(SpellCastResult),
    DontReport,
}

impl WorldSession {
    pub(in crate::session) fn update_player_collision_height_like_cpp(&mut self) {
        let mut hub = crate::session::hub_mut(self);
        let (presentation, mut control) = hub.aura_removal_mount_accesses_like_cpp();
        control.update_player_collision_height_like_cpp(&presentation, cfg!(test));
    }

    /// C++ `Unit::SetShapeshiftForm`: write the canonical Unit field and keep
    /// the transitional Player gameplay projection in sync for the fallback
    /// readers.
    pub(crate) fn set_represented_shapeshift_form_like_cpp(&mut self, form_id: u32) -> bool {
        self.core.set_shapeshift_form_with_fixture_like_cpp(
            form_id, cfg!(test),
            #[cfg(any(test, feature = "test-fixtures"))]
            &mut self.fixtures.auras.represented_shapeshift_form_like_cpp,
        )
    }

    pub(crate) fn represented_primary_specialization_id_like_cpp(&self) -> Option<u32> {
        let canonical = self
            .core
            .with_owned_player_like_cpp(Player::primary_specialization_id_like_cpp);
        #[cfg(test)]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            return Some(
                self.fixtures
                    .progression
                    .represented_primary_specialization_id_like_cpp,
            );
        }
        canonical
    }

    pub(crate) fn set_represented_primary_specialization_id_like_cpp(
        &mut self,
        spec_id: u32,
    ) -> bool {
        let canonical = self
            .core
            .with_owned_player_mut_like_cpp(|player| player.set_primary_specialization(spec_id))
            .is_some();
        #[cfg(test)]
        if canonical || self.core.player_handle_like_cpp.is_none() {
            self.fixtures
                .progression
                .represented_primary_specialization_id_like_cpp = spec_id;
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
            .catalogs
            .mount_capability_store
            .as_ref()
            .ok_or(wow_data::MountCapabilityRejectLikeCpp::MissingCapabilityRow)?;
        let type_store = self
            .catalogs
            .mount_type_x_capability_store
            .as_ref()
            .ok_or(wow_data::MountCapabilityRejectLikeCpp::MissingMountTypeCapabilities)?;
        let area_store = self
            .catalogs
            .area_table_store
            .as_ref()
            .ok_or(wow_data::MountCapabilityRejectLikeCpp::Area)?;

        let map_id = u32::from(self.core.player_map_id_like_cpp());
        let map = self
            .catalogs
            .maps
            .store
            .as_ref()
            .and_then(|store| store.get(map_id));
        let (_, area_id) = crate::session::hub_ref(self)
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
        let visible_auras = crate::session::hub_ref(self)
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
        let (is_submerged, is_in_water) =
            crate::session::hub_ref(self).represented_player_mount_liquid_state_like_cpp()?;
        self.represented_mount_capability_for_type_like_cpp(
            mount_type_id,
            u32::from(
                crate::session::hub_ref(self)
                    .resolved_player_skill_value_like_cpp(SKILL_RIDING_LIKE_CPP)?,
            ),
            mount_restriction_flags,
            is_submerged,
            is_in_water,
        )
    }

    #[cfg(test)]
    pub(crate) fn represented_force_deselects_like_cpp(
        &self,
    ) -> &[RepresentedForceDeselectLikeCpp] {
        self.social.represented_force_deselects_for_test_like_cpp()
    }

    pub(crate) fn apply_far_sight_like_cpp(&mut self, enable: bool) {
        if !enable {
            #[cfg(test)]
            if let Some(player_guid) = self.player_guid() {
                self.visibility
                    .set_represented_seer_guid_fixture_like_cpp(Some(player_guid));
            }
            return;
        }

        let Some(target) = ({
            let (s, h) = crate::session::split_visibility_ref(self);
            s.current_canonical_farsight_object_like_cpp(h)
        }) else {
            debug!("CMSG_FAR_SIGHT enable requested with no current viewpoint");
            return;
        };
        if self.canonical_map_has_seer_like_object_like_cpp(target) {
            #[cfg(test)]
            {
                self.visibility
                    .set_represented_seer_guid_fixture_like_cpp(Some(target));
            }
        } else {
            debug!("CMSG_FAR_SIGHT enable target {:?} is not resoluble", target);
        }
    }
}

#[cfg(test)]
#[path = "../../unit_tests/session/player_presentation/f3_shims.rs"]
mod f3_shims;
