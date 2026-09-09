//! GameObject template, loot and runtime state state definitions, part 2 of 2.
//!
//! Separated from the game_object.rs root under #636. Behaviour is preserved.

use super::*;

impl GameObjectTemplateData {
    pub const fn new(go_type: u32, data: [u32; MAX_GAMEOBJECT_DATA]) -> Self {
        Self { go_type, data }
    }

    pub const fn get_loot_id_like_cpp(&self) -> u32 {
        match self.go_type {
            GAMEOBJECT_TYPE_CHEST
            | GAMEOBJECT_TYPE_FISHING_HOLE
            | GAMEOBJECT_TYPE_GATHERING_NODE => self.data[GAMEOBJECT_DATA_CHEST_LOOT],
            _ => 0,
        }
    }

    pub const fn is_despawn_at_action_like_cpp(&self) -> bool {
        match self.go_type {
            GAMEOBJECT_TYPE_CHEST => self.data[GAMEOBJECT_DATA_CHEST_CONSUMABLE] != 0,
            GAMEOBJECT_TYPE_GOOBER => self.data[GAMEOBJECT_DATA_GOOBER_CONSUMABLE] != 0,
            _ => false,
        }
    }

    pub const fn get_condition_id1_like_cpp(&self) -> u32 {
        let index = match self.go_type {
            GAMEOBJECT_TYPE_DOOR => 7,
            GAMEOBJECT_TYPE_BUTTON => 9,
            GAMEOBJECT_TYPE_QUESTGIVER => 10,
            GAMEOBJECT_TYPE_CHEST => 17,
            GAMEOBJECT_TYPE_GENERIC => 6,
            GAMEOBJECT_TYPE_TRAP => 15,
            GAMEOBJECT_TYPE_CHAIR => 4,
            GAMEOBJECT_TYPE_SPELL_FOCUS => 8,
            GAMEOBJECT_TYPE_TEXT => 4,
            GAMEOBJECT_TYPE_GOOBER => 22,
            GAMEOBJECT_TYPE_CAMERA => 4,
            GAMEOBJECT_TYPE_RITUAL => 8,
            GAMEOBJECT_TYPE_MAILBOX => 0,
            GAMEOBJECT_TYPE_SPELLCASTER => 5,
            GAMEOBJECT_TYPE_FLAGSTAND => 8,
            GAMEOBJECT_TYPE_AURA_GENERATOR => 3,
            GAMEOBJECT_TYPE_GUILD_BANK => 0,
            GAMEOBJECT_TYPE_NEW_FLAG => 4,
            GAMEOBJECT_TYPE_ITEM_FORGE => 0,
            GAMEOBJECT_TYPE_GATHERING_NODE => 11,
            _ => return 0,
        };
        self.data[index]
    }

    pub const fn get_interact_radius_override_like_cpp(&self) -> u32 {
        let index = match self.go_type {
            GAMEOBJECT_TYPE_DOOR => 12,
            GAMEOBJECT_TYPE_BUTTON => 10,
            GAMEOBJECT_TYPE_QUESTGIVER => 12,
            GAMEOBJECT_TYPE_CHEST => 9,
            GAMEOBJECT_TYPE_BINDER => 0,
            GAMEOBJECT_TYPE_GENERIC => 9,
            GAMEOBJECT_TYPE_TRAP => 21,
            GAMEOBJECT_TYPE_CHAIR => 5,
            GAMEOBJECT_TYPE_SPELL_FOCUS => 9,
            GAMEOBJECT_TYPE_TEXT => 6,
            GAMEOBJECT_TYPE_GOOBER => 33,
            GAMEOBJECT_TYPE_AREADAMAGE => 8,
            GAMEOBJECT_TYPE_CAMERA => 5,
            GAMEOBJECT_TYPE_FISHING_NODE => 0,
            GAMEOBJECT_TYPE_RITUAL => 9,
            GAMEOBJECT_TYPE_MAILBOX => 1,
            GAMEOBJECT_TYPE_SPELLCASTER => 8,
            GAMEOBJECT_TYPE_MEETINGSTONE => 3,
            GAMEOBJECT_TYPE_FLAGSTAND => 13,
            GAMEOBJECT_TYPE_FISHING_HOLE => 5,
            GAMEOBJECT_TYPE_FLAGDROP => 10,
            GAMEOBJECT_TYPE_AURA_GENERATOR => 7,
            GAMEOBJECT_TYPE_DUNGEON_DIFFICULTY => 11,
            GAMEOBJECT_TYPE_BARBER_CHAIR => 3,
            GAMEOBJECT_TYPE_DESTRUCTIBLE_BUILDING => 27,
            GAMEOBJECT_TYPE_GUILD_BANK => 1,
            GAMEOBJECT_TYPE_NEW_FLAG => 14,
            GAMEOBJECT_TYPE_NEW_FLAG_DROP => 2,
            GAMEOBJECT_TYPE_GATHERING_NODE => 24,
            _ => return 0,
        };
        self.data[index]
    }

