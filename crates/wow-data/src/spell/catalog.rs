// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Effective spell metadata: effects, entries and restrictions.

use super::*;

/// Metadata for a spell from Spell.db2 and related tables.
#[derive(Debug, Clone)]
pub struct SpellInfo {
    /// Spell ID
    pub spell_id: i32,
    /// Cast time in milliseconds (0 = instant)
    pub cast_time_ms: u32,
    /// Global cooldown in milliseconds
    pub cooldown_ms: u32,
    /// Per-spell cooldown in milliseconds (0 = no per-spell cooldown)
    pub recovery_time_ms: u32,
    /// First effect type (primary effect) — e.g., 2 (damage), 6 (aura), 10 (heal)
    pub effect_type: u32,
    /// Base damage/healing before bonuses
    pub effect_base_points: i32,
    /// Spell power / attack power coefficient (0.0 = no scaling)
    pub effect_bonus_coefficient: f32,
    /// Aura type if effect_type == SPELL_EFFECT_APPLY_AURA
    pub aura_type: Option<i32>,
    /// Display flags (channelled, etc.)
    pub display_flags: u32,
    /// C++ `SpellInfo::RequiresSpellFocus`, hydrated from
    /// `SpellCastingRequirementsEntry::RequiresSpellFocus`.
    pub requires_spell_focus: u32,
    /// C++ `SpellInfo::PowerCosts`, hydrated from `SpellPower.db2`.
    pub power_costs: Vec<SpellPowerCostInfoLikeCpp>,
    /// Spell effects keyed by C++ `SpellEffectInfo::EffectIndex`.
    pub effects: Vec<SpellEffectInfo>,
}

/// Represented subset of C++ `SpellPowerEntry` stored on `SpellInfo::PowerCosts`.
#[derive(Debug, Clone, PartialEq)]
pub struct SpellPowerCostInfoLikeCpp {
    pub order_index: u8,
    pub power_type: i8,
    pub mana_cost: i32,
    pub mana_cost_per_level: i32,
    pub mana_per_second: i32,
    pub power_cost_pct: f32,
    pub power_cost_max_pct: f32,
    pub power_pct_per_second: f32,
    pub required_aura_spell_id: i32,
    pub optional_cost: u32,
}

/// Minimal `SpellEffectInfo` fields needed by C++ ConditionMgr validation.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SpellEffectInfo {
    pub effect_index: u32,
    pub effect: u32,
    pub effect_aura: i32,
    pub effect_base_points: i32,
    pub effect_die_sides: i32,
    pub effect_spell_class_mask: [u32; 4],
    pub effect_misc_value_1: i32,
    pub effect_misc_value_2: i32,
    pub effect_trigger_spell: i32,
    /// C++ `SpellEffectEntry::EffectRadiusIndex[0]` / TargetA radius index.
    pub effect_radius_index_1: u32,
    pub position_facing: f32,
    pub chain_targets: i32,
    pub implicit_target_1: u32,
    pub implicit_target_2: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellPetAuraSourceEffectLikeCpp {
    pub effect: u32,
    pub apply_aura_name: i32,
    pub target_a: u32,
    pub calc_value: i32,
}

