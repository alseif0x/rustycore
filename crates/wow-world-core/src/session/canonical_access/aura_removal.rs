// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Selected canonical authority for aura-removal transitions.

mod threat;
mod mount_control;
mod pet_control;
mod stats;
mod read_views;
pub use read_views::{AuraNpcAccessBuilderLikeCpp, AuraConditionAccessBuilderLikeCpp};
pub use stats::AuraStatsAccessBuilderLikeCpp;
pub use mount_control::AuraMountControlAccessLikeCpp;
#[cfg(any(test, feature = "test-fixtures"))]
pub use mount_control::AuraMountControlFixtureRefsLikeCpp;
#[cfg(any(test, feature = "test-fixtures"))]
pub use pet_control::AuraDismountPetFixtureRefsLikeCpp;

use crate::session::SessionCore;
use wow_constants::UnitFlags;
use wow_entities::{AuraApplicationLikeCpp, AuraCastProvenanceLikeCpp, AuraSubsystem};
#[cfg(any(test, feature = "test-fixtures"))]
use std::collections::HashMap;

#[cfg(any(test, feature = "test-fixtures"))]
pub struct AuraRemovalFixtureRefsLikeCpp<'a> {
    complete: &'a mut bool,
    tombstoned: &'a mut bool,
    visible: &'a mut HashMap<u8, AuraApplicationLikeCpp>,
    threat: &'a mut HashMap<u8, wow_entities::AuraThreatSnapshotLikeCpp>,
    mount_display: &'a mut i32,
    mounted: &'a mut bool,
    unit_flags: &'a mut UnitFlags,
    object_scale: &'a f32,
}

#[cfg(any(test, feature = "test-fixtures"))]
impl<'a> AuraRemovalFixtureRefsLikeCpp<'a> {
    pub fn new(
        complete: &'a mut bool,
        tombstoned: &'a mut bool,
        visible: &'a mut HashMap<u8, AuraApplicationLikeCpp>,
        threat: &'a mut HashMap<u8, wow_entities::AuraThreatSnapshotLikeCpp>,
        mount_display: &'a mut i32,
        mounted: &'a mut bool,
        unit_flags: &'a mut UnitFlags,
        object_scale: &'a f32,
    ) -> Self {
        Self { complete, tombstoned, visible, threat, mount_display, mounted, unit_flags, object_scale }
    }
}

/// Borrows the canonical owner and the existing selected fixture fields.
/// Each operation drops its canonical guard before returning to application.
pub struct PlayerAuraRemovalAccessLikeCpp<'a> {
    core: &'a SessionCore,
    #[cfg(any(test, feature = "test-fixtures"))]
    fixtures: AuraRemovalFixtureRefsLikeCpp<'a>,
}

impl SessionCore {
    pub fn spell_has_attribute_with_stores_like_cpp(
        &self, spell_store: Option<&wow_data::SpellStore>, difficulty_store: Option<&wow_data::DifficultyStore>,
        spell_id: i32, attribute_word: usize, attribute: u32,
    ) -> bool {
        let Some(spell_store) = spell_store else { return false; };
        spell_store.has_attribute_for_difficulty_like_cpp(
            spell_id, self.current_map_difficulty_id_like_cpp(), difficulty_store, attribute_word, attribute,
        )
    }
    pub fn player_aura_removal_access_like_cpp<'a>(
        &'a self,
        #[cfg(any(test, feature = "test-fixtures"))] fixtures: AuraRemovalFixtureRefsLikeCpp<'a>,
    ) -> PlayerAuraRemovalAccessLikeCpp<'a> {
        PlayerAuraRemovalAccessLikeCpp {
            core: self,
            #[cfg(any(test, feature = "test-fixtures"))]
            fixtures,
        }
    }
}

