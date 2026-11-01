// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The spell-pull attack start (#1363 slice 12b3), moved from the World
//! `session/spell_state/spell_publication.rs`: the creature attack-start
//! command, then `SMSG_ATTACK_START` to the visible spell-pull recipients.

use std::time::Instant;

use wow_core::ObjectGuid;
use wow_world_core::session::mailbox::{
    CreatureAttackStartLikeCppCommand, SendIfVisibleLikeCppCommand,
};
use wow_world_entities::CombatSessionHostLikeCpp;

pub fn publish_spell_pull_attack_start_like_cpp<H: CombatSessionHostLikeCpp>(
    host: &mut H,
    attacker_guid: ObjectGuid,
    victim_guid: ObjectGuid,
) {
    use wow_packet::ServerPacket;

    let map_id = host.core().player_map_id_like_cpp();
    let instance_id = host
        .core()
        .current_canonical_player_map_key_like_cpp()
        .map(|key| key.instance_id)
        .unwrap_or(0);
    let command = CreatureAttackStartLikeCppCommand {
        attacker_guid,
        victim_guid,
        previous_victim_guid: None,
        map_id,
        instance_id,
        packet_already_broadcast: false,
    };
    wow_world_entities::handle_creature_attack_start_like_cpp_command_like_cpp(host, command);

    let packet_bytes = wow_packet::packets::combat::AttackStart {
        attacker: attacker_guid,
        victim: victim_guid,
    }
    .to_bytes();
    let Some((source_position, source_combat_reach, visibility_range)) = host
        .core_mut()
        .mutate_world_creature(attacker_guid, |creature| {
            (
                creature.position(),
                creature.unit().world().combat_reach(),
                creature.visibility_range_like_cpp(),
            )
        })
    else {
        return;
    };
    let Some(registry) = host.core().player_registry.as_ref() else {
        return;
    };
    for registration in registry.spell_pull_recipients(
        victim_guid,
        map_id,
        instance_id,
        source_position,
        source_combat_reach,
        visibility_range,
    ) {
        let _ = registry.publish_current_send_if_visible(
            registration,
            SendIfVisibleLikeCppCommand {
                queued_at: Instant::now(),
                source_guid: attacker_guid,
                map_id,
                instance_id,
                packet_bytes: packet_bytes.clone(),
            },
        );
    }
}
