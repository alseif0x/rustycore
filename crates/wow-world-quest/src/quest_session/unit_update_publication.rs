// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The unit values-update publication and its viewer-dependent NPC flags
//! (C++ `UF::UnitData` `NpcFlags` with `ViewerDependentValue<NpcFlagsTag>`
//! filtering `UNIT_NPC_FLAG_SPELLCLICK` per viewer; #1363 slice 12d, moved
//! from the World `session/publication/operations.rs` and
//! `session/world_entities/creature_interaction.rs`). The per-viewer
//! spell-click visibility stays a World operation behind
//! `QuestSessionHostLikeCpp::represented_can_see_spell_click_on_creature_like_cpp`.

use wow_core::ObjectGuid;
use wow_data::spell_click::UNIT_NPC_FLAG_SPELLCLICK_LIKE_CPP;
use wow_world_core::entity_update_bridge::unit_values_update_to_packet;

use crate::QuestSessionHostLikeCpp;

pub fn represented_unit_values_update_to_update_object_like_cpp<
    H: QuestSessionHostLikeCpp + ?Sized,
>(
    host: &H,
    unit_guid: ObjectGuid,
    map_id: u16,
    values_update: &wow_entities::UnitValuesUpdate,
) -> Option<wow_packet::packets::update::UpdateObject> {
    let packet_update = unit_values_update_to_packet(values_update)?;
    Some(represented_unit_packet_update_to_update_object_like_cpp(
        host,
        unit_guid,
        map_id,
        packet_update,
    ))
}

pub fn represented_unit_packet_update_to_update_object_like_cpp<
    H: QuestSessionHostLikeCpp + ?Sized,
>(
    host: &H,
    unit_guid: ObjectGuid,
    map_id: u16,
    mut packet_update: wow_packet::packets::update::UnitDataValuesDeltaUpdate,
) -> wow_packet::packets::update::UpdateObject {
    if unit_guid.is_any_type_creature()
        && (packet_update.npc_flags[0] & UNIT_NPC_FLAG_SPELLCLICK_LIKE_CPP as u32) != 0
    {
        let npc_flags =
            u64::from(packet_update.npc_flags[0]) | (u64::from(packet_update.npc_flags[1]) << 32);
        let filtered =
            represented_viewer_dependent_creature_npc_flags_like_cpp(host, unit_guid, npc_flags);
        packet_update.npc_flags[0] = filtered as u32;
        packet_update.npc_flags[1] = (filtered >> 32) as u32;
    }

    wow_packet::packets::update::UpdateObject::unit_values_update(unit_guid, map_id, packet_update)
}

pub fn represented_viewer_dependent_creature_npc_flags_like_cpp<
    H: QuestSessionHostLikeCpp + ?Sized,
>(
    host: &H,
    creature_guid: ObjectGuid,
    npc_flags: u64,
) -> u64 {
    if (npc_flags & UNIT_NPC_FLAG_SPELLCLICK_LIKE_CPP) == 0 {
        return npc_flags;
    }

    crate::represented_viewer_dependent_creature_npc_flags_like_cpp(
        npc_flags,
        host.represented_can_see_spell_click_on_creature_like_cpp(creature_guid),
    )
}
