use std::collections::{HashMap, HashSet};

use wow_constants::{TypeId, TypeMask};
use wow_core::{ObjectGuid, Position};
use crate::{
    CreatureLoot, LootInstallOutcome, OwnedLootAuthority, OwnedLootAuthorityLifecycle,
    OwnedLootAuthorityStamp, OwnedLootSnapshot,
};

pub use wow_data_model::game_object::{
    BarberChairUseSource, CameraUseSource, CapturePointUseSource, ChairUseSource,
    FlagDropUseSource, FlagStandUseSource, GAMEOBJECT_DATA_CHEST_DUNGEON_ENCOUNTER,
    GAMEOBJECT_DATA_CHEST_LINKED_TRAP, GAMEOBJECT_DATA_CHEST_LOOT,
    GAMEOBJECT_DATA_CHEST_PERSONAL_LOOT, GAMEOBJECT_DATA_CHEST_PUSH_LOOT,
    GAMEOBJECT_DATA_CHEST_TRIGGERED_EVENT, GAMEOBJECT_DATA_CHEST_USE_GROUP_LOOT_RULES,
    GAMEOBJECT_DATA_GATHERING_NODE_DESPAWN_DELAY, GAMEOBJECT_DATA_GATHERING_NODE_LINKED_TRAP,
    GAMEOBJECT_DATA_GATHERING_NODE_MAX_LOOTS, GAMEOBJECT_DATA_GATHERING_NODE_SPELL,
    GAMEOBJECT_DATA_GATHERING_NODE_TRIGGERED_EVENT, GAMEOBJECT_DATA_GATHERING_NODE_XP_DIFFICULTY,
    GAMEOBJECT_DATA_SPELL_FOCUS_LINKED_TRAP, GAMEOBJECT_DATA_SPELL_FOCUS_RADIUS,
    GAMEOBJECT_DATA_SPELL_FOCUS_TYPE, GAMEOBJECT_DATA_UI_LINK_SPELL_FOCUS_RADIUS,
    GAMEOBJECT_DATA_UI_LINK_SPELL_FOCUS_TYPE, GAMEOBJECT_TYPE_AREADAMAGE,
    GAMEOBJECT_TYPE_BARBER_CHAIR, GAMEOBJECT_TYPE_BINDER, GAMEOBJECT_TYPE_BUTTON,
    GAMEOBJECT_TYPE_CAMERA, GAMEOBJECT_TYPE_CAPTURE_POINT, GAMEOBJECT_TYPE_CHAIR,
    GAMEOBJECT_TYPE_CHEST, GAMEOBJECT_TYPE_DESTRUCTIBLE_BUILDING, GAMEOBJECT_TYPE_DOOR,
    GAMEOBJECT_TYPE_DUNGEON_DIFFICULTY, GAMEOBJECT_TYPE_FISHING_HOLE,
    GAMEOBJECT_TYPE_FISHING_NODE, GAMEOBJECT_TYPE_FLAGDROP, GAMEOBJECT_TYPE_FLAGSTAND,
    GAMEOBJECT_TYPE_GATHERING_NODE, GAMEOBJECT_TYPE_GENERIC, GAMEOBJECT_TYPE_GOOBER,
    GAMEOBJECT_TYPE_GUARDPOST, GAMEOBJECT_TYPE_GUILD_BANK, GAMEOBJECT_TYPE_ITEM_FORGE,
    GAMEOBJECT_TYPE_MAILBOX, GAMEOBJECT_TYPE_MAP_OBJECT, GAMEOBJECT_TYPE_MEETINGSTONE,
    GAMEOBJECT_TYPE_MINI_GAME, GAMEOBJECT_TYPE_NEW_FLAG, GAMEOBJECT_TYPE_NEW_FLAG_DROP,
    GAMEOBJECT_TYPE_QUESTGIVER, GAMEOBJECT_TYPE_RITUAL, GAMEOBJECT_TYPE_SPELLCASTER,
    GAMEOBJECT_TYPE_SPELL_FOCUS, GAMEOBJECT_TYPE_TEXT, GAMEOBJECT_TYPE_TRANSPORT,
    GAMEOBJECT_TYPE_TRAP, GAMEOBJECT_TYPE_UI_LINK, GameObjectLootSource, GameObjectTemplateData,
    GameObjectTemplateLifecycleRecord, GatheringNodeUseSource, GooberUseSource,
    GuardPostUseSource, ItemForgeUseSource,
    MAX_GAMEOBJECT_DATA, MeetingStoneUseSource,
    NewFlagDropUseSource, NewFlagUseSource, QuestgiverUseSource, RitualUseSource,
    SpellFocusUseSource, SpellcasterUseSource, TrapUseSource, UiLinkUseSource,
    ui_link_player_interaction_type_like_cpp,
};
use wow_data_model::game_object::{GAMEOBJECT_TYPE_MAP_OBJ_TRANSPORT, MAX_GAMEOBJECT_TYPE};

use crate::{
    CreateObjectFlags, MapBindingError, ObjectChangedFields, ObjectDataUpdate, UpdateMask,
    WorldObject,
    update_fields::{GAME_OBJECT_DATA_BITS, TYPEID_GAME_OBJECT},
};

mod interaction;
mod use_values;
pub use use_values::{ChairPlacement, CooldownOutcome, GameObjectUseValues, TrapUseEffect};
pub use interaction::{gameobject_interaction_distance, gameobject_display_box_contains};
mod ops_1;
mod ops_2;
mod state_1;
mod state_2;
#[allow(unused_imports)]
pub use ops_1::*;
#[allow(unused_imports)]
pub use ops_2::*;
#[allow(unused_imports)]
pub use state_1::*;
#[allow(unused_imports)]
pub use state_2::*;

#[cfg(test)]
#[path = "game_object/tests/mod.rs"]
mod tests;
