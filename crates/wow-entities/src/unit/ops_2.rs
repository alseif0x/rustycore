//! Unit values, visibility and health-revision state operations, part 2 of 3.
//!
//! The inherent `Unit` impl is divided by responsibility under
//! #636; every method keeps its original body.

use super::*;

impl Unit {
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
    pub fn total_aura_modifier_like_cpp(&self, aura_type: i32) -> i32 {
        self.subsystems
            .auras
            .total_aura_modifier_like_cpp(aura_type)
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
    pub const fn can_parry_like_cpp(&self) -> bool {
        self.can_parry
    }
    pub fn set_can_parry_like_cpp(&mut self, can_parry: bool) {
        self.can_parry = can_parry;
    }
    pub const fn can_block_like_cpp(&self) -> bool {
        self.can_block
    }
    pub fn set_can_block_like_cpp(&mut self, can_block: bool) {
        self.can_block = can_block;
    }
    pub const fn emote_state_like_cpp(&self) -> u32 {
        self.emote_state
    }
    pub fn set_emote_state_like_cpp(&mut self, emote_state: u32) {
        let packet_value = emote_state.min(i32::MAX as u32) as i32;
        if self.emote_state != emote_state || self.data.emote_state != packet_value {
            self.emote_state = emote_state;
            self.data.emote_state = packet_value;
            self.mark_unit_data_nested(UNIT_DATA_MODS_PARENT_BIT, UNIT_DATA_EMOTE_STATE_BIT);
        }
    }
    pub(super) fn apply_creature_attack_ai_side_effects_like_cpp(
        &mut self,
        victim_guid: ObjectGuid,
    ) {
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
    pub(super) fn apply_player_controlled_owner_attacked_like_cpp(
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
    pub(super) fn has_offhand_weapon_for_attack_like_cpp(&self) -> bool {
        self.world().object().type_id() != TypeId::Player && self.can_dual_wield
    }
    pub(super) fn delay_offhand_attack_like_cpp(&mut self) {
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
    pub fn speed_rate_at_like_cpp(&self, move_type_index: usize) -> Option<f32> {
        self.speed_rate.get(move_type_index).copied()
    }
    pub fn set_speed_rate_at_like_cpp(&mut self, move_type_index: usize, rate: f32) -> bool {
        let Some(speed_rate) = self.speed_rate.get_mut(move_type_index) else {
            return false;
        };
        *speed_rate = rate.max(0.0);
        true
    }
    pub fn set_speed_rate_like_cpp(&mut self, move_type: wow_constants::UnitMoveType, rate: f32) {
        let slot = move_type as usize;
        if slot < MAX_MOVE_TYPE {
            self.speed_rate[slot] = rate.max(0.0);
        }
    }
    pub const fn movement_counter_like_cpp(&self) -> u32 {
        self.movement_counter_like_cpp
    }
    pub fn next_movement_counter_like_cpp(&mut self) -> u32 {
        let current = self.movement_counter_like_cpp;
        self.movement_counter_like_cpp = self.movement_counter_like_cpp.wrapping_add(1);
        current
    }
    pub fn reset_movement_counter_like_cpp(&mut self) {
        self.movement_counter_like_cpp = 0;
    }
    pub const fn movement_force_mod_magnitude_like_cpp(&self) -> f32 {
        self.movement_force_mod_magnitude_like_cpp
    }
    pub fn set_movement_force_mod_magnitude_like_cpp(&mut self, magnitude: f32) {
        self.movement_force_mod_magnitude_like_cpp = magnitude;
    }
    pub fn set_mod_casting_speed_like_cpp(&mut self, casting_speed: f32) {
        self.set_f32_field(UNIT_DATA_MOD_CASTING_SPEED_BIT, casting_speed, |data| {
            &mut data.mod_casting_speed
        });
    }
    pub fn set_mod_spell_haste_like_cpp(&mut self, spell_haste: f32) {
        self.set_f32_field(UNIT_DATA_MOD_SPELL_HASTE_BIT, spell_haste, |data| {
            &mut data.mod_spell_haste
        });
    }
    pub fn set_mod_haste_like_cpp(&mut self, haste: f32) {
        self.set_f32_field(UNIT_DATA_MOD_HASTE_BIT, haste, |data| &mut data.mod_haste);
    }
    pub fn set_mod_ranged_haste_like_cpp(&mut self, ranged_haste: f32) {
        self.set_f32_field(UNIT_DATA_MOD_RANGED_HASTE_BIT, ranged_haste, |data| {
            &mut data.mod_ranged_haste
        });
    }
    pub fn set_mod_haste_regen_like_cpp(&mut self, haste_regen: f32) {
        self.set_f32_field(UNIT_DATA_MOD_HASTE_REGEN_BIT, haste_regen, |data| {
            &mut data.mod_haste_regen
        });
    }
    pub fn set_mod_time_rate_like_cpp(&mut self, time_rate: f32) {
        self.set_f32_field(UNIT_DATA_MOD_TIME_RATE_BIT, time_rate, |data| {
            &mut data.mod_time_rate
        });
    }
    pub fn set_hover_height_like_cpp(&mut self, hover_height: f32) {
        self.set_f32_field(UNIT_DATA_HOVER_HEIGHT_BIT, hover_height.max(0.0), |data| {
            &mut data.hover_height
        });
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
    pub fn set_display_scales_like_cpp(&mut self, display_scale: f32, native_display_scale: f32) {
        self.set_f32_field(
            UNIT_DATA_DISPLAY_SCALE_BIT,
            display_scale.max(0.0),
            |data| &mut data.display_scale,
        );
        self.set_f32_field(
            UNIT_DATA_NATIVE_DISPLAY_SCALE_BIT,
            native_display_scale.max(0.0),
            |data| &mut data.native_display_scale,
        );
    }
    pub fn set_native_display_id_like_cpp(&mut self, native_display_id: u32) {
        self.set_i32_field(
            UNIT_DATA_NATIVE_DISPLAY_ID_BIT,
            native_display_id as i32,
            |data| &mut data.native_display_id,
        );
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
    pub fn critter_guid_like_cpp(&self) -> Option<ObjectGuid> {
        (!self.data.critter.is_empty()).then_some(self.data.critter)
    }
    pub fn set_critter_guid_like_cpp(&mut self, critter: Option<ObjectGuid>) {
        self.set_guid_field(
            UNIT_DATA_CRITTER_BIT,
            critter.unwrap_or(ObjectGuid::EMPTY),
            |data| &mut data.critter,
        );
    }
    pub fn battle_pet_companion_guid_like_cpp(&self) -> Option<ObjectGuid> {
        (!self.data.battle_pet_companion_guid.is_empty())
            .then_some(self.data.battle_pet_companion_guid)
    }
    pub fn set_battle_pet_companion_guid_like_cpp(&mut self, guid: Option<ObjectGuid>) {
        self.set_guid_field(
            UNIT_DATA_BATTLE_PET_COMPANION_GUID_BIT,
            guid.unwrap_or(ObjectGuid::EMPTY),
            |data| &mut data.battle_pet_companion_guid,
        );
    }
    pub const fn battle_pet_companion_name_timestamp_like_cpp(&self) -> u32 {
        self.data.battle_pet_companion_name_timestamp
    }
    pub fn set_battle_pet_companion_name_timestamp_like_cpp(&mut self, timestamp: u32) {
        self.set_u32_field(
            UNIT_DATA_BATTLE_PET_COMPANION_NAME_TIMESTAMP_BIT,
            timestamp,
            |data| &mut data.battle_pet_companion_name_timestamp,
        );
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
    pub fn set_unit_flags3_like_cpp(&mut self, flags: UnitFlags3) {
        if self.data.flags3 != flags.bits() {
            self.data.flags3 = flags.bits();
            self.mark_unit_data(UNIT_DATA_FLAGS3_BIT);
        }
    }
    pub fn unit_flags3_like_cpp(&self) -> UnitFlags3 {
        UnitFlags3::from_bits_truncate(self.data.flags3)
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
    pub(super) fn set_npc_flags_index_like_cpp(&mut self, index: usize, flags: u32) {
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
        let target = &mut self.data.stand_state;
        if *target != state as u8 {
            *target = state as u8;
            // C++ generated `UpdateField<uint8, 32, 56>` marks both the
            // block-parent and field bits. The ordinary scalar helper only
            // covers block zero, so preserve the generated mask explicitly.
            self.mark_unit_data_nested(UNIT_DATA_STAND_STATE_PARENT_BIT, UNIT_DATA_STAND_STATE_BIT);
            // C++ UpdateField assignment also queues the owning Object for
            // Map::SendObjectUpdates when it is in world.
            self.world_mut()
                .object_mut()
                .add_to_object_update_if_needed();
        }
    }
    pub fn replace_all_vis_flags_like_cpp(&mut self, flags: u8) {
        self.set_u8_field(UNIT_DATA_VIS_FLAGS_BIT, flags, |data| &mut data.vis_flags);
    }
    pub const fn vis_flags_like_cpp(&self) -> u8 {
        self.data.vis_flags
    }
    pub fn set_anim_tier_like_cpp(&mut self, anim_tier: u8) {
        self.set_u8_field(UNIT_DATA_ANIM_TIER_BIT, anim_tier, |data| {
            &mut data.anim_tier
        });
    }
    pub const fn anim_tier_like_cpp(&self) -> u8 {
        self.data.anim_tier
    }
    pub fn set_sheath_like_cpp(&mut self, state: SheathState) {
        self.set_u8_field(UNIT_DATA_SHEATHE_STATE_BIT, state as u8, |data| {
            &mut data.sheathe_state
        });
    }
    pub fn sheath_like_cpp(&self) -> SheathState {
        match self.data.sheathe_state {
            1 => SheathState::Melee,
            2 => SheathState::Ranged,
            _ => SheathState::Unarmed,
        }
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
    pub fn set_ai_anim_kit_id_like_cpp(&mut self, anim_kit_id: u16) -> bool {
        if self.ai_anim_kit_id == anim_kit_id {
            return false;
        }
        self.ai_anim_kit_id = anim_kit_id;
        true
    }
    pub const fn ai_anim_kit_id_like_cpp(&self) -> u16 {
        self.ai_anim_kit_id
    }
    pub fn set_movement_anim_kit_id_like_cpp(&mut self, anim_kit_id: u16) -> bool {
        if self.movement_anim_kit_id == anim_kit_id {
            return false;
        }
        self.movement_anim_kit_id = anim_kit_id;
        true
    }
    pub const fn movement_anim_kit_id_like_cpp(&self) -> u16 {
        self.movement_anim_kit_id
    }
    pub fn set_melee_anim_kit_id_like_cpp(&mut self, anim_kit_id: u16) -> bool {
        if self.melee_anim_kit_id == anim_kit_id {
            return false;
        }
        self.melee_anim_kit_id = anim_kit_id;
        true
    }
    pub const fn melee_anim_kit_id_like_cpp(&self) -> u16 {
        self.melee_anim_kit_id
    }
    pub fn replace_all_pet_flags_like_cpp(&mut self, flags: u8) {
        self.set_u8_field(UNIT_DATA_PET_FLAGS_BIT, flags, |data| &mut data.pet_flags);
    }
    pub const fn pet_flags_like_cpp(&self) -> u8 {
        self.data.pet_flags
    }
    pub fn set_shapeshift_form_like_cpp(&mut self, form: ShapeShiftForm) {
        self.set_u8_field(UNIT_DATA_SHAPESHIFT_FORM_BIT, form as u8, |data| {
            &mut data.shapeshift_form
        });
    }
    pub fn shapeshift_form_like_cpp(&self) -> ShapeShiftForm {
        match self.data.shapeshift_form {
            1 => ShapeShiftForm::CatForm,
            2 => ShapeShiftForm::TreeForm,
            3 => ShapeShiftForm::TravelForm,
            4 => ShapeShiftForm::AquaticForm,
            5 => ShapeShiftForm::BearForm,
            8 => ShapeShiftForm::DireBearForm,
            16 => ShapeShiftForm::GhostWolf,
            28 => ShapeShiftForm::Shadowform,
            _ => ShapeShiftForm::None,
        }
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
        let health_before = self.data.health;
        self.set_u64_field(UNIT_DATA_HEALTH_BIT, value, |data| &mut data.health);
        if self.data.health != health_before {
            self.advance_health_state_revision_like_cpp();
        }
    }
    pub fn set_max_health(&mut self, mut value: u64) {
        if value == 0 {
            value = 1;
        }
        let current = self.data.health;
        let max_health_before = self.data.max_health;
        self.set_u64_field(UNIT_DATA_MAX_HEALTH_BIT, value, |data| &mut data.max_health);
        if self.data.max_health != max_health_before {
            self.advance_health_state_revision_like_cpp();
        }
        if value < current {
            self.set_health(value);
        }
    }
    pub fn set_wild_battle_pet_level_like_cpp(&mut self, level: u32) {
        self.set_i32_field(UNIT_DATA_WILD_BATTLE_PET_LEVEL_BIT, level as i32, |data| {
            &mut data.wild_battle_pet_level
        });
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
    pub fn get_create_mana_like_cpp(&self) -> i32 {
        self.data.base_mana
    }
    pub fn set_create_mana_like_cpp(&mut self, value: i32) {
        self.set_i32_field(UNIT_DATA_BASE_MANA_BIT, value.max(0), |data| {
            &mut data.base_mana
        });
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
    pub fn replace_create_power_arrays_like_cpp(
        &mut self,
        power: [i32; MAX_POWERS_PER_CLASS],
        max_power: [i32; MAX_POWERS_PER_CLASS],
    ) {
        for index in 0..MAX_POWERS_PER_CLASS {
            if self.data.power[index] != power[index] {
                self.data.power[index] = power[index];
                self.mark_unit_data_array(
                    UNIT_DATA_POWER_PARENT_BIT,
                    UNIT_DATA_POWER_FIRST_BIT,
                    index,
                );
            }
            if self.data.max_power[index] != max_power[index] {
                self.data.max_power[index] = max_power[index];
                self.mark_unit_data_array(
                    UNIT_DATA_POWER_PARENT_BIT,
                    UNIT_DATA_MAX_POWER_FIRST_BIT,
                    index,
                );
            }
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
    pub(super) fn set_u64_field(
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
    pub(super) fn set_i32_field(
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
    pub(super) fn set_u32_field(
        &mut self,
        bit: usize,
        value: u32,
        field: impl FnOnce(&mut UnitDataValues) -> &mut u32,
    ) {
        let target = field(&mut self.data);
        if *target != value {
            *target = value;
            self.mark_unit_data(bit);
        }
    }
    pub(super) fn set_u8_field(
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
    pub(super) fn set_guid_field(
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
}
