//! Immutable GameObjectTemplate catalog data and pure C++ field projections.

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

#[cfg(test)]
mod tests;
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

impl GameObjectLootSource {
    pub const fn loot_ids_like_cpp(&self) -> [u32; 3] {
        [self.loot_id, self.personal_loot_id, self.push_loot_id]
    }

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

pub const fn ui_link_player_interaction_type_like_cpp(ui_link_type: u32) -> i32 {
    match ui_link_type {
        0 => 54,
        1 => 39,
        2 => 40,
        3 => 44,
        _ => 0,
    }
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

/// Represented subset of TrinityCore `GameObjectTemplate`, template addon and override data
/// consumed by `GameObject::Create`.
///
/// ObjectMgr lookups, zone-script entry overrides, model creation, phasing, terrain visible-map
/// setup and AddToMap are deliberately external. Callers pass values already resolved from the
/// C++ template/addon/override sources consumed by the entity lifecycle.
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