    pub const fn get_lock_id_like_cpp(&self) -> u32 {
        let index = match self.go_type {
            GAMEOBJECT_TYPE_DOOR => 1,
            GAMEOBJECT_TYPE_BUTTON => 1,
            GAMEOBJECT_TYPE_QUESTGIVER => 0,
            GAMEOBJECT_TYPE_CHEST => 0,
            GAMEOBJECT_TYPE_TRAP => 0,
            GAMEOBJECT_TYPE_GOOBER => 0,
            GAMEOBJECT_TYPE_AREADAMAGE => 0,
            GAMEOBJECT_TYPE_CAMERA => 0,
            GAMEOBJECT_TYPE_FLAGSTAND => 0,
            GAMEOBJECT_TYPE_FISHING_HOLE => 4,
            GAMEOBJECT_TYPE_FLAGDROP => 0,
            GAMEOBJECT_TYPE_NEW_FLAG => 0,
            GAMEOBJECT_TYPE_NEW_FLAG_DROP => 0,
            GAMEOBJECT_TYPE_GATHERING_NODE => 3,
            _ => return 0,
        };
        self.data[index]
    }

    pub const fn is_usable_mounted_like_cpp(&self) -> bool {
        let index = match self.go_type {
            GAMEOBJECT_TYPE_MAILBOX => return true,
            GAMEOBJECT_TYPE_BARBER_CHAIR => return false,
            GAMEOBJECT_TYPE_QUESTGIVER => 8,
            GAMEOBJECT_TYPE_TEXT => 3,
            GAMEOBJECT_TYPE_GOOBER => 17,
            GAMEOBJECT_TYPE_SPELLCASTER => 3,
            GAMEOBJECT_TYPE_UI_LINK => 1,
            _ => return false,
        };

        self.data[index] != 0
    }

    pub const fn get_no_damage_immune_like_cpp(&self) -> u32 {
        let index = match self.go_type {
            GAMEOBJECT_TYPE_DOOR => 3,
            GAMEOBJECT_TYPE_BUTTON => 4,
            GAMEOBJECT_TYPE_QUESTGIVER => 5,
            GAMEOBJECT_TYPE_CHEST => {
                return if self.data[22] == 0 { 1 } else { 0 };
            }
            GAMEOBJECT_TYPE_GOOBER => 11,
            GAMEOBJECT_TYPE_FLAGSTAND => 5,
            GAMEOBJECT_TYPE_FLAGDROP => 3,
            _ => return 0,
        };

        self.data[index]
    }

    pub const fn get_cooldown_like_cpp(&self) -> u32 {
        match self.go_type {
            GAMEOBJECT_TYPE_TRAP => self.data[5],
            GAMEOBJECT_TYPE_GOOBER => self.data[6],
            _ => 0,
        }
    }

    pub const fn get_auto_close_time_like_cpp(&self) -> u32 {
        match self.go_type {
            GAMEOBJECT_TYPE_DOOR | GAMEOBJECT_TYPE_BUTTON => self.data[2],
            GAMEOBJECT_TYPE_TRAP => self.data[6],
            GAMEOBJECT_TYPE_GOOBER => self.data[3],
            _ => 0,
        }
    }

