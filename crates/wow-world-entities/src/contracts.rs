use std::time::Instant;

use wow_constants::PowerType;
use wow_core::{ObjectGuid, Position};
use wow_entities::{CreatureAddonLifecycleRecordLikeCpp, MovementGeneratorType, PhaseShift};

/// Parameters for spawning nearby creatures after login.
pub struct PendingCreatureSpawn {
    pub map_id: u16,
    pub position: wow_core::Position,
    pub zone_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreatureCreateStatsLikeCpp {
    pub health: i64,
    pub max_health: i64,
    pub power_type: PowerType,
    pub power: i32,
    pub max_power: i32,
    pub base_mana: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PendingCreatureKillRewardLikeCpp {
    pub killer_guid: ObjectGuid,
    pub creature_guid: ObjectGuid,
    pub creature_entry: u32,
    pub creature_level: u8,
}

#[cfg(any(test, feature = "test-fixtures"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepresentedCreatureKillEventLikeCpp {
    KillerProc {
        attacker_guid: ObjectGuid,
        victim_guid: ObjectGuid,
    },
    TapperTargetDiesProc {
        tapper_guid: ObjectGuid,
        victim_guid: ObjectGuid,
    },
    VictimDeathProc {
        victim_guid: ObjectGuid,
    },
    DeliveredKillingBlowCriteria {
        player_guid: ObjectGuid,
        victim_guid: ObjectGuid,
        quantity: u32,
    },
    DeathStateJustDied {
        victim_guid: ObjectGuid,
    },
    ZoneScriptUnitDeath {
        unit_guid: ObjectGuid,
    },
    TapperPetKilledUnitAi {
        tapper_guid: ObjectGuid,
        pet_guid: ObjectGuid,
        victim_guid: ObjectGuid,
    },
    LootFlagsApplied {
        creature_guid: ObjectGuid,
        lootable: bool,
        can_skin: bool,
        skinnable: bool,
    },
    CreatureOnHealthDepletedAi {
        creature_guid: ObjectGuid,
        attacker_guid: ObjectGuid,
        is_kill: bool,
    },
    CreatureJustDiedAi {
        creature_guid: ObjectGuid,
        killer_guid: ObjectGuid,
    },
    ScriptMgrOnCreatureKill {
        killer_guid: ObjectGuid,
        creature_guid: ObjectGuid,
    },
    CreatureKillReputationAwarded {
        creature_guid: ObjectGuid,
        faction_id: u32,
        reputation: i32,
        spillover_only: bool,
    },
}

/// One creature aura this session applied, with the wall-clock deadline its
/// represented duration expires at.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RepresentedCreatureAuraLikeCpp {
    pub target_guid: ObjectGuid,
    pub spell_id: i32,
    pub caster_guid: ObjectGuid,
    pub slot: u8,
    pub effect_mask: u32,
    pub applied_at: Instant,
    pub duration_ms: u32,
}

#[derive(Debug, Clone)]
pub struct MaterializedCreatureSpawnLikeCpp {
    pub guid: ObjectGuid,
    pub position: Position,
    pub create_data: wow_packet::packets::update::CreatureCreateData,
    pub min_damage: u32,
    pub max_damage: u32,
    pub aggro_radius: f32,
    pub loot_id: u32,
    pub skin_loot_id: u32,
    pub gold_min: u32,
    pub gold_max: u32,
    pub respawn_delay_secs: u32,
    pub selected_equipment_id: u8,
    pub original_equipment_id: i8,
    pub script_name: String,
    pub string_id: Option<String>,
    pub addon: Option<CreatureAddonLifecycleRecordLikeCpp>,
    pub phase_use_flags: u8,
    pub phase_id: u16,
    pub phase_group_id: u32,
    pub terrain_swap_map: i32,
    pub flags_extra: u32,
    pub ground_movement_type: u8,
    pub swim_allowed: bool,
    pub flight_movement_type: u8,
    pub rooted: bool,
    pub chase_movement_type: u8,
    pub random_movement_type: u8,
    pub interaction_pause_timer_ms: u32,
    pub wander_distance: f32,
    pub default_movement_type: MovementGeneratorType,
    pub waypoint_path_id: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RepresentedSpellClickCreatureSnapshotLikeCpp {
    pub guid: ObjectGuid,
    pub entry: u32,
    pub map_id: u32,
    pub instance_id: u32,
    pub position: Position,
    pub phase_shift: PhaseShift,
    pub npc_flags: u32,
    pub faction_template_id: u32,
    pub level: u32,
    pub health: u64,
    pub max_health: u64,
    pub is_alive: bool,
    pub is_in_world: bool,
    pub is_summon: bool,
    pub owner_guid: Option<ObjectGuid>,
}
