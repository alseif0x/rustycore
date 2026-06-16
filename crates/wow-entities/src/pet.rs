use std::collections::BTreeMap;

use wow_constants::{DeathState, PowerType, UnitFlags};
use wow_core::ObjectGuid;

use crate::{
    Creature, CreatureRuntimePlan, ReactState, UNIT_MASK_CONTROLABLE_GUARDIAN, UNIT_MASK_GUARDIAN,
    UNIT_MASK_HUNTER_PET, UNIT_MASK_MINION, UNIT_MASK_PET, UNIT_MASK_SUMMON,
    unit_action_button_action_like_cpp, unit_action_button_type_like_cpp,
};

pub const HAPPINESS_LEVEL_SIZE: u32 = 333_000;
pub const MAX_ACTIVE_PETS: usize = 5;
pub const MAX_PET_STABLES: usize = 200;
pub const PET_FOCUS_REGEN_AMOUNT_LIKE_CPP: f32 = 24.0;
pub const PET_FOCUS_REGEN_INTERVAL_MS: u32 = 4_000;
pub const PET_XP_FACTOR: f32 = 0.05;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PetType {
    Summon = 0,
    Hunter = 1,
    Max = 4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(i16)]
pub enum PetSaveMode {
    AsDeleted = -2,
    AsCurrent = -3,
    FirstActiveSlot = 0,
    NotInSlot = -1,
}

impl PetSaveMode {
    pub const fn active_slot(index: u8) -> i16 {
        index as i16
    }

    pub const fn stable_slot(index: u16) -> i16 {
        5 + index as i16
    }

    pub const fn is_active_slot(slot: i16) -> bool {
        slot >= 0 && slot < MAX_ACTIVE_PETS as i16
    }

