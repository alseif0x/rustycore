//! Canonical Player lifetime owned by `MapManager`.
//!
//! C++ keeps one `Player` object behind `WorldSession::_player` and transfers
//! that same object between maps. During a far teleport the Player is alive but
//! has no Map (`WorldSession.h:980,1882`, `WorldSession.cpp:978-985`, and
//! `Player::TeleportTo`). `Map::AddPlayerToMap` and `RemovePlayerFromMap`
//! perform the active-container transition (`Map.cpp:427-462,907-934`), while
//! far teleport removes with `delete = false` (`Player.cpp:1453-1455`). Rust
//! therefore needs an explicit detached residence; rebuilding a Player from
//! Session fields would create a second authority.

use wow_core::{ObjectGuid, Position};
#[cfg(test)]
use wow_entities::MapObjectRecord;
use wow_entities::{AccessorObjectKind, Player};

use super::MapManager;
use crate::map::{
    MapRuntimePlayerAttachErrorLikeCpp, MapRuntimePlayerDetachErrorLikeCpp,
    MapRuntimePlayerRelocationErrorLikeCpp,
};
use crate::{MapKey, MapObjectRelocationError, MapObjectRelocationOutcome, RemoveFromMapError};

#[cfg(test)]
mod failure_tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PlayerHandle {
    guid: ObjectGuid,
    generation: u64,
}

impl PlayerHandle {
    pub const fn guid(self) -> ObjectGuid {
        self.guid
    }

