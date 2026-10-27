use wow_core::ObjectGuid;
use wow_world_core::session::{
    PlayerStatAuraView, PreparedPlayerVisiblePacket, SessionHubHostLikeCpp,
    prepare_player_visible_packet,
};

const FAKE_INEBRIATE: i32 = 304;

/// Input to the single native SetDrunkValue operation. The effect delta is
/// resolved under the same MapManager guard as the canonical mutation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerDrunkValueInput {
    Set(u8),
    EffectDelta(i32),
}

/// Apply C++ `Player::SetDrunkValue` to the current session Player. The return
/// value reports a threshold crossing; unchanged values still run the native
/// timer, visibility and update-field behavior.
pub fn set_player_drunk_value_like_cpp<H: SessionHubHostLikeCpp>(
    host: &H,
    target_guid: ObjectGuid,
    input: PlayerDrunkValueInput,
    cast_item_guid: Option<ObjectGuid>,
    cast_item_entry: Option<u32>,
) -> Result<bool, &'static str> {
    if host.player_guid() != Some(target_guid) {
        return Ok(false);
    }

    let item_id = match cast_item_guid.filter(|guid| !guid.is_empty()) {
        Some(_) => cast_item_entry
            .ok_or("Cast item GUID has no canonical item entry")? as i32,
        None => 0,
    };

    let hub = host.hub_ref();
    let core = hub.core;
    let handle = core
        .player_handle_like_cpp
        .ok_or("Current Player has no canonical MapManager handle")?;
    if handle.guid() != target_guid {
        return Err("Current Player handle does not match the EffectInebriate target");
    }
    let shared_manager = core
        .canonical_map_manager
        .as_ref()
        .ok_or("Canonical MapManager is unavailable")?;
    let mut manager = shared_manager
        .lock()
        .map_err(|_| "Canonical MapManager lock is unavailable")?;

    let (requested_value, modifier, has_fake_aura, old_threshold, new_threshold) = manager
        .with_player_like_cpp(handle, |player| -> Result<_, &'static str> {
            if player.guid() != target_guid {
                return Err("Canonical Player does not match the current session");
            }
            let current = player.inebriation_like_cpp();
            let requested = match input {
                PlayerDrunkValueInput::Set(value) => value,
                PlayerDrunkValueInput::EffectDelta(delta) => i32::from(current)
                    .checked_add(delta)
                    .ok_or("EffectInebriate amount overflows the native i32 addition")?
                    .clamp(0, 100) as u8,
            };
            let resulting = requested.min(100);
            let aura_state = &player.unit().subsystems().auras;
            if !aura_state.persisted_player_aura_authority_complete_like_cpp()
                || aura_state.spell_hit_aura_authority_tombstoned_like_cpp()
            {
                return Err("Canonical Player Aura authority is incomplete or tombstoned");
            }
            let aura_catalogs = hub
                .catalogs
                .player_stat_aura_catalogs()
                .ok_or("Player Aura spell catalogs are unavailable")?;
            let aura_view = PlayerStatAuraView::new(
                aura_state.runtime_applications_like_cpp(),
                aura_catalogs,
            )
            .ok_or("Canonical Player Aura metadata is unavailable")?;
            let applied_fake_effects = aura_state
                .applied_aura_types
                .get(&FAKE_INEBRIATE)
                .map(Vec::as_slice)
                .unwrap_or(&[]);
            let (modifier, has_fake_aura) = aura_view
                .fake_inebriation_facts(applied_fake_effects)
                .ok_or("Fake-inebriate spell-group metadata is unavailable")?;
            Ok((
                requested,
                modifier,
                has_fake_aura,
                wow_entities::Player::drunken_state_by_value_like_cpp(current),
                wow_entities::Player::drunken_state_by_value_like_cpp(resulting),
            ))
        })
        .ok_or("Canonical Player is unavailable")??;

    let prepared_packet: Option<PreparedPlayerVisiblePacket> = if old_threshold != new_threshold {
        let packet = wow_packet::packets::misc::CrossedInebriationThreshold {
            guid: target_guid,
            threshold: i32::from(new_threshold),
            item_id,
        };
        Some(
            prepare_player_visible_packet(
                &manager,
                handle,
                wow_packet::ServerPacket::to_bytes(&packet),
                true,
            )
            .ok_or("Native Player visible-set recipients could not be prepared")?,
        )
    } else {
        None
    };

    let threshold_changed = manager
        .with_player_mut_like_cpp(handle, |player| {
            player.set_drunk_value_like_cpp(requested_value, modifier, has_fake_aura)
        })
        .ok_or("Canonical Player disappeared before SetDrunkValue")?;
    drop(manager);

    if let Some(prepared_packet) = prepared_packet {
        let _ = core
            .packet_publication_access_like_cpp()
            .publish_prepared_player_visible_packet(&prepared_packet);
    }
    Ok(threshold_changed)
}