impl SpellPetAuraSourceEffectLikeCpp {
    pub const fn is_valid_pet_aura_source_like_cpp(self) -> bool {
        self.effect == spell_effect_types::SPELL_EFFECT_DUMMY
            || (self.effect == spell_effect_types::SPELL_EFFECT_APPLY_AURA
                && self.apply_aura_name == SPELL_AURA_DUMMY_LIKE_CPP)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpellThreatEntryLikeCpp {
    pub flat_mod: i32,
    pub pct_mod: f32,
    pub ap_pct_mod: f32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellLinkedSpellInfoLikeCpp {
    /// Precomputed C++ `SpellEffectInfo::CalcValue()` values paired with
    /// `EffectIndex`. Rust does not have full CalcValue yet, so callers must
    /// pass authoritative values when this warning needs exact parity.
    pub effect_calc_values_by_index: Vec<(u32, i32)>,
}

impl SpellLinkedSpellInfoLikeCpp {
    pub fn from_represented_spell_info_base_points(spell_info: &SpellInfo) -> Self {
        Self {
            effect_calc_values_by_index: spell_info
                .effects()
                .iter()
                .map(|effect| (effect.effect_index, effect.effect_base_points))
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpellProcEntryLikeCpp {
    pub school_mask: u8,
    pub spell_family_name: u16,
    pub spell_family_mask: [u32; 4],
    pub proc_flags: [u32; 2],
    pub spell_type_mask: u32,
    pub spell_phase_mask: u32,
    pub hit_mask: u32,
    pub attributes_mask: u32,
    pub disable_effects_mask: u32,
    pub procs_per_minute: f32,
    pub chance: f32,
    pub cooldown_ms: u32,
    pub charges: u32,
}

impl SpellProcEntryLikeCpp {
    pub(super) fn from_row_like_cpp(row: &SpellProcRowLikeCpp) -> Self {
        Self {
            school_mask: row.school_mask,
            spell_family_name: row.spell_family_name,
            spell_family_mask: row.spell_family_mask,
            proc_flags: row.proc_flags,
            spell_type_mask: row.spell_type_mask,
            spell_phase_mask: row.spell_phase_mask,
            hit_mask: row.hit_mask,
            attributes_mask: row.attributes_mask,
            disable_effects_mask: row.disable_effects_mask,
            procs_per_minute: row.procs_per_minute,
            chance: row.chance,
            cooldown_ms: row.cooldown_ms,
            charges: u32::from(row.charges),
        }
    }

    pub fn proc_flags_any_like_cpp(&self) -> bool {
        self.proc_flags[0] != 0 || self.proc_flags[1] != 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellProcEventSpellInfoLikeCpp {
    pub spell_family_name: u16,
    pub spell_family_mask: [u32; 4],
}

impl SpellProcEventSpellInfoLikeCpp {
    pub fn is_affected_like_cpp(&self, family_name: u16, family_mask: [u32; 4]) -> bool {
        if family_name == 0 {
            return true;
        }

        if family_name != self.spell_family_name {
            return false;
        }

        if family_mask.iter().any(|mask| *mask != 0)
            && !family_mask
                .iter()
                .zip(self.spell_family_mask.iter())
                .any(|(required, actual)| required & actual != 0)
        {
            return false;
        }

        true
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellProcEventInfoLikeCpp {
    pub type_mask: [u32; 2],
    pub actor_is_player: bool,
    pub action_target_exists: bool,
    pub action_target_is_honor_or_xp: bool,
    pub proc_spell_has_positive_power_cost: Option<bool>,
    pub school_mask: u8,
    pub spell_info: Option<SpellProcEventSpellInfoLikeCpp>,
    pub spell_type_mask: u32,
    pub spell_phase_mask: u32,
    pub hit_mask: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImplicitProcAuraInfoLikeCpp {
    pub spell_type_mask: u32,
    pub triggered_can_proc: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImplicitSpellProcEffectLikeCpp {
    pub effect_index: u32,
    pub is_effect: bool,
    pub is_aura: bool,
    pub aura_type: i32,
    pub spell_class_mask: [u32; 4],
    pub calc_value: i32,
    pub trigger_spell: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpellProcSourceSpellInfoLikeCpp {
    pub spell_id: u32,
    pub difficulty: u32,
    pub first_rank_spell_id: u32,
    pub next_rank_spell_id: Option<u32>,
    pub spell_family_name: u16,
    pub proc_flags: [u32; 2],
    pub proc_charges: u32,
    pub proc_chance: f32,
    pub proc_cooldown_ms: u32,
    pub proc_base_ppm: f32,
    pub attributes3: u32,
    pub effects: Vec<SpellEffectInfo>,
}

impl SpellProcSourceSpellInfoLikeCpp {
    pub fn from_loaded_spell_like_cpp(
        spell_id: u32,
        difficulty: u32,
        spells: &SpellStore,
        spell_chains: &SpellChainStoreLikeCpp,
        spell_aura_options: &crate::spell_db2::SpellAuraOptionsStore,
        spell_misc: &crate::spell_db2::SpellMiscStore,
        spell_class_options: &crate::spell_db2::SpellClassOptionsStore,
        spell_procs_per_minute: &crate::spell_db2::SpellProcsPerMinuteStore,
    ) -> Option<Self> {
        let spell = spells.get(i32::try_from(spell_id).ok()?)?;
        let difficulty_id = u8::try_from(difficulty).unwrap_or(0);
        let aura_options =
            spell_aura_options.entry_for_spell_difficulty_like_cpp(spell_id, difficulty_id);
        let spell_misc = spell_misc.entry_for_spell_difficulty_like_cpp(spell_id, difficulty_id);
        let spell_class_options = spell_class_options.entry_for_spell_like_cpp(spell_id);

        Some(Self {
            spell_id,
            difficulty,
            first_rank_spell_id: spell_chains.first_spell_in_chain_like_cpp(spell_id),
            next_rank_spell_id: match spell_chains.next_spell_in_chain_like_cpp(spell_id) {
                0 => None,
                next => Some(next),
            },
            spell_family_name: spell_class_options
                .map(|entry| u16::from(entry.spell_class_set))
                .unwrap_or(0),
            proc_flags: aura_options
                .map(|entry| {
                    [
                        entry.proc_type_mask[0] as u32,
                        entry.proc_type_mask[1] as u32,
                    ]
                })
                .unwrap_or([0, 0]),
            proc_charges: aura_options
                .map(|entry| entry.proc_charges as u32)
                .unwrap_or(0),
            proc_chance: aura_options
                .map(|entry| f32::from(entry.proc_chance))
                .unwrap_or(0.0),
            proc_cooldown_ms: aura_options
                .map(|entry| entry.proc_category_recovery as u32)
                .unwrap_or(0),
            proc_base_ppm: aura_options
                .and_then(|entry| {
                    spell_procs_per_minute.get(u32::from(entry.spell_procs_per_minute_id))
                })
                .map(|entry| entry.base_proc_rate)
                .unwrap_or(0.0),
            attributes3: spell_misc
                .map(|entry| entry.attributes[3] as u32)
                .unwrap_or(0),
            effects: spell.effects().to_vec(),
        })
    }

    pub fn is_ranked_like_cpp(&self) -> bool {
        self.first_rank_spell_id != self.spell_id || self.next_rank_spell_id.is_some()
    }

    pub fn implicit_proc_source_like_cpp(&self) -> ImplicitSpellProcSourceLikeCpp {
        ImplicitSpellProcSourceLikeCpp {
            spell_id: self.spell_id,
            difficulty: self.difficulty,
            spell_family_name: self.spell_family_name,
            proc_flags: self.proc_flags,
            proc_chance: self.proc_chance,
            proc_cooldown_ms: self.proc_cooldown_ms,
            proc_charges: self.proc_charges,
            proc_base_ppm: self.proc_base_ppm,
            attributes3: self.attributes3,
            effects: self
                .effects
                .iter()
                .map(|effect| ImplicitSpellProcEffectLikeCpp {
                    effect_index: effect.effect_index,
                    is_effect: effect.effect != 0,
                    is_aura: effect.is_aura_like_cpp(),
                    aura_type: effect.effect_aura,
                    spell_class_mask: effect.effect_spell_class_mask,
                    calc_value: effect.calc_value_no_caster_like_cpp(),
                    trigger_spell: u32::try_from(effect.effect_trigger_spell).unwrap_or(0),
                })
                .collect(),
        }
    }
}

impl SpellInfo {
    /// Convenience: returns the effective cooldown (per-spell or global, whichever is larger).
    pub fn effective_cooldown_ms(&self) -> u32 {
        self.recovery_time_ms.max(self.cooldown_ms)
    }

    /// Returns true if this spell has a cast time (not instant).
    pub fn has_cast_time(&self) -> bool {
        self.cast_time_ms > 0
    }

    pub fn effects(&self) -> &[SpellEffectInfo] {
        &self.effects
    }

    pub fn has_aura_like_cpp(&self, aura_type: i32) -> bool {
        self.effects
            .iter()
            .any(|effect| effect.effect_aura == aura_type)
    }

    pub fn has_effect_like_cpp(&self, effect_type: u32) -> bool {
        self.effects
            .iter()
            .any(|effect| effect.effect == effect_type)
    }

    /// Returns every distinct primary-profession skill line referenced by a
    /// C++ `SPELL_EFFECT_SKILL` effect.
    ///
    /// C++ treats a missing `SkillLine` as non-primary. Rust's effective store
    /// can know an SQL-only record identity without hydrating category/parent
    /// payload, so an authorization caller must distinguish that case and fail
    /// closed rather than silently granting capacity.
    ///
    /// This returns ordered effect metadata only. It is not the set of skills
    /// learned by `Player::AddSpell`, whose C++ `SpellLearnSkillNode` selects
    /// a narrower outcome; the caller must resolve the actual learn path.
    ///
    /// The caller must first resolve this `SpellInfo` through the effective
    /// spell-key authority. This payload-only predicate cannot prove that a
    /// formerly hydrated spell remains effective after SQL/hotfix removal.
    pub fn primary_profession_skill_effect_ids_like_cpp(
        &self,
        skill_lines: &crate::skill_talent::SkillLineStore,
    ) -> Result<Vec<u32>, PrimaryProfessionSpellClassificationErrorLikeCpp> {
        let mut skill_effects: Vec<_> = self
            .effects
            .iter()
            .filter(|effect| effect.effect == spell_effect_types::SPELL_EFFECT_SKILL)
            .collect();
        skill_effects.sort_by_key(|effect| effect.effect_index);

        let mut seen_primary_skills = BTreeSet::new();
        let mut primary_skills = Vec::new();
        for effect in skill_effects {
            let skill_id = u32::try_from(effect.effect_misc_value_1).map_err(|_| {
                PrimaryProfessionSpellClassificationErrorLikeCpp::InvalidSkillId {
                    spell_id: self.spell_id,
                    effect_index: effect.effect_index,
                    skill_id: effect.effect_misc_value_1,
                }
            })?;
            let Some(is_primary) = skill_lines.is_primary_profession_skill_like_cpp(skill_id)
            else {
                return Err(
                    PrimaryProfessionSpellClassificationErrorLikeCpp::MissingSkillLinePayload {
                        spell_id: self.spell_id,
                        skill_id,
                    },
                );
            };
            if is_primary && seen_primary_skills.insert(skill_id) {
                primary_skills.push(skill_id);
            }
        }

        Ok(primary_skills)
    }

    /// C++ `SpellInfo::IsPrimaryProfession`.
    ///
    /// This is a boolean property of the spell's effects, not a description
    /// of which skills `Player::AddSpell` will learn. If partial metadata
    /// makes one effect undecidable, a later hydrated primary effect still
    /// proves the boolean result; otherwise the missing payload fails closed.
    pub fn is_primary_profession_like_cpp(
        &self,
        skill_lines: &crate::skill_talent::SkillLineStore,
    ) -> Result<bool, PrimaryProfessionSpellClassificationErrorLikeCpp> {
        let mut skill_effects: Vec<_> = self
            .effects
            .iter()
            .filter(|effect| effect.effect == spell_effect_types::SPELL_EFFECT_SKILL)
            .collect();
        skill_effects.sort_by_key(|effect| effect.effect_index);

        let mut undecidable = None;
        for effect in skill_effects {
            let skill_id = match u32::try_from(effect.effect_misc_value_1) {
                Ok(skill_id) => skill_id,
                Err(_) => {
                    undecidable.get_or_insert(
                        PrimaryProfessionSpellClassificationErrorLikeCpp::InvalidSkillId {
                            spell_id: self.spell_id,
                            effect_index: effect.effect_index,
                            skill_id: effect.effect_misc_value_1,
                        },
                    );
                    continue;
                }
            };
            match skill_lines.is_primary_profession_skill_like_cpp(skill_id) {
                Some(true) => return Ok(true),
                Some(false) => {}
                None => {
                    undecidable.get_or_insert(
                        PrimaryProfessionSpellClassificationErrorLikeCpp::MissingSkillLinePayload {
                            spell_id: self.spell_id,
                            skill_id,
                        },
                    );
                }
            }
        }

        undecidable.map_or(Ok(false), Err)
    }

    /// C++ `SpellInfo::IsPrimaryProfessionFirstRank`.
    ///
    /// `SpellInfo::GetRank()` returns one for an unranked spell. That differs
    /// intentionally from Rust's existing `SpellMgr::GetSpellRank`-shaped
    /// accessor, which returns zero when no chain node exists.
    pub fn is_primary_profession_first_rank_like_cpp(
        &self,
        skill_lines: &crate::skill_talent::SkillLineStore,
        spell_chains: &SpellChainStoreLikeCpp,
    ) -> Result<bool, PrimaryProfessionSpellClassificationErrorLikeCpp> {
        let spell_id = u32::try_from(self.spell_id).map_err(|_| {
            PrimaryProfessionSpellClassificationErrorLikeCpp::InvalidSpellId {
                spell_id: self.spell_id,
            }
        })?;
        let rank = match spell_chains.spell_chain_lookup_like_cpp(spell_id) {
            SpellChainLookupLikeCpp::Node(node) => node.rank,
            SpellChainLookupLikeCpp::Unranked => 1,
            SpellChainLookupLikeCpp::Indeterminate(_) => {
                // Preserve the other safe short-circuit in C++'s
                // `IsPrimaryProfession() && GetRank() == 1`: a spell proven
                // non-primary is false regardless of an ambiguous rank.
                return match self.is_primary_profession_like_cpp(skill_lines) {
                    Ok(false) => Ok(false),
                    Ok(true) | Err(_) => Err(
                        PrimaryProfessionSpellClassificationErrorLikeCpp::RankChainIndeterminate {
                            spell_id,
                        },
                    ),
                };
            }
        };
        // With complete C++ data this is equivalent to the original
        // `IsPrimaryProfession() && GetRank() == 1`. Resolving rank first also
        // avoids requiring partial SkillLine payload when rank already proves
        // the result false.
        if rank != 1 {
            return Ok(false);
        }

        self.is_primary_profession_like_cpp(skill_lines)
    }

    pub fn requires_spell_focus_like_cpp(&self) -> bool {
        self.requires_spell_focus != 0
    }

    /// Represented subset of C++ `SpellInfo::CalcPowerCost` (`SpellInfo.cpp:3984`).
    ///
    /// This covers the DB2 `ManaCost` flat amount and mana percentage costs used
    /// by early live casts. Aura/spellmod/NPC scaling and non-mana max-power DB2
    /// lookups are intentionally deferred.
    pub fn calc_power_costs_like_cpp(&self, caster_create_mana: i32) -> Vec<SpellPowerCostLikeCpp> {
        let mut costs = Vec::new();

        for power in &self.power_costs {
            // C++ skips this power entry unless the caster has the required aura.
            // The represented cast path has no full aura query yet, so fail-open
            // by ignoring gated costs rather than charging the wrong row.
            if power.required_aura_spell_id != 0 {
                continue;
            }

            let mut amount = power.mana_cost;
            if power.power_cost_pct != 0.0 {
                if power.power_type == PowerType::Mana as i8 {
                    amount +=
                        calculate_pct_i32_like_cpp(caster_create_mana.max(0), power.power_cost_pct);
                } else {
                    continue;
                }
            }

            Self::push_power_cost_like_cpp(&mut costs, power.power_type, amount);
        }

        costs
    }

    fn push_power_cost_like_cpp(
        costs: &mut Vec<SpellPowerCostLikeCpp>,
        power_type: i8,
        amount: i32,
    ) {
        if amount == 0 {
            return;
        }

        if let Some(existing) = costs.iter_mut().find(|cost| cost.power_type == power_type) {
            existing.amount = existing.amount.saturating_add(amount);
        } else {
            costs.push(SpellPowerCostLikeCpp { power_type, amount });
        }
    }

    pub fn normalized_implicit_target_effect_mask_like_cpp(&self, mut effect_mask: u32) -> u32 {
        let original_mask = effect_mask;
        for effect in &self.effects {
            let bit = 1u32.checked_shl(effect.effect_index).unwrap_or(0);
            if bit == 0 || (original_mask & bit) == 0 {
                continue;
            }

            if !effect.accepts_implicit_target_conditions_like_cpp() {
                effect_mask &= !bit;
            }
        }
        effect_mask
    }
}

impl SpellEffectInfo {
    pub fn is_aura_like_cpp(&self) -> bool {
        use spell_effect_types::*;
        matches!(
            self.effect,
            SPELL_EFFECT_APPLY_AURA
                | SPELL_EFFECT_APPLY_AREA_AURA_PARTY
                | SPELL_EFFECT_APPLY_AREA_AURA_RAID
                | SPELL_EFFECT_APPLY_AREA_AURA_FRIEND
                | SPELL_EFFECT_APPLY_AREA_AURA_ENEMY
                | SPELL_EFFECT_APPLY_AREA_AURA_PET
                | SPELL_EFFECT_APPLY_AREA_AURA_OWNER
                | SPELL_EFFECT_APPLY_AURA_ON_PET
                | SPELL_EFFECT_APPLY_AREA_AURA_SUMMONS
                | SPELL_EFFECT_APPLY_AREA_AURA_PARTY_NONRANDOM
        )
    }

    pub fn calc_value_no_caster_with_die_roll_like_cpp<F>(&self, mut roll_die: F) -> i32
    where
        F: FnMut(i32, i32) -> i32,
    {
        let mut value = f64::from(self.effect_base_points);
        match self.effect_die_sides {
            0 => {}
            1 => value += 1.0,
            die_sides if die_sides > 1 => value += f64::from(roll_die(1, die_sides)),
            die_sides => value += f64::from(roll_die(die_sides, 1)),
        }
        value.round() as i32
    }

    pub fn calc_value_no_caster_like_cpp(&self) -> i32 {
        use rand::Rng;

        self.calc_value_no_caster_with_die_roll_like_cpp(|min, max| {
            rand::thread_rng().gen_range(min..=max)
        })
    }

    pub fn is_mounted_aura_like_cpp(&self) -> bool {
        self.effect == spell_effect_types::SPELL_EFFECT_APPLY_AURA
            && self.effect_aura == aura_types::SPELL_AURA_MOUNTED
    }

    pub fn is_mod_shapeshift_aura_like_cpp(&self) -> bool {
        self.effect == spell_effect_types::SPELL_EFFECT_APPLY_AURA
            && self.effect_aura == aura_types::SPELL_AURA_MOD_SHAPESHIFT
    }

    pub fn is_provide_spell_focus_aura_like_cpp(&self) -> bool {
        self.effect == spell_effect_types::SPELL_EFFECT_APPLY_AURA
            && self.effect_aura == aura_types::SPELL_AURA_PROVIDE_SPELL_FOCUS
    }

    pub fn is_battle_pet_xp_pct_aura_like_cpp(&self) -> bool {
        self.effect == spell_effect_types::SPELL_EFFECT_APPLY_AURA
            && self.effect_aura == aura_types::SPELL_AURA_MOD_BATTLE_PET_XP_PCT
    }

    pub fn has_focus_destination_implicit_target_like_cpp(&self) -> bool {
        matches!(
            self.implicit_target_1,
            implicit_targets::TARGET_DEST_NEARBY_ENTRY
                | implicit_targets::TARGET_DEST_NEARBY_ENTRY_2
                | implicit_targets::TARGET_DEST_NEARBY_ENTRY_OR_DB
        ) || matches!(
            self.implicit_target_2,
            implicit_targets::TARGET_DEST_NEARBY_ENTRY
                | implicit_targets::TARGET_DEST_NEARBY_ENTRY_2
                | implicit_targets::TARGET_DEST_NEARBY_ENTRY_OR_DB
        )
    }

    pub fn accepts_implicit_target_conditions_like_cpp(&self) -> bool {
        self.chain_targets > 0
            || implicit_target_category_accepts_conditions_like_cpp(self.implicit_target_1)
            || implicit_target_category_accepts_conditions_like_cpp(self.implicit_target_2)
            || spell_effect_accepts_implicit_target_conditions_like_cpp(self.effect)
    }

    pub fn has_spell_target_position_target_like_cpp(&self) -> bool {
        matches!(
            self.implicit_target_1,
            implicit_targets::TARGET_DEST_DB | implicit_targets::TARGET_DEST_NEARBY_ENTRY_OR_DB
        ) || matches!(
            self.implicit_target_2,
            implicit_targets::TARGET_DEST_DB | implicit_targets::TARGET_DEST_NEARBY_ENTRY_OR_DB
        )
    }
}

/// In-memory store of all spells loaded from DB2 or hotfixes database.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct SpellInterruptRowLikeCpp {
    pub(super) key: (i32, u8),
    pub(super) flags: ([u32; 2], [u32; 2]),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct SpellHitEffectMechanicRowLikeCpp {
    pub(super) record_id: u32,
    pub(super) mechanic: i32,
}
