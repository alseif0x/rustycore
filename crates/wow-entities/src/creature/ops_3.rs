//! Creature lifecycle and runtime state operations, part 3 of 3.
//!
//! The inherent `Creature` impl is divided by responsibility under
//! #636; every method keeps its original body.

use super::*;

impl Creature {
    pub const fn attack_reputation_faction_id_like_cpp(&self) -> Option<u32> {
        self.attack_reputation_faction_id
    }
    pub fn set_attack_reputation_faction_id_like_cpp(&mut self, faction_id: Option<u32>) {
        self.attack_reputation_faction_id = faction_id;
    }
    pub const fn is_contested_guard_like_cpp(&self) -> bool {
        self.is_contested_guard_faction
    }
    pub fn set_contested_guard_like_cpp(&mut self, contested_guard: bool) {
        self.is_contested_guard_faction = contested_guard;
    }
    pub const fn runtime_state(&self) -> &CreatureRuntimeState {
        &self.runtime_state
    }
    pub const fn loot_lifecycle_revision_like_cpp(&self) -> u64 {
        self.loot_lifecycle_revision
    }
    pub(super) fn advance_loot_lifecycle_revision_like_cpp(&mut self) -> u64 {
        // Never wrap to an earlier lifetime: even an unreachable counter
        // exhaustion must remain fail-closed for async generation tokens.
        self.loot_lifecycle_revision = self.loot_lifecycle_revision.saturating_add(1).max(1);
        self.loot_lifecycle_revision
    }
    pub fn runtime_state_mut(&mut self) -> &mut CreatureRuntimeState {
        &mut self.runtime_state
    }
    pub fn tap_list(&self) -> &[ObjectGuid] {
        &self.tap_list
    }
    pub fn has_loot_recipient(&self) -> bool {
        self.runtime_state.has_loot_recipient
    }
    pub const fn loot_authority_like_cpp(&self) -> &OwnedLootAuthority {
        &self.loot_authority
    }
    /// Bind this runtime mirror to the same C++ `Creature::loot` authority as
    /// another coexisting map model.
    pub fn rebind_loot_authority_like_cpp(&mut self, authority: OwnedLootAuthority) -> bool {
        if self.loot_authority.shares_storage_like_cpp(&authority) {
            self.sync_loot_summaries_from_authority_like_cpp();
            return false;
        }

        // Invalidate leases retained against a displaced mirror before the
        // replacement authority can serve another claim for the same C++
        // object.
        self.loot_authority.detach_like_cpp();
        self.loot_authority = authority;
        self.sync_loot_summaries_from_authority_like_cpp();
        true
    }
    /// Rebinds only if this mirror still owns the authority observed by the
    /// caller. This is the object-local compare/exchange boundary used when
    /// the legacy and canonical map models are reconciled without holding
    /// both map locks at once.
    pub fn rebind_loot_authority_if_current_like_cpp(
        &mut self,
        expected: &OwnedLootAuthority,
        expected_stamp: OwnedLootAuthorityStamp,
        authority: OwnedLootAuthority,
    ) -> Option<bool> {
        if !self.loot_authority.shares_storage_like_cpp(expected) {
            return None;
        }

        if self.loot_authority.stamp_like_cpp() != expected_stamp {
            return None;
        }

        if self.loot_authority.shares_storage_like_cpp(&authority) {
            self.sync_loot_summaries_from_authority_like_cpp();
            return Some(false);
        }

        if !self.loot_authority.detach_if_stamp_like_cpp(expected_stamp) {
            return None;
        }

        self.loot_authority = authority;
        self.sync_loot_summaries_from_authority_like_cpp();
        Some(true)
    }
    /// Assigns authority identity to a temporary whole-entity snapshot.
    ///
    /// The snapshot may share its old `Arc` with a live legacy entity, so it
    /// must not detach that allocation. Only the later CAS against the actual
    /// owning entity is allowed to invalidate a displaced authority.
    pub fn adopt_loot_authority_for_snapshot_like_cpp(&mut self, authority: OwnedLootAuthority) {
        self.loot_authority = authority;
        self.sync_loot_summaries_from_authority_like_cpp();
    }
    pub fn share_loot_authority_like_cpp(&mut self, authority: OwnedLootAuthority) {
        self.rebind_loot_authority_like_cpp(authority);
    }
    pub fn initialize_loot_authority_like_cpp(
        &mut self,
        shared: Option<CreatureLoot>,
        personal: HashMap<ObjectGuid, CreatureLoot>,
    ) -> LootInstallOutcome {
        let outcome = self.loot_authority.initialize_like_cpp(shared, personal);
        self.sync_loot_summaries_from_authority_like_cpp();
        outcome
    }
    pub fn initialize_shared_loot_authority_like_cpp(
        &mut self,
        loot: CreatureLoot,
    ) -> LootInstallOutcome {
        let outcome = self.loot_authority.initialize_shared_like_cpp(loot);
        self.sync_loot_summaries_from_authority_like_cpp();
        outcome
    }
    pub fn upsert_personal_loot_authority_like_cpp(
        &mut self,
        player: ObjectGuid,
        loot: CreatureLoot,
        replace: bool,
    ) -> LootInstallOutcome {
        let outcome = self
            .loot_authority
            .upsert_personal_like_cpp(player, loot, replace);
        self.sync_loot_summaries_from_authority_like_cpp();
        outcome
    }
    pub fn replace_loot_authority_like_cpp(
        &mut self,
        shared: Option<CreatureLoot>,
        personal: HashMap<ObjectGuid, CreatureLoot>,
    ) -> u64 {
        let generation = self.loot_authority.replace_like_cpp(shared, personal);
        self.sync_loot_summaries_from_authority_like_cpp();
        generation
    }
    pub fn sync_loot_summaries_from_authority_like_cpp(&mut self) {
        self.shared_loot = self
            .loot_authority
            .shared_snapshot_like_cpp()
            .map(|snapshot| creature_owned_loot_from_snapshot(&snapshot));
        self.personal_loot = self
            .loot_authority
            .personal_snapshots_like_cpp()
            .into_iter()
            .map(|(player, snapshot)| (player, creature_owned_loot_from_snapshot(&snapshot)))
            .collect();
    }
    pub const fn shared_loot_like_cpp(&self) -> Option<&CreatureOwnedLoot> {
        self.shared_loot.as_ref()
    }
    pub fn set_shared_loot_like_cpp(&mut self, loot: CreatureOwnedLoot) {
        self.shared_loot = Some(loot);
    }
    pub fn clear_shared_loot_like_cpp(&mut self) {
        self.shared_loot = None;
    }
    pub fn personal_loot_like_cpp(&self, guid: ObjectGuid) -> Option<&CreatureOwnedLoot> {
        self.personal_loot.get(&guid)
    }
    pub fn loot_for_player_like_cpp(&self, guid: ObjectGuid) -> Option<&CreatureOwnedLoot> {
        if self.personal_loot.is_empty() {
            return self.shared_loot.as_ref();
        }

        self.personal_loot.get(&guid)
    }
    pub fn set_personal_loot_like_cpp(&mut self, guid: ObjectGuid, loot: CreatureOwnedLoot) {
        self.personal_loot.insert(guid, loot);
    }
    pub fn clear_personal_loot_like_cpp(&mut self) {
        self.personal_loot.clear();
    }
    pub fn personal_loot_count_like_cpp(&self) -> usize {
        self.personal_loot.len()
    }
    pub fn clear_loot_like_cpp(&mut self) {
        self.advance_loot_lifecycle_revision_like_cpp();
        self.loot_authority.retire_like_cpp();
        self.shared_loot = None;
        self.personal_loot.clear();
    }
    pub fn is_fully_looted_like_cpp(&self) -> bool {
        if !self.loot_authority.is_pristine_like_cpp() {
            return self.loot_authority.is_fully_looted_like_cpp();
        }

        if self
            .shared_loot
            .as_ref()
            .is_some_and(|loot| !loot.is_looted_like_cpp())
        {
            return false;
        }

        for loot in self.personal_loot.values() {
            if !loot.is_looted_like_cpp() {
                return false;
            }
        }

        true
    }
    pub fn is_reputation_gain_disabled(&self) -> bool {
        self.disable_reputation_gain
    }
    pub fn set_disable_reputation_gain(&mut self, disabled: bool) {
        self.disable_reputation_gain = disabled;
    }
    pub fn set_pickpocket_loot_restore(&mut self, restore_time: i64) {
        self.pickpocket_loot_restore = restore_time;
    }
    pub const fn pickpocket_loot_restore(&self) -> i64 {
        self.pickpocket_loot_restore
    }
    pub fn reset_pickpocket_loot_restore(&mut self) {
        self.pickpocket_loot_restore = 0;
        self.runtime_state.pickpocket_reset_count =
            self.runtime_state.pickpocket_reset_count.saturating_add(1);
    }
    pub fn set_dont_clear_tap_list_on_evade(&mut self, dont_clear: bool) {
        if self.spawn_id == 0 {
            self.dont_clear_tap_list_on_evade = dont_clear;
        }
    }
    pub const fn dont_clear_tap_list_on_evade(&self) -> bool {
        self.dont_clear_tap_list_on_evade
    }
    pub fn set_tapped_by_player(&mut self, player_guid: ObjectGuid, group_guids: &[ObjectGuid]) {
        if self.tap_list.len() >= CREATURE_TAPPERS_SOFT_CAP || player_guid == ObjectGuid::EMPTY {
            return;
        }
        self.insert_tapper(player_guid);
        for guid in group_guids {
            if self.tap_list.len() >= CREATURE_TAPPERS_SOFT_CAP {
                break;
            }
            if *guid != ObjectGuid::EMPTY {
                self.insert_tapper(*guid);
            }
        }
        self.runtime_state.has_loot_recipient = !self.tap_list.is_empty();
    }
    pub fn is_tapped_by(&self, player_guid: ObjectGuid) -> bool {
        self.tap_list.contains(&player_guid)
    }
    pub fn clear_tap_list(&mut self) {
        self.tap_list.clear();
        self.runtime_state.has_loot_recipient = false;
    }
    pub fn clear_tap_list_for_evade(&mut self) {
        if !self.dont_clear_tap_list_on_evade {
            self.clear_tap_list();
        }
    }
    pub(super) fn insert_tapper(&mut self, guid: ObjectGuid) {
        if !self.tap_list.contains(&guid) && self.tap_list.len() < CREATURE_TAPPERS_SOFT_CAP {
            self.tap_list.push(guid);
        }
    }
    pub fn apply_death_transition(&mut self, state: DeathState, now: i64) -> CreatureRuntimePlan {
        self.set_death_state_runtime(state, now)
    }
    pub fn set_death_state_runtime(&mut self, state: DeathState, now: i64) -> CreatureRuntimePlan {
        self.set_death_state_runtime_with_fall_like_cpp(state, now, None)
    }
    pub fn set_death_state_runtime_with_fall_like_cpp(
        &mut self,
        state: DeathState,
        now: i64,
        death_fall: Option<CreatureDeathFallContextLikeCpp>,
    ) -> CreatureRuntimePlan {
        let mut plan = CreatureRuntimePlan::new();
        self.unit.set_death_state(state);

        match state {
            DeathState::JustDied => {
                if self.ai_ownership.state != CreatureAiState::Dead {
                    self.advance_loot_lifecycle_revision_like_cpp();
                }
                let needs_falling = death_fall.filter(|context| {
                    (self.is_flying_like_cpp() || self.is_hovering_like_cpp())
                        && !context.is_underwater
                });
                self.corpse_remove_time = now.saturating_add(self.corpse_delay as i64);
                let respawn_delay = self.respawn_delay as i64;
                self.respawn_time = if self.respawn_compatibility_mode {
                    now.saturating_add(respawn_delay)
                        .saturating_add(self.corpse_delay as i64)
                } else {
                    now.saturating_add(respawn_delay)
                };
                self.runtime_state.save_respawn_requested = true;
                self.runtime_state.visibility_update_requested = true;
                self.release_spell_focus_like_cpp(None, false, false, false);
                self.do_not_reacquire_spell_focus_target_like_cpp();
                self.unit.set_target(ObjectGuid::EMPTY);
                self.unit.set_attacking(None);
                self.unit.subsystems_mut().combat.end_all_combat();
                self.unit.subsystems_mut().combat.clear_attackers();
                if self
                    .unit
                    .is_non_melee_spell_cast_like_cpp(false, false, false, true)
                {
                    self.unit.interrupt_non_melee_spells(None, false, true);
                }
                {
                    let subsystems = self.unit.subsystems_mut();
                    subsystems.vehicle.exit_vehicle();
                    for summon_slot in &mut subsystems.control.summon_slots {
                        *summon_slot = ObjectGuid::EMPTY;
                    }
                    subsystems.control.remove_all_controlled();
                }
                if !self.unit.world().object().guid().is_pet() {
                    let mut flags = self.unit.unit_flags_like_cpp();
                    flags.remove(UnitFlags::PET_IN_COMBAT);
                    self.unit.set_unit_flags_like_cpp(flags);
                }
                self.unit
                    .subsystems_mut()
                    .auras
                    .remove_all_auras_on_death_like_cpp();
                self.unit
                    .subsystems_mut()
                    .auras
                    .clear_all_reactives_like_cpp();
                self.unit.subsystems_mut().auras.clear_diminishings();
                let stop_on_death = if self.unit.subsystems().vehicle.vehicle_guid.is_some() {
                    false
                } else {
                    self.unit.subsystems_mut().motion.stop_on_death()
                };
                if stop_on_death {
                    self.unit.clear_unit_state(UnitState::MOVING.bits());
                }
                self.unit.set_health(0);
                self.unit.set_power(self.power_type(), 0);
                self.unit.set_emote_state_like_cpp(0);
                self.unit
                    .set_stand_state_like_cpp(UnitStandStateType::Stand);
                self.unit.set_npc_flags_like_cpp(0);
                self.unit.set_npc_flags2_like_cpp(0);
                self.unit.set_mount_display_id(0);
                self.unit.world_mut().set_active(false);
                self.already_searched_assistance = false;
                self.already_call_assistance = false;
                self.runtime_state
                    .movement_flags
                    .remove(MovementFlag::HOVER | MovementFlag::DISABLE_GRAVITY);
                plan.extend([
                    CreatureRuntimeAction::SaveRespawnTime,
                    CreatureRuntimeAction::ReleaseSpellFocus,
                    CreatureRuntimeAction::CancelSpellFocusReacquire,
                    CreatureRuntimeAction::ClearTarget,
                    CreatureRuntimeAction::ClearNpcFlags,
                    CreatureRuntimeAction::ClearMount,
                    CreatureRuntimeAction::Deactivate,
                    CreatureRuntimeAction::ClearAssistanceSearch,
                ]);
                if let Some(context) = needs_falling {
                    let has_root_or_stun_state = self
                        .unit
                        .has_unit_state((UnitState::ROOT | UnitState::STUNNED).bits());
                    let fall_plan = self.unit.subsystems_mut().motion.move_fall_like_cpp(
                        context.movement_id,
                        context.duration_ms,
                        context.has_valid_ground_height,
                        context.vertical_delta,
                        has_root_or_stun_state,
                        false,
                    );
                    if matches!(fall_plan, MoveFallPlan::SplineStarted) {
                        plan.push(CreatureRuntimeAction::MoveFall);
                    }
                }
                self.unit.set_death_state(DeathState::Corpse);
            }
            DeathState::JustRespawned => {
                self.advance_loot_lifecycle_revision_like_cpp();
                let motion_initialize_outcome = self.aim_initialize_like_cpp();
                let is_pet = self.unit.world().object().guid().is_pet();
                if is_pet {
                    self.unit.set_health(self.unit.data().max_health);
                } else {
                    self.set_spawn_health_like_cpp();
                    self.unit
                        .world_mut()
                        .object_mut()
                        .replace_all_dynamic_flags(0);
                    self.unit
                        .set_npc_flags_like_cpp(self.ai_ownership.npc_flags);
                    self.unit
                        .set_npc_flags2_like_cpp(self.ai_ownership.npc_flags2);
                    let mut flags = UnitFlags::from_bits_truncate(self.ai_ownership.unit_flags);
                    flags.remove(UnitFlags::SKINNABLE | UnitFlags::IN_COMBAT);
                    self.unit.set_unit_flags_like_cpp(flags);
                    self.unit
                        .set_unit_flags2_like_cpp(UnitFlags2::from_bits_truncate(
                            self.ai_ownership.unit_flags2,
                        ));
                    self.unit
                        .set_unit_flags3_like_cpp(UnitFlags3::from_bits_truncate(
                            self.ai_ownership.unit_flags3,
                        ));
                    self.set_melee_damage_school_like_cpp(self.lifecycle_metadata.damage_school);
                }
                self.unit.clear_unit_state(UnitState::ALL_ERASABLE.bits());
                self.clear_tap_list();
                self.player_damage_req = 0;
                self.cannot_reach_target = false;
                self.cannot_reach_timer = 0;
                self.respawn_time = 0;
                self.corpse_remove_time = 0;
                self.reset_pickpocket_loot_restore();
                self.reset_loot_mode();
                let addon = self.lifecycle_metadata.addon.clone();
                self.load_creatures_addon_represented_like_cpp(addon.as_ref());
                self.trigger_just_appeared = true;
                self.runtime_state.ai_reset_requested = true;
                self.runtime_state.visibility_update_requested = true;
                if motion_initialize_outcome.motion_master_initialize_represented {
                    self.unit
                        .subsystems_mut()
                        .motion
                        .direct_initialize_like_cpp();
                }
                plan.extend([
                    CreatureRuntimeAction::ClearTapList,
                    CreatureRuntimeAction::ResetPlayerDamageReq,
                    CreatureRuntimeAction::ResetCannotReachTarget,
                    CreatureRuntimeAction::UpdateMovementFlags,
                    CreatureRuntimeAction::ClearErasableUnitState,
                    CreatureRuntimeAction::InitializeMotion,
                    CreatureRuntimeAction::ResetAi,
                    CreatureRuntimeAction::LoadAddonAndSparring,
                ]);
                self.unit.set_death_state(DeathState::Alive);
            }
            _ => {}
        }

        plan
    }
    pub(super) fn load_creatures_addon_represented_like_cpp(
        &mut self,
        addon: Option<&CreatureAddonLifecycleRecordLikeCpp>,
    ) -> bool {
        let Some(addon) = addon else {
            return false;
        };

        if addon.mount_display_id != 0 {
            self.unit.set_mount_display_id(addon.mount_display_id);
        }
        self.unit.set_stand_state_like_cpp(addon.stand_state);
        self.unit.replace_all_vis_flags_like_cpp(addon.vis_flags);
        self.unit.set_anim_tier_like_cpp(addon.anim_tier);
        if self.can_hover_like_cpp() {
            self.runtime_state
                .movement_flags
                .insert(MovementFlag::HOVER);
        }
        self.unit.set_sheath_like_cpp(addon.sheath_state);
        self.unit.replace_all_pvp_flags_like_cpp(addon.pvp_flags);
        self.unit.replace_all_pet_flags_like_cpp(0);
        self.unit.set_shapeshift_form_like_cpp(ShapeShiftForm::None);
        if addon.emote != 0 {
            self.unit.set_emote_state_like_cpp(addon.emote);
        }
        if addon.path_id != 0 {
            self.waypoint_path_id = addon.path_id;
        }
        self.unit.set_ai_anim_kit_id_like_cpp(addon.ai_anim_kit_id);
        self.unit
            .set_movement_anim_kit_id_like_cpp(addon.movement_anim_kit_id);
        self.unit
            .set_melee_anim_kit_id_like_cpp(addon.melee_anim_kit_id);
        if addon.visibility_distance_type != VisibilityDistanceTypeLikeCpp::Normal {
            self.unit
                .world_mut()
                .set_visibility_distance_override_like_cpp(addon.visibility_distance_type);
            let mut flags2 = self.unit.unit_flags2_like_cpp();
            flags2.remove(
                UnitFlags2::LARGE_AOI | UnitFlags2::GIGANTIC_AOI | UnitFlags2::INFINITE_AOI,
            );
            match addon.visibility_distance_type {
                VisibilityDistanceTypeLikeCpp::Large => flags2.insert(UnitFlags2::LARGE_AOI),
                VisibilityDistanceTypeLikeCpp::Gigantic => flags2.insert(UnitFlags2::GIGANTIC_AOI),
                VisibilityDistanceTypeLikeCpp::Infinite => flags2.insert(UnitFlags2::INFINITE_AOI),
                _ => {}
            }
            self.unit.set_unit_flags2_like_cpp(flags2);
        }
        let self_guid = self.unit.world().object().guid();
        if addon.aura_applications.is_empty() {
            for spell_id in &addon.auras {
                self.unit
                    .subsystems_mut()
                    .auras
                    .add_self_cast_addon_aura_like_cpp(*spell_id, self_guid);
            }
        } else {
            for aura in &addon.aura_applications {
                self.unit
                    .subsystems_mut()
                    .auras
                    .add_self_cast_addon_aura_application_like_cpp(
                        aura.spell_id,
                        self_guid,
                        aura.effect_mask,
                        aura.flags,
                    );
            }
        }
        true
    }
    pub fn remove_corpse_runtime(
        &mut self,
        now: i64,
        set_spawn_time: bool,
        destroy_for_nearby_players: bool,
    ) -> CreatureRuntimePlan {
        let mut plan = CreatureRuntimePlan::new();
        if self.unit.death_state() != DeathState::Corpse {
            return plan;
        }

        // C++ drops the corpse's object-owned Loot during removal (directly
        // in compatibility mode and via object destruction otherwise). Rust
        // leases can outlive the map reference, so retire it at the lifecycle
        // boundary before an in-flight async claim can commit.
        self.clear_loot_like_cpp();
        self.runtime_state.remove_corpse_requested = true;
        self.runtime_state.corpse_removed_count =
            self.runtime_state.corpse_removed_count.saturating_add(1);
        self.runtime_state.loot_removed_count =
            self.runtime_state.loot_removed_count.saturating_add(1);
        self.corpse_remove_time = now;
        plan.extend([
            CreatureRuntimeAction::RemoveAllAuras,
            CreatureRuntimeAction::RemoveLoot,
            CreatureRuntimeAction::CorpseRemovedAiHook,
        ]);

        if destroy_for_nearby_players {
            self.runtime_state.visibility_destroy_requested = true;
            plan.push(CreatureRuntimeAction::DestroyVisibility);
        }

        if set_spawn_time {
            self.respawn_time = self
                .respawn_time
                .max(now.saturating_add(self.respawn_delay as i64));
            self.runtime_state.save_respawn_requested = !self.respawn_compatibility_mode;
            if !self.respawn_compatibility_mode {
                plan.push(CreatureRuntimeAction::SaveRespawnTime);
            }
        }

        if self.respawn_compatibility_mode {
            self.unit.set_death_state(DeathState::Dead);
            plan.push(CreatureRuntimeAction::RelocateToRespawnPosition);
        } else {
            self.runtime_state.object_remove_requested = true;
            plan.push(CreatureRuntimeAction::RequestObjectRemove);
        }

        plan
    }
    pub fn respawn_runtime(&mut self, force: bool, now: i64) -> CreatureRuntimePlan {
        let mut plan = CreatureRuntimePlan::new();
        if force {
            if self.unit.is_alive() {
                plan.extend(
                    self.set_death_state_runtime(DeathState::JustDied, now)
                        .actions()
                        .iter()
                        .copied(),
                );
            } else if self.unit.death_state() != DeathState::Corpse {
                self.unit.set_death_state(DeathState::Corpse);
            }
        }

        if self.respawn_compatibility_mode {
            self.runtime_state.visibility_destroy_requested = true;
            plan.push(CreatureRuntimeAction::DestroyVisibility);
            plan.extend(
                self.remove_corpse_runtime(now, false, false)
                    .actions()
                    .iter()
                    .copied(),
            );
            if self.unit.death_state() == DeathState::Dead {
                self.respawn_time = 0;
                self.reset_pickpocket_loot_restore();
                self.runtime_state.loot_removed_count =
                    self.runtime_state.loot_removed_count.saturating_add(1);
                plan.extend([
                    CreatureRuntimeAction::ResetPickpocketLoot,
                    CreatureRuntimeAction::RemoveLoot,
                    CreatureRuntimeAction::RestoreOriginalEntry,
                    CreatureRuntimeAction::SelectLevel,
                ]);
                plan.extend(
                    self.set_death_state_runtime(DeathState::JustRespawned, now)
                        .actions()
                        .iter()
                        .copied(),
                );
                plan.extend([
                    CreatureRuntimeAction::ResetDisplay,
                    CreatureRuntimeAction::ResetReactState,
                    CreatureRuntimeAction::UpdatePool,
                ]);
            }
            self.runtime_state.visibility_update_requested = true;
            plan.push(CreatureRuntimeAction::UpdateVisibility);
        } else if self.spawn_id != 0 {
            self.runtime_state.map_respawn_requested = true;
            self.runtime_state.respawn_requested = true;
            plan.push(CreatureRuntimeAction::RequestMapRespawn);
        }

        plan
    }
    pub fn forced_despawn_runtime(
        &mut self,
        time_ms_to_despawn: u32,
        force_respawn_timer_secs: u32,
        now: i64,
    ) -> CreatureRuntimePlan {
        let mut plan = CreatureRuntimePlan::new();
        if time_ms_to_despawn > 0 {
            self.runtime_state.forced_despawn_pending = true;
            plan.push(CreatureRuntimeAction::RequestDelayedForcedDespawn);
            return plan;
        }

        if self.respawn_compatibility_mode {
            let corpse_delay = self.corpse_delay;
            let respawn_delay = self.respawn_delay;
            let mut override_respawn_time = false;
            self.runtime_state.visibility_destroy_requested = true;
            plan.push(CreatureRuntimeAction::DestroyVisibility);

            if self.unit.is_alive() {
                if force_respawn_timer_secs > 0 {
                    self.corpse_delay = 0;
                    self.respawn_delay = force_respawn_timer_secs;
                    override_respawn_time = true;
                }
                plan.extend(
                    self.set_death_state_runtime(DeathState::JustDied, now)
                        .actions()
                        .iter()
                        .copied(),
                );
            }

            plan.extend(
                self.remove_corpse_runtime(now, !override_respawn_time, false)
                    .actions()
                    .iter()
                    .copied(),
            );
            self.corpse_delay = corpse_delay;
            self.respawn_delay = respawn_delay;
        } else {
            if force_respawn_timer_secs > 0 {
                self.respawn_time = now.saturating_add(force_respawn_timer_secs as i64);
            } else {
                self.respawn_time = now.saturating_add(self.respawn_delay as i64);
            }
            self.runtime_state.save_respawn_requested = true;
            self.runtime_state.object_remove_requested = true;
            plan.extend([
                CreatureRuntimeAction::SaveRespawnTime,
                CreatureRuntimeAction::RequestObjectRemove,
            ]);
        }

        plan
    }
    pub fn all_loot_removed_from_corpse(
        &mut self,
        now: i64,
        decay_rate: f32,
        is_fully_skinned: bool,
    ) -> CreatureRuntimePlan {
        let mut plan = CreatureRuntimePlan::new();
        if self.corpse_remove_time <= now {
            return plan;
        }

        let effective_decay_rate = if self.ignore_corpse_decay_ratio {
            1.0
        } else {
            decay_rate.max(0.0)
        };
        self.corpse_remove_time = if is_fully_skinned {
            now
        } else {
            now.saturating_add((self.corpse_delay as f32 * effective_decay_rate) as i64)
        };
        self.respawn_time = self.respawn_time.max(
            self.corpse_remove_time
                .saturating_add(self.respawn_delay as i64),
        );
        self.runtime_state.remove_corpse_requested = is_fully_skinned;
        plan.push(CreatureRuntimeAction::UpdateLoot);
        plan
    }
    pub fn runtime_update_plan(
        &mut self,
        diff_ms: u32,
        now: i64,
        context: CreatureRuntimeUpdateContext,
    ) -> CreatureRuntimePlan {
        let mut plan = CreatureRuntimePlan::new();

        if context.ai_enabled
            && self.trigger_just_appeared
            && self.unit.death_state() != DeathState::Dead
        {
            self.trigger_just_appeared = false;
            self.runtime_state.appeared_notified = true;
            plan.push(CreatureRuntimeAction::NotifyJustAppeared);
        }

        match self.unit.death_state() {
            DeathState::Dead => {
                if self.respawn_compatibility_mode && self.respawn_time <= now {
                    self.runtime_state.respawn_requested = true;
                    plan.extend(self.respawn_runtime(false, now).actions().iter().copied());
                }
            }
            DeathState::Corpse => {
                if context.has_loot || context.has_personal_loot {
                    self.runtime_state.loot_updated_count =
                        self.runtime_state.loot_updated_count.saturating_add(1);
                    plan.push(CreatureRuntimeAction::UpdateLoot);
                }
                if self.corpse_remove_time <= now {
                    plan.extend(
                        self.remove_corpse_runtime(now, false, true)
                            .actions()
                            .iter()
                            .copied(),
                    );
                }
            }
            DeathState::Alive => {
                if context.ai_enabled && !context.in_evade_mode && context.is_engaged {
                    if consume_timer(&mut self.boundary_check_time, diff_ms) {
                        plan.push(CreatureRuntimeAction::BoundaryCheck);
                        self.boundary_check_time = DEFAULT_BOUNDARY_CHECK_TIME_MS;
                    }
                }

                if self.combat_pulse_delay > 0 && context.is_engaged && context.is_dungeon {
                    if consume_timer(&mut self.combat_pulse_time, diff_ms) {
                        if context.has_map_players {
                            plan.push(CreatureRuntimeAction::CombatPulse);
                        }
                        self.combat_pulse_time = self.combat_pulse_delay.saturating_mul(1_000);
                    }
                }

                plan.push(CreatureRuntimeAction::AiUpdateTick);
                plan.push(CreatureRuntimeAction::MeleeAttackIfReady);

                if consume_timer(&mut self.regen_timer, diff_ms) {
                    let can_regen_health = !context.in_evade_mode
                        && (!context.is_engaged
                            || context.is_polymorphed
                            || (context.cannot_reach_target
                                && (context.allow_cannot_reach_regen || !context.is_raid)));
                    if self.regenerate_health && can_regen_health {
                        plan.push(CreatureRuntimeAction::RegenerateHealth);
                    }
                    plan.push(CreatureRuntimeAction::RegeneratePower);
                    self.regen_timer = CREATURE_REGEN_INTERVAL_MS;
                }

                if context.cannot_reach_target && !context.in_evade_mode && !context.is_raid {
                    self.cannot_reach_target = true;
                    self.cannot_reach_timer = self.cannot_reach_timer.saturating_add(diff_ms);
                    if self.cannot_reach_timer >= CREATURE_NOPATH_EVADE_TIME_MS {
                        self.runtime_state.evade_requested =
                            Some(CreatureRuntimeEvadeReason::NoPath);
                        plan.push(CreatureRuntimeAction::Evade(
                            CreatureRuntimeEvadeReason::NoPath,
                        ));
                    }
                } else {
                    self.cannot_reach_timer = 0;
                }
            }
            _ => {}
        }

        plan
    }
}
