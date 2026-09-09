//! Spell metadata and proc rules state definitions, part 1 of 2.
//!
//! Separated from the mod.rs root under #646. Behaviour is preserved.

use super::*;

/// C++ `MAX_SPELL_EFFECTS` (`DBCEnums.h`).
pub const MAX_SPELL_EFFECTS_LIKE_CPP: i32 = 32;

/// C++ `TOTAL_SPELL_EFFECTS` (`SharedDefines.h`): last effect id 315 + sentinel.
pub const TOTAL_SPELL_EFFECTS_LIKE_CPP: i32 = 316;

/// C++ `TOTAL_AURAS` (`SpellAuraDefines.h`): last aura id 544 + sentinel.
pub const TOTAL_AURAS_LIKE_CPP: i32 = 545;

/// C++ `TOTAL_SPELL_TARGETS` (`SharedDefines.h`): last target id 152 + sentinel.
pub const TOTAL_SPELL_TARGETS_LIKE_CPP: i32 = 153;

/// Difficulty-aware spell metadata used by C++ spell-hit resolution.
///
/// Each effect mechanic is keyed by `SpellEffectEntry::EffectIndex`. A zero
/// value is retained when the effect row exists so that an exact-difficulty
/// row still suppresses the same effect slot from a fallback difficulty.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SpellHitMetadataLikeCpp {
    /// C++ `SpellInfo::CategoryId`, resolved from `SpellCategories`.
    pub category_id: u32,
    /// C++ `SpellInfo::ChargeCategoryId`, resolved from `SpellCategories`.
    pub charge_category_id: u32,
    pub defense_type: i8,
    pub spell_mechanic: i8,
    pub school_mask: u8,
    pub effect_mechanics: BTreeMap<u32, i32>,
}

/// Missing or malformed data that prevents safe C++ primary-profession
/// classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimaryProfessionSpellClassificationErrorLikeCpp {
    InvalidSpellId {
        spell_id: i32,
    },
    InvalidSkillId {
        spell_id: i32,
        effect_index: u32,
        skill_id: i32,
    },
    MissingSkillLinePayload {
        spell_id: i32,
        skill_id: u32,
    },
    RankChainIndeterminate {
        spell_id: u32,
    },
}

