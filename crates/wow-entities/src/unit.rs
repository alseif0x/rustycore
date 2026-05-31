use wow_constants::{
    DeathState, Gender, PowerType, SpellState, TypeId, TypeMask, UnitFlags, UnitFlags2,
    UnitPvpFlags, UnitStandStateType, UnitState, WeaponAttackType,
};
use wow_core::ObjectGuid;

use crate::{
    CurrentSpellRef, CurrentSpellSlot, MotionMasterAddToWorldOutcomeLikeCpp, ObjectDataUpdate,
    UnitSubsystems, UpdateMask, VehicleKitRemoveOutcomeLikeCpp, VisibleItemValues, WorldObject,
    update_fields::{TYPEID_UNIT, UNIT_DATA_BITS},
};

pub const MAX_MOVE_TYPE: usize = 9;
pub const MAX_ATTACK: usize = 3;
pub const MAX_POWERS: usize = 26;
pub const MAX_POWERS_PER_CLASS: usize = 10;
pub const BASE_MINDAMAGE: f32 = 1.0;
pub const BASE_MAXDAMAGE: f32 = 2.0;
pub const DEFAULT_PLAYER_DISPLAY_SCALE: f32 = 1.0;
pub const AUTO_SHOT_SPELL_ID: u32 = 75;
pub const SPELL_AURA_MOD_UNATTACKABLE_LIKE_CPP: i32 = 93;
pub const SPELL_AURA_DISABLE_ATTACKING_EXCEPT_ABILITIES_LIKE_CPP: i32 = 264;
pub const SPELL_AURA_MOD_STALKED_LIKE_CPP: i32 = 68;
pub const SPELL_AURA_INTERRUPT_FLAG_ATTACKING_LIKE_CPP: u32 = 0x0000_1000;
pub const SPELL_AURA_INTERRUPT_FLAG_ENTER_WORLD_LIKE_CPP: u32 = 0x0040_0000;
pub const MAX_VISIBILITY_AURA_TYPES_LIKE_CPP: usize = 38;
pub const MAX_PLAYER_STEALTH_DETECT_RANGE_LIKE_CPP: f32 = 30.0;
pub const GHOST_VISIBILITY_ALIVE_LIKE_CPP: u32 = 0x1;

pub const UNIT_DATA_PARENT_BIT: usize = 0;
pub const UNIT_DATA_HEALTH_BIT: usize = 5;
pub const UNIT_DATA_MAX_HEALTH_BIT: usize = 6;
pub const UNIT_DATA_DISPLAY_ID_BIT: usize = 7;
pub const UNIT_DATA_DISPLAY_POWER_BIT: usize = 28;
pub const UNIT_DATA_LEVEL_BIT: usize = 30;
pub const UNIT_DATA_FACTION_TEMPLATE_BIT: usize = 40;
pub const UNIT_DATA_FLAGS_BIT: usize = 41;
pub const UNIT_DATA_FLAGS2_BIT: usize = 42;
pub const UNIT_DATA_FLAGS3_BIT: usize = 43;
pub const UNIT_DATA_BOUNDING_RADIUS_BIT: usize = 46;
pub const UNIT_DATA_COMBAT_REACH_BIT: usize = 47;
pub const UNIT_DATA_DISPLAY_SCALE_BIT: usize = 48;
pub const UNIT_DATA_NATIVE_DISPLAY_ID_BIT: usize = 49;
pub const UNIT_DATA_NATIVE_DISPLAY_SCALE_BIT: usize = 50;
pub const UNIT_DATA_MOUNT_DISPLAY_ID_BIT: usize = 51;
pub const UNIT_DATA_STAND_STATE_BIT: usize = 56;
pub const UNIT_DATA_PVP_FLAGS_BIT: usize = 78;
pub const UNIT_DATA_TARGET_BIT: usize = 19;
pub const UNIT_DATA_RACE_BIT: usize = 24;
pub const UNIT_DATA_CLASS_ID_BIT: usize = 25;
pub const UNIT_DATA_PLAYER_CLASS_ID_BIT: usize = 26;
pub const UNIT_DATA_SEX_BIT: usize = 27;
pub const UNIT_DATA_NPC_FLAGS_PARENT_BIT: usize = 113;
pub const UNIT_DATA_NPC_FLAGS_FIRST_BIT: usize = 114;
pub const UNIT_DATA_POWER_PARENT_BIT: usize = 116;
pub const UNIT_DATA_POWER_FIRST_BIT: usize = 137;
pub const UNIT_DATA_MAX_POWER_FIRST_BIT: usize = 147;
pub const UNIT_DATA_VIRTUAL_ITEMS_PARENT_BIT: usize = 167;
pub const UNIT_DATA_VIRTUAL_ITEMS_FIRST_BIT: usize = 168;

pub const BASE_MOVE_SPEED: [f32; MAX_MOVE_TYPE] =
    [2.5, 7.0, 4.5, 4.722222, 2.5, 3.141594, 7.0, 4.5, 3.14];

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UnitDataValues {
    pub health: u64,
    pub max_health: u64,
    pub display_id: i32,
    pub target: ObjectGuid,
    pub race: u8,
    pub class_id: u8,
    pub player_class_id: u8,
    pub sex: u8,
    pub display_power: u8,
    pub level: i32,
    pub faction_template: i32,
    pub flags: u32,
    pub flags2: u32,
    pub flags3: u32,
    pub bounding_radius: f32,
    pub combat_reach: f32,
    pub display_scale: f32,
    pub native_display_id: i32,
    pub native_display_scale: f32,
    pub mount_display_id: i32,
    pub stand_state: u8,
    pub pvp_flags: u8,
    pub npc_flags: [u32; 2],
    pub power: [i32; MAX_POWERS_PER_CLASS],
    pub max_power: [i32; MAX_POWERS_PER_CLASS],
    pub virtual_items: [VisibleItemValues; MAX_ATTACK],
}

