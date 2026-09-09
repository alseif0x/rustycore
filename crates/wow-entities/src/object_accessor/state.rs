//! Object accessor records and lookups state definitions, part 1 of 1.
//!
//! Separated from the object_accessor.rs root under #648. Behaviour is preserved.

use super::*;

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
    pub(super) normalized_name: String,
    pub(super) body: AccessorPlayerBody,
    pub(super) inventory: PlayerInventoryStorage,
    pub(super) items: HashMap<ObjectGuid, Item>,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) enum AccessorPlayerBody {
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
    pub(super) kind: AccessorObjectKind,
    pub(super) body: MapObjectBody,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) enum MapObjectBody {
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
    pub(super) players: HashMap<ObjectGuid, AccessorPlayer>,
    pub(super) player_names: HashMap<String, ObjectGuid>,
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

pub(super) fn same_map(left: &WorldObject, right: &WorldObject) -> bool {
    left.has_current_map()
        && right.has_current_map()
        && left.map_id() == right.map_id()
        && left.instance_id() == right.instance_id()
}

pub(super) fn kind_accepts_guid(kind: AccessorObjectKind, guid_kind: AccessorObjectKind) -> bool {
    kind == guid_kind
}
