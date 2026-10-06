use super::LootState;
use crate::RepresentedCreatureLootStateLikeCpp;
use std::collections::HashMap;
use wow_core::ObjectGuid;
use wow_entities::AccessorObjectKind;
use wow_loot::OwnedLootAuthority;
use wow_packet::packets::loot::CreatureLoot;
use wow_world_core::session::{HubMut, HubRef};

impl LootState {
    /// Install kill-time pools only while the exact creature death lifetime
    /// observed before async template generation is still current. C++ runs
    /// `Unit::Kill` and loot creation on one map thread; this lock-scoped CAS
    /// is the Rust equivalent and prevents corpse-removal/respawn ABA.
    pub fn install_represented_creature_kill_loot_if_current_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        creature_guid: ObjectGuid,
        expected_authority: &OwnedLootAuthority,
        expected_object_generation: u64,
        expected_loot_lifecycle_revision: u64,
        shared: Option<CreatureLoot>,
        personal: HashMap<ObjectGuid, CreatureLoot>,
    ) -> bool {
        let expected_authority = expected_authority.clone();
        hub.core
            .mutate_world_creature(creature_guid, move |world_creature| {
                if world_creature.is_alive()
                    || world_creature.creature.loot_lifecycle_revision_like_cpp()
                        != expected_loot_lifecycle_revision
                    || !world_creature
                        .creature
                        .loot_authority_like_cpp()
                        .shares_storage_like_cpp(&expected_authority)
                    || !expected_authority.is_retired_like_cpp()
                    || expected_authority.generation_like_cpp() != expected_object_generation
                {
                    return false;
                }

                let installed = if expected_object_generation == 0 {
                    expected_authority
                        .initialize_pristine_like_cpp(shared, personal)
                        .installed()
                } else {
                    expected_authority
                        .replace_retired_generation_like_cpp(
                            expected_object_generation,
                            shared,
                            personal,
                        )
                        .is_some()
                };
                if installed {
                    world_creature
                        .creature
                        .sync_loot_summaries_from_authority_like_cpp();
                }
                installed
            })
            .unwrap_or(false)
    }

    /// C++ `Unit::Kill` first resolves every tap-list GUID through
    /// `ObjectAccessor::GetPlayer(*creature, guid)`. Only connected players in
    /// the creature's exact map instance receive an overworld personal pool.
    pub fn represented_connected_creature_tappers_like_cpp(
        &self,
        hub: HubRef<'_>,
        tappers: &[ObjectGuid],
    ) -> Vec<ObjectGuid> {
        let current_player = hub.core.player_guid();
        let map_id = hub.core.player_map_id_like_cpp();
        let instance_id = hub
            .core
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        let registry = hub.core.player_registry();
        let mut connected = tappers
            .iter()
            .copied()
            .filter(|tapper| {
                if !tapper.is_player() {
                    return false;
                }
                if Some(*tapper) == current_player {
                    return true;
                }
                registry
                    .and_then(|registry| registry.loot_presence(*tapper))
                    .is_some_and(|player| {
                        player.is_in_world
                            && player.map_id == map_id
                            && player.instance_id == instance_id
                    })
            })
            .collect::<Vec<_>>();
        connected.sort_unstable_by_key(|guid| (guid.high_value(), guid.low_value()));
        connected.dedup();
        connected
    }

    pub fn represented_creature_loot_state_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        guid: ObjectGuid,
    ) -> Option<RepresentedCreatureLootStateLikeCpp> {
        hub.core
            .mutate_world_creature(guid, |creature| RepresentedCreatureLootStateLikeCpp {
                is_alive: creature.is_alive(),
                position: creature.position(),
                level: creature.level(),
                entry: creature.entry(),
                loot_id: creature.loot_id(),
                gold_min: creature.gold_min(),
                gold_max: creature.gold_max(),
                dungeon_encounter_id: creature.dungeon_encounter_id(),
                tappers: creature.creature.tap_list().to_vec(),
                loot_lifecycle_revision: creature.creature.loot_lifecycle_revision_like_cpp(),
            })
    }

    pub fn represented_creature_position_for_loot_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        guid: ObjectGuid,
    ) -> Option<wow_core::Position> {
        if let Some(position) = self.canonical_map_object_position_for_loot_like_cpp(
            hub.shared(),
            guid,
            &[AccessorObjectKind::Creature],
        ) {
            return Some(position);
        }

        self.represented_creature_loot_state_like_cpp(hub, guid)
            .map(|creature| creature.position)
    }
}