/// Calculated spell power cost, mirroring C++ `Spell::m_powerCost`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellPowerCostLikeCpp {
    pub power_type: i8,
    pub amount: i32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpellTargetPositionLikeCpp {
    pub target_map_id: u16,
    pub position: wow_core::Position,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpellTargetPositionRowLikeCpp {
    pub spell_id: u32,
    pub effect_index: u32,
    pub target_map_id: u16,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub orientation: Option<f32>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SpellTargetPositionLoadReportLikeCpp {
    pub loaded: usize,
    pub skipped_missing_map: usize,
    pub skipped_missing_spell: usize,
    pub skipped_missing_effect: usize,
    pub skipped_zero_position: usize,
    pub skipped_unsupported_target: usize,
}

pub const SPELL_AURA_DUMMY_LIKE_CPP: i32 = 0;

pub const TARGET_UNIT_PET_LIKE_CPP: u32 = 5;

pub const SKILL_DUAL_WIELD_LIKE_CPP: u16 = 118;

pub const SPELL_GROUP_CORE_RANGE_MAX_LIKE_CPP: u32 = 5;

pub const SPELL_GROUP_DB_RANGE_MIN_LIKE_CPP: u32 = 1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellPetAuraRowLikeCpp {
    pub spell_id: u32,
    pub effect_index: u8,
    pub pet_entry: u32,
    pub aura_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellPetAuraSourceLookupLikeCpp {
    SpellMissing,
    EffectIndexMissing,
    Found(SpellPetAuraSourceEffectLikeCpp),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellPetAuraLoadErrorKindLikeCpp {
    SpellMissing,
    EffectIndexMissing,
    SourceEffectNotDummy,
    AuraSpellMissing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellPetAuraLoadErrorLikeCpp {
    pub row: SpellPetAuraRowLikeCpp,
    pub kind: SpellPetAuraLoadErrorKindLikeCpp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellPetAuraLoadOutcomeLikeCpp {
    pub store: SpellPetAuraStoreLikeCpp,
    pub loaded_row_count: usize,
    pub errors: Vec<SpellPetAuraLoadErrorLikeCpp>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpellThreatRowLikeCpp {
    pub spell_id: u32,
    pub flat_mod: i32,
    pub pct_mod: f32,
    pub ap_pct_mod: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpellThreatLoadErrorLikeCpp {
    pub row: SpellThreatRowLikeCpp,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpellThreatLoadOutcomeLikeCpp {
    pub store: SpellThreatStoreLikeCpp,
    pub loaded_row_count: usize,
    pub errors: Vec<SpellThreatLoadErrorLikeCpp>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SpellLinkedTypeLikeCpp {
    Cast,
    Hit,
    Aura,
    Remove,
}

impl SpellLinkedTypeLikeCpp {
    pub fn from_u8_like_cpp(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Cast),
            1 => Some(Self::Hit),
            2 => Some(Self::Aura),
            3 => Some(Self::Remove),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellLinkedRowLikeCpp {
    pub spell_trigger: i32,
    pub spell_effect: i32,
    pub link_type: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellLinkedLoadErrorKindLikeCpp {
    TriggerSpellMissing,
    EffectSpellMissing,
    InvalidLinkType,
    SelfTriggerLoop,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellLinkedLoadErrorLikeCpp {
    pub row: SpellLinkedRowLikeCpp,
    pub kind: SpellLinkedLoadErrorKindLikeCpp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellLinkedLoadWarningKindLikeCpp {
    TriggerEffectSameBasePoint { effect_index: u32 },
    NegativeTriggerLinkTypeCoercedToRemove,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellLinkedLoadWarningLikeCpp {
    pub row: SpellLinkedRowLikeCpp,
    pub kind: SpellLinkedLoadWarningKindLikeCpp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellLinkedLoadOutcomeLikeCpp {
    pub store: SpellLinkedStoreLikeCpp,
    pub loaded_row_count: usize,
    pub errors: Vec<SpellLinkedLoadErrorLikeCpp>,
    pub warnings: Vec<SpellLinkedLoadWarningLikeCpp>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellTotemModelRowLikeCpp {
    pub spell_id: u32,
    pub race_id: u8,
    pub display_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellTotemModelLoadErrorKindLikeCpp {
    SpellMissing,
    RaceMissing,
    DisplayMissing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellTotemModelLoadErrorLikeCpp {
    pub row: SpellTotemModelRowLikeCpp,
    pub kind: SpellTotemModelLoadErrorKindLikeCpp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellTotemModelLoadOutcomeLikeCpp {
    pub store: SpellTotemModelStoreLikeCpp,
    pub loaded_row_count: usize,
    pub errors: Vec<SpellTotemModelLoadErrorLikeCpp>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellRequiredRowLikeCpp {
    pub spell_id: u32,
    pub req_spell: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellRequiredLoadErrorKindLikeCpp {
    SpellMissing,
    RequiredSpellMissing,
    SameRankChain,
    Duplicate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellRequiredLoadErrorLikeCpp {
    pub row: SpellRequiredRowLikeCpp,
    pub kind: SpellRequiredLoadErrorKindLikeCpp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellRequiredLoadOutcomeLikeCpp {
    pub store: SpellRequiredStoreLikeCpp,
    pub loaded_row_count: usize,
    pub errors: Vec<SpellRequiredLoadErrorLikeCpp>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellRankEdgeLikeCpp {
    pub spell_id: u32,
    pub supercedes_spell_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellChainNodeLikeCpp {
    pub prev_spell_id: Option<u32>,
    pub next_spell_id: Option<u32>,
    pub first_spell_id: u32,
    pub last_spell_id: u32,
    pub rank: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpellChainLoadDiagnosticLikeCpp {
    SelfLoop {
        spell_id: u32,
    },
    MultiplePredecessors {
        spell_id: u32,
        predecessor_spell_ids: Vec<u32>,
    },
    Cycle {
        spell_ids: Vec<u32>,
    },
    RankOutOfRange {
        first_spell_id: u32,
        spell_id: u32,
        rank: usize,
    },
    InvalidEffectiveSkillLineAbilityRankEndpoints {
        record_id: u32,
        spell_raw: i128,
        supercedes_spell_raw: i128,
        affected_spell_ids: Vec<u32>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellChainLookupLikeCpp<'a> {
    Unranked,
    Node(&'a SpellChainNodeLikeCpp),
    Indeterminate(&'a [SpellChainLoadDiagnosticLikeCpp]),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SpellChainLoadOutcomeLikeCpp {
    pub store: SpellChainStoreLikeCpp,
    pub diagnostics_in_order_like_cpp: Vec<SpellChainLoadDiagnosticLikeCpp>,
}

impl SpellChainLoadOutcomeLikeCpp {
    /// Fail every known weak component touched by one invalid effective rank
    /// row closed. If neither raw endpoint fits C++'s signed `int32` source
    /// domain, the complete rank projection becomes indeterminate.
    pub(super) fn mark_invalid_skill_line_ability_rank_row_like_cpp(
        &mut self,
        record_id: u32,
        spell_raw: i128,
        supercedes_spell_raw: i128,
        affected_spell_ids: &[u32],
    ) {
        let diagnostic =
            SpellChainLoadDiagnosticLikeCpp::InvalidEffectiveSkillLineAbilityRankEndpoints {
                record_id,
                spell_raw,
                supercedes_spell_raw,
                affected_spell_ids: affected_spell_ids.to_vec(),
            };

        if affected_spell_ids.is_empty() || self.store.global_indeterminate_like_cpp.is_some() {
            let mut global_diagnostics = self
                .store
                .global_indeterminate_like_cpp
                .as_ref()
                .map(|diagnostics| diagnostics.to_vec())
                .unwrap_or_default();
            if self.store.global_indeterminate_like_cpp.is_none() {
                for local_diagnostics in self.store.indeterminate_by_spell_id_like_cpp.values() {
                    for local_diagnostic in local_diagnostics.iter() {
                        if !global_diagnostics.contains(local_diagnostic) {
                            global_diagnostics.push(local_diagnostic.clone());
                        }
                    }
                }
            }
            if !global_diagnostics.contains(&diagnostic) {
                global_diagnostics.push(diagnostic.clone());
            }
            self.store.global_indeterminate_like_cpp = Some(global_diagnostics.into());
            self.store.chains_by_spell_id.clear();
            self.store.indeterminate_by_spell_id_like_cpp.clear();
            if !self.diagnostics_in_order_like_cpp.contains(&diagnostic) {
                self.diagnostics_in_order_like_cpp.push(diagnostic);
            }
            return;
        }

        let mut complete_affected_spell_ids = BTreeSet::new();
        let mut combined_diagnostics = Vec::new();
        for spell_id in affected_spell_ids {
            if let Some(node) = self.store.chains_by_spell_id.get(spell_id) {
                let first_spell_id = node.first_spell_id;
                complete_affected_spell_ids.extend(
                    self.store.chains_by_spell_id.iter().filter_map(
                        |(candidate_spell_id, candidate)| {
                            (candidate.first_spell_id == first_spell_id)
                                .then_some(*candidate_spell_id)
                        },
                    ),
                );
                continue;
            }

            if let Some(existing) = self
                .store
                .indeterminate_by_spell_id_like_cpp
                .get(spell_id)
                .cloned()
            {
                combined_diagnostics.extend(existing.iter().cloned());
                complete_affected_spell_ids.extend(
                    self.store
                        .indeterminate_by_spell_id_like_cpp
                        .iter()
                        .filter_map(|(candidate_spell_id, candidate)| {
                            std::sync::Arc::ptr_eq(candidate, &existing)
                                .then_some(*candidate_spell_id)
                        }),
                );
                continue;
            }

            complete_affected_spell_ids.insert(*spell_id);
        }

        // Diagnostics are inserted in deterministic effective RecordID order;
        // deduplication preserves the first occurrence.
        let mut deduplicated_diagnostics = Vec::new();
        for existing in combined_diagnostics {
            if !deduplicated_diagnostics.contains(&existing) {
                deduplicated_diagnostics.push(existing);
            }
        }
        if !deduplicated_diagnostics.contains(&diagnostic) {
            deduplicated_diagnostics.push(diagnostic.clone());
        }
        let shared_diagnostics: std::sync::Arc<[SpellChainLoadDiagnosticLikeCpp]> =
            deduplicated_diagnostics.into();

        for affected_spell_id in complete_affected_spell_ids {
            self.store.chains_by_spell_id.remove(&affected_spell_id);
            self.store
                .indeterminate_by_spell_id_like_cpp
                .insert(affected_spell_id, shared_diagnostics.clone());
        }
        if !self.diagnostics_in_order_like_cpp.contains(&diagnostic) {
            self.diagnostics_in_order_like_cpp.push(diagnostic);
        }
    }
}

pub(super) fn spell_rank_endpoint_id_from_raw_like_cpp(raw: i128) -> Option<u32> {
    i32::try_from(raw).ok().map(|value| value as u32)
}

pub(super) fn spell_chain_cycles_like_cpp(
    component_spell_ids: &[u32],
    chain_next_by_spell_id: &BTreeMap<u32, u32>,
) -> Vec<Vec<u32>> {
    let mut completed = BTreeSet::new();
    let mut cycles = Vec::new();

    for &start_spell_id in component_spell_ids {
        if completed.contains(&start_spell_id) {
            continue;
        }

        let mut path = Vec::new();
        let mut path_index_by_spell_id = BTreeMap::new();
        let mut current_spell_id = Some(start_spell_id);
        while let Some(spell_id) = current_spell_id {
            if completed.contains(&spell_id) {
                break;
            }
            if let Some(&cycle_start) = path_index_by_spell_id.get(&spell_id) {
                let mut cycle = path[cycle_start..].to_vec();
                if let Some((minimum_index, _)) = cycle
                    .iter()
                    .enumerate()
                    .min_by_key(|(_, spell_id)| *spell_id)
                {
                    cycle.rotate_left(minimum_index);
                }
                cycles.push(cycle);
                break;
            }

            path_index_by_spell_id.insert(spell_id, path.len());
            path.push(spell_id);
            current_spell_id = chain_next_by_spell_id.get(&spell_id).copied();
        }
        completed.extend(path);
    }

    cycles.sort();
    cycles
}

pub const SPELL_AREA_FLAG_AUTOCAST_LIKE_CPP: u8 = 0x1;

pub const SPELL_AREA_FLAG_AUTOREMOVE_LIKE_CPP: u8 = 0x2;

pub const SPELL_AREA_FLAG_IGNORE_AUTOCAST_ON_QUEST_STATUS_CHANGE_LIKE_CPP: u8 = 0x4;

pub const GENDER_MALE_LIKE_CPP: u8 = 0;

pub const GENDER_FEMALE_LIKE_CPP: u8 = 1;

pub const GENDER_NONE_LIKE_CPP: u8 = 2;

pub const SPELL_ATTR0_CU_SHARE_DAMAGE_LIKE_CPP: u32 = 0x0000_0008;

pub const SPELL_ATTR0_CU_NO_INITIAL_THREAT_LIKE_CPP: u32 = 0x0000_0010;

pub const SPELL_ATTR0_CU_CAN_CRIT_LIKE_CPP: u32 = 0x0000_0080;

pub const SPELL_ATTR0_CU_DIRECT_DAMAGE_LIKE_CPP: u32 = 0x0000_0100;

pub const SPELL_ATTR0_CU_IS_TALENT_LIKE_CPP: u32 = 0x0080_0000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellAreaRowLikeCpp {
    pub spell_id: u32,
    pub area_id: u32,
    pub quest_start: u32,
    pub quest_start_status: u32,
    pub quest_end_status: u32,
    pub quest_end: u32,
    pub aura_spell: i32,
    pub race_mask: u64,
    pub gender: u8,
    pub flags: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellAreaLikeCpp {
    pub spell_id: u32,
    pub area_id: u32,
    pub quest_start: u32,
    pub quest_end: u32,
    pub aura_spell: i32,
    pub race_mask: u64,
    pub gender: u8,
    pub quest_start_status: u32,
    pub quest_end_status: u32,
    pub flags: u8,
}

impl From<SpellAreaRowLikeCpp> for SpellAreaLikeCpp {
    fn from(row: SpellAreaRowLikeCpp) -> Self {
        Self {
            spell_id: row.spell_id,
            area_id: row.area_id,
            quest_start: row.quest_start,
            quest_end: row.quest_end,
            aura_spell: row.aura_spell,
            race_mask: row.race_mask,
            gender: row.gender,
            quest_start_status: row.quest_start_status,
            quest_end_status: row.quest_end_status,
            flags: row.flags,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellAreaLoadErrorKindLikeCpp {
    SpellMissing,
    DuplicateSimilarRequirements,
    AreaMissing,
    QuestStartMissing,
    QuestEndMissing,
    AuraSpellMissing,
    AuraSpellSelfRequirement,
    AuraAutocastChain,
    InvalidRaceMask,
    InvalidGender,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellAreaLoadErrorLikeCpp {
    pub row: SpellAreaRowLikeCpp,
    pub kind: SpellAreaLoadErrorKindLikeCpp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellAreaLoadOutcomeLikeCpp {
    pub store: SpellAreaStoreLikeCpp,
    pub loaded_row_count: usize,
    pub errors: Vec<SpellAreaLoadErrorLikeCpp>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellGroupRowLikeCpp {
    pub group_id: u32,
    pub spell_id: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellGroupLoadErrorKindLikeCpp {
    CoreRangeGroupMissing,
    ReferencedGroupMissing,
    SpellMissing,
    SpellNotFirstRank,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellGroupLoadErrorLikeCpp {
    pub row: SpellGroupRowLikeCpp,
    pub kind: SpellGroupLoadErrorKindLikeCpp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellGroupLoadOutcomeLikeCpp {
    pub store: SpellGroupStoreLikeCpp,
    pub loaded_row_count: usize,
    pub errors: Vec<SpellGroupLoadErrorLikeCpp>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum SpellGroupStackRuleLikeCpp {
    Default = 0,
    Exclusive = 1,
    ExclusiveFromSameCaster = 2,
    ExclusiveSameEffect = 3,
    ExclusiveHighest = 4,
}

impl SpellGroupStackRuleLikeCpp {
    pub const MAX_LIKE_CPP: u8 = 5;

    pub const fn from_u8_like_cpp(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Default),
            1 => Some(Self::Exclusive),
            2 => Some(Self::ExclusiveFromSameCaster),
            3 => Some(Self::ExclusiveSameEffect),
            4 => Some(Self::ExclusiveHighest),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellGroupStackRuleRowLikeCpp {
    pub group_id: u32,
    pub stack_rule: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellGroupStackRuleLoadErrorKindLikeCpp {
    StackRuleMissing,
    GroupMissing,
    SameEffectSpellMissing,
    SameEffectSpellAuraMissing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellGroupStackRuleLoadErrorLikeCpp {
    pub row: SpellGroupStackRuleRowLikeCpp,
    pub spell_id: Option<u32>,
    pub kind: SpellGroupStackRuleLoadErrorKindLikeCpp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellGroupStackRuleLoadOutcomeLikeCpp {
    pub store: SpellGroupStackRuleStoreLikeCpp,
    pub loaded_row_count: usize,
    pub same_effect_parsed_count: usize,
    pub errors: Vec<SpellGroupStackRuleLoadErrorLikeCpp>,
}

pub const SPELL_SCHOOL_MASK_ALL_LIKE_CPP: u8 = 0x7F;

pub const PROC_FLAG_HEARTBEAT_LIKE_CPP: u32 = 0x0000_0001;

pub const PROC_FLAG_KILL_LIKE_CPP: u32 = 0x0000_0002;

pub const PROC_FLAG_DEAL_MELEE_SWING_LIKE_CPP: u32 = 0x0000_0004;

pub const PROC_FLAG_TAKE_MELEE_SWING_LIKE_CPP: u32 = 0x0000_0008;

pub const PROC_FLAG_DEAL_MELEE_ABILITY_LIKE_CPP: u32 = 0x0000_0010;

pub const PROC_FLAG_TAKE_MELEE_ABILITY_LIKE_CPP: u32 = 0x0000_0020;

pub const PROC_FLAG_DEAL_RANGED_ATTACK_LIKE_CPP: u32 = 0x0000_0040;

pub const PROC_FLAG_TAKE_RANGED_ATTACK_LIKE_CPP: u32 = 0x0000_0080;

pub const PROC_FLAG_DEAL_RANGED_ABILITY_LIKE_CPP: u32 = 0x0000_0100;

pub const PROC_FLAG_TAKE_RANGED_ABILITY_LIKE_CPP: u32 = 0x0000_0200;

pub const PROC_FLAG_DEAL_HELPFUL_ABILITY_LIKE_CPP: u32 = 0x0000_0400;

pub const PROC_FLAG_TAKE_HELPFUL_ABILITY_LIKE_CPP: u32 = 0x0000_0800;

pub const PROC_FLAG_DEAL_HARMFUL_ABILITY_LIKE_CPP: u32 = 0x0000_1000;

pub const PROC_FLAG_TAKE_HARMFUL_ABILITY_LIKE_CPP: u32 = 0x0000_2000;

pub const PROC_FLAG_DEAL_HELPFUL_SPELL_LIKE_CPP: u32 = 0x0000_4000;

pub const PROC_FLAG_TAKE_HELPFUL_SPELL_LIKE_CPP: u32 = 0x0000_8000;

pub const PROC_FLAG_DEAL_HARMFUL_SPELL_LIKE_CPP: u32 = 0x0001_0000;

pub const PROC_FLAG_TAKE_HARMFUL_SPELL_LIKE_CPP: u32 = 0x0002_0000;

pub const PROC_FLAG_DEAL_HARMFUL_PERIODIC_LIKE_CPP: u32 = 0x0004_0000;

pub const PROC_FLAG_TAKE_HARMFUL_PERIODIC_LIKE_CPP: u32 = 0x0008_0000;

pub const PROC_FLAG_TAKE_ANY_DAMAGE_LIKE_CPP: u32 = 0x0010_0000;

pub const PROC_FLAG_DEAL_HELPFUL_PERIODIC_LIKE_CPP: u32 = 0x0020_0000;

pub const PROC_FLAG_MAIN_HAND_WEAPON_SWING_LIKE_CPP: u32 = 0x0040_0000;

pub const PROC_FLAG_OFF_HAND_WEAPON_SWING_LIKE_CPP: u32 = 0x0080_0000;

pub const PROC_FLAG_TAKE_HELPFUL_PERIODIC_LIKE_CPP: u32 = 0x8000_0000;

pub const PROC_FLAG_2_CAST_SUCCESSFUL_LIKE_CPP: u32 = 0x0000_0004;

pub const PROC_SPELL_TYPE_DAMAGE_LIKE_CPP: u32 = 0x0000_0001;

pub const PROC_SPELL_TYPE_HEAL_LIKE_CPP: u32 = 0x0000_0002;

pub const PROC_SPELL_TYPE_NO_DMG_HEAL_LIKE_CPP: u32 = 0x0000_0004;

pub const PROC_SPELL_TYPE_MASK_ALL_LIKE_CPP: u32 = PROC_SPELL_TYPE_DAMAGE_LIKE_CPP
    | PROC_SPELL_TYPE_HEAL_LIKE_CPP
    | PROC_SPELL_TYPE_NO_DMG_HEAL_LIKE_CPP;

pub const PROC_SPELL_PHASE_CAST_LIKE_CPP: u32 = 0x0000_0001;

pub const PROC_SPELL_PHASE_HIT_LIKE_CPP: u32 = 0x0000_0002;

pub const PROC_SPELL_PHASE_FINISH_LIKE_CPP: u32 = 0x0000_0004;

pub const PROC_SPELL_PHASE_MASK_ALL_LIKE_CPP: u32 = PROC_SPELL_PHASE_CAST_LIKE_CPP
    | PROC_SPELL_PHASE_HIT_LIKE_CPP
    | PROC_SPELL_PHASE_FINISH_LIKE_CPP;

pub const PROC_HIT_NORMAL_LIKE_CPP: u32 = 0x0000_0001;

pub const PROC_HIT_CRITICAL_LIKE_CPP: u32 = 0x0000_0002;

pub const PROC_HIT_MISS_LIKE_CPP: u32 = 0x0000_0004;

pub const PROC_HIT_BLOCK_LIKE_CPP: u32 = 0x0000_0040;

pub const PROC_HIT_ABSORB_LIKE_CPP: u32 = 0x0000_0400;

pub const PROC_HIT_REFLECT_LIKE_CPP: u32 = 0x0000_0800;

pub const PROC_HIT_MASK_ALL_LIKE_CPP: u32 = 0x0007_FFFF;

pub const PROC_ATTR_REQ_SPELLMOD_LIKE_CPP: u32 = 0x0000_0008;

pub const PROC_ATTR_REQ_EXP_OR_HONOR_LIKE_CPP: u32 = 0x0000_0001;

pub const PROC_ATTR_TRIGGERED_CAN_PROC_LIKE_CPP: u32 = 0x0000_0002;

pub const PROC_ATTR_REQ_POWER_COST_LIKE_CPP: u32 = 0x0000_0004;

pub const PROC_ATTR_USE_STACKS_FOR_CHARGES_LIKE_CPP: u32 = 0x0000_0010;

pub const PROC_ATTR_REDUCE_PROC_60_LIKE_CPP: u32 = 0x0000_0080;

pub const PROC_ATTR_ALL_ALLOWED_LIKE_CPP: u32 = PROC_ATTR_REQ_EXP_OR_HONOR_LIKE_CPP
    | PROC_ATTR_TRIGGERED_CAN_PROC_LIKE_CPP
    | PROC_ATTR_REQ_POWER_COST_LIKE_CPP
    | PROC_ATTR_REQ_SPELLMOD_LIKE_CPP
    | PROC_ATTR_USE_STACKS_FOR_CHARGES_LIKE_CPP
    | PROC_ATTR_REDUCE_PROC_60_LIKE_CPP;

pub const SPELL_PROC_FLAG_MASK_LIKE_CPP: u32 = PROC_FLAG_DEAL_MELEE_ABILITY_LIKE_CPP
    | PROC_FLAG_TAKE_MELEE_ABILITY_LIKE_CPP
    | PROC_FLAG_DEAL_RANGED_ATTACK_LIKE_CPP
    | PROC_FLAG_TAKE_RANGED_ATTACK_LIKE_CPP
    | PROC_FLAG_DEAL_RANGED_ABILITY_LIKE_CPP
    | PROC_FLAG_TAKE_RANGED_ABILITY_LIKE_CPP
    | PROC_FLAG_DEAL_HELPFUL_ABILITY_LIKE_CPP
    | PROC_FLAG_TAKE_HELPFUL_ABILITY_LIKE_CPP
    | PROC_FLAG_DEAL_HARMFUL_ABILITY_LIKE_CPP
    | PROC_FLAG_TAKE_HARMFUL_ABILITY_LIKE_CPP
    | PROC_FLAG_DEAL_HELPFUL_SPELL_LIKE_CPP
    | PROC_FLAG_TAKE_HELPFUL_SPELL_LIKE_CPP
    | PROC_FLAG_DEAL_HARMFUL_SPELL_LIKE_CPP
    | PROC_FLAG_TAKE_HARMFUL_SPELL_LIKE_CPP
    | PROC_FLAG_DEAL_HARMFUL_PERIODIC_LIKE_CPP
    | PROC_FLAG_TAKE_HARMFUL_PERIODIC_LIKE_CPP
    | PROC_FLAG_DEAL_HELPFUL_PERIODIC_LIKE_CPP
    | PROC_FLAG_TAKE_HELPFUL_PERIODIC_LIKE_CPP;

pub const DONE_HIT_PROC_FLAG_MASK_LIKE_CPP: u32 = PROC_FLAG_DEAL_MELEE_SWING_LIKE_CPP
    | PROC_FLAG_DEAL_RANGED_ATTACK_LIKE_CPP
    | PROC_FLAG_DEAL_MELEE_ABILITY_LIKE_CPP
    | PROC_FLAG_DEAL_RANGED_ABILITY_LIKE_CPP
    | PROC_FLAG_DEAL_HELPFUL_ABILITY_LIKE_CPP
    | PROC_FLAG_DEAL_HARMFUL_ABILITY_LIKE_CPP
    | PROC_FLAG_DEAL_HELPFUL_SPELL_LIKE_CPP
    | PROC_FLAG_DEAL_HARMFUL_SPELL_LIKE_CPP
    | PROC_FLAG_DEAL_HARMFUL_PERIODIC_LIKE_CPP
    | PROC_FLAG_DEAL_HELPFUL_PERIODIC_LIKE_CPP
    | PROC_FLAG_MAIN_HAND_WEAPON_SWING_LIKE_CPP
    | PROC_FLAG_OFF_HAND_WEAPON_SWING_LIKE_CPP;

pub const TAKEN_HIT_PROC_FLAG_MASK_LIKE_CPP: u32 = PROC_FLAG_TAKE_MELEE_SWING_LIKE_CPP
    | PROC_FLAG_TAKE_RANGED_ATTACK_LIKE_CPP
    | PROC_FLAG_TAKE_MELEE_ABILITY_LIKE_CPP
    | PROC_FLAG_TAKE_RANGED_ABILITY_LIKE_CPP
    | PROC_FLAG_TAKE_HELPFUL_ABILITY_LIKE_CPP
    | PROC_FLAG_TAKE_HARMFUL_ABILITY_LIKE_CPP
    | PROC_FLAG_TAKE_HELPFUL_SPELL_LIKE_CPP
    | PROC_FLAG_TAKE_HARMFUL_SPELL_LIKE_CPP
    | PROC_FLAG_TAKE_HARMFUL_PERIODIC_LIKE_CPP
    | PROC_FLAG_TAKE_HELPFUL_PERIODIC_LIKE_CPP
    | PROC_FLAG_TAKE_ANY_DAMAGE_LIKE_CPP;

pub const REQ_SPELL_PHASE_PROC_FLAG_MASK_LIKE_CPP: u32 =
    SPELL_PROC_FLAG_MASK_LIKE_CPP & DONE_HIT_PROC_FLAG_MASK_LIKE_CPP;

pub const PROC_FLAG_DEATH_LIKE_CPP: u32 = 0x0100_0000;

pub const CAN_PROC_FROM_PROCS_UNRESTRICTED_DONE_FLAGS_LIKE_CPP: u32 =
    PROC_FLAG_DEAL_MELEE_ABILITY_LIKE_CPP
        | PROC_FLAG_DEAL_RANGED_ATTACK_LIKE_CPP
        | PROC_FLAG_DEAL_RANGED_ABILITY_LIKE_CPP
        | PROC_FLAG_DEAL_HELPFUL_ABILITY_LIKE_CPP
        | PROC_FLAG_DEAL_HARMFUL_ABILITY_LIKE_CPP
        | PROC_FLAG_DEAL_HELPFUL_SPELL_LIKE_CPP
        | PROC_FLAG_DEAL_HARMFUL_SPELL_LIKE_CPP
        | PROC_FLAG_DEAL_HARMFUL_PERIODIC_LIKE_CPP
        | PROC_FLAG_DEAL_HELPFUL_PERIODIC_LIKE_CPP;
