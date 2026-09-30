//! Unit construction, world presence, shared vision and movement fields.

use super::*;

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
            health_state_revision_like_cpp: HealthStateRevisionLikeCpp::new(),
            unit_state: 0,
            base_attack_speed: [0; MAX_ATTACK],
            mod_attack_speed_pct: [1.0; MAX_ATTACK],
            mod_autoattack_damage_pct: 1.0,
            attack_timer: [0; MAX_ATTACK],
            weapon_damage: [[BASE_MINDAMAGE, BASE_MAXDAMAGE]; MAX_ATTACK],
            can_dual_wield: false,
            can_parry: false,
            can_block: false,
            emote_state: 0,
            movement_flags: MovementFlag::NONE,
            movement_time: 0,
            speed_rate: [1.0; MAX_MOVE_TYPE],
            movement_counter_like_cpp: 0,
            movement_force_mod_magnitude_like_cpp: 1.0,
            ai_anim_kit_id: 0,
            movement_anim_kit_id: 0,
            melee_anim_kit_id: 0,
            power_index: [None; MAX_POWERS],
            visibility_detection: UnitVisibilityDetectionStateLikeCpp::default(),
            subsystems: UnitSubsystems::default(),
            power_regen: UnitPowerRegenStateLikeCpp::default(),
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
    pub const fn movement_flags_like_cpp(&self) -> MovementFlag {
        self.movement_flags
    }
    pub fn set_movement_flags_like_cpp(&mut self, flags: MovementFlag) {
        self.movement_flags = flags;
    }
    pub const fn movement_time_like_cpp(&self) -> u32 {
        self.movement_time
    }
    pub fn set_movement_time_like_cpp(&mut self, time: u32) {
        self.movement_time = time;
    }
}