    pub const fn trap_use_source_like_cpp(&self) -> Option<TrapUseSource> {
        if self.go_type != GAMEOBJECT_TYPE_TRAP {
            return None;
        }

        Some(TrapUseSource {
            radius: self.data[2],
            spell_id: self.data[3],
            charges: self.data[4],
            cooldown_secs: self.data[5],
            start_delay_secs: self.data[7],
            ignore_totems: self.data[14] != 0,
            check_all_units: self.data[20] != 0,
        })
    }

    pub const fn chair_use_source_like_cpp(&self) -> Option<ChairUseSource> {
        if self.go_type != GAMEOBJECT_TYPE_CHAIR {
            return None;
        }

        Some(ChairUseSource {
            chair_slots: self.data[0],
            chair_height: self.data[1],
            triggered_event_id: self.data[3],
        })
    }

    pub const fn barber_chair_use_source_like_cpp(&self) -> Option<BarberChairUseSource> {
        if self.go_type != GAMEOBJECT_TYPE_BARBER_CHAIR {
            return None;
        }

        Some(BarberChairUseSource {
            chair_height: self.data[0],
            sit_anim_kit: self.data[2],
            customization_scope: self.data[4],
        })
    }

    pub const fn ui_link_use_source_like_cpp(&self) -> Option<UiLinkUseSource> {
        if self.go_type != GAMEOBJECT_TYPE_UI_LINK {
            return None;
        }

        Some(UiLinkUseSource {
            ui_link_type: self.data[0],
        })
    }

    pub const fn spell_focus_use_source_like_cpp(&self) -> Option<SpellFocusUseSource> {
        match self.go_type {
            GAMEOBJECT_TYPE_SPELL_FOCUS => Some(SpellFocusUseSource {
                focus_type: self.data[GAMEOBJECT_DATA_SPELL_FOCUS_TYPE],
                radius: self.data[GAMEOBJECT_DATA_SPELL_FOCUS_RADIUS],
                linked_trap_entry: self.data[GAMEOBJECT_DATA_SPELL_FOCUS_LINKED_TRAP],
            }),
            GAMEOBJECT_TYPE_UI_LINK => Some(SpellFocusUseSource {
                focus_type: self.data[GAMEOBJECT_DATA_UI_LINK_SPELL_FOCUS_TYPE],
                radius: self.data[GAMEOBJECT_DATA_UI_LINK_SPELL_FOCUS_RADIUS],
                linked_trap_entry: 0,
            }),
            _ => None,
        }
    }

    pub const fn item_forge_use_source_like_cpp(&self) -> Option<ItemForgeUseSource> {
        if self.go_type != GAMEOBJECT_TYPE_ITEM_FORGE {
            return None;
        }

        Some(ItemForgeUseSource {
            condition_id: self.data[0],
            forge_type: self.data[5],
        })
    }

    pub const fn capture_point_use_source_like_cpp(&self) -> Option<CapturePointUseSource> {
        if self.go_type != GAMEOBJECT_TYPE_CAPTURE_POINT {
            return None;
        }

        Some(CapturePointUseSource {
            capture_time_ms: self.data[0],
            assault_broadcast_horde: self.data[4],
            capture_broadcast_horde: self.data[5],
            defended_broadcast_horde: self.data[6],
            assault_broadcast_alliance: self.data[7],
            capture_broadcast_alliance: self.data[8],
            defended_broadcast_alliance: self.data[9],
            world_state_id: self.data[10],
            contested_event_horde: self.data[11],
            capture_event_horde: self.data[12],
            defended_event_horde: self.data[13],
            contested_event_alliance: self.data[14],
            capture_event_alliance: self.data[15],
            defended_event_alliance: self.data[16],
            spell_visual_ids: [
                self.data[17],
                self.data[18],
                self.data[19],
                self.data[20],
                self.data[21],
            ],
        })
    }

