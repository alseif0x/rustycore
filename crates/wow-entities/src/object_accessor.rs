use std::collections::HashMap;

use wow_constants::{TypeId, TypeMask};
use wow_core::ObjectGuid;
use wow_core::guid::HighGuid;

use crate::{
    AreaTrigger, Conversation, Corpse, Creature, DynamicObject, GameObject, Item, Pet, Player,
    PlayerInventoryStorage, SceneObject, Transport, WorldObject,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AccessorObjectKind {
    Player,
    Creature,
    Pet,
    GameObject,
    Transport,
    DynamicObject,
    AreaTrigger,
    Corpse,
    SceneObject,
    Conversation,
}

impl AccessorObjectKind {
    pub fn from_guid(guid: ObjectGuid) -> Option<Self> {
        match guid.high_type() {
            HighGuid::Player => Some(Self::Player),
            HighGuid::Creature | HighGuid::Vehicle => Some(Self::Creature),
            HighGuid::Pet => Some(Self::Pet),
            HighGuid::GameObject => Some(Self::GameObject),
            HighGuid::Transport => Some(Self::Transport),
            HighGuid::DynamicObject => Some(Self::DynamicObject),
            HighGuid::AreaTrigger => Some(Self::AreaTrigger),
            HighGuid::Corpse => Some(Self::Corpse),
            HighGuid::SceneObject => Some(Self::SceneObject),
            HighGuid::Conversation => Some(Self::Conversation),
            _ => None,
        }
    }

    pub const fn type_mask(self) -> TypeMask {
        match self {
            Self::Player => TypeMask::PLAYER,
            Self::Creature | Self::Pet => TypeMask::UNIT,
            Self::GameObject | Self::Transport => TypeMask::GAME_OBJECT,
            Self::DynamicObject => TypeMask::DYNAMIC_OBJECT,
            Self::AreaTrigger => TypeMask::AREA_TRIGGER,
            Self::Corpse => TypeMask::CORPSE,
            Self::SceneObject => TypeMask::SCENE_OBJECT,
            Self::Conversation => TypeMask::CONVERSATION,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AccessorPlayer {
    normalized_name: String,
    body: AccessorPlayerBody,
    inventory: PlayerInventoryStorage,
    items: HashMap<ObjectGuid, Item>,
}

#[derive(Debug, Clone, PartialEq)]
enum AccessorPlayerBody {
    WorldObject(WorldObject),
    Player(Box<Player>),
}

impl AccessorPlayer {
    pub fn new(name: impl AsRef<str>, object: WorldObject) -> Result<Self, ObjectAccessorError> {
        Self::new_with_inventory(name, object, PlayerInventoryStorage::default())
    }

    pub fn new_with_inventory(
        name: impl AsRef<str>,
        object: WorldObject,
        inventory: PlayerInventoryStorage,
    ) -> Result<Self, ObjectAccessorError> {
        Self::new_with_inventory_and_items(name, object, inventory, [])
    }

    pub fn new_with_inventory_and_items(
        name: impl AsRef<str>,
        object: WorldObject,
        inventory: PlayerInventoryStorage,
        items: impl IntoIterator<Item = Item>,
    ) -> Result<Self, ObjectAccessorError> {
        if !object.guid().is_player() {
            return Err(ObjectAccessorError::WrongGuidKind {
                guid: object.guid(),
                expected: AccessorObjectKind::Player,
            });
        }

        let normalized_name =
            normalize_player_name(name.as_ref()).ok_or(ObjectAccessorError::InvalidPlayerName)?;

        Ok(Self {
            normalized_name,
            body: AccessorPlayerBody::WorldObject(object),
            inventory,
            items: items
                .into_iter()
                .map(|item| (item.object().guid(), item))
                .collect(),
        })
    }

    pub fn new_player(name: impl AsRef<str>, player: Player) -> Result<Self, ObjectAccessorError> {
        Self::new_player_with_inventory(name, player, PlayerInventoryStorage::default())
    }

    pub fn new_player_with_inventory(
        name: impl AsRef<str>,
        player: Player,
        inventory: PlayerInventoryStorage,
    ) -> Result<Self, ObjectAccessorError> {
        Self::new_player_with_inventory_and_items(name, player, inventory, [])
    }

    pub fn new_player_with_inventory_and_items(
        name: impl AsRef<str>,
        player: Player,
        inventory: PlayerInventoryStorage,
        items: impl IntoIterator<Item = Item>,
    ) -> Result<Self, ObjectAccessorError> {
        if !player.unit().world().guid().is_player() {
            return Err(ObjectAccessorError::WrongGuidKind {
                guid: player.unit().world().guid(),
                expected: AccessorObjectKind::Player,
            });
        }

        let normalized_name =
            normalize_player_name(name.as_ref()).ok_or(ObjectAccessorError::InvalidPlayerName)?;

        Ok(Self {
            normalized_name,
            body: AccessorPlayerBody::Player(Box::new(player)),
            inventory,
            items: items
                .into_iter()
                .map(|item| (item.object().guid(), item))
                .collect(),
        })
    }

    pub fn normalized_name(&self) -> &str {
        &self.normalized_name
    }

    pub fn object(&self) -> &WorldObject {
        match &self.body {
            AccessorPlayerBody::WorldObject(object) => object,
            AccessorPlayerBody::Player(player) => player.unit().world(),
        }
    }

    pub fn object_mut(&mut self) -> &mut WorldObject {
        match &mut self.body {
            AccessorPlayerBody::WorldObject(object) => object,
            AccessorPlayerBody::Player(player) => player.unit_mut().world_mut(),
        }
    }

    pub fn player(&self) -> Option<&Player> {
        match &self.body {
            AccessorPlayerBody::Player(player) => Some(player.as_ref()),
            AccessorPlayerBody::WorldObject(_) => None,
        }
    }

    pub fn player_mut(&mut self) -> Option<&mut Player> {
        match &mut self.body {
            AccessorPlayerBody::Player(player) => Some(player.as_mut()),
            AccessorPlayerBody::WorldObject(_) => None,
        }
    }

    pub const fn inventory(&self) -> &PlayerInventoryStorage {
        &self.inventory
    }

    pub fn inventory_mut(&mut self) -> &mut PlayerInventoryStorage {
        &mut self.inventory
    }

    pub fn item(&self, guid: ObjectGuid) -> Option<&Item> {
        self.items.get(&guid)
    }

    pub fn item_mut(&mut self, guid: ObjectGuid) -> Option<&mut Item> {
        self.items.get_mut(&guid)
    }

    pub fn insert_item(&mut self, item: Item) -> Option<Item> {
        self.items.insert(item.object().guid(), item)
    }

    pub fn remove_item(&mut self, guid: ObjectGuid) -> Option<Item> {
        self.items.remove(&guid)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AccessorObjectRef<'a> {
    WorldObject(&'a WorldObject),
    Item(&'a Item),
}

pub trait PlayerSaveSink {
    type Error;

    fn save_player(&mut self, player: &AccessorPlayer) -> Result<(), Self::Error>;
}

impl<F, E> PlayerSaveSink for F
where
    F: FnMut(&AccessorPlayer) -> Result<(), E>,
{
    type Error = E;

    fn save_player(&mut self, player: &AccessorPlayer) -> Result<(), Self::Error> {
        self(player)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerSaveError<E> {
    pub guid: ObjectGuid,
    pub source: E,
}

pub trait ObjectAccessorMapSource {
    fn map_id(&self) -> u32;
    fn instance_id(&self) -> u32;
    fn map_object_record(&self, guid: ObjectGuid) -> Option<&MapObjectRecord>;
}

#[derive(Debug, Clone, PartialEq)]
pub struct MapObjectRecord {
    kind: AccessorObjectKind,
    body: MapObjectBody,
}

#[derive(Debug, Clone, PartialEq)]
enum MapObjectBody {
    WorldObject(WorldObject),
    AreaTrigger(AreaTrigger),
    Conversation(Conversation),
    Corpse(Corpse),
    Creature(Box<Creature>),
    DynamicObject(DynamicObject),
    GameObject(GameObject),
    Pet(Box<Pet>),
    Player(Box<Player>),
    SceneObject(SceneObject),
    Transport(Box<Transport>),
}

impl MapObjectRecord {
    pub fn new(kind: AccessorObjectKind, object: WorldObject) -> Result<Self, ObjectAccessorError> {
        if !object.has_current_map() {
            return Err(ObjectAccessorError::ObjectHasNoMap {
                guid: object.guid(),
            });
        }

        let guid_kind = AccessorObjectKind::from_guid(object.guid()).ok_or(
            ObjectAccessorError::UnsupportedGuidKind {
                guid: object.guid(),
            },
        )?;
        if !kind_accepts_guid(kind, guid_kind) {
            return Err(ObjectAccessorError::WrongGuidKind {
                guid: object.guid(),
                expected: kind,
            });
        }

        Ok(Self {
            kind,
            body: MapObjectBody::WorldObject(object),
        })
    }

    pub fn new_game_object(game_object: GameObject) -> Result<Self, ObjectAccessorError> {
        let record = Self::new(AccessorObjectKind::GameObject, game_object.world().clone())?;
        Ok(Self {
            kind: record.kind,
            body: MapObjectBody::GameObject(game_object),
        })
    }

    pub fn new_transport(transport: Transport) -> Result<Self, ObjectAccessorError> {
        let record = Self::new(AccessorObjectKind::Transport, transport.world().clone())?;
        Ok(Self {
            kind: record.kind,
            body: MapObjectBody::Transport(Box::new(transport)),
        })
    }

    pub fn new_creature(creature: Creature) -> Result<Self, ObjectAccessorError> {
        let record = Self::new(
            AccessorObjectKind::Creature,
            creature.unit().world().clone(),
        )?;
        Ok(Self {
            kind: record.kind,
            body: MapObjectBody::Creature(Box::new(creature)),
        })
    }

    pub fn new_pet(pet: Pet) -> Result<Self, ObjectAccessorError> {
        let record = Self::new(
            AccessorObjectKind::Pet,
            pet.creature().unit().world().clone(),
        )?;
        Ok(Self {
            kind: record.kind,
            body: MapObjectBody::Pet(Box::new(pet)),
        })
    }

    pub fn new_area_trigger(area_trigger: AreaTrigger) -> Result<Self, ObjectAccessorError> {
        let record = Self::new(
            AccessorObjectKind::AreaTrigger,
            area_trigger.world().clone(),
        )?;
        Ok(Self {
            kind: record.kind,
            body: MapObjectBody::AreaTrigger(area_trigger),
        })
    }

    pub fn new_conversation(conversation: Conversation) -> Result<Self, ObjectAccessorError> {
        let record = Self::new(
            AccessorObjectKind::Conversation,
            conversation.world().clone(),
        )?;
        Ok(Self {
            kind: record.kind,
            body: MapObjectBody::Conversation(conversation),
        })
    }

    pub fn new_corpse(corpse: Corpse) -> Result<Self, ObjectAccessorError> {
        let record = Self::new(AccessorObjectKind::Corpse, corpse.world().clone())?;
        Ok(Self {
            kind: record.kind,
            body: MapObjectBody::Corpse(corpse),
        })
    }

    pub fn new_dynamic_object(dynamic_object: DynamicObject) -> Result<Self, ObjectAccessorError> {
        let record = Self::new(
            AccessorObjectKind::DynamicObject,
            dynamic_object.world().clone(),
        )?;
        Ok(Self {
            kind: record.kind,
            body: MapObjectBody::DynamicObject(dynamic_object),
        })
    }

    pub fn new_scene_object(scene_object: SceneObject) -> Result<Self, ObjectAccessorError> {
        let record = Self::new(
            AccessorObjectKind::SceneObject,
            scene_object.world().clone(),
        )?;
        Ok(Self {
            kind: record.kind,
            body: MapObjectBody::SceneObject(scene_object),
        })
    }

    pub fn new_player(player: Player) -> Result<Self, ObjectAccessorError> {
        Self::new_boxed_player(Box::new(player))
    }

    pub fn new_boxed_player(player: Box<Player>) -> Result<Self, ObjectAccessorError> {
        let record = Self::new(AccessorObjectKind::Player, player.unit().world().clone())?;
        Ok(Self {
            kind: record.kind,
            body: MapObjectBody::Player(player),
        })
    }

    pub const fn kind(&self) -> AccessorObjectKind {
        self.kind
    }

    pub fn object(&self) -> &WorldObject {
        match &self.body {
            MapObjectBody::WorldObject(object) => object,
            MapObjectBody::AreaTrigger(area_trigger) => area_trigger.world(),
            MapObjectBody::Conversation(conversation) => conversation.world(),
            MapObjectBody::Corpse(corpse) => corpse.world(),
            MapObjectBody::Creature(creature) => creature.unit().world(),
            MapObjectBody::DynamicObject(dynamic_object) => dynamic_object.world(),
            MapObjectBody::GameObject(game_object) => game_object.world(),
            MapObjectBody::Pet(pet) => pet.creature().unit().world(),
            MapObjectBody::Player(player) => player.unit().world(),
            MapObjectBody::SceneObject(scene_object) => scene_object.world(),
            MapObjectBody::Transport(transport) => transport.world(),
        }
    }

    pub fn object_mut(&mut self) -> &mut WorldObject {
        match &mut self.body {
            MapObjectBody::WorldObject(object) => object,
            MapObjectBody::AreaTrigger(area_trigger) => area_trigger.world_mut(),
            MapObjectBody::Conversation(conversation) => conversation.world_mut(),
            MapObjectBody::Corpse(corpse) => corpse.world_mut(),
            MapObjectBody::Creature(creature) => creature.unit_mut().world_mut(),
            MapObjectBody::DynamicObject(dynamic_object) => dynamic_object.world_mut(),
            MapObjectBody::GameObject(game_object) => game_object.world_mut(),
            MapObjectBody::Pet(pet) => pet.creature_mut().unit_mut().world_mut(),
            MapObjectBody::Player(player) => player.unit_mut().world_mut(),
            MapObjectBody::SceneObject(scene_object) => scene_object.world_mut(),
            MapObjectBody::Transport(transport) => transport.world_mut(),
        }
    }

    pub fn area_trigger(&self) -> Option<&AreaTrigger> {
        match &self.body {
            MapObjectBody::AreaTrigger(area_trigger) => Some(area_trigger),
            _ => None,
        }
    }

    pub fn area_trigger_mut(&mut self) -> Option<&mut AreaTrigger> {
        match &mut self.body {
            MapObjectBody::AreaTrigger(area_trigger) => Some(area_trigger),
            _ => None,
        }
    }

    pub fn conversation(&self) -> Option<&Conversation> {
        match &self.body {
            MapObjectBody::Conversation(conversation) => Some(conversation),
            _ => None,
        }
    }

    pub fn conversation_mut(&mut self) -> Option<&mut Conversation> {
        match &mut self.body {
            MapObjectBody::Conversation(conversation) => Some(conversation),
            _ => None,
        }
    }

    pub fn corpse(&self) -> Option<&Corpse> {
        match &self.body {
            MapObjectBody::Corpse(corpse) => Some(corpse),
            _ => None,
        }
    }

    pub fn corpse_mut(&mut self) -> Option<&mut Corpse> {
        match &mut self.body {
            MapObjectBody::Corpse(corpse) => Some(corpse),
            _ => None,
        }
    }

    pub fn creature(&self) -> Option<&Creature> {
        match &self.body {
            MapObjectBody::Creature(creature) => Some(creature.as_ref()),
            _ => None,
        }
    }

    pub fn creature_mut(&mut self) -> Option<&mut Creature> {
        match &mut self.body {
            MapObjectBody::Creature(creature) => Some(creature.as_mut()),
            _ => None,
        }
    }

    pub fn dynamic_object(&self) -> Option<&DynamicObject> {
        match &self.body {
            MapObjectBody::DynamicObject(dynamic_object) => Some(dynamic_object),
            _ => None,
        }
    }

    pub fn dynamic_object_mut(&mut self) -> Option<&mut DynamicObject> {
        match &mut self.body {
            MapObjectBody::DynamicObject(dynamic_object) => Some(dynamic_object),
            _ => None,
        }
    }

    pub fn game_object(&self) -> Option<&GameObject> {
        match &self.body {
            MapObjectBody::GameObject(game_object) => Some(game_object),
            MapObjectBody::Transport(transport) => Some(transport.game_object()),
            _ => None,
        }
    }

    pub fn game_object_mut(&mut self) -> Option<&mut GameObject> {
        match &mut self.body {
            MapObjectBody::GameObject(game_object) => Some(game_object),
            MapObjectBody::Transport(transport) => Some(transport.game_object_mut()),
            _ => None,
        }
    }

    pub fn pet(&self) -> Option<&Pet> {
        match &self.body {
            MapObjectBody::Pet(pet) => Some(pet.as_ref()),
            _ => None,
        }
    }

    pub fn pet_mut(&mut self) -> Option<&mut Pet> {
        match &mut self.body {
            MapObjectBody::Pet(pet) => Some(pet.as_mut()),
            _ => None,
        }
    }

    pub fn player(&self) -> Option<&Player> {
        match &self.body {
            MapObjectBody::Player(player) => Some(player.as_ref()),
            _ => None,
        }
    }

    pub fn player_mut(&mut self) -> Option<&mut Player> {
        match &mut self.body {
            MapObjectBody::Player(player) => Some(player.as_mut()),
            _ => None,
        }
    }

    /// Recover the typed Player value when Map ownership is transferred.
    ///
    /// C++ keeps the same `Player*` alive while `Map::RemovePlayerFromMap`
    /// detaches it for a far teleport. Rust stores typed objects by value, so
    /// the Map owner must be able to move that value without cloning it or
    /// degrading it to a `WorldObject` projection.
    pub fn into_player(self) -> Result<Box<Player>, Self> {
        match self.body {
            MapObjectBody::Player(player) => Ok(player),
            body => Err(Self {
                kind: self.kind,
                body,
            }),
        }
    }

    pub fn scene_object(&self) -> Option<&SceneObject> {
        match &self.body {
            MapObjectBody::SceneObject(scene_object) => Some(scene_object),
            _ => None,
        }
    }

    pub fn scene_object_mut(&mut self) -> Option<&mut SceneObject> {
        match &mut self.body {
            MapObjectBody::SceneObject(scene_object) => Some(scene_object),
            _ => None,
        }
    }

    pub fn transport(&self) -> Option<&Transport> {
        match &self.body {
            MapObjectBody::Transport(transport) => Some(transport.as_ref()),
            _ => None,
        }
    }

    pub fn transport_mut(&mut self) -> Option<&mut Transport> {
        match &mut self.body {
            MapObjectBody::Transport(transport) => Some(transport.as_mut()),
            _ => None,
        }
    }

    pub fn into_object(self) -> WorldObject {
        match self.body {
            MapObjectBody::WorldObject(object) => object,
            MapObjectBody::AreaTrigger(area_trigger) => area_trigger.world().clone(),
            MapObjectBody::Conversation(conversation) => conversation.world().clone(),
            MapObjectBody::Corpse(corpse) => corpse.world().clone(),
            MapObjectBody::Creature(creature) => creature.unit().world().clone(),
            MapObjectBody::DynamicObject(dynamic_object) => dynamic_object.world().clone(),
            MapObjectBody::GameObject(game_object) => game_object.world().clone(),
            MapObjectBody::Pet(pet) => pet.creature().unit().world().clone(),
            MapObjectBody::Player(player) => player.unit().world().clone(),
            MapObjectBody::SceneObject(scene_object) => scene_object.world().clone(),
            MapObjectBody::Transport(transport) => transport.world().clone(),
        }
    }
}

#[derive(Debug, Default)]
pub struct ObjectAccessor {
    players: HashMap<ObjectGuid, AccessorPlayer>,
    player_names: HashMap<String, ObjectGuid>,
}

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

    fn insert_player_record(&mut self, player: AccessorPlayer) {
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

    fn get_map_object_from_source<'a, Source>(
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

    fn get_map_record_from_source<'a, Source>(
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

    fn get_item_ref_for_player_context(
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectAccessorError {
    InvalidPlayerName,
    UnsupportedGuidKind {
        guid: ObjectGuid,
    },
    WrongGuidKind {
        guid: ObjectGuid,
        expected: AccessorObjectKind,
    },
    ObjectHasNoMap {
        guid: ObjectGuid,
    },
}

pub fn normalize_player_name(name: &str) -> Option<String> {
    let mut chars = name.chars();
    let first = chars.next()?;
    let mut normalized = String::new();
    normalized.extend(first.to_uppercase());
    for ch in chars {
        normalized.extend(ch.to_lowercase());
    }
    Some(normalized)
}

fn same_map(left: &WorldObject, right: &WorldObject) -> bool {
    left.has_current_map()
        && right.has_current_map()
        && left.map_id() == right.map_id()
        && left.instance_id() == right.instance_id()
}

fn kind_accepts_guid(kind: AccessorObjectKind, guid_kind: AccessorObjectKind) -> bool {
    kind == guid_kind
}

#[cfg(test)]
mod tests {
    use super::*;
    use wow_constants::{TypeId, TypeMask};
    use wow_core::Position;

    fn guid(high: HighGuid, counter: i64) -> ObjectGuid {
        if high == HighGuid::Player {
            ObjectGuid::create_global(high, 0, counter)
        } else if high == HighGuid::Transport {
            ObjectGuid::create_transport(high, counter)
        } else {
            ObjectGuid::create_world_object(high, 0, 1, 530, 1, 100, counter)
        }
    }

    fn world_object(high: HighGuid, map_id: u32, instance_id: u32, in_world: bool) -> WorldObject {
        let type_id = guid(high, 1).type_id();
        let type_mask = match type_id {
            wow_core::guid::TypeId::Player => TypeMask::PLAYER,
            wow_core::guid::TypeId::Unit => TypeMask::UNIT,
            wow_core::guid::TypeId::GameObject => TypeMask::GAME_OBJECT,
            wow_core::guid::TypeId::DynamicObject => TypeMask::DYNAMIC_OBJECT,
            wow_core::guid::TypeId::Corpse => TypeMask::CORPSE,
            wow_core::guid::TypeId::AreaTrigger => TypeMask::AREA_TRIGGER,
            wow_core::guid::TypeId::SceneObject => TypeMask::SCENE_OBJECT,
            wow_core::guid::TypeId::Conversation => TypeMask::CONVERSATION,
            _ => TypeMask::OBJECT,
        };
        let mut object = WorldObject::new(false, convert_type_id(type_id), type_mask);
        object.object_mut().create(guid(high, 1));
        object.set_map(map_id, instance_id).unwrap();
        object.relocate(Position::xyz(1.0, 2.0, 3.0));
        if in_world {
            object.object_mut().add_to_world();
        }
        object
    }

    fn item(guid: ObjectGuid, entry: u32) -> Item {
        let mut item = Item::default();
        item.object_mut().create(guid);
        item.object_mut().set_entry(entry);
        item
    }

    fn player_entity(
        counter: i64,
        map_id: u32,
        instance_id: u32,
        in_world: bool,
        attacking: Option<ObjectGuid>,
    ) -> Player {
        let mut player = Player::new(Some(counter as u64), false);
        player
            .unit_mut()
            .world_mut()
            .object_mut()
            .create(guid(HighGuid::Player, counter));
        player
            .unit_mut()
            .world_mut()
            .set_map(map_id, instance_id)
            .unwrap();
        player
            .unit_mut()
            .world_mut()
            .relocate(Position::xyz(1.0, 2.0, 3.0));
        if in_world {
            player.unit_mut().world_mut().object_mut().add_to_world();
        }
        player.unit_mut().set_attacking(attacking);
        player
    }

    fn run_player_stack_test(test: impl FnOnce() + Send + 'static) {
        std::thread::Builder::new()
            .stack_size(32 * 1024 * 1024)
            .spawn(test)
            .unwrap()
            .join()
            .unwrap();
    }

    fn convert_type_id(type_id: wow_core::guid::TypeId) -> TypeId {
        match type_id {
            wow_core::guid::TypeId::Object => TypeId::Object,
            wow_core::guid::TypeId::Item => TypeId::Item,
            wow_core::guid::TypeId::Container => TypeId::Container,
            wow_core::guid::TypeId::AzeriteEmpoweredItem => TypeId::AzeriteEmpoweredItem,
            wow_core::guid::TypeId::AzeriteItem => TypeId::AzeriteItem,
            wow_core::guid::TypeId::Unit => TypeId::Unit,
            wow_core::guid::TypeId::Player => TypeId::Player,
            wow_core::guid::TypeId::ActivePlayer => TypeId::ActivePlayer,
            wow_core::guid::TypeId::GameObject => TypeId::GameObject,
            wow_core::guid::TypeId::DynamicObject => TypeId::DynamicObject,
            wow_core::guid::TypeId::Corpse => TypeId::Corpse,
            wow_core::guid::TypeId::AreaTrigger => TypeId::AreaTrigger,
            wow_core::guid::TypeId::SceneObject => TypeId::SceneObject,
            wow_core::guid::TypeId::Conversation => TypeId::Conversation,
        }
    }

    #[test]
    fn player_name_normalization_matches_cpp_shape() {
        assert_eq!(normalize_player_name("thrall"), Some("Thrall".to_string()));
        assert_eq!(normalize_player_name("THRALL"), Some("Thrall".to_string()));
        assert_eq!(normalize_player_name(""), None);
    }

    #[test]
    fn global_player_lookup_distinguishes_connected_and_in_world() {
        let mut accessor = ObjectAccessor::default();
        let player = world_object(HighGuid::Player, 1, 0, false);
        let player_guid = player.guid();

        accessor.add_player("jaina", player).unwrap();
        assert!(accessor.find_connected_player(player_guid).is_some());
        assert!(accessor.find_connected_player_by_name("JAINA").is_some());
        assert!(accessor.find_player(player_guid).is_none());
        assert!(accessor.find_player_by_name("jaina").is_none());

        let mut in_world = world_object(HighGuid::Player, 1, 0, true);
        in_world.object_mut().create(player_guid);
        accessor.add_player("jaina", in_world).unwrap();
        assert!(accessor.find_player(player_guid).is_some());
        assert_eq!(accessor.save_all_players_count(), 1);
    }

    #[test]
    fn map_object_record_can_store_typed_gameobject_like_cpp() {
        let mut game_object = GameObject::new();
        let guid = guid(HighGuid::GameObject, 77);
        game_object.world_mut().object_mut().create(guid);
        game_object.world_mut().object_mut().set_entry(123);
        game_object.world_mut().set_map(571, 0).unwrap();
        game_object
            .world_mut()
            .relocate(Position::xyz(1.0, 2.0, 3.0));
        game_object.set_created_by(ObjectGuid::create_player(1, 42));

        let mut record = MapObjectRecord::new_game_object(game_object).unwrap();

        assert_eq!(record.kind(), AccessorObjectKind::GameObject);
        assert_eq!(record.object().guid(), guid);
        assert_eq!(
            record.game_object().unwrap().owner_guid(),
            ObjectGuid::create_player(1, 42)
        );
        record.object_mut().relocate(Position::xyz(4.0, 5.0, 6.0));
        assert_eq!(record.game_object().unwrap().world().position().x, 4.0);
    }

    #[test]
    fn map_object_record_can_store_typed_creature_like_cpp() {
        let mut creature = Creature::new(false);
        let guid = guid(HighGuid::Creature, 78);
        creature.unit_mut().world_mut().object_mut().create(guid);
        creature.unit_mut().world_mut().object_mut().set_entry(321);
        creature.unit_mut().world_mut().set_map(571, 0).unwrap();
        creature
            .unit_mut()
            .world_mut()
            .relocate(Position::xyz(1.0, 2.0, 3.0));
        creature.unit_mut().set_level(42);

        let mut record = MapObjectRecord::new_creature(creature).unwrap();

        assert_eq!(record.kind(), AccessorObjectKind::Creature);
        assert_eq!(record.object().guid(), guid);
        assert_eq!(record.creature().unwrap().unit().data().level, 42);
        record
            .creature_mut()
            .unwrap()
            .unit_mut()
            .world_mut()
            .relocate(Position::xyz(4.0, 5.0, 6.0));
        assert_eq!(record.object().position().x, 4.0);
    }

    #[test]
    fn map_object_record_can_store_typed_player_like_cpp() {
        let mut player = Player::new(Some(7), false);
        let player_guid = ObjectGuid::create_player(1, 42);
        let victim_guid = guid(HighGuid::Creature, 77);
        player
            .unit_mut()
            .world_mut()
            .object_mut()
            .create(player_guid);
        player.unit_mut().world_mut().set_map(571, 7).unwrap();
        player
            .unit_mut()
            .world_mut()
            .relocate(Position::xyz(1.0, 2.0, 3.0));
        player.unit_mut().set_attacking(Some(victim_guid));

        let mut record = MapObjectRecord::new_player(player).unwrap();

        assert_eq!(record.kind(), AccessorObjectKind::Player);
        assert_eq!(record.object().guid(), player_guid);
        assert_eq!(
            record.player().unwrap().unit().attacking(),
            Some(victim_guid)
        );
        record.player_mut().unwrap().unit_mut().set_attacking(None);
        assert_eq!(record.player().unwrap().unit().attacking(), None);
    }

    #[test]
    fn typed_global_player_lookup_preserves_player_body_like_cpp_hashmap_holder() {
        run_player_stack_test(|| {
            let mut accessor = ObjectAccessor::default();
            let context = world_object(HighGuid::Player, 530, 1, true);
            let target = guid(HighGuid::Creature, 4_201);
            let player = player_entity(4_200, 530, 1, true, Some(target));
            let player_guid = player.unit().world().guid();
            let source = TestMapSource {
                map_id: 530,
                instance_id: 1,
                records: std::collections::HashMap::new(),
            };

            accessor.add_player_entity("anduin", player).unwrap();

            assert_eq!(
                accessor
                    .find_connected_player_entity(player_guid)
                    .unwrap()
                    .unit()
                    .attacking(),
                Some(target)
            );
            assert_eq!(
                accessor
                    .find_player_entity(player_guid)
                    .unwrap()
                    .unit()
                    .attacking(),
                Some(target)
            );
            assert_eq!(
                accessor
                    .get_player_entity(&context, player_guid)
                    .unwrap()
                    .unit()
                    .attacking(),
                Some(target)
            );
            assert_eq!(
                accessor
                    .get_typed_player_from_map_source(&context, &source, player_guid)
                    .unwrap()
                    .unit()
                    .attacking(),
                Some(target)
            );
        });
    }

    #[test]
    fn typed_global_player_lookup_rejects_legacy_and_cpp_early_returns() {
        run_player_stack_test(|| {
            let mut accessor = ObjectAccessor::default();
            let context = world_object(HighGuid::Player, 530, 1, true);
            let legacy = world_object(HighGuid::Player, 530, 1, true);
            let legacy_guid = legacy.guid();
            let not_in_world = player_entity(4_210, 530, 1, false, None);
            let not_in_world_guid = not_in_world.unit().world().guid();
            let other_map = player_entity(4_211, 571, 1, true, None);
            let other_map_guid = other_map.unit().world().guid();
            let other_instance = player_entity(4_212, 530, 2, true, None);
            let other_instance_guid = other_instance.unit().world().guid();

            accessor.add_player("legacy", legacy).unwrap();
            accessor.add_player_entity("ghost", not_in_world).unwrap();
            accessor.add_player_entity("map", other_map).unwrap();
            accessor
                .add_player_entity("instance", other_instance)
                .unwrap();

            assert!(accessor.get_player(&context, legacy_guid).is_some());
            assert!(accessor.get_player_entity(&context, legacy_guid).is_none());
            assert!(
                accessor
                    .find_connected_player_entity(not_in_world_guid)
                    .is_some()
            );
            assert!(accessor.find_player_entity(not_in_world_guid).is_none());
            assert!(
                accessor
                    .get_player_entity(&context, not_in_world_guid)
                    .is_none()
            );
            assert!(
                accessor
                    .get_player_entity(&context, other_map_guid)
                    .is_none()
            );
            assert!(
                accessor
                    .get_player_entity(&context, other_instance_guid)
                    .is_none()
            );
            assert!(
                accessor
                    .get_player_entity(&context, guid(HighGuid::Creature, 4_213))
                    .is_none()
            );
        });
    }

    #[test]
    fn typed_player_map_source_ignores_map_object_record_player_like_cpp_hashmap_holder() {
        run_player_stack_test(|| {
            let mut accessor = ObjectAccessor::default();
            let context = world_object(HighGuid::Player, 530, 1, true);
            let source_only_player = player_entity(4_220, 530, 1, true, None);
            let source_only_guid = source_only_player.unit().world().guid();
            let global_player = player_entity(4_221, 530, 1, true, None);
            let global_guid = global_player.unit().world().guid();
            let mut source = TestMapSource {
                map_id: 530,
                instance_id: 1,
                records: std::collections::HashMap::new(),
            };
            source.records.insert(
                source_only_guid,
                MapObjectRecord::new_player(source_only_player).unwrap(),
            );

            assert!(
                accessor
                    .get_typed_player_from_map_source(&context, &source, source_only_guid)
                    .is_none()
            );

            accessor
                .add_player_entity("tyrande", global_player)
                .unwrap();
            assert!(
                accessor
                    .get_typed_player_from_map_source(&context, &source, global_guid)
                    .is_some()
            );
            source.map_id = 571;
            assert!(
                accessor
                    .get_typed_player_from_map_source(&context, &source, global_guid)
                    .is_none()
            );
            source.map_id = 530;
            source.instance_id = 2;
            assert!(
                accessor
                    .get_typed_player_from_map_source(&context, &source, global_guid)
                    .is_none()
            );
        });
    }

    #[test]
    fn player_worldobject_dispatch_and_type_mask_stay_on_global_registry_like_cpp() {
        run_player_stack_test(|| {
            let mut accessor = ObjectAccessor::default();
            let context = world_object(HighGuid::Player, 530, 1, true);
            let player = player_entity(4_230, 530, 1, true, None);
            let player_guid = player.unit().world().guid();
            let source = TestMapSource {
                map_id: 530,
                instance_id: 1,
                records: std::collections::HashMap::new(),
            };

            accessor.add_player_entity("uther", player).unwrap();

            assert_eq!(
                accessor
                    .get_world_object_from_map_source(&context, &source, player_guid)
                    .unwrap()
                    .guid(),
                player_guid
            );
            assert!(matches!(
                accessor.get_object_ref_by_type_mask_from_map_source(
                    &context,
                    &source,
                    player_guid,
                    TypeMask::PLAYER
                ),
                Some(AccessorObjectRef::WorldObject(object)) if object.guid() == player_guid
            ));
            assert_eq!(
                accessor
                    .get_unit_from_map_source(&context, &source, player_guid)
                    .unwrap()
                    .guid(),
                player_guid
            );
        });
    }

    #[test]
    fn typed_player_registration_validates_player_high_guid() {
        run_player_stack_test(|| {
            let mut player = player_entity(4_240, 530, 1, true, None);
            let creature_guid = guid(HighGuid::Creature, 4_241);
            player
                .unit_mut()
                .world_mut()
                .object_mut()
                .create(creature_guid);

            let error = AccessorPlayer::new_player("bad", player).unwrap_err();
            assert_eq!(
                error,
                ObjectAccessorError::WrongGuidKind {
                    guid: creature_guid,
                    expected: AccessorObjectKind::Player,
                }
            );
        });
    }

    #[test]
    fn save_all_players_with_invokes_sink_once_per_registered_player() {
        let mut accessor = ObjectAccessor::default();
        let player_a = world_object(HighGuid::Player, 1, 0, true);
        let guid_a = player_a.guid();
        let mut player_b = world_object(HighGuid::Player, 1, 0, true);
        player_b
            .object_mut()
            .create(ObjectGuid::create_global(HighGuid::Player, 0, 2));
        let guid_b = player_b.guid();

        accessor.add_player("jaina", player_a).unwrap();
        accessor.add_player("thrall", player_b).unwrap();

        let mut saved = Vec::new();
        let count = accessor
            .save_all_players_with(|player| {
                saved.push(player.object().guid());
                Ok::<(), ()>(())
            })
            .unwrap();

        assert_eq!(count, 2);
        assert!(saved.contains(&guid_a));
        assert!(saved.contains(&guid_b));
        assert_eq!(saved.len(), 2);
    }

    #[test]
    fn save_all_players_with_propagates_error_with_player_guid() {
        let mut accessor = ObjectAccessor::default();
        let player = world_object(HighGuid::Player, 1, 0, true);
        let guid = player.guid();
        accessor.add_player("jaina", player).unwrap();

        let error = accessor
            .save_all_players_with(|_| Err::<(), _>("db unavailable"))
            .unwrap_err();

        assert_eq!(error.guid, guid);
        assert_eq!(error.source, "db unavailable");
    }

    #[test]
    fn save_all_players_with_does_not_break_name_or_canonical_map_source_lookup() {
        let mut accessor = ObjectAccessor::default();
        let context = world_object(HighGuid::Player, 530, 1, true);
        let player_guid = context.guid();
        let creature = world_object(HighGuid::Creature, 530, 1, true);
        let creature_guid = creature.guid();
        let record = MapObjectRecord::new(AccessorObjectKind::Creature, creature).unwrap();
        let mut source = TestMapSource {
            map_id: 530,
            instance_id: 1,
            records: std::collections::HashMap::new(),
        };
        source.records.insert(creature_guid, record);

        accessor.add_player("valeera", context.clone()).unwrap();

        let saved = accessor
            .save_all_players_with(|_| Ok::<(), ()>(()))
            .unwrap();

        assert_eq!(saved, 1);
        assert_eq!(
            accessor
                .find_connected_player_by_name("VALEERA")
                .unwrap()
                .guid(),
            player_guid
        );
        assert_eq!(
            accessor
                .get_creature_from_map_source(&context, &source, creature_guid)
                .unwrap()
                .guid(),
            creature_guid
        );
    }

    #[derive(Default)]
    struct TestMapSource {
        map_id: u32,
        instance_id: u32,
        records: std::collections::HashMap<ObjectGuid, MapObjectRecord>,
    }

    impl ObjectAccessorMapSource for TestMapSource {
        fn map_id(&self) -> u32 {
            self.map_id
        }

        fn instance_id(&self) -> u32 {
            self.instance_id
        }

        fn map_object_record(&self, guid: ObjectGuid) -> Option<&MapObjectRecord> {
            self.records.get(&guid)
        }
    }

    fn typed_creature_record(
        map_id: u32,
        instance_id: u32,
        counter: i64,
        level: u8,
    ) -> (ObjectGuid, MapObjectRecord) {
        let mut creature = Creature::new(false);
        let creature_guid = guid(HighGuid::Creature, counter);
        creature
            .unit_mut()
            .world_mut()
            .object_mut()
            .create(creature_guid);
        creature
            .unit_mut()
            .world_mut()
            .set_map(map_id, instance_id)
            .unwrap();
        creature
            .unit_mut()
            .world_mut()
            .relocate(Position::xyz(10.0, 20.0, 30.0));
        creature.unit_mut().set_level(level);

        (
            creature_guid,
            MapObjectRecord::new_creature(creature).unwrap(),
        )
    }

    fn typed_game_object_record(
        map_id: u32,
        instance_id: u32,
        counter: i64,
        linked_trap: ObjectGuid,
    ) -> (ObjectGuid, MapObjectRecord) {
        let mut game_object = GameObject::new();
        let game_object_guid = guid(HighGuid::GameObject, counter);
        game_object
            .world_mut()
            .object_mut()
            .create(game_object_guid);
        game_object
            .world_mut()
            .set_map(map_id, instance_id)
            .unwrap();
        game_object
            .world_mut()
            .relocate(Position::xyz(10.0, 20.0, 30.0));
        game_object.set_linked_trap_like_cpp(linked_trap);

        (
            game_object_guid,
            MapObjectRecord::new_game_object(game_object).unwrap(),
        )
    }

    #[test]
    fn typed_map_source_lookup_preserves_creature_and_gameobject_bodies_like_cpp() {
        let accessor = ObjectAccessor::default();
        let context = world_object(HighGuid::Player, 530, 1, true);
        let linked_trap = guid(HighGuid::GameObject, 4101);
        let (creature_guid, creature_record) = typed_creature_record(530, 1, 4102, 61);
        let (game_object_guid, game_object_record) =
            typed_game_object_record(530, 1, 4103, linked_trap);
        let mut source = TestMapSource {
            map_id: 530,
            instance_id: 1,
            records: std::collections::HashMap::new(),
        };
        source.records.insert(creature_guid, creature_record);
        source.records.insert(game_object_guid, game_object_record);

        let creature = accessor
            .get_typed_creature_from_map_source(&context, &source, creature_guid)
            .unwrap();
        assert_eq!(creature.level(), 61);
        assert_eq!(creature.unit().world().guid(), creature_guid);

        let game_object = accessor
            .get_typed_game_object_from_map_source(&context, &source, game_object_guid)
            .unwrap();
        assert_eq!(game_object.linked_trap_guid_like_cpp(), linked_trap);
        assert_eq!(game_object.world().guid(), game_object_guid);
    }

    #[test]
    fn typed_map_source_lookup_rejects_generic_worldobject_fallback_like_cpp() {
        let accessor = ObjectAccessor::default();
        let context = world_object(HighGuid::Player, 530, 1, true);
        let generic_creature = world_object(HighGuid::Creature, 530, 1, true);
        let generic_creature_guid = generic_creature.guid();
        let generic_game_object = world_object(HighGuid::GameObject, 530, 1, true);
        let generic_game_object_guid = generic_game_object.guid();
        let mut source = TestMapSource {
            map_id: 530,
            instance_id: 1,
            records: std::collections::HashMap::new(),
        };
        source.records.insert(
            generic_creature_guid,
            MapObjectRecord::new(AccessorObjectKind::Creature, generic_creature).unwrap(),
        );
        source.records.insert(
            generic_game_object_guid,
            MapObjectRecord::new(AccessorObjectKind::GameObject, generic_game_object).unwrap(),
        );

        assert_eq!(
            accessor
                .get_creature_from_map_source(&context, &source, generic_creature_guid)
                .unwrap()
                .guid(),
            generic_creature_guid
        );
        assert_eq!(
            accessor
                .get_game_object_from_map_source(&context, &source, generic_game_object_guid)
                .unwrap()
                .guid(),
            generic_game_object_guid
        );
        assert!(
            accessor
                .get_typed_creature_from_map_source(&context, &source, generic_creature_guid)
                .is_none()
        );
        assert!(
            accessor
                .get_typed_game_object_from_map_source(&context, &source, generic_game_object_guid)
                .is_none()
        );
    }

    #[test]
    fn typed_map_source_lookup_requires_source_and_context_same_map_like_cpp() {
        let accessor = ObjectAccessor::default();
        let context = world_object(HighGuid::Player, 530, 1, true);
        let (creature_guid, creature_record) = typed_creature_record(530, 1, 4104, 62);
        let mut source = TestMapSource {
            map_id: 530,
            instance_id: 1,
            records: std::collections::HashMap::new(),
        };
        source.records.insert(creature_guid, creature_record);

        source.map_id = 571;
        assert!(
            accessor
                .get_typed_creature_from_map_source(&context, &source, creature_guid)
                .is_none()
        );
        source.map_id = 530;
        source.instance_id = 2;
        assert!(
            accessor
                .get_typed_creature_from_map_source(&context, &source, creature_guid)
                .is_none()
        );
        source.instance_id = 1;

        let wrong_context = world_object(HighGuid::Player, 571, 1, true);
        assert!(
            accessor
                .get_typed_creature_from_map_source(&wrong_context, &source, creature_guid)
                .is_none()
        );
    }

    #[test]
    fn typed_map_source_lookup_does_not_cross_creature_and_gameobject_kinds_like_cpp() {
        let accessor = ObjectAccessor::default();
        let context = world_object(HighGuid::Player, 530, 1, true);
        let (creature_guid, creature_record) = typed_creature_record(530, 1, 4105, 63);
        let (game_object_guid, game_object_record) =
            typed_game_object_record(530, 1, 4106, guid(HighGuid::GameObject, 4107));
        let mut source = TestMapSource {
            map_id: 530,
            instance_id: 1,
            records: std::collections::HashMap::new(),
        };
        source.records.insert(creature_guid, creature_record);
        source.records.insert(game_object_guid, game_object_record);

        assert!(
            accessor
                .get_typed_creature_from_map_source(&context, &source, game_object_guid)
                .is_none()
        );
        assert!(
            accessor
                .get_typed_game_object_from_map_source(&context, &source, creature_guid)
                .is_none()
        );
    }

    fn typed_dynamic_object_record(
        map_id: u32,
        instance_id: u32,
        counter: i64,
        caster: ObjectGuid,
        spell_id: i32,
    ) -> (ObjectGuid, MapObjectRecord) {
        let mut dynamic_object = DynamicObject::new(false);
        let dynamic_object_guid = guid(HighGuid::DynamicObject, counter);
        dynamic_object
            .world_mut()
            .object_mut()
            .create(dynamic_object_guid);
        dynamic_object
            .world_mut()
            .set_map(map_id, instance_id)
            .unwrap();
        dynamic_object
            .world_mut()
            .relocate(Position::xyz(10.0, 20.0, 30.0));
        dynamic_object.set_caster_guid(caster);
        dynamic_object.set_spell_id(spell_id);
        dynamic_object.set_radius(12.5);

        (
            dynamic_object_guid,
            MapObjectRecord::new_dynamic_object(dynamic_object).unwrap(),
        )
    }

    fn typed_area_trigger_record(
        map_id: u32,
        instance_id: u32,
        counter: i64,
        caster: ObjectGuid,
        spell_id: i32,
    ) -> (ObjectGuid, MapObjectRecord) {
        let mut area_trigger = AreaTrigger::new();
        let area_trigger_guid = guid(HighGuid::AreaTrigger, counter);
        area_trigger
            .world_mut()
            .object_mut()
            .create(area_trigger_guid);
        area_trigger
            .world_mut()
            .set_map(map_id, instance_id)
            .unwrap();
        area_trigger
            .world_mut()
            .relocate(Position::xyz(10.0, 20.0, 30.0));
        area_trigger.set_caster_guid(caster);
        area_trigger.set_spell_id(spell_id);
        area_trigger.set_duration(4_500);

        (
            area_trigger_guid,
            MapObjectRecord::new_area_trigger(area_trigger).unwrap(),
        )
    }

    fn typed_corpse_record(
        map_id: u32,
        instance_id: u32,
        counter: i64,
        owner: ObjectGuid,
        ghost_time: i64,
    ) -> (ObjectGuid, MapObjectRecord) {
        let mut corpse = Corpse::new_at(crate::CorpseType::ResurrectablePve, ghost_time);
        let corpse_guid = guid(HighGuid::Corpse, counter);
        corpse.world_mut().object_mut().create(corpse_guid);
        corpse.world_mut().set_map(map_id, instance_id).unwrap();
        corpse.world_mut().relocate(Position::xyz(10.0, 20.0, 30.0));
        corpse.set_owner_guid(owner);
        corpse.set_display_id(11_111);

        (corpse_guid, MapObjectRecord::new_corpse(corpse).unwrap())
    }

    fn typed_scene_object_record(
        map_id: u32,
        instance_id: u32,
        counter: i64,
        creator: ObjectGuid,
        script_package_id: i32,
    ) -> (ObjectGuid, MapObjectRecord) {
        let mut scene_object = SceneObject::new();
        let scene_object_guid = guid(HighGuid::SceneObject, counter);
        scene_object
            .world_mut()
            .object_mut()
            .create(scene_object_guid);
        scene_object
            .world_mut()
            .set_map(map_id, instance_id)
            .unwrap();
        scene_object
            .world_mut()
            .relocate(Position::xyz(10.0, 20.0, 30.0));
        scene_object.set_created_by(creator);
        scene_object.set_script_package_id(script_package_id);
        scene_object.set_scene_type(crate::SceneType::PetBattle);

        (
            scene_object_guid,
            MapObjectRecord::new_scene_object(scene_object).unwrap(),
        )
    }

    fn typed_conversation_record(
        map_id: u32,
        instance_id: u32,
        counter: i64,
        creator: ObjectGuid,
        duration_ms: i32,
    ) -> (ObjectGuid, MapObjectRecord) {
        let mut conversation = Conversation::new();
        let conversation_guid = guid(HighGuid::Conversation, counter);
        conversation
            .world_mut()
            .object_mut()
            .create(conversation_guid);
        conversation
            .world_mut()
            .set_map(map_id, instance_id)
            .unwrap();
        conversation
            .world_mut()
            .relocate(Position::xyz(10.0, 20.0, 30.0));
        conversation.set_creator_guid(creator);
        conversation.set_duration_ms(duration_ms);
        conversation.set_texture_kit_id(77);
        conversation.add_line(crate::ConversationLine {
            conversation_line_id: 9901,
            start_time: 12,
            ui_camera_id: 34,
            actor_index: 1,
            flags: 2,
        });

        (
            conversation_guid,
            MapObjectRecord::new_conversation(conversation).unwrap(),
        )
    }

    #[test]
    fn typed_map_source_lookup_preserves_non_creature_gameobject_bodies_like_cpp() {
        let accessor = ObjectAccessor::default();
        let context = world_object(HighGuid::Player, 530, 1, true);
        let caster = guid(HighGuid::Creature, 4110);
        let creator = guid(HighGuid::Player, 4111);
        let (dynamic_guid, dynamic_record) =
            typed_dynamic_object_record(530, 1, 4112, caster, 12345);
        let (area_trigger_guid, area_trigger_record) =
            typed_area_trigger_record(530, 1, 4113, caster, 12346);
        let (corpse_guid, corpse_record) = typed_corpse_record(530, 1, 4114, creator, 98765);
        let (scene_guid, scene_record) = typed_scene_object_record(530, 1, 4115, creator, 4567);
        let (conversation_guid, conversation_record) =
            typed_conversation_record(530, 1, 4116, creator, 8901);
        let mut source = TestMapSource {
            map_id: 530,
            instance_id: 1,
            records: std::collections::HashMap::new(),
        };
        source.records.insert(dynamic_guid, dynamic_record);
        source
            .records
            .insert(area_trigger_guid, area_trigger_record);
        source.records.insert(corpse_guid, corpse_record);
        source.records.insert(scene_guid, scene_record);
        source
            .records
            .insert(conversation_guid, conversation_record);

        let dynamic_object = accessor
            .get_typed_dynamic_object_from_map_source(&context, &source, dynamic_guid)
            .unwrap();
        assert_eq!(dynamic_object.caster_guid(), caster);
        assert_eq!(dynamic_object.spell_id(), 12345);
        assert_eq!(dynamic_object.radius(), 12.5);
        assert_eq!(dynamic_object.world().guid(), dynamic_guid);

        let area_trigger = accessor
            .get_typed_area_trigger_from_map_source(&context, &source, area_trigger_guid)
            .unwrap();
        assert_eq!(area_trigger.caster_guid(), caster);
        assert_eq!(area_trigger.spell_id(), 12346);
        assert_eq!(area_trigger.duration_ms(), 4_500);
        assert_eq!(area_trigger.world().guid(), area_trigger_guid);

        let corpse = accessor
            .get_typed_corpse_from_map_source(&context, &source, corpse_guid)
            .unwrap();
        assert_eq!(corpse.corpse_type(), crate::CorpseType::ResurrectablePve);
        assert_eq!(corpse.ghost_time(), 98765);
        assert_eq!(corpse.data().owner, creator);
        assert_eq!(corpse.data().display_id, 11_111);
        assert_eq!(corpse.world().guid(), corpse_guid);

        let scene_object = accessor
            .get_typed_scene_object_from_map_source(&context, &source, scene_guid)
            .unwrap();
        assert_eq!(scene_object.creator_guid(), creator);
        assert_eq!(scene_object.data().script_package_id, 4567);
        assert_eq!(
            scene_object.data().scene_type,
            crate::SceneType::PetBattle as u32
        );
        assert_eq!(scene_object.world().guid(), scene_guid);

        let conversation = accessor
            .get_typed_conversation_from_map_source(&context, &source, conversation_guid)
            .unwrap();
        assert_eq!(conversation.creator_guid(), creator);
        assert_eq!(conversation.duration_ms(), 8901);
        assert_eq!(conversation.texture_kit_id(), 77);
        assert_eq!(conversation.data().lines[0].conversation_line_id, 9901);
        assert_eq!(conversation.world().guid(), conversation_guid);
    }

    #[test]
    fn typed_map_source_lookup_rejects_generic_non_creature_gameobject_fallbacks_like_cpp() {
        let accessor = ObjectAccessor::default();
        let context = world_object(HighGuid::Player, 530, 1, true);
        let generic_dynamic = world_object(HighGuid::DynamicObject, 530, 1, true);
        let generic_dynamic_guid = generic_dynamic.guid();
        let generic_area_trigger = world_object(HighGuid::AreaTrigger, 530, 1, true);
        let generic_area_trigger_guid = generic_area_trigger.guid();
        let generic_corpse = world_object(HighGuid::Corpse, 530, 1, true);
        let generic_corpse_guid = generic_corpse.guid();
        let generic_scene = world_object(HighGuid::SceneObject, 530, 1, true);
        let generic_scene_guid = generic_scene.guid();
        let generic_conversation = world_object(HighGuid::Conversation, 530, 1, true);
        let generic_conversation_guid = generic_conversation.guid();
        let mut source = TestMapSource {
            map_id: 530,
            instance_id: 1,
            records: std::collections::HashMap::new(),
        };
        source.records.insert(
            generic_dynamic_guid,
            MapObjectRecord::new(AccessorObjectKind::DynamicObject, generic_dynamic).unwrap(),
        );
        source.records.insert(
            generic_area_trigger_guid,
            MapObjectRecord::new(AccessorObjectKind::AreaTrigger, generic_area_trigger).unwrap(),
        );
        source.records.insert(
            generic_corpse_guid,
            MapObjectRecord::new(AccessorObjectKind::Corpse, generic_corpse).unwrap(),
        );
        source.records.insert(
            generic_scene_guid,
            MapObjectRecord::new(AccessorObjectKind::SceneObject, generic_scene).unwrap(),
        );
        source.records.insert(
            generic_conversation_guid,
            MapObjectRecord::new(AccessorObjectKind::Conversation, generic_conversation).unwrap(),
        );

        assert_eq!(
            accessor
                .get_dynamic_object_from_map_source(&context, &source, generic_dynamic_guid)
                .unwrap()
                .guid(),
            generic_dynamic_guid
        );
        assert_eq!(
            accessor
                .get_area_trigger_from_map_source(&context, &source, generic_area_trigger_guid)
                .unwrap()
                .guid(),
            generic_area_trigger_guid
        );
        assert_eq!(
            accessor
                .get_corpse_from_map_source(&context, &source, generic_corpse_guid)
                .unwrap()
                .guid(),
            generic_corpse_guid
        );
        assert_eq!(
            accessor
                .get_scene_object_from_map_source(&context, &source, generic_scene_guid)
                .unwrap()
                .guid(),
            generic_scene_guid
        );
        assert_eq!(
            accessor
                .get_conversation_from_map_source(&context, &source, generic_conversation_guid)
                .unwrap()
                .guid(),
            generic_conversation_guid
        );

        assert!(
            accessor
                .get_typed_dynamic_object_from_map_source(&context, &source, generic_dynamic_guid)
                .is_none()
        );
        assert!(
            accessor
                .get_typed_area_trigger_from_map_source(
                    &context,
                    &source,
                    generic_area_trigger_guid
                )
                .is_none()
        );
        assert!(
            accessor
                .get_typed_corpse_from_map_source(&context, &source, generic_corpse_guid)
                .is_none()
        );
        assert!(
            accessor
                .get_typed_scene_object_from_map_source(&context, &source, generic_scene_guid)
                .is_none()
        );
        assert!(
            accessor
                .get_typed_conversation_from_map_source(
                    &context,
                    &source,
                    generic_conversation_guid
                )
                .is_none()
        );
    }

    #[test]
    fn typed_map_source_lookup_does_not_cross_non_creature_gameobject_kinds_like_cpp() {
        let accessor = ObjectAccessor::default();
        let context = world_object(HighGuid::Player, 530, 1, true);
        let caster = guid(HighGuid::Creature, 4120);
        let creator = guid(HighGuid::Player, 4121);
        let (dynamic_guid, dynamic_record) = typed_dynamic_object_record(530, 1, 4122, caster, 1);
        let (area_trigger_guid, area_trigger_record) =
            typed_area_trigger_record(530, 1, 4123, caster, 2);
        let (corpse_guid, corpse_record) = typed_corpse_record(530, 1, 4124, creator, 3);
        let (scene_guid, scene_record) = typed_scene_object_record(530, 1, 4125, creator, 4);
        let (conversation_guid, conversation_record) =
            typed_conversation_record(530, 1, 4126, creator, 5);
        let mut source = TestMapSource {
            map_id: 530,
            instance_id: 1,
            records: std::collections::HashMap::new(),
        };
        source.records.insert(dynamic_guid, dynamic_record);
        source
            .records
            .insert(area_trigger_guid, area_trigger_record);
        source.records.insert(corpse_guid, corpse_record);
        source.records.insert(scene_guid, scene_record);
        source
            .records
            .insert(conversation_guid, conversation_record);

        assert!(
            accessor
                .get_typed_dynamic_object_from_map_source(&context, &source, area_trigger_guid)
                .is_none()
        );
        assert!(
            accessor
                .get_typed_area_trigger_from_map_source(&context, &source, dynamic_guid)
                .is_none()
        );
        assert!(
            accessor
                .get_typed_corpse_from_map_source(&context, &source, scene_guid)
                .is_none()
        );
        assert!(
            accessor
                .get_typed_scene_object_from_map_source(&context, &source, corpse_guid)
                .is_none()
        );
        assert!(
            accessor
                .get_typed_conversation_from_map_source(&context, &source, dynamic_guid)
                .is_none()
        );
        assert!(
            accessor
                .get_typed_dynamic_object_from_map_source(&context, &source, conversation_guid)
                .is_none()
        );
    }

    #[test]
    fn typed_map_source_lookup_requires_source_and_context_same_map_for_non_creature_gameobject_like_cpp()
     {
        let accessor = ObjectAccessor::default();
        let context = world_object(HighGuid::Player, 530, 1, true);
        let (dynamic_guid, dynamic_record) =
            typed_dynamic_object_record(530, 1, 4130, guid(HighGuid::Creature, 4131), 7);
        let mut source = TestMapSource {
            map_id: 530,
            instance_id: 1,
            records: std::collections::HashMap::new(),
        };
        source.records.insert(dynamic_guid, dynamic_record);

        source.map_id = 571;
        assert!(
            accessor
                .get_typed_dynamic_object_from_map_source(&context, &source, dynamic_guid)
                .is_none()
        );
        source.map_id = 530;
        source.instance_id = 2;
        assert!(
            accessor
                .get_typed_dynamic_object_from_map_source(&context, &source, dynamic_guid)
                .is_none()
        );
        source.instance_id = 1;

        let wrong_context = world_object(HighGuid::Player, 571, 1, true);
        assert!(
            accessor
                .get_typed_dynamic_object_from_map_source(&wrong_context, &source, dynamic_guid)
                .is_none()
        );
    }

    #[test]
    fn map_source_lookup_reads_canonical_source_without_bridge_storage() {
        let accessor = ObjectAccessor::default();
        let context = world_object(HighGuid::Player, 530, 1, true);
        let creature = world_object(HighGuid::Creature, 530, 1, true);
        let creature_guid = creature.guid();
        let record = MapObjectRecord::new(AccessorObjectKind::Creature, creature).unwrap();
        let mut source = TestMapSource {
            map_id: 530,
            instance_id: 1,
            records: std::collections::HashMap::new(),
        };
        source.records.insert(creature_guid, record);

        assert_eq!(
            accessor
                .get_world_object_from_map_source(&context, &source, creature_guid)
                .unwrap()
                .guid(),
            creature_guid
        );
        assert!(matches!(
            accessor.get_object_ref_by_type_mask_from_map_source(
                &context,
                &source,
                creature_guid,
                TypeMask::UNIT
            ),
            Some(AccessorObjectRef::WorldObject(object)) if object.guid() == creature_guid
        ));

        source.instance_id = 2;
        assert!(
            accessor
                .get_world_object_from_map_source(&context, &source, creature_guid)
                .is_none()
        );
    }

    #[test]
    fn get_player_requires_same_map_like_cpp_get_player_map() {
        let mut accessor = ObjectAccessor::default();
        let context = world_object(HighGuid::Creature, 1, 0, true);
        let same_map_player = world_object(HighGuid::Player, 1, 0, true);
        let same_guid = same_map_player.guid();
        let mut other_map_player = world_object(HighGuid::Player, 2, 0, true);
        other_map_player
            .object_mut()
            .create(ObjectGuid::create_global(HighGuid::Player, 0, 2));
        let other_guid = other_map_player.guid();

        accessor.add_player("anduin", same_map_player).unwrap();
        accessor.add_player("baine", other_map_player).unwrap();

        assert!(accessor.get_player(&context, same_guid).is_some());
        assert!(accessor.get_player(&context, other_guid).is_none());
    }

    #[test]
    fn world_object_dispatches_by_high_guid_to_map_source() {
        let accessor = ObjectAccessor::default();
        let context = world_object(HighGuid::Player, 530, 1, true);
        let creature = world_object(HighGuid::Creature, 530, 1, true);
        let gameobject = world_object(HighGuid::GameObject, 530, 1, true);
        let creature_guid = creature.guid();
        let gameobject_guid = gameobject.guid();
        let mut source = TestMapSource {
            map_id: 530,
            instance_id: 1,
            records: std::collections::HashMap::new(),
        };

        source.records.insert(
            creature_guid,
            MapObjectRecord::new(AccessorObjectKind::Creature, creature).unwrap(),
        );
        source.records.insert(
            gameobject_guid,
            MapObjectRecord::new(AccessorObjectKind::GameObject, gameobject).unwrap(),
        );

        assert_eq!(
            accessor
                .get_world_object_from_map_source(&context, &source, creature_guid)
                .unwrap()
                .guid(),
            creature_guid
        );
        assert_eq!(
            accessor
                .get_world_object_from_map_source(&context, &source, gameobject_guid)
                .unwrap()
                .guid(),
            gameobject_guid
        );
    }

    #[test]
    fn object_by_type_mask_matches_cpp_dispatch_rules_with_map_source() {
        let accessor = ObjectAccessor::default();
        let context = world_object(HighGuid::Player, 530, 1, true);
        let creature = world_object(HighGuid::Creature, 530, 1, true);
        let creature_guid = creature.guid();
        let mut source = TestMapSource {
            map_id: 530,
            instance_id: 1,
            records: std::collections::HashMap::new(),
        };
        source.records.insert(
            creature_guid,
            MapObjectRecord::new(AccessorObjectKind::Creature, creature).unwrap(),
        );

        assert!(
            accessor
                .get_object_ref_by_type_mask_from_map_source(
                    &context,
                    &source,
                    creature_guid,
                    TypeMask::UNIT
                )
                .is_some()
        );
        assert!(
            accessor
                .get_object_ref_by_type_mask_from_map_source(
                    &context,
                    &source,
                    creature_guid,
                    TypeMask::GAME_OBJECT
                )
                .is_none()
        );
        assert!(
            accessor
                .get_object_ref_by_type_mask_from_map_source(
                    &context,
                    &source,
                    creature_guid,
                    TypeMask::PLAYER
                )
                .is_none()
        );
    }

    #[test]
    fn type_mask_item_uses_player_inventory_like_cpp_branch() {
        let mut accessor = ObjectAccessor::default();
        let context = world_object(HighGuid::Player, 530, 1, true);
        let player_guid = context.guid();
        let item_guid = ObjectGuid::create_item(1, 77);
        let mut inventory = PlayerInventoryStorage::default();
        inventory.items[0] = Some(item_guid);
        let item = item(item_guid, 6948);

        accessor
            .add_player_with_inventory_and_items("valeera", context.clone(), inventory, [item])
            .unwrap();

        let found = accessor.get_object_ref_by_type_mask(&context, item_guid, TypeMask::ITEM);
        match found {
            Some(AccessorObjectRef::Item(item)) => {
                assert_eq!(item.object().guid(), item_guid);
                assert_eq!(item.object().entry(), 6948);
            }
            other => panic!("expected item ref, got {other:?}"),
        }
        assert!(
            accessor
                .get_object_by_type_mask(&context, item_guid, TypeMask::ITEM)
                .is_none()
        );
        assert!(
            accessor
                .get_object_ref_by_type_mask(&context, item_guid, TypeMask::UNIT)
                .is_none()
        );

        let non_player_context = world_object(HighGuid::Creature, 530, 1, true);
        assert!(
            accessor
                .get_object_ref_by_type_mask(&non_player_context, item_guid, TypeMask::ITEM)
                .is_none()
        );
        assert!(accessor.player_inventory_mut(player_guid).is_some());
    }

    #[test]
    fn type_mask_item_requires_registered_item_object_like_cpp_item_pointer() {
        let mut accessor = ObjectAccessor::default();
        let context = world_object(HighGuid::Player, 530, 1, true);
        let item_guid = ObjectGuid::create_item(1, 77);
        let mut inventory = PlayerInventoryStorage::default();
        inventory.items[0] = Some(item_guid);

        accessor
            .add_player_with_inventory("valeera", context.clone(), inventory)
            .unwrap();

        assert!(
            accessor
                .get_object_ref_by_type_mask(&context, item_guid, TypeMask::ITEM)
                .is_none()
        );
    }

    #[test]
    fn corpse_is_directly_accessible_but_not_returned_by_type_mask_like_cpp() {
        let accessor = ObjectAccessor::default();
        let context = world_object(HighGuid::Player, 530, 1, true);
        let corpse = world_object(HighGuid::Corpse, 530, 1, true);
        let corpse_guid = corpse.guid();
        let mut source = TestMapSource {
            map_id: 530,
            instance_id: 1,
            records: std::collections::HashMap::new(),
        };

        source.records.insert(
            corpse_guid,
            MapObjectRecord::new(AccessorObjectKind::Corpse, corpse).unwrap(),
        );

        assert_eq!(
            accessor
                .get_corpse_from_map_source(&context, &source, corpse_guid)
                .unwrap()
                .guid(),
            corpse_guid
        );
        assert!(
            accessor
                .get_world_object_from_map_source(&context, &source, corpse_guid)
                .is_some()
        );
        assert!(
            accessor
                .get_object_ref_by_type_mask_from_map_source(
                    &context,
                    &source,
                    corpse_guid,
                    TypeMask::CORPSE
                )
                .is_none()
        );
    }

    fn typed_pet_record(
        map_id: u32,
        instance_id: u32,
        counter: i64,
        owner: ObjectGuid,
        specialization: u16,
    ) -> (ObjectGuid, MapObjectRecord) {
        let mut pet = Pet::new(owner, crate::PetType::Hunter);
        let pet_guid = guid(HighGuid::Pet, counter);
        pet.creature_mut()
            .unit_mut()
            .world_mut()
            .object_mut()
            .create(pet_guid);
        pet.creature_mut()
            .unit_mut()
            .world_mut()
            .set_map(map_id, instance_id)
            .unwrap();
        pet.creature_mut()
            .unit_mut()
            .world_mut()
            .relocate(Position::xyz(10.0, 20.0, 30.0));
        pet.set_duration(12_345);
        pet.set_specialization(specialization);

        (pet_guid, MapObjectRecord::new_pet(pet).unwrap())
    }

    fn typed_transport_record(
        map_id: u32,
        instance_id: u32,
        counter: i64,
        passenger: ObjectGuid,
    ) -> (ObjectGuid, MapObjectRecord) {
        let mut transport = Transport::new();
        let transport_guid = guid(HighGuid::Transport, counter);
        transport.world_mut().object_mut().create(transport_guid);
        transport.world_mut().set_map(map_id, instance_id).unwrap();
        transport
            .world_mut()
            .relocate(Position::xyz(10.0, 20.0, 30.0));
        transport.world_mut().object_mut().add_to_world();
        transport
            .game_object_mut()
            .set_linked_trap_like_cpp(guid(HighGuid::GameObject, 9_901));
        transport.set_movement_state(crate::TransportMovementState::WaitingOnPauseWaypoint);
        transport.set_path_progress_ms(4_321);
        transport.set_position_change_timer_ms(222);
        assert!(transport.add_passenger(passenger));

        (
            transport_guid,
            MapObjectRecord::new_transport(transport).unwrap(),
        )
    }

    #[test]
    fn typed_pet_map_source_preserves_body_and_generic_pet_lookup_like_cpp() {
        let accessor = ObjectAccessor::default();
        let context = world_object(HighGuid::Player, 530, 1, true);
        let owner = guid(HighGuid::Player, 4_120);
        let (pet_guid, pet_record) = typed_pet_record(530, 1, 4_121, owner, 77);
        let mut source = TestMapSource {
            map_id: 530,
            instance_id: 1,
            records: std::collections::HashMap::new(),
        };
        source.records.insert(pet_guid, pet_record);

        let generic = accessor
            .get_pet_from_map_source(&context, &source, pet_guid)
            .unwrap();
        assert_eq!(generic.guid(), pet_guid);

        let pet = accessor
            .get_typed_pet_from_map_source(&context, &source, pet_guid)
            .unwrap();
        assert_eq!(pet.owner_guid(), owner);
        assert_eq!(pet.pet_type(), crate::PetType::Hunter);
        assert_eq!(pet.specialization(), 77);
        assert_eq!(pet.duration_ms(), 12_345);
        assert_eq!(pet.creature().unit().world().guid(), pet_guid);
    }

    #[test]
    fn typed_transport_map_source_preserves_body_and_embedded_gameobject_like_cpp() {
        let accessor = ObjectAccessor::default();
        let context = world_object(HighGuid::Player, 530, 1, true);
        let passenger = guid(HighGuid::Player, 4_130);
        let (transport_guid, transport_record) = typed_transport_record(530, 1, 4_131, passenger);
        let mut source = TestMapSource {
            map_id: 530,
            instance_id: 1,
            records: std::collections::HashMap::new(),
        };
        source.records.insert(transport_guid, transport_record);

        let game_object = accessor
            .get_typed_game_object_from_map_source(&context, &source, transport_guid)
            .unwrap();
        assert_eq!(game_object.world().guid(), transport_guid);
        assert_eq!(
            game_object.linked_trap_guid_like_cpp(),
            guid(HighGuid::GameObject, 9_901)
        );

        let transport = accessor
            .get_typed_transport_from_map_source(&context, &source, transport_guid)
            .unwrap();
        assert_eq!(
            transport.movement_state(),
            crate::TransportMovementState::WaitingOnPauseWaypoint
        );
        assert_eq!(transport.path_progress_ms(), 4_321);
        assert_eq!(transport.position_change_timer_ms(), 222);
        assert!(transport.passengers().contains(&passenger));
        assert_eq!(transport.world().guid(), transport_guid);
    }

    #[test]
    fn typed_pet_transport_helpers_reject_generic_worldobject_fallbacks_like_cpp() {
        let accessor = ObjectAccessor::default();
        let context = world_object(HighGuid::Player, 530, 1, true);
        let generic_pet = world_object(HighGuid::Pet, 530, 1, true);
        let generic_pet_guid = generic_pet.guid();
        let generic_transport = world_object(HighGuid::Transport, 530, 1, true);
        let generic_transport_guid = generic_transport.guid();
        let mut source = TestMapSource {
            map_id: 530,
            instance_id: 1,
            records: std::collections::HashMap::new(),
        };
        source.records.insert(
            generic_pet_guid,
            MapObjectRecord::new(AccessorObjectKind::Pet, generic_pet).unwrap(),
        );
        source.records.insert(
            generic_transport_guid,
            MapObjectRecord::new(AccessorObjectKind::Transport, generic_transport).unwrap(),
        );

        assert_eq!(
            accessor
                .get_pet_from_map_source(&context, &source, generic_pet_guid)
                .unwrap()
                .guid(),
            generic_pet_guid
        );
        assert_eq!(
            accessor
                .get_world_object_from_map_source(&context, &source, generic_transport_guid)
                .unwrap()
                .guid(),
            generic_transport_guid
        );
        assert_eq!(
            accessor
                .get_game_object_from_map_source(&context, &source, generic_transport_guid)
                .unwrap()
                .guid(),
            generic_transport_guid
        );
        assert!(
            accessor
                .get_typed_pet_from_map_source(&context, &source, generic_pet_guid)
                .is_none()
        );
        assert!(
            accessor
                .get_typed_transport_from_map_source(&context, &source, generic_transport_guid)
                .is_none()
        );
        assert!(
            accessor
                .get_typed_game_object_from_map_source(&context, &source, generic_transport_guid)
                .is_none()
        );
    }

    #[test]
    fn typed_pet_transport_lookup_does_not_cross_kinds_like_cpp() {
        let accessor = ObjectAccessor::default();
        let context = world_object(HighGuid::Player, 530, 1, true);
        let owner = guid(HighGuid::Player, 4_140);
        let (creature_guid, creature_record) = typed_creature_record(530, 1, 4_141, 60);
        let (pet_guid, pet_record) = typed_pet_record(530, 1, 4_142, owner, 11);
        let (game_object_guid, game_object_record) =
            typed_game_object_record(530, 1, 4_143, guid(HighGuid::GameObject, 4_144));
        let (transport_guid, transport_record) =
            typed_transport_record(530, 1, 4_145, guid(HighGuid::Player, 4_146));
        let mut source = TestMapSource {
            map_id: 530,
            instance_id: 1,
            records: std::collections::HashMap::new(),
        };
        source.records.insert(creature_guid, creature_record);
        source.records.insert(pet_guid, pet_record);
        source.records.insert(game_object_guid, game_object_record);
        source.records.insert(transport_guid, transport_record);

        assert!(
            accessor
                .get_typed_pet_from_map_source(&context, &source, creature_guid)
                .is_none()
        );
        assert!(
            accessor
                .get_typed_creature_from_map_source(&context, &source, pet_guid)
                .is_none()
        );
        assert!(
            accessor
                .get_typed_transport_from_map_source(&context, &source, game_object_guid)
                .is_none()
        );
        assert!(
            accessor
                .get_typed_transport_from_map_source(&context, &source, transport_guid)
                .is_some()
        );
        assert!(
            accessor
                .get_typed_game_object_from_map_source(&context, &source, transport_guid)
                .is_some()
        );
        assert!(
            accessor
                .get_typed_transport_from_map_source(&context, &source, game_object_guid)
                .is_none()
        );
    }

    #[test]
    fn typed_transport_lookup_requires_source_and_context_same_map_like_cpp() {
        let accessor = ObjectAccessor::default();
        let context = world_object(HighGuid::Player, 530, 1, true);
        let (transport_guid, transport_record) =
            typed_transport_record(530, 1, 4_150, guid(HighGuid::Player, 4_151));
        let mut source = TestMapSource {
            map_id: 530,
            instance_id: 1,
            records: std::collections::HashMap::new(),
        };
        source.records.insert(transport_guid, transport_record);

        source.map_id = 571;
        assert!(
            accessor
                .get_typed_transport_from_map_source(&context, &source, transport_guid)
                .is_none()
        );
        source.map_id = 530;
        source.instance_id = 2;
        assert!(
            accessor
                .get_typed_transport_from_map_source(&context, &source, transport_guid)
                .is_none()
        );
        source.instance_id = 1;

        let wrong_context = world_object(HighGuid::Player, 571, 1, true);
        assert!(
            accessor
                .get_typed_transport_from_map_source(&wrong_context, &source, transport_guid)
                .is_none()
        );
    }

    #[test]
    fn unit_and_creature_or_pet_or_vehicle_helpers_match_cpp() {
        let accessor = ObjectAccessor::default();
        let context = world_object(HighGuid::Player, 530, 1, true);
        let pet = world_object(HighGuid::Pet, 530, 1, true);
        let pet_guid = pet.guid();
        let mut source = TestMapSource {
            map_id: 530,
            instance_id: 1,
            records: std::collections::HashMap::new(),
        };
        source.records.insert(
            pet_guid,
            MapObjectRecord::new(AccessorObjectKind::Pet, pet).unwrap(),
        );

        assert_eq!(
            accessor
                .get_unit_from_map_source(&context, &source, pet_guid)
                .unwrap()
                .guid(),
            pet_guid
        );
        assert_eq!(
            accessor
                .get_creature_or_pet_or_vehicle_from_map_source(&context, &source, pet_guid)
                .unwrap()
                .guid(),
            pet_guid
        );
    }
}
