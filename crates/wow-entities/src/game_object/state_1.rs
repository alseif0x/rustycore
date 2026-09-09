//! GameObject template, loot and runtime state state definitions, part 1 of 2.
//!
//! Separated from the game_object.rs root under #636. Behaviour is preserved.

use super::*;

pub const DEFAULT_GAMEOBJECT_RESPAWN_DELAY_SECS: u32 = 300;

pub const GAMEOBJECT_LOOT_MODE_DEFAULT: u16 = 0x1;

pub const GAMEOBJECT_TYPE_DOOR: u32 = 0;

pub const GAMEOBJECT_TYPE_BUTTON: u32 = 1;

pub const GAMEOBJECT_TYPE_QUESTGIVER: u32 = 2;

pub const GAMEOBJECT_TYPE_CHEST: u32 = 3;

pub const GAMEOBJECT_TYPE_BINDER: u32 = 4;

pub const GAMEOBJECT_TYPE_GENERIC: u32 = 5;

pub const GAMEOBJECT_TYPE_TRAP: u32 = 6;

pub const GAMEOBJECT_TYPE_CHAIR: u32 = 7;

pub const GAMEOBJECT_TYPE_SPELL_FOCUS: u32 = 8;

pub const GAMEOBJECT_TYPE_TEXT: u32 = 9;

pub const GAMEOBJECT_TYPE_GOOBER: u32 = 10;

pub const GAMEOBJECT_TYPE_TRANSPORT: u32 = 11;

pub const GAMEOBJECT_TYPE_AREADAMAGE: u32 = 12;

pub const GAMEOBJECT_TYPE_CAMERA: u32 = 13;

pub const GAMEOBJECT_TYPE_MAP_OBJECT: u32 = 14;

// C++ anchor: /home/server/woltk-trinity-legacy/src/server/game/Miscellaneous/SharedDefines.h:2842
pub const GAMEOBJECT_TYPE_MAP_OBJ_TRANSPORT: u32 = 15;

pub const GAMEOBJECT_TYPE_FISHING_NODE: u32 = 17;

pub const GAMEOBJECT_TYPE_RITUAL: u32 = 18;

pub const GAMEOBJECT_TYPE_MAILBOX: u32 = 19;

pub const GAMEOBJECT_TYPE_GUARDPOST: u32 = 21;

pub const GAMEOBJECT_TYPE_SPELLCASTER: u32 = 22;

pub const GAMEOBJECT_TYPE_MEETINGSTONE: u32 = 23;

pub const GAMEOBJECT_TYPE_FLAGSTAND: u32 = 24;

pub const GAMEOBJECT_TYPE_FISHING_HOLE: u32 = 25;

pub const GAMEOBJECT_TYPE_FLAGDROP: u32 = 26;

pub const GAMEOBJECT_TYPE_MINI_GAME: u32 = 27;

pub const GAMEOBJECT_TYPE_AURA_GENERATOR: u32 = 30;

pub const GAMEOBJECT_TYPE_DUNGEON_DIFFICULTY: u32 = 31;

pub const GAMEOBJECT_TYPE_BARBER_CHAIR: u32 = 32;

pub const GAMEOBJECT_TYPE_DESTRUCTIBLE_BUILDING: u32 = 33;

pub const GAMEOBJECT_TYPE_GUILD_BANK: u32 = 34;

pub const GAMEOBJECT_TYPE_NEW_FLAG: u32 = 36;

pub const GAMEOBJECT_TYPE_NEW_FLAG_DROP: u32 = 37;

pub const GAMEOBJECT_TYPE_CAPTURE_POINT: u32 = 42;

pub const GAMEOBJECT_TYPE_ITEM_FORGE: u32 = 47;

pub const GAMEOBJECT_TYPE_UI_LINK: u32 = 48;

pub const GAMEOBJECT_TYPE_GATHERING_NODE: u32 = 50;

pub const GO_DYNFLAG_LO_ACTIVATE: u32 = 0x0004;

pub const GO_DYNFLAG_LO_DEPLETED: u32 = 0x0010;

pub const GO_DYNFLAG_LO_SPARKLE: u32 = 0x0020;

