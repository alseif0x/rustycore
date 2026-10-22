// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Player corpse ownership on the Map (#43).
//!
//! C++ `Map::AddCorpse`, `Map::GetCorpseByPlayer`, `Map::RemoveCorpse` and
//! `Map::ConvertCorpseToBones` (`Maps/Map.cpp:3716-3802`). C++ keeps a
//! `_corpsesByPlayer` index; Rust derives the same answer from the typed
//! corpse records (one resurrectable corpse per owner, bones excluded), so
//! there is no second mutable index to keep in step. Database writes stay
//! with the caller's persistence port; this module is synchronous Map state.

use super::*;

use crate::player_corpse_types::{CORPSE_FLAG_BONES_LIKE_CPP, ConvertCorpseToBonesOutcomeLikeCpp};

impl<Terrain, Lifecycle> Map<Terrain, Lifecycle>
where
    Terrain: TerrainGridLoader,
    Lifecycle: GridLifecycle,
{
    /// C++ `Map::GetCorpseByPlayer`: the owner's resurrectable corpse.
    pub fn corpse_guid_by_player_like_cpp(&self, owner: ObjectGuid) -> Option<ObjectGuid> {
        self.entity_world.iter().find_map(|(guid, record)| {
            if record.kind() != AccessorObjectKind::Corpse {
                return None;
            }
            let corpse = record.corpse()?;
            (corpse.corpse_type() != wow_entities::CorpseType::Bones
                && corpse.data().owner == owner)
                .then_some(*guid)
        })
    }

    /// C++ `Map::IsRemovalGrid`: no grid, or a grid in `GRID_STATE_REMOVAL`.
    pub fn is_removal_grid_like_cpp(&self, x: f32, y: f32) -> bool {
        let cell = Cell::from_world(x, y);
        self.get_ngrid(GridCoord::new(cell.grid_x(), cell.grid_y()))
            .is_none_or(|grid| grid.state() == GridStateKind::Removal)
    }

    /// C++ `Map::AddCorpse` followed by `Map::AddToMap` for a corpse built by
    /// `Player::CreateCorpse` or `Map::ConvertCorpseToBones`.
    pub fn add_corpse_to_map_like_cpp(
        &mut self,
        mut corpse: Corpse,
    ) -> Result<AddToMapOutcome, AddToMapError> {
        let position = corpse.world().position();
        let cell = Cell::from_world(position.x, position.y);
        corpse.set_cell_coord(cell.cell_x(), cell.cell_y());
        let record = MapObjectRecord::new_corpse(corpse).map_err(MapObjectStoreError::from)?;
        self.add_map_object_record_to_map_like_cpp(record)
    }

    /// The Map half of C++ `Map::ConvertCorpseToBones`: remove the owner's
    /// corpse and, when `bones_enabled` (C++ `insignia` or the
    /// `Death.Bones.*` config) and the corpse's grid is not being removed,
    /// add bones with the corpse's counter, appearance and position. The
    /// caller deletes the corpse rows (`Corpse::DeleteFromDB`).
    pub fn convert_corpse_to_bones_like_cpp(
        &mut self,
        owner: ObjectGuid,
        bones_enabled: bool,
        now: i64,
    ) -> Option<ConvertCorpseToBonesOutcomeLikeCpp> {
        let corpse_guid = self.corpse_guid_by_player_like_cpp(owner)?;
        let corpse = self.get_typed_corpse(corpse_guid)?.clone();

        // C++ `Map::RemoveCorpse`: destroy visibility, then leave the grid.
        if self.remove_from_map_like_cpp(corpse_guid, true).is_err() {
            let _ = self.entity_world.remove(&corpse_guid);
        }

        let position = corpse.world().position();
        let bones_guid = (bones_enabled && !self.is_removal_grid_like_cpp(position.x, position.y))
            .then(|| self.add_bones_for_corpse_like_cpp(&corpse, now))
            .flatten();

        Some(ConvertCorpseToBonesOutcomeLikeCpp {
            corpse_guid,
            bones_guid,
        })
    }

    fn add_bones_for_corpse_like_cpp(&mut self, corpse: &Corpse, now: i64) -> Option<ObjectGuid> {
        let corpse_guid = corpse.world().object().guid();
        let mut bones = Corpse::new_at(wow_entities::CorpseType::Bones, now);
        // C++ `Corpse::Create(corpse->GetGUID().GetCounter(), this)`.
        let bones_guid = ObjectGuid::create_world_object(
            HighGuid::Corpse,
            0,
            corpse_guid.realm_id(),
            self.map_id() as u16,
            0,
            0,
            corpse_guid.counter(),
        );
        bones.world_mut().object_mut().create(bones_guid);
        bones
            .world_mut()
            .set_map(self.map_id(), self.instance_id())
            .ok()?;
        let data = corpse.data().clone();
        bones.replace_all_corpse_dynamic_flags(data.dynamic_flags);
        bones.set_owner_guid(data.owner);
        bones.set_party_guid(data.party_guid);
        bones.set_guild_guid(data.guild_guid);
        bones.set_display_id(data.display_id);
        bones.set_race(data.race_id);
        bones.set_sex(data.sex);
        bones.set_class(data.class);
        bones.set_customizations(data.customizations);
        bones.replace_all_flags(data.flags | CORPSE_FLAG_BONES_LIKE_CPP);
        bones.set_faction_template(data.faction_template);
        bones.world_mut().relocate(corpse.world().position());
        // C++ `PhasingHandler::InheritPhaseShift(bones, corpse)`.
        *bones.world_mut().phase_shift_mut() = corpse.world().phase_shift().clone();
        self.add_corpse_to_map_like_cpp(bones).ok()?;
        Some(bones_guid)
    }
}
