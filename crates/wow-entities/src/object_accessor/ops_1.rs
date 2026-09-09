//! Object accessor records and lookups operations, part 1 of 1.
//!
//! The inherent `ObjectAccessor` impl is divided by responsibility under
//! #648; every method keeps its original body.

use super::*;

impl ObjectAccessor {
    pub fn add_player(
        &mut self,
        name: impl AsRef<str>,
        object: WorldObject,
    ) -> Result<(), ObjectAccessorError> {
        self.add_player_with_inventory(name, object, PlayerInventoryStorage::default())
    }
    pub fn add_player_with_inventory(
        &mut self,
        name: impl AsRef<str>,
        object: WorldObject,
        inventory: PlayerInventoryStorage,
    ) -> Result<(), ObjectAccessorError> {
        self.add_player_with_inventory_and_items(name, object, inventory, [])
    }
    pub fn add_player_with_inventory_and_items(
        &mut self,
        name: impl AsRef<str>,
        object: WorldObject,
        inventory: PlayerInventoryStorage,
        items: impl IntoIterator<Item = Item>,
    ) -> Result<(), ObjectAccessorError> {
        let player = AccessorPlayer::new_with_inventory_and_items(name, object, inventory, items)?;
        self.insert_player_record(player);
        Ok(())
    }
    pub fn add_player_entity(
        &mut self,
        name: impl AsRef<str>,
        player: Player,
    ) -> Result<(), ObjectAccessorError> {
        self.add_player_entity_with_inventory(name, player, PlayerInventoryStorage::default())
    }
    pub fn add_player_entity_with_inventory(
        &mut self,
        name: impl AsRef<str>,
        player: Player,
        inventory: PlayerInventoryStorage,
    ) -> Result<(), ObjectAccessorError> {
        self.add_player_entity_with_inventory_and_items(name, player, inventory, [])
    }
    pub fn add_player_entity_with_inventory_and_items(
        &mut self,
        name: impl AsRef<str>,
        player: Player,
        inventory: PlayerInventoryStorage,
        items: impl IntoIterator<Item = Item>,
    ) -> Result<(), ObjectAccessorError> {
        let player =
            AccessorPlayer::new_player_with_inventory_and_items(name, player, inventory, items)?;
        self.insert_player_record(player);
        Ok(())
    }
    pub(super) fn insert_player_record(&mut self, player: AccessorPlayer) {
        let guid = player.object().guid();
        let normalized_name = player.normalized_name.clone();
        if let Some(previous) = self.players.insert(guid, player) {
            self.player_names.remove(previous.normalized_name());
        }
        self.player_names.insert(normalized_name, guid);
    }
    pub fn player_inventory_mut(
        &mut self,
        guid: ObjectGuid,
    ) -> Option<&mut PlayerInventoryStorage> {
        self.players
            .get_mut(&guid)
            .map(AccessorPlayer::inventory_mut)
    }
    pub fn player_item(&self, player_guid: ObjectGuid, item_guid: ObjectGuid) -> Option<&Item> {
        self.players.get(&player_guid)?.item(item_guid)
    }
    pub fn player_item_mut(
        &mut self,
        player_guid: ObjectGuid,
        item_guid: ObjectGuid,
    ) -> Option<&mut Item> {
        self.players.get_mut(&player_guid)?.item_mut(item_guid)
    }
    pub fn insert_player_item(
        &mut self,
        player_guid: ObjectGuid,
        item: Item,
    ) -> Option<Option<Item>> {
        self.players
            .get_mut(&player_guid)
            .map(|player| player.insert_item(item))
    }
    pub fn remove_player_item(
        &mut self,
        player_guid: ObjectGuid,
        item_guid: ObjectGuid,
    ) -> Option<Item> {
        self.players.get_mut(&player_guid)?.remove_item(item_guid)
    }
    pub fn remove_player(&mut self, guid: ObjectGuid) -> Option<AccessorPlayer> {
        let removed = self.players.remove(&guid)?;
        self.player_names.remove(removed.normalized_name());
        Some(removed)
    }
    pub fn find_connected_player(&self, guid: ObjectGuid) -> Option<&WorldObject> {
        self.players.get(&guid).map(AccessorPlayer::object)
    }
    pub fn find_connected_player_entity(&self, guid: ObjectGuid) -> Option<&Player> {
        self.players.get(&guid)?.player()
    }
    pub fn find_player_entity(&self, guid: ObjectGuid) -> Option<&Player> {
        self.find_connected_player_entity(guid)
            .filter(|player| player.unit().world().object().is_in_world())
    }
    pub fn player_object_mut(&mut self, guid: ObjectGuid) -> Option<&mut WorldObject> {
        self.players.get_mut(&guid).map(AccessorPlayer::object_mut)
    }
    pub fn find_connected_player_by_name(&self, name: &str) -> Option<&WorldObject> {
        let normalized = normalize_player_name(name)?;
        let guid = self.player_names.get(&normalized)?;
        self.find_connected_player(*guid)
    }
    pub fn find_player(&self, guid: ObjectGuid) -> Option<&WorldObject> {
        self.find_connected_player(guid)
            .filter(|player| player.object().is_in_world())
    }
    pub fn find_player_by_name(&self, name: &str) -> Option<&WorldObject> {
        self.find_connected_player_by_name(name)
            .filter(|player| player.object().is_in_world())
    }
    pub fn find_player_by_low_guid(&self, low_guid: i64) -> Option<&WorldObject> {
        self.players
            .values()
            .find(|player| player.object().guid().counter() == low_guid)
            .map(AccessorPlayer::object)
            .filter(|player| player.object().is_in_world())
    }
    pub fn players(&self) -> impl Iterator<Item = (&ObjectGuid, &AccessorPlayer)> {
        self.players.iter()
    }
    pub fn save_all_players_with<F, E>(&self, mut save: F) -> Result<usize, PlayerSaveError<E>>
    where
        F: FnMut(&AccessorPlayer) -> Result<(), E>,
    {
        let mut saved = 0;
        for player in self.players.values() {
            save(player).map_err(|source| PlayerSaveError {
                guid: player.object().guid(),
                source,
            })?;
            saved += 1;
        }
        Ok(saved)
    }
    /// Legacy/test-only convenience mirroring the previous bridge helper.
    /// Use `save_all_players_with` when representing Trinity's real
    /// `ObjectAccessor::SaveAllPlayers()` behavior.
    pub fn save_all_players_count(&self) -> usize {
        self.players.len()
    }
    pub fn get_world_object(
        &self,
        context: &WorldObject,
        guid: ObjectGuid,
    ) -> Option<&WorldObject> {
        match AccessorObjectKind::from_guid(guid)? {
            AccessorObjectKind::Player => self.get_player(context, guid),
            AccessorObjectKind::Creature
            | AccessorObjectKind::Pet
            | AccessorObjectKind::GameObject
            | AccessorObjectKind::Transport
            | AccessorObjectKind::DynamicObject
            | AccessorObjectKind::AreaTrigger
            | AccessorObjectKind::Corpse
            | AccessorObjectKind::SceneObject
            | AccessorObjectKind::Conversation => None,
        }
    }
    pub fn get_object_by_type_mask(
        &self,
        context: &WorldObject,
        guid: ObjectGuid,
        type_mask: TypeMask,
    ) -> Option<&WorldObject> {
        match self.get_object_ref_by_type_mask(context, guid, type_mask)? {
            AccessorObjectRef::WorldObject(object) => Some(object),
            AccessorObjectRef::Item(_) => None,
        }
    }
    pub fn get_object_ref_by_type_mask(
        &self,
        context: &WorldObject,
        guid: ObjectGuid,
        type_mask: TypeMask,
    ) -> Option<AccessorObjectRef<'_>> {
        if guid.high_type() == HighGuid::Item {
            return self.get_item_ref_for_player_context(context, guid, type_mask);
        }

