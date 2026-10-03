use std::sync::Arc;
use std::time::{Duration, Instant};

use wow_core::ObjectGuid;
use wow_entities::{AccessorObjectKind, GameObject};
use wow_world_core::session::{HubMut, HubRef};

use crate::{RepresentedGameObjectUseEffect, WorldEntitiesState};

impl WorldEntitiesState {
    pub fn mutate_canonical_gameobject_by_guid_like_cpp<R>(
        &mut self,
        hub: &mut HubMut<'_>,
        guid: ObjectGuid,
        f: impl FnOnce(&mut wow_entities::GameObject) -> R,
    ) -> Option<R> {
        let map_key = hub
            .core
            .canonical_object_lookup_map_key_like_cpp(u32::from(
                hub.core.player_map_id_like_cpp(),
            ))?;
        let manager = Arc::clone(hub.core.canonical_map_manager.as_ref()?);
        let mut manager = manager.lock().ok()?;
        let managed = manager.find_map_mut(map_key.map_id, map_key.instance_id)?;
        let gameobject = managed.map_mut().get_typed_game_object_mut(guid)?;
        Some(f(gameobject))
    }

    pub fn canonical_gameobject_linked_trap_guid_like_cpp(
        &self,
        hub: HubRef<'_>,
        guid: ObjectGuid,
    ) -> Option<ObjectGuid> {
        if guid.is_empty() || !guid.is_game_object() {
            return None;
        }
        let map_key = hub
            .core
            .canonical_object_lookup_map_key_like_cpp(u32::from(
                hub.core.player_map_id_like_cpp(),
            ))?;
        let manager = hub.core.canonical_map_manager.as_ref()?;
        let Ok(manager) = manager.lock() else {
            return None;
        };
        let map = manager.find_map(map_key.map_id, map_key.instance_id)?;
        let gameobject = map.map().get_typed_game_object(guid)?;
        let linked_trap_guid = gameobject.linked_trap_guid_like_cpp();
        (!linked_trap_guid.is_empty()).then_some(linked_trap_guid)
    }

