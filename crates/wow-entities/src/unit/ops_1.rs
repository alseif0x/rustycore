//! Unit values, visibility and health-revision state operations, part 1 of 3.
//!
//! The inherent `Unit` impl is divided by responsibility under
//! #636; every method keeps its original body.

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
        if self.death_state == state {
            return;
        }
        self.death_state = state;
        self.advance_health_state_revision_like_cpp();
    }
    pub const fn health_state_revision_like_cpp(&self) -> u64 {
        self.health_state_revision_like_cpp.value
    }
    pub fn health_state_revision_authority_like_cpp(&self) -> HealthStateRevisionAuthorityLikeCpp {
        self.health_state_revision_like_cpp.authority.clone()
    }
    pub fn shares_health_state_revision_authority_like_cpp(
        &self,
        authority: &HealthStateRevisionAuthorityLikeCpp,
    ) -> bool {
        self.health_state_revision_like_cpp
            .authority
            .shares_storage_like_cpp(authority)
    }
    /// Copy the canonical health tuple into a temporary whole-entity snapshot
    /// immediately before that snapshot replaces the canonical object.
    ///
    /// This deliberately copies the revision exactly: the authoritative state
    /// did not change, only the container object did. Callers must never use
    /// this on a live owner to replay an older mirror transition.
    pub fn preserve_authoritative_health_state_for_snapshot_like_cpp(
        &mut self,
        authoritative: &Self,
    ) {
        self.set_u64_field(
            UNIT_DATA_MAX_HEALTH_BIT,
            authoritative.data.max_health,
            |data| &mut data.max_health,
        );
        self.set_u64_field(UNIT_DATA_HEALTH_BIT, authoritative.data.health, |data| {
            &mut data.health
        });
        self.death_state = authoritative.death_state;
        self.health_state_revision_like_cpp = authoritative.health_state_revision_like_cpp.clone();
    }
    /// Mark a mirror replay as the exact already-committed health transition.
    ///
    /// Replaying normal setters is intentional because it runs the mirror's
    /// local death hooks, but those setters reserve fresh sequence values. The
    /// caller may invoke this only after verifying that the resulting
    /// health/death tuple and incarnation metadata exactly match the canonical
    /// commit. The shared allocator never moves backward; only this mirror's
    /// local state version is aligned with the committed version.
    pub fn adopt_committed_health_state_revision_for_mirror_like_cpp(
        &mut self,
        committed_revision: u64,
    ) {
        assert_ne!(
            committed_revision, 0,
            "a committed health transition must carry a nonzero revision"
        );
        self.health_state_revision_like_cpp
            .authority
            .allocator
            .fetch_max(committed_revision, Ordering::AcqRel);
        self.health_state_revision_like_cpp.value = committed_revision;
    }
    pub(super) fn advance_health_state_revision_like_cpp(&mut self) {
        self.health_state_revision_like_cpp.advance();
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
    pub fn is_non_melee_spell_cast_like_cpp(
        &self,
        with_delayed: bool,
        skip_channeled: bool,
        skip_autorepeat: bool,
        skip_instant: bool,
    ) -> bool {
        if let Some(spell) = self.current_spell(CurrentSpellSlot::Generic) {
            if spell.state != SpellState::Finished
                && (with_delayed || spell.state != SpellState::Delayed)
                && (!skip_instant || spell.cast_time_ms > 0)
            {
                return true;
            }
        }

        if !skip_channeled {
            if let Some(spell) = self.current_spell(CurrentSpellSlot::Channeled) {
                if spell.state != SpellState::Finished {
                    return true;
                }
            }
        }

        !skip_autorepeat && self.current_spell(CurrentSpellSlot::Autorepeat).is_some()
    }
    pub fn find_current_spell_by_spell_id(&self, spell_id: u32) -> Option<CurrentSpellRef> {
        self.subsystems
            .spells
            .find_current_spell_by_spell_id(spell_id)
    }
    pub(super) fn sync_casting_unit_state(&mut self) {
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
    /// C++ `Unit::AddExtraAttacks`.
    ///
    /// The target bucket is `_lastDamagedTargetGuid` first, then current
    /// selection (`UNIT_FIELD_TARGET`), otherwise the call is a no-op.
    pub fn add_extra_attacks_like_cpp(&mut self, count: u32) -> Option<ObjectGuid> {
        let target = self.last_damaged_target_like_cpp().unwrap_or_else(|| {
            let selected = self.data().target;
            if selected.is_empty() {
                ObjectGuid::EMPTY
            } else {
                selected
            }
        });
        if target.is_empty() {
            return None;
        }
        self.subsystems
            .combat
            .add_extra_attacks_for_like_cpp(target, count);
        Some(target)
    }
    pub fn extra_attacks_for_like_cpp(&self, target: ObjectGuid) -> u32 {
        self.subsystems.combat.extra_attacks_for_like_cpp(target)
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
}