        match AccessorObjectKind::from_guid(guid)? {
            AccessorObjectKind::Player if type_mask.contains(TypeMask::PLAYER) => self
                .get_player(context, guid)
                .map(AccessorObjectRef::WorldObject),
            AccessorObjectKind::GameObject
            | AccessorObjectKind::Transport
            | AccessorObjectKind::Creature
            | AccessorObjectKind::Pet
            | AccessorObjectKind::DynamicObject
            | AccessorObjectKind::AreaTrigger
            | AccessorObjectKind::SceneObject
            | AccessorObjectKind::Conversation
            | AccessorObjectKind::Corpse => None,
            _ => None,
        }
    }
    pub fn get_world_object_from_map_source<'a, Source>(
        &'a self,
        context: &WorldObject,
        source: &'a Source,
        guid: ObjectGuid,
    ) -> Option<&'a WorldObject>
    where
        Source: ObjectAccessorMapSource + ?Sized,
    {
        match AccessorObjectKind::from_guid(guid)? {
            AccessorObjectKind::Player => self.get_player(context, guid),
            AccessorObjectKind::Creature => self.get_map_object_from_source(
                context,
                source,
                guid,
                &[AccessorObjectKind::Creature],
            ),
            AccessorObjectKind::Pet => {
                self.get_map_object_from_source(context, source, guid, &[AccessorObjectKind::Pet])
            }
            AccessorObjectKind::GameObject | AccessorObjectKind::Transport => self
                .get_map_object_from_source(
                    context,
                    source,
                    guid,
                    &[
                        AccessorObjectKind::GameObject,
                        AccessorObjectKind::Transport,
                    ],
                ),
            AccessorObjectKind::DynamicObject => self.get_map_object_from_source(
                context,
                source,
                guid,
                &[AccessorObjectKind::DynamicObject],
            ),
            AccessorObjectKind::AreaTrigger => self.get_map_object_from_source(
                context,
                source,
                guid,
                &[AccessorObjectKind::AreaTrigger],
            ),
            AccessorObjectKind::Corpse => self.get_map_object_from_source(
                context,
                source,
                guid,
                &[AccessorObjectKind::Corpse],
            ),
            AccessorObjectKind::SceneObject => self.get_map_object_from_source(
                context,
                source,
                guid,
                &[AccessorObjectKind::SceneObject],
            ),
            AccessorObjectKind::Conversation => self.get_map_object_from_source(
                context,
                source,
                guid,
                &[AccessorObjectKind::Conversation],
            ),
        }
    }
    pub fn get_object_ref_by_type_mask_from_map_source<'a, Source>(
        &'a self,
        context: &WorldObject,
        source: &'a Source,
        guid: ObjectGuid,
        type_mask: TypeMask,
    ) -> Option<AccessorObjectRef<'a>>
    where
        Source: ObjectAccessorMapSource + ?Sized,
    {
        if guid.high_type() == HighGuid::Item {
            return self.get_item_ref_for_player_context(context, guid, type_mask);
        }

        match AccessorObjectKind::from_guid(guid)? {
            AccessorObjectKind::Player if type_mask.contains(TypeMask::PLAYER) => self
                .get_player(context, guid)
                .map(AccessorObjectRef::WorldObject),
            AccessorObjectKind::GameObject | AccessorObjectKind::Transport
                if type_mask.contains(TypeMask::GAME_OBJECT) =>
            {
                self.get_map_object_from_source(
                    context,
                    source,
                    guid,
                    &[
                        AccessorObjectKind::GameObject,
                        AccessorObjectKind::Transport,
                    ],
                )
                .map(AccessorObjectRef::WorldObject)
            }
            AccessorObjectKind::Creature | AccessorObjectKind::Pet
                if type_mask.contains(TypeMask::UNIT) =>
            {
                self.get_map_object_from_source(
                    context,
                    source,
                    guid,
                    &[AccessorObjectKind::Creature, AccessorObjectKind::Pet],
                )
                .map(AccessorObjectRef::WorldObject)
            }
            AccessorObjectKind::DynamicObject if type_mask.contains(TypeMask::DYNAMIC_OBJECT) => {
                self.get_map_object_from_source(
                    context,
                    source,
                    guid,
                    &[AccessorObjectKind::DynamicObject],
                )
                .map(AccessorObjectRef::WorldObject)
            }
            AccessorObjectKind::AreaTrigger if type_mask.contains(TypeMask::AREA_TRIGGER) => self
                .get_map_object_from_source(
                    context,
                    source,
                    guid,
                    &[AccessorObjectKind::AreaTrigger],
                )
                .map(AccessorObjectRef::WorldObject),
            AccessorObjectKind::SceneObject if type_mask.contains(TypeMask::SCENE_OBJECT) => self
                .get_map_object_from_source(
                    context,
                    source,
                    guid,
                    &[AccessorObjectKind::SceneObject],
                )
                .map(AccessorObjectRef::WorldObject),
            AccessorObjectKind::Conversation if type_mask.contains(TypeMask::CONVERSATION) => self
                .get_map_object_from_source(
                    context,
                    source,
                    guid,
                    &[AccessorObjectKind::Conversation],
                )
                .map(AccessorObjectRef::WorldObject),
            AccessorObjectKind::Corpse => None,
            _ => None,
        }
    }
    pub fn get_unit_from_map_source<'a, Source>(
        &'a self,
        context: &WorldObject,
        source: &'a Source,
        guid: ObjectGuid,
    ) -> Option<&'a WorldObject>
    where
        Source: ObjectAccessorMapSource + ?Sized,
    {
        if guid.is_player() {
            return self.get_player(context, guid);
        }
        if guid.is_pet() {
            return self.get_pet_from_map_source(context, source, guid);
        }
        self.get_creature_from_map_source(context, source, guid)
    }
    pub fn get_creature_from_map_source<'a, Source>(
        &'a self,
        context: &WorldObject,
        source: &'a Source,
        guid: ObjectGuid,
    ) -> Option<&'a WorldObject>
    where
        Source: ObjectAccessorMapSource + ?Sized,
    {
        self.get_map_object_from_source(context, source, guid, &[AccessorObjectKind::Creature])
    }
    pub fn get_typed_creature_from_map_source<'a, Source>(
        &'a self,
        context: &WorldObject,
        source: &'a Source,
        guid: ObjectGuid,
    ) -> Option<&'a Creature>
    where
        Source: ObjectAccessorMapSource + ?Sized,
    {
        self.get_map_record_from_source(context, source, guid, &[AccessorObjectKind::Creature])?
            .creature()
    }
    pub fn get_pet_from_map_source<'a, Source>(
        &'a self,
        context: &WorldObject,
        source: &'a Source,
        guid: ObjectGuid,
    ) -> Option<&'a WorldObject>
    where
        Source: ObjectAccessorMapSource + ?Sized,
    {
        self.get_map_object_from_source(context, source, guid, &[AccessorObjectKind::Pet])
    }
    pub fn get_typed_pet_from_map_source<'a, Source>(
        &'a self,
        context: &WorldObject,
        source: &'a Source,
        guid: ObjectGuid,
    ) -> Option<&'a Pet>
    where
        Source: ObjectAccessorMapSource + ?Sized,
    {
        self.get_map_record_from_source(context, source, guid, &[AccessorObjectKind::Pet])?
            .pet()
    }
    pub fn get_creature_or_pet_or_vehicle_from_map_source<'a, Source>(
        &'a self,
        context: &WorldObject,
        source: &'a Source,
        guid: ObjectGuid,
    ) -> Option<&'a WorldObject>
    where
        Source: ObjectAccessorMapSource + ?Sized,
    {
        if guid.is_pet() {
            return self.get_pet_from_map_source(context, source, guid);
        }
        if guid.is_creature_or_vehicle() {
            return self.get_creature_from_map_source(context, source, guid);
        }
        None
    }
    pub fn get_game_object_from_map_source<'a, Source>(
        &'a self,
        context: &WorldObject,
        source: &'a Source,
        guid: ObjectGuid,
    ) -> Option<&'a WorldObject>
    where
        Source: ObjectAccessorMapSource + ?Sized,
    {
        self.get_map_object_from_source(
            context,
            source,
            guid,
            &[
                AccessorObjectKind::GameObject,
                AccessorObjectKind::Transport,
            ],
        )
    }
    pub fn get_typed_game_object_from_map_source<'a, Source>(
        &'a self,
        context: &WorldObject,
        source: &'a Source,
        guid: ObjectGuid,
    ) -> Option<&'a GameObject>
    where
        Source: ObjectAccessorMapSource + ?Sized,
    {
        self.get_map_record_from_source(
            context,
            source,
            guid,
            &[
                AccessorObjectKind::GameObject,
                AccessorObjectKind::Transport,
            ],
        )?
        .game_object()
    }
    pub fn get_typed_transport_from_map_source<'a, Source>(
        &'a self,
        context: &WorldObject,
        source: &'a Source,
        guid: ObjectGuid,
    ) -> Option<&'a Transport>
    where
        Source: ObjectAccessorMapSource + ?Sized,
    {
        self.get_map_record_from_source(context, source, guid, &[AccessorObjectKind::Transport])?
            .transport()
    }
    pub fn get_dynamic_object_from_map_source<'a, Source>(
        &'a self,
        context: &WorldObject,
        source: &'a Source,
        guid: ObjectGuid,
    ) -> Option<&'a WorldObject>
    where
        Source: ObjectAccessorMapSource + ?Sized,
    {
        self.get_map_object_from_source(context, source, guid, &[AccessorObjectKind::DynamicObject])
    }
    pub fn get_typed_dynamic_object_from_map_source<'a, Source>(
        &'a self,
        context: &WorldObject,
        source: &'a Source,
        guid: ObjectGuid,
    ) -> Option<&'a DynamicObject>
    where
        Source: ObjectAccessorMapSource + ?Sized,
    {
        self.get_map_record_from_source(
            context,
            source,
            guid,
            &[AccessorObjectKind::DynamicObject],
        )?
        .dynamic_object()
    }
    pub fn get_area_trigger_from_map_source<'a, Source>(
        &'a self,
        context: &WorldObject,
        source: &'a Source,
        guid: ObjectGuid,
    ) -> Option<&'a WorldObject>
    where
        Source: ObjectAccessorMapSource + ?Sized,
    {
        self.get_map_object_from_source(context, source, guid, &[AccessorObjectKind::AreaTrigger])
    }
    pub fn get_typed_area_trigger_from_map_source<'a, Source>(
        &'a self,
        context: &WorldObject,
        source: &'a Source,
        guid: ObjectGuid,
    ) -> Option<&'a AreaTrigger>
    where
        Source: ObjectAccessorMapSource + ?Sized,
    {
        self.get_map_record_from_source(context, source, guid, &[AccessorObjectKind::AreaTrigger])?
            .area_trigger()
    }
    pub fn get_corpse_from_map_source<'a, Source>(
        &'a self,
        context: &WorldObject,
        source: &'a Source,
        guid: ObjectGuid,
    ) -> Option<&'a WorldObject>
    where
        Source: ObjectAccessorMapSource + ?Sized,
    {
        self.get_map_object_from_source(context, source, guid, &[AccessorObjectKind::Corpse])
    }
    pub fn get_typed_corpse_from_map_source<'a, Source>(
        &'a self,
        context: &WorldObject,
        source: &'a Source,
        guid: ObjectGuid,
    ) -> Option<&'a Corpse>
    where
        Source: ObjectAccessorMapSource + ?Sized,
    {
        self.get_map_record_from_source(context, source, guid, &[AccessorObjectKind::Corpse])?
            .corpse()
    }
    pub fn get_scene_object_from_map_source<'a, Source>(
        &'a self,
        context: &WorldObject,
        source: &'a Source,
        guid: ObjectGuid,
    ) -> Option<&'a WorldObject>
    where
        Source: ObjectAccessorMapSource + ?Sized,
    {
        self.get_map_object_from_source(context, source, guid, &[AccessorObjectKind::SceneObject])
    }
    pub fn get_typed_scene_object_from_map_source<'a, Source>(
        &'a self,
        context: &WorldObject,
        source: &'a Source,
        guid: ObjectGuid,
    ) -> Option<&'a SceneObject>
    where
        Source: ObjectAccessorMapSource + ?Sized,
    {
        self.get_map_record_from_source(context, source, guid, &[AccessorObjectKind::SceneObject])?
            .scene_object()
    }
    pub fn get_conversation_from_map_source<'a, Source>(
        &'a self,
        context: &WorldObject,
        source: &'a Source,
        guid: ObjectGuid,
    ) -> Option<&'a WorldObject>
    where
        Source: ObjectAccessorMapSource + ?Sized,
    {
        self.get_map_object_from_source(context, source, guid, &[AccessorObjectKind::Conversation])
    }
    pub fn get_typed_conversation_from_map_source<'a, Source>(
        &'a self,
        context: &WorldObject,
        source: &'a Source,
        guid: ObjectGuid,
    ) -> Option<&'a Conversation>
    where
        Source: ObjectAccessorMapSource + ?Sized,
    {
        self.get_map_record_from_source(context, source, guid, &[AccessorObjectKind::Conversation])?
            .conversation()
    }
    pub fn get_player(&self, context: &WorldObject, guid: ObjectGuid) -> Option<&WorldObject> {
        let player = self.find_connected_player(guid)?;
        player
            .object()
            .is_in_world()
            .then_some(player)
            .filter(|player| same_map(context, player))
    }
    pub fn get_player_entity(&self, context: &WorldObject, guid: ObjectGuid) -> Option<&Player> {
        let player = self.find_player_entity(guid)?;
        same_map(context, player.unit().world()).then_some(player)
    }
    pub fn get_typed_player_from_map_source<'a, Source>(
        &'a self,
        context: &WorldObject,
        source: &Source,
        guid: ObjectGuid,
    ) -> Option<&'a Player>
    where
        Source: ObjectAccessorMapSource + ?Sized,
    {
        if !context.has_current_map()
            || context.map_id() != source.map_id()
            || context.instance_id() != source.instance_id()
        {
            return None;
        }

        self.get_player_entity(context, guid)
    }
    #[deprecated(note = "map-local unit lookup requires ObjectAccessorMapSource")]
    pub fn get_unit(&self, context: &WorldObject, guid: ObjectGuid) -> Option<&WorldObject> {
        if guid.is_player() {
            return self.get_player(context, guid);
        }
        let _ = (context, guid);
        None
    }
    #[deprecated(note = "map-local creature lookup requires ObjectAccessorMapSource")]
    pub fn get_creature(&self, context: &WorldObject, guid: ObjectGuid) -> Option<&WorldObject> {
        let _ = (context, guid);
        None
    }
    #[deprecated(note = "map-local pet lookup requires ObjectAccessorMapSource")]
    pub fn get_pet(&self, context: &WorldObject, guid: ObjectGuid) -> Option<&WorldObject> {
        let _ = (context, guid);
        None
    }
    #[deprecated(note = "map-local creature/pet/vehicle lookup requires ObjectAccessorMapSource")]
    pub fn get_creature_or_pet_or_vehicle(
        &self,
        context: &WorldObject,
        guid: ObjectGuid,
    ) -> Option<&WorldObject> {
        let _ = (context, guid);
        None
    }
    #[deprecated(note = "map-local gameobject lookup requires ObjectAccessorMapSource")]
    pub fn get_game_object(&self, context: &WorldObject, guid: ObjectGuid) -> Option<&WorldObject> {
        let _ = (context, guid);
        None
    }
    #[deprecated(note = "map-local transport lookup requires ObjectAccessorMapSource")]
    pub fn get_transport(&self, context: &WorldObject, guid: ObjectGuid) -> Option<&WorldObject> {
        let _ = (context, guid);
        None
    }
    #[deprecated(note = "map-local dynamic object lookup requires ObjectAccessorMapSource")]
    pub fn get_dynamic_object(
        &self,
        context: &WorldObject,
        guid: ObjectGuid,
    ) -> Option<&WorldObject> {
        let _ = (context, guid);
        None
    }
    #[deprecated(note = "map-local area trigger lookup requires ObjectAccessorMapSource")]
    pub fn get_area_trigger(
        &self,
        context: &WorldObject,
        guid: ObjectGuid,
    ) -> Option<&WorldObject> {
        let _ = (context, guid);
        None
    }
    #[deprecated(note = "map-local corpse lookup requires ObjectAccessorMapSource")]
    pub fn get_corpse(&self, context: &WorldObject, guid: ObjectGuid) -> Option<&WorldObject> {
        let _ = (context, guid);
        None
    }
    #[deprecated(note = "map-local scene object lookup requires ObjectAccessorMapSource")]
    pub fn get_scene_object(
        &self,
        context: &WorldObject,
        guid: ObjectGuid,
    ) -> Option<&WorldObject> {
        let _ = (context, guid);
        None
    }
    #[deprecated(note = "map-local conversation lookup requires ObjectAccessorMapSource")]
    pub fn get_conversation(
        &self,
        context: &WorldObject,
        guid: ObjectGuid,
    ) -> Option<&WorldObject> {
        let _ = (context, guid);
        None
    }
    pub(super) fn get_map_object_from_source<'a, Source>(
        &'a self,
        context: &WorldObject,
        source: &'a Source,
        guid: ObjectGuid,
        allowed: &[AccessorObjectKind],
    ) -> Option<&'a WorldObject>
    where
        Source: ObjectAccessorMapSource + ?Sized,
    {
        Some(
            self.get_map_record_from_source(context, source, guid, allowed)?
                .object(),
        )
    }
    pub(super) fn get_map_record_from_source<'a, Source>(
        &'a self,
        context: &WorldObject,
        source: &'a Source,
        guid: ObjectGuid,
        allowed: &[AccessorObjectKind],
    ) -> Option<&'a MapObjectRecord>
    where
        Source: ObjectAccessorMapSource + ?Sized,
    {
        if !context.has_current_map()
            || context.map_id() != source.map_id()
            || context.instance_id() != source.instance_id()
        {
            return None;
        }

        let record = source.map_object_record(guid)?;
        if !allowed.contains(&record.kind()) || !same_map(context, record.object()) {
            return None;
        }

        Some(record)
    }
    pub(super) fn get_item_ref_for_player_context(
        &self,
        context: &WorldObject,
        guid: ObjectGuid,
        type_mask: TypeMask,
    ) -> Option<AccessorObjectRef<'_>> {
        if !type_mask.contains(TypeMask::ITEM) || context.object().type_id() != TypeId::Player {
            return None;
        }

        let player = self.players.get(&context.guid())?;
        let item_guid = player.inventory().get_item_by_guid_everywhere(guid)?;
        player.item(item_guid).map(AccessorObjectRef::Item)
    }
}
