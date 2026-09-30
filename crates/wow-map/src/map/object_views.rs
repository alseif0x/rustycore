// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Private borrowed projections of the canonical map object body.
//!
//! Views carry only a borrow. Kind and typed-body projections remain separate:
//! a generic record can have a typed kind without a typed body, and Transport
//! retains its GameObject projection. Owned insertion and removal stay unchanged.

use wow_core::ObjectGuid;
use wow_entities::{
    AccessorObjectKind, AreaTrigger, Conversation, Corpse, Creature, DynamicObject, GameObject,
    MapObjectRecord, Pet, Player, SceneObject, Transport, Unit, WorldObject,
};

pub(crate) struct ObjectRef<'a> {
    body: ObjectRefBody<'a>,
}

pub(in crate::map) struct ObjectMut<'a> {
    body: ObjectMutBody<'a>,
}

enum ObjectRefBody<'a> {
    Record(&'a MapObjectRecord),
    Creature(&'a Creature),
}

enum ObjectMutBody<'a> {
    Record(&'a mut MapObjectRecord),
    Creature(&'a mut Creature),
}

impl<'a> ObjectRef<'a> {
    pub(in crate::map) fn new(record: &'a MapObjectRecord) -> Self {
        Self {
            body: ObjectRefBody::Record(record),
        }
    }

    pub(in crate::map) fn from_creature(creature: &'a Creature) -> Self {
        Self {
            body: ObjectRefBody::Creature(creature),
        }
    }

    pub(crate) fn kind(&self) -> AccessorObjectKind {
        match &self.body {
            ObjectRefBody::Record(record) => record.kind(),
            ObjectRefBody::Creature(_) => AccessorObjectKind::Creature,
        }
    }

    pub(in crate::map) fn object(&self) -> &'a WorldObject {
        match &self.body {
            ObjectRefBody::Record(record) => record.object(),
            ObjectRefBody::Creature(creature) => creature.unit().world(),
        }
    }

    pub(crate) fn area_trigger(&self) -> Option<&'a AreaTrigger> {
        match &self.body {
            ObjectRefBody::Record(record) => record.area_trigger(),
            ObjectRefBody::Creature(_) => None,
        }
    }

    pub(crate) fn conversation(&self) -> Option<&'a Conversation> {
        match &self.body {
            ObjectRefBody::Record(record) => record.conversation(),
            ObjectRefBody::Creature(_) => None,
        }
    }

    pub(in crate::map) fn corpse(&self) -> Option<&'a Corpse> {
        match &self.body {
            ObjectRefBody::Record(record) => record.corpse(),
            ObjectRefBody::Creature(_) => None,
        }
    }

    pub(crate) fn creature(&self) -> Option<&'a Creature> {
        match &self.body {
            ObjectRefBody::Record(record) => record.creature(),
            ObjectRefBody::Creature(creature) => Some(*creature),
        }
    }

    pub(crate) fn dynamic_object(&self) -> Option<&'a DynamicObject> {
        match &self.body {
            ObjectRefBody::Record(record) => record.dynamic_object(),
            ObjectRefBody::Creature(_) => None,
        }
    }

    pub(crate) fn game_object(&self) -> Option<&'a GameObject> {
        match &self.body {
            ObjectRefBody::Record(record) => record.game_object(),
            ObjectRefBody::Creature(_) => None,
        }
    }

    pub(crate) fn pet(&self) -> Option<&'a Pet> {
        match &self.body {
            ObjectRefBody::Record(record) => record.pet(),
            ObjectRefBody::Creature(_) => None,
        }
    }

    pub(in crate::map) fn player(&self) -> Option<&'a Player> {
        match &self.body {
            ObjectRefBody::Record(record) => record.player(),
            ObjectRefBody::Creature(_) => None,
        }
    }

    pub(crate) fn scene_object(&self) -> Option<&'a SceneObject> {
        match &self.body {
            ObjectRefBody::Record(record) => record.scene_object(),
            ObjectRefBody::Creature(_) => None,
        }
    }

    pub(crate) fn transport(&self) -> Option<&'a Transport> {
        match &self.body {
            ObjectRefBody::Record(record) => record.transport(),
            ObjectRefBody::Creature(_) => None,
        }
    }

    pub(in crate::map) fn charmer_guid(&self) -> Option<ObjectGuid> {
        match &self.body {
            ObjectRefBody::Record(record) => record.charmer_guid_like_cpp(),
            ObjectRefBody::Creature(creature) => {
                creature.unit().subsystems().control.charmer_guid_like_cpp()
            }
        }
    }

    pub(in crate::map) fn is_unit_owner(&self) -> bool {
        matches!(
            self.kind(),
            AccessorObjectKind::Player | AccessorObjectKind::Creature | AccessorObjectKind::Pet
        ) && (self.player().is_some() || self.creature().is_some() || self.pet().is_some())
    }

    pub(in crate::map) fn unit(&self) -> Option<&'a Unit> {
        match self.kind() {
            AccessorObjectKind::Player => self.player().map(Player::unit),
            AccessorObjectKind::Creature => self.creature().map(Creature::unit),
            AccessorObjectKind::Pet => self.pet().map(|pet| pet.creature().unit()),
            _ => None,
        }
    }
}

