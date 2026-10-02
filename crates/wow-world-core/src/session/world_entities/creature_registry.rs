use crate::session::creature_canonical_adapter::sync_canonical_creature_entity_on_map_like_cpp;
use crate::session::state::SessionCore;
use wow_core::ObjectGuid;

impl SessionCore {
    pub fn sync_canonical_creature_entity_like_cpp(&mut self, creature: wow_entities::Creature) {
        let guid = creature.guid();
        let expected_legacy_authority = creature.loot_authority_like_cpp().clone();
        let expected_legacy_stamp = expected_legacy_authority.stamp_like_cpp();
        let (map_id, instance_id) = self.current_legacy_runtime_map_key_like_cpp();
        let Some(manager) = self.canonical_map_manager.as_ref() else {
            return;
        };
        let authority = sync_canonical_creature_entity_on_map_like_cpp(
            manager,
            u32::from(map_id),
            instance_id,
            creature,
        );
        if let Some(authority) = authority {
            let _ = self.rebind_legacy_creature_loot_authority_like_cpp(
                guid,
                &expected_legacy_authority,
                expected_legacy_stamp,
                authority,
            );
        }
    }

    pub fn mutate_world_creature<F, R>(&mut self, guid: ObjectGuid, f: F) -> Option<R>
    where
        F: FnOnce(&mut crate::map_manager::WorldCreature) -> R,
    {
        let (map_id, instance_id) = self.current_legacy_runtime_map_key_like_cpp();
        let mut f = Some(f);
        if let Some(manager) = self.map_manager.as_ref().cloned() {
            let result = {
                let mut manager = manager
                    .write()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                if let Some(creature) = manager.find_creature_mut(map_id, instance_id, guid) {
                    let result = f.take().expect("creature mutator is called once")(creature);
                    Some((result, creature.creature.clone()))
                } else {
                    None
                }
            };
            if let Some((result, creature)) = result {
                self.sync_canonical_creature_entity_like_cpp(creature);
                return Some(result);
            }
        }

        None
    }
}
