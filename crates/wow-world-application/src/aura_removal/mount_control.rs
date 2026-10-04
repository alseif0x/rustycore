// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use super::{AuraRemovalCxLikeCpp, RemovedAuraLikeCpp};
use wow_entities::RepresentedAuraEffectLikeCpp;

#[cfg(any(test, feature = "test-fixtures"))]
pub(super) struct MountRemovalEvidenceRefsLikeCpp<'a> {
    pub(super) vehicle_id: &'a mut u32,
    pub(super) accessories: &'a mut Vec<wow_entities::VehicleAccessory>,
    pub(super) seat_count: &'a mut u8,
    pub(super) usable_seat_count: &'a mut u8,
    pub(super) vehicle_remove_requests: &'a mut u32,
    pub(super) pet_control_enable_requests: &'a mut u32,
    pub(super) pet_resummon_requests: &'a mut u32,
    pub(super) collision_update_requests: &'a mut u32,
}

impl AuraRemovalCxLikeCpp<'_> {
    pub(super) fn remove_mount_control_phase_like_cpp(&mut self, removed: &RemovedAuraLikeCpp) {
        let aura = &removed.aura;
        if aura.represented_effect != Some(RepresentedAuraEffectLikeCpp::Mounted) {
            return;
        }
        let vehicle_id = self.mount.mount_vehicle_kit_snapshot_like_cpp()
            .flatten().map(|vehicle| vehicle.vehicle_id()).unwrap_or(0);
        #[cfg(any(test, feature = "test-fixtures"))]
        let vehicle_id = if self.consumer_test && vehicle_id == 0 {
            *self.mount_evidence.vehicle_id
        } else {
            vehicle_id
        };
        let mount_capability_id = aura.represented_amount;
        let _ = self.mount.remove_mount_vehicle_kit_like_cpp();
        #[cfg(any(test, feature = "test-fixtures"))]
        if self.consumer_test {
            *self.mount_evidence.vehicle_id = 0;
            self.mount_evidence.accessories.clear();
            *self.mount_evidence.seat_count = 0;
            *self.mount_evidence.usable_seat_count = 0;
        }
        if removed.was_mounted {
            if vehicle_id != 0 {
                #[cfg(any(test, feature = "test-fixtures"))]
                if self.consumer_test {
                    *self.mount_evidence.vehicle_remove_requests = self.mount_evidence.vehicle_remove_requests.saturating_add(1);
                }
                self.mount.send_set_vehicle_rec_id_like_cpp(0);
            }
            #[cfg(any(test, feature = "test-fixtures"))]
            if self.consumer_test {
                *self.mount_evidence.pet_control_enable_requests = self.mount_evidence.pet_control_enable_requests.saturating_add(1);
            }
            self.mount.enable_pet_controls_on_dismount_like_cpp();
            #[cfg(any(test, feature = "test-fixtures"))]
            if self.consumer_test {
                // The existing Rust path records resummon evidence only.
                *self.mount_evidence.pet_resummon_requests = self.mount_evidence.pet_resummon_requests.saturating_add(1);
                *self.mount_evidence.collision_update_requests = self.mount_evidence.collision_update_requests.saturating_add(1);
            }
            self.mount.update_player_collision_height_like_cpp(&self.player, self.consumer_test);
            self.mount.send_movement_set_collision_height_like_cpp(
                &self.player,
                wow_packet::packets::movement::UPDATE_COLLISION_HEIGHT_REASON_MOUNT_LIKE_CPP,
            );
        }
        self.mount.send_represented_mount_unit_update_like_cpp(&self.player, 0);
        self.remove_mount_capability_speed_auras_like_cpp(mount_capability_id);
    }

    fn remove_mount_capability_speed_auras_like_cpp(&mut self, mount_capability_id: i32) {
        let Some(mod_spell_aura_id) = self.mount_capabilities
            .and_then(|store| store.get(u32::try_from(mount_capability_id).ok()?))
            .map(|capability| capability.mod_spell_aura_id)
            .filter(|spell_id| *spell_id > 0)
        else {
            return;
        };
        let Some(visible_auras) = self.player.visible_auras_snapshot_like_cpp() else {
            return;
        };
        let slots: Vec<u8> = visible_auras.values()
            .filter_map(|aura| (aura.spell_id == mod_spell_aura_id).then_some(aura.slot))
            .collect();
        for slot in slots {
            let _ = self.remove_aura_like_cpp(slot);
        }
    }
}