    pub const fn flag_stand_use_source_like_cpp(&self) -> Option<FlagStandUseSource> {
        if self.go_type != GAMEOBJECT_TYPE_FLAGSTAND {
            return None;
        }

        Some(FlagStandUseSource {
            pickup_spell_id: self.data[1],
            return_aura_id: self.data[3],
            return_spell_id: self.data[4],
        })
    }

    pub const fn flag_drop_use_source_like_cpp(&self) -> Option<FlagDropUseSource> {
        if self.go_type != GAMEOBJECT_TYPE_FLAGDROP {
            return None;
        }

        Some(FlagDropUseSource {
            event_id: self.data[1],
            pickup_spell_id: self.data[2],
            expire_duration_ms: self.data[6],
        })
    }

    pub const fn questgiver_use_source_like_cpp(&self) -> Option<QuestgiverUseSource> {
        if self.go_type != GAMEOBJECT_TYPE_QUESTGIVER {
            return None;
        }

        Some(QuestgiverUseSource {
            gossip_id: self.data[3],
        })
    }

    pub const fn ritual_use_source_like_cpp(&self) -> Option<RitualUseSource> {
        if self.go_type != GAMEOBJECT_TYPE_RITUAL {
            return None;
        }

        Some(RitualUseSource {
            casters_required: self.data[0],
            spell_id: self.data[1],
            anim_spell_id: self.data[2],
            persistent: self.data[3] != 0,
            caster_target_spell_id: self.data[4],
            caster_target_spell_targets: self.data[5],
            casters_grouped: self.data[6] != 0,
            no_target_check: self.data[7] != 0,
            allow_unfriendly_cross_faction_party: self.data[10] != 0,
        })
    }

    pub const fn meeting_stone_use_source_like_cpp(&self) -> Option<MeetingStoneUseSource> {
        if self.go_type != GAMEOBJECT_TYPE_MEETINGSTONE {
            return None;
        }

        Some(MeetingStoneUseSource {
            area_id: self.data[2],
            prevent_unfriendly_outside_instances: self.data[4] != 0,
            content_tuning_id: 0,
        })
    }

    pub const fn new_flag_use_source_like_cpp(&self) -> Option<NewFlagUseSource> {
        if self.go_type != GAMEOBJECT_TYPE_NEW_FLAG {
            return None;
        }

        Some(NewFlagUseSource {
            pickup_spell_id: self.data[1],
            expire_duration_ms: self.data[7],
            respawn_time_ms: self.data[8],
            flag_drop_entry: self.data[9],
            exclusive_category: self.data[10] as i32,
            world_state_id: self.data[11],
            return_on_defender_interact: self.data[12] != 0,
        })
    }

    pub const fn new_flag_drop_use_source_like_cpp(&self) -> Option<NewFlagDropUseSource> {
        if self.go_type != GAMEOBJECT_TYPE_NEW_FLAG_DROP {
            return None;
        }

        Some(NewFlagDropUseSource {
            spawn_vignette_id: self.data[1],
        })
    }

    pub const fn spellcaster_use_source_like_cpp(&self) -> Option<SpellcasterUseSource> {
        if self.go_type != GAMEOBJECT_TYPE_SPELLCASTER {
            return None;
        }

        Some(SpellcasterUseSource {
            spell_id: self.data[0],
            charges: self.data[1],
            party_only: self.data[2] != 0,
        })
    }

    pub const fn guard_post_use_source_like_cpp(&self) -> Option<GuardPostUseSource> {
        if self.go_type != GAMEOBJECT_TYPE_GUARDPOST {
            return None;
        }

        Some(GuardPostUseSource {
            creature_id: self.data[0],
            charges: self.data[1],
            prefer_only_if_in_line_of_sight: self.data[2] != 0,
        })
    }

    pub const fn spell_focus_linked_trap_like_cpp(&self) -> u32 {
        if let Some(source) = self.spell_focus_use_source_like_cpp() {
            source.linked_trap_entry
        } else {
            0
        }
    }