impl<'a> ObjectMut<'a> {
    pub(in crate::map) fn new(record: &'a mut MapObjectRecord) -> Self {
        Self {
            body: ObjectMutBody::Record(record),
        }
    }

    pub(in crate::map) fn from_creature(creature: &'a mut Creature) -> Self {
        Self {
            body: ObjectMutBody::Creature(creature),
        }
    }

    pub(in crate::map) fn reborrow(&mut self) -> ObjectMut<'_> {
        match &mut self.body {
            ObjectMutBody::Record(record) => ObjectMut::new(record),
            ObjectMutBody::Creature(creature) => ObjectMut::from_creature(creature),
        }
    }

    pub(in crate::map) fn as_ref(&self) -> ObjectRef<'_> {
        match &self.body {
            ObjectMutBody::Record(record) => ObjectRef::new(record),
            ObjectMutBody::Creature(creature) => ObjectRef::from_creature(creature),
        }
    }

    pub(in crate::map) fn kind(&self) -> AccessorObjectKind {
        self.as_ref().kind()
    }

    pub(in crate::map) fn object(&self) -> &WorldObject {
        self.as_ref().object()
    }

    pub(in crate::map) fn object_mut(self) -> &'a mut WorldObject {
        match self.body {
            ObjectMutBody::Record(record) => record.object_mut(),
            ObjectMutBody::Creature(creature) => creature.unit_mut().world_mut(),
        }
    }

    pub(in crate::map) fn area_trigger_mut(self) -> Option<&'a mut AreaTrigger> {
        match self.body {
            ObjectMutBody::Record(record) => record.area_trigger_mut(),
            ObjectMutBody::Creature(_) => None,
        }
    }

    pub(in crate::map) fn conversation_mut(self) -> Option<&'a mut Conversation> {
        match self.body {
            ObjectMutBody::Record(record) => record.conversation_mut(),
            ObjectMutBody::Creature(_) => None,
        }
    }

    pub(in crate::map) fn corpse_mut(self) -> Option<&'a mut Corpse> {
        match self.body {
            ObjectMutBody::Record(record) => record.corpse_mut(),
            ObjectMutBody::Creature(_) => None,
        }
    }

    pub(in crate::map) fn creature_mut(self) -> Option<&'a mut Creature> {
        match self.body {
            ObjectMutBody::Record(record) => record.creature_mut(),
            ObjectMutBody::Creature(creature) => Some(creature),
        }
    }

    pub(in crate::map) fn dynamic_object_mut(self) -> Option<&'a mut DynamicObject> {
        match self.body {
            ObjectMutBody::Record(record) => record.dynamic_object_mut(),
            ObjectMutBody::Creature(_) => None,
        }
    }

    pub(in crate::map) fn game_object_mut(self) -> Option<&'a mut GameObject> {
        match self.body {
            ObjectMutBody::Record(record) => record.game_object_mut(),
            ObjectMutBody::Creature(_) => None,
        }
    }

    pub(in crate::map) fn pet_mut(self) -> Option<&'a mut Pet> {
        match self.body {
            ObjectMutBody::Record(record) => record.pet_mut(),
            ObjectMutBody::Creature(_) => None,
        }
    }

    pub(in crate::map) fn player_mut(self) -> Option<&'a mut Player> {
        match self.body {
            ObjectMutBody::Record(record) => record.player_mut(),
            ObjectMutBody::Creature(_) => None,
        }
    }

    pub(in crate::map) fn scene_object_mut(self) -> Option<&'a mut SceneObject> {
        match self.body {
            ObjectMutBody::Record(record) => record.scene_object_mut(),
            ObjectMutBody::Creature(_) => None,
        }
    }

    pub(in crate::map) fn transport_mut(self) -> Option<&'a mut Transport> {
        match self.body {
            ObjectMutBody::Record(record) => record.transport_mut(),
            ObjectMutBody::Creature(_) => None,
        }
    }

    pub(in crate::map) fn unit_mut(self) -> Option<&'a mut Unit> {
        match self.kind() {
            AccessorObjectKind::Player => self.player_mut().map(Player::unit_mut),
            AccessorObjectKind::Creature => self.creature_mut().map(Creature::unit_mut),
            AccessorObjectKind::Pet => self.pet_mut().map(|pet| pet.creature_mut().unit_mut()),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests;