impl Default for UnitDataValues {
    fn default() -> Self {
        Self {
            health: 0,
            max_health: 0,
            display_id: 0,
            target: ObjectGuid::EMPTY,
            race: 0,
            class_id: 0,
            player_class_id: 0,
            sex: Gender::Male as u8,
            display_power: PowerType::Mana as u8,
            level: 0,
            faction_template: 0,
            flags: 0,
            flags2: 0,
            flags3: 0,
            bounding_radius: 0.0,
            combat_reach: 0.0,
            display_scale: 0.0,
            native_display_id: 0,
            native_display_scale: 0.0,
            mount_display_id: 0,
            stand_state: UnitStandStateType::Stand as u8,
            pvp_flags: 0,
            npc_flags: [0; 2],
            power: [0; MAX_POWERS_PER_CLASS],
            max_power: [0; MAX_POWERS_PER_CLASS],
            virtual_items: [VisibleItemValues::default(); MAX_ATTACK],
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct UnitDataUpdate {
    pub mask: UpdateMask,
    pub values: UnitDataValues,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UnitValuesUpdate {
    pub changed_object_type_mask: u32,
    pub object_data: Option<ObjectDataUpdate>,
    pub unit_data: Option<UnitDataUpdate>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnitRemoveFromWorldOutcomeLikeCpp {
    pub guid: ObjectGuid,
    pub was_in_world: bool,
    pub during_remove_entered: bool,
    pub ai_on_despawn_represented: bool,
    pub vehicle_remove: Option<VehicleKitRemoveOutcomeLikeCpp>,
    pub leave_world_cleanup_represented: bool,
    pub world_object_removed: bool,
    pub during_remove_cleared: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitAddToWorldOutcomeLikeCpp {
    pub guid: ObjectGuid,
    pub world_object_added: bool,
    pub is_in_world_after: bool,
    pub motion_master_add_to_world: MotionMasterAddToWorldOutcomeLikeCpp,
    pub removed_enter_world_auras: Vec<crate::AppliedAuraRef>,
    pub aura_interrupt_flags_enter_world: u32,
}

impl UnitValuesUpdate {
    pub const fn has_data(&self) -> bool {
        self.changed_object_type_mask != 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnitAttackStartOutcome {
    NewTarget { previous: Option<ObjectGuid> },
    MeleeStartedSameTarget,
    MeleeStoppedSameTarget,
    NoChangeSameTarget,
    InvalidSelfTarget,
    InvalidDeadAttacker,
    InvalidDeadVictim,
    InvalidVictimNotInWorld,
    InvalidMountedAttacker,
    InvalidAttackerEvading,
    InvalidVictimGameMaster,
    InvalidVictimEvading,
    InvalidAttackTarget,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnitAttackStopOutcome {
    Stopped { victim: ObjectGuid },
    NoVictim,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnitSharedVisionSetWorldObjectRequestLikeCpp {
    pub unit_guid: ObjectGuid,
    pub on: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnitSharedVisionUpdateOutcomeLikeCpp {
    pub player_guid: ObjectGuid,
    pub inserted_or_removed: bool,
    pub set_world_object: Option<UnitSharedVisionSetWorldObjectRequestLikeCpp>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UnitAttackContextLikeCpp {
    pub attacker_is_mounted_player: bool,
    pub attacker_is_evading_creature: bool,
    pub victim_is_game_master_player: bool,
    pub victim_is_evading_creature: bool,
    pub controlled_creatures_with_ai: Vec<ObjectGuid>,
    pub visibility_represented: bool,
    pub attacker_can_see_or_detect_target: bool,
    pub victim_unit_state: u32,
    pub attacker_unit_flags: u32,
    pub victim_unit_flags: u32,
    pub attacker_is_player_uber: bool,
    pub relation_represented: bool,
    pub attacker_is_hostile_to_victim: bool,
    pub victim_is_hostile_to_attacker: bool,
    pub attacker_is_friendly_to_victim: bool,
    pub victim_is_friendly_to_attacker: bool,
    pub attacker_has_affecting_player: bool,
    pub victim_has_affecting_player: bool,
    pub victim_is_pet: bool,
    pub victim_affecting_player_is_mounted: bool,
    pub player_creature_reputation_represented: bool,
    pub creature_is_contested_guard: bool,
    pub player_has_contested_pvp_flag: bool,
    pub creature_has_forced_reputation_rank: bool,
    pub player_at_war_with_creature_faction: bool,
    pub player_player_duel_in_progress: bool,
    pub sanctuary_represented: bool,
    pub attacker_in_sanctuary: bool,
    pub victim_in_sanctuary: bool,
    pub pvp_represented: bool,
    pub victim_is_pvp: bool,
    pub attacker_is_ffa_pvp: bool,
    pub victim_is_ffa_pvp: bool,
    pub attacker_has_pvp_unk1_flag: bool,
    pub victim_has_pvp_unk1_flag: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitVisibilityDetectionStateLikeCpp {
    never_visible_for_seer: bool,
    seer_can_never_see_target: bool,
    always_visible_for_seer: bool,
    seer_can_always_see_target: bool,
    target_owner_group_visible_for_seer: bool,
    always_detectable_for_seer: bool,
    invisible_due_to_despawn: bool,
    private_object_owner: ObjectGuid,
    seer_private_object_owner: ObjectGuid,
    seer_group_visible_for_private_owner: bool,
    object_id_visibility_conditions_met: bool,
    server_side_visibility_gm: u32,
    server_side_visibility_detect_gm: u32,
    server_side_visibility_ghost: u32,
    server_side_visibility_detect_ghost: u32,
    ghost_visible_to_seer_by_group: bool,
    seer_can_always_see_target_guid: ObjectGuid,
    invisibility_flags: u64,
    invisibility: [i32; MAX_VISIBILITY_AURA_TYPES_LIKE_CPP],
    invisibility_detect_flags: u64,
    invisibility_detect: [i32; MAX_VISIBILITY_AURA_TYPES_LIKE_CPP],
    stealth_flags: u64,
    stealth: [i32; MAX_VISIBILITY_AURA_TYPES_LIKE_CPP],
    stealth_detect: [i32; MAX_VISIBILITY_AURA_TYPES_LIKE_CPP],
}

impl Default for UnitVisibilityDetectionStateLikeCpp {
    fn default() -> Self {
        Self {
            never_visible_for_seer: false,
            seer_can_never_see_target: false,
            always_visible_for_seer: false,
            seer_can_always_see_target: false,
            target_owner_group_visible_for_seer: false,
            always_detectable_for_seer: false,
            invisible_due_to_despawn: false,
            private_object_owner: ObjectGuid::EMPTY,
            seer_private_object_owner: ObjectGuid::EMPTY,
            seer_group_visible_for_private_owner: false,
            object_id_visibility_conditions_met: true,
            server_side_visibility_gm: 0,
            server_side_visibility_detect_gm: 0,
            server_side_visibility_ghost: GHOST_VISIBILITY_ALIVE_LIKE_CPP,
            server_side_visibility_detect_ghost: GHOST_VISIBILITY_ALIVE_LIKE_CPP,
            ghost_visible_to_seer_by_group: false,
            seer_can_always_see_target_guid: ObjectGuid::EMPTY,
            invisibility_flags: 0,
            invisibility: [0; MAX_VISIBILITY_AURA_TYPES_LIKE_CPP],
            invisibility_detect_flags: 0,
            invisibility_detect: [0; MAX_VISIBILITY_AURA_TYPES_LIKE_CPP],
            stealth_flags: 0,
            stealth: [0; MAX_VISIBILITY_AURA_TYPES_LIKE_CPP],
            stealth_detect: [0; MAX_VISIBILITY_AURA_TYPES_LIKE_CPP],
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Unit {
    world: WorldObject,
    data: UnitDataValues,
    unit_data_changes: UpdateMask,
    death_state: DeathState,
    unit_state: u32,
    base_attack_speed: [u32; MAX_ATTACK],
    mod_attack_speed_pct: [f32; MAX_ATTACK],
    attack_timer: [u32; MAX_ATTACK],
    weapon_damage: [[f32; 2]; MAX_ATTACK],
    can_dual_wield: bool,
    emote_state: u32,
    speed_rate: [f32; MAX_MOVE_TYPE],
    power_index: [Option<usize>; MAX_POWERS],
    visibility_detection: UnitVisibilityDetectionStateLikeCpp,
    subsystems: UnitSubsystems,
}

impl Unit {
    pub fn new(is_world_object: bool) -> Self {
        let mut world = WorldObject::new(
            is_world_object,
            TypeId::Unit,
            TypeMask::OBJECT | TypeMask::UNIT,
        );
        world
            .object_mut()
            .create_flags_mut()
            .insert(crate::CreateObjectFlags::MOVEMENT_UPDATE);

        let mut unit = Self {
            world,
            data: UnitDataValues::default(),
            unit_data_changes: UpdateMask::new(UNIT_DATA_BITS),
            death_state: DeathState::Alive,
            unit_state: 0,
            base_attack_speed: [0; MAX_ATTACK],
            mod_attack_speed_pct: [1.0; MAX_ATTACK],
            attack_timer: [0; MAX_ATTACK],
            weapon_damage: [[BASE_MINDAMAGE, BASE_MAXDAMAGE]; MAX_ATTACK],
            can_dual_wield: false,
            emote_state: 0,
            speed_rate: [1.0; MAX_MOVE_TYPE],
            power_index: [None; MAX_POWERS],
            visibility_detection: UnitVisibilityDetectionStateLikeCpp::default(),
            subsystems: UnitSubsystems::default(),
        };
        unit.set_power_index(PowerType::Mana, Some(0));
        unit
    }

    pub const fn world(&self) -> &WorldObject {
        &self.world
    }

    pub fn world_mut(&mut self) -> &mut WorldObject {
        &mut self.world
    }

    /// Bounded represented seam for C++ `Unit::AddToWorld()`
    /// (`Unit.cpp:9471-9477`). This preserves the local statement order
    /// `WorldObject::AddToWorld()`, `MotionMaster::AddToWorld()`, and
    /// `RemoveAurasWithInterruptFlags(EnterWorld)`. It does not execute aura
    /// scripts/procs/update masks/packets or real MotionMaster pathing/runtime
    /// beyond the represented local helper.
    pub fn add_to_world_like_cpp(&mut self) -> UnitAddToWorldOutcomeLikeCpp {
        let guid = self.world().object().guid();
        self.world_mut().object_mut().add_to_world();
        let motion_master_add_to_world = self.subsystems_mut().motion.add_to_world_like_cpp();
        let removed_enter_world_auras = self
            .subsystems_mut()
            .auras
            .remove_interruptible_auras(SPELL_AURA_INTERRUPT_FLAG_ENTER_WORLD_LIKE_CPP, 0);

        UnitAddToWorldOutcomeLikeCpp {
            guid,
            world_object_added: true,
            is_in_world_after: self.world().object().is_in_world(),
            motion_master_add_to_world,
            removed_enter_world_auras,
            aura_interrupt_flags_enter_world: SPELL_AURA_INTERRUPT_FLAG_ENTER_WORLD_LIKE_CPP,
        }
    }

    /// Bounded represented seam for C++ `Unit::RemoveFromWorld()`
    /// (`Unit.cpp:9479-9533`). This preserves the `IsInWorld()` guard and the
    /// local ordering around `RemoveVehicleKit(true)` and
    /// `WorldObject::RemoveFromWorld()` without overclaiming AI, aura,
    /// control, owned-object, follower, totem, ObjectAccessor, packet, or DB
    /// cleanup runtime.
    pub fn remove_from_world_like_cpp(&mut self) -> Option<UnitRemoveFromWorldOutcomeLikeCpp> {
        let guid = self.world().object().guid();
        if !self.world().object().is_in_world() {
            return None;
        }

        let vehicle_remove = {
            let remove = self
                .subsystems_mut()
                .vehicle
                .remove_vehicle_kit_like_cpp(true);
            remove.had_kit.then_some(remove)
        };

        self.world_mut().object_mut().remove_from_world();

        Some(UnitRemoveFromWorldOutcomeLikeCpp {
            guid,
            was_in_world: true,
            during_remove_entered: true,
            ai_on_despawn_represented: false,
            vehicle_remove,
            leave_world_cleanup_represented: false,
            world_object_removed: !self.world().object().is_in_world(),
            during_remove_cleared: true,
        })
    }

    pub fn add_player_to_vision_like_cpp(
        &mut self,
        player_guid: ObjectGuid,
    ) -> UnitSharedVisionUpdateOutcomeLikeCpp {
        let was_empty = !self.subsystems.control.has_shared_vision();
        let set_world_object = if was_empty {
            let unit_guid = self.world().object().guid();
            self.world_mut().set_active(true);
            Some(UnitSharedVisionSetWorldObjectRequestLikeCpp {
                unit_guid,
                on: true,
            })
        } else {
            None
        };
        let inserted_or_removed = self.subsystems.control.add_shared_vision(player_guid);

        UnitSharedVisionUpdateOutcomeLikeCpp {
            player_guid,
            inserted_or_removed,
            set_world_object,
        }
    }

    pub fn remove_player_from_vision_like_cpp(
        &mut self,
        player_guid: ObjectGuid,
    ) -> UnitSharedVisionUpdateOutcomeLikeCpp {
        let inserted_or_removed = self.subsystems.control.remove_shared_vision(player_guid);
        let set_world_object = if self.subsystems.control.has_shared_vision() {
            None
        } else {
            let unit_guid = self.world().object().guid();
            self.world_mut().set_active(false);
            Some(UnitSharedVisionSetWorldObjectRequestLikeCpp {
                unit_guid,
                on: false,
            })
        };

        UnitSharedVisionUpdateOutcomeLikeCpp {
            player_guid,
            inserted_or_removed,
            set_world_object,
        }
    }

    pub const fn collision_height_like_cpp(&self) -> f32 {
        self.world.collision_height_like_cpp()
    }

    pub fn set_collision_height_like_cpp(&mut self, height: f32) {
        self.world.set_collision_height_like_cpp(height);
    }

    pub const fn visibility_detection_like_cpp(&self) -> &UnitVisibilityDetectionStateLikeCpp {
        &self.visibility_detection
    }

    pub fn replace_visibility_detection_like_cpp(
        &mut self,
        state: UnitVisibilityDetectionStateLikeCpp,
    ) {
        self.visibility_detection = state;
    }

    pub fn set_never_visible_for_seer_like_cpp(&mut self, never_visible: bool) {
        self.visibility_detection.never_visible_for_seer = never_visible;
    }

    pub fn set_seer_can_never_see_target_like_cpp(&mut self, can_never_see: bool) {
        self.visibility_detection.seer_can_never_see_target = can_never_see;
    }

    pub fn set_always_visible_for_seer_like_cpp(&mut self, always_visible: bool) {
        self.visibility_detection.always_visible_for_seer = always_visible;
    }

    pub fn set_seer_can_always_see_target_like_cpp(&mut self, can_always_see: bool) {
        self.visibility_detection.seer_can_always_see_target = can_always_see;
    }

    pub fn set_target_owner_group_visible_for_seer_like_cpp(&mut self, visible: bool) {
        self.visibility_detection
            .target_owner_group_visible_for_seer = visible;
    }

    pub fn set_seer_can_always_see_target_guid_like_cpp(&mut self, guid: ObjectGuid) {
        self.visibility_detection.seer_can_always_see_target_guid = guid;
    }

    pub fn set_always_detectable_for_seer_like_cpp(&mut self, always_detectable: bool) {
        self.visibility_detection.always_detectable_for_seer = always_detectable;
    }

    pub fn set_invisible_due_to_despawn_like_cpp(&mut self, invisible_due_to_despawn: bool) {
        self.visibility_detection.invisible_due_to_despawn = invisible_due_to_despawn;
    }

    pub fn set_private_object_owner_like_cpp(&mut self, owner: ObjectGuid) {
        self.visibility_detection.private_object_owner = owner;
    }

    pub const fn private_object_owner_like_cpp(&self) -> ObjectGuid {
        self.visibility_detection.private_object_owner
    }

    pub fn set_seer_private_object_owner_like_cpp(&mut self, owner: ObjectGuid) {
        self.visibility_detection.seer_private_object_owner = owner;
    }

    pub fn set_seer_group_visible_for_private_owner_like_cpp(&mut self, visible: bool) {
        self.visibility_detection
            .seer_group_visible_for_private_owner = visible;
    }

    pub fn set_object_id_visibility_conditions_met_like_cpp(&mut self, met: bool) {
        self.visibility_detection
            .object_id_visibility_conditions_met = met;
    }

    pub fn set_server_side_gm_visibility_like_cpp(&mut self, visibility: u32) {
        self.visibility_detection.server_side_visibility_gm = visibility;
    }

    pub fn set_server_side_gm_visibility_detect_like_cpp(&mut self, detect: u32) {
        self.visibility_detection.server_side_visibility_detect_gm = detect;
    }

    pub fn set_server_side_ghost_visibility_like_cpp(&mut self, visibility: u32) {
        self.visibility_detection.server_side_visibility_ghost =
            visibility & (GHOST_VISIBILITY_ALIVE_LIKE_CPP | 0x2);
    }

    pub fn set_server_side_ghost_visibility_detect_like_cpp(&mut self, detect: u32) {
        self.visibility_detection
            .server_side_visibility_detect_ghost = detect & (GHOST_VISIBILITY_ALIVE_LIKE_CPP | 0x2);
    }

    pub fn set_ghost_visible_to_seer_by_group_like_cpp(&mut self, visible: bool) {
        self.visibility_detection.ghost_visible_to_seer_by_group = visible;
    }

    pub fn set_invisibility_like_cpp(&mut self, aura_type: usize, value: i32) {
        if aura_type >= MAX_VISIBILITY_AURA_TYPES_LIKE_CPP {
            return;
        }
        let flag = 1_u64 << aura_type;
        self.visibility_detection.invisibility[aura_type] = value;
        if value > 0 {
            self.visibility_detection.invisibility_flags |= flag;
        } else {
            self.visibility_detection.invisibility_flags &= !flag;
        }
    }

    pub fn set_invisibility_detect_like_cpp(&mut self, aura_type: usize, value: i32) {
        if aura_type >= MAX_VISIBILITY_AURA_TYPES_LIKE_CPP {
            return;
        }
        let flag = 1_u64 << aura_type;
        self.visibility_detection.invisibility_detect[aura_type] = value;
        if value > 0 {
            self.visibility_detection.invisibility_detect_flags |= flag;
        } else {
            self.visibility_detection.invisibility_detect_flags &= !flag;
        }
    }

    pub fn set_stealth_like_cpp(&mut self, aura_type: usize, value: i32) {
        if aura_type >= MAX_VISIBILITY_AURA_TYPES_LIKE_CPP {
            return;
        }
        let flag = 1_u64 << aura_type;
        self.visibility_detection.stealth[aura_type] = value;
        if value > 0 {
            self.visibility_detection.stealth_flags |= flag;
        } else {
            self.visibility_detection.stealth_flags &= !flag;
        }
    }

    pub fn set_stealth_detect_like_cpp(&mut self, aura_type: usize, value: i32) {
        if aura_type < MAX_VISIBILITY_AURA_TYPES_LIKE_CPP {
            self.visibility_detection.stealth_detect[aura_type] = value;
        }
    }

    pub const fn has_stealth_aura_like_cpp(&self) -> bool {
        self.visibility_detection.stealth_flags != 0
    }

    pub fn can_detect_invisibility_of_like_cpp(&self, target: &Self) -> bool {
        let target_flags = target.visibility_detection.invisibility_flags;
        if target_flags == 0 {
            return true;
        }
        if target_flags & self.visibility_detection.invisibility_detect_flags != target_flags {
            return false;
        }

        for aura_type in 0..MAX_VISIBILITY_AURA_TYPES_LIKE_CPP {
            let flag = 1_u64 << aura_type;
            if target_flags & flag == 0 {
                continue;
            }
            if self.visibility_detection.invisibility_detect[aura_type]
                < target.visibility_detection.invisibility[aura_type]
            {
                return false;
            }
        }
        true
    }

    pub fn can_detect_stealth_of_like_cpp(
        &self,
        target: &Self,
        seer_is_player: bool,
        check_alert: bool,
    ) -> bool {
        let target_flags = target.visibility_detection.stealth_flags;
        if target_flags == 0 {
            return true;
        }

        let distance = self.world.exact_distance(&target.world);
        let combat_reach = self.data.combat_reach.max(0.0);
        if distance < combat_reach {
            return true;
        }
        if !self
            .world
            .has_in_arc(std::f32::consts::PI, &target.world, 2.0)
        {
            return false;
        }

        for aura_type in 0..MAX_VISIBILITY_AURA_TYPES_LIKE_CPP {
            let flag = 1_u64 << aura_type;
            if target_flags & flag == 0 {
                continue;
            }

            let level = self.data.level.max(1);
            let detection_value =
                30 + (level - 1) * 5 + self.visibility_detection.stealth_detect[aura_type]
                    - target.visibility_detection.stealth[aura_type];
            let mut visibility_range = detection_value as f32 * 0.3 + combat_reach;
            if seer_is_player {
                visibility_range = visibility_range.min(MAX_PLAYER_STEALTH_DETECT_RANGE_LIKE_CPP);
            }
            if check_alert {
                visibility_range += visibility_range * 0.08 + 1.5;
            }
            if distance > visibility_range {
                return false;
            }
        }
        true
    }

    pub fn can_see_or_detect_unit_like_cpp(
        &self,
        target: &Self,
        implicit_detect: bool,
        seer_is_player: bool,
        check_alert: bool,
    ) -> bool {
        let seer_guid = self.world.object().guid();
        if !seer_guid.is_empty() && seer_guid == target.world.object().guid() {
            return true;
        }
        if target.visibility_detection.never_visible_for_seer
            || self.visibility_detection.seer_can_never_see_target
            || (self.world.has_current_map()
                && target.world.has_current_map()
                && !self.world.is_in_map(&target.world))
            || !self.world.in_same_phase(&target.world)
        {
            return false;
        }
        if target.visibility_detection.always_visible_for_seer
            || self.visibility_detection.seer_can_always_see_target
            || target
                .subsystems
                .control
                .charmer_or_owner_guid()
                .is_some_and(|owner_guid| owner_guid == seer_guid)
            || target
                .visibility_detection
                .target_owner_group_visible_for_seer
            || (!self
                .visibility_detection
                .seer_can_always_see_target_guid
                .is_empty()
                && self.visibility_detection.seer_can_always_see_target_guid
                    == target.world.object().guid())
        {
            return true;
        }

        let private_owner = target.visibility_detection.private_object_owner;
        if !private_owner.is_empty()
            && private_owner != self.world.object().guid()
            && private_owner != self.visibility_detection.seer_private_object_owner
            && !self
                .visibility_detection
                .seer_group_visible_for_private_owner
        {
            return false;
        }

        if target
            .world
            .smooth_phasing_like_cpp()
            .is_some_and(|smooth_phasing| {
                smooth_phasing.is_being_replaced_for_seer_like_cpp(seer_guid)
            })
        {
            return false;
        }

        if private_owner.is_empty()
            && !target
                .visibility_detection
                .object_id_visibility_conditions_met
        {
            return false;
        }

        let gm_visibility = target.visibility_detection.server_side_visibility_gm;
        if gm_visibility == 0 {
            if self.visibility_detection.server_side_visibility_detect_gm != 0 {
                return true;
            }
        } else {
            return self.visibility_detection.server_side_visibility_detect_gm >= gm_visibility;
        }

        if target.visibility_detection.server_side_visibility_ghost
            & self
                .visibility_detection
                .server_side_visibility_detect_ghost
            == 0
            && !(seer_is_player && target.visibility_detection.ghost_visible_to_seer_by_group)
        {
            return false;
        }
        if target.visibility_detection.invisible_due_to_despawn {
            return false;
        }
        if target.visibility_detection.always_detectable_for_seer
            || target
                .subsystems
                .auras
                .has_aura_type_with_caster_like_cpp(SPELL_AURA_MOD_STALKED_LIKE_CPP, seer_guid)
        {
            return true;
        }
        if !implicit_detect && !self.can_detect_invisibility_of_like_cpp(target) {
            return false;
        }
        if !implicit_detect
            && !self.can_detect_stealth_of_like_cpp(target, seer_is_player, check_alert)
        {
            return false;
        }
        true
    }

    pub(crate) fn set_type(&mut self, type_id: TypeId, type_mask: TypeMask) {
        self.world.object_mut().set_type(type_id, type_mask);
    }

    pub const fn data(&self) -> &UnitDataValues {
        &self.data
    }

    pub const fn death_state(&self) -> DeathState {
        self.death_state
    }

    pub fn set_death_state(&mut self, state: DeathState) {
        self.death_state = state;
    }

    pub const fn is_alive(&self) -> bool {
        matches!(self.death_state, DeathState::Alive)
    }

    pub const fn is_dead(&self) -> bool {
        matches!(self.death_state, DeathState::Dead | DeathState::Corpse)
    }

    pub const fn unit_state(&self) -> u32 {
        self.unit_state
    }

    pub fn add_unit_state(&mut self, flags: u32) {
        self.unit_state |= flags;
    }

    pub fn clear_unit_state(&mut self, flags: u32) {
        self.unit_state &= !flags;
    }

    pub fn has_unit_state(&self, flags: u32) -> bool {
        (self.unit_state & flags) != 0
    }

    pub fn set_current_cast_spell(
        &mut self,
        slot: CurrentSpellSlot,
        spell: CurrentSpellRef,
    ) -> Option<CurrentSpellRef> {
        if self.subsystems.spells.current_spell(slot) == Some(spell) {
            return None;
        }

        match slot {
            CurrentSpellSlot::Generic => {
                self.interrupt_spell(CurrentSpellSlot::Generic, false, true);
                if self
                    .current_spell(CurrentSpellSlot::Channeled)
                    .is_some_and(|current| !current.allow_actions_during_channel)
                {
                    self.interrupt_spell(CurrentSpellSlot::Channeled, false, true);
                }
                if self
                    .current_spell(CurrentSpellSlot::Autorepeat)
                    .is_some_and(|current| current.spell_id != AUTO_SHOT_SPELL_ID)
                {
                    self.interrupt_spell(CurrentSpellSlot::Autorepeat, true, true);
                }
                if spell.cast_time_ms > 0 {
                    self.add_unit_state(UnitState::CASTING.bits());
                }
            }
            CurrentSpellSlot::Channeled => {
                self.interrupt_spell(CurrentSpellSlot::Generic, false, true);
                self.interrupt_spell(CurrentSpellSlot::Channeled, true, true);
                if self
                    .current_spell(CurrentSpellSlot::Autorepeat)
                    .is_some_and(|current| current.spell_id != AUTO_SHOT_SPELL_ID)
                {
                    self.interrupt_spell(CurrentSpellSlot::Autorepeat, true, true);
                }
                self.add_unit_state(UnitState::CASTING.bits());
            }
            CurrentSpellSlot::Autorepeat => {
                if spell.spell_id != AUTO_SHOT_SPELL_ID {
                    self.interrupt_spell(CurrentSpellSlot::Generic, false, true);
                    self.interrupt_spell(CurrentSpellSlot::Channeled, false, true);
                }
            }
            CurrentSpellSlot::Melee => {}
        }

        self.subsystems.spells.current_spells.insert(slot, spell)
    }

    pub fn current_spell(&self, slot: CurrentSpellSlot) -> Option<CurrentSpellRef> {
        self.subsystems.spells.current_spell(slot)
    }

    pub fn interrupt_spell(
        &mut self,
        slot: CurrentSpellSlot,
        with_delayed: bool,
        with_instant: bool,
    ) -> Option<CurrentSpellRef> {
        let spell = self.current_spell(slot)?;
        if !with_delayed && spell.state == SpellState::Delayed {
            return None;
        }
        if !with_instant && spell.cast_time_ms == 0 && spell.state != SpellState::Casting {
            return None;
        }
        if !spell.interruptible {
            return None;
        }

        let removed = self.subsystems.spells.clear_current_spell(slot);
        self.sync_casting_unit_state();
        removed
    }

    pub fn finish_spell(&mut self, slot: CurrentSpellSlot) -> Option<CurrentSpellRef> {
        let removed = self.subsystems.spells.clear_current_spell(slot);
        self.sync_casting_unit_state();
        removed
    }

    pub fn interrupt_non_melee_spells(
        &mut self,
        spell_id: Option<u32>,
        with_delayed: bool,
        with_instant: bool,
    ) -> Vec<(CurrentSpellSlot, CurrentSpellRef)> {
        let mut removed = Vec::new();
        for slot in [
            CurrentSpellSlot::Generic,
            CurrentSpellSlot::Autorepeat,
            CurrentSpellSlot::Channeled,
        ] {
            let Some(spell) = self.current_spell(slot) else {
                continue;
            };
            if spell_id.is_some_and(|wanted| wanted != spell.spell_id) {
                continue;
            }
            let slot_with_delayed = with_delayed || slot == CurrentSpellSlot::Channeled;
            let slot_with_instant = with_instant || slot == CurrentSpellSlot::Channeled;
            if let Some(interrupted) =
                self.interrupt_spell(slot, slot_with_delayed, slot_with_instant)
            {
                removed.push((slot, interrupted));
            }
        }
        removed
    }

    pub fn find_current_spell_by_spell_id(&self, spell_id: u32) -> Option<CurrentSpellRef> {
        self.subsystems
            .spells
            .find_current_spell_by_spell_id(spell_id)
    }

    fn sync_casting_unit_state(&mut self) {
        if self.current_spell(CurrentSpellSlot::Generic).is_none()
            && self.current_spell(CurrentSpellSlot::Channeled).is_none()
        {
            self.clear_unit_state(UnitState::CASTING.bits());
        }
    }

    pub const fn attacking(&self) -> Option<ObjectGuid> {
        self.subsystems.combat.attacking_guid
    }

    pub fn set_attacking(&mut self, victim: Option<ObjectGuid>) {
        self.subsystems.combat.set_attacking(victim);
    }

    pub fn add_attacker_like_cpp(&mut self, attacker: ObjectGuid) -> bool {
        self.subsystems.combat.add_attacker(attacker)
    }

    pub fn remove_attacker_like_cpp(&mut self, attacker: ObjectGuid) -> bool {
        self.subsystems.combat.remove_attacker(attacker)
    }

    pub fn has_attacker_like_cpp(&self, attacker: ObjectGuid) -> bool {
        self.subsystems.combat.attackers.contains(&attacker)
    }

    pub const fn last_damaged_target_like_cpp(&self) -> Option<ObjectGuid> {
        self.subsystems.combat.last_damaged_target_guid
    }

    pub fn set_last_damaged_target_like_cpp(&mut self, target: Option<ObjectGuid>) {
        self.subsystems
            .combat
            .set_last_damaged_target_like_cpp(target);
    }

    pub fn attack_like_cpp(
        &mut self,
        victim_guid: ObjectGuid,
        victim_alive: bool,
        victim_in_world: bool,
        melee_attack: bool,
    ) -> UnitAttackStartOutcome {
        self.attack_with_context_like_cpp(
            victim_guid,
            victim_alive,
            victim_in_world,
            melee_attack,
            UnitAttackContextLikeCpp::default(),
        )
    }

    pub fn attack_with_context_like_cpp(
        &mut self,
        victim_guid: ObjectGuid,
        victim_alive: bool,
        victim_in_world: bool,
        melee_attack: bool,
        context: UnitAttackContextLikeCpp,
    ) -> UnitAttackStartOutcome {
        let self_guid = self.world().object().guid();
        if victim_guid.is_empty() || victim_guid == self_guid {
            return UnitAttackStartOutcome::InvalidSelfTarget;
        }
        if !self.is_alive() {
            return UnitAttackStartOutcome::InvalidDeadAttacker;
        }
        if !victim_in_world {
            return UnitAttackStartOutcome::InvalidVictimNotInWorld;
        }
        if !victim_alive {
            return UnitAttackStartOutcome::InvalidDeadVictim;
        }
        if context.attacker_is_mounted_player {
            return UnitAttackStartOutcome::InvalidMountedAttacker;
        }
        if context.attacker_is_evading_creature {
            return UnitAttackStartOutcome::InvalidAttackerEvading;
        }
        if context.victim_is_game_master_player {
            return UnitAttackStartOutcome::InvalidVictimGameMaster;
        }
        if context.victim_is_evading_creature {
            return UnitAttackStartOutcome::InvalidVictimEvading;
        }
        if !Self::is_valid_attack_target_represented_like_cpp(&context) {
            return UnitAttackStartOutcome::InvalidAttackTarget;
        }

        if self
            .subsystems
            .auras
            .has_aura_type_like_cpp(SPELL_AURA_MOD_UNATTACKABLE_LIKE_CPP)
        {
            self.subsystems
                .auras
                .remove_auras_by_type_like_cpp(SPELL_AURA_MOD_UNATTACKABLE_LIKE_CPP);
        }

        if self.attacking() == Some(victim_guid) {
            if melee_attack {
                if !self.has_unit_state(UnitState::MELEE_ATTACKING.bits()) {
                    self.add_unit_state(UnitState::MELEE_ATTACKING.bits());
                    return UnitAttackStartOutcome::MeleeStartedSameTarget;
                }
            } else if self.has_unit_state(UnitState::MELEE_ATTACKING.bits()) {
                self.clear_unit_state(UnitState::MELEE_ATTACKING.bits());
                return UnitAttackStartOutcome::MeleeStoppedSameTarget;
            }
            return UnitAttackStartOutcome::NoChangeSameTarget;
        }

        let previous = self.attacking();
        if previous.is_some() {
            self.interrupt_spell(CurrentSpellSlot::Melee, true, true);
            if !melee_attack {
                self.clear_unit_state(UnitState::MELEE_ATTACKING.bits());
            }
        }

        self.set_attacking(Some(victim_guid));
        self.set_target(victim_guid);
        if melee_attack {
            self.add_unit_state(UnitState::MELEE_ATTACKING.bits());
        }
        self.apply_creature_attack_ai_side_effects_like_cpp(victim_guid);
        self.delay_offhand_attack_like_cpp();
        self.apply_player_controlled_owner_attacked_like_cpp(
            victim_guid,
            &context.controlled_creatures_with_ai,
        );

        UnitAttackStartOutcome::NewTarget { previous }
    }

    pub fn is_valid_attack_target_represented_like_cpp(context: &UnitAttackContextLikeCpp) -> bool {
        let attacker_flags = UnitFlags::from_bits_truncate(context.attacker_unit_flags);
        let victim_flags = UnitFlags::from_bits_truncate(context.victim_unit_flags);

        if context.visibility_represented && !context.attacker_can_see_or_detect_target {
            return false;
        }
        if context.victim_unit_state & UnitState::IN_FLIGHT.bits() != 0 {
            return false;
        }
        if context.attacker_is_player_uber {
            return false;
        }
        if victim_flags.intersects(
            UnitFlags::NON_ATTACKABLE
                | UnitFlags::NON_ATTACKABLE_2
                | UnitFlags::ON_TAXI
                | UnitFlags::NOT_ATTACKABLE_1
                | UnitFlags::UNINTERACTIBLE,
        ) {
            return false;
        }
        if !attacker_flags.contains(UnitFlags::PLAYER_CONTROLLED)
            && victim_flags.contains(UnitFlags::IMMUNE_TO_NPC)
        {
            return false;
        }
        if !victim_flags.contains(UnitFlags::PLAYER_CONTROLLED)
            && attacker_flags.contains(UnitFlags::IMMUNE_TO_NPC)
        {
            return false;
        }
        if attacker_flags.contains(UnitFlags::PLAYER_CONTROLLED)
            && victim_flags.contains(UnitFlags::IMMUNE_TO_PC)
        {
            return false;
        }
        if victim_flags.contains(UnitFlags::PLAYER_CONTROLLED)
            && attacker_flags.contains(UnitFlags::IMMUNE_TO_PC)
        {
            return false;
        }
        if context.relation_represented {
            let attacker_player_controlled = attacker_flags.contains(UnitFlags::PLAYER_CONTROLLED);
            let victim_player_controlled = victim_flags.contains(UnitFlags::PLAYER_CONTROLLED);
            if !attacker_player_controlled && !victim_player_controlled {
                return context.attacker_is_hostile_to_victim
                    || context.victim_is_hostile_to_attacker;
            }
            if context.attacker_is_friendly_to_victim || context.victim_is_friendly_to_attacker {
                return false;
            }
        }
        if !context.attacker_has_affecting_player
            && context.victim_has_affecting_player
            && context.victim_is_pet
            && context.victim_affecting_player_is_mounted
        {
            return false;
        }

        let attacker_has_player = context.attacker_has_affecting_player;
        let victim_has_player = context.victim_has_affecting_player;
        if attacker_has_player ^ victim_has_player {
            if context.player_creature_reputation_represented {
                if context.creature_is_contested_guard && context.player_has_contested_pvp_flag {
                    return true;
                }
                if !context.creature_has_forced_reputation_rank
                    && !context.player_at_war_with_creature_faction
                {
                    return false;
                }
            }
        }

        if attacker_has_player && victim_has_player {
            if context.player_player_duel_in_progress {
                return true;
            }
            if context.sanctuary_represented
                && (context.attacker_in_sanctuary || context.victim_in_sanctuary)
            {
                return false;
            }
            if !context.pvp_represented {
                return false;
            }
            if context.victim_is_pvp {
                return true;
            }
            if context.attacker_is_ffa_pvp && context.victim_is_ffa_pvp {
                return true;
            }
            return context.attacker_has_pvp_unk1_flag || context.victim_has_pvp_unk1_flag;
        }

        true
    }

    pub fn attack_stop_like_cpp(&mut self) -> UnitAttackStopOutcome {
        let Some(victim) = self.attacking() else {
            return UnitAttackStopOutcome::NoVictim;
        };

        self.set_attacking(None);
        self.set_target(ObjectGuid::EMPTY);
        self.clear_unit_state(UnitState::MELEE_ATTACKING.bits());
        self.interrupt_spell(CurrentSpellSlot::Melee, true, true);

        UnitAttackStopOutcome::Stopped { victim }
    }

    pub const fn subsystems(&self) -> &UnitSubsystems {
        &self.subsystems
    }

    pub fn subsystems_mut(&mut self) -> &mut UnitSubsystems {
        &mut self.subsystems
    }

    pub const fn base_attack_speed(&self) -> [u32; MAX_ATTACK] {
        self.base_attack_speed
    }

    pub const fn mod_attack_speed_pct(&self) -> [f32; MAX_ATTACK] {
        self.mod_attack_speed_pct
    }

    pub const fn attack_timer(&self, attack: WeaponAttackType) -> u32 {
        self.attack_timer[attack as usize]
    }

    pub fn set_attack_timer(&mut self, attack: WeaponAttackType, time_ms: u32) {
        let slot = attack as usize;
        if slot < MAX_ATTACK {
            self.attack_timer[slot] = time_ms;
        }
    }

    pub fn reset_attack_timer_like_cpp(&mut self, attack: WeaponAttackType) {
        let slot = attack as usize;
        if slot < MAX_ATTACK {
            self.attack_timer[slot] =
                (self.base_attack_speed[slot] as f32 * self.mod_attack_speed_pct[slot]) as u32;
        }
    }

    pub fn update_attack_timers_like_cpp(&mut self, diff_ms: u32) {
        for timer in &mut self.attack_timer {
            *timer = timer.saturating_sub(diff_ms);
        }
    }

    pub fn is_attack_ready_like_cpp(&self, attack: WeaponAttackType) -> bool {
        self.attack_timer(attack) == 0
    }

    pub fn can_attacker_state_update_melee_like_cpp(&self, extra: bool) -> bool {
        if self.unit_flags_like_cpp().contains(UnitFlags::PACIFIED) {
            return false;
        }
        if !extra && self.has_unit_state((UnitState::CONTROLLED | UnitState::CHARGING).bits()) {
            return false;
        }
        if self
            .subsystems
            .auras
            .has_aura_type_like_cpp(SPELL_AURA_DISABLE_ATTACKING_EXCEPT_ABILITIES_LIKE_CPP)
        {
            return false;
        }
        true
    }

    pub fn remove_attacking_interrupt_auras_like_cpp(&mut self) -> usize {
        self.subsystems
            .auras
            .remove_interruptible_auras(SPELL_AURA_INTERRUPT_FLAG_ATTACKING_LIKE_CPP, 0)
            .len()
    }

    pub fn set_base_attack_time_like_cpp(&mut self, attack: WeaponAttackType, time_ms: u32) {
        let slot = attack as usize;
        if slot < MAX_ATTACK {
            self.base_attack_speed[slot] = time_ms;
        }
    }

    pub const fn can_dual_wield_like_cpp(&self) -> bool {
        self.can_dual_wield
    }

    pub fn set_can_dual_wield_like_cpp(&mut self, can_dual_wield: bool) {
        self.can_dual_wield = can_dual_wield;
    }

    pub const fn emote_state_like_cpp(&self) -> u32 {
        self.emote_state
    }

    pub fn set_emote_state_like_cpp(&mut self, emote_state: u32) {
        self.emote_state = emote_state;
    }

    fn apply_creature_attack_ai_side_effects_like_cpp(&mut self, victim_guid: ObjectGuid) {
        if self.world().object().type_id() != TypeId::Unit
            || self.subsystems.control.controlled_by_player
        {
            return;
        }

        self.subsystems.combat.add_threat(victim_guid, 0.0);
        self.subsystems.ai.send_hostile_reaction_like_cpp();
        self.subsystems.ai.call_assistance_like_cpp();
        self.set_emote_state_like_cpp(0);
        self.set_stand_state_like_cpp(UnitStandStateType::Stand);
    }

    fn apply_player_controlled_owner_attacked_like_cpp(
        &mut self,
        victim_guid: ObjectGuid,
        controlled_creatures_with_ai: &[ObjectGuid],
    ) {
        if self.world().object().type_id() != TypeId::Player {
            return;
        }

        self.subsystems
            .control
            .notify_controlled_owner_attacked_like_cpp(controlled_creatures_with_ai, victim_guid);
    }

    fn has_offhand_weapon_for_attack_like_cpp(&self) -> bool {
        self.world().object().type_id() != TypeId::Player && self.can_dual_wield
    }

    fn delay_offhand_attack_like_cpp(&mut self) {
        if !self.has_offhand_weapon_for_attack_like_cpp() {
            return;
        }

        let base = WeaponAttackType::BaseAttack as usize;
        let off = WeaponAttackType::OffAttack as usize;
        let delay = self.attack_timer[base].saturating_add(self.base_attack_speed[base] / 2);
        self.attack_timer[off] = self.attack_timer[off].max(delay);
    }

    pub const fn weapon_damage(&self, attack: WeaponAttackType) -> [f32; 2] {
        self.weapon_damage[attack as usize]
    }

    pub fn set_weapon_damage(
        &mut self,
        attack: WeaponAttackType,
        min_damage: f32,
        max_damage: f32,
    ) {
        let slot = attack as usize;
        if slot < MAX_ATTACK {
            self.weapon_damage[slot] = [min_damage, max_damage];
        }
    }

    pub const fn speed_rate(&self) -> [f32; MAX_MOVE_TYPE] {
        self.speed_rate
    }

    pub fn unit_data_changes_mask(&self) -> &UpdateMask {
        &self.unit_data_changes
    }

    pub fn clear_unit_data_changes(&mut self) {
        self.unit_data_changes.reset_all();
    }

    pub fn set_level(&mut self, level: u8) {
        self.set_i32_field(UNIT_DATA_LEVEL_BIT, i32::from(level), |data| {
            &mut data.level
        });
    }

    pub fn set_faction(&mut self, faction: u32) {
        self.set_i32_field(UNIT_DATA_FACTION_TEMPLATE_BIT, faction as i32, |data| {
            &mut data.faction_template
        });
    }

    pub fn set_bounding_radius(&mut self, radius: f32) {
        self.set_f32_field(UNIT_DATA_BOUNDING_RADIUS_BIT, radius, |data| {
            &mut data.bounding_radius
        });
    }

    pub fn set_combat_reach(&mut self, reach: f32) {
        let reach = reach.max(0.0);
        self.set_f32_field(UNIT_DATA_COMBAT_REACH_BIT, reach, |data| {
            &mut data.combat_reach
        });
        self.world.set_combat_reach(reach);
    }

    pub fn set_display_id(&mut self, display_id: u32, set_native: bool) {
        self.set_i32_field(UNIT_DATA_DISPLAY_ID_BIT, display_id as i32, |data| {
            &mut data.display_id
        });
        self.set_f32_field(
            UNIT_DATA_DISPLAY_SCALE_BIT,
            DEFAULT_PLAYER_DISPLAY_SCALE,
            |data| &mut data.display_scale,
        );

        if set_native {
            self.set_i32_field(UNIT_DATA_NATIVE_DISPLAY_ID_BIT, display_id as i32, |data| {
                &mut data.native_display_id
            });
            self.set_f32_field(
                UNIT_DATA_NATIVE_DISPLAY_SCALE_BIT,
                DEFAULT_PLAYER_DISPLAY_SCALE,
                |data| &mut data.native_display_scale,
            );
        }
    }

    pub fn set_display_power(&mut self, power: PowerType) {
        self.set_u8_field(UNIT_DATA_DISPLAY_POWER_BIT, power as u8, |data| {
            &mut data.display_power
        });
    }

    pub fn set_mount_display_id(&mut self, display_id: u32) {
        self.set_i32_field(UNIT_DATA_MOUNT_DISPLAY_ID_BIT, display_id as i32, |data| {
            &mut data.mount_display_id
        });
    }

    pub fn set_target(&mut self, target: ObjectGuid) {
        self.set_guid_field(UNIT_DATA_TARGET_BIT, target, |data| &mut data.target);
    }

    pub fn set_unit_flags_like_cpp(&mut self, flags: UnitFlags) {
        if self.data.flags != flags.bits() {
            self.data.flags = flags.bits();
            self.mark_unit_data(UNIT_DATA_FLAGS_BIT);
        }
    }

    pub fn unit_flags_like_cpp(&self) -> UnitFlags {
        UnitFlags::from_bits_truncate(self.data.flags)
    }

    pub fn set_unit_flags2_like_cpp(&mut self, flags: UnitFlags2) {
        if self.data.flags2 != flags.bits() {
            self.data.flags2 = flags.bits();
            self.mark_unit_data(UNIT_DATA_FLAGS2_BIT);
        }
    }

    pub fn unit_flags2_like_cpp(&self) -> UnitFlags2 {
        UnitFlags2::from_bits_truncate(self.data.flags2)
    }

    pub fn set_npc_flags_like_cpp(&mut self, flags: u32) {
        self.set_npc_flags_index_like_cpp(0, flags);
    }

    pub fn set_npc_flags2_like_cpp(&mut self, flags: u32) {
        self.set_npc_flags_index_like_cpp(1, flags);
    }

    pub const fn npc_flags_like_cpp(&self) -> [u32; 2] {
        self.data.npc_flags
    }

    fn set_npc_flags_index_like_cpp(&mut self, index: usize, flags: u32) {
        if index >= self.data.npc_flags.len() {
            return;
        }
        if self.data.npc_flags[index] != flags {
            self.data.npc_flags[index] = flags;
            self.mark_unit_data_array(
                UNIT_DATA_NPC_FLAGS_PARENT_BIT,
                UNIT_DATA_NPC_FLAGS_FIRST_BIT,
                index,
            );
        }
    }

    pub fn set_race(&mut self, race: u8) {
        self.set_u8_field(UNIT_DATA_RACE_BIT, race, |data| &mut data.race);
    }

    pub fn set_class(&mut self, class_id: u8) {
        self.set_u8_field(UNIT_DATA_CLASS_ID_BIT, class_id, |data| &mut data.class_id);
    }

    pub fn set_player_class(&mut self, class_id: u8) {
        self.set_u8_field(UNIT_DATA_PLAYER_CLASS_ID_BIT, class_id, |data| {
            &mut data.player_class_id
        });
    }

    pub fn set_gender(&mut self, gender: Gender) {
        self.set_u8_field(UNIT_DATA_SEX_BIT, gender as u8, |data| &mut data.sex);
    }

    pub fn stand_state_like_cpp(&self) -> UnitStandStateType {
        match self.data.stand_state {
            1 => UnitStandStateType::Sit,
            2 => UnitStandStateType::SitChair,
            3 => UnitStandStateType::Sleep,
            4 => UnitStandStateType::SitLowChair,
            5 => UnitStandStateType::SitMediumChair,
            6 => UnitStandStateType::SitHighChair,
            7 => UnitStandStateType::Dead,
            8 => UnitStandStateType::Kneel,
            9 => UnitStandStateType::Submerged,
            10 => UnitStandStateType::Max,
            _ => UnitStandStateType::Stand,
        }
    }

    pub fn is_stand_state_like_cpp(&self) -> bool {
        !matches!(
            self.stand_state_like_cpp(),
            UnitStandStateType::Sit
                | UnitStandStateType::SitChair
                | UnitStandStateType::SitLowChair
                | UnitStandStateType::SitMediumChair
                | UnitStandStateType::SitHighChair
                | UnitStandStateType::Sleep
                | UnitStandStateType::Kneel
        )
    }

    pub fn set_stand_state_like_cpp(&mut self, state: UnitStandStateType) {
        self.set_u8_field(UNIT_DATA_STAND_STATE_BIT, state as u8, |data| {
            &mut data.stand_state
        });
    }

    pub fn replace_all_pvp_flags_like_cpp(&mut self, flags: UnitPvpFlags) {
        self.set_u8_field(UNIT_DATA_PVP_FLAGS_BIT, flags.bits(), |data| {
            &mut data.pvp_flags
        });
    }

    pub fn set_pvp_flag_like_cpp(&mut self, flags: UnitPvpFlags) {
        self.replace_all_pvp_flags_like_cpp(self.pvp_flags_like_cpp() | flags);
    }

    pub fn remove_pvp_flag_like_cpp(&mut self, flags: UnitPvpFlags) {
        self.replace_all_pvp_flags_like_cpp(self.pvp_flags_like_cpp() & !flags);
    }

    pub fn pvp_flags_like_cpp(&self) -> UnitPvpFlags {
        UnitPvpFlags::from_bits_retain(self.data.pvp_flags)
    }

    pub fn has_pvp_flag_like_cpp(&self, flags: UnitPvpFlags) -> bool {
        self.pvp_flags_like_cpp().intersects(flags)
    }

    pub fn is_pvp_like_cpp(&self) -> bool {
        self.has_pvp_flag_like_cpp(UnitPvpFlags::PVP)
    }

    pub fn is_ffa_pvp_like_cpp(&self) -> bool {
        self.has_pvp_flag_like_cpp(UnitPvpFlags::FFA_PVP)
    }

    pub fn is_in_sanctuary_like_cpp(&self) -> bool {
        self.has_pvp_flag_like_cpp(UnitPvpFlags::SANCTUARY)
    }

    pub fn set_health(&mut self, mut value: u64) {
        if matches!(self.death_state, DeathState::JustDied | DeathState::Corpse) {
            value = 0;
        } else if value > self.data.max_health {
            value = self.data.max_health;
        }
        self.set_u64_field(UNIT_DATA_HEALTH_BIT, value, |data| &mut data.health);
    }

    pub fn set_max_health(&mut self, mut value: u64) {
        if value == 0 {
            value = 1;
        }
        let current = self.data.health;
        self.set_u64_field(UNIT_DATA_MAX_HEALTH_BIT, value, |data| &mut data.max_health);
        if value < current {
            self.set_health(value);
        }
    }

    pub fn set_power_index(&mut self, power: PowerType, index: Option<usize>) {
        if let Some(slot) = power_slot(power) {
            self.power_index[slot] = index.filter(|value| *value < MAX_POWERS_PER_CLASS);
        }
    }

    pub fn get_power_index(&self, power: PowerType) -> Option<usize> {
        power_slot(power).and_then(|slot| self.power_index[slot])
    }

    pub fn get_power(&self, power: PowerType) -> i32 {
        self.get_power_index(power)
            .map(|index| self.data.power[index])
            .unwrap_or(0)
    }

    pub fn get_max_power(&self, power: PowerType) -> i32 {
        self.get_power_index(power)
            .map(|index| self.data.max_power[index])
            .unwrap_or(0)
    }

    pub fn set_power(&mut self, power: PowerType, mut value: i32) {
        let Some(index) = self.get_power_index(power) else {
            return;
        };
        let max = self.data.max_power[index];
        if value > max {
            value = max;
        }
        if self.data.power[index] != value {
            self.data.power[index] = value;
            self.mark_unit_data_array(UNIT_DATA_POWER_PARENT_BIT, UNIT_DATA_POWER_FIRST_BIT, index);
        }
    }

    pub fn set_max_power(&mut self, power: PowerType, value: i32) {
        let Some(index) = self.get_power_index(power) else {
            return;
        };
        let current = self.data.power[index];
        if self.data.max_power[index] != value {
            self.data.max_power[index] = value;
            self.mark_unit_data_array(
                UNIT_DATA_POWER_PARENT_BIT,
                UNIT_DATA_MAX_POWER_FIRST_BIT,
                index,
            );
        }
        if value < current {
            self.set_power(power, value);
        }
    }

    pub fn set_virtual_item(&mut self, index: usize, visible: Option<VisibleItemValues>) {
        if index >= MAX_ATTACK {
            return;
        }

        let value = visible.unwrap_or_default();
        if self.data.virtual_items[index] != value {
            self.data.virtual_items[index] = value;
            self.mark_unit_data_array(
                UNIT_DATA_VIRTUAL_ITEMS_PARENT_BIT,
                UNIT_DATA_VIRTUAL_ITEMS_FIRST_BIT,
                index,
            );
        }
    }

    pub fn mark_virtual_item_changed(&mut self, index: usize) {
        if index >= MAX_ATTACK {
            return;
        }

        self.mark_unit_data_array(
            UNIT_DATA_VIRTUAL_ITEMS_PARENT_BIT,
            UNIT_DATA_VIRTUAL_ITEMS_FIRST_BIT,
            index,
        );
    }

    pub fn changed_object_type_mask(&self) -> u32 {
        self.world.object().changed_object_type_mask()
            | if self.unit_data_changes.is_any_set() {
                1 << TYPEID_UNIT
            } else {
                0
            }
    }

    pub fn values_update(&self) -> UnitValuesUpdate {
        let object_update = self.world.object().values_update();
        UnitValuesUpdate {
            changed_object_type_mask: self.changed_object_type_mask(),
            object_data: object_update.object_data,
            unit_data: self.unit_data_changes.is_any_set().then(|| UnitDataUpdate {
                mask: self.unit_data_changes.clone(),
                values: self.data,
            }),
        }
    }

    fn set_u64_field(
        &mut self,
        bit: usize,
        value: u64,
        field: impl FnOnce(&mut UnitDataValues) -> &mut u64,
    ) {
        let target = field(&mut self.data);
        if *target != value {
            *target = value;
            self.mark_unit_data(bit);
        }
    }

    fn set_i32_field(
        &mut self,
        bit: usize,
        value: i32,
        field: impl FnOnce(&mut UnitDataValues) -> &mut i32,
    ) {
        let target = field(&mut self.data);
        if *target != value {
            *target = value;
            self.mark_unit_data(bit);
        }
    }

    fn set_u8_field(
        &mut self,
        bit: usize,
        value: u8,
        field: impl FnOnce(&mut UnitDataValues) -> &mut u8,
    ) {
        let target = field(&mut self.data);
        if *target != value {
            *target = value;
            self.mark_unit_data(bit);
        }
    }

    fn set_guid_field(
        &mut self,
        bit: usize,
        value: ObjectGuid,
        field: impl FnOnce(&mut UnitDataValues) -> &mut ObjectGuid,
    ) {
        let target = field(&mut self.data);
        if *target != value {
            *target = value;
            self.mark_unit_data(bit);
        }
    }

    fn set_f32_field(
        &mut self,
        bit: usize,
        value: f32,
        field: impl FnOnce(&mut UnitDataValues) -> &mut f32,
    ) {
        let target = field(&mut self.data);
        if *target != value {
            *target = value;
            self.mark_unit_data(bit);
        }
    }

    fn mark_unit_data(&mut self, bit: usize) {
        self.unit_data_changes.set(UNIT_DATA_PARENT_BIT);
        self.unit_data_changes.set(bit);
    }

    fn mark_unit_data_array(&mut self, parent_bit: usize, first_element_bit: usize, index: usize) {
        self.unit_data_changes.set(parent_bit);
        self.unit_data_changes.set(first_element_bit + index);
    }
}

fn power_slot(power: PowerType) -> Option<usize> {
    let value = power as i8;
    (0..MAX_POWERS as i8)
        .contains(&value)
        .then_some(value as usize)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AppliedAuraRef, AuraRef, CurrentSpellRef, CurrentSpellSlot, MAX_SUMMON_SLOT,
        MOTIONMASTER_FLAG_INITIALIZATION_PENDING, MOTIONMASTER_FLAG_INITIALIZING,
        MotionMasterDelayedActionPayload, MovementGeneratorKind, MovementGeneratorRef,
        MovementSlot, OwnedAuraRef,
    };

    #[test]
    fn unit_constructor_matches_cpp_base_state() {
        let unit = Unit::new(true);

        assert_eq!(unit.world().object().type_id(), TypeId::Unit);
        assert_eq!(
            unit.world().object().type_mask(),
            TypeMask::OBJECT | TypeMask::UNIT
        );
        assert!(
            unit.world()
                .object()
                .create_flags()
                .contains(crate::CreateObjectFlags::MOVEMENT_UPDATE)
        );
        assert_eq!(unit.death_state(), DeathState::Alive);
        assert_eq!(unit.unit_state(), 0);
        assert_eq!(unit.attacking(), None);
        assert_eq!(unit.base_attack_speed(), [0; MAX_ATTACK]);
        assert_eq!(unit.mod_attack_speed_pct(), [1.0; MAX_ATTACK]);
        assert_eq!(unit.attack_timer(WeaponAttackType::BaseAttack), 0);
        assert_eq!(unit.attack_timer(WeaponAttackType::OffAttack), 0);
        assert!(!unit.can_dual_wield_like_cpp());
        assert_eq!(unit.emote_state_like_cpp(), 0);
        assert_eq!(unit.weapon_damage(WeaponAttackType::BaseAttack), [1.0, 2.0]);
        assert_eq!(unit.speed_rate(), [1.0; MAX_MOVE_TYPE]);
        assert_eq!(unit.collision_height_like_cpp(), 0.0);
        assert_eq!(unit.world().collision_height_like_cpp(), 0.0);
        assert!(unit.subsystems().auras.owned_auras.is_empty());
        assert!(unit.subsystems().auras.applied_auras.is_empty());
        assert!(unit.subsystems().auras.interruptible_auras.is_empty());
        assert!(unit.subsystems().auras.aura_state_auras.is_empty());
        assert_eq!(unit.subsystems().auras.aura_state_mask, 0);
        assert_eq!(unit.subsystems().auras.removed_auras_count, 0);
        assert!(unit.subsystems().auras.can_proc());
        assert!(unit.subsystems().spells.current_spells.is_empty());
        assert!(unit.subsystems().spells.history.cooldowns.is_empty());
        assert!(unit.subsystems().combat.threat.is_empty());
        assert!(unit.subsystems().combat.threat_refs.is_empty());
        assert!(unit.subsystems().combat.threatened_by_me.is_empty());
        assert!(unit.subsystems().combat.pve_refs.is_empty());
        assert!(unit.subsystems().combat.pvp_refs.is_empty());
        assert_eq!(unit.subsystems().combat.current_victim_guid, None);
        assert_eq!(unit.subsystems().combat.fixate_guid, None);
        assert!(unit.subsystems().combat.attackers.is_empty());
        assert_eq!(unit.subsystems().combat.attacking_guid, None);
        assert!(!unit.subsystems().combat.combat_disallowed);
        assert_eq!(
            unit.subsystems().motion.current_generator,
            MovementGeneratorKind::Idle
        );
        assert!(!unit.subsystems().motion.paused);
        assert!(!unit.subsystems().motion.spline.enabled);
        assert!(unit.subsystems().motion.spline.finalized);
        assert_eq!(unit.subsystems().control.charmer_guid, None);
        assert_eq!(unit.subsystems().control.owner_guid, None);
        assert_eq!(unit.subsystems().control.minion_guid, None);
        assert_eq!(
            unit.subsystems().control.summon_slots,
            [ObjectGuid::EMPTY; MAX_SUMMON_SLOT]
        );
        assert!(!unit.subsystems().control.has_charm_info());
        assert_eq!(unit.subsystems().vehicle.vehicle_guid, None);
        assert_eq!(unit.subsystems().ai.active_ai, None);
        assert!(!unit.subsystems().ai.locked);
        assert!(!unit.subsystems().ai.scheduled_change_pending);
        assert!(!unit.unit_data_changes_mask().is_any_set());
    }

    #[test]
    fn unit_add_to_world_like_cpp_removes_enter_world_auras_and_initializes_motion_once() {
        let mut unit = Unit::new(true);
        let caster = ObjectGuid::new(0, 47501);
        let enter_world_aura = AppliedAuraRef::new(47_501, caster, 1, 0x1);
        let attacking_aura = AppliedAuraRef::new(47_502, caster, 2, 0x1);
        unit.subsystems_mut().auras.register_applied_aura(
            enter_world_aura,
            None,
            SPELL_AURA_INTERRUPT_FLAG_ENTER_WORLD_LIKE_CPP,
            0,
        );
        unit.subsystems_mut().auras.register_applied_aura(
            attacking_aura,
            None,
            SPELL_AURA_INTERRUPT_FLAG_ATTACKING_LIKE_CPP,
            0,
        );
        unit.subsystems_mut().motion.push_delayed_payload_like_cpp(
            MotionMasterDelayedActionPayload::Add(MovementGeneratorRef::new(
                MovementGeneratorKind::Point,
                MovementSlot::Active,
            )),
        );

        let first = unit.add_to_world_like_cpp();

        assert_eq!(first.guid, unit.world().object().guid());
        assert!(first.world_object_added);
        assert!(first.is_in_world_after);
        assert_eq!(
            first.aura_interrupt_flags_enter_world,
            SPELL_AURA_INTERRUPT_FLAG_ENTER_WORLD_LIKE_CPP
        );
        assert_eq!(first.removed_enter_world_auras, vec![enter_world_aura]);
        assert!(!unit.subsystems().auras.has_applied(enter_world_aura));
        assert!(unit.subsystems().auras.has_applied(attacking_aura));

        let motion = first.motion_master_add_to_world;
        assert!(motion.had_initialization_pending);
        assert!(motion.entered_initializing);
        assert!(motion.direct_initialize_represented);
        assert!(motion.exited_initializing);
        assert_eq!(motion.resolved_delayed_actions.len(), 1);
        assert!(
            !unit
                .subsystems()
                .motion
                .has_motion_master_flag(MOTIONMASTER_FLAG_INITIALIZATION_PENDING)
        );
        assert!(
            !unit
                .subsystems()
                .motion
                .has_motion_master_flag(MOTIONMASTER_FLAG_INITIALIZING)
        );
        assert_eq!(
            unit.subsystems().motion.current_generator,
            MovementGeneratorKind::Point
        );

        let second = unit.add_to_world_like_cpp();

        assert!(!second.motion_master_add_to_world.had_initialization_pending);
        assert!(
            !second
                .motion_master_add_to_world
                .direct_initialize_represented
        );
        assert!(second.removed_enter_world_auras.is_empty());
        assert!(unit.subsystems().auras.has_applied(attacking_aura));
        assert_eq!(
            unit.subsystems().motion.current_generator,
            MovementGeneratorKind::Point
        );
    }

    #[test]
    fn shared_vision_add_empty_to_non_empty_activates_and_requests_world_object_on_like_cpp() {
        let mut unit = Unit::new(false);
        let unit_guid = ObjectGuid::new(1, 42);
        let player_guid = ObjectGuid::new(1, 100);
        unit.world_mut().object_mut().create(unit_guid);

        let outcome = unit.add_player_to_vision_like_cpp(player_guid);

        assert_eq!(outcome.player_guid, player_guid);
        assert!(outcome.inserted_or_removed);
        assert_eq!(
            outcome.set_world_object,
            Some(UnitSharedVisionSetWorldObjectRequestLikeCpp {
                unit_guid,
                on: true,
            })
        );
        assert!(unit.world().is_active());
        assert!(!unit.world().is_world_object());
        assert!(unit.subsystems().control.has_shared_vision());
    }

    #[test]
    fn shared_vision_add_non_empty_or_duplicate_does_not_request_world_object_again_like_cpp() {
        let mut unit = Unit::new(false);
        let unit_guid = ObjectGuid::new(1, 43);
        let first_player = ObjectGuid::new(1, 101);
        let second_player = ObjectGuid::new(1, 102);
        unit.world_mut().object_mut().create(unit_guid);

        let first = unit.add_player_to_vision_like_cpp(first_player);
        assert!(first.set_world_object.is_some());

        let second = unit.add_player_to_vision_like_cpp(second_player);
        assert_eq!(second.player_guid, second_player);
        assert!(second.inserted_or_removed);
        assert_eq!(second.set_world_object, None);
        assert!(unit.world().is_active());

        let duplicate = unit.add_player_to_vision_like_cpp(second_player);
        assert_eq!(duplicate.player_guid, second_player);
        assert!(!duplicate.inserted_or_removed);
        assert_eq!(duplicate.set_world_object, None);
        assert!(unit.world().is_active());
    }

    #[test]
    fn shared_vision_remove_keeps_active_until_last_viewer_then_requests_off_like_cpp() {
        let mut unit = Unit::new(false);
        let unit_guid = ObjectGuid::new(1, 44);
        let first_player = ObjectGuid::new(1, 103);
        let second_player = ObjectGuid::new(1, 104);
        unit.world_mut().object_mut().create(unit_guid);
        unit.add_player_to_vision_like_cpp(first_player);
        unit.add_player_to_vision_like_cpp(second_player);

        let first_remove = unit.remove_player_from_vision_like_cpp(first_player);
        assert_eq!(first_remove.player_guid, first_player);
        assert!(first_remove.inserted_or_removed);
        assert_eq!(first_remove.set_world_object, None);
        assert!(unit.world().is_active());
        assert!(unit.subsystems().control.has_shared_vision());

        let last_remove = unit.remove_player_from_vision_like_cpp(second_player);
        assert_eq!(last_remove.player_guid, second_player);
        assert!(last_remove.inserted_or_removed);
        assert_eq!(
            last_remove.set_world_object,
            Some(UnitSharedVisionSetWorldObjectRequestLikeCpp {
                unit_guid,
                on: false,
            })
        );
        assert!(!unit.world().is_active());
        assert!(!unit.world().is_world_object());
        assert!(!unit.subsystems().control.has_shared_vision());
    }

    #[test]
    fn shared_vision_remove_absent_from_empty_still_requests_off_like_cpp() {
        let mut unit = Unit::new(false);
        let unit_guid = ObjectGuid::new(1, 45);
        let absent_player = ObjectGuid::new(1, 105);
        unit.world_mut().object_mut().create(unit_guid);
        unit.world_mut().set_active(true);

        let outcome = unit.remove_player_from_vision_like_cpp(absent_player);

        assert_eq!(outcome.player_guid, absent_player);
        assert!(!outcome.inserted_or_removed);
        assert_eq!(
            outcome.set_world_object,
            Some(UnitSharedVisionSetWorldObjectRequestLikeCpp {
                unit_guid,
                on: false,
            })
        );
        assert!(!unit.world().is_active());
        assert!(!unit.world().is_world_object());
        assert!(!unit.subsystems().control.has_shared_vision());
    }

    #[test]
    fn shared_vision_remove_absent_keeps_active_when_other_viewers_remain_like_cpp() {
        let mut unit = Unit::new(false);
        let unit_guid = ObjectGuid::new(1, 46);
        let present_player = ObjectGuid::new(1, 106);
        let absent_player = ObjectGuid::new(1, 107);
        unit.world_mut().object_mut().create(unit_guid);
        unit.add_player_to_vision_like_cpp(present_player);

        let outcome = unit.remove_player_from_vision_like_cpp(absent_player);

        assert_eq!(outcome.player_guid, absent_player);
        assert!(!outcome.inserted_or_removed);
        assert_eq!(outcome.set_world_object, None);
        assert!(unit.world().is_active());
        assert!(unit.subsystems().control.has_shared_vision());
    }

    #[test]
    fn attacking_uses_combat_subsystem_as_single_source_of_truth() {
        let mut unit = Unit::new(true);
        let victim = ObjectGuid::new(1, 10);
        let other_victim = ObjectGuid::new(1, 11);

        unit.set_attacking(Some(victim));
        assert_eq!(unit.attacking(), Some(victim));
        assert_eq!(unit.subsystems().combat.attacking_guid, Some(victim));

        unit.subsystems_mut()
            .combat
            .set_attacking(Some(other_victim));
        assert_eq!(unit.attacking(), Some(other_victim));

        unit.subsystems_mut().combat.clear_attackers();
        assert_eq!(unit.attacking(), None);

        unit.set_attacking(Some(victim));
        unit.subsystems_mut().clear_runtime_state();
        assert_eq!(unit.attacking(), None);
    }

    #[test]
    fn attack_like_cpp_records_target_and_melee_state() {
        let mut unit = Unit::new(true);
        let attacker = ObjectGuid::new(1, 9);
        let victim = ObjectGuid::new(1, 10);
        unit.world_mut().object_mut().create(attacker);

        assert_eq!(
            unit.attack_like_cpp(victim, true, true, true),
            UnitAttackStartOutcome::NewTarget { previous: None }
        );
        assert_eq!(unit.attacking(), Some(victim));
        assert_eq!(unit.data().target, victim);
        assert!(unit.has_unit_state(UnitState::MELEE_ATTACKING.bits()));

        assert_eq!(
            unit.attack_like_cpp(victim, true, true, true),
            UnitAttackStartOutcome::NoChangeSameTarget
        );
        assert_eq!(
            unit.attack_like_cpp(victim, true, true, false),
            UnitAttackStartOutcome::MeleeStoppedSameTarget
        );
        assert!(!unit.has_unit_state(UnitState::MELEE_ATTACKING.bits()));
        assert_eq!(
            unit.attack_like_cpp(victim, true, true, true),
            UnitAttackStartOutcome::MeleeStartedSameTarget
        );
        assert!(unit.has_unit_state(UnitState::MELEE_ATTACKING.bits()));
    }

    #[test]
    fn attack_like_cpp_switches_target_and_interrupts_melee_spell() {
        let mut unit = Unit::new(true);
        let attacker = ObjectGuid::new(1, 9);
        let first = ObjectGuid::new(1, 10);
        let second = ObjectGuid::new(1, 11);
        let melee = CurrentSpellRef::new(700, Some(attacker), None).with_cast_time_ms(1_000);
        unit.world_mut().object_mut().create(attacker);

        assert_eq!(
            unit.attack_like_cpp(first, true, true, true),
            UnitAttackStartOutcome::NewTarget { previous: None }
        );
        unit.subsystems_mut()
            .spells
            .set_current_spell(CurrentSpellSlot::Melee, melee);
        assert_eq!(
            unit.attack_like_cpp(second, true, true, false),
            UnitAttackStartOutcome::NewTarget {
                previous: Some(first)
            }
        );

        assert_eq!(unit.attacking(), Some(second));
        assert_eq!(unit.data().target, second);
        assert_eq!(unit.current_spell(CurrentSpellSlot::Melee), None);
        assert!(!unit.has_unit_state(UnitState::MELEE_ATTACKING.bits()));
    }

    #[test]
    fn attack_stop_like_cpp_clears_target_melee_state_and_spell() {
        let mut unit = Unit::new(true);
        let attacker = ObjectGuid::new(1, 9);
        let victim = ObjectGuid::new(1, 10);
        let melee = CurrentSpellRef::new(701, Some(attacker), None).with_cast_time_ms(1_000);
        unit.world_mut().object_mut().create(attacker);
        unit.attack_like_cpp(victim, true, true, true);
        unit.subsystems_mut()
            .spells
            .set_current_spell(CurrentSpellSlot::Melee, melee);

        assert_eq!(
            unit.attack_stop_like_cpp(),
            UnitAttackStopOutcome::Stopped { victim }
        );
        assert_eq!(unit.attacking(), None);
        assert_eq!(unit.data().target, ObjectGuid::EMPTY);
        assert_eq!(unit.current_spell(CurrentSpellSlot::Melee), None);
        assert!(!unit.has_unit_state(UnitState::MELEE_ATTACKING.bits()));
        assert_eq!(unit.attack_stop_like_cpp(), UnitAttackStopOutcome::NoVictim);
    }

    #[test]
    fn attacker_set_helpers_match_cpp_insert_erase_shape() {
        let mut victim = Unit::new(true);
        let attacker = ObjectGuid::new(1, 9);

        assert!(victim.add_attacker_like_cpp(attacker));
        assert!(!victim.add_attacker_like_cpp(attacker));
        assert!(victim.has_attacker_like_cpp(attacker));
        assert!(victim.remove_attacker_like_cpp(attacker));
        assert!(!victim.remove_attacker_like_cpp(attacker));
        assert!(!victim.has_attacker_like_cpp(attacker));
    }

    #[test]
    fn attack_with_context_like_cpp_rejects_mounted_evading_and_gm_targets() {
        let mut unit = Unit::new(true);
        let attacker = ObjectGuid::new(1, 9);
        let victim = ObjectGuid::new(1, 10);
        unit.world_mut().object_mut().create(attacker);

        assert_eq!(
            unit.attack_with_context_like_cpp(
                victim,
                true,
                true,
                true,
                UnitAttackContextLikeCpp {
                    attacker_is_mounted_player: true,
                    ..Default::default()
                },
            ),
            UnitAttackStartOutcome::InvalidMountedAttacker
        );
        assert_eq!(unit.attacking(), None);

        assert_eq!(
            unit.attack_with_context_like_cpp(
                victim,
                true,
                true,
                true,
                UnitAttackContextLikeCpp {
                    attacker_is_evading_creature: true,
                    ..Default::default()
                },
            ),
            UnitAttackStartOutcome::InvalidAttackerEvading
        );
        assert_eq!(
            unit.attack_with_context_like_cpp(
                victim,
                true,
                true,
                true,
                UnitAttackContextLikeCpp {
                    victim_is_game_master_player: true,
                    ..Default::default()
                },
            ),
            UnitAttackStartOutcome::InvalidVictimGameMaster
        );
        assert_eq!(
            unit.attack_with_context_like_cpp(
                victim,
                true,
                true,
                true,
                UnitAttackContextLikeCpp {
                    victim_is_evading_creature: true,
                    ..Default::default()
                },
            ),
            UnitAttackStartOutcome::InvalidVictimEvading
        );
    }

    #[test]
    fn valid_attack_target_represented_rejects_cpp_unit_state_flags_and_immunities() {
        assert!(!Unit::is_valid_attack_target_represented_like_cpp(
            &UnitAttackContextLikeCpp {
                visibility_represented: true,
                attacker_can_see_or_detect_target: false,
                ..Default::default()
            }
        ));
        assert!(!Unit::is_valid_attack_target_represented_like_cpp(
            &UnitAttackContextLikeCpp {
                victim_unit_state: UnitState::IN_FLIGHT.bits(),
                ..Default::default()
            }
        ));
        for flag in [
            UnitFlags::NON_ATTACKABLE,
            UnitFlags::NON_ATTACKABLE_2,
            UnitFlags::ON_TAXI,
            UnitFlags::NOT_ATTACKABLE_1,
            UnitFlags::UNINTERACTIBLE,
        ] {
            assert!(!Unit::is_valid_attack_target_represented_like_cpp(
                &UnitAttackContextLikeCpp {
                    victim_unit_flags: flag.bits(),
                    ..Default::default()
                }
            ));
        }
        assert!(!Unit::is_valid_attack_target_represented_like_cpp(
            &UnitAttackContextLikeCpp {
                attacker_is_player_uber: true,
                ..Default::default()
            }
        ));
        assert!(!Unit::is_valid_attack_target_represented_like_cpp(
            &UnitAttackContextLikeCpp {
                victim_unit_flags: UnitFlags::IMMUNE_TO_NPC.bits(),
                ..Default::default()
            }
        ));
        assert!(!Unit::is_valid_attack_target_represented_like_cpp(
            &UnitAttackContextLikeCpp {
                attacker_unit_flags: UnitFlags::IMMUNE_TO_NPC.bits(),
                ..Default::default()
            }
        ));
        assert!(!Unit::is_valid_attack_target_represented_like_cpp(
            &UnitAttackContextLikeCpp {
                attacker_unit_flags: UnitFlags::PLAYER_CONTROLLED.bits(),
                victim_unit_flags: UnitFlags::IMMUNE_TO_PC.bits(),
                ..Default::default()
            }
        ));
        assert!(!Unit::is_valid_attack_target_represented_like_cpp(
            &UnitAttackContextLikeCpp {
                attacker_unit_flags: UnitFlags::IMMUNE_TO_PC.bits(),
                victim_unit_flags: UnitFlags::PLAYER_CONTROLLED.bits(),
                ..Default::default()
            }
        ));
        assert!(Unit::is_valid_attack_target_represented_like_cpp(
            &UnitAttackContextLikeCpp {
                attacker_unit_flags: UnitFlags::PLAYER_CONTROLLED.bits(),
                victim_unit_flags: UnitFlags::IMMUNE_TO_NPC.bits(),
                ..Default::default()
            }
        ));
    }

    #[test]
    fn can_see_or_detect_unit_like_cpp_rejects_gm_visibility_above_detect() {
        let mut seer = Unit::new(true);
        let mut target = Unit::new(true);

        target.set_server_side_gm_visibility_like_cpp(2);
        seer.set_server_side_gm_visibility_detect_like_cpp(1);
        assert!(!seer.can_see_or_detect_unit_like_cpp(&target, false, true, false));

        seer.set_server_side_gm_visibility_detect_like_cpp(2);
        assert!(seer.can_see_or_detect_unit_like_cpp(&target, false, true, false));

        target.set_invisibility_like_cpp(0, 100);
        assert!(seer.can_see_or_detect_unit_like_cpp(&target, false, true, false));
    }

    #[test]
    fn can_detect_invisibility_like_cpp_requires_flag_and_sufficient_value() {
        let mut seer = Unit::new(true);
        let mut target = Unit::new(true);

        target.set_invisibility_like_cpp(3, 25);
        assert!(!seer.can_see_or_detect_unit_like_cpp(&target, false, true, false));

        seer.set_invisibility_detect_like_cpp(2, 100);
        assert!(!seer.can_detect_invisibility_of_like_cpp(&target));

        seer.set_invisibility_detect_like_cpp(3, 24);
        assert!(!seer.can_detect_invisibility_of_like_cpp(&target));

        seer.set_invisibility_detect_like_cpp(3, 25);
        assert!(seer.can_see_or_detect_unit_like_cpp(&target, false, true, false));
        assert!(seer.can_see_or_detect_unit_like_cpp(&target, true, true, false));

        target.set_invisibility_like_cpp(37, 12);
        assert!(!seer.can_detect_invisibility_of_like_cpp(&target));
        seer.set_invisibility_detect_like_cpp(37, 12);
        assert!(seer.can_detect_invisibility_of_like_cpp(&target));
    }

    #[test]
    fn can_detect_stealth_like_cpp_uses_front_arc_level_distance_and_player_cap() {
        let mut seer = Unit::new(true);
        let mut target = Unit::new(true);
        seer.set_level(10);
        seer.world_mut()
            .relocate(wow_core::Position::new(0.0, 0.0, 0.0, 0.0));
        target
            .world_mut()
            .relocate(wow_core::Position::new(20.0, 0.0, 0.0, 0.0));
        target.set_stealth_like_cpp(0, 1);
        assert!(seer.can_detect_stealth_of_like_cpp(&target, true, false));

        target.set_stealth_like_cpp(0, 100);
        assert!(!seer.can_detect_stealth_of_like_cpp(&target, true, false));

        seer.set_stealth_detect_like_cpp(0, 100);
        assert!(seer.can_detect_stealth_of_like_cpp(&target, true, false));

        target
            .world_mut()
            .relocate(wow_core::Position::new(-20.0, 0.0, 0.0, 0.0));
        assert!(!seer.can_detect_stealth_of_like_cpp(&target, true, false));

        target
            .world_mut()
            .relocate(wow_core::Position::new(35.0, 0.0, 0.0, 0.0));
        seer.set_level(80);
        target.set_stealth_like_cpp(0, 1);
        assert!(!seer.can_detect_stealth_of_like_cpp(&target, true, false));
        assert!(seer.can_detect_stealth_of_like_cpp(&target, false, false));
    }

    #[test]
    fn can_see_or_detect_unit_like_cpp_applies_cpp_visibility_gates_before_detection() {
        let mut seer = Unit::new(true);
        let mut target = Unit::new(true);
        let seer_guid = ObjectGuid::new(1, 11);
        let owner_guid = ObjectGuid::new(1, 12);
        seer.world_mut().object_mut().create(seer_guid);
        target
            .world_mut()
            .object_mut()
            .create(ObjectGuid::new(1, 13));

        target.set_never_visible_for_seer_like_cpp(true);
        target.set_always_visible_for_seer_like_cpp(true);
        assert!(!seer.can_see_or_detect_unit_like_cpp(&target, false, true, false));

        target.set_never_visible_for_seer_like_cpp(false);
        seer.set_seer_can_never_see_target_like_cpp(true);
        target.set_always_visible_for_seer_like_cpp(true);
        assert!(!seer.can_see_or_detect_unit_like_cpp(&target, false, true, false));

        seer.set_seer_can_never_see_target_like_cpp(false);
        target.set_always_visible_for_seer_like_cpp(false);
        target
            .subsystems_mut()
            .control
            .set_owner_guid(Some(seer_guid));
        target.set_invisibility_like_cpp(0, 100);
        assert!(seer.can_see_or_detect_unit_like_cpp(&target, false, true, false));

        target.subsystems_mut().control.set_owner_guid(None);
        assert!(!seer.can_see_or_detect_unit_like_cpp(&target, false, true, false));
        target.set_target_owner_group_visible_for_seer_like_cpp(true);
        assert!(seer.can_see_or_detect_unit_like_cpp(&target, false, true, false));

        target.set_target_owner_group_visible_for_seer_like_cpp(false);
        target.set_invisibility_like_cpp(0, 0);
        target.set_private_object_owner_like_cpp(owner_guid);
        assert!(!seer.can_see_or_detect_unit_like_cpp(&target, false, true, false));

        seer.set_seer_group_visible_for_private_owner_like_cpp(true);
        assert!(seer.can_see_or_detect_unit_like_cpp(&target, false, true, false));

        seer.set_seer_group_visible_for_private_owner_like_cpp(false);
        seer.set_seer_private_object_owner_like_cpp(owner_guid);
        assert!(seer.can_see_or_detect_unit_like_cpp(&target, false, true, false));

        seer.set_seer_private_object_owner_like_cpp(ObjectGuid::EMPTY);
        target.set_private_object_owner_like_cpp(seer_guid);
        assert!(seer.can_see_or_detect_unit_like_cpp(&target, false, true, false));

        target.set_private_object_owner_like_cpp(ObjectGuid::EMPTY);
        target
            .world_mut()
            .get_or_create_smooth_phasing_like_cpp()
            .set_viewer_dependent_info_like_cpp(
                seer_guid,
                crate::SmoothPhasingInfoLikeCpp::default(),
            );
        target.set_always_detectable_for_seer_like_cpp(true);
        assert!(!seer.can_see_or_detect_unit_like_cpp(&target, false, true, false));

        target
            .world_mut()
            .smooth_phasing_mut_like_cpp()
            .unwrap()
            .disable_replacement_for_seer_like_cpp(seer_guid);
        assert!(seer.can_see_or_detect_unit_like_cpp(&target, false, true, false));

        target.set_object_id_visibility_conditions_met_like_cpp(false);
        assert!(!seer.can_see_or_detect_unit_like_cpp(&target, false, true, false));

        target.set_private_object_owner_like_cpp(seer_guid);
        assert!(seer.can_see_or_detect_unit_like_cpp(&target, false, true, false));

        target.set_private_object_owner_like_cpp(ObjectGuid::EMPTY);
        target.set_object_id_visibility_conditions_met_like_cpp(true);
        target.set_always_detectable_for_seer_like_cpp(false);
        target.set_invisibility_like_cpp(0, 100);
        assert!(!seer.can_see_or_detect_unit_like_cpp(&target, false, true, false));

        target
            .subsystems_mut()
            .auras
            .register_applied_aura_type_like_cpp(
                AppliedAuraRef::new(53338, seer_guid, 0, 0x1),
                SPELL_AURA_MOD_STALKED_LIKE_CPP,
            );
        assert!(seer.can_see_or_detect_unit_like_cpp(&target, false, true, false));

        target
            .subsystems_mut()
            .auras
            .remove_auras_by_type_like_cpp(SPELL_AURA_MOD_STALKED_LIKE_CPP);
        seer.set_seer_can_always_see_target_guid_like_cpp(target.world().object().guid());
        assert!(seer.can_see_or_detect_unit_like_cpp(&target, false, true, false));
    }

    #[test]
    fn can_see_or_detect_unit_like_cpp_applies_ghost_despawn_and_always_detectable_gates() {
        let mut seer = Unit::new(true);
        let mut target = Unit::new(true);

        target.set_server_side_ghost_visibility_like_cpp(0x2);
        assert!(!seer.can_see_or_detect_unit_like_cpp(&target, false, true, false));

        target.set_ghost_visible_to_seer_by_group_like_cpp(true);
        assert!(seer.can_see_or_detect_unit_like_cpp(&target, false, true, false));

        target.set_ghost_visible_to_seer_by_group_like_cpp(false);
        seer.set_server_side_ghost_visibility_detect_like_cpp(0x2);
        assert!(seer.can_see_or_detect_unit_like_cpp(&target, false, true, false));

        target.set_invisible_due_to_despawn_like_cpp(true);
        assert!(!seer.can_see_or_detect_unit_like_cpp(&target, false, true, false));

        target.set_invisible_due_to_despawn_like_cpp(false);
        target.set_invisibility_like_cpp(0, 100);
        assert!(!seer.can_see_or_detect_unit_like_cpp(&target, false, true, false));

        target.set_always_detectable_for_seer_like_cpp(true);
        assert!(seer.can_see_or_detect_unit_like_cpp(&target, false, true, false));
    }

    #[test]
    fn valid_attack_target_represented_applies_cpp_relation_rules_when_known() {
        assert!(!Unit::is_valid_attack_target_represented_like_cpp(
            &UnitAttackContextLikeCpp {
                relation_represented: true,
                ..Default::default()
            }
        ));
        assert!(Unit::is_valid_attack_target_represented_like_cpp(
            &UnitAttackContextLikeCpp {
                relation_represented: true,
                attacker_is_hostile_to_victim: true,
                ..Default::default()
            }
        ));
        assert!(Unit::is_valid_attack_target_represented_like_cpp(
            &UnitAttackContextLikeCpp {
                relation_represented: true,
                victim_is_hostile_to_attacker: true,
                ..Default::default()
            }
        ));
        assert!(!Unit::is_valid_attack_target_represented_like_cpp(
            &UnitAttackContextLikeCpp {
                relation_represented: true,
                attacker_unit_flags: UnitFlags::PLAYER_CONTROLLED.bits(),
                victim_is_friendly_to_attacker: true,
                ..Default::default()
            }
        ));
        assert!(Unit::is_valid_attack_target_represented_like_cpp(
            &UnitAttackContextLikeCpp {
                relation_represented: true,
                attacker_unit_flags: UnitFlags::PLAYER_CONTROLLED.bits(),
                victim_unit_flags: UnitFlags::PLAYER_CONTROLLED.bits(),
                attacker_is_hostile_to_victim: true,
                ..Default::default()
            }
        ));
    }

    #[test]
    fn valid_attack_target_represented_rejects_npc_attacking_mounted_player_pet_like_cpp() {
        assert!(!Unit::is_valid_attack_target_represented_like_cpp(
            &UnitAttackContextLikeCpp {
                victim_unit_flags: UnitFlags::PLAYER_CONTROLLED.bits(),
                victim_has_affecting_player: true,
                victim_is_pet: true,
                victim_affecting_player_is_mounted: true,
                ..Default::default()
            }
        ));
        assert!(!Unit::is_valid_attack_target_represented_like_cpp(
            &UnitAttackContextLikeCpp {
                attacker_has_affecting_player: true,
                victim_unit_flags: UnitFlags::PLAYER_CONTROLLED.bits(),
                victim_has_affecting_player: true,
                victim_is_pet: true,
                victim_affecting_player_is_mounted: true,
                ..Default::default()
            }
        ));
        assert!(Unit::is_valid_attack_target_represented_like_cpp(
            &UnitAttackContextLikeCpp {
                attacker_has_affecting_player: true,
                victim_unit_flags: UnitFlags::PLAYER_CONTROLLED.bits(),
                victim_has_affecting_player: true,
                victim_is_pet: true,
                victim_affecting_player_is_mounted: true,
                pvp_represented: true,
                victim_is_pvp: true,
                ..Default::default()
            }
        ));
    }

    #[test]
    fn valid_attack_target_represented_applies_cpp_player_creature_reputation_rules() {
        assert!(!Unit::is_valid_attack_target_represented_like_cpp(
            &UnitAttackContextLikeCpp {
                attacker_has_affecting_player: true,
                player_creature_reputation_represented: true,
                ..Default::default()
            }
        ));
        assert!(Unit::is_valid_attack_target_represented_like_cpp(
            &UnitAttackContextLikeCpp {
                attacker_has_affecting_player: true,
                player_creature_reputation_represented: true,
                player_at_war_with_creature_faction: true,
                ..Default::default()
            }
        ));
        assert!(Unit::is_valid_attack_target_represented_like_cpp(
            &UnitAttackContextLikeCpp {
                attacker_has_affecting_player: true,
                player_creature_reputation_represented: true,
                creature_has_forced_reputation_rank: true,
                ..Default::default()
            }
        ));
        assert!(Unit::is_valid_attack_target_represented_like_cpp(
            &UnitAttackContextLikeCpp {
                attacker_has_affecting_player: true,
                player_creature_reputation_represented: true,
                creature_is_contested_guard: true,
                player_has_contested_pvp_flag: true,
                ..Default::default()
            }
        ));
    }

    #[test]
    fn valid_attack_target_represented_applies_cpp_duel_sanctuary_and_pvp_rules() {
        let player_pair = UnitAttackContextLikeCpp {
            attacker_unit_flags: UnitFlags::PLAYER_CONTROLLED.bits(),
            victim_unit_flags: UnitFlags::PLAYER_CONTROLLED.bits(),
            attacker_has_affecting_player: true,
            victim_has_affecting_player: true,
            ..Default::default()
        };

        assert!(Unit::is_valid_attack_target_represented_like_cpp(
            &UnitAttackContextLikeCpp {
                player_player_duel_in_progress: true,
                sanctuary_represented: true,
                attacker_in_sanctuary: true,
                ..player_pair.clone()
            }
        ));
        assert!(!Unit::is_valid_attack_target_represented_like_cpp(
            &UnitAttackContextLikeCpp {
                sanctuary_represented: true,
                victim_in_sanctuary: true,
                ..player_pair.clone()
            }
        ));
        assert!(!Unit::is_valid_attack_target_represented_like_cpp(
            &UnitAttackContextLikeCpp {
                ..player_pair.clone()
            }
        ));
        assert!(!Unit::is_valid_attack_target_represented_like_cpp(
            &UnitAttackContextLikeCpp {
                pvp_represented: true,
                ..player_pair.clone()
            }
        ));
        assert!(Unit::is_valid_attack_target_represented_like_cpp(
            &UnitAttackContextLikeCpp {
                pvp_represented: true,
                victim_is_pvp: true,
                ..player_pair.clone()
            }
        ));
        assert!(Unit::is_valid_attack_target_represented_like_cpp(
            &UnitAttackContextLikeCpp {
                pvp_represented: true,
                attacker_is_ffa_pvp: true,
                victim_is_ffa_pvp: true,
                ..player_pair.clone()
            }
        ));
        assert!(Unit::is_valid_attack_target_represented_like_cpp(
            &UnitAttackContextLikeCpp {
                pvp_represented: true,
                attacker_has_pvp_unk1_flag: true,
                ..player_pair
            }
        ));
    }

    #[test]
    fn attack_like_cpp_rejects_represented_invalid_attack_target() {
        let mut unit = Unit::new(true);
        let attacker = ObjectGuid::new(1, 9);
        let victim = ObjectGuid::new(1, 10);
        unit.world_mut().object_mut().create(attacker);

        assert_eq!(
            unit.attack_with_context_like_cpp(
                victim,
                true,
                true,
                true,
                UnitAttackContextLikeCpp {
                    victim_unit_flags: UnitFlags::NON_ATTACKABLE.bits(),
                    ..Default::default()
                },
            ),
            UnitAttackStartOutcome::InvalidAttackTarget
        );
        assert_eq!(unit.attacking(), None);
        assert_eq!(unit.data().target, ObjectGuid::EMPTY);
    }

    #[test]
    fn attack_like_cpp_removes_unattackable_aura_type_before_melee_state() {
        let mut unit = Unit::new(true);
        let attacker = ObjectGuid::new(1, 9);
        let victim = ObjectGuid::new(1, 10);
        let aura = AppliedAuraRef::new(400, attacker, 0, 0x1);
        unit.world_mut().object_mut().create(attacker);
        unit.subsystems_mut()
            .auras
            .register_applied_aura_type_like_cpp(aura, SPELL_AURA_MOD_UNATTACKABLE_LIKE_CPP);

        assert!(
            unit.subsystems()
                .auras
                .has_aura_type_like_cpp(SPELL_AURA_MOD_UNATTACKABLE_LIKE_CPP)
        );

        assert_eq!(
            unit.attack_like_cpp(victim, true, true, true),
            UnitAttackStartOutcome::NewTarget { previous: None }
        );

        assert!(!unit.subsystems().auras.has_applied(aura));
        assert!(
            !unit
                .subsystems()
                .auras
                .has_aura_type_like_cpp(SPELL_AURA_MOD_UNATTACKABLE_LIKE_CPP)
        );
        assert_eq!(unit.subsystems().auras.removed_count(), 1);
        assert_eq!(unit.attacking(), Some(victim));
    }

    #[test]
    fn attacker_state_update_melee_guards_match_cpp() {
        let mut unit = Unit::new(true);
        let attacker = ObjectGuid::new(1, 30);
        unit.world_mut().object_mut().create(attacker);
        assert!(unit.can_attacker_state_update_melee_like_cpp(false));

        unit.set_unit_flags_like_cpp(UnitFlags::PACIFIED);
        assert!(!unit.can_attacker_state_update_melee_like_cpp(false));
        unit.set_unit_flags_like_cpp(UnitFlags::empty());

        unit.add_unit_state(UnitState::STUNNED.bits());
        assert!(!unit.can_attacker_state_update_melee_like_cpp(false));
        assert!(unit.can_attacker_state_update_melee_like_cpp(true));
        unit.clear_unit_state(UnitState::STUNNED.bits());

        let aura = AppliedAuraRef::new(402, attacker, 0, 0x1);
        unit.subsystems_mut()
            .auras
            .register_applied_aura_type_like_cpp(
                aura,
                SPELL_AURA_DISABLE_ATTACKING_EXCEPT_ABILITIES_LIKE_CPP,
            );
        assert!(!unit.can_attacker_state_update_melee_like_cpp(false));
    }

    #[test]
    fn attacker_state_update_removes_attacking_interrupt_auras_like_cpp() {
        let mut unit = Unit::new(true);
        let attacker = ObjectGuid::new(1, 31);
        unit.world_mut().object_mut().create(attacker);
        let removed_by_attacking = AppliedAuraRef::new(403, attacker, 0, 0x1);
        let kept = AppliedAuraRef::new(404, attacker, 0, 0x2);
        unit.subsystems_mut().auras.register_applied_aura(
            removed_by_attacking,
            None,
            SPELL_AURA_INTERRUPT_FLAG_ATTACKING_LIKE_CPP,
            0,
        );
        unit.subsystems_mut()
            .auras
            .register_applied_aura(kept, None, 0x20, 0);

        assert_eq!(unit.remove_attacking_interrupt_auras_like_cpp(), 1);
        assert!(!unit.subsystems().auras.has_applied(removed_by_attacking));
        assert!(unit.subsystems().auras.has_applied(kept));
    }

    #[test]
    fn attack_timers_update_ready_and_reset_like_cpp() {
        let mut unit = Unit::new(true);
        unit.set_base_attack_time_like_cpp(WeaponAttackType::BaseAttack, 2_000);
        unit.set_attack_timer(WeaponAttackType::BaseAttack, 250);

        assert!(!unit.is_attack_ready_like_cpp(WeaponAttackType::BaseAttack));
        unit.update_attack_timers_like_cpp(100);
        assert_eq!(unit.attack_timer(WeaponAttackType::BaseAttack), 150);
        unit.update_attack_timers_like_cpp(200);
        assert_eq!(unit.attack_timer(WeaponAttackType::BaseAttack), 0);
        assert!(unit.is_attack_ready_like_cpp(WeaponAttackType::BaseAttack));
        unit.reset_attack_timer_like_cpp(WeaponAttackType::BaseAttack);
        assert_eq!(unit.attack_timer(WeaponAttackType::BaseAttack), 2_000);
    }

    #[test]
    fn attack_like_cpp_delays_non_player_offhand_timer_like_cpp() {
        let mut unit = Unit::new(true);
        let attacker = ObjectGuid::new(1, 9);
        let victim = ObjectGuid::new(1, 10);
        unit.world_mut().object_mut().create(attacker);
        unit.set_can_dual_wield_like_cpp(true);
        unit.set_base_attack_time_like_cpp(WeaponAttackType::BaseAttack, 2_000);
        unit.set_attack_timer(WeaponAttackType::BaseAttack, 600);
        unit.set_attack_timer(WeaponAttackType::OffAttack, 100);

        assert_eq!(
            unit.attack_like_cpp(victim, true, true, true),
            UnitAttackStartOutcome::NewTarget { previous: None }
        );
        assert_eq!(unit.attack_timer(WeaponAttackType::OffAttack), 1_600);

        let next_victim = ObjectGuid::new(1, 11);
        unit.set_attack_timer(WeaponAttackType::BaseAttack, 500);
        unit.set_attack_timer(WeaponAttackType::OffAttack, 2_200);
        assert_eq!(
            unit.attack_like_cpp(next_victim, true, true, true),
            UnitAttackStartOutcome::NewTarget {
                previous: Some(victim)
            }
        );
        assert_eq!(unit.attack_timer(WeaponAttackType::OffAttack), 2_200);
    }

    #[test]
    fn attack_like_cpp_does_not_delay_player_offhand_timer() {
        let mut player_unit = Unit::new(true);
        let attacker = ObjectGuid::new(1, 12);
        let victim = ObjectGuid::new(1, 13);
        player_unit.set_type(
            TypeId::Player,
            TypeMask::OBJECT | TypeMask::UNIT | TypeMask::PLAYER,
        );
        player_unit.world_mut().object_mut().create(attacker);
        player_unit.set_can_dual_wield_like_cpp(true);
        player_unit.set_base_attack_time_like_cpp(WeaponAttackType::BaseAttack, 2_000);
        player_unit.set_attack_timer(WeaponAttackType::BaseAttack, 600);
        player_unit.set_attack_timer(WeaponAttackType::OffAttack, 100);

        assert_eq!(
            player_unit.attack_like_cpp(victim, true, true, true),
            UnitAttackStartOutcome::NewTarget { previous: None }
        );
        assert_eq!(player_unit.attack_timer(WeaponAttackType::OffAttack), 100);
    }

    #[test]
    fn attack_like_cpp_applies_creature_ai_side_effects_for_uncontrolled_unit() {
        let mut unit = Unit::new(true);
        let attacker = ObjectGuid::new(1, 14);
        let victim = ObjectGuid::new(1, 15);
        unit.world_mut().object_mut().create(attacker);
        unit.set_emote_state_like_cpp(88);
        unit.set_stand_state_like_cpp(UnitStandStateType::Sit);

        assert_eq!(
            unit.attack_like_cpp(victim, true, true, true),
            UnitAttackStartOutcome::NewTarget { previous: None }
        );

        assert!(unit.subsystems().combat.is_threatened_by(victim));
        assert_eq!(unit.subsystems().ai.hostile_reaction_count, 1);
        assert_eq!(unit.subsystems().ai.call_assistance_count, 1);
        assert_eq!(unit.emote_state_like_cpp(), 0);
        assert_eq!(unit.stand_state_like_cpp(), UnitStandStateType::Stand);
    }

    #[test]
    fn attack_like_cpp_skips_creature_ai_side_effects_when_controlled_by_player() {
        let mut unit = Unit::new(true);
        let attacker = ObjectGuid::new(1, 16);
        let victim = ObjectGuid::new(1, 17);
        let charmer = ObjectGuid::create_player(1, 18);
        unit.world_mut().object_mut().create(attacker);
        unit.subsystems_mut().control.apply_charmed_by(
            charmer,
            crate::CharmType::Charm,
            true,
            None,
            false,
        );

        assert_eq!(
            unit.attack_like_cpp(victim, true, true, true),
            UnitAttackStartOutcome::NewTarget { previous: None }
        );

        assert!(!unit.subsystems().combat.is_threatened_by(victim));
        assert_eq!(unit.subsystems().ai.hostile_reaction_count, 0);
        assert_eq!(unit.subsystems().ai.call_assistance_count, 0);
    }

    #[test]
    fn player_attack_like_cpp_notifies_controlled_creature_ai_owner_attacked() {
        let mut player_unit = Unit::new(true);
        let player = ObjectGuid::create_player(1, 19);
        let victim = ObjectGuid::new(1, 20);
        let controlled_creature = ObjectGuid::new(1, 21);
        let controlled_without_ai = ObjectGuid::new(1, 22);
        let uncontrolled_creature = ObjectGuid::new(1, 23);
        player_unit.set_type(
            TypeId::Player,
            TypeMask::OBJECT | TypeMask::UNIT | TypeMask::PLAYER,
        );
        player_unit.world_mut().object_mut().create(player);
        assert!(
            player_unit
                .subsystems_mut()
                .control
                .add_controlled(controlled_creature)
        );
        assert!(
            player_unit
                .subsystems_mut()
                .control
                .add_controlled(controlled_without_ai)
        );

        assert_eq!(
            player_unit.attack_with_context_like_cpp(
                victim,
                true,
                true,
                true,
                UnitAttackContextLikeCpp {
                    controlled_creatures_with_ai: vec![controlled_creature, uncontrolled_creature],
                    ..Default::default()
                },
            ),
            UnitAttackStartOutcome::NewTarget { previous: None }
        );

        assert_eq!(
            player_unit
                .subsystems()
                .control
                .owner_attacked_notifications,
            vec![crate::ControlledOwnerAttackedNotification {
                controlled: controlled_creature,
                victim,
            }]
        );
    }

    #[test]
    fn unit_subsystem_helpers_do_not_mark_update_fields() {
        let mut unit = Unit::new(true);
        let caster = ObjectGuid::new(1, 1);
        let target = ObjectGuid::new(1, 2);

        unit.clear_unit_data_changes();
        let owned = OwnedAuraRef::new(17, caster, None);
        let applied = AppliedAuraRef::new(17, caster, 3, 0x7);
        unit.subsystems_mut().auras.add_owned(owned);
        unit.subsystems_mut().auras.add_applied(applied);
        unit.subsystems_mut()
            .auras
            .set_visible(3, AuraRef::new(17, caster));
        unit.subsystems_mut().spells.set_current_spell(
            CurrentSpellSlot::Channeled,
            CurrentSpellRef::new(42, Some(caster), None),
        );
        unit.subsystems_mut()
            .spells
            .history
            .set_cooldown(42, 100, 1_500);
        unit.subsystems_mut().combat.add_threat(target, 2.0);
        unit.subsystems_mut().motion.start_spline(9, 500);
        unit.subsystems_mut().control.set_charmer(caster, true);
        unit.subsystems_mut().vehicle.enter_vehicle(target, Some(0));
        unit.subsystems_mut().ai.push("TestAI");

        assert!(unit.subsystems().auras.has_owned(owned));
        assert!(unit.subsystems().auras.has_applied(applied));
        assert_eq!(
            unit.subsystems()
                .spells
                .current_spell(CurrentSpellSlot::Channeled)
                .map(|spell| spell.spell_id),
            Some(42)
        );
        assert_eq!(
            unit.subsystems()
                .spells
                .history
                .cooldown(42)
                .map(|cooldown| cooldown.cooldown_end_ms),
            Some(1_600)
        );
        assert!(unit.subsystems().combat.is_threatened_by(target));
        assert!(unit.subsystems().motion.spline.enabled);
        assert!(unit.subsystems().control.is_charmed());
        assert_eq!(unit.subsystems().vehicle.vehicle_guid, Some(target));
        assert_eq!(unit.subsystems().ai.active_ai.as_deref(), Some("TestAI"));
        assert!(!unit.unit_data_changes_mask().is_any_set());
    }

    #[test]
    fn current_spell_slots_follow_cpp_ids_and_breakage_rules() {
        assert_eq!(CurrentSpellSlot::Melee as u8, 0);
        assert_eq!(CurrentSpellSlot::Generic as u8, 1);
        assert_eq!(CurrentSpellSlot::Channeled as u8, 2);
        assert_eq!(CurrentSpellSlot::Autorepeat as u8, 3);

        let mut unit = Unit::new(true);
        let caster = ObjectGuid::new(1, 1);
        let generic = CurrentSpellRef::new(100, Some(caster), None).with_cast_time_ms(1_500);
        let auto_shot = CurrentSpellRef::new(AUTO_SHOT_SPELL_ID, Some(caster), None);
        let other_auto = CurrentSpellRef::new(200, Some(caster), None);

        unit.set_current_cast_spell(CurrentSpellSlot::Autorepeat, auto_shot);
        unit.set_current_cast_spell(CurrentSpellSlot::Generic, generic);
        assert_eq!(unit.current_spell(CurrentSpellSlot::Generic), Some(generic));
        assert_eq!(
            unit.current_spell(CurrentSpellSlot::Autorepeat),
            Some(auto_shot)
        );
        assert!(unit.has_unit_state(UnitState::CASTING.bits()));

        unit.set_current_cast_spell(CurrentSpellSlot::Autorepeat, other_auto);
        assert_eq!(unit.current_spell(CurrentSpellSlot::Generic), None);
        assert_eq!(
            unit.current_spell(CurrentSpellSlot::Autorepeat),
            Some(other_auto)
        );
        assert!(!unit.has_unit_state(UnitState::CASTING.bits()));
    }

    #[test]
    fn current_spell_generic_respects_channels_that_allow_actions() {
        let mut unit = Unit::new(true);
        let caster = ObjectGuid::new(1, 1);
        let channel_with_actions = CurrentSpellRef::new(300, Some(caster), None)
            .with_cast_time_ms(2_000)
            .with_allow_actions_during_channel(true);
        let generic = CurrentSpellRef::new(301, Some(caster), None).with_cast_time_ms(1_000);

        unit.set_current_cast_spell(CurrentSpellSlot::Channeled, channel_with_actions);
        unit.set_current_cast_spell(CurrentSpellSlot::Generic, generic);
        assert_eq!(
            unit.current_spell(CurrentSpellSlot::Channeled),
            Some(channel_with_actions)
        );
        assert_eq!(unit.current_spell(CurrentSpellSlot::Generic), Some(generic));

        let regular_channel =
            CurrentSpellRef::new(302, Some(caster), None).with_cast_time_ms(2_000);
        let next_generic = CurrentSpellRef::new(303, Some(caster), None).with_cast_time_ms(1_000);
        unit.set_current_cast_spell(CurrentSpellSlot::Channeled, regular_channel);
        unit.set_current_cast_spell(CurrentSpellSlot::Generic, next_generic);
        assert_eq!(unit.current_spell(CurrentSpellSlot::Channeled), None);
        assert_eq!(
            unit.current_spell(CurrentSpellSlot::Generic),
            Some(next_generic)
        );
    }

    #[test]
    fn interrupt_spell_honors_cpp_delayed_instant_and_interruptible_guards() {
        let mut unit = Unit::new(true);
        let caster = ObjectGuid::new(1, 1);
        let instant = CurrentSpellRef::new(400, Some(caster), None);
        let delayed = CurrentSpellRef::new(401, Some(caster), None)
            .with_cast_time_ms(1_000)
            .with_state(SpellState::Delayed);
        let casting_instant =
            CurrentSpellRef::new(402, Some(caster), None).with_state(SpellState::Casting);
        let protected = CurrentSpellRef::new(403, Some(caster), None)
            .with_cast_time_ms(1_000)
            .with_interruptible(false);

        unit.set_current_cast_spell(CurrentSpellSlot::Generic, instant);
        assert_eq!(
            unit.interrupt_spell(CurrentSpellSlot::Generic, true, false),
            None
        );
        assert_eq!(unit.current_spell(CurrentSpellSlot::Generic), Some(instant));

        unit.set_current_cast_spell(CurrentSpellSlot::Generic, delayed);
        assert_eq!(
            unit.interrupt_spell(CurrentSpellSlot::Generic, false, true),
            None
        );
        assert_eq!(unit.current_spell(CurrentSpellSlot::Generic), Some(delayed));

        unit.set_current_cast_spell(CurrentSpellSlot::Generic, casting_instant);
        assert_eq!(
            unit.interrupt_spell(CurrentSpellSlot::Generic, true, false),
            Some(casting_instant)
        );

        unit.set_current_cast_spell(CurrentSpellSlot::Generic, protected);
        assert_eq!(
            unit.interrupt_spell(CurrentSpellSlot::Generic, true, true),
            None
        );
        assert_eq!(
            unit.current_spell(CurrentSpellSlot::Generic),
            Some(protected)
        );
        assert_eq!(
            unit.finish_spell(CurrentSpellSlot::Generic),
            Some(protected)
        );
        assert_eq!(unit.current_spell(CurrentSpellSlot::Generic), None);
    }

    #[test]
    fn interrupt_non_melee_spells_filters_and_forces_channeled_interrupts() {
        let mut unit = Unit::new(true);
        let caster = ObjectGuid::new(1, 1);
        let melee = CurrentSpellRef::new(500, Some(caster), None);
        let generic = CurrentSpellRef::new(501, Some(caster), None).with_cast_time_ms(1_000);
        let auto = CurrentSpellRef::new(502, Some(caster), None);
        let delayed_channel = CurrentSpellRef::new(503, Some(caster), None)
            .with_state(SpellState::Delayed)
            .with_cast_time_ms(1_000);

        unit.subsystems_mut()
            .spells
            .set_current_spell(CurrentSpellSlot::Melee, melee);
        unit.subsystems_mut()
            .spells
            .set_current_spell(CurrentSpellSlot::Generic, generic);
        unit.subsystems_mut()
            .spells
            .set_current_spell(CurrentSpellSlot::Autorepeat, auto);
        unit.subsystems_mut()
            .spells
            .set_current_spell(CurrentSpellSlot::Channeled, delayed_channel);

        let removed = unit.interrupt_non_melee_spells(Some(503), false, false);
        assert_eq!(
            removed,
            vec![(CurrentSpellSlot::Channeled, delayed_channel)]
        );
        assert_eq!(unit.current_spell(CurrentSpellSlot::Melee), Some(melee));
        assert_eq!(unit.current_spell(CurrentSpellSlot::Generic), Some(generic));
        assert_eq!(unit.current_spell(CurrentSpellSlot::Autorepeat), Some(auto));

        let removed = unit.interrupt_non_melee_spells(None, true, true);
        assert_eq!(
            removed,
            vec![
                (CurrentSpellSlot::Generic, generic),
                (CurrentSpellSlot::Autorepeat, auto),
            ]
        );
        assert_eq!(unit.current_spell(CurrentSpellSlot::Melee), Some(melee));
    }

    #[test]
    fn find_current_spell_by_spell_id_searches_all_cpp_slots() {
        let mut unit = Unit::new(true);
        let caster = ObjectGuid::new(1, 1);
        let melee = CurrentSpellRef::new(600, Some(caster), None);
        let channel = CurrentSpellRef::new(601, Some(caster), None).with_cast_time_ms(1_000);

        unit.set_current_cast_spell(CurrentSpellSlot::Melee, melee);
        unit.set_current_cast_spell(CurrentSpellSlot::Channeled, channel);

        assert_eq!(unit.find_current_spell_by_spell_id(600), Some(melee));
        assert_eq!(unit.find_current_spell_by_spell_id(601), Some(channel));
        assert_eq!(unit.find_current_spell_by_spell_id(602), None);
    }

    #[test]
    fn health_and_max_health_follow_cpp_clamps() {
        let mut unit = Unit::new(true);

        unit.set_max_health(0);
        assert_eq!(unit.data().max_health, 1);
        assert!(
            unit.unit_data_changes_mask()
                .is_set(UNIT_DATA_MAX_HEALTH_BIT)
        );

        unit.clear_unit_data_changes();
        unit.set_max_health(100);
        unit.set_health(150);
        assert_eq!(unit.data().health, 100);
        assert!(unit.unit_data_changes_mask().is_set(UNIT_DATA_HEALTH_BIT));

        unit.clear_unit_data_changes();
        unit.set_max_health(40);
        assert_eq!(unit.data().max_health, 40);
        assert_eq!(unit.data().health, 40);
        assert!(
            unit.unit_data_changes_mask()
                .is_set(UNIT_DATA_MAX_HEALTH_BIT)
        );
        assert!(unit.unit_data_changes_mask().is_set(UNIT_DATA_HEALTH_BIT));

        unit.clear_unit_data_changes();
        unit.set_death_state(DeathState::Corpse);
        unit.set_health(30);
        assert_eq!(unit.data().health, 0);
    }

    #[test]
    fn stand_state_helpers_match_cpp_sit_sleep_kneel_rules() {
        let mut unit = Unit::new(true);
        assert_eq!(unit.stand_state_like_cpp(), UnitStandStateType::Stand);
        assert!(unit.is_stand_state_like_cpp());

        unit.clear_unit_data_changes();
        unit.set_stand_state_like_cpp(UnitStandStateType::SitChair);
        assert_eq!(unit.stand_state_like_cpp(), UnitStandStateType::SitChair);
        assert!(!unit.is_stand_state_like_cpp());
        assert!(
            unit.unit_data_changes_mask()
                .is_set(UNIT_DATA_STAND_STATE_BIT)
        );

        unit.set_stand_state_like_cpp(UnitStandStateType::Sleep);
        assert!(!unit.is_stand_state_like_cpp());
        unit.set_stand_state_like_cpp(UnitStandStateType::Kneel);
        assert!(!unit.is_stand_state_like_cpp());
        unit.set_stand_state_like_cpp(UnitStandStateType::Dead);
        assert!(unit.is_stand_state_like_cpp());
        unit.set_stand_state_like_cpp(UnitStandStateType::Submerged);
        assert!(unit.is_stand_state_like_cpp());
    }

    #[test]
    fn npc_flags_setters_mark_cpp_parent_and_element_bits() {
        let mut unit = Unit::new(true);
        unit.clear_unit_data_changes();

        unit.set_npc_flags_like_cpp(0x40);
        unit.set_npc_flags2_like_cpp(0x1);

        assert_eq!(unit.npc_flags_like_cpp(), [0x40, 0x1]);
        assert!(
            unit.unit_data_changes_mask()
                .is_set(UNIT_DATA_NPC_FLAGS_PARENT_BIT)
        );
        assert!(
            unit.unit_data_changes_mask()
                .is_set(UNIT_DATA_NPC_FLAGS_FIRST_BIT)
        );
        assert!(
            unit.unit_data_changes_mask()
                .is_set(UNIT_DATA_NPC_FLAGS_FIRST_BIT + 1)
        );
    }

    #[test]
    fn power_setters_use_derived_power_index_and_cpp_clamps() {
        let mut unit = Unit::new(true);

        assert_eq!(unit.get_power(PowerType::Energy), 0);
        unit.set_power(PowerType::Energy, 10);
        assert!(
            !unit
                .unit_data_changes_mask()
                .is_set(UNIT_DATA_POWER_PARENT_BIT)
        );

        unit.set_power_index(PowerType::Energy, Some(3));
        unit.set_max_power(PowerType::Energy, 100);
        unit.set_power(PowerType::Energy, 150);

        assert_eq!(unit.get_power_index(PowerType::Energy), Some(3));
        assert_eq!(unit.get_power(PowerType::Energy), 100);
        assert_eq!(unit.get_max_power(PowerType::Energy), 100);
        assert!(
            unit.unit_data_changes_mask()
                .is_set(UNIT_DATA_POWER_PARENT_BIT)
        );
        assert!(
            unit.unit_data_changes_mask()
                .is_set(UNIT_DATA_POWER_FIRST_BIT + 3)
        );
        assert!(
            unit.unit_data_changes_mask()
                .is_set(UNIT_DATA_MAX_POWER_FIRST_BIT + 3)
        );
    }

    #[test]
    fn virtual_item_updates_mark_cpp_parent_and_element_bits() {
        let mut unit = Unit::new(true);
        unit.clear_unit_data_changes();

        unit.set_virtual_item(
            1,
            Some(VisibleItemValues {
                item_id: 19019,
                item_appearance_mod_id: 2,
                item_visual: 3,
            }),
        );

        assert_eq!(unit.data().virtual_items[1].item_id, 19019);
        assert!(
            unit.unit_data_changes_mask()
                .is_set(UNIT_DATA_VIRTUAL_ITEMS_PARENT_BIT)
        );
        assert!(
            unit.unit_data_changes_mask()
                .is_set(UNIT_DATA_VIRTUAL_ITEMS_FIRST_BIT + 1)
        );
        assert!(
            !unit
                .unit_data_changes_mask()
                .is_set(UNIT_DATA_VIRTUAL_ITEMS_FIRST_BIT)
        );

        unit.clear_unit_data_changes();
        unit.set_virtual_item(1, None);
        assert_eq!(unit.data().virtual_items[1], VisibleItemValues::default());
        assert!(
            unit.unit_data_changes_mask()
                .is_set(UNIT_DATA_VIRTUAL_ITEMS_PARENT_BIT)
        );
        assert!(
            unit.unit_data_changes_mask()
                .is_set(UNIT_DATA_VIRTUAL_ITEMS_FIRST_BIT + 1)
        );
    }

    #[test]
    fn virtual_item_mark_changed_forces_default_value_delta() {
        let mut unit = Unit::new(true);
        unit.clear_unit_data_changes();

        unit.mark_virtual_item_changed(2);

        assert_eq!(unit.data().virtual_items[2], VisibleItemValues::default());
        assert!(
            unit.unit_data_changes_mask()
                .is_set(UNIT_DATA_VIRTUAL_ITEMS_PARENT_BIT)
        );
        assert!(
            unit.unit_data_changes_mask()
                .is_set(UNIT_DATA_VIRTUAL_ITEMS_FIRST_BIT + 2)
        );
    }

    #[test]
    fn unit_collision_height_setter_delegates_to_embedded_world_object() {
        let mut unit = Unit::new(true);

        unit.set_collision_height_like_cpp(2.03128);
        assert_eq!(unit.collision_height_like_cpp(), 2.03128);
        assert_eq!(unit.world().collision_height_like_cpp(), 2.03128);

        unit.set_collision_height_like_cpp(-10.0);
        assert_eq!(unit.collision_height_like_cpp(), 0.0);
        assert_eq!(unit.world().collision_height_like_cpp(), 0.0);
    }

    #[test]
    fn unit_set_combat_reach_keeps_unit_data_and_world_object_coherent() {
        let mut unit = Unit::new(true);

        unit.set_combat_reach(1.75);
        assert_eq!(unit.data().combat_reach, 1.75);
        assert_eq!(unit.world().combat_reach(), 1.75);

        unit.set_combat_reach(-1.0);
        assert_eq!(unit.data().combat_reach, 0.0);
        assert_eq!(unit.world().combat_reach(), 0.0);
    }

    #[test]
    fn display_level_faction_and_reach_mark_unitdata_bits() {
        let mut unit = Unit::new(true);

        unit.set_level(70);
        unit.set_race(1);
        unit.set_class(2);
        unit.set_player_class(2);
        unit.set_gender(Gender::Female);
        unit.set_target(ObjectGuid::new(7, 11));
        unit.set_faction(35);
        unit.set_bounding_radius(0.5);
        unit.set_combat_reach(1.5);
        unit.set_display_id(1234, true);

        assert_eq!(unit.data().level, 70);
        assert_eq!(unit.data().race, 1);
        assert_eq!(unit.data().class_id, 2);
        assert_eq!(unit.data().player_class_id, 2);
        assert_eq!(unit.data().sex, Gender::Female as u8);
        assert_eq!(unit.data().target, ObjectGuid::new(7, 11));
        assert_eq!(unit.data().faction_template, 35);
        assert_eq!(unit.data().bounding_radius, 0.5);
        assert_eq!(unit.data().combat_reach, 1.5);
        assert_eq!(unit.world().combat_reach(), 1.5);
        assert_eq!(unit.data().display_id, 1234);
        assert_eq!(unit.data().display_scale, DEFAULT_PLAYER_DISPLAY_SCALE);
        assert_eq!(unit.data().native_display_id, 1234);
        assert_eq!(
            unit.data().native_display_scale,
            DEFAULT_PLAYER_DISPLAY_SCALE
        );
        assert!(unit.unit_data_changes_mask().is_set(UNIT_DATA_PARENT_BIT));
        assert!(unit.unit_data_changes_mask().is_set(UNIT_DATA_LEVEL_BIT));
        assert!(unit.unit_data_changes_mask().is_set(UNIT_DATA_RACE_BIT));
        assert!(unit.unit_data_changes_mask().is_set(UNIT_DATA_CLASS_ID_BIT));
        assert!(
            unit.unit_data_changes_mask()
                .is_set(UNIT_DATA_PLAYER_CLASS_ID_BIT)
        );
        assert!(unit.unit_data_changes_mask().is_set(UNIT_DATA_SEX_BIT));
        assert!(unit.unit_data_changes_mask().is_set(UNIT_DATA_TARGET_BIT));
        assert!(
            unit.unit_data_changes_mask()
                .is_set(UNIT_DATA_FACTION_TEMPLATE_BIT)
        );
        assert!(
            unit.unit_data_changes_mask()
                .is_set(UNIT_DATA_BOUNDING_RADIUS_BIT)
        );
        assert!(
            unit.unit_data_changes_mask()
                .is_set(UNIT_DATA_COMBAT_REACH_BIT)
        );
        assert!(
            unit.unit_data_changes_mask()
                .is_set(UNIT_DATA_DISPLAY_ID_BIT)
        );
        assert!(
            unit.unit_data_changes_mask()
                .is_set(UNIT_DATA_DISPLAY_SCALE_BIT)
        );
        assert!(
            unit.unit_data_changes_mask()
                .is_set(UNIT_DATA_NATIVE_DISPLAY_ID_BIT)
        );
        assert!(
            unit.unit_data_changes_mask()
                .is_set(UNIT_DATA_NATIVE_DISPLAY_SCALE_BIT)
        );
    }

    #[test]
    fn values_update_sets_unit_object_type_bit() {
        let mut unit = Unit::new(true);

        unit.set_level(12);
        let update = unit.values_update();

        assert!(update.has_data());
        assert_eq!(update.changed_object_type_mask, 1 << TYPEID_UNIT);
        let unit_data = update.unit_data.unwrap();
        assert_eq!(unit_data.values.level, 12);
        assert!(unit_data.mask.is_set(UNIT_DATA_LEVEL_BIT));
    }

    #[test]
    fn pvp_flags_match_cpp_unit_pvp_state_helpers() {
        let mut unit = Unit::new(true);

        unit.set_pvp_flag_like_cpp(UnitPvpFlags::PVP | UnitPvpFlags::FFA_PVP);
        assert!(unit.is_pvp_like_cpp());
        assert!(unit.is_ffa_pvp_like_cpp());
        assert!(!unit.is_in_sanctuary_like_cpp());
        assert!(
            unit.unit_data_changes_mask()
                .is_set(UNIT_DATA_PVP_FLAGS_BIT)
        );

        unit.remove_pvp_flag_like_cpp(UnitPvpFlags::FFA_PVP);
        assert!(unit.is_pvp_like_cpp());
        assert!(!unit.is_ffa_pvp_like_cpp());

        unit.replace_all_pvp_flags_like_cpp(UnitPvpFlags::SANCTUARY | UnitPvpFlags::UNK1);
        assert!(!unit.is_pvp_like_cpp());
        assert!(unit.is_in_sanctuary_like_cpp());
        assert!(unit.has_pvp_flag_like_cpp(UnitPvpFlags::UNK1));
    }
}
