//! Pet lifecycle, aura and persistence state state definitions, part 1 of 1.
//!
//! Separated from the pet.rs root under #636. Behaviour is preserved.

use super::*;

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

pub const DEMONIC_KNOWLEDGE_AURA_LIKE_CPP: u32 = 35_696;

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
pub enum PetCleanupActionBarOperationLikeCpp {
    ClearSlot { index: usize },
    EnableAutocast { index: usize, spell_id: u32 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PetCleanupActionBarOutcomeLikeCpp {
    pub action_bar: [u32; MAX_UNIT_ACTION_BAR_INDEX],
    pub operations: Vec<PetCleanupActionBarOperationLikeCpp>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PetLearnSpellHighRankOutcomeLikeCpp {
    pub attempted_spell_ids: Vec<u32>,
    pub learned_spell_ids: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PetLearnPetPassivesOutcomeLikeCpp {
    pub creature_template_missing: bool,
    pub creature_family_missing: bool,
    pub attempted_spell_ids: Vec<u32>,
    pub learned_spell_ids: Vec<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PetLearnPetTalentOutcomeLikeCpp {
    pub talent_id: u32,
    pub debug_log_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PetAuraLikeCpp {
    pub auras_by_pet_entry: BTreeMap<u32, u32>,
    pub remove_on_change_pet: bool,
    pub damage: i32,
}

impl PetAuraLikeCpp {
    pub fn new(pet_entry: u32, aura_id: u32, remove_on_change_pet: bool, damage: i32) -> Self {
        let mut auras_by_pet_entry = BTreeMap::new();
        auras_by_pet_entry.insert(pet_entry, aura_id);
        Self {
            auras_by_pet_entry,
            remove_on_change_pet,
            damage,
        }
    }

    pub fn with_aura(mut self, pet_entry: u32, aura_id: u32) -> Self {
        self.auras_by_pet_entry.insert(pet_entry, aura_id);
        self
    }

    pub fn add_aura_like_cpp(&mut self, pet_entry: u32, aura_id: u32) {
        self.auras_by_pet_entry.insert(pet_entry, aura_id);
    }

    pub fn aura_for_pet_entry_like_cpp(&self, pet_entry: u32) -> u32 {
        self.auras_by_pet_entry
            .get(&pet_entry)
            .copied()
            .or_else(|| self.auras_by_pet_entry.get(&0).copied())
            .unwrap_or(0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PetCastPetAuraPlanLikeCpp {
    pub owner_pet_aura_index: usize,
    pub aura_id: u32,
    pub spell_value_base_point0: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PetCastPetAurasOutcomeLikeCpp {
    pub skipped_not_permanent: bool,
    pub removed_owner_pet_aura_indices: Vec<usize>,
    pub removed_pet_aura_spell_ids: Vec<u32>,
    pub cast_auras: Vec<PetCastPetAuraPlanLikeCpp>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PetInitCreateSpellsOutcomeLikeCpp {
    pub init_pet_action_bar: bool,
    pub cleared_spell_count: usize,
    pub cleared_autospell_count: usize,
    pub learn_pet_passives: PetLearnPetPassivesOutcomeLikeCpp,
    pub init_levelup_spells_for_level: bool,
    pub cast_pet_auras_current: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PetLearnSpellOutcomeLikeCpp {
    pub learned: bool,
    pub packet_spell_ids: Vec<u32>,
    pub send_direct_message: bool,
    pub pet_spell_initialize: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PetLearnSpellsOutcomeLikeCpp {
    pub learned_spell_ids: Vec<u32>,
    pub packet_spell_ids: Vec<u32>,
    pub send_session_packet: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PetToggleAutocastOutcomeLikeCpp {
    pub spell_found: bool,
    pub autocastable: bool,
    pub autospell_added: bool,
    pub autospell_removed: bool,
    pub active_changed: bool,
    pub marked_changed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PetRemoveSpellOutcomeLikeCpp {
    pub removed: bool,
    pub erased_new_spell: bool,
    pub marked_removed: bool,
    pub remove_auras_due_to_spell: bool,
    pub learned_prev_spell_id: Option<u32>,
    pub cleared_action_bar_slot: Option<usize>,
    pub pet_spell_initialize: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PetUnlearnSpellOutcomeLikeCpp {
    pub remove_spell: PetRemoveSpellOutcomeLikeCpp,
    pub packet_spell_ids: Vec<u32>,
    pub send_direct_message: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PetUnlearnSpellsOutcomeLikeCpp {
    pub remove_spells: Vec<(u32, PetRemoveSpellOutcomeLikeCpp)>,
    pub packet_spell_ids: Vec<u32>,
    pub send_session_packet: bool,
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
    pub(super) creature: Creature,
    pub(super) unit_type_mask: u32,
    pub(super) owner_guid: ObjectGuid,
    pub(super) pet_type: PetType,
    pub(super) duration_ms: i32,
    pub(super) loading: bool,
    pub(super) removed: bool,
    pub(super) focus_regen_timer_ms: u32,
    pub(super) pet_experience: u32,
    pub(super) pet_next_level_experience: u32,
    pub(super) group_update_mask: u32,
    pub(super) pet_specialization: u16,
    /// C++ `UF::UnitData::CreatedBySpell` for the live Pet.
    pub(super) created_by_spell_id: u32,
    pub(super) declined_name: Option<String>,
    pub(super) declined_names: Option<PetDeclinedNamesLikeCpp>,
    pub(super) spells: BTreeMap<u32, PetSpell>,
    pub(super) autospells: Vec<u32>,
}

impl PetRemoveSpellOutcomeLikeCpp {
    pub(super) const fn not_removed() -> Self {
        Self {
            removed: false,
            erased_new_spell: false,
            marked_removed: false,
            remove_auras_due_to_spell: false,
            learned_prev_spell_id: None,
            cleared_action_bar_slot: None,
            pet_spell_initialize: false,
        }
    }
}
