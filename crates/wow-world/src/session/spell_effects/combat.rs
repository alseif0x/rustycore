//! Represented PvE and PvP combat state changed by effects.
//!
//! Moved out of the Session root under #621. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(in crate::session) fn stop_represented_player_pve_combat_like_cpp(
        &mut self,
        target_guid: ObjectGuid,
    ) {
        if self.player_guid() == Some(target_guid) {
            if self.resolved_combat_target_like_cpp().flatten().is_some() {
                let _ = self.stop_player_attack_like_cpp();
            }
            self.set_combat_target_like_cpp(None);
            self.set_in_combat_like_cpp(false);
        }

        let pve_refs = {
            let Some(manager) = self.canonical_map_manager.as_ref().cloned() else {
                return;
            };
            let Ok(mut manager) = manager.lock() else {
                return;
            };
            let Some(managed) = manager.find_map_mut(u32::from(self.player_map_id_like_cpp()), 0)
            else {
                return;
            };
            let map = managed.map_mut();
            let Some(player) = map.get_typed_player_mut(target_guid) else {
                return;
            };
            let pve_refs: Vec<ObjectGuid> = player
                .unit()
                .subsystems()
                .combat
                .pve_refs
                .keys()
                .copied()
                .collect();
            player
                .unit_mut()
                .subsystems_mut()
                .combat
                .end_all_pve_combat();

            for owner_guid in &pve_refs {
                if let Some(owner) = map.get_typed_creature_mut(*owner_guid) {
                    owner
                        .unit_mut()
                        .subsystems_mut()
                        .combat
                        .purge_combat_ref_like_cpp(target_guid);
                    owner
                        .unit_mut()
                        .subsystems_mut()
                        .combat
                        .scale_threat(target_guid, 0.0);
                    owner.unit_mut().remove_attacker_like_cpp(target_guid);
                } else if let Some(owner) = map.get_typed_player_mut(*owner_guid) {
                    owner
                        .unit_mut()
                        .subsystems_mut()
                        .combat
                        .purge_combat_ref_like_cpp(target_guid);
                    owner
                        .unit_mut()
                        .subsystems_mut()
                        .combat
                        .scale_threat(target_guid, 0.0);
                    owner.unit_mut().remove_attacker_like_cpp(target_guid);
                }
            }
            pve_refs
        };

        for owner_guid in pve_refs {
            let _ = self.mutate_world_creature(owner_guid, |owner| {
                owner
                    .creature
                    .unit_mut()
                    .subsystems_mut()
                    .combat
                    .purge_combat_ref_like_cpp(target_guid);
                owner
                    .creature
                    .unit_mut()
                    .subsystems_mut()
                    .combat
                    .scale_threat(target_guid, 0.0);
                owner
                    .creature
                    .unit_mut()
                    .remove_attacker_like_cpp(target_guid);
                owner.creature.ai_ownership_mut().combat_target = None;
            });
        }
    }
}