pub const GO_DYNFLAG_LO_NO_INTERACT: u32 = 0x0080;

pub const GO_DYNFLAG_LO_HIGHLIGHT: u32 = 0x0200;

// C++ anchor: /home/server/woltk-trinity-legacy/src/server/game/Miscellaneous/SharedDefines.h:2892
pub const MAX_GAMEOBJECT_TYPE: u32 = 63;

pub const MAX_GAMEOBJECT_DATA: usize = 35;

pub const GAMEOBJECT_DATA_CHEST_LOOT: usize = 1;

pub const GAMEOBJECT_DATA_CHEST_RESTOCK_TIME: usize = 2;

pub const GAMEOBJECT_DATA_CHEST_CONSUMABLE: usize = 3;

pub const GAMEOBJECT_DATA_CHEST_TRIGGERED_EVENT: usize = 6;

pub const GAMEOBJECT_DATA_CHEST_LINKED_TRAP: usize = 7;

// C++ anchor: /home/server/woltk-trinity-legacy/src/server/game/Entities/GameObject/GameObjectData.h:105
pub const GAMEOBJECT_DATA_CHEST_QUEST_ID: usize = 8;

pub const GAMEOBJECT_DATA_CHEST_USE_GROUP_LOOT_RULES: usize = 15;

pub const GAMEOBJECT_DATA_CHEST_DUNGEON_ENCOUNTER: usize = 25;

pub const GAMEOBJECT_DATA_CHEST_PERSONAL_LOOT: usize = 30;

pub const GAMEOBJECT_DATA_CHEST_PUSH_LOOT: usize = 33;

// C++ anchor: /home/server/woltk-trinity-legacy/src/server/game/Entities/GameObject/GameObjectData.h:65-68
pub const GAMEOBJECT_DATA_BUTTON_LINKED_TRAP: usize = 3;

pub const GAMEOBJECT_DATA_GOOBER_CONSUMABLE: usize = 5;

pub const GAMEOBJECT_DATA_GATHERING_NODE_DESPAWN_DELAY: usize = 6;

pub const GAMEOBJECT_DATA_GATHERING_NODE_TRIGGERED_EVENT: usize = 7;

pub const GAMEOBJECT_DATA_GATHERING_NODE_XP_DIFFICULTY: usize = 13;

pub const GAMEOBJECT_DATA_GATHERING_NODE_SPELL: usize = 14;

pub const GAMEOBJECT_DATA_GATHERING_NODE_MAX_LOOTS: usize = 18;

pub const GAMEOBJECT_DATA_GATHERING_NODE_LINKED_TRAP: usize = 20;

// C++ anchors:
// - GameObjectData.h:191-193 for GAMEOBJECT_TYPE_SPELL_FOCUS
// - GameObjectData.h:697-699 for GAMEOBJECT_TYPE_UI_LINK
pub const GAMEOBJECT_DATA_SPELL_FOCUS_TYPE: usize = 0;

pub const GAMEOBJECT_DATA_SPELL_FOCUS_RADIUS: usize = 1;

pub const GAMEOBJECT_DATA_SPELL_FOCUS_LINKED_TRAP: usize = 2;

pub const GAMEOBJECT_DATA_UI_LINK_SPELL_FOCUS_TYPE: usize = 3;

pub const GAMEOBJECT_DATA_UI_LINK_SPELL_FOCUS_RADIUS: usize = 4;

pub const GO_FLAG_IN_USE: u32 = 0x0000_0001;

pub const GO_FLAG_INTERACT_COND: u32 = 0x0000_0004;

// C++ anchor: /home/server/woltk-trinity-legacy/src/server/game/Miscellaneous/SharedDefines.h:2902
pub const GO_FLAG_NODESPAWN: u32 = 0x0000_0020;

// C++ anchor: /home/server/woltk-trinity-legacy/src/server/game/Miscellaneous/SharedDefines.h:2914
pub const GO_FLAG_MAP_OBJECT: u32 = 0x0010_0000;

pub const GO_FLAG_IN_MULTI_USE: u32 = 0x0020_0000;

pub const GAME_OBJECT_DATA_PARENT_BIT: usize = 0;