    pub const fn is_stabled_slot(slot: i16) -> bool {
        slot >= 5 && slot < (5 + MAX_PET_STABLES as i16)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ActiveState {
    Decide = 0x00,
    Passive = 0x01,
    Disabled = 0x81,
    Enabled = 0xC1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PetSpellState {
    Unchanged = 0,
    Changed = 1,
    New = 2,
    Removed = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PetSpellType {
    Normal = 0,
    Family = 1,
    Talent = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PetSpell {
    pub active: ActiveState,
    pub state: PetSpellState,
    pub spell_type: PetSpellType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PetDeclinedNamesLikeCpp {
    pub names: [String; 5],
}

#[derive(Debug, Clone, PartialEq)]
pub struct PetStableInfo {
    pub name: String,
    pub action_bar: String,
    pub pet_number: u32,
    pub creature_id: u32,
    pub display_id: u32,
    pub experience: u32,
    pub health: u32,
    pub mana: u32,
    pub last_save_time: u32,
    pub created_by_spell_id: u32,
    pub specialization_id: u16,
    pub level: u8,
    pub react_state: ReactState,
    pub pet_type: PetType,
    pub was_renamed: bool,
}

impl Default for PetStableInfo {
    fn default() -> Self {
        Self {
            name: String::new(),
            action_bar: String::new(),
            pet_number: 0,
            creature_id: 0,
            display_id: 0,
            experience: 0,
            health: 0,
            mana: 0,
            last_save_time: 0,
            created_by_spell_id: 0,
            specialization_id: 0,
            level: 0,
            react_state: ReactState::Passive,
            pet_type: PetType::Max,
            was_renamed: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct PetStable {
    pub current_pet_index: Option<u32>,
    pub active_pets: Vec<Option<PetStableInfo>>,
    pub stabled_pets: Vec<Option<PetStableInfo>>,
    pub unslotted_pets: Vec<PetStableInfo>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PetLoadSelection {
    pub pet_number: u32,
    pub creature_id: u32,
    pub slot: i16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PetLoadInfoResult {
    Found(PetLoadSelection),
    Deleted,
}

impl PetLoadInfoResult {
    pub const fn selection(self) -> Option<PetLoadSelection> {
        match self {
            Self::Found(selection) => Some(selection),
            Self::Deleted => None,
        }
    }

    pub const fn save_mode(self) -> i16 {
        match self {
            Self::Found(selection) => selection.slot,
            Self::Deleted => PetSaveMode::AsDeleted as i16,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PetDurationUpdateOutcome {
    Skipped,
    Active,
    Expired { save_mode: PetSaveMode },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PetCorpseUpdateOutcome {
    Skipped,
    NotCorpse,
    KeepCorpse,
    Remove { save_mode: PetSaveMode },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PetAliveOwnerUpdateOutcome {
    Skipped,
    NotAlive,
    Keep,
    RemoveLostOwner {
        save_mode: PetSaveMode,
        return_reagent: bool,
    },
    RemoveUnlinkedControlled {
        save_mode: PetSaveMode,
        unexpected_hunter: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PetLevelUpdateOutcome {
    pub changed: bool,
    pub reset_experience: bool,
    pub refresh_stats: bool,
    pub init_levelup_spells: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PetXpUpdateOutcome {
    pub accepted: bool,
    pub levels_gained: u8,
    pub level_update: PetLevelUpdateOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PetDeathStateUpdateOutcome {
    pub creature_plan: CreatureRuntimePlan,
    pub cleared_hunter_corpse_flags: bool,
    pub cast_pet_auras_current: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Pet {
    creature: Creature,
    unit_type_mask: u32,
    owner_guid: ObjectGuid,
    pet_type: PetType,
    duration_ms: i32,
    loading: bool,
    removed: bool,
    focus_regen_timer_ms: u32,
    pet_experience: u32,
    pet_next_level_experience: u32,
    group_update_mask: u32,
    pet_specialization: u16,
    declined_name: Option<String>,
    declined_names: Option<PetDeclinedNamesLikeCpp>,
    spells: BTreeMap<u32, PetSpell>,
    autospells: Vec<u32>,
}

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

    pub fn reset_group_update_flag(&mut self) {
        self.group_update_mask = 0;
    }

    pub const fn specialization(&self) -> u16 {
        self.pet_specialization
    }

    pub fn set_specialization(&mut self, specialization: u16) {
        self.pet_specialization = specialization;
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
        state: PetSpellState,
        spell_type: PetSpellType,
    ) -> bool {
        if spell_id == 0 {
            return false;
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
        let changed = self.spells.get(&spell_id).copied() != Some(spell);
        self.spells.insert(spell_id, spell);
        self.sync_autospell(spell_id, active);
        changed
    }

    pub fn remove_spell(&mut self, spell_id: u32) -> bool {
        if let Some(spell) = self.spells.get_mut(&spell_id) {
            spell.state = PetSpellState::Removed;
            self.autospells.retain(|known| *known != spell_id);
            return true;
        }
        false
    }

    pub fn toggle_autocast(&mut self, spell_id: u32, apply: bool) -> bool {
        let Some(spell) = self.spells.get_mut(&spell_id) else {
            return false;
        };
        let active = if apply {
            ActiveState::Enabled
        } else {
            ActiveState::Disabled
        };
        spell.active = active;
        self.sync_autospell(spell_id, active);
        true
    }

    pub fn is_permanent_pet_for(&self, owner_guid: ObjectGuid, pet_number: u32) -> bool {
        self.owner_guid == owner_guid && self.pet_type == PetType::Hunter && pet_number != 0
    }

    pub fn pet_next_level_xp_for_owner_level(owner_next_level_xp: u32) -> u32 {
        (owner_next_level_xp as f32 * PET_XP_FACTOR) as u32
    }

    pub fn generate_action_bar_data_like_cpp(action_bar: &[u32]) -> String {
        let mut data = String::new();
        for packed in action_bar.iter().take(10) {
            data.push_str(&format!(
                "{} {} ",
                unit_action_button_type_like_cpp(*packed),
                unit_action_button_action_like_cpp(*packed)
            ));
        }
        data
    }

    pub fn fill_pet_info_like_cpp(
        &self,
        pet_number: u32,
        action_bar: &[u32],
        forced_react_state: Option<ReactState>,
        can_be_renamed: bool,
        last_save_time: u32,
        created_by_spell_id: u32,
    ) -> PetStableInfo {
        let unit = self.creature.unit();
        PetStableInfo {
            name: unit.world().name().to_string(),
            action_bar: Self::generate_action_bar_data_like_cpp(action_bar),
            pet_number,
            creature_id: self.creature.entry(),
            display_id: unit.data().native_display_id as u32,
            experience: self.pet_experience,
            health: unit.data().health as u32,
            mana: unit.get_power(PowerType::Mana) as u32,
            last_save_time,
            created_by_spell_id,
            specialization_id: self.pet_specialization,
            level: self.creature.level(),
            react_state: forced_react_state.unwrap_or_else(|| self.creature.react_state()),
            pet_type: self.pet_type,
            was_renamed: !can_be_renamed,
        }
    }

    pub fn get_load_pet_info(
        stable: &PetStable,
        pet_entry: u32,
        pet_number: u32,
        slot: Option<i16>,
    ) -> Option<PetLoadSelection> {
        Self::get_load_pet_info_result_like_cpp(stable, pet_entry, pet_number, slot).selection()
    }

    pub fn get_load_pet_info_result_like_cpp(
        stable: &PetStable,
        pet_entry: u32,
        pet_number: u32,
        slot: Option<i16>,
    ) -> PetLoadInfoResult {
        if pet_number != 0 {
            for (index, pet) in stable.active_pets.iter().enumerate() {
                if let Some(pet) = pet {
                    if pet.pet_number == pet_number {
                        return PetLoadInfoResult::Found(PetLoadSelection {
                            pet_number: pet.pet_number,
                            creature_id: pet.creature_id,
                            slot: index as i16,
                        });
                    }
                }
            }
            for (index, pet) in stable.stabled_pets.iter().enumerate() {
                if let Some(pet) = pet {
                    if pet.pet_number == pet_number {
                        return PetLoadInfoResult::Found(PetLoadSelection {
                            pet_number: pet.pet_number,
                            creature_id: pet.creature_id,
                            slot: PetSaveMode::stable_slot(index as u16),
                        });
                    }
                }
            }
            for pet in &stable.unslotted_pets {
                if pet.pet_number == pet_number {
                    return PetLoadInfoResult::Found(PetLoadSelection {
                        pet_number: pet.pet_number,
                        creature_id: pet.creature_id,
                        slot: PetSaveMode::NotInSlot as i16,
                    });
                }
            }
        } else if let Some(slot) = slot {
            if slot == PetSaveMode::AsCurrent as i16 {
                if let Some(index) = stable.current_pet_index {
                    if let Some(Some(pet)) = stable.active_pets.get(index as usize) {
                        return PetLoadInfoResult::Found(PetLoadSelection {
                            pet_number: pet.pet_number,
                            creature_id: pet.creature_id,
                            slot: index as i16,
                        });
                    }
                }
            } else if PetSaveMode::is_active_slot(slot) {
                if let Some(Some(pet)) = stable.active_pets.get(slot as usize) {
                    return PetLoadInfoResult::Found(PetLoadSelection {
                        pet_number: pet.pet_number,
                        creature_id: pet.creature_id,
                        slot,
                    });
                }
            } else if PetSaveMode::is_stabled_slot(slot) {
                let index = (slot - PetSaveMode::stable_slot(0)) as usize;
                if let Some(Some(pet)) = stable.stabled_pets.get(index) {
                    return PetLoadInfoResult::Found(PetLoadSelection {
                        pet_number: pet.pet_number,
                        creature_id: pet.creature_id,
                        slot,
                    });
                }
            }
        } else if pet_entry != 0 {
            for pet in &stable.unslotted_pets {
                if pet.creature_id == pet_entry {
                    return PetLoadInfoResult::Found(PetLoadSelection {
                        pet_number: pet.pet_number,
                        creature_id: pet.creature_id,
                        slot: PetSaveMode::NotInSlot as i16,
                    });
                }
            }
        } else {
            if let Some(Some(pet)) = stable.active_pets.first() {
                return PetLoadInfoResult::Found(PetLoadSelection {
                    pet_number: pet.pet_number,
                    creature_id: pet.creature_id,
                    slot: PetSaveMode::FirstActiveSlot as i16,
                });
            }
            if let Some(pet) = stable.unslotted_pets.first() {
                return PetLoadInfoResult::Found(PetLoadSelection {
                    pet_number: pet.pet_number,
                    creature_id: pet.creature_id,
                    slot: PetSaveMode::NotInSlot as i16,
                });
            }
        }

        PetLoadInfoResult::Deleted
    }

    fn sync_autospell(&mut self, spell_id: u32, active: ActiveState) {
        if active == ActiveState::Enabled {
            if !self.autospells.contains(&spell_id) {
                self.autospells.push(spell_id);
            }
        } else {
            self.autospells.retain(|known| *known != spell_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn owner_guid() -> ObjectGuid {
        ObjectGuid::create_global(wow_core::guid::HighGuid::Player, 0, 1)
    }

    fn pet_guid(counter: i64) -> ObjectGuid {
        ObjectGuid::new((wow_core::guid::HighGuid::Pet as i64) << 58, counter)
    }

    fn pet_info(pet_number: u32, creature_id: u32) -> PetStableInfo {
        PetStableInfo {
            pet_number,
            creature_id,
            ..PetStableInfo::default()
        }
    }

    #[test]
    fn pet_constructor_matches_cpp_guardian_base_state() {
        let summon = Pet::new(owner_guid(), PetType::Summon);

        assert!(summon.creature().unit().world().is_world_object());
        assert_eq!(summon.creature().unit().world().name(), "Pet");
        assert_eq!(
            summon.unit_type_mask(),
            UNIT_MASK_SUMMON
                | UNIT_MASK_MINION
                | UNIT_MASK_GUARDIAN
                | UNIT_MASK_PET
                | UNIT_MASK_CONTROLABLE_GUARDIAN
        );
        assert_eq!(summon.owner_guid(), owner_guid());
        assert_eq!(summon.pet_type(), PetType::Summon);
        assert!(summon.is_controlled());
        assert!(!summon.is_temporary_summoned());
        assert_eq!(summon.duration_ms(), 0);
        assert!(!summon.is_loading());
        assert!(!summon.is_removed());
        assert_eq!(summon.focus_regen_timer_ms(), PET_FOCUS_REGEN_INTERVAL_MS);
        assert_eq!(summon.group_update_mask(), 0);
        assert_eq!(summon.specialization(), 0);
        assert!(summon.declined_name().is_none());
        assert!(summon.declined_names().is_none());

        let hunter = Pet::new(owner_guid(), PetType::Hunter);
        assert!((hunter.unit_type_mask() & UNIT_MASK_HUNTER_PET) != 0);
    }

    #[test]
    fn pet_power_index_delegates_to_creature_bridge() {
        let mut pet = Pet::new(owner_guid(), PetType::Hunter);

        assert_eq!(pet.get_power_index(PowerType::Mana), Some(0));
        assert_eq!(pet.get_power_index(PowerType::ComboPoints), Some(2));
        assert_eq!(pet.get_power_index(PowerType::Energy), None);

        pet.creature_mut().set_power_type(PowerType::Focus);
        assert_eq!(pet.get_power_index(PowerType::Focus), Some(0));
        assert_eq!(pet.get_power_index(PowerType::Mana), None);
    }

    #[test]
    fn pet_type_duration_group_and_focus_state_follow_cpp_shape() {
        let mut pet = Pet::new(owner_guid(), PetType::Max);

        assert!(!pet.is_controlled());
        pet.set_pet_type(PetType::Hunter);
        assert!(pet.is_controlled());
        assert!((pet.unit_type_mask() & UNIT_MASK_HUNTER_PET) != 0);

        pet.set_duration(100);
        assert!(pet.is_temporary_summoned());
        assert!(!pet.tick_focus_regen_timer(1_000));
        assert_eq!(pet.focus_regen_timer_ms(), 3_000);
        assert!(pet.tick_focus_regen_timer(3_000));
        assert_eq!(pet.focus_regen_timer_ms(), PET_FOCUS_REGEN_INTERVAL_MS);

        pet.set_group_update_flag(0x1);
        pet.set_group_update_flag(0x4);
        assert_eq!(pet.group_update_mask(), 0x5);
        pet.reset_group_update_flag();
        assert_eq!(pet.group_update_mask(), 0);
    }

    #[test]
    fn pet_duration_update_matches_cpp_temporary_summon_expiry_modes() {
        let mut summon = Pet::new(owner_guid(), PetType::Summon);
        summon.set_duration(100);
        assert_eq!(
            summon.update_duration_like_cpp(40),
            PetDurationUpdateOutcome::Active
        );
        assert_eq!(summon.duration_ms(), 60);
        assert_eq!(
            summon.update_duration_like_cpp(60),
            PetDurationUpdateOutcome::Expired {
                save_mode: PetSaveMode::NotInSlot
            }
        );
        assert_eq!(summon.duration_ms(), 0);

        let mut hunter = Pet::new(owner_guid(), PetType::Hunter);
        hunter.set_duration(1);
        assert_eq!(
            hunter.update_duration_like_cpp(1),
            PetDurationUpdateOutcome::Expired {
                save_mode: PetSaveMode::AsDeleted
            }
        );
    }

    #[test]
    fn pet_duration_update_skips_cpp_removed_loading_and_permanent_states() {
        let mut permanent = Pet::new(owner_guid(), PetType::Summon);
        assert_eq!(
            permanent.update_duration_like_cpp(100),
            PetDurationUpdateOutcome::Active
        );

        let mut removed = Pet::new(owner_guid(), PetType::Summon);
        removed.set_duration(100);
        removed.set_removed(true);
        assert_eq!(
            removed.update_duration_like_cpp(100),
            PetDurationUpdateOutcome::Skipped
        );
        assert_eq!(removed.duration_ms(), 100);

        let mut loading = Pet::new(owner_guid(), PetType::Summon);
        loading.set_duration(100);
        loading.set_loading(true);
        assert_eq!(
            loading.update_duration_like_cpp(100),
            PetDurationUpdateOutcome::Skipped
        );
        assert_eq!(loading.duration_ms(), 100);
    }

    #[test]
    fn pet_corpse_update_matches_cpp_hunter_corpse_keep_and_remove() {
        let mut pet = Pet::new(owner_guid(), PetType::Hunter);
        pet.creature_mut().set_corpse_delay(15, false);
        pet.creature_mut()
            .set_death_state_runtime(DeathState::JustDied, 1_000);

        assert_eq!(pet.creature().unit().death_state(), DeathState::Corpse);
        assert_eq!(pet.creature().corpse_remove_time(), 1_015);
        assert_eq!(
            pet.update_corpse_like_cpp(1_014),
            PetCorpseUpdateOutcome::KeepCorpse
        );
        assert_eq!(
            pet.update_corpse_like_cpp(1_015),
            PetCorpseUpdateOutcome::Remove {
                save_mode: PetSaveMode::NotInSlot
            }
        );
    }

    #[test]
    fn pet_corpse_update_removes_non_hunter_corpses_like_cpp() {
        let mut pet = Pet::new(owner_guid(), PetType::Summon);
        pet.creature_mut().set_corpse_delay(15, false);
        pet.creature_mut()
            .set_death_state_runtime(DeathState::JustDied, 1_000);

        assert_eq!(
            pet.update_corpse_like_cpp(1_001),
            PetCorpseUpdateOutcome::Remove {
                save_mode: PetSaveMode::NotInSlot
            }
        );
    }

    #[test]
    fn pet_corpse_update_skips_cpp_removed_loading_and_non_corpse_states() {
        let pet = Pet::new(owner_guid(), PetType::Hunter);
        assert_eq!(
            pet.update_corpse_like_cpp(1_000),
            PetCorpseUpdateOutcome::NotCorpse
        );

        let mut removed = Pet::new(owner_guid(), PetType::Hunter);
        removed.set_removed(true);
        assert_eq!(
            removed.update_corpse_like_cpp(1_000),
            PetCorpseUpdateOutcome::Skipped
        );

        let mut loading = Pet::new(owner_guid(), PetType::Hunter);
        loading.set_loading(true);
        assert_eq!(
            loading.update_corpse_like_cpp(1_000),
            PetCorpseUpdateOutcome::Skipped
        );
    }

    #[test]
    fn pet_alive_owner_link_removes_lost_owner_like_cpp() {
        let mut pet = Pet::new(owner_guid(), PetType::Hunter);
        pet.creature_mut()
            .unit_mut()
            .world_mut()
            .object_mut()
            .create(pet_guid(10));

        assert_eq!(
            pet.update_alive_owner_link_like_cpp(false, false, Some(pet_guid(10))),
            PetAliveOwnerUpdateOutcome::RemoveLostOwner {
                save_mode: PetSaveMode::NotInSlot,
                return_reagent: true
            },
            "C++ removes when pet is outside owner visibility range and not possessed"
        );
        assert_eq!(
            pet.update_alive_owner_link_like_cpp(true, false, None),
            PetAliveOwnerUpdateOutcome::RemoveLostOwner {
                save_mode: PetSaveMode::NotInSlot,
                return_reagent: true
            },
            "C++ removes controlled pets when owner->GetPetGUID() is empty"
        );
    }

    #[test]
    fn pet_alive_owner_link_removes_unlinked_controlled_pet_like_cpp() {
        let mut summon = Pet::new(owner_guid(), PetType::Summon);
        summon
            .creature_mut()
            .unit_mut()
            .world_mut()
            .object_mut()
            .create(pet_guid(11));

        assert_eq!(
            summon.update_alive_owner_link_like_cpp(true, false, Some(pet_guid(12))),
            PetAliveOwnerUpdateOutcome::RemoveUnlinkedControlled {
                save_mode: PetSaveMode::NotInSlot,
                unexpected_hunter: false
            }
        );

        let mut hunter = Pet::new(owner_guid(), PetType::Hunter);
        hunter
            .creature_mut()
            .unit_mut()
            .world_mut()
            .object_mut()
            .create(pet_guid(13));
        assert_eq!(
            hunter.update_alive_owner_link_like_cpp(true, false, Some(pet_guid(14))),
            PetAliveOwnerUpdateOutcome::RemoveUnlinkedControlled {
                save_mode: PetSaveMode::NotInSlot,
                unexpected_hunter: true
            },
            "C++ ASSERTs this unexpected hunter-pet unlink case before Remove(PET_SAVE_NOT_IN_SLOT)"
        );
    }

    #[test]
    fn pet_alive_owner_link_keeps_valid_alive_and_skips_non_alive_like_cpp() {
        let mut pet = Pet::new(owner_guid(), PetType::Hunter);
        pet.creature_mut()
            .unit_mut()
            .world_mut()
            .object_mut()
            .create(pet_guid(15));

        assert_eq!(
            pet.update_alive_owner_link_like_cpp(true, false, Some(pet_guid(15))),
            PetAliveOwnerUpdateOutcome::Keep
        );
        assert_eq!(
            pet.update_alive_owner_link_like_cpp(false, true, Some(pet_guid(15))),
            PetAliveOwnerUpdateOutcome::Keep,
            "C++ allows out-of-range possessed pets through the distance branch"
        );

        pet.creature_mut()
            .set_death_state_runtime(DeathState::JustDied, 1_000);
        assert_eq!(
            pet.update_alive_owner_link_like_cpp(true, false, Some(pet_guid(15))),
            PetAliveOwnerUpdateOutcome::NotAlive
        );
    }

    #[test]
    fn pet_set_death_state_hunter_corpse_clears_lootable_and_skinnable_like_cpp() {
        let mut pet = Pet::new(owner_guid(), PetType::Hunter);
        pet.creature_mut()
            .unit_mut()
            .world_mut()
            .object_mut()
            .create(pet_guid(1));
        pet.creature_mut()
            .unit_mut()
            .world_mut()
            .object_mut()
            .replace_all_dynamic_flags(0x44);
        pet.creature_mut()
            .unit_mut()
            .set_unit_flags_like_cpp(UnitFlags::SKINNABLE | UnitFlags::IN_COMBAT);

        let outcome = pet.set_death_state_like_cpp(DeathState::JustDied, 1_000);

        assert_eq!(pet.creature().unit().death_state(), DeathState::Corpse);
        assert!(outcome.cleared_hunter_corpse_flags);
        assert!(!outcome.cast_pet_auras_current);
        assert_eq!(pet.creature().unit().world().object().dynamic_flags(), 0);
        assert!(
            !pet.creature()
                .unit()
                .unit_flags_like_cpp()
                .contains(UnitFlags::SKINNABLE),
            "C++ Pet::setDeathState(CORPSE) removes UNIT_FLAG_SKINNABLE for hunter pets"
        );
        assert!(
            pet.creature()
                .unit()
                .unit_flags_like_cpp()
                .contains(UnitFlags::IN_COMBAT),
            "Pet-specific C++ branch removes SKINNABLE only; broader combat cleanup belongs to Creature/Unit"
        );
    }

    #[test]
    fn pet_set_death_state_non_hunter_corpse_keeps_pet_specific_flags_like_cpp() {
        let mut pet = Pet::new(owner_guid(), PetType::Summon);
        pet.creature_mut()
            .unit_mut()
            .world_mut()
            .object_mut()
            .create(pet_guid(2));
        pet.creature_mut()
            .unit_mut()
            .world_mut()
            .object_mut()
            .replace_all_dynamic_flags(0x44);
        pet.creature_mut()
            .unit_mut()
            .set_unit_flags_like_cpp(UnitFlags::SKINNABLE);

        let outcome = pet.set_death_state_like_cpp(DeathState::JustDied, 1_000);

        assert_eq!(pet.creature().unit().death_state(), DeathState::Corpse);
        assert!(!outcome.cleared_hunter_corpse_flags);
        assert_eq!(pet.creature().unit().world().object().dynamic_flags(), 0x44);
        assert!(
            pet.creature()
                .unit()
                .unit_flags_like_cpp()
                .contains(UnitFlags::SKINNABLE)
        );
    }

    #[test]
    fn pet_set_death_state_alive_requests_current_pet_auras_like_cpp() {
        let mut pet = Pet::new(owner_guid(), PetType::Hunter);
        pet.creature_mut()
            .unit_mut()
            .world_mut()
            .object_mut()
            .create(pet_guid(3));

        let outcome = pet.set_death_state_like_cpp(DeathState::Alive, 1_000);

        assert_eq!(pet.creature().unit().death_state(), DeathState::Alive);
        assert!(!outcome.cleared_hunter_corpse_flags);
        assert!(
            outcome.cast_pet_auras_current,
            "C++ Pet::setDeathState(ALIVE) calls CastPetAuras(true)"
        );
    }

    #[test]
    fn pet_focus_regen_timer_preserves_cpp_overshoot_and_lag_reset() {
        let mut pet = Pet::new(owner_guid(), PetType::Hunter);

        assert!(pet.tick_focus_regen_timer(4_500));
        assert_eq!(pet.focus_regen_timer_ms(), 3_500);

        assert!(pet.tick_focus_regen_timer(7_500));
        assert_eq!(pet.focus_regen_timer_ms(), 1);

        assert!(pet.tick_focus_regen_timer(10_000));
        assert_eq!(pet.focus_regen_timer_ms(), PET_FOCUS_REGEN_INTERVAL_MS);
    }

    #[test]
    fn pet_regenerate_focus_matches_cpp_base_amount_and_clamps() {
        let mut pet = Pet::new(owner_guid(), PetType::Hunter);
        pet.creature_mut().set_power_type(PowerType::Focus);
        pet.creature_mut()
            .unit_mut()
            .set_max_power(PowerType::Focus, 100);
        pet.creature_mut()
            .unit_mut()
            .set_power(PowerType::Focus, 40);

        assert_eq!(pet.regenerate_focus_like_cpp(1.0, 1.0, 0, true), 24);
        assert_eq!(pet.creature().unit().get_power(PowerType::Focus), 64);

        assert_eq!(pet.regenerate_focus_like_cpp(2.0, 0.5, 5, true), 28);
        assert_eq!(pet.creature().unit().get_power(PowerType::Focus), 92);

        assert_eq!(pet.regenerate_focus_like_cpp(1.0, 1.0, 0, true), 8);
        assert_eq!(pet.creature().unit().get_power(PowerType::Focus), 100);
        assert_eq!(pet.regenerate_focus_like_cpp(1.0, 1.0, 0, true), 0);
    }

    #[test]
    fn pet_regenerate_focus_honors_cpp_power_guards() {
        let mut pet = Pet::new(owner_guid(), PetType::Hunter);

        assert_eq!(pet.regenerate_focus_like_cpp(1.0, 1.0, 0, true), 0);

        pet.creature_mut().set_power_type(PowerType::Focus);
        pet.creature_mut()
            .unit_mut()
            .set_max_power(PowerType::Focus, 100);
        pet.creature_mut()
            .unit_mut()
            .set_power(PowerType::Focus, 40);

        assert_eq!(pet.regenerate_focus_like_cpp(1.0, 1.0, 0, false), 0);
        assert_eq!(pet.creature().unit().get_power(PowerType::Focus), 40);
    }

    #[test]
    fn pet_give_level_updates_hunter_xp_and_levelup_hooks_like_cpp() {
        let mut pet = Pet::new(owner_guid(), PetType::Hunter);
        pet.creature_mut().unit_mut().set_level(10);
        pet.set_pet_experience(50);
        pet.set_pet_next_level_experience(500);

        let outcome = pet.give_pet_level_like_cpp(11, |level| u32::from(level) * 1_000);

        assert_eq!(
            outcome,
            PetLevelUpdateOutcome {
                changed: true,
                reset_experience: true,
                refresh_stats: true,
                init_levelup_spells: true
            }
        );
        assert_eq!(pet.creature().level(), 11);
        assert_eq!(pet.pet_experience(), 0);
        assert_eq!(pet.pet_next_level_experience(), 550);

        let unchanged = pet.give_pet_level_like_cpp(11, |_| 999);
        assert_eq!(
            unchanged,
            PetLevelUpdateOutcome {
                changed: false,
                reset_experience: false,
                refresh_stats: false,
                init_levelup_spells: false
            }
        );
    }

    #[test]
    fn pet_give_xp_matches_cpp_gates_and_level_rollover() {
        let mut pet = Pet::new(owner_guid(), PetType::Hunter);
        pet.creature_mut().unit_mut().set_level(10);
        pet.set_pet_experience(90);
        pet.set_pet_next_level_experience(100);

        let outcome = pet.give_pet_xp_like_cpp(700, 80, 12, |level| u32::from(level) * 1_000);

        assert_eq!(outcome.accepted, true);
        assert_eq!(outcome.levels_gained, 2);
        assert_eq!(
            outcome.level_update,
            PetLevelUpdateOutcome {
                changed: true,
                reset_experience: true,
                refresh_stats: true,
                init_levelup_spells: true
            }
        );
        assert_eq!(pet.creature().level(), 12);
        assert_eq!(
            pet.pet_experience(),
            0,
            "C++ clears pet XP when the pet reaches min(max-player-level, owner-level)"
        );
        assert_eq!(pet.pet_next_level_experience(), 600);

        let mut summon = Pet::new(owner_guid(), PetType::Summon);
        summon.creature_mut().unit_mut().set_level(10);
        assert_eq!(
            summon.give_pet_xp_like_cpp(1, 80, 80, |_| 100).accepted,
            false
        );

        let mut dead = Pet::new(owner_guid(), PetType::Hunter);
        dead.creature_mut().unit_mut().set_level(10);
        dead.creature_mut()
            .set_death_state_runtime(DeathState::JustDied, 1_000);
        assert_eq!(
            dead.give_pet_xp_like_cpp(1, 80, 80, |_| 100).accepted,
            false
        );
    }

    #[test]
    fn pet_give_xp_handles_zero_next_level_xp_like_cpp_initialized_field_assumption() {
        let mut pet = Pet::new(owner_guid(), PetType::Hunter);
        pet.creature_mut().unit_mut().set_level(10);
        pet.set_pet_experience(0);
        pet.set_pet_next_level_experience(0);

        let outcome = pet.give_pet_xp_like_cpp(600, 80, 12, |level| u32::from(level) * 1_000);

        assert_eq!(outcome.levels_gained, 2);
        assert_eq!(pet.creature().level(), 12);
        assert_eq!(pet.pet_experience(), 0);
    }

    #[test]
    fn pet_give_xp_uses_cpp_uint32_wrapping_sum() {
        let mut pet = Pet::new(owner_guid(), PetType::Hunter);
        pet.creature_mut().unit_mut().set_level(10);
        pet.set_pet_experience(u32::MAX);
        pet.set_pet_next_level_experience(100);

        let outcome = pet.give_pet_xp_like_cpp(2, 80, 80, |_| 1_000);

        assert!(outcome.accepted);
        assert_eq!(outcome.levels_gained, 0);
        assert_eq!(pet.creature().level(), 10);
        assert_eq!(pet.pet_experience(), 1);
    }

    #[test]
    fn pet_generate_action_bar_data_matches_cpp_type_action_format() {
        let mut action_bar = [0u32; 10];
        action_bar[0] = crate::make_unit_action_button_like_cpp(
            crate::COMMAND_ATTACK_LIKE_CPP,
            crate::ACT_COMMAND_LIKE_CPP,
        );
        action_bar[3] =
            crate::make_unit_action_button_like_cpp(12_345, crate::ACT_ENABLED_LIKE_CPP);
        action_bar[4] =
            crate::make_unit_action_button_like_cpp(23_456, crate::ACT_DISABLED_LIKE_CPP);
        action_bar[9] = crate::make_unit_action_button_like_cpp(
            crate::COMMAND_STAY_LIKE_CPP,
            crate::ACT_REACTION_LIKE_CPP,
        );

        let expected = action_bar
            .iter()
            .map(|packed| {
                format!(
                    "{} {} ",
                    unit_action_button_type_like_cpp(*packed),
                    unit_action_button_action_like_cpp(*packed)
                )
            })
            .collect::<String>();

        assert_eq!(
            Pet::generate_action_bar_data_like_cpp(&action_bar),
            expected
        );
        assert!(
            expected.ends_with(' '),
            "C++ ostream appends a trailing space after every action-bar entry"
        );
    }

    #[test]
    fn pet_fill_pet_info_matches_cpp_field_copy() {
        let mut pet = Pet::new(owner_guid(), PetType::Hunter);
        pet.creature_mut()
            .unit_mut()
            .world_mut()
            .object_mut()
            .create(pet_guid(21));
        pet.creature_mut()
            .unit_mut()
            .world_mut()
            .object_mut()
            .set_entry(500);
        pet.creature_mut().unit_mut().world_mut().set_name("Misha");
        pet.creature_mut().set_display_id(12_345, true, None);
        pet.creature_mut().unit_mut().set_level(43);
        pet.creature_mut().unit_mut().set_max_health(1_000);
        pet.creature_mut().unit_mut().set_health(888);
        pet.creature_mut().set_power_type(PowerType::Mana);
        pet.creature_mut()
            .unit_mut()
            .set_max_power(PowerType::Mana, 500);
        pet.creature_mut()
            .unit_mut()
            .set_power(PowerType::Mana, 222);
        pet.creature_mut().set_react_state(ReactState::Defensive);
        pet.set_pet_experience(777);
        pet.set_specialization(12);

        let mut action_bar = [0u32; 10];
        action_bar[0] = crate::make_unit_action_button_like_cpp(
            crate::COMMAND_ATTACK_LIKE_CPP,
            crate::ACT_COMMAND_LIKE_CPP,
        );
        action_bar[3] =
            crate::make_unit_action_button_like_cpp(11_111, crate::ACT_ENABLED_LIKE_CPP);

        let forced = pet.fill_pet_info_like_cpp(
            42,
            &action_bar,
            Some(ReactState::Passive),
            false,
            1_700_000_000,
            9_001,
        );

        assert_eq!(forced.name, "Misha");
        assert_eq!(
            forced.action_bar,
            Pet::generate_action_bar_data_like_cpp(&action_bar)
        );
        assert_eq!(forced.pet_number, 42);
        assert_eq!(forced.creature_id, 500);
        assert_eq!(forced.display_id, 12_345);
        assert_eq!(forced.experience, 777);
        assert_eq!(forced.health, 888);
        assert_eq!(forced.mana, 222);
        assert_eq!(forced.last_save_time, 1_700_000_000);
        assert_eq!(forced.created_by_spell_id, 9_001);
        assert_eq!(forced.specialization_id, 12);
        assert_eq!(forced.level, 43);
        assert_eq!(forced.react_state, ReactState::Passive);
        assert_eq!(forced.pet_type, PetType::Hunter);
        assert!(forced.was_renamed);

        let unforced = pet.fill_pet_info_like_cpp(43, &action_bar, None, true, 7, 8);
        assert_eq!(unforced.react_state, ReactState::Defensive);
        assert!(!unforced.was_renamed);
    }

    #[test]
    fn pet_spell_map_and_autospells_match_cpp_field_shape() {
        let mut pet = Pet::new(owner_guid(), PetType::Summon);

        assert!(pet.add_spell(
            123,
            ActiveState::Enabled,
            PetSpellState::New,
            PetSpellType::Normal
        ));
        assert!(pet.has_spell(123));
        assert_eq!(pet.get_pet_auto_spell_size(), 1);
        assert_eq!(pet.get_pet_auto_spell_on_pos(0), 123);

        assert!(pet.toggle_autocast(123, false));
        assert_eq!(pet.get_pet_auto_spell_size(), 0);
        assert_eq!(pet.get_pet_auto_spell_on_pos(1), 0);

        assert!(pet.remove_spell(123));
        assert!(!pet.has_spell(123));
    }

    #[test]
    fn pet_declined_names_store_all_cpp_cases() {
        let mut pet = Pet::new(owner_guid(), PetType::Hunter);
        let names = PetDeclinedNamesLikeCpp {
            names: ["Mishy", "Mishya", "Mishu", "Mishom", "Mishe"].map(str::to_string),
        };

        pet.set_declined_names(Some(names.clone()));

        assert_eq!(pet.declined_names(), Some(&names));
    }

    #[test]
    fn stable_lookup_matches_cpp_priority_order() {
        let stable = PetStable {
            current_pet_index: Some(1),
            active_pets: vec![Some(pet_info(10, 100)), Some(pet_info(20, 200))],
            stabled_pets: vec![Some(pet_info(30, 300))],
            unslotted_pets: vec![pet_info(40, 400)],
        };

        assert_eq!(
            Pet::get_load_pet_info(&stable, 0, 20, None),
            Some(PetLoadSelection {
                pet_number: 20,
                creature_id: 200,
                slot: 1,
            })
        );
        assert_eq!(
            Pet::get_load_pet_info(&stable, 0, 30, None).unwrap().slot,
            PetSaveMode::stable_slot(0)
        );
        assert_eq!(
            Pet::get_load_pet_info(&stable, 0, 0, Some(PetSaveMode::AsCurrent as i16)),
            Some(PetLoadSelection {
                pet_number: 20,
                creature_id: 200,
                slot: 1,
            })
        );
        assert_eq!(
            Pet::get_load_pet_info(&stable, 400, 0, None).unwrap().slot,
            PetSaveMode::NotInSlot as i16
        );
    }

    #[test]
    fn stable_lookup_preserves_cpp_deleted_save_mode() {
        let stable = PetStable {
            current_pet_index: Some(0),
            active_pets: vec![None],
            stabled_pets: vec![None],
            unslotted_pets: Vec::new(),
        };

        let result = Pet::get_load_pet_info_result_like_cpp(&stable, 0, 999, None);
        assert_eq!(result, PetLoadInfoResult::Deleted);
        assert_eq!(result.selection(), None);
        assert_eq!(result.save_mode(), PetSaveMode::AsDeleted as i16);
        assert_eq!(Pet::get_load_pet_info(&stable, 0, 999, None), None);

        let result = Pet::get_load_pet_info_result_like_cpp(
            &stable,
            0,
            0,
            Some(PetSaveMode::AsCurrent as i16),
        );
        assert_eq!(result, PetLoadInfoResult::Deleted);
        assert_eq!(result.save_mode(), PetSaveMode::AsDeleted as i16);
    }

    #[test]
    fn pet_save_slot_helpers_match_cpp_ranges() {
        assert!(PetSaveMode::is_active_slot(0));
        assert!(PetSaveMode::is_active_slot(4));
        assert!(!PetSaveMode::is_active_slot(5));
        assert!(PetSaveMode::is_stabled_slot(5));
        assert!(PetSaveMode::is_stabled_slot(204));
        assert!(!PetSaveMode::is_stabled_slot(205));
    }

    #[test]
    fn permanent_pet_and_xp_factor_match_cpp_shape() {
        let pet = Pet::new(owner_guid(), PetType::Hunter);
        assert!(pet.is_permanent_pet_for(owner_guid(), 1));
        assert!(!pet.is_permanent_pet_for(owner_guid(), 0));
        assert_eq!(Pet::pet_next_level_xp_for_owner_level(10_000), 500);
    }
}