    pub const fn camera_use_source_like_cpp(&self) -> Option<CameraUseSource> {
        if self.go_type != GAMEOBJECT_TYPE_CAMERA {
            return None;
        }

        Some(CameraUseSource {
            cinematic_id: self.data[1],
            event_id: self.data[2],
        })
    }

    pub const fn goober_use_source_like_cpp(&self) -> Option<GooberUseSource> {
        if self.go_type != GAMEOBJECT_TYPE_GOOBER {
            return None;
        }

        Some(GooberUseSource {
            lock_id: self.data[0],
            quest_id: self.data[1],
            event_id: self.data[2],
            auto_close_ms: self.data[3],
            custom_anim: self.data[4],
            consumable: self.data[GAMEOBJECT_DATA_GOOBER_CONSUMABLE] != 0,
            page_id: self.data[7],
            spell_id: self.data[10],
            linked_trap_entry: self.data[12],
            gossip_id: self.data[19],
            allow_multi_interact: self.data[20] != 0,
            player_cast: self.data[23] != 0,
        })
    }

    pub const fn chest_loot_source_like_cpp(&self) -> Option<GameObjectLootSource> {
        if self.go_type != GAMEOBJECT_TYPE_CHEST {
            return None;
        }

        Some(GameObjectLootSource {
            loot_id: self.get_loot_id_like_cpp(),
            use_group_loot_rules: self.data[GAMEOBJECT_DATA_CHEST_USE_GROUP_LOOT_RULES] != 0,
            dungeon_encounter_id: self.data[GAMEOBJECT_DATA_CHEST_DUNGEON_ENCOUNTER],
            personal_loot_id: self.data[GAMEOBJECT_DATA_CHEST_PERSONAL_LOOT],
            push_loot_id: self.data[GAMEOBJECT_DATA_CHEST_PUSH_LOOT],
            triggered_event_id: self.data[GAMEOBJECT_DATA_CHEST_TRIGGERED_EVENT],
            linked_trap_entry: self.data[GAMEOBJECT_DATA_CHEST_LINKED_TRAP],
            chest_restock_time_secs: self.data[GAMEOBJECT_DATA_CHEST_RESTOCK_TIME],
            chest_consumable: self.data[GAMEOBJECT_DATA_CHEST_CONSUMABLE] != 0,
            chest_quest_id: self.data[GAMEOBJECT_DATA_CHEST_QUEST_ID],
        })
    }

    pub const fn gathering_node_use_source_like_cpp(&self) -> Option<GatheringNodeUseSource> {
        if self.go_type != GAMEOBJECT_TYPE_GATHERING_NODE {
            return None;
        }

        Some(GatheringNodeUseSource {
            loot_id: self.get_loot_id_like_cpp(),
            despawn_delay_secs: self.data[GAMEOBJECT_DATA_GATHERING_NODE_DESPAWN_DELAY],
            triggered_event_id: self.data[GAMEOBJECT_DATA_GATHERING_NODE_TRIGGERED_EVENT],
            xp_difficulty: self.data[GAMEOBJECT_DATA_GATHERING_NODE_XP_DIFFICULTY],
            spell_id: self.data[GAMEOBJECT_DATA_GATHERING_NODE_SPELL],
            max_loots: self.data[GAMEOBJECT_DATA_GATHERING_NODE_MAX_LOOTS],
            linked_trap_entry: self.data[GAMEOBJECT_DATA_GATHERING_NODE_LINKED_TRAP],
        })
    }