pub const GAME_OBJECT_DATA_DISPLAY_ID_BIT: usize = 4;

pub const GAME_OBJECT_DATA_CREATED_BY_BIT: usize = 9;

pub const GAME_OBJECT_DATA_FLAGS_BIT: usize = 11;

pub const GAME_OBJECT_DATA_FACTION_TEMPLATE_BIT: usize = 13;

pub const GAME_OBJECT_DATA_LEVEL_BIT: usize = 14;

pub const GAME_OBJECT_DATA_STATE_BIT: usize = 15;

pub const GAME_OBJECT_DATA_TYPE_ID_BIT: usize = 16;

pub const GAME_OBJECT_DATA_PERCENT_HEALTH_BIT: usize = 17;

pub const GAME_OBJECT_DATA_ART_KIT_BIT: usize = 18;

pub const GAME_OBJECT_DATA_CUSTOM_PARAM_BIT: usize = 19;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i8)]
pub enum GoState {
    Active = 0,
    Ready = 1,
    Destroyed = 2,
    TransportActive = 24,
    TransportStopped = 25,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum LootState {
    NotReady = 0,
    Ready = 1,
    Activated = 2,
    JustDeactivated = 3,
}

/// Represented status for the `m_despawnDelay` branch of TrinityCore
/// `GameObject::Update(diff)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameObjectUpdateStatusLikeCpp {
    Updated,
    DespawnRequested,
}

/// Evidence for the bounded Rust representation of TrinityCore
/// `GameObject::Update(diff)`.
///
/// C++ anchors:
/// - `GameObject.cpp:1215-1233`: `WorldObject::Update(diff)`, AI lookup/
///   initialization branch, `m_despawnDelay` countdown, and immediate
///   `DespawnOrUnsummon(0ms, m_despawnRespawnTime)` when the delay expires.
/// - `GameObject.cpp:1235-1274` and `1276+`: go-type implementation,
///   per-player state/visibility packets and loot-state machine remain explicit
///   gaps; booleans here mark non-represented branches, not execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameObjectUpdateOutcomeLikeCpp {
    pub diff_ms: u32,
    pub status: GameObjectUpdateStatusLikeCpp,
    pub despawn_delay_before_ms: u32,
    pub despawn_delay_after_ms: u32,
    pub despawn_respawn_time_secs: u32,
    pub world_update_would_run: bool,
    pub ai_update_not_represented: bool,
    pub go_type_impl_update_not_represented: bool,
    pub despawn_or_unsummon_requested: bool,
}

