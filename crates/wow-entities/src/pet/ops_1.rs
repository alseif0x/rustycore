//! Pet lifecycle, aura and persistence state operations, part 1 of 2.
//!
//! The inherent `Pet` impl is divided by responsibility under
//! #636; every method keeps its original body.

use super::*;

impl Pet {
    pub fn new(owner_guid: ObjectGuid, pet_type: PetType) -> Self {
        let mut creature = Creature::new(true);
        creature.unit_mut().world_mut().set_name("Pet");

        let mut unit_type_mask = UNIT_MASK_SUMMON
            | UNIT_MASK_MINION
            | UNIT_MASK_GUARDIAN
            | UNIT_MASK_PET
            | UNIT_MASK_CONTROLABLE_GUARDIAN;
        if pet_type == PetType::Hunter {
            unit_type_mask |= UNIT_MASK_HUNTER_PET;
        }

        Self {
            creature,
            unit_type_mask,
            owner_guid,
            pet_type,
            duration_ms: 0,
            loading: false,
            removed: false,
            focus_regen_timer_ms: PET_FOCUS_REGEN_INTERVAL_MS,
            pet_experience: 0,
            pet_next_level_experience: 0,
            group_update_mask: 0,
            pet_specialization: 0,
            created_by_spell_id: 0,
            declined_name: None,
            declined_names: None,
            spells: BTreeMap::new(),
            autospells: Vec::new(),
        }
    }
    pub const fn creature(&self) -> &Creature {
        &self.creature
    }
    pub fn creature_mut(&mut self) -> &mut Creature {
        &mut self.creature
    }
    pub fn get_power_index(&self, power: PowerType) -> Option<usize> {
        self.creature.get_power_index(power)
    }
    pub const fn unit_type_mask(&self) -> u32 {
        self.unit_type_mask
    }
    pub const fn owner_guid(&self) -> ObjectGuid {
        self.owner_guid
    }
    pub const fn created_by_spell_id_like_cpp(&self) -> u32 {
        self.created_by_spell_id
    }
    pub fn set_created_by_spell_id_like_cpp(&mut self, spell_id: u32) {
        self.created_by_spell_id = spell_id;
    }
    pub fn add_to_world_like_cpp(&mut self) -> PetAddToWorldOutcomeLikeCpp {
        let guid = self.creature.guid();
        let mut inserted_pet_lookup = false;
        let mut unit_add_to_world = None;
        let mut aim_initialize_represented = false;
        let mut zone_script_on_creature_create_represented = false;

        if !self.creature.unit().world().object().is_in_world() {
            inserted_pet_lookup = true;
            unit_add_to_world = Some(self.creature.unit_mut().add_to_world_like_cpp());
            aim_initialize_represented = true;
            zone_script_on_creature_create_represented = true;
        }

        let follow_command_flags_reset = self
            .creature
            .unit_mut()
            .subsystems_mut()
            .control
            .charm_info
            .as_mut()
            .is_some_and(|charm_info| {
                if charm_info.command_state == crate::COMMAND_FOLLOW_LIKE_CPP as u8 {
                    charm_info.is_command_attack = false;
                    charm_info.is_command_follow = false;
                    charm_info.is_at_stay = false;
                    charm_info.is_following = false;
                    charm_info.is_returning = false;
                    true
                } else {
                    false
                }
            });

        PetAddToWorldOutcomeLikeCpp {
            guid,
            inserted_pet_lookup,
            unit_add_to_world,
            aim_initialize_represented,
            zone_script_on_creature_create_represented,
            follow_command_flags_reset,
        }
    }
    pub fn remove_from_world_like_cpp(&mut self) -> Option<PetRemoveFromWorldOutcomeLikeCpp> {
        let guid = self.creature.guid();
        if !self.creature.unit().world().object().is_in_world() {
            return None;
        }

        let unit_remove_from_world = self.creature.unit_mut().remove_from_world_like_cpp();

        Some(PetRemoveFromWorldOutcomeLikeCpp {
            guid,
            unit_remove_from_world,
            removed_pet_lookup: true,
        })
    }
    pub fn debug_info_with_guardian_like_cpp(&self, guardian_debug_info: &str) -> Option<String> {
        let pet_number = self
            .creature
            .unit()
            .subsystems()
            .control
            .charm_info
            .as_ref()
            .map(|charm_info| charm_info.pet_number)?;

        Some(format!(
            "{guardian_debug_info}\nPetType: {} PetNumber: {pet_number}",
            self.pet_type as u8
        ))
    }
    pub const fn pet_type(&self) -> PetType {
        self.pet_type
    }
    pub fn set_pet_type(&mut self, pet_type: PetType) {
        self.pet_type = pet_type;
        if pet_type == PetType::Hunter {
            self.unit_type_mask |= UNIT_MASK_HUNTER_PET;
        } else {
            self.unit_type_mask &= !UNIT_MASK_HUNTER_PET;
        }
    }
    pub const fn is_controlled(&self) -> bool {
        matches!(self.pet_type, PetType::Summon | PetType::Hunter)
    }
    pub const fn is_temporary_summoned(&self) -> bool {
        self.duration_ms > 0
    }
    pub const fn duration_ms(&self) -> i32 {
        self.duration_ms
    }
    pub fn set_duration(&mut self, duration_ms: i32) {
        self.duration_ms = duration_ms;
    }
    pub fn update_duration_like_cpp(&mut self, diff_ms: u32) -> PetDurationUpdateOutcome {
        if self.removed || self.loading || self.duration_ms <= 0 {
            return if self.removed || self.loading {
                PetDurationUpdateOutcome::Skipped
            } else {
                PetDurationUpdateOutcome::Active
            };
        }

        if self.duration_ms as u32 > diff_ms {
            self.duration_ms -= diff_ms as i32;
            PetDurationUpdateOutcome::Active
        } else {
            self.duration_ms = 0;
            let save_mode = if self.pet_type == PetType::Summon {
                PetSaveMode::NotInSlot
            } else {
                PetSaveMode::AsDeleted
            };
            PetDurationUpdateOutcome::Expired { save_mode }
        }
    }
    pub fn update_corpse_like_cpp(&self, now: i64) -> PetCorpseUpdateOutcome {
        if self.removed || self.loading {
            return PetCorpseUpdateOutcome::Skipped;
        }

        if self.creature.unit().death_state() != DeathState::Corpse {
            return PetCorpseUpdateOutcome::NotCorpse;
        }

        if self.pet_type != PetType::Hunter || self.creature.corpse_remove_time() <= now {
            PetCorpseUpdateOutcome::Remove {
                save_mode: PetSaveMode::NotInSlot,
            }
        } else {
            PetCorpseUpdateOutcome::KeepCorpse
        }
    }
    pub fn update_alive_owner_link_like_cpp(
        &self,
        owner_within_visibility_range: bool,
        is_possessed: bool,
        owner_pet_guid: Option<ObjectGuid>,
    ) -> PetAliveOwnerUpdateOutcome {
        if self.removed || self.loading {
            return PetAliveOwnerUpdateOutcome::Skipped;
        }

        if self.creature.unit().death_state() != DeathState::Alive {
            return PetAliveOwnerUpdateOutcome::NotAlive;
        }

        if (!owner_within_visibility_range && !is_possessed)
            || (self.is_controlled() && owner_pet_guid.is_none())
        {
            return PetAliveOwnerUpdateOutcome::RemoveLostOwner {
                save_mode: PetSaveMode::NotInSlot,
                return_reagent: true,
            };
        }

        if self.is_controlled() {
            let pet_guid = self.creature.guid();
            if owner_pet_guid != Some(pet_guid) {
                return PetAliveOwnerUpdateOutcome::RemoveUnlinkedControlled {
                    save_mode: PetSaveMode::NotInSlot,
                    unexpected_hunter: self.pet_type == PetType::Hunter,
                };
            }
        }

        PetAliveOwnerUpdateOutcome::Keep
    }
    pub fn remove_plan_like_cpp(
        &self,
        save_mode: PetSaveMode,
        return_reagent: bool,
    ) -> PetRemovePlanLikeCpp {
        PetRemovePlanLikeCpp {
            owner_guid: self.owner_guid,
            pet_guid: self.creature.guid(),
            save_mode,
            return_reagent,
        }
    }
    pub fn set_death_state_like_cpp(
        &mut self,
        state: DeathState,
        now: i64,
    ) -> PetDeathStateUpdateOutcome {
        let creature_plan = self.creature.set_death_state_runtime(state, now);
        let death_state = self.creature.unit().death_state();
        let mut cleared_hunter_corpse_flags = false;
        let mut cast_pet_auras_current = false;

        if death_state == DeathState::Corpse {
            if self.pet_type == PetType::Hunter {
                self.creature
                    .unit_mut()
                    .world_mut()
                    .object_mut()
                    .replace_all_dynamic_flags(0);
                let mut flags = self.creature.unit().unit_flags_like_cpp();
                flags.remove(UnitFlags::SKINNABLE);
                self.creature.unit_mut().set_unit_flags_like_cpp(flags);
                cleared_hunter_corpse_flags = true;
            }
        } else if death_state == DeathState::Alive {
            cast_pet_auras_current = true;
        }

        PetDeathStateUpdateOutcome {
            creature_plan,
            cleared_hunter_corpse_flags,
            cast_pet_auras_current,
        }
    }
    pub const fn is_loading(&self) -> bool {
        self.loading
    }
    pub fn set_loading(&mut self, loading: bool) {
        self.loading = loading;
    }
    pub const fn is_removed(&self) -> bool {
        self.removed
    }
    pub fn set_removed(&mut self, removed: bool) {
        self.removed = removed;
    }
    pub const fn focus_regen_timer_ms(&self) -> u32 {
        self.focus_regen_timer_ms
    }
    pub fn tick_focus_regen_timer(&mut self, diff_ms: u32) -> bool {
        if self.focus_regen_timer_ms > diff_ms {
            self.focus_regen_timer_ms -= diff_ms;
            false
        } else {
            let overshoot_ms = diff_ms - self.focus_regen_timer_ms;
            self.focus_regen_timer_ms = if overshoot_ms <= PET_FOCUS_REGEN_INTERVAL_MS {
                let remaining = PET_FOCUS_REGEN_INTERVAL_MS - overshoot_ms;
                remaining.max(1)
            } else {
                PET_FOCUS_REGEN_INTERVAL_MS
            };
            true
        }
    }
    pub fn regenerate_focus_like_cpp(
        &mut self,
        rate_power_focus: f32,
        aura_percent_multiplier: f32,
        aura_flat_modifier: i32,
        can_regenerate_power: bool,
    ) -> i32 {
        if !can_regenerate_power || self.get_power_index(PowerType::Focus).is_none() {
            return 0;
        }

        let cur_focus = self.creature.unit().get_power(PowerType::Focus);
        let max_focus = self.creature.unit().get_max_power(PowerType::Focus);
        if cur_focus >= max_focus {
            return 0;
        }

        let add_value = (PET_FOCUS_REGEN_AMOUNT_LIKE_CPP
            * rate_power_focus
            * aura_percent_multiplier)
            + (aura_flat_modifier as f32 * PET_FOCUS_REGEN_INTERVAL_MS as f32 / (5.0 * 1_000.0));
        let delta = add_value as i32;
        if delta == 0 {
            return 0;
        }

        let next_focus = (cur_focus + delta).clamp(0, max_focus);
        self.creature
            .unit_mut()
            .set_power(PowerType::Focus, next_focus);
        next_focus - cur_focus
    }
    pub const fn pet_experience(&self) -> u32 {
        self.pet_experience
    }
    pub fn set_pet_experience(&mut self, experience: u32) {
        self.pet_experience = experience;
    }
    pub const fn pet_next_level_experience(&self) -> u32 {
        self.pet_next_level_experience
    }
    pub fn set_pet_next_level_experience(&mut self, experience: u32) {
        self.pet_next_level_experience = experience;
    }
    pub fn give_pet_level_like_cpp(
        &mut self,
        level: u8,
        xp_for_level: impl Fn(u8) -> u32,
    ) -> PetLevelUpdateOutcome {
        let current_level = self.creature.level();
        if level == 0 || level == current_level {
            return PetLevelUpdateOutcome {
                changed: false,
                reset_experience: false,
                refresh_stats: false,
                init_levelup_spells: false,
            };
        }

        let mut reset_experience = false;
        if self.pet_type == PetType::Hunter {
            self.set_pet_experience(0);
            self.set_pet_next_level_experience(Self::pet_next_level_xp_for_owner_level(
                xp_for_level(level),
            ));
            reset_experience = true;
        }

        self.creature.unit_mut().set_level(level);
        PetLevelUpdateOutcome {
            changed: true,
            reset_experience,
            refresh_stats: true,
            init_levelup_spells: true,
        }
    }
    pub fn synchronize_level_with_owner_like_cpp(
        &mut self,
        owner_level: u8,
        xp_for_level: impl Fn(u8) -> u32,
    ) -> Option<PetLevelUpdateOutcome> {
        match self.pet_type {
            PetType::Summon | PetType::Hunter => {
                Some(self.give_pet_level_like_cpp(owner_level, xp_for_level))
            }
            PetType::Max => None,
        }
    }
    pub fn give_pet_xp_like_cpp(
        &mut self,
        xp: u32,
        max_player_level: u8,
        owner_level: u8,
        xp_for_level: impl Fn(u8) -> u32 + Copy,
    ) -> PetXpUpdateOutcome {
        let mut level_update = PetLevelUpdateOutcome {
            changed: false,
            reset_experience: false,
            refresh_stats: false,
            init_levelup_spells: false,
        };

        if self.pet_type != PetType::Hunter
            || xp < 1
            || self.creature.unit().death_state() != DeathState::Alive
        {
            return PetXpUpdateOutcome {
                accepted: false,
                levels_gained: 0,
                level_update,
            };
        }

        let max_level = max_player_level.min(owner_level);
        let mut pet_level = self.creature.level();
        if pet_level >= max_level {
            return PetXpUpdateOutcome {
                accepted: false,
                levels_gained: 0,
                level_update,
            };
        }

        let mut next_level_xp = self.pet_next_level_experience;
        let mut new_xp = self.pet_experience.wrapping_add(xp);
        let mut levels_gained = 0u8;

        while new_xp >= next_level_xp && pet_level < max_level {
            new_xp -= next_level_xp;
            pet_level = pet_level.saturating_add(1);
            levels_gained = levels_gained.saturating_add(1);
            level_update = self.give_pet_level_like_cpp(pet_level, xp_for_level);
            next_level_xp = self.pet_next_level_experience;
        }

        self.set_pet_experience(if pet_level < max_level { new_xp } else { 0 });

        PetXpUpdateOutcome {
            accepted: true,
            levels_gained,
            level_update,
        }
    }
    pub const fn group_update_mask(&self) -> u32 {
        self.group_update_mask
    }
    pub fn set_group_update_flag(&mut self, flag: u32) {
        self.group_update_mask |= flag;
    }
    pub fn set_group_update_flag_like_cpp(
        &mut self,
        flag: u32,
        owner_has_group: bool,
    ) -> Option<PetGroupUpdateOutcomeLikeCpp> {
        if !owner_has_group {
            return None;
        }

        self.group_update_mask |= flag;
        Some(PetGroupUpdateOutcomeLikeCpp {
            group_update_mask: self.group_update_mask,
            owner_group_flag: Some(GROUP_UPDATE_FLAG_PET_LIKE_CPP),
        })
    }
    pub fn reset_group_update_flag(&mut self) {
        self.group_update_mask = 0;
    }
    pub fn reset_group_update_flag_like_cpp(
        &mut self,
        owner_has_group: bool,
    ) -> PetGroupUpdateOutcomeLikeCpp {
        self.group_update_mask = GROUP_UPDATE_FLAG_PET_NONE_LIKE_CPP;
        PetGroupUpdateOutcomeLikeCpp {
            group_update_mask: self.group_update_mask,
            owner_group_flag: owner_has_group.then_some(GROUP_UPDATE_FLAG_PET_LIKE_CPP),
        }
    }
    pub fn set_display_id_like_cpp(
        &mut self,
        model_id: u32,
        set_native: bool,
        owner_has_group: bool,
    ) -> PetSetDisplayIdOutcomeLikeCpp {
        self.creature.set_display_id(model_id, set_native, None);
        let group_update = self
            .is_controlled()
            .then(|| {
                self.set_group_update_flag_like_cpp(
                    GROUP_UPDATE_FLAG_PET_MODEL_ID_LIKE_CPP,
                    owner_has_group,
                )
            })
            .flatten();

        PetSetDisplayIdOutcomeLikeCpp {
            model_id,
            set_native,
            group_update,
        }
    }
    pub fn have_in_diet_like_cpp(
        item_food_type: u32,
        creature_has_template: bool,
        creature_family_pet_food_mask: Option<u32>,
    ) -> bool {
        if item_food_type == 0 || !creature_has_template {
            return false;
        }

        let Some(diet) = creature_family_pet_food_mask else {
            return false;
        };

        let food_shift = item_food_type.saturating_sub(1);
        let Some(food_mask) = 1u32.checked_shl(food_shift) else {
            return false;
        };

        (diet & food_mask) != 0
    }
    pub fn native_object_scale_like_cpp(
        pet_type: PetType,
        level: u8,
        guardian_native_scale: f32,
        creature_family_scale: Option<PetFamilyScaleLikeCpp>,
    ) -> f32 {
        let Some(family) = creature_family_scale else {
            return guardian_native_scale;
        };

        if family.min_scale <= 0.0 || pet_type != PetType::Hunter {
            return guardian_native_scale;
        }

        if level >= family.max_scale_level {
            family.max_scale
        } else if level <= family.min_scale_level {
            family.min_scale
        } else {
            family.min_scale
                + (level - family.min_scale_level) as f32 / family.max_scale_level as f32
                    * (family.max_scale - family.min_scale)
        }
    }
    pub const fn is_permanent_pet_for_like_cpp(
        pet_type: PetType,
        owner_class: Class,
        creature_type: CreatureType,
    ) -> bool {
        match pet_type {
            PetType::Summon => match owner_class {
                Class::Warlock => matches!(creature_type, CreatureType::Demon),
                Class::DeathKnight => matches!(creature_type, CreatureType::Undead),
                Class::Mage => matches!(creature_type, CreatureType::Elemental),
                _ => false,
            },
            PetType::Hunter => true,
            PetType::Max => false,
        }
    }
    pub const fn specialization(&self) -> u16 {
        self.pet_specialization
    }
    pub fn set_specialization(&mut self, specialization: u16) {
        self.pet_specialization = specialization;
    }
    pub fn learn_specialization_spells_plan_like_cpp(
        pet_level: u8,
        spec_spells: &[PetSpecializationSpellLikeCpp],
    ) -> Vec<u32> {
        spec_spells
            .iter()
            .filter_map(|spec_spell| {
                if spec_spell.spell_exists && spec_spell.spell_level <= pet_level {
                    Some(spec_spell.spell_id)
                } else {
                    None
                }
            })
            .collect()
    }
    pub fn remove_specialization_spells_plan_like_cpp(
        normal_spec_spells_by_index: &[&[u32]],
        override_spec_spells_by_index: &[&[u32]],
    ) -> Vec<u32> {
        let mut unlearned_spells = Vec::new();
        for index in 0..PET_MAX_SPECIALIZATIONS_LIKE_CPP {
            if let Some(spells) = normal_spec_spells_by_index.get(index) {
                unlearned_spells.extend_from_slice(spells);
            }
            if let Some(spells) = override_spec_spells_by_index.get(index) {
                unlearned_spells.extend_from_slice(spells);
            }
        }
        unlearned_spells
    }
    pub fn set_specialization_like_cpp(
        &mut self,
        spec: u16,
        spec_exists: bool,
        learned_spec_spells: &[PetSpecializationSpellLikeCpp],
        normal_spec_spells_by_index: &[&[u32]],
        override_spec_spells_by_index: &[&[u32]],
    ) -> PetSetSpecializationOutcomeLikeCpp {
        if self.pet_specialization == spec {
            return PetSetSpecializationOutcomeLikeCpp {
                changed: false,
                removed_specialization_spells: Vec::new(),
                remove_learn_prev: false,
                remove_clear_action_bar: false,
                learned_specialization_spells: Vec::new(),
                cleanup_action_bar: false,
                pet_spell_initialize: false,
                packet_spec_id: None,
            };
        }

        let removed_specialization_spells = Self::remove_specialization_spells_plan_like_cpp(
            normal_spec_spells_by_index,
            override_spec_spells_by_index,
        );

        if !spec_exists {
            self.pet_specialization = 0;
            return PetSetSpecializationOutcomeLikeCpp {
                changed: true,
                removed_specialization_spells,
                remove_learn_prev: true,
                remove_clear_action_bar: false,
                learned_specialization_spells: Vec::new(),
                cleanup_action_bar: false,
                pet_spell_initialize: false,
                packet_spec_id: None,
            };
        }

        self.pet_specialization = spec;
        let learned_specialization_spells = Self::learn_specialization_spells_plan_like_cpp(
            self.creature.level(),
            learned_spec_spells,
        );

        PetSetSpecializationOutcomeLikeCpp {
            changed: true,
            removed_specialization_spells,
            remove_learn_prev: true,
            remove_clear_action_bar: false,
            learned_specialization_spells,
            cleanup_action_bar: true,
            pet_spell_initialize: true,
            packet_spec_id: Some(self.pet_specialization),
        }
    }
    pub fn declined_name(&self) -> Option<&str> {
        self.declined_name.as_deref()
    }
    pub fn set_declined_name(&mut self, declined_name: Option<String>) {
        self.declined_name = declined_name;
    }
    pub fn declined_names(&self) -> Option<&PetDeclinedNamesLikeCpp> {
        self.declined_names.as_ref()
    }
    pub fn set_declined_names(&mut self, declined_names: Option<PetDeclinedNamesLikeCpp>) {
        self.declined_names = declined_names;
    }
    pub fn spells(&self) -> &BTreeMap<u32, PetSpell> {
        &self.spells
    }
    pub fn autospells(&self) -> &[u32] {
        &self.autospells
    }
    pub fn get_pet_auto_spell_size(&self) -> u8 {
        self.autospells.len().min(u8::MAX as usize) as u8
    }
    pub fn get_pet_auto_spell_on_pos(&self, pos: u8) -> u32 {
        self.autospells
            .get(pos as usize)
            .copied()
            .unwrap_or_default()
    }
    pub fn has_spell(&self, spell_id: u32) -> bool {
        self.spells
            .get(&spell_id)
            .is_some_and(|spell| spell.state != PetSpellState::Removed)
    }
    pub fn add_spell(
        &mut self,
        spell_id: u32,
        active: ActiveState,
        mut state: PetSpellState,
        spell_type: PetSpellType,
    ) -> bool {
        if spell_id == 0 {
            return false;
        }

        if let Some(existing) = self.spells.get_mut(&spell_id) {
            if existing.state == PetSpellState::Removed {
                state = PetSpellState::Changed;
            } else {
                if state == PetSpellState::Unchanged && existing.state != PetSpellState::Unchanged {
                    existing.state = PetSpellState::Unchanged;
                    if active == ActiveState::Enabled || active == ActiveState::Disabled {
                        existing.active = active;
                        if active == ActiveState::Enabled {
                            if !self.autospells.contains(&spell_id) {
                                self.autospells.push(spell_id);
                            }
                        } else {
                            self.autospells.retain(|known| *known != spell_id);
                        }
                    }
                }
                return false;
            }
        }

        let active = if active == ActiveState::Decide {
            ActiveState::Disabled
        } else {
            active
        };

        let spell = PetSpell {
            active,
            state,
            spell_type,
        };
        self.spells.insert(spell_id, spell);
        self.sync_autospell(spell_id, active);
        true
    }
    pub fn learn_spell_like_cpp(&mut self, spell_id: u32) -> PetLearnSpellOutcomeLikeCpp {
        let learned = self.add_spell(
            spell_id,
            ActiveState::Decide,
            PetSpellState::New,
            PetSpellType::Normal,
        );
        let packet_spell_ids = if learned { vec![spell_id] } else { Vec::new() };

        PetLearnSpellOutcomeLikeCpp {
            learned,
            packet_spell_ids,
            send_direct_message: learned && !self.loading,
            pet_spell_initialize: learned && !self.loading,
        }
    }
    pub fn learn_spells_like_cpp(
        &mut self,
        spell_ids: impl IntoIterator<Item = u32>,
    ) -> PetLearnSpellsOutcomeLikeCpp {
        let mut learned_spell_ids = Vec::new();

        for spell_id in spell_ids {
            if self.add_spell(
                spell_id,
                ActiveState::Decide,
                PetSpellState::New,
                PetSpellType::Normal,
            ) {
                learned_spell_ids.push(spell_id);
            }
        }

        PetLearnSpellsOutcomeLikeCpp {
            packet_spell_ids: learned_spell_ids.clone(),
            learned_spell_ids,
            send_session_packet: !self.loading,
        }
    }
    pub fn init_pet_create_spells_like_cpp(
        &mut self,
        charm_info: &mut CharmInfoState,
        creature_family_id_like_cpp: Option<u32>,
        creature_family_exists_like_cpp: impl FnMut(u32) -> bool,
        pet_family_spells_like_cpp: impl FnMut(u32) -> Vec<u32>,
    ) -> PetInitCreateSpellsOutcomeLikeCpp {
        charm_info.init_pet_action_bar_like_cpp();

        let cleared_spell_count = self.spells.len();
        let cleared_autospell_count = self.autospells.len();
        self.spells.clear();
        self.autospells.clear();

        let learn_pet_passives = self.learn_pet_passives_like_cpp(
            creature_family_id_like_cpp,
            creature_family_exists_like_cpp,
            pet_family_spells_like_cpp,
        );

        PetInitCreateSpellsOutcomeLikeCpp {
            init_pet_action_bar: true,
            cleared_spell_count,
            cleared_autospell_count,
            learn_pet_passives,
            init_levelup_spells_for_level: true,
            cast_pet_auras_current: false,
        }
    }
    pub fn remove_spell(&mut self, spell_id: u32) -> bool {
        if let Some(spell) = self.spells.get_mut(&spell_id) {
            spell.state = PetSpellState::Removed;
            self.autospells.retain(|known| *known != spell_id);
            return true;
        }
        false
    }
    pub fn unlearn_spell_like_cpp(
        &mut self,
        spell_id: u32,
        learn_prev: bool,
        clear_action_bar: bool,
        action_bar: &mut [u32; MAX_UNIT_ACTION_BAR_INDEX],
        prev_spell_in_chain_like_cpp: impl FnMut(u32) -> u32,
        first_spell_in_chain_like_cpp: impl FnMut(u32) -> u32,
    ) -> PetUnlearnSpellOutcomeLikeCpp {
        let remove_spell = self.remove_spell_like_cpp(
            spell_id,
            learn_prev,
            clear_action_bar,
            action_bar,
            prev_spell_in_chain_like_cpp,
            first_spell_in_chain_like_cpp,
        );
        let packet_spell_ids = if remove_spell.removed {
            vec![spell_id]
        } else {
            Vec::new()
        };

        PetUnlearnSpellOutcomeLikeCpp {
            send_direct_message: remove_spell.removed && !self.loading,
            remove_spell,
            packet_spell_ids,
        }
    }
    pub fn unlearn_spells_like_cpp(
        &mut self,
        spell_ids: impl IntoIterator<Item = u32>,
        learn_prev: bool,
        clear_action_bar: bool,
        action_bar: &mut [u32; MAX_UNIT_ACTION_BAR_INDEX],
        mut prev_spell_in_chain_like_cpp: impl FnMut(u32) -> u32,
        mut first_spell_in_chain_like_cpp: impl FnMut(u32) -> u32,
    ) -> PetUnlearnSpellsOutcomeLikeCpp {
        let mut remove_spells = Vec::new();
        let mut packet_spell_ids = Vec::new();

        for spell_id in spell_ids {
            let remove_spell = self.remove_spell_like_cpp(
                spell_id,
                learn_prev,
                clear_action_bar,
                action_bar,
                &mut prev_spell_in_chain_like_cpp,
                &mut first_spell_in_chain_like_cpp,
            );
            if remove_spell.removed {
                packet_spell_ids.push(spell_id);
            }
            remove_spells.push((spell_id, remove_spell));
        }

        PetUnlearnSpellsOutcomeLikeCpp {
            remove_spells,
            packet_spell_ids,
            send_session_packet: !self.loading,
        }
    }
}
