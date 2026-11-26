//! Unit::SendMeleeAttackStart/Stop and WorldObject::SendCombatLogMessage.

use std::cell::OnceCell;
use std::time::Instant;

use crate::creature_spell_publication::{CombatLogUnitSource, combat_log_spell_cast_data_for_unit};
use wow_core::ObjectGuid;
use wow_packet::ServerPacket;
use wow_packet::packets::combat::AttackerStateUpdate;
use wow_world_core::session::HubRef;
use wow_world_core::session::mailbox::SendIfVisibleLikeCppCommand;

pub(crate) fn publish_attack_state(
    hub: HubRef<'_>,
    attacker: ObjectGuid,
    packet: &impl ServerPacket,
) {
    publish(
        hub,
        attacker,
        hub.core.current_runtime_map_key_like_cpp(),
        packet.to_bytes(),
        None,
    );
}

pub(crate) fn publish_hit(hub: HubRef<'_>, map: (u16, u32), packet: &AttackerStateUpdate) {
    publish(hub, packet.attacker, map, packet.to_bytes(), Some(packet));
}

fn publish(
    hub: HubRef<'_>,
    attacker: ObjectGuid,
    (map_id, instance_id): (u16, u32),
    basic: Vec<u8>,
    hit: Option<&AttackerStateUpdate>,
) {
    let Some(local) = hub.core.player_guid() else {
        return;
    };
    let known = local == attacker || hub.core.client_visible_guids_like_cpp.contains(&attacker);
    #[cfg(any(test, feature = "test-fixtures"))]
    if hub.core.player_handle_like_cpp.is_none() {
        if known {
            hub.core.send_raw_packet(&basic);
        }
        return;
    }
    let source = (|| {
        let manager = hub.core.canonical_map_manager.as_ref()?.lock().ok()?;
        let (key, _) =
            manager.player_active_residence_revision_like_cpp(hub.core.player_handle_like_cpp?)?;
        if key.map_id != u32::from(map_id) || key.instance_id != instance_id {
            return None;
        }
        let map = manager.find_map(key.map_id, key.instance_id)?.map();
        let (world, source) = if attacker.is_player() {
            let player = map.get_typed_player(attacker)?;
            (player.unit().world(), CombatLogUnitSource::Player(player))
        } else {
            let creature = map.get_typed_creature(attacker)?;
            (
                creature.unit().world(),
                CombatLogUnitSource::Creature(creature),
            )
        };
        Some((
            map.message_dist_observers_like_cpp(world),
            hit.map(|_| combat_log_spell_cast_data_for_unit(source)),
        ))
    })();
    let Some((mut observers, log)) = source else {
        return;
    };
    observers.sort_unstable();
    observers.dedup();

    let full = OnceCell::new();
    let bytes = |advanced| match (advanced, hit, log.as_ref()) {
        (true, Some(packet), Some(log)) => {
            full.get_or_init(|| packet.to_full_log_bytes_like_cpp(log))
        }
        _ => &basic,
    };
    if known && (local == attacker || observers.binary_search(&local).is_ok()) {
        hub.core.send_raw_packet(bytes(
            hub.represented_advanced_combat_logging_enabled_like_cpp(),
        ));
    }
    let Some(registry) = hub.core.player_registry() else {
        return;
    };
    for guid in observers {
        if guid == local {
            continue;
        }
        let Some(recipient) = registry.runtime_recipient(guid) else {
            continue;
        };
        if recipient.is_in_world
            && recipient.map_id == map_id
            && recipient.instance_id == instance_id
            && recipient.committed_visibility.contains(&attacker)
        {
            registry.publish_current_send_if_visible(
                recipient.registration,
                SendIfVisibleLikeCppCommand {
                    queued_at: Instant::now(),
                    source_guid: attacker,
                    map_id,
                    instance_id,
                    packet_bytes: bytes(recipient.advanced_combat_logging).clone(),
                },
            );
        }
    }
}