/// Evidence for the bounded Rust representation of TrinityCore
/// `GameObject::EnableCollision(bool)`.
///
/// C++ anchor: `GameObject.cpp:3856-3864` returns early when `!m_model`; otherwise it only
/// forwards the requested value to `m_model->enableCollision(enable)`. The commented map insert
/// remains intentionally unrepresented here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameObjectCollisionOutcomeLikeCpp {
    pub requested_enable: bool,
    pub represented_model_present: bool,
    pub previous_collision_enabled: Option<bool>,
    pub new_collision_enabled: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GameObjectLootSource {
    pub loot_id: u32,
    pub use_group_loot_rules: bool,
    pub dungeon_encounter_id: u32,
    pub personal_loot_id: u32,
    pub push_loot_id: u32,
    pub triggered_event_id: u32,
    pub linked_trap_entry: u32,
    pub chest_restock_time_secs: u32,
    pub chest_consumable: bool,
    pub chest_quest_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GameObjectOwnedLoot {
    pub(super) gold: u32,
    pub(super) unlooted_count: u32,
}

impl GameObjectOwnedLoot {
    pub const fn new(gold: u32, unlooted_count: u32) -> Self {
        Self {
            gold,
            unlooted_count,
        }
    }

    pub const fn gold(&self) -> u32 {
        self.gold
    }

    pub const fn unlooted_count(&self) -> u32 {
        self.unlooted_count
    }

    pub const fn is_looted_like_cpp(&self) -> bool {
        self.gold == 0 && self.unlooted_count == 0
    }
}

pub(super) fn game_object_owned_loot_from_snapshot(
    snapshot: &OwnedLootSnapshot,
) -> GameObjectOwnedLoot {
    GameObjectOwnedLoot::new(snapshot.loot.coins, u32::from(snapshot.loot.unlooted_count))
}

impl GameObjectLootSource {
    pub const fn is_empty(&self) -> bool {
        self.loot_id == 0 && self.personal_loot_id == 0 && self.push_loot_id == 0
    }

    pub const fn open_loot_id_like_cpp(&self) -> u32 {
        if self.loot_id != 0 {
            self.loot_id
        } else {
            self.personal_loot_id
        }
    }

    pub const fn has_open_loot_like_cpp(&self) -> bool {
        self.open_loot_id_like_cpp() != 0
    }

    /// C++ stores every `chestPersonalLoot` result in `m_personalLoot` when
    /// there is no shared `GetLootId()` result. `DungeonEncounter` only
    /// chooses how the personal pools are generated; it does not decide
    /// whether the loot is personal (`GameObject.cpp:2584-2613`).
    pub const fn uses_personal_loot_like_cpp(&self) -> bool {
        self.loot_id == 0 && self.personal_loot_id != 0
    }

    pub const fn is_personal_encounter_loot_like_cpp(&self) -> bool {
        self.uses_personal_loot_like_cpp() && self.dungeon_encounter_id != 0
    }

    pub const fn should_autostore_push_loot_like_cpp(&self) -> bool {
        self.loot_id == 0 && self.push_loot_id != 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameObjectTemplateData {
    pub go_type: u32,
    pub data: [u32; MAX_GAMEOBJECT_DATA],
}

/// Represented subset of TrinityCore `GameObjectTemplate`, template addon and override data
/// consumed by `GameObject::Create`.
///
/// ObjectMgr lookups, zone-script entry overrides, model creation, phasing, terrain visible-map
/// setup and AddToMap are deliberately external. Callers pass values already resolved from the
/// C++ template/addon/override sources that `wow-entities` can intrinsically own.
#[derive(Debug, Clone, PartialEq)]
pub struct GameObjectTemplateLifecycleRecord {
    pub entry: u32,
    pub name: String,
    pub go_type: u32,
    pub display_id: u32,
    pub scale: f32,
    pub faction: u32,
    pub flags: u32,
    pub data: [u32; MAX_GAMEOBJECT_DATA],
    pub world_effect_id: u32,
    pub anim_kit_id: u16,
    pub level: u32,
    pub percent_health: u8,
    pub custom_param: u32,
}

/// Resolved, testable input for TrinityCore `GameObject::Create`.
///
/// Transport type 11 remains a resolved intrinsic shape only here: transport GUID/server-time,
/// implementation data, `startOpen`, active state transitions, pathing and passenger runtime are
/// owned by future transport/map wiring, not by this entity lifecycle record.
#[derive(Debug, Clone, PartialEq)]
pub struct GameObjectCreateLifecycleRecord {
    pub guid: ObjectGuid,
    pub map_id: u32,
    pub instance_id: u32,
    pub position: Position,
    pub rotation: [f32; 4],
    pub anim_progress: u8,
    pub go_state: GoState,
    pub art_kit: u32,
    pub dynamic: bool,
    pub spawn_id: u64,
    pub template: GameObjectTemplateLifecycleRecord,
}

/// Represented subset of TrinityCore `GameObjectData` consumed by `GameObject::LoadFromDB`.
#[derive(Debug, Clone, PartialEq)]
pub struct GameObjectLoadFromDbLifecycleRecord {
    pub create: GameObjectCreateLifecycleRecord,
    pub spawntimesecs: i32,
    /// Effective map-owned respawn time after caller-owned `Map`/time processing.
    ///
    /// C++ `LoadFromDB` clears due timers (`respawnTime <= now`) and calls `RemoveRespawnTime`
    /// before applying entity state. `wow-entities` does not own Map time or DB timer removal, so
    /// callers must pre-normalize due timers to `0` before building this record.
    pub effective_map_respawn_time: i64,
    pub despawn_possible: bool,
    pub despawn_at_action: bool,
    pub respawn_compatibility_mode: bool,
    /// C++ stores `GameObjectData::StringId` in `m_stringIds[1]`; Rust stores the represented
    /// lifecycle handoff value as entity metadata only, with DB/phasing/AddToMap still external.
    pub string_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GameObjectLifecycleError {
    InvalidGameObjectType { entry: u32, go_type: u32 },
    InvalidMapObjectTransportType { entry: u32 },
    InvalidPosition { entry: u32, position: Position },
    MapBinding { entry: u32, source: MapBindingError },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GatheringNodeUseSource {
    pub loot_id: u32,
    pub despawn_delay_secs: u32,
    pub triggered_event_id: u32,
    pub xp_difficulty: u32,
    pub spell_id: u32,
    pub max_loots: u32,
    pub linked_trap_entry: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TrapUseSource {
    pub radius: u32,
    pub spell_id: u32,
    pub charges: u32,
    pub cooldown_secs: u32,
    pub start_delay_secs: u32,
    pub ignore_totems: bool,
    pub check_all_units: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ChairUseSource {
    pub chair_slots: u32,
    pub chair_height: u32,
    pub triggered_event_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BarberChairUseSource {
    pub chair_height: u32,
    pub sit_anim_kit: u32,
    pub customization_scope: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct UiLinkUseSource {
    pub ui_link_type: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SpellFocusUseSource {
    pub focus_type: u32,
    pub radius: u32,
    pub linked_trap_entry: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ItemForgeUseSource {
    pub condition_id: u32,
    pub forge_type: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CapturePointUseSource {
    pub capture_time_ms: u32,
    pub assault_broadcast_horde: u32,
    pub capture_broadcast_horde: u32,
    pub defended_broadcast_horde: u32,
    pub assault_broadcast_alliance: u32,
    pub capture_broadcast_alliance: u32,
    pub defended_broadcast_alliance: u32,
    pub world_state_id: u32,
    pub contested_event_horde: u32,
    pub capture_event_horde: u32,
    pub defended_event_horde: u32,
    pub contested_event_alliance: u32,
    pub capture_event_alliance: u32,
    pub defended_event_alliance: u32,
    pub spell_visual_ids: [u32; 5],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FlagStandUseSource {
    pub pickup_spell_id: u32,
    pub return_aura_id: u32,
    pub return_spell_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FlagDropUseSource {
    pub event_id: u32,
    pub pickup_spell_id: u32,
    pub expire_duration_ms: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct NewFlagUseSource {
    pub pickup_spell_id: u32,
    pub expire_duration_ms: u32,
    pub respawn_time_ms: u32,
    pub flag_drop_entry: u32,
    pub exclusive_category: i32,
    pub world_state_id: u32,
    pub return_on_defender_interact: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct NewFlagDropUseSource {
    pub spawn_vignette_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RitualUseSource {
    pub casters_required: u32,
    pub spell_id: u32,
    pub anim_spell_id: u32,
    pub persistent: bool,
    pub caster_target_spell_id: u32,
    pub caster_target_spell_targets: u32,
    pub casters_grouped: bool,
    pub no_target_check: bool,
    pub allow_unfriendly_cross_faction_party: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MeetingStoneUseSource {
    pub area_id: u32,
    pub prevent_unfriendly_outside_instances: bool,
    pub content_tuning_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct QuestgiverUseSource {
    pub gossip_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GuardPostUseSource {
    pub creature_id: u32,
    pub charges: u32,
    pub prefer_only_if_in_line_of_sight: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SpellcasterUseSource {
    pub spell_id: u32,
    pub charges: u32,
    pub party_only: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CameraUseSource {
    pub cinematic_id: u32,
    pub event_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GooberUseSource {
    pub lock_id: u32,
    pub quest_id: u32,
    pub event_id: u32,
    pub auto_close_ms: u32,
    pub custom_anim: u32,
    pub consumable: bool,
    pub page_id: u32,
    pub spell_id: u32,
    pub linked_trap_entry: u32,
    pub gossip_id: u32,
    pub allow_multi_interact: bool,
    pub player_cast: bool,
}