    pub fn set_canonical_gameobject_spell_id_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        guid: ObjectGuid,
        spell_id: u32,
    ) {
        let Some(map_key) = hub
            .core
            .canonical_object_lookup_map_key_like_cpp(u32::from(hub.core.player_map_id_like_cpp()))
        else {
            return;
        };
        let Some(manager) = hub.core.canonical_map_manager.as_ref() else {
            return;
        };
        let Ok(mut manager) = manager.lock() else {
            return;
        };
        let Some(map) = manager.find_map_mut(map_key.map_id, map_key.instance_id) else {
            return;
        };
        if let Some(game_object) = map.map_mut().get_typed_game_object_mut(guid) {
            game_object.set_spell_id(spell_id);
        }
    }

    pub fn represented_or_canonical_gameobject_owner_guid_like_cpp(
        &self,
        hub: HubRef<'_>,
        guid: ObjectGuid,
    ) -> Option<ObjectGuid> {
        let map_key = hub
            .core
            .canonical_object_lookup_map_key_like_cpp(u32::from(hub.core.player_map_id_like_cpp()));
        let canonical_owner = map_key
            .and_then(|map_key| {
                hub.core
                    .canonical_map_manager
                    .as_ref()
                    .and_then(|manager| manager.lock().ok())
                    .and_then(|manager| {
                        manager
                            .find_map(map_key.map_id, map_key.instance_id)
                            .and_then(|map| map.map().get_typed_game_object(guid))
                            .map(|game_object| game_object.owner_guid())
                    })
            })
            .filter(|owner_guid| !owner_guid.is_empty());
        canonical_owner.or_else(|| {
            self.represented_gameobject_use_states
                .get(&guid)
                .and_then(|state| state.owner_guid)
        })
    }

    pub fn upsert_canonical_gameobject_map_object_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        map_id: u16,
        guid: ObjectGuid,
        entry: u32,
        position: wow_core::Position,
    ) {
        let owner_guid = self
            .represented_gameobject_use_states
            .get(&guid)
            .and_then(|state| state.owner_guid);
        let Some(map_key) = hub
            .core
            .canonical_object_lookup_map_key_like_cpp(u32::from(map_id))
        else {
            return;
        };
        // A represented object from a stale client/map context must never be
        // materialized beside the player in a different map.
        if map_key.map_id != u32::from(map_id) {
            return;
        }
        let Some(manager) = hub.core.canonical_map_manager.as_ref() else {
            return;
        };
        let Ok(mut manager) = manager.lock() else {
            return;
        };
        let Some(map) = manager.find_map_mut(map_key.map_id, map_key.instance_id) else {
            return;
        };
        if map.map().get_game_object(guid).is_some() {
            let _ = map.map_mut().relocate_map_object_like_cpp(guid, position);
            if let Some(owner_guid) = owner_guid
                && let Some(game_object) = map.map_mut().get_typed_game_object_mut(guid)
            {
                game_object.set_created_by(owner_guid);
            }
            return;
        }

        let mut game_object = GameObject::new();
        game_object.world_mut().object_mut().create(guid);
        game_object.world_mut().object_mut().set_entry(entry);
        if let Some(owner_guid) = owner_guid {
            game_object.set_created_by(owner_guid);
        }
        if game_object
            .world_mut()
            .set_map(map_key.map_id, map_key.instance_id)
            .is_err()
        {
            return;
        }
        game_object.world_mut().relocate(position);
        let _ = map
            .map_mut()
            .add_to_map_like_cpp(AccessorObjectKind::GameObject, game_object.world().clone());
        game_object.world_mut().object_mut().add_to_world();
        let Ok(record) = wow_entities::MapObjectRecord::new_game_object(game_object) else {
            return;
        };
        let _ = map.map_mut().insert_map_object_record(record);
    }

    pub fn represented_gameobject_is_friendly_to_player_like_cpp(
        &self,
        hub: HubRef<'_>,
        gameobject_guid: ObjectGuid,
    ) -> Option<bool> {
        let gameobject_faction = self
            .represented_gameobject_use_states
            .get(&gameobject_guid)
            .and_then(|state| state.faction_template)?;
        let player_faction = hub.player_faction_template_id_like_cpp()?;
        let store = hub.catalogs.factions.template_store.as_ref()?;
        let gameobject_entry = store.get(gameobject_faction)?;
        let player_entry = store.get(player_faction)?;
        Some(gameobject_entry.is_friendly_to_like_cpp(player_entry))
    }

    pub fn apply_represented_gameobject_cooldown_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        cooldown_secs: u32,
    ) -> bool {
        if cooldown_secs == 0 {
            return true;
        }

        let now = Instant::now();
        let state = self
            .represented_gameobject_use_states
            .entry(gameobject_guid)
            .or_default();
        if state
            .cooldown_until
            .is_some_and(|cooldown_until| cooldown_until > now)
        {
            self.represented_gameobject_use_effects
                .push(RepresentedGameObjectUseEffect::CooldownRejected { gameobject_guid });
            return false;
        }

        state.cooldown_until =
            Some(now + Duration::from_millis(u64::from(cooldown_secs).saturating_mul(1000)));
        self.represented_gameobject_use_effects.push(
            RepresentedGameObjectUseEffect::CooldownStarted {
                gameobject_guid,
                cooldown_secs,
            },
        );
        true
    }

    pub fn represented_gameobject_area_id_like_cpp(
        &self,
        hub: HubRef<'_>,
        gameobject_guid: ObjectGuid,
    ) -> Option<u32> {
        self.represented_gameobject_use_states
            .get(&gameobject_guid)
            .and_then(|state| state.area_id)
            .or_else(|| hub.player_zone_area_like_cpp().map(|(_, area_id)| area_id))
    }

    pub fn represented_gameobject_spell_lookup_difficulty_id_like_cpp(
        &self,
        hub: HubRef<'_>,
    ) -> u8 {
        hub.core.current_map_difficulty_id_like_cpp()
    }
}