    pub const fn generation(self) -> u64 {
        self.generation
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerResidenceLikeCpp {
    Detached,
    Active(MapKey),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PlayerOwnershipLikeCpp {
    generation: u64,
    residence: PlayerResidenceLikeCpp,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PlayerOwnerError {
    EmptyGuid,
    InvalidGuid { guid: ObjectGuid },
    InvalidPosition { position: Position },
    ReplacementRetireFailed { guid: ObjectGuid },
    GenerationExhausted,
    AlreadyOwned { guid: ObjectGuid },
    ActivePlayerMissing { guid: ObjectGuid },
    AmbiguousActivePlayer { guid: ObjectGuid },
    StaleHandle,
    NotDetached,
    NotActive,
    MissingMap { key: MapKey },
    MissingPlayer { guid: ObjectGuid },
    DetachedPlayerStillInWorld { guid: ObjectGuid },
    DetachedPlayerStillBound { guid: ObjectGuid, key: MapKey },
    ActiveObjectAlreadyPresent { guid: ObjectGuid, key: MapKey },
    ActiveObjectNotPlayer { guid: ObjectGuid, key: MapKey },
    RelocatePlayer(MapObjectRelocationError),
    RemoveFromMap(RemoveFromMapError),
}

impl MapManager {
    /// Adopt an already map-owned Player into the generation-checked lifetime
    /// registry without cloning or relocating it. This is the transition seam
    /// for callers that inserted the canonical record before handles existed.
    pub fn adopt_active_player_like_cpp(
        &mut self,
        guid: ObjectGuid,
    ) -> Result<PlayerHandle, PlayerOwnerError> {
        if self.player_owners_like_cpp.contains_key(&guid) {
            return Err(PlayerOwnerError::AlreadyOwned { guid });
        }
        if AccessorObjectKind::from_guid(guid) != Some(AccessorObjectKind::Player) {
            return Err(PlayerOwnerError::InvalidGuid { guid });
        }
        let mut residence = None;
        for (key, managed) in &self.maps {
            if managed.map().get_typed_player(guid).is_none() {
                continue;
            }
            if residence.replace(*key).is_some() {
                return Err(PlayerOwnerError::AmbiguousActivePlayer { guid });
            }
        }
        let key = residence.ok_or(PlayerOwnerError::ActivePlayerMissing { guid })?;
        let generation = self.allocate_player_generation_like_cpp()?;
        self.player_owners_like_cpp.insert(
            guid,
            PlayerOwnershipLikeCpp {
                generation,
                residence: PlayerResidenceLikeCpp::Active(key),
            },
        );
        Ok(PlayerHandle { guid, generation })
    }

    /// Install one selected character under the canonical lifetime owner.
    /// Replacing the same GUID invalidates every older handle.
    pub fn install_detached_player_like_cpp(
        &mut self,
        mut player: Box<Player>,
    ) -> Result<PlayerHandle, PlayerOwnerError> {
        let guid = player.guid();
        if guid.is_empty() {
            return Err(PlayerOwnerError::EmptyGuid);
        }
        if AccessorObjectKind::from_guid(guid) != Some(AccessorObjectKind::Player) {
            return Err(PlayerOwnerError::InvalidGuid { guid });
        }

        // Admission can fail when the incarnation space is exhausted. Reserve it
        // before retiring the existing Player; failed replacement must not destroy
        // that owner. A later retirement failure may consume a generation, never reuse it.
        let generation = self.allocate_player_generation_like_cpp()?;
        if let Some(previous) = self.player_owners_like_cpp.get(&guid).copied() {
            let previous = PlayerHandle {
                guid,
                generation: previous.generation,
            };
            if self.retire_player_like_cpp(previous).is_none() {
                return Err(PlayerOwnerError::ReplacementRetireFailed { guid });
            }
        }

        // A loaded Player may carry its saved map id, but detached ownership
        // means it is not currently contained by any Map.
        player
            .unit_mut()
            .world_mut()
            .object_mut()
            .remove_from_world();
        if player.unit().world().has_current_map() {
            let _ = player.unit_mut().world_mut().reset_map();
        }

        self.detached_players_like_cpp.insert(guid, player);
        self.player_owners_like_cpp.insert(
            guid,
            PlayerOwnershipLikeCpp {
                generation,
                residence: PlayerResidenceLikeCpp::Detached,
            },
        );
        Ok(PlayerHandle { guid, generation })
    }

    pub fn player_residence_like_cpp(
        &self,
        handle: PlayerHandle,
    ) -> Option<PlayerResidenceLikeCpp> {
        self.current_player_owner_like_cpp(handle)
            .map(|owner| owner.residence)
    }

    pub fn with_player_like_cpp<R>(
        &self,
        handle: PlayerHandle,
        read: impl FnOnce(&Player) -> R,
    ) -> Option<R> {
        match self.current_player_owner_like_cpp(handle)?.residence {
            PlayerResidenceLikeCpp::Detached => self
                .detached_players_like_cpp
                .get(&handle.guid)
                .map(|player| read(player)),
            PlayerResidenceLikeCpp::Active(key) => self
                .find_map(key.map_id, key.instance_id)?
                .map()
                .get_typed_player(handle.guid)
                .map(read),
        }
    }

    pub fn with_player_mut_like_cpp<R>(
        &mut self,
        handle: PlayerHandle,
        write: impl FnOnce(&mut Player) -> R,
    ) -> Option<R> {
        match self.current_player_owner_like_cpp(handle)?.residence {
            PlayerResidenceLikeCpp::Detached => self
                .detached_players_like_cpp
                .get_mut(&handle.guid)
                .map(|player| write(player)),
            PlayerResidenceLikeCpp::Active(key) => self
                .find_map_mut(key.map_id, key.instance_id)?
                .map_mut()
                .get_typed_player_mut(handle.guid)
                .map(write),
        }
    }

    pub fn attach_player_like_cpp(
        &mut self,
        handle: PlayerHandle,
        key: MapKey,
        position: Position,
    ) -> Result<(), PlayerOwnerError> {
        if !crate::coords::is_valid_map_coord_2d(position.x, position.y) {
            return Err(PlayerOwnerError::InvalidPosition { position });
        }
        let owner = self
            .current_player_owner_like_cpp(handle)
            .ok_or(PlayerOwnerError::StaleHandle)?;
        if owner.residence != PlayerResidenceLikeCpp::Detached {
            return Err(PlayerOwnerError::NotDetached);
        }
        if self.find_map(key.map_id, key.instance_id).is_none() {
            return Err(PlayerOwnerError::MissingMap { key });
        }
        let player = self
            .detached_players_like_cpp
            .remove(&handle.guid)
            .ok_or(PlayerOwnerError::MissingPlayer { guid: handle.guid })?;
        let attach = self
            .find_map_mut(key.map_id, key.instance_id)
            .expect("target map was checked before Player insertion")
            .runtime
            .attach_player_like_cpp(player, position);
        if let Err((error, player)) = attach {
            self.detached_players_like_cpp.insert(handle.guid, player);
            return Err(match error {
                MapRuntimePlayerAttachErrorLikeCpp::InvalidGuid { guid } => {
                    PlayerOwnerError::InvalidGuid { guid }
                }
                MapRuntimePlayerAttachErrorLikeCpp::InvalidPosition { position } => {
                    PlayerOwnerError::InvalidPosition { position }
                }
                MapRuntimePlayerAttachErrorLikeCpp::PlayerStillInWorld { guid } => {
                    PlayerOwnerError::DetachedPlayerStillInWorld { guid }
                }
                MapRuntimePlayerAttachErrorLikeCpp::PlayerStillBound { guid, key } => {
                    PlayerOwnerError::DetachedPlayerStillBound { guid, key }
                }
                MapRuntimePlayerAttachErrorLikeCpp::ObjectAlreadyPresent { guid } => {
                    PlayerOwnerError::ActiveObjectAlreadyPresent { guid, key }
                }
            });
        }
        self.player_owners_like_cpp
            .get_mut(&handle.guid)
            .expect("current Player handle must retain its owner row")
            .residence = PlayerResidenceLikeCpp::Active(key);
        Ok(())
    }

    pub fn detach_player_like_cpp(&mut self, handle: PlayerHandle) -> Result<(), PlayerOwnerError> {
        let owner = self
            .current_player_owner_like_cpp(handle)
            .ok_or(PlayerOwnerError::StaleHandle)?;
        let PlayerResidenceLikeCpp::Active(key) = owner.residence else {
            return Err(PlayerOwnerError::NotActive);
        };
        let player = self
            .find_map_mut(key.map_id, key.instance_id)
            .ok_or(PlayerOwnerError::MissingMap { key })?
            .runtime
            .detach_player_like_cpp(handle.guid)
            .map_err(|error| match error {
                MapRuntimePlayerDetachErrorLikeCpp::MissingPlayer { guid } => {
                    PlayerOwnerError::MissingPlayer { guid }
                }
                MapRuntimePlayerDetachErrorLikeCpp::ObjectNotPlayer { guid } => {
                    PlayerOwnerError::ActiveObjectNotPlayer { guid, key }
                }
                MapRuntimePlayerDetachErrorLikeCpp::RemoveFromMap(error) => {
                    PlayerOwnerError::RemoveFromMap(error)
                }
            })?;
        self.detached_players_like_cpp.insert(handle.guid, player);
        self.player_owners_like_cpp
            .get_mut(&handle.guid)
            .expect("current Player handle must retain its owner row")
            .residence = PlayerResidenceLikeCpp::Detached;
        Ok(())
    }

    /// Relocate the current active incarnation through its owning map runtime.
    /// A detached Player is deliberately rejected: it remains a real value,
    /// but C++ movement packets have no `Map::PlayerRelocation` target during
    /// the far-teleport window.
    pub fn relocate_player_like_cpp(
        &mut self,
        handle: PlayerHandle,
        position: Position,
    ) -> Result<MapObjectRelocationOutcome, PlayerOwnerError> {
        if !crate::coords::is_valid_map_coord_2d(position.x, position.y) {
            return Err(PlayerOwnerError::InvalidPosition { position });
        }
        let owner = self
            .current_player_owner_like_cpp(handle)
            .ok_or(PlayerOwnerError::StaleHandle)?;
        let PlayerResidenceLikeCpp::Active(key) = owner.residence else {
            return Err(PlayerOwnerError::NotActive);
        };
        self.find_map_mut(key.map_id, key.instance_id)
            .ok_or(PlayerOwnerError::MissingMap { key })?
            .runtime
            .relocate_player_like_cpp(handle.guid, position)
            .map_err(|error| match error {
                MapRuntimePlayerRelocationErrorLikeCpp::MissingPlayer { guid } => {
                    PlayerOwnerError::MissingPlayer { guid }
                }
                MapRuntimePlayerRelocationErrorLikeCpp::ObjectNotPlayer { guid } => {
                    PlayerOwnerError::ActiveObjectNotPlayer { guid, key }
                }
                MapRuntimePlayerRelocationErrorLikeCpp::Relocation(error) => {
                    PlayerOwnerError::RelocatePlayer(error)
                }
            })
    }

    pub fn retire_player_like_cpp(&mut self, handle: PlayerHandle) -> Option<Box<Player>> {
        let owner = self.current_player_owner_like_cpp(handle)?;
        if matches!(owner.residence, PlayerResidenceLikeCpp::Active(_))
            && self.detach_player_like_cpp(handle).is_err()
        {
            return None;
        }
        self.player_owners_like_cpp.remove(&handle.guid);
        self.detached_players_like_cpp.remove(&handle.guid)
    }

    fn current_player_owner_like_cpp(
        &self,
        handle: PlayerHandle,
    ) -> Option<PlayerOwnershipLikeCpp> {
        self.player_owners_like_cpp
            .get(&handle.guid)
            .copied()
            .filter(|owner| owner.generation == handle.generation)
    }

    fn allocate_player_generation_like_cpp(&mut self) -> Result<u64, PlayerOwnerError> {
        let generation = self.next_player_generation_like_cpp;
        self.next_player_generation_like_cpp = generation
            .checked_add(1)
            .ok_or(PlayerOwnerError::GenerationExhausted)?;
        Ok(generation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn detached_player(counter: i64, level: u8) -> Box<Player> {
        let mut player = Box::new(Player::new(Some(counter as u64), false));
        player
            .unit_mut()
            .world_mut()
            .object_mut()
            .create(ObjectGuid::create_player(1, counter));
        player.unit_mut().set_level(level);
        player
    }

    #[test]
    fn one_player_value_moves_detached_to_map_and_back_like_cpp() {
        let mut manager = MapManager::default();
        manager.create_world_map(530, 0);
        let handle = manager
            .install_detached_player_like_cpp(detached_player(90_001, 27))
            .unwrap();

        assert_eq!(
            manager.player_residence_like_cpp(handle),
            Some(PlayerResidenceLikeCpp::Detached)
        );
        manager
            .attach_player_like_cpp(handle, MapKey::new(530, 0), Position::xyz(1.0, 2.0, 3.0))
            .unwrap();
        assert_eq!(
            manager.player_residence_like_cpp(handle),
            Some(PlayerResidenceLikeCpp::Active(MapKey::new(530, 0)))
        );
        assert_eq!(
            manager.with_player_like_cpp(handle, |player| player.unit().data().level),
            Some(27)
        );

        manager.detach_player_like_cpp(handle).unwrap();
        assert_eq!(
            manager.player_residence_like_cpp(handle),
            Some(PlayerResidenceLikeCpp::Detached)
        );
        assert_eq!(
            manager.with_player_like_cpp(handle, |player| player.unit().data().level),
            Some(27)
        );
    }

    #[test]
    fn replacement_invalidates_the_old_generation_like_cpp() {
        let mut manager = MapManager::default();
        let old = manager
            .install_detached_player_like_cpp(detached_player(90_002, 10))
            .unwrap();
        let current = manager
            .install_detached_player_like_cpp(detached_player(90_002, 11))
            .unwrap();

        assert_ne!(old.generation(), current.generation());
        assert_eq!(
            manager.with_player_like_cpp(old, |player| player.unit().data().level),
            None
        );
        assert_eq!(
            manager.with_player_like_cpp(current, |player| player.unit().data().level),
            Some(11)
        );
    }

    #[test]
    fn active_player_relocation_moves_the_owner_and_cell_index_like_cpp() {
        let mut manager = MapManager::default();
        let key = MapKey::new(571, 0);
        manager.create_world_map(key.map_id, key.instance_id);
        let handle = manager
            .install_detached_player_like_cpp(detached_player(90_007, 23))
            .unwrap();
        manager
            .attach_player_like_cpp(handle, key, Position::xyz(1.0, 2.0, 3.0))
            .unwrap();

        let position = Position::xyz(90.0, 20.0, 5.0);
        let outcome = manager.relocate_player_like_cpp(handle, position).unwrap();

        assert!(outcome.relocated);
        assert!(outcome.moved_between_cells);
        assert_eq!(
            manager.with_player_like_cpp(handle, |player| {
                (
                    player.unit().world().position(),
                    player.unit().world().current_cell(),
                )
            }),
            Some((
                position,
                Some((
                    outcome.new_cell.x_coord % crate::MAX_NUMBER_OF_CELLS,
                    outcome.new_cell.y_coord % crate::MAX_NUMBER_OF_CELLS,
                )),
            ))
        );
    }

    #[test]
    fn detached_and_stale_players_cannot_enter_map_relocation_like_cpp() {
        let mut manager = MapManager::default();
        let key = MapKey::new(571, 0);
        manager.create_world_map(key.map_id, key.instance_id);
        let detached = manager
            .install_detached_player_like_cpp(detached_player(90_008, 23))
            .unwrap();
        let destination = Position::xyz(90.0, 20.0, 5.0);

        assert_eq!(
            manager.relocate_player_like_cpp(detached, destination),
            Err(PlayerOwnerError::NotActive)
        );
        assert_eq!(
            manager.with_player_like_cpp(detached, |player| player.unit().world().position()),
            Some(Position::ZERO),
            "a rejected map relocation must not mutate the detached owner"
        );

        let current = manager
            .install_detached_player_like_cpp(detached_player(90_008, 24))
            .unwrap();
        manager
            .attach_player_like_cpp(current, key, Position::xyz(1.0, 2.0, 3.0))
            .unwrap();
        assert_eq!(
            manager.relocate_player_like_cpp(detached, destination),
            Err(PlayerOwnerError::StaleHandle)
        );
        assert_eq!(
            manager.with_player_like_cpp(current, |player| player.unit().world().position()),
            Some(Position::xyz(1.0, 2.0, 3.0)),
            "a stale session cannot move the replacement incarnation"
        );
    }

    #[test]
    fn attach_to_missing_map_keeps_the_player_detached_like_cpp() {
        let mut manager = MapManager::default();
        let handle = manager
            .install_detached_player_like_cpp(detached_player(90_003, 12))
            .unwrap();

        assert_eq!(
            manager.attach_player_like_cpp(
                handle,
                MapKey::new(571, 0),
                Position::xyz(4.0, 5.0, 6.0)
            ),
            Err(PlayerOwnerError::MissingMap {
                key: MapKey::new(571, 0)
            })
        );
        assert_eq!(
            manager.player_residence_like_cpp(handle),
            Some(PlayerResidenceLikeCpp::Detached)
        );
    }

    #[test]
    fn invalid_destination_keeps_the_player_detached_like_cpp() {
        let mut manager = MapManager::default();
        manager.create_world_map(571, 0);
        let handle = manager
            .install_detached_player_like_cpp(detached_player(90_004, 13))
            .unwrap();
        let invalid = Position::xyz(100_000.0, 5.0, 6.0);

        assert_eq!(
            manager.attach_player_like_cpp(handle, MapKey::new(571, 0), invalid),
            Err(PlayerOwnerError::InvalidPosition { position: invalid })
        );
        assert_eq!(
            manager.player_residence_like_cpp(handle),
            Some(PlayerResidenceLikeCpp::Detached)
        );
        assert_eq!(
            manager.with_player_like_cpp(handle, |player| player.unit().data().level),
            Some(13)
        );
    }

    #[test]
    fn conflicting_active_record_keeps_the_owned_player_detached_like_cpp() {
        let mut manager = MapManager::default();
        let key = MapKey::new(571, 0);
        manager.create_world_map(key.map_id, key.instance_id);
        let mut existing = detached_player(90_006, 34);
        existing
            .unit_mut()
            .world_mut()
            .set_map(key.map_id, key.instance_id)
            .unwrap();
        existing
            .unit_mut()
            .world_mut()
            .relocate(Position::xyz(7.0, 8.0, 9.0));
        manager
            .find_map_mut(key.map_id, key.instance_id)
            .unwrap()
            .map_mut()
            .add_map_object_record_to_map_like_cpp(
                MapObjectRecord::new_boxed_player(existing).unwrap(),
            )
            .unwrap();

        let handle = manager
            .install_detached_player_like_cpp(detached_player(90_006, 12))
            .unwrap();
        assert_eq!(
            manager.attach_player_like_cpp(handle, key, Position::xyz(1.0, 2.0, 3.0)),
            Err(PlayerOwnerError::ActiveObjectAlreadyPresent {
                guid: handle.guid(),
                key,
            })
        );
        assert_eq!(
            manager.player_residence_like_cpp(handle),
            Some(PlayerResidenceLikeCpp::Detached)
        );
        assert_eq!(
            manager.with_player_like_cpp(handle, |player| player.unit().data().level),
            Some(12),
            "rejected attach returns the exact detached Player to its owner"
        );
        assert_eq!(
            manager
                .find_map(key.map_id, key.instance_id)
                .unwrap()
                .map()
                .get_typed_player(handle.guid())
                .unwrap()
                .unit()
                .data()
                .level,
            34,
            "the pre-existing map record is not overwritten"
        );
    }

    #[test]
    fn existing_map_player_is_adopted_without_replacement_like_cpp() {
        let mut manager = MapManager::default();
        let key = MapKey::new(530, 0);
        manager.create_world_map(key.map_id, key.instance_id);
        let mut player = detached_player(90_005, 34);
        player
            .unit_mut()
            .world_mut()
            .set_map(key.map_id, key.instance_id)
            .unwrap();
        player
            .unit_mut()
            .world_mut()
            .relocate(Position::xyz(7.0, 8.0, 9.0));
        manager
            .find_map_mut(key.map_id, key.instance_id)
            .unwrap()
            .map_mut()
            .add_map_object_record_to_map_like_cpp(
                MapObjectRecord::new_boxed_player(player).unwrap(),
            )
            .unwrap();

        let handle = manager
            .adopt_active_player_like_cpp(ObjectGuid::create_player(1, 90_005))
            .unwrap();
        assert_eq!(
            manager.player_residence_like_cpp(handle),
            Some(PlayerResidenceLikeCpp::Active(key))
        );
        assert_eq!(
            manager.with_player_like_cpp(handle, |player| player.unit().data().level),
            Some(34)
        );
    }
}
