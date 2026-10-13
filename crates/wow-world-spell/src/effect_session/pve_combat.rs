// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Represented PvE combat state changed by effects (#1363 slice 9c), from the
//! World `session/spell_effects/combat.rs`.

use super::SpellEffectHostLikeCpp;
use wow_core::ObjectGuid;

pub fn stop_represented_player_pve_combat_like_cpp<H: SpellEffectHostLikeCpp>(
    host: &mut H,
    target_guid: ObjectGuid,
) {
    if host.player_guid() == Some(target_guid) {
        if host
            .hub_ref()
            .resolved_combat_target_like_cpp()
            .flatten()
            .is_some()
        {
            let _ = host.hub_mut().stop_player_attack_like_cpp();
        }
        host.hub_mut().set_combat_target_like_cpp(None);
        host.hub_mut().set_in_combat_like_cpp(false);
    }

    let pve_refs = {
        let Some(manager) = host.core().canonical_map_manager.as_ref().cloned() else {
            return;
        };
        let Ok(mut manager) = manager.lock() else {
            return;
        };
        let Some(managed) =
            manager.find_map_mut(u32::from(host.core().player_map_id_like_cpp()), 0)
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
        let _ = host.core_mut().mutate_world_creature(owner_guid, |owner| {
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
            owner.ai_ownership_mut().combat_target = None;
        });
    }
}
