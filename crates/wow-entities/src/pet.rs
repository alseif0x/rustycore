use std::collections::BTreeMap;

use wow_constants::{Class, CreatureType, DeathState, PowerType, UnitFlags};
use wow_core::ObjectGuid;

use crate::{
    Creature, CreatureRuntimePlan, ReactState, UNIT_MASK_CONTROLABLE_GUARDIAN, UNIT_MASK_GUARDIAN,
    UNIT_MASK_HUNTER_PET, UNIT_MASK_MINION, UNIT_MASK_PET, UNIT_MASK_SUMMON,
    UnitAddToWorldOutcomeLikeCpp, UnitRemoveFromWorldOutcomeLikeCpp,
    unit_action_button_action_like_cpp, unit_action_button_type_like_cpp,
};

pub const HAPPINESS_LEVEL_SIZE: u32 = 333_000;
pub const MAX_ACTIVE_PETS: usize = 5;
pub const MAX_PET_STABLES: usize = 200;
pub const PET_FOCUS_REGEN_AMOUNT_LIKE_CPP: f32 = 24.0;
pub const PET_FOCUS_REGEN_INTERVAL_MS: u32 = 4_000;
pub const PET_XP_FACTOR: f32 = 0.05;
pub const GROUP_UPDATE_FLAG_PET_LIKE_CPP: u32 = 0x0001_0000;
pub const GROUP_UPDATE_FLAG_PET_NONE_LIKE_CPP: u32 = 0x0000_0000;
pub const GROUP_UPDATE_FLAG_PET_MODEL_ID_LIKE_CPP: u32 = 0x0000_0004;
pub const PET_MAX_SPECIALIZATIONS_LIKE_CPP: usize = 4;

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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PetFamilyScaleLikeCpp {
    pub min_scale: f32,
    pub min_scale_level: u8,
    pub max_scale: f32,
    pub max_scale_level: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PetSpecializationSpellLikeCpp {
    pub spell_id: u32,
    pub spell_exists: bool,
    pub spell_level: u8,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PetRemovePlanLikeCpp {
    pub owner_guid: ObjectGuid,
    pub pet_guid: ObjectGuid,
    pub save_mode: PetSaveMode,
    pub return_reagent: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PetAddToWorldOutcomeLikeCpp {
    pub guid: ObjectGuid,
    pub inserted_pet_lookup: bool,
    pub unit_add_to_world: Option<UnitAddToWorldOutcomeLikeCpp>,
    pub aim_initialize_represented: bool,
    pub zone_script_on_creature_create_represented: bool,
    pub follow_command_flags_reset: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PetRemoveFromWorldOutcomeLikeCpp {
    pub guid: ObjectGuid,
    pub unit_remove_from_world: Option<UnitRemoveFromWorldOutcomeLikeCpp>,
    pub removed_pet_lookup: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PetGroupUpdateOutcomeLikeCpp {
    pub group_update_mask: u32,
    pub owner_group_flag: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PetSetDisplayIdOutcomeLikeCpp {
    pub model_id: u32,
    pub set_native: bool,
    pub group_update: Option<PetGroupUpdateOutcomeLikeCpp>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PetSetSpecializationOutcomeLikeCpp {
    pub changed: bool,
    pub removed_specialization_spells: Vec<u32>,
    pub remove_learn_prev: bool,
    pub remove_clear_action_bar: bool,
    pub learned_specialization_spells: Vec<u32>,
    pub cleanup_action_bar: bool,
    pub pet_spell_initialize: bool,
    pub packet_spec_id: Option<u16>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PetSaveToDbSkipReason {
    ZeroEntry,
    NotControlled,
    OwnerNotPlayer,
    TemporaryUnsummonedHunterCurrent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PetSaveToDbPlan {
    pub pet_number: u32,
    pub effective_mode: i16,
    pub save_auras_before_cleanup: bool,
    pub remove_all_auras_before_spell_save: bool,
    pub save_spells: bool,
    pub save_spell_history: bool,
    pub delete_existing_pet_row: bool,
    pub fill_pet_info: bool,
    pub insert_pet_row: bool,
    pub insert_slot: Option<i16>,
    pub remove_all_auras_before_delete: bool,
    pub delete_from_db_pet_number: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PetSaveToDbOperationLikeCpp {
    BeginAuraSpellHistoryTransaction,
    SaveAuras,
    RemoveAllAurasBeforeSpellSave,
    SaveSpells,
    SaveSpellHistory,
    CommitAuraSpellHistoryTransaction,
    BeginPetRowTransaction,
    DeleteCharacterPetById { pet_number: u32 },
    FillPetInfo,
    InsertPetRow { pet_number: u32, insert_slot: i16 },
    CommitPetRowTransaction,
    RemoveAllAurasBeforeDelete,
    DeleteFromDb { pet_number: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PetDeleteFromDbOperationLikeCpp {
    BeginTransaction,
    DeleteCharacterPetById { pet_number: u32 },
    DeleteCharacterPetDeclinedName { pet_number: u32 },
    DeletePetAuraEffects { pet_number: u32 },
    DeletePetAuras { pet_number: u32 },
    DeletePetSpells { pet_number: u32 },
    DeletePetSpellCooldowns { pet_number: u32 },
    DeletePetSpellCharges { pet_number: u32 },
    CommitTransaction,
}

impl PetSaveToDbPlan {
    pub fn operations_like_cpp(&self) -> Vec<PetSaveToDbOperationLikeCpp> {
        let mut operations = vec![PetSaveToDbOperationLikeCpp::BeginAuraSpellHistoryTransaction];

        if self.save_auras_before_cleanup {
            operations.push(PetSaveToDbOperationLikeCpp::SaveAuras);
        }
        if self.remove_all_auras_before_spell_save {
            operations.push(PetSaveToDbOperationLikeCpp::RemoveAllAurasBeforeSpellSave);
        }
        if self.save_spells {
            operations.push(PetSaveToDbOperationLikeCpp::SaveSpells);
        }
        if self.save_spell_history {
            operations.push(PetSaveToDbOperationLikeCpp::SaveSpellHistory);
        }

        operations.push(PetSaveToDbOperationLikeCpp::CommitAuraSpellHistoryTransaction);

        if self.insert_pet_row {
            operations.push(PetSaveToDbOperationLikeCpp::BeginPetRowTransaction);
            if self.delete_existing_pet_row {
                operations.push(PetSaveToDbOperationLikeCpp::DeleteCharacterPetById {
                    pet_number: self.pet_number,
                });
            }
            if self.fill_pet_info {
                operations.push(PetSaveToDbOperationLikeCpp::FillPetInfo);
            }
            if let Some(insert_slot) = self.insert_slot {
                operations.push(PetSaveToDbOperationLikeCpp::InsertPetRow {
                    pet_number: self.pet_number,
                    insert_slot,
                });
            }
            operations.push(PetSaveToDbOperationLikeCpp::CommitPetRowTransaction);
        } else {
            if self.remove_all_auras_before_delete {
                operations.push(PetSaveToDbOperationLikeCpp::RemoveAllAurasBeforeDelete);
            }
            if let Some(pet_number) = self.delete_from_db_pet_number {
                operations.push(PetSaveToDbOperationLikeCpp::DeleteFromDb { pet_number });
            }
        }

        operations
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PetSpellSaveOperationLikeCpp {
    DeleteBySpell {
        pet_number: u32,
        spell_id: u32,
    },
    Insert {
        pet_number: u32,
        spell_id: u32,
        active: ActiveState,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PetAuraSaveEffectLikeCpp {
    pub effect_index: u8,
    pub amount: i32,
    pub base_amount: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PetAuraSaveRefLikeCpp {
    pub caster_guid: ObjectGuid,
    pub spell_id: u32,
    pub effect_mask: u32,
    pub recalculate_mask: u32,
    pub difficulty: u8,
    pub stack_count: u8,
    pub max_duration_ms: i32,
    pub duration_ms: i32,
    pub charges: u8,
    pub can_be_saved: bool,
    pub is_pet_aura: bool,
    pub effects: Vec<PetAuraSaveEffectLikeCpp>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PetAuraSaveOperationLikeCpp {
    DeleteAuraEffects {
        pet_number: u32,
    },
    DeleteAuras {
        pet_number: u32,
    },
    InsertAura {
        pet_number: u32,
        caster_guid: ObjectGuid,
        spell_id: u32,
        effect_mask: u32,
        recalculate_mask: u32,
        difficulty: u8,
        stack_count: u8,
        max_duration_ms: i32,
        duration_ms: i32,
        charges: u8,
    },
    InsertAuraEffect {
        pet_number: u32,
        caster_guid: ObjectGuid,
        spell_id: u32,
        effect_mask: u32,
        effect_index: u8,
        amount: i32,
        base_amount: i32,
    },
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

    pub fn save_spells_plan_like_cpp(
        &mut self,
        pet_number: u32,
    ) -> Vec<PetSpellSaveOperationLikeCpp> {
        let spell_ids = self.spells.keys().copied().collect::<Vec<_>>();
        let mut operations = Vec::new();

        for spell_id in spell_ids {
            let Some(spell) = self.spells.get(&spell_id).copied() else {
                continue;
            };

            if spell.spell_type == PetSpellType::Family {
                continue;
            }

            match spell.state {
                PetSpellState::Removed => {
                    operations.push(PetSpellSaveOperationLikeCpp::DeleteBySpell {
                        pet_number,
                        spell_id,
                    });
                    self.spells.remove(&spell_id);
                }
                PetSpellState::Changed => {
                    operations.push(PetSpellSaveOperationLikeCpp::DeleteBySpell {
                        pet_number,
                        spell_id,
                    });
                    operations.push(PetSpellSaveOperationLikeCpp::Insert {
                        pet_number,
                        spell_id,
                        active: spell.active,
                    });
                    if let Some(saved_spell) = self.spells.get_mut(&spell_id) {
                        saved_spell.state = PetSpellState::Unchanged;
                    }
                }
                PetSpellState::New => {
                    operations.push(PetSpellSaveOperationLikeCpp::Insert {
                        pet_number,
                        spell_id,
                        active: spell.active,
                    });
                    if let Some(saved_spell) = self.spells.get_mut(&spell_id) {
                        saved_spell.state = PetSpellState::Unchanged;
                    }
                }
                PetSpellState::Unchanged => {}
            }
        }

        operations
    }

    pub fn save_auras_plan_like_cpp(
        pet_number: u32,
        pet_guid: ObjectGuid,
        auras: &[PetAuraSaveRefLikeCpp],
    ) -> Vec<PetAuraSaveOperationLikeCpp> {
        let mut operations = vec![
            PetAuraSaveOperationLikeCpp::DeleteAuraEffects { pet_number },
            PetAuraSaveOperationLikeCpp::DeleteAuras { pet_number },
        ];

        for aura in auras {
            if !aura.can_be_saved || aura.is_pet_aura {
                continue;
            }

            let caster_guid = if aura.caster_guid == pet_guid {
                ObjectGuid::EMPTY
            } else {
                aura.caster_guid
            };

            operations.push(PetAuraSaveOperationLikeCpp::InsertAura {
                pet_number,
                caster_guid,
                spell_id: aura.spell_id,
                effect_mask: aura.effect_mask,
                recalculate_mask: aura.recalculate_mask,
                difficulty: aura.difficulty,
                stack_count: aura.stack_count,
                max_duration_ms: aura.max_duration_ms,
                duration_ms: aura.duration_ms,
                charges: aura.charges,
            });

            for effect in &aura.effects {
                operations.push(PetAuraSaveOperationLikeCpp::InsertAuraEffect {
                    pet_number,
                    caster_guid,
                    spell_id: aura.spell_id,
                    effect_mask: aura.effect_mask,
                    effect_index: effect.effect_index,
                    amount: effect.amount,
                    base_amount: effect.base_amount,
                });
            }
        }

        operations
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

    pub fn prepare_save_pet_to_db_like_cpp(
        &self,
        mut mode: i16,
        pet_number: u32,
        temporary_unsummoned_pet_number: Option<u32>,
        current_active_pet_index: Option<u32>,
    ) -> Result<PetSaveToDbPlan, PetSaveToDbSkipReason> {
        if self.creature.entry() == 0 {
            return Err(PetSaveToDbSkipReason::ZeroEntry);
        }

        if !self.is_controlled() {
            return Err(PetSaveToDbSkipReason::NotControlled);
        }

        if !self.owner_guid.is_player() {
            return Err(PetSaveToDbSkipReason::OwnerNotPlayer);
        }

        if mode == PetSaveMode::AsCurrent as i16 {
            if temporary_unsummoned_pet_number.is_some_and(|number| number != pet_number) {
                if self.pet_type == PetType::Hunter {
                    return Err(PetSaveToDbSkipReason::TemporaryUnsummonedHunterCurrent);
                }
                mode = PetSaveMode::NotInSlot as i16;
            }
        }

        if mode == PetSaveMode::AsCurrent as i16 {
            if let Some(active_slot) = current_active_pet_index {
                mode = active_slot as i16;
            }
        }

        let delete_path = mode == PetSaveMode::AsDeleted as i16;
        let remove_all_auras_before_spell_save = !PetSaveMode::is_active_slot(mode);
        let insert_slot = if delete_path {
            None
        } else {
            Some(
                current_active_pet_index
                    .map(|slot| slot as i16)
                    .unwrap_or(PetSaveMode::NotInSlot as i16),
            )
        };

        Ok(PetSaveToDbPlan {
            pet_number,
            effective_mode: mode,
            save_auras_before_cleanup: true,
            remove_all_auras_before_spell_save,
            save_spells: true,
            save_spell_history: true,
            delete_existing_pet_row: !delete_path,
            fill_pet_info: !delete_path,
            insert_pet_row: !delete_path,
            insert_slot,
            remove_all_auras_before_delete: delete_path,
            delete_from_db_pet_number: delete_path.then_some(pet_number),
        })
    }

    pub fn delete_from_db_plan_like_cpp(pet_number: u32) -> Vec<PetDeleteFromDbOperationLikeCpp> {
        vec![
            PetDeleteFromDbOperationLikeCpp::BeginTransaction,
            PetDeleteFromDbOperationLikeCpp::DeleteCharacterPetById { pet_number },
            PetDeleteFromDbOperationLikeCpp::DeleteCharacterPetDeclinedName { pet_number },
            PetDeleteFromDbOperationLikeCpp::DeletePetAuraEffects { pet_number },
            PetDeleteFromDbOperationLikeCpp::DeletePetAuras { pet_number },
            PetDeleteFromDbOperationLikeCpp::DeletePetSpells { pet_number },
            PetDeleteFromDbOperationLikeCpp::DeletePetSpellCooldowns { pet_number },
            PetDeleteFromDbOperationLikeCpp::DeletePetSpellCharges { pet_number },
            PetDeleteFromDbOperationLikeCpp::CommitTransaction,
        ]
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
    fn pet_add_to_world_uses_unit_path_and_resets_follow_flags_like_cpp() {
        let mut pet = Pet::new(owner_guid(), PetType::Hunter);
        pet.creature_mut()
            .unit_mut()
            .world_mut()
            .object_mut()
            .create(pet_guid(2));
        let charm_info = pet
            .creature_mut()
            .unit_mut()
            .subsystems_mut()
            .control
            .init_charm_info();
        charm_info.command_state = crate::COMMAND_FOLLOW_LIKE_CPP as u8;
        charm_info.is_command_attack = true;
        charm_info.is_command_follow = true;
        charm_info.is_at_stay = true;
        charm_info.is_following = true;
        charm_info.is_returning = true;

        let first = pet.add_to_world_like_cpp();
        assert_eq!(first.guid, pet_guid(2));
        assert!(first.inserted_pet_lookup);
        assert!(first.unit_add_to_world.is_some());
        assert!(first.aim_initialize_represented);
        assert!(first.zone_script_on_creature_create_represented);
        assert!(first.follow_command_flags_reset);
        assert!(pet.creature().unit().world().object().is_in_world());
        let charm_info = pet
            .creature()
            .unit()
            .subsystems()
            .control
            .charm_info
            .as_ref()
            .unwrap();
        assert_eq!(
            charm_info.command_state,
            crate::COMMAND_FOLLOW_LIKE_CPP as u8,
            "C++ clears transient follow flags but does not change CommandState"
        );
        assert!(!charm_info.is_command_attack);
        assert!(!charm_info.is_command_follow);
        assert!(!charm_info.is_at_stay);
        assert!(!charm_info.is_following);
        assert!(!charm_info.is_returning);

        let charm_info = pet
            .creature_mut()
            .unit_mut()
            .subsystems_mut()
            .control
            .charm_info
            .as_mut()
            .unwrap();
        charm_info.is_command_follow = true;
        charm_info.is_following = true;
        let second = pet.add_to_world_like_cpp();
        assert!(!second.inserted_pet_lookup);
        assert!(second.unit_add_to_world.is_none());
        assert!(second.follow_command_flags_reset);
        assert!(
            !pet.creature()
                .unit()
                .subsystems()
                .control
                .charm_info
                .as_ref()
                .unwrap()
                .is_following,
            "C++ follow cleanup is outside the IsInWorld guard"
        );
    }

    #[test]
    fn pet_remove_from_world_uses_unit_path_and_pet_lookup_like_cpp() {
        let mut pet = Pet::new(owner_guid(), PetType::Hunter);
        pet.creature_mut()
            .unit_mut()
            .world_mut()
            .object_mut()
            .create(pet_guid(3));
        assert!(pet.remove_from_world_like_cpp().is_none());

        let add = pet.add_to_world_like_cpp();
        assert!(add.unit_add_to_world.is_some());
        let remove = pet.remove_from_world_like_cpp().unwrap();
        assert_eq!(remove.guid, pet_guid(3));
        assert!(remove.unit_remove_from_world.is_some());
        assert!(remove.removed_pet_lookup);
        assert!(!pet.creature().unit().world().object().is_in_world());
        assert!(pet.remove_from_world_like_cpp().is_none());
    }

    #[test]
    fn pet_debug_info_appends_pet_type_and_number_like_cpp() {
        let mut hunter = Pet::new(owner_guid(), PetType::Hunter);
        hunter
            .creature_mut()
            .unit_mut()
            .subsystems_mut()
            .control
            .init_charm_info()
            .pet_number = 42;
        assert_eq!(
            hunter.debug_info_with_guardian_like_cpp("GuardianDebug"),
            Some("GuardianDebug\nPetType: 1 PetNumber: 42".to_string())
        );

        let mut summon = Pet::new(owner_guid(), PetType::Summon);
        summon
            .creature_mut()
            .unit_mut()
            .subsystems_mut()
            .control
            .init_charm_info()
            .pet_number = 7;
        assert_eq!(
            summon.debug_info_with_guardian_like_cpp("G"),
            Some("G\nPetType: 0 PetNumber: 7".to_string()),
            "C++ uses std::to_string(getPetType()), so the enum is numeric"
        );
    }

    #[test]
    fn pet_debug_info_requires_charm_info_like_cpp_assumption() {
        let pet = Pet::new(owner_guid(), PetType::Hunter);
        assert_eq!(pet.debug_info_with_guardian_like_cpp("GuardianDebug"), None);
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
    fn pet_group_update_flags_follow_cpp_owner_group_gate() {
        let mut pet = Pet::new(owner_guid(), PetType::Hunter);

        assert_eq!(
            pet.set_group_update_flag_like_cpp(GROUP_UPDATE_FLAG_PET_MODEL_ID_LIKE_CPP, false),
            None,
            "C++ Pet::SetGroupUpdateFlag mutates only when owner has a group"
        );
        assert_eq!(pet.group_update_mask(), 0);

        assert_eq!(
            pet.set_group_update_flag_like_cpp(GROUP_UPDATE_FLAG_PET_MODEL_ID_LIKE_CPP, true),
            Some(PetGroupUpdateOutcomeLikeCpp {
                group_update_mask: GROUP_UPDATE_FLAG_PET_MODEL_ID_LIKE_CPP,
                owner_group_flag: Some(GROUP_UPDATE_FLAG_PET_LIKE_CPP),
            })
        );
        assert_eq!(
            pet.group_update_mask(),
            GROUP_UPDATE_FLAG_PET_MODEL_ID_LIKE_CPP
        );

        assert_eq!(
            pet.reset_group_update_flag_like_cpp(false),
            PetGroupUpdateOutcomeLikeCpp {
                group_update_mask: GROUP_UPDATE_FLAG_PET_NONE_LIKE_CPP,
                owner_group_flag: None,
            }
        );
        assert_eq!(pet.group_update_mask(), 0);

        pet.set_group_update_flag_like_cpp(GROUP_UPDATE_FLAG_PET_MODEL_ID_LIKE_CPP, true);
        assert_eq!(
            pet.reset_group_update_flag_like_cpp(true),
            PetGroupUpdateOutcomeLikeCpp {
                group_update_mask: GROUP_UPDATE_FLAG_PET_NONE_LIKE_CPP,
                owner_group_flag: Some(GROUP_UPDATE_FLAG_PET_LIKE_CPP),
            }
        );
    }

    #[test]
    fn pet_set_display_id_marks_pet_model_group_update_only_for_controlled_pets_like_cpp() {
        let mut hunter = Pet::new(owner_guid(), PetType::Hunter);
        let outcome = hunter.set_display_id_like_cpp(12_345, true, true);
        assert_eq!(
            outcome,
            PetSetDisplayIdOutcomeLikeCpp {
                model_id: 12_345,
                set_native: true,
                group_update: Some(PetGroupUpdateOutcomeLikeCpp {
                    group_update_mask: GROUP_UPDATE_FLAG_PET_MODEL_ID_LIKE_CPP,
                    owner_group_flag: Some(GROUP_UPDATE_FLAG_PET_LIKE_CPP),
                }),
            }
        );
        assert_eq!(hunter.creature().unit().data().display_id, 12_345);
        assert_eq!(hunter.creature().unit().data().native_display_id, 12_345);

        let mut uncontrolled = Pet::new(owner_guid(), PetType::Max);
        let outcome = uncontrolled.set_display_id_like_cpp(22_222, false, true);
        assert_eq!(
            outcome,
            PetSetDisplayIdOutcomeLikeCpp {
                model_id: 22_222,
                set_native: false,
                group_update: None,
            },
            "C++ Pet::SetDisplayId returns before SetGroupUpdateFlag when !isControlled()"
        );
        assert_eq!(uncontrolled.group_update_mask(), 0);
        assert_eq!(uncontrolled.creature().unit().data().display_id, 22_222);
        assert_eq!(uncontrolled.creature().unit().data().native_display_id, 0);
    }

    #[test]
    fn pet_have_in_diet_matches_cpp_food_type_and_family_mask() {
        assert!(
            !Pet::have_in_diet_like_cpp(0, true, Some(0xFFFF)),
            "C++ returns false before template lookup when item->FoodType is zero"
        );
        assert!(
            !Pet::have_in_diet_like_cpp(1, false, Some(0xFFFF)),
            "C++ returns false when GetCreatureTemplate() is missing"
        );
        assert!(
            !Pet::have_in_diet_like_cpp(1, true, None),
            "C++ returns false when CreatureFamily lookup is missing"
        );
        assert!(Pet::have_in_diet_like_cpp(1, true, Some(0b0001)));
        assert!(Pet::have_in_diet_like_cpp(4, true, Some(0b1000)));
        assert!(!Pet::have_in_diet_like_cpp(4, true, Some(0b0100)));
        assert!(
            !Pet::have_in_diet_like_cpp(33, true, Some(u32::MAX)),
            "C++ food masks are u32; out-of-range represented input fails closed"
        );
    }

    #[test]
    fn pet_native_object_scale_matches_cpp_hunter_family_scale() {
        let family = PetFamilyScaleLikeCpp {
            min_scale: 0.75,
            min_scale_level: 10,
            max_scale: 1.25,
            max_scale_level: 50,
        };

        assert_eq!(
            Pet::native_object_scale_like_cpp(PetType::Summon, 40, 1.5, Some(family)),
            1.5,
            "C++ applies CreatureFamily scaling only to hunter pets"
        );
        assert_eq!(
            Pet::native_object_scale_like_cpp(PetType::Hunter, 40, 1.5, None),
            1.5,
            "C++ falls back to Guardian::GetNativeObjectScale when CreatureFamily lookup is missing"
        );
        assert_eq!(
            Pet::native_object_scale_like_cpp(
                PetType::Hunter,
                40,
                1.5,
                Some(PetFamilyScaleLikeCpp {
                    min_scale: 0.0,
                    ..family
                })
            ),
            1.5,
            "C++ requires CreatureFamilyEntry::MinScale > 0.0"
        );

        assert_eq!(
            Pet::native_object_scale_like_cpp(PetType::Hunter, 8, 1.5, Some(family)),
            family.min_scale
        );
        assert_eq!(
            Pet::native_object_scale_like_cpp(PetType::Hunter, 50, 1.5, Some(family)),
            family.max_scale
        );
        assert_eq!(
            Pet::native_object_scale_like_cpp(PetType::Hunter, 30, 1.5, Some(family)),
            0.95,
            "C++ divides the middle interpolation by MaxScaleLevel, not by the min-max span"
        );
    }

    #[test]
    fn pet_is_permanent_pet_for_matches_cpp_owner_class_and_creature_type() {
        assert!(Pet::is_permanent_pet_for_like_cpp(
            PetType::Hunter,
            Class::Warrior,
            CreatureType::Critter
        ));
        assert!(Pet::is_permanent_pet_for_like_cpp(
            PetType::Summon,
            Class::Warlock,
            CreatureType::Demon
        ));
        assert!(Pet::is_permanent_pet_for_like_cpp(
            PetType::Summon,
            Class::DeathKnight,
            CreatureType::Undead
        ));
        assert!(Pet::is_permanent_pet_for_like_cpp(
            PetType::Summon,
            Class::Mage,
            CreatureType::Elemental
        ));

        assert!(
            !Pet::is_permanent_pet_for_like_cpp(
                PetType::Summon,
                Class::Warlock,
                CreatureType::Elemental
            ),
            "C++ requires warlock summon pets to use CREATURE_TYPE_DEMON"
        );
        assert!(
            !Pet::is_permanent_pet_for_like_cpp(
                PetType::Summon,
                Class::DeathKnight,
                CreatureType::Demon
            ),
            "C++ requires death knight summon pets to use CREATURE_TYPE_UNDEAD"
        );
        assert!(
            !Pet::is_permanent_pet_for_like_cpp(PetType::Summon, Class::Mage, CreatureType::Demon),
            "C++ requires mage summon pets to use CREATURE_TYPE_ELEMENTAL"
        );
        assert!(!Pet::is_permanent_pet_for_like_cpp(
            PetType::Summon,
            Class::Shaman,
            CreatureType::Elemental
        ));
        assert!(!Pet::is_permanent_pet_for_like_cpp(
            PetType::Max,
            Class::Hunter,
            CreatureType::Beast
        ));
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
    fn pet_remove_plan_delegates_to_owner_remove_pet_like_cpp() {
        let mut pet = Pet::new(owner_guid(), PetType::Summon);
        pet.creature_mut()
            .unit_mut()
            .world_mut()
            .object_mut()
            .create(pet_guid(16));

        assert_eq!(
            pet.remove_plan_like_cpp(PetSaveMode::NotInSlot, true),
            PetRemovePlanLikeCpp {
                owner_guid: owner_guid(),
                pet_guid: pet_guid(16),
                save_mode: PetSaveMode::NotInSlot,
                return_reagent: true,
            },
            "C++ Pet::Remove(mode, returnreagent) delegates GetOwner()->RemovePet(this, mode, returnreagent)"
        );
        assert_eq!(
            pet.remove_plan_like_cpp(PetSaveMode::AsDeleted, false),
            PetRemovePlanLikeCpp {
                owner_guid: owner_guid(),
                pet_guid: pet_guid(16),
                save_mode: PetSaveMode::AsDeleted,
                return_reagent: false,
            }
        );
    }

    #[test]
    fn pet_update_remove_outcomes_convert_to_remove_plan_like_cpp() {
        let mut lost_owner = Pet::new(owner_guid(), PetType::Hunter);
        lost_owner
            .creature_mut()
            .unit_mut()
            .world_mut()
            .object_mut()
            .create(pet_guid(17));
        let PetAliveOwnerUpdateOutcome::RemoveLostOwner {
            save_mode,
            return_reagent,
        } = lost_owner.update_alive_owner_link_like_cpp(false, false, Some(pet_guid(17)))
        else {
            panic!("expected C++ lost-owner branch to request Pet::Remove");
        };
        assert_eq!(
            lost_owner.remove_plan_like_cpp(save_mode, return_reagent),
            PetRemovePlanLikeCpp {
                owner_guid: owner_guid(),
                pet_guid: pet_guid(17),
                save_mode: PetSaveMode::NotInSlot,
                return_reagent: true,
            }
        );

        let mut expired = Pet::new(owner_guid(), PetType::Summon);
        expired
            .creature_mut()
            .unit_mut()
            .world_mut()
            .object_mut()
            .create(pet_guid(18));
        expired.set_duration(1);
        let PetDurationUpdateOutcome::Expired { save_mode } = expired.update_duration_like_cpp(1)
        else {
            panic!("expected C++ duration branch to request Pet::Remove");
        };
        assert_eq!(
            expired.remove_plan_like_cpp(save_mode, false),
            PetRemovePlanLikeCpp {
                owner_guid: owner_guid(),
                pet_guid: pet_guid(18),
                save_mode: PetSaveMode::NotInSlot,
                return_reagent: false,
            }
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
    fn pet_synchronize_level_with_owner_delegates_to_give_pet_level_like_cpp() {
        let mut hunter = Pet::new(owner_guid(), PetType::Hunter);
        hunter.creature_mut().unit_mut().set_level(10);
        hunter.set_pet_experience(77);
        hunter.set_pet_next_level_experience(100);

        assert_eq!(
            hunter.synchronize_level_with_owner_like_cpp(12, |level| u32::from(level) * 1_000),
            Some(PetLevelUpdateOutcome {
                changed: true,
                reset_experience: true,
                refresh_stats: true,
                init_levelup_spells: true,
            }),
            "C++ Pet::SynchronizeLevelWithOwner calls GivePetLevel for HUNTER_PET"
        );
        assert_eq!(hunter.creature().level(), 12);
        assert_eq!(hunter.pet_experience(), 0);
        assert_eq!(hunter.pet_next_level_experience(), 600);

        let mut summon = Pet::new(owner_guid(), PetType::Summon);
        summon.creature_mut().unit_mut().set_level(4);
        assert_eq!(
            summon.synchronize_level_with_owner_like_cpp(8, |_| 999),
            Some(PetLevelUpdateOutcome {
                changed: true,
                reset_experience: false,
                refresh_stats: true,
                init_levelup_spells: true,
            }),
            "C++ also synchronizes SUMMON_PET, but GivePetLevel does not reset hunter XP fields"
        );
        assert_eq!(summon.creature().level(), 8);

        let mut max = Pet::new(owner_guid(), PetType::Max);
        max.creature_mut().unit_mut().set_level(3);
        assert_eq!(max.synchronize_level_with_owner_like_cpp(9, |_| 999), None);
        assert_eq!(
            max.creature().level(),
            3,
            "C++ default branch leaves non summon/hunter pet types unchanged"
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
    fn pet_specialization_spell_plans_match_cpp_filters_and_order() {
        let learned = [
            PetSpecializationSpellLikeCpp {
                spell_id: 101,
                spell_exists: true,
                spell_level: 9,
            },
            PetSpecializationSpellLikeCpp {
                spell_id: 102,
                spell_exists: false,
                spell_level: 1,
            },
            PetSpecializationSpellLikeCpp {
                spell_id: 103,
                spell_exists: true,
                spell_level: 12,
            },
        ];
        assert_eq!(
            Pet::learn_specialization_spells_plan_like_cpp(10, &learned),
            vec![101],
            "C++ skips missing SpellInfo and spells above pet level"
        );

        let normal_specs: [&[u32]; PET_MAX_SPECIALIZATIONS_LIKE_CPP] =
            [&[11, 12], &[], &[31], &[41, 42]];
        let override_specs: [&[u32]; PET_MAX_SPECIALIZATIONS_LIKE_CPP] =
            [&[111], &[221, 222], &[], &[441]];
        assert_eq!(
            Pet::remove_specialization_spells_plan_like_cpp(&normal_specs, &override_specs),
            vec![11, 12, 111, 221, 222, 31, 41, 42, 441],
            "C++ loops index 0..MAX_SPECIALIZATIONS and appends normal then override spec spells"
        );
    }

    #[test]
    fn pet_set_specialization_like_cpp_preserves_cpp_side_effect_order() {
        let normal_specs: [&[u32]; PET_MAX_SPECIALIZATIONS_LIKE_CPP] =
            [&[11], &[21, 22], &[], &[41]];
        let override_specs: [&[u32]; PET_MAX_SPECIALIZATIONS_LIKE_CPP] =
            [&[111], &[], &[331], &[441, 442]];
        let learned = [
            PetSpecializationSpellLikeCpp {
                spell_id: 501,
                spell_exists: true,
                spell_level: 8,
            },
            PetSpecializationSpellLikeCpp {
                spell_id: 502,
                spell_exists: true,
                spell_level: 20,
            },
        ];

        let mut unchanged = Pet::new(owner_guid(), PetType::Hunter);
        unchanged.set_specialization(7);
        assert_eq!(
            unchanged.set_specialization_like_cpp(
                7,
                true,
                &learned,
                &normal_specs,
                &override_specs
            ),
            PetSetSpecializationOutcomeLikeCpp {
                changed: false,
                removed_specialization_spells: Vec::new(),
                remove_learn_prev: false,
                remove_clear_action_bar: false,
                learned_specialization_spells: Vec::new(),
                cleanup_action_bar: false,
                pet_spell_initialize: false,
                packet_spec_id: None,
            },
            "C++ returns before removing old specialization spells when spec is unchanged"
        );

        let mut invalid = Pet::new(owner_guid(), PetType::Hunter);
        invalid.set_specialization(3);
        invalid.creature_mut().unit_mut().set_level(10);
        assert_eq!(
            invalid.set_specialization_like_cpp(
                999,
                false,
                &learned,
                &normal_specs,
                &override_specs
            ),
            PetSetSpecializationOutcomeLikeCpp {
                changed: true,
                removed_specialization_spells: vec![11, 111, 21, 22, 331, 41, 441, 442],
                remove_learn_prev: true,
                remove_clear_action_bar: false,
                learned_specialization_spells: Vec::new(),
                cleanup_action_bar: false,
                pet_spell_initialize: false,
                packet_spec_id: None,
            },
            "C++ removes old spec spells before LookupEntry(spec), then sets specialization to 0 and returns"
        );
        assert_eq!(invalid.specialization(), 0);

        let mut valid = Pet::new(owner_guid(), PetType::Hunter);
        valid.set_specialization(3);
        valid.creature_mut().unit_mut().set_level(10);
        assert_eq!(
            valid.set_specialization_like_cpp(7, true, &learned, &normal_specs, &override_specs),
            PetSetSpecializationOutcomeLikeCpp {
                changed: true,
                removed_specialization_spells: vec![11, 111, 21, 22, 331, 41, 441, 442],
                remove_learn_prev: true,
                remove_clear_action_bar: false,
                learned_specialization_spells: vec![501],
                cleanup_action_bar: true,
                pet_spell_initialize: true,
                packet_spec_id: Some(7),
            }
        );
        assert_eq!(valid.specialization(), 7);
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
    fn pet_prepare_save_to_db_matches_cpp_initial_guards() {
        let pet = Pet::new(owner_guid(), PetType::Hunter);
        assert_eq!(
            pet.prepare_save_pet_to_db_like_cpp(PetSaveMode::AsCurrent as i16, 42, None, Some(0)),
            Err(PetSaveToDbSkipReason::ZeroEntry)
        );

        let mut uncontrolled = Pet::new(owner_guid(), PetType::Max);
        uncontrolled
            .creature_mut()
            .unit_mut()
            .world_mut()
            .object_mut()
            .set_entry(500);
        assert_eq!(
            uncontrolled.prepare_save_pet_to_db_like_cpp(
                PetSaveMode::AsCurrent as i16,
                42,
                None,
                Some(0)
            ),
            Err(PetSaveToDbSkipReason::NotControlled)
        );

        let mut non_player_owner = Pet::new(pet_guid(99), PetType::Hunter);
        non_player_owner
            .creature_mut()
            .unit_mut()
            .world_mut()
            .object_mut()
            .set_entry(500);
        assert_eq!(
            non_player_owner.prepare_save_pet_to_db_like_cpp(
                PetSaveMode::AsCurrent as i16,
                42,
                None,
                Some(0)
            ),
            Err(PetSaveToDbSkipReason::OwnerNotPlayer)
        );
    }

    #[test]
    fn pet_prepare_save_to_db_remaps_current_and_preserves_cpp_insert_slot_quirk() {
        let mut pet = Pet::new(owner_guid(), PetType::Hunter);
        pet.creature_mut()
            .unit_mut()
            .world_mut()
            .object_mut()
            .set_entry(500);

        let current = pet
            .prepare_save_pet_to_db_like_cpp(PetSaveMode::AsCurrent as i16, 42, None, Some(2))
            .unwrap();

        assert_eq!(current.pet_number, 42);
        assert_eq!(current.effective_mode, PetSaveMode::active_slot(2));
        assert!(current.save_auras_before_cleanup);
        assert!(!current.remove_all_auras_before_spell_save);
        assert!(current.save_spells);
        assert!(current.save_spell_history);
        assert!(current.delete_existing_pet_row);
        assert!(current.fill_pet_info);
        assert!(current.insert_pet_row);
        assert_eq!(current.insert_slot, Some(PetSaveMode::active_slot(2)));
        assert!(!current.remove_all_auras_before_delete);
        assert_eq!(current.delete_from_db_pet_number, None);

        let stable_slot = pet
            .prepare_save_pet_to_db_like_cpp(PetSaveMode::stable_slot(3), 42, None, Some(1))
            .unwrap();
        assert_eq!(stable_slot.effective_mode, PetSaveMode::stable_slot(3));
        assert!(
            stable_slot.remove_all_auras_before_spell_save,
            "C++ removes all auras before saving spells for stable/not-in-slot saves"
        );
        assert_eq!(
            stable_slot.insert_slot,
            Some(PetSaveMode::active_slot(1)),
            "C++ CHAR_INS_PET stores current active slot, not the already-computed mode"
        );

        let without_current_slot = pet
            .prepare_save_pet_to_db_like_cpp(PetSaveMode::stable_slot(4), 42, None, None)
            .unwrap();
        assert_eq!(
            without_current_slot.insert_slot,
            Some(PetSaveMode::NotInSlot as i16)
        );
    }

    #[test]
    fn pet_prepare_save_to_db_handles_temporary_unsummoned_current_like_cpp() {
        let mut hunter = Pet::new(owner_guid(), PetType::Hunter);
        hunter
            .creature_mut()
            .unit_mut()
            .world_mut()
            .object_mut()
            .set_entry(500);
        assert_eq!(
            hunter.prepare_save_pet_to_db_like_cpp(
                PetSaveMode::AsCurrent as i16,
                42,
                Some(7),
                Some(0)
            ),
            Err(PetSaveToDbSkipReason::TemporaryUnsummonedHunterCurrent)
        );

        let mut summon = Pet::new(owner_guid(), PetType::Summon);
        summon
            .creature_mut()
            .unit_mut()
            .world_mut()
            .object_mut()
            .set_entry(600);
        let plan = summon
            .prepare_save_pet_to_db_like_cpp(PetSaveMode::AsCurrent as i16, 42, Some(7), Some(0))
            .unwrap();
        assert_eq!(plan.effective_mode, PetSaveMode::NotInSlot as i16);
        assert!(plan.remove_all_auras_before_spell_save);
        assert!(plan.insert_pet_row);
        assert_eq!(plan.delete_from_db_pet_number, None);
    }

    #[test]
    fn pet_prepare_save_to_db_delete_path_matches_cpp_delete_from_db_shape() {
        let mut pet = Pet::new(owner_guid(), PetType::Hunter);
        pet.creature_mut()
            .unit_mut()
            .world_mut()
            .object_mut()
            .set_entry(500);

        let plan = pet
            .prepare_save_pet_to_db_like_cpp(PetSaveMode::AsDeleted as i16, 42, None, Some(3))
            .unwrap();

        assert_eq!(plan.effective_mode, PetSaveMode::AsDeleted as i16);
        assert!(plan.save_auras_before_cleanup);
        assert!(plan.remove_all_auras_before_spell_save);
        assert!(plan.save_spells);
        assert!(plan.save_spell_history);
        assert!(!plan.delete_existing_pet_row);
        assert!(!plan.fill_pet_info);
        assert!(!plan.insert_pet_row);
        assert_eq!(plan.insert_slot, None);
        assert!(plan.remove_all_auras_before_delete);
        assert_eq!(plan.delete_from_db_pet_number, Some(42));
    }

    #[test]
    fn pet_save_to_db_operations_preserve_cpp_transaction_order() {
        let mut pet = Pet::new(owner_guid(), PetType::Hunter);
        pet.creature_mut()
            .unit_mut()
            .world_mut()
            .object_mut()
            .set_entry(500);

        let current = pet
            .prepare_save_pet_to_db_like_cpp(PetSaveMode::AsCurrent as i16, 42, None, Some(2))
            .unwrap();
        assert_eq!(
            current.operations_like_cpp(),
            vec![
                PetSaveToDbOperationLikeCpp::BeginAuraSpellHistoryTransaction,
                PetSaveToDbOperationLikeCpp::SaveAuras,
                PetSaveToDbOperationLikeCpp::SaveSpells,
                PetSaveToDbOperationLikeCpp::SaveSpellHistory,
                PetSaveToDbOperationLikeCpp::CommitAuraSpellHistoryTransaction,
                PetSaveToDbOperationLikeCpp::BeginPetRowTransaction,
                PetSaveToDbOperationLikeCpp::DeleteCharacterPetById { pet_number: 42 },
                PetSaveToDbOperationLikeCpp::FillPetInfo,
                PetSaveToDbOperationLikeCpp::InsertPetRow {
                    pet_number: 42,
                    insert_slot: PetSaveMode::active_slot(2),
                },
                PetSaveToDbOperationLikeCpp::CommitPetRowTransaction,
            ]
        );

        let stable_slot = pet
            .prepare_save_pet_to_db_like_cpp(PetSaveMode::stable_slot(3), 42, None, Some(1))
            .unwrap();
        assert_eq!(
            stable_slot.operations_like_cpp(),
            vec![
                PetSaveToDbOperationLikeCpp::BeginAuraSpellHistoryTransaction,
                PetSaveToDbOperationLikeCpp::SaveAuras,
                PetSaveToDbOperationLikeCpp::RemoveAllAurasBeforeSpellSave,
                PetSaveToDbOperationLikeCpp::SaveSpells,
                PetSaveToDbOperationLikeCpp::SaveSpellHistory,
                PetSaveToDbOperationLikeCpp::CommitAuraSpellHistoryTransaction,
                PetSaveToDbOperationLikeCpp::BeginPetRowTransaction,
                PetSaveToDbOperationLikeCpp::DeleteCharacterPetById { pet_number: 42 },
                PetSaveToDbOperationLikeCpp::FillPetInfo,
                PetSaveToDbOperationLikeCpp::InsertPetRow {
                    pet_number: 42,
                    insert_slot: PetSaveMode::active_slot(1),
                },
                PetSaveToDbOperationLikeCpp::CommitPetRowTransaction,
            ]
        );

        let delete = pet
            .prepare_save_pet_to_db_like_cpp(PetSaveMode::AsDeleted as i16, 42, None, Some(1))
            .unwrap();
        assert_eq!(
            delete.operations_like_cpp(),
            vec![
                PetSaveToDbOperationLikeCpp::BeginAuraSpellHistoryTransaction,
                PetSaveToDbOperationLikeCpp::SaveAuras,
                PetSaveToDbOperationLikeCpp::RemoveAllAurasBeforeSpellSave,
                PetSaveToDbOperationLikeCpp::SaveSpells,
                PetSaveToDbOperationLikeCpp::SaveSpellHistory,
                PetSaveToDbOperationLikeCpp::CommitAuraSpellHistoryTransaction,
                PetSaveToDbOperationLikeCpp::RemoveAllAurasBeforeDelete,
                PetSaveToDbOperationLikeCpp::DeleteFromDb { pet_number: 42 },
            ]
        );
        assert_eq!(
            Pet::delete_from_db_plan_like_cpp(42),
            vec![
                PetDeleteFromDbOperationLikeCpp::BeginTransaction,
                PetDeleteFromDbOperationLikeCpp::DeleteCharacterPetById { pet_number: 42 },
                PetDeleteFromDbOperationLikeCpp::DeleteCharacterPetDeclinedName { pet_number: 42 },
                PetDeleteFromDbOperationLikeCpp::DeletePetAuraEffects { pet_number: 42 },
                PetDeleteFromDbOperationLikeCpp::DeletePetAuras { pet_number: 42 },
                PetDeleteFromDbOperationLikeCpp::DeletePetSpells { pet_number: 42 },
                PetDeleteFromDbOperationLikeCpp::DeletePetSpellCooldowns { pet_number: 42 },
                PetDeleteFromDbOperationLikeCpp::DeletePetSpellCharges { pet_number: 42 },
                PetDeleteFromDbOperationLikeCpp::CommitTransaction,
            ]
        );
    }

    #[test]
    fn pet_save_spells_plan_matches_cpp_state_machine_and_family_skip() {
        let mut pet = Pet::new(owner_guid(), PetType::Hunter);
        assert!(pet.add_spell(
            100,
            ActiveState::Enabled,
            PetSpellState::New,
            PetSpellType::Normal
        ));
        assert!(pet.add_spell(
            200,
            ActiveState::Disabled,
            PetSpellState::Changed,
            PetSpellType::Normal
        ));
        assert!(pet.add_spell(
            300,
            ActiveState::Passive,
            PetSpellState::Unchanged,
            PetSpellType::Normal
        ));
        assert!(pet.add_spell(
            400,
            ActiveState::Enabled,
            PetSpellState::New,
            PetSpellType::Family
        ));
        assert!(pet.add_spell(
            500,
            ActiveState::Enabled,
            PetSpellState::New,
            PetSpellType::Normal
        ));
        assert!(pet.remove_spell(500));

        let operations = pet.save_spells_plan_like_cpp(42);

        assert_eq!(
            operations,
            vec![
                PetSpellSaveOperationLikeCpp::Insert {
                    pet_number: 42,
                    spell_id: 100,
                    active: ActiveState::Enabled
                },
                PetSpellSaveOperationLikeCpp::DeleteBySpell {
                    pet_number: 42,
                    spell_id: 200
                },
                PetSpellSaveOperationLikeCpp::Insert {
                    pet_number: 42,
                    spell_id: 200,
                    active: ActiveState::Disabled
                },
                PetSpellSaveOperationLikeCpp::DeleteBySpell {
                    pet_number: 42,
                    spell_id: 500
                },
            ],
            "C++ iterates the spell map in key order and appends delete/insert statements per state"
        );
        assert_eq!(
            pet.spells().get(&100).unwrap().state,
            PetSpellState::Unchanged
        );
        assert_eq!(
            pet.spells().get(&200).unwrap().state,
            PetSpellState::Unchanged
        );
        assert_eq!(
            pet.spells().get(&300).unwrap().state,
            PetSpellState::Unchanged
        );
        assert_eq!(
            pet.spells().get(&400).unwrap().state,
            PetSpellState::New,
            "C++ skips PETSPELL_FAMILY before handling state, so even NEW family passives stay dirty"
        );
        assert!(!pet.spells().contains_key(&500));
    }

    #[test]
    fn pet_save_auras_plan_matches_cpp_delete_filter_and_insert_order() {
        let pet_guid = pet_guid(42);
        let other_caster = owner_guid();
        let saved_self_cast = PetAuraSaveRefLikeCpp {
            caster_guid: pet_guid,
            spell_id: 7_001,
            effect_mask: 0x3,
            recalculate_mask: 0x2,
            difficulty: 1,
            stack_count: 2,
            max_duration_ms: 30_000,
            duration_ms: 12_000,
            charges: 3,
            can_be_saved: true,
            is_pet_aura: false,
            effects: vec![
                PetAuraSaveEffectLikeCpp {
                    effect_index: 0,
                    amount: 11,
                    base_amount: 10,
                },
                PetAuraSaveEffectLikeCpp {
                    effect_index: 1,
                    amount: 22,
                    base_amount: 20,
                },
            ],
        };
        let saved_external_cast = PetAuraSaveRefLikeCpp {
            caster_guid: other_caster,
            spell_id: 7_002,
            effect_mask: 0x4,
            recalculate_mask: 0,
            difficulty: 0,
            stack_count: 1,
            max_duration_ms: -1,
            duration_ms: -1,
            charges: 0,
            can_be_saved: true,
            is_pet_aura: false,
            effects: vec![],
        };
        let not_saveable = PetAuraSaveRefLikeCpp {
            spell_id: 7_003,
            can_be_saved: false,
            ..saved_external_cast.clone()
        };
        let pet_aura = PetAuraSaveRefLikeCpp {
            spell_id: 7_004,
            can_be_saved: true,
            is_pet_aura: true,
            ..saved_external_cast.clone()
        };

        let operations = Pet::save_auras_plan_like_cpp(
            77,
            pet_guid,
            &[saved_self_cast, saved_external_cast, not_saveable, pet_aura],
        );

        assert_eq!(
            operations,
            vec![
                PetAuraSaveOperationLikeCpp::DeleteAuraEffects { pet_number: 77 },
                PetAuraSaveOperationLikeCpp::DeleteAuras { pet_number: 77 },
                PetAuraSaveOperationLikeCpp::InsertAura {
                    pet_number: 77,
                    caster_guid: ObjectGuid::EMPTY,
                    spell_id: 7_001,
                    effect_mask: 0x3,
                    recalculate_mask: 0x2,
                    difficulty: 1,
                    stack_count: 2,
                    max_duration_ms: 30_000,
                    duration_ms: 12_000,
                    charges: 3,
                },
                PetAuraSaveOperationLikeCpp::InsertAuraEffect {
                    pet_number: 77,
                    caster_guid: ObjectGuid::EMPTY,
                    spell_id: 7_001,
                    effect_mask: 0x3,
                    effect_index: 0,
                    amount: 11,
                    base_amount: 10,
                },
                PetAuraSaveOperationLikeCpp::InsertAuraEffect {
                    pet_number: 77,
                    caster_guid: ObjectGuid::EMPTY,
                    spell_id: 7_001,
                    effect_mask: 0x3,
                    effect_index: 1,
                    amount: 22,
                    base_amount: 20,
                },
                PetAuraSaveOperationLikeCpp::InsertAura {
                    pet_number: 77,
                    caster_guid: other_caster,
                    spell_id: 7_002,
                    effect_mask: 0x4,
                    recalculate_mask: 0,
                    difficulty: 0,
                    stack_count: 1,
                    max_duration_ms: -1,
                    duration_ms: -1,
                    charges: 0,
                },
            ],
            "C++ deletes existing pet aura rows first, clears self-caster GUIDs, then appends aura/effect inserts"
        );
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
