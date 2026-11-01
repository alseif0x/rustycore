// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use wow_core::ObjectGuid;
use wow_loot::OwnedLootAuthority;
use wow_packet::packets::update::UpdateObject;
use wow_world_core::session::{LootReleaseAccessLikeCpp, PacketPublicationAccessLikeCpp};
use wow_world_instances::InstanceState;

/// Read-only publication inputs; each query remains at its release phase.
pub struct LootReleasePublicationCxLikeCpp<'a> {
    owner: LootReleaseAccessLikeCpp<'a>,
    publication: PacketPublicationAccessLikeCpp<'a>,
    instances: &'a InstanceState,
}

impl<'a> LootReleasePublicationCxLikeCpp<'a> {
    pub fn new(
        owner: LootReleaseAccessLikeCpp<'a>,
        publication: PacketPublicationAccessLikeCpp<'a>,
        instances: &'a InstanceState,
    ) -> Self {
        Self {
            owner,
            publication,
            instances,
        }
    }

    pub fn send_creature_loot_release_dynamic_flags_update_like_cpp(
        &self,
        creature_guid: ObjectGuid,
        values_update: &wow_entities::UnitValuesUpdate,
        authority: Option<&OwnedLootAuthority>,
    ) -> usize {
        let Some(player_guid) = self.owner.player_guid_like_cpp() else {
            return 0;
        };
        let Some(packet_update) =
            wow_world_core::entity_update_bridge::unit_values_update_to_packet(values_update)
        else {
            return 0;
        };
        let map_id = self.owner.player_map_id_like_cpp();
        let instance_id = self.owner.loot_instance_id_like_cpp();
        let mut sent = 0;
        if self
            .owner
            .creature_is_client_visible_like_cpp(creature_guid)
        {
            let source_update = self.owner.creature_loot_release_values_for_viewer_like_cpp(
                creature_guid,
                player_guid,
                self.instances.has_pending_bind_like_cpp(),
                authority,
                packet_update.clone(),
            );
            self.publication
                .send_packet(&UpdateObject::unit_values_update(
                    creature_guid,
                    map_id,
                    source_update,
                ));
            sent += 1;
        }
        sent += self.owner.queue_creature_loot_release_values_like_cpp(
            player_guid,
            creature_guid,
            map_id,
            instance_id,
            &packet_update,
            authority,
        );
        sent
    }
}
