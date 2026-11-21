use crate::InstanceState;
use wow_packet::packets::instance::{
    PendingRaidLock, RaidInstanceMessage, RaidInstanceMessageType,
};
use wow_world_core::session::{
    ExpireInstanceMapOccupantLikeCppCommand, HubMut, HubRef, connection_identity::unix_now,
};

/// C++ InstanceMap::AddPlayerToMap, before Map::AddPlayerToMap sends the self create.
pub fn start_raid_lock_on_map_entry(hub: &mut HubMut<'_>, publish: bool) {
    if let Some((instance_id, prompt)) = raid_lock_prompt(hub.shared()) {
        apply_prompt(hub, instance_id, prompt, publish);
    }
    complete_instance_map_player_entry_if_registered_like_cpp(hub);
}

/// Complete the admission marker once the selected Player is in the live
/// registry. Login registers after its entry prompt; worldport already has
/// this registration when the prompt is published.
pub fn complete_instance_map_player_entry_if_registered_like_cpp(hub: &mut HubMut<'_>) {
    let (Some(guid), Some(handle), Some(key), Some(registry), Some(manager)) = (
        hub.core.player_guid,
        hub.core.player_handle_like_cpp,
        hub.core.current_canonical_player_map_key_like_cpp(),
        hub.core.player_registry().cloned(),
        hub.core
            .canonical_map_manager
            .as_ref()
            .map(std::sync::Arc::clone),
    ) else {
        return;
    };
    if handle.guid() != guid
        || registry
            .selected_aura_recipient(guid, handle, key)
            .is_none()
    {
        return;
    }
    if let Ok(mut manager) = manager.lock()
        && manager.current_player_admission_like_cpp(guid).is_some_and(
            |(current_handle, revision)| {
                current_handle == handle
                    && manager.player_active_residence_revision_like_cpp(handle)
                        == Some((key, revision))
                    && manager
                        .find_map(key.map_id, key.instance_id)
                        .is_some_and(|map| map.map().get_typed_player(guid).is_some())
            },
        )
    {
        manager.complete_instance_player_entry_like_cpp(key, guid);
    }
}

/// Apply one map-owned occupied-expiry event to its frozen recipient.
/// MapManager and InstanceLockMgr are read in separate scopes; no map guard is
/// retained while packets are published or Player pending-bind state changes.
pub fn apply_instance_map_expiry_to_occupant_like_cpp(
    hub: &mut HubMut<'_>,
    command: ExpireInstanceMapOccupantLikeCppCommand,
) {
    let Some(manager) = hub
        .core
        .canonical_map_manager
        .as_ref()
        .map(std::sync::Arc::clone)
    else {
        return;
    };
    if hub.core.player_guid != Some(command.recipient)
        || hub.core.player_handle_like_cpp != Some(command.handle)
        || hub.core.current_canonical_player_map_key_like_cpp() != Some(command.map_key)
    {
        return;
    }
    let current_map_is_exact = manager.lock().ok().is_some_and(|manager| {
        manager.map_incarnation_like_cpp(command.map_key) == Some(command.map_incarnation)
            && manager.current_player_admission_like_cpp(command.recipient)
                == Some((command.handle, command.residence_revision))
            && manager.player_active_residence_revision_like_cpp(command.handle)
                == Some((command.map_key, command.residence_revision))
            && manager
                .find_map(command.map_key.map_id, command.map_key.instance_id)
                .is_some_and(|map| {
                    map.instance_lock_context() == Some(command.instance_lock_context)
                        && map.map().get_typed_player(command.recipient).is_some()
                        && (!command.has_instance_script || map.has_instance_script())
                })
    });
    if !current_map_is_exact {
        return;
    }

    // `Reset(Expire)` broadcasts this before it separately builds the pending
    // prompt from the attached lock's current data.
    hub.core.send_packet_realm(&RaidInstanceMessage {
        message_type: RaidInstanceMessageType::Expired,
        map_id: command.map_key.map_id,
        difficulty_id: u32::from(command.instance_lock_context.difficulty_id),
        locked: false,
        extended: false,
    });

    if !command.has_instance_script {
        return;
    }
    let locks = hub.core.instance_lock_manager_access_like_cpp(hub.config);
    let attached_lock = crate::binding::resolve_attached_raid_lock(
        &locks,
        command.instance_lock_context,
        command.map_key,
        command.instance_lock_context.difficulty_id,
        hub.catalogs.map_store().map(AsRef::as_ref),
        hub.catalogs.map_difficulty_store().map(AsRef::as_ref),
    );
    let Some((lock, entries)) = attached_lock else {
        return;
    };
    let pending_lock = PendingRaidLock {
        time_until_lock: 60_000,
        // C++ `i_instanceLock->GetData()` reads the attached lock's own data,
        // not SharedInstanceLock's separate initialization-data projection.
        completed_mask: lock.data.completed_encounters_mask,
        extending: true,
        warning_only: entries.is_flex_locking,
    };
    apply_prompt(hub, command.map_key.instance_id, pending_lock, true);
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
