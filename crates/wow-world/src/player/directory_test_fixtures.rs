use std::sync::{Arc, Mutex};

use wow_core::ObjectGuid;

use crate::{
    SharedCanonicalMapManager,
    player_directory::{PlayerRegistry, PlayerSessionRegistrationLikeCpp},
};

impl PlayerRegistry {
    /// Test harness whose registrations install the real canonical owner that
    /// production composition has already placed on the map before publication.
    #[must_use]
    pub fn with_canonical_player_fixtures_like_cpp() -> Self {
        let mut registry = Self::default();
        registry.fixture_installs_canonical_players = true;
        assert!(
            registry
                .bind_canonical_map_manager(Arc::new(Mutex::new(wow_map::MapManager::default(),)))
        );
        registry
    }

    #[must_use]
    pub fn fixture_canonical_map_manager_like_cpp(&self) -> Option<SharedCanonicalMapManager> {
        self.canonical_map_manager_like_cpp()
    }

    pub(crate) fn install_canonical_player_fixture_like_cpp(
        &self,
        guid: ObjectGuid,
        registration: &PlayerSessionRegistrationLikeCpp,
    ) {
        if !self.fixture_installs_canonical_players {
            return;
        }
        let Some(manager) = self.canonical_map_manager_like_cpp() else {
            return;
        };
        let Ok(mut manager) = manager.lock() else {
            return;
        };
        let map = manager
            .create_world_map(
                u32::from(registration.placement.map_id),
                registration.placement.instance_id,
            )
            .map_mut();
        if map.get_typed_player(guid).is_some() {
            return;
        }
        let mut player =
            wow_entities::Player::new(Some(u64::from(registration.identity.account_id)), false);
        player.unit_mut().world_mut().object_mut().create(guid);
        player
            .unit_mut()
            .world_mut()
            .set_name(registration.identity.player_name.clone());
        player
            .unit_mut()
            .world_mut()
            .set_map(
                u32::from(registration.placement.map_id),
                registration.placement.instance_id,
            )
            .unwrap();
        player
            .unit_mut()
            .world_mut()
            .relocate(registration.placement.position);
        player.unit_mut().world_mut().object_mut().add_to_world();
        player.unit_mut().set_level(registration.placement.level);
        player.unit_mut().set_faction(1);
        player.unit_mut().set_max_health(100);
        player.unit_mut().set_health(100);
        player.gameplay_state_mut().dungeon_difficulty_id = 1;
        map.insert_map_object_record(wow_entities::MapObjectRecord::new_player(player).unwrap())
            .unwrap();
    }
}