impl PlayerAuraRemovalAccessLikeCpp<'_> {
    pub fn has_canonical_died_state_like_cpp(&self) -> Option<bool> {
        self.core.mutate_canonical_player_like_cpp(|player| {
            player.unit().has_unit_state(wow_constants::UnitState::DIED.bits())
        })
    }

    pub fn clear_canonical_died_state_like_cpp(&self) -> Option<()> {
        self.core.mutate_canonical_player_like_cpp(|player| {
            player.unit_mut().clear_unit_state(wow_constants::UnitState::DIED.bits());
        })
    }

    pub fn inventory_valuation_access_like_cpp(&self) -> crate::session::InventoryValuationAccessLikeCpp<'_> {
        self.core.inventory_valuation_access_like_cpp()
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn registry_hydration_access_like_cpp(&self) -> crate::session::PlayerRegistryHydrationAccessLikeCpp<'_> {
        self.core.player_registry_hydration_access_like_cpp()
    }

    pub fn aura_difficulty_like_cpp(&self) -> u8 {
        self.core.current_map_difficulty_id_like_cpp()
    }

    pub fn aura_cast_level_like_cpp(
        &self,
        #[cfg(any(test, feature = "test-fixtures"))] fixture_level: &u8,
    ) -> u8 {
        self.core.player_level_with_fixture_like_cpp(
            #[cfg(any(test, feature = "test-fixtures"))] fixture_level,
        )
    }

    pub fn aura_cast_map_id_like_cpp(&self) -> u16 {
        self.core.player_map_id_like_cpp()
    }

    pub fn inventory_access_like_cpp(&self) -> crate::session::OwnedInventoryAccessLikeCpp<'_> {
        self.core.owned_inventory_access_like_cpp()
    }

    pub fn item_modifiers_access_like_cpp(&self) -> crate::session::OwnedItemModifiersAccessLikeCpp<'_> {
        self.core.owned_item_modifiers_access_like_cpp()
    }

    pub fn spell_has_attribute_like_cpp(
        &self, spell_store: Option<&wow_data::SpellStore>, difficulty_store: Option<&wow_data::DifficultyStore>,
        spell_id: i32, attribute_word: usize, attribute: u32,
    ) -> bool {
        self.core.spell_has_attribute_with_stores_like_cpp(spell_store, difficulty_store, spell_id, attribute_word, attribute)
    }
    pub fn player_guid_like_cpp(&self) -> Option<wow_core::ObjectGuid> {
        self.core.player_guid()
    }

    pub fn is_logged_in_like_cpp(&self) -> bool {
        self.core.state == crate::session::SessionState::LoggedIn
    }

    pub fn packet_publication_like_cpp(&self) -> crate::session::PacketPublicationAccessLikeCpp<'_> {
        self.core.packet_publication_access_like_cpp()
    }

    pub fn spell_acquisition_access_like_cpp(&self) -> crate::session::OwnedSpellAcquisitionAccessLikeCpp<'_> {
        self.core.owned_spell_acquisition_access_like_cpp()
    }

    pub fn aura_effects_by_spell_aura_type_like_cpp(
        &self, spell_store: Option<&wow_data::SpellStore>, aura_type: i32,
    ) -> Option<Vec<(i32, i32)>> {
        let visible_auras = self.visible_auras_snapshot_like_cpp()?;
        let spell_store = spell_store?;
        Some(crate::session::player_aura_effects_by_spell_aura_type_like_cpp(
            &visible_auras, spell_store, aura_type,
        ))
    }

    pub fn autoattack_damage_multiplier_like_cpp(&self, spell_store: Option<&wow_data::SpellStore>) -> f32 {
        let (Some(auras), Some(spell_store)) = (self.visible_auras_snapshot_like_cpp(), spell_store) else {
            return 1.0;
        };
        SessionCore::represented_autoattack_damage_multiplier_from_snapshot_like_cpp(&auras, spell_store)
    }

    pub fn apply_attack_speed_multipliers_like_cpp(&self, multipliers: [f32; 3], autoattack: f32) {
        let _ = self.core.mutate_canonical_player_like_cpp(|player| {
            player.unit_mut().apply_attack_time_multipliers_like_cpp(multipliers);
            player.unit_mut().set_mod_autoattack_damage_pct_like_cpp(autoattack);
        });
    }

    pub fn player_class_like_cpp(&self, #[cfg(any(test, feature = "test-fixtures"))] fixture: &u8) -> u8 {
        self.core.player_class_with_fixture_like_cpp(
            #[cfg(any(test, feature = "test-fixtures"))] fixture,
        )
    }

    pub fn calculate_display_power_type_like_cpp(
        &self, class_power: wow_constants::PowerType, aura_power: Option<u8>,
    ) -> Option<wow_constants::PowerType> {
        self.core.canonical_player_snapshot_like_cpp(|player| {
            player.unit().calculate_display_power_type_like_cpp(class_power, aura_power)
        })
    }

    pub fn sync_calculated_display_power_like_cpp(&self, power: Option<wow_constants::PowerType>) -> bool {
        self.core.sync_calculated_display_power_like_cpp(power)
    }

    pub fn set_shapeshift_form_like_cpp(
        &self, form_id: u32, consumer_test: bool,
        #[cfg(any(test, feature = "test-fixtures"))] fixture_form: &mut u32,
    ) -> bool {
        self.core.set_shapeshift_form_with_fixture_like_cpp(
            form_id, consumer_test,
            #[cfg(any(test, feature = "test-fixtures"))] fixture_form,
        )
    }
    pub fn aura_subsystem_snapshot_like_cpp(&self) -> Option<AuraSubsystem> {
        self.core.player_aura_subsystem_snapshot_with_fixture_refs_like_cpp(
            #[cfg(any(test, feature = "test-fixtures"))]
            self.fixtures.complete,
            #[cfg(any(test, feature = "test-fixtures"))]
            self.fixtures.tombstoned,
            #[cfg(any(test, feature = "test-fixtures"))]
            self.fixtures.visible,
            #[cfg(any(test, feature = "test-fixtures"))]
            self.fixtures.threat,
        )
    }

    pub fn visible_auras_snapshot_like_cpp(
        &self,
    ) -> Option<std::collections::HashMap<u8, AuraApplicationLikeCpp>> {
        self.aura_subsystem_snapshot_like_cpp()
            .map(|auras| auras.runtime_applications_like_cpp().clone())
    }

    pub fn player_has_visible_aura_spell_like_cpp(&self, spell_id: i32) -> Option<bool> {
        self.aura_subsystem_snapshot_like_cpp().map(|auras| {
            auras.runtime_applications_like_cpp().values().any(|aura| aura.spell_id == spell_id)
        })
    }

    pub fn resolved_player_mounted_like_cpp(&self) -> Option<bool> {
        self.player_unit_presentation_snapshot_like_cpp()
            .map(|(flags, _, _)| flags.contains(UnitFlags::MOUNT))
    }

    pub fn player_unit_presentation_snapshot_like_cpp(&self) -> Option<(UnitFlags, i32, f32)> {
        let canonical = self.core.with_owned_player_like_cpp(|player| {
            (
                player.unit().unit_flags_like_cpp(),
                player.unit().data().mount_display_id,
                player.unit().world().object().scale(),
            )
        });
        #[cfg(any(test, feature = "test-fixtures"))]
        let canonical = if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            Some((*self.fixtures.unit_flags, *self.fixtures.mount_display, *self.fixtures.object_scale))
        } else {
            canonical
        };
        canonical
    }

    pub fn set_player_mount_presentation_like_cpp(&mut self, display_id: i32, mounted: bool) -> bool {
        let mut canonical = self.core.with_owned_player_mut_like_cpp(|player| {
            player.set_mount_presentation_like_cpp(u32::try_from(display_id).unwrap_or(0), mounted);
        }).is_some();
        #[cfg(any(test, feature = "test-fixtures"))]
        if !canonical && self.core.player_handle_like_cpp.is_none()
            && let Some(guid) = self.core.player_guid()
        {
            canonical = self.core.mutate_canonical_player_by_guid_like_cpp(guid, |player| {
                player.set_mount_presentation_like_cpp(u32::try_from(display_id).unwrap_or(0), mounted);
            }).is_some();
        }
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical || self.core.player_handle_like_cpp.is_none() {
            *self.fixtures.mount_display = display_id;
            *self.fixtures.mounted = mounted;
            if mounted {
                self.fixtures.unit_flags.insert(UnitFlags::MOUNT);
            } else {
                self.fixtures.unit_flags.remove(UnitFlags::MOUNT);
            }
            return true;
        }
        canonical
    }

    /// Retained fixture mutation seam: reconstruct, invoke once, then mirror
    /// the same existing fields. It does not expose a borrowed subsystem.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn mutate_player_aura_subsystem_like_cpp<R>(
        &mut self,
        mutate: impl FnOnce(&mut AuraSubsystem) -> R,
    ) -> Option<R> {
        let mut mutate = Some(mutate);
        if self.core.player_handle_like_cpp.is_none() {
            let mut auras = self.aura_subsystem_snapshot_like_cpp()?;
            let result = mutate.take().expect("test Player aura mutation executes once")(&mut auras);
            *self.fixtures.complete = auras.persisted_player_aura_authority_complete_like_cpp();
            *self.fixtures.tombstoned = auras.spell_hit_aura_authority_tombstoned_like_cpp();
            *self.fixtures.visible = auras.runtime_applications_like_cpp().clone();
            self.fixtures.threat.clear();
            for slot in 0..=u8::MAX {
                if let Some(snapshot) = auras.threat_snapshot_like_cpp(slot) {
                    self.fixtures.threat.insert(slot, snapshot.clone());
                }
            }
            return Some(result);
        }
        self.core.with_owned_player_mut_like_cpp(|player| {
            mutate.take().expect("Player aura mutation executes once")(
                &mut player.unit_mut().subsystems_mut().auras,
            )
        })
    }

    pub fn remove_player_visible_aura_like_cpp(&mut self, slot: u8) -> Option<AuraApplicationLikeCpp> {
        let canonical = self.core.with_owned_player_mut_like_cpp(|player| {
            player.remove_player_visible_aura_like_cpp(slot)
        }).flatten();
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            return self.mutate_player_aura_subsystem_like_cpp(|auras| {
                auras.remove_runtime_application_like_cpp(slot)
            }).flatten();
        }
        canonical
    }

    pub fn insert_player_visible_aura_canonical_like_cpp(
        &self,
        aura: AuraApplicationLikeCpp,
        provenance: AuraCastProvenanceLikeCpp,
    ) -> bool {
        let slot = aura.slot;
        self.core.with_owned_player_mut_like_cpp(|player| {
            player.insert_player_visible_aura_like_cpp(aura);
            player.unit_mut().subsystems_mut().auras.set_aura_cast_provenance_like_cpp(slot, provenance);
        }).is_some()
    }

    pub fn player_handle_absent_like_cpp(&self) -> bool {
        self.core.player_handle_like_cpp.is_none()
    }

    pub fn apply_transform_canonical_like_cpp(
        &self, spell_id: i32, new_is_positive: bool, current_is_positive: Option<bool>,
    ) -> bool {
        self.core.with_owned_player_mut_like_cpp(|player| {
            player.unit_mut().subsystems_mut().auras.apply_transform_aura_like_cpp(
                spell_id, new_is_positive, current_is_positive,
            );
        }).is_some()
    }

    pub fn remove_transform_canonical_like_cpp(&self, spell_id: i32) -> bool {
        self.core.with_owned_player_mut_like_cpp(|player| {
            player.unit_mut().subsystems_mut().auras.remove_transform_aura_like_cpp(spell_id);
        }).is_some()
    }
}

