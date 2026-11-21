use crate::InstanceState;
use wow_packet::packets::instance::PendingRaidLock;
use wow_world_core::session::{HubMut, HubRef, connection_identity::unix_now};

/// C++ InstanceMap::AddPlayerToMap, before Map::AddPlayerToMap sends the self create.
pub fn start_raid_lock_on_map_entry(hub: &mut HubMut<'_>, publish: bool) {
    let Some((instance_id, prompt)) = raid_lock_prompt(hub.shared()) else {
        return;
    };
    apply_prompt(hub, instance_id, prompt, publish);
}

fn raid_lock_prompt(hub: HubRef<'_>) -> Option<(u32, PendingRaidLock)> {
    let key = hub.core.current_canonical_player_map_key_like_cpp()?;
    if key.instance_id == 0 {
        return None;
    }
    let (context, difficulty) = {
        let manager = hub.core.canonical_map_manager.as_ref()?.lock().ok()?;
        let map = manager.find_map(key.map_id, key.instance_id)?;
        if !map.has_instance_script() {
            return None;
        }
        (map.instance_lock_context()?, map.difficulty())
    };
    let (lock, entries) = crate::binding::resolve_attached_raid_lock(
        &hub.core.instance_lock_manager_access_like_cpp(hub.config),
        context,
        key,
        difficulty,
        hub.catalogs.map_store().map(AsRef::as_ref),
        hub.catalogs.map_difficulty_store().map(AsRef::as_ref),
    )?;
    if !entries.has_reset_schedule() || entries.is_using_encounter_locks || lock.is_new {
        return None;
    }
    let guid = hub.core.player_guid()?;
    let now = u64::try_from(unix_now()).ok()?;
    let manager = hub.core.instance_lock_mgr.as_ref()?.read().ok()?;
    let player_lock = manager.find_active_instance_lock_at(guid, &entries, now);
    let completed_mask = lock.data.completed_encounters_mask;
    if player_lock.is_some_and(|own| {
        !(own.is_expired_at(now) && own.extended)
            && own.data.completed_encounters_mask == completed_mask
    }) {
        return None;
    }
    Some((
        key.instance_id,
        PendingRaidLock {
            time_until_lock: 60_000,
            completed_mask,
            extending: player_lock.is_some_and(|own| own.extended),
            warning_only: entries.is_flex_locking,
        },
    ))
}

impl InstanceState {
    #[allow(dead_code)]
    pub fn send_pending_raid_lock_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        instance_id: u32,
        completed_mask: u32,
        extending: bool,
        warning_only: bool,
    ) {
        apply_prompt(
            hub,
            instance_id,
            PendingRaidLock {
                time_until_lock: 60_000,
                completed_mask,
                extending,
                warning_only,
            },
            true,
        );
    }
}

fn apply_prompt(hub: &mut HubMut<'_>, instance_id: u32, prompt: PendingRaidLock, publish: bool) {
    let map_id = u32::from(hub.core.player_map_id_like_cpp());
    let owner_resolved = hub
        .core
        .with_owned_player_mut_like_cpp(|player| {
            if !prompt.warning_only {
                player.set_pending_bind_like_cpp(crate::RepresentedPendingBind {
                    map_id,
                    instance_id,
                    completed_mask: prompt.completed_mask,
                    time_until_lock_ms: 60_000,
                });
            }
        })
        .is_some();
    if owner_resolved && publish {
        hub.core.send_packet_realm(&prompt);
    }
}