    pub const fn get_linked_gameobject_entry_like_cpp(&self) -> u32 {
        match self.go_type {
            // C++ anchor: GameObjectData.h:1049-1059 `GAMEOBJECT_TYPE_BUTTON` -> `button.linkedTrap`.
            GAMEOBJECT_TYPE_BUTTON => self.data[GAMEOBJECT_DATA_BUTTON_LINKED_TRAP],
            GAMEOBJECT_TYPE_SPELL_FOCUS => self.spell_focus_linked_trap_like_cpp(),
            GAMEOBJECT_TYPE_GOOBER => self.data[12],
            GAMEOBJECT_TYPE_CHEST => self.data[GAMEOBJECT_DATA_CHEST_LINKED_TRAP],
            GAMEOBJECT_TYPE_GATHERING_NODE => self.data[GAMEOBJECT_DATA_GATHERING_NODE_LINKED_TRAP],
            _ => 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GameObjectDataValues {
    pub display_id: i32,
    pub created_by: ObjectGuid,
    pub flags: u32,
    pub faction_template: i32,
    pub level: i32,
    pub state: i8,
    pub type_id: i8,
    pub percent_health: u8,
    pub art_kit: u32,
    pub custom_param: u32,
}

impl Default for GameObjectDataValues {
    fn default() -> Self {
        Self {
            display_id: 0,
            created_by: ObjectGuid::EMPTY,
            flags: 0,
            faction_template: 0,
            level: 0,
            state: GoState::Active as i8,
            type_id: 0,
            percent_health: 0,
            art_kit: 0,
            custom_param: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GameObjectDataUpdate {
    pub mask: UpdateMask,
    pub values: GameObjectDataValues,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GameObjectValuesUpdate {
    pub changed_object_type_mask: u32,
    pub object_data: Option<ObjectDataUpdate>,
    pub game_object_data: Option<GameObjectDataUpdate>,
}

impl GameObjectValuesUpdate {
    pub const fn has_data(&self) -> bool {
        self.changed_object_type_mask != 0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GameObject {
    pub(super) world: WorldObject,
    pub(super) data: GameObjectDataValues,
    pub(super) game_object_data_changes: UpdateMask,
    pub(super) spell_id: u32,
    pub(super) respawn_time: i64,
    pub(super) respawn_delay_time: u32,
    pub(super) despawn_delay: u32,
    pub(super) despawn_respawn_time: u32,
    pub(super) restock_time: i64,
    pub(super) loot_state: LootState,
    pub(super) loot_state_unit_guid: ObjectGuid,
    /// Monotonic identity for one C++ `GameObject::loot` lifetime. `ClearLoot`
    /// advances it before a restock can generate another pool, so async work
    /// captured for the previous use cannot install into the next use of the
    /// same spawn GUID.
    pub(super) loot_lifecycle_revision: u64,
    pub(super) loot_authority: OwnedLootAuthority,
    pub(super) shared_loot: Option<GameObjectOwnedLoot>,
    pub(super) personal_loot: HashMap<ObjectGuid, GameObjectOwnedLoot>,
    pub(super) unique_users: HashSet<ObjectGuid>,
    pub(super) spawned_by_default: bool,
    pub(super) use_times: u32,
    pub(super) cooldown_time: i64,
    pub(super) prev_go_state: GoState,
    pub(super) packed_rotation: i64,
    pub(super) local_rotation: [f32; 4],
    pub(super) spawn_id: u64,
    pub(super) loot_mode: u16,
    pub(super) respawn_compatibility_mode: bool,
    pub(super) anim_kit_id: u16,
    pub(super) world_effect_id: u32,
    pub(super) lifecycle_string_id: String,
    pub(super) linked_trap_guid: ObjectGuid,
    pub(super) stationary_position: Position,
    pub(super) go_anim_progress_like_cpp: u8,
    pub(super) represented_baseline_flags_like_cpp: Option<u32>,
    /// Resolved C++ `GetGOInfo()->chest` source carried only when this entity was
    /// constructed or explicitly seeded with a CHEST template source.
    ///
    /// This is represented template evidence for bounded `GameObject::Update`
    /// branches only; it is not a full live `GameObjectTemplate`/ObjectMgr owner.
    pub(super) chest_loot_source_like_cpp: Option<GameObjectLootSource>,
    /// Resolved C++ `GetGOInfo()->goober` source carried only when this entity was
    /// constructed or explicitly seeded with a GOOBER template source.
    ///
    /// This is represented template evidence for bounded `GameObject::Update`
    /// branches only; it is not a full live `GameObjectTemplate`/ObjectMgr owner.
    pub(super) goober_use_source_like_cpp: Option<GooberUseSource>,
    /// Resolved C++ `GetGOInfo()->GetSpellFocusType/Radius` source carried
    /// only when this entity was constructed or explicitly seeded with a
    /// SPELL_FOCUS/UI_LINK template source.
    ///
    /// This is represented template evidence for bounded `Spell::SearchSpellFocus`
    /// only; it is not a full live `GameObjectTemplate`/ObjectMgr owner.
    pub(super) spell_focus_use_source_like_cpp: Option<SpellFocusUseSource>,
    /// Explicit represented evidence for TrinityCore `GameObject::m_model != nullptr`.
    ///
    /// This flag exists only so map-owned AddToWorld/RemoveFromWorld seams can decide whether
    /// to register a represented `GameObjectModel` key in `Map`'s represented DynamicMapTree.
    /// It is not a real `GameObjectModel`, not model geometry, not `GO_FLAG_MAP_OBJECT`, not
    /// `EnableCollision`, and not DB/model-store hydration. Callers/tests must set it explicitly;
    /// Rust must not infer it from display id, template, or gameobject type until real
    /// `GameObjectModel::Create`/DB2 model runtime exists.
    pub(super) represented_gameobject_model_like_cpp: bool,
    /// Explicit represented evidence for TrinityCore `m_model && m_model->isMapObject()`.
    ///
    /// This is set only by a caller/test that represents `GameObject::CreateModel()` output.
    /// It may be true only when `represented_gameobject_model_like_cpp` is true, and it is the
    /// only represented source that toggles `GO_FLAG_MAP_OBJECT`.
    pub(super) represented_gameobject_model_is_map_object_like_cpp: bool,
    /// Last represented `m_model->enableCollision(enable)` value.
    ///
    /// `None` means the C++ call has not been represented or returned early because there was no
    /// represented model evidence. This is not real collision, BIH, LOS, intersection or height
    /// runtime.
    pub(super) represented_gameobject_model_collision_enabled_like_cpp: Option<bool>,
    /// Explicit represented evidence for TrinityCore `GameObject::m_goData != nullptr`.
    ///
    /// This is only the bounded `GameObjectData` presence needed by `SaveRespawnTime()`; it is not
    /// full ObjectMgr/DB metadata and must not be inferred from `spawn_id` alone.
    pub(super) represented_gameobject_data_present_like_cpp: bool,
    pub(super) grid_unload_cleanup_before_delete_count: u32,
    pub(super) grid_unload_delete_requested: bool,
    pub(super) grid_unload_respawn_relocation_requested: bool,
}

pub(super) fn pack_gameobject_local_rotation(rotation: [f32; 4]) -> i64 {
    const PACK_YZ: i64 = 1 << 20;
    const PACK_X: i64 = PACK_YZ << 1;
    const PACK_YZ_MASK: i64 = (PACK_YZ << 1) - 1;
    const PACK_X_MASK: i64 = (PACK_X << 1) - 1;

    let dot = rotation[0] * rotation[0]
        + rotation[1] * rotation[1]
        + rotation[2] * rotation[2]
        + rotation[3] * rotation[3];
    if dot <= f32::EPSILON {
        return 0;
    }

    let inv_len = 1.0 / dot.sqrt();
    let rx = rotation[0] * inv_len;
    let ry = rotation[1] * inv_len;
    let rz = rotation[2] * inv_len;
    let rw = rotation[3] * inv_len;
    let w_sign = if rw >= 0.0 { 1 } else { -1 };

    let x = ((rx * PACK_X as f32) as i32 as i64) * i64::from(w_sign) & PACK_X_MASK;
    let y = ((ry * PACK_YZ as f32) as i32 as i64) * i64::from(w_sign) & PACK_YZ_MASK;
    let z = ((rz * PACK_YZ as f32) as i32 as i64) * i64::from(w_sign) & PACK_YZ_MASK;

    z | (y << 21) | (x << 42)
}

impl Default for GameObject {
    fn default() -> Self {
        Self::new()
    }
}