impl SessionCore {
    pub fn sync_calculated_display_power_like_cpp(
        &self, power: Option<wow_constants::PowerType>,
    ) -> bool {
        let Some(power) = power else {
            return false;
        };
        let Some(player_guid) = self.player_guid() else {
            return false;
        };
        let current = u8::try_from(power as i8).unwrap_or(0);
        let changed = self
            .mutate_canonical_player_like_cpp(|player| {
                if player.unit().data().display_power == current {
                    return false;
                }
                player.unit_mut().set_display_power(power);
                true
            })
            .unwrap_or(false);
        if !changed {
            return false;
        }
        let mut mask = wow_entities::UpdateMask::new(wow_entities::UNIT_DATA_DISPLAY_POWER_BIT + 1);
        mask.set(wow_entities::UNIT_DATA_DISPLAY_POWER_BIT);
        let update = wow_entities::PlayerValuesUpdate {
            changed_object_type_mask: 0,
            object_data: None,
            unit_data: Some(wow_entities::UnitDataUpdate {
                mask,
                values: wow_entities::UnitDataValues {
                    display_power: current,
                    ..Default::default()
                },
            }),
            player_data: None,
            active_player_data: None,
        };
        if let Some(packet) = crate::entity_update_bridge::player_values_update_to_update_object(
            player_guid,
            self.player_map_id_like_cpp(),
            &update,
        ) {
            self.send_packet(&packet);
        }
        true
    }
    pub fn set_shapeshift_form_with_fixture_like_cpp(
        &self, form_id: u32, consumer_test: bool,
        #[cfg(any(test, feature = "test-fixtures"))] fixture_form: &mut u32,
    ) -> bool {
        let canonical = self.with_owned_player_mut_like_cpp(|player| {
            player.unit_mut().set_shapeshift_form_id_like_cpp(u8::try_from(form_id).unwrap_or(0));
            player.set_shapeshift_form_id_like_cpp(form_id);
        }).is_some();
        #[cfg(any(test, feature = "test-fixtures"))]
        if consumer_test && (canonical || self.player_handle_like_cpp.is_none()) {
            *fixture_form = form_id;
            return true;
        }
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let _ = consumer_test;
        canonical
    }
}

impl crate::session::HubMut<'_> {
    pub fn player_aura_removal_access_like_cpp(&mut self) -> PlayerAuraRemovalAccessLikeCpp<'_> {
        self.core.player_aura_removal_access_like_cpp(
            #[cfg(any(test, feature = "test-fixtures"))]
            AuraRemovalFixtureRefsLikeCpp::new(
                &mut self.fixtures.auras.player_aura_authority_complete_like_cpp,
                &mut self.fixtures.auras.player_spell_hit_aura_authority_tombstoned_like_cpp,
                &mut self.fixtures.auras.visible_auras,
                &mut self.fixtures.auras.canonical_threat_aura_snapshots_like_cpp,
                &mut self.fixtures.vehicles.player_mount_display_id_like_cpp,
                &mut self.fixtures.vehicles.player_mounted_like_cpp,
                &mut self.fixtures.presentation.player_unit_flags_like_cpp,
                &self.fixtures.presentation.player_object_scale_like_cpp,
            ),
        )
    }
}
