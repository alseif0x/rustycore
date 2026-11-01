//! Difficulty-aware SpellInfo component and SpellMisc payload readers.

use super::*;
use std::collections::{BTreeSet, HashSet};

pub(super) fn resolve_component_payload_like_cpp<T: Clone>(
    requested_difficulty_id: u8,
    difficulty_store: Option<&crate::difficulty::DifficultyStore>,
    mut exact_value: impl FnMut(u8) -> Option<T>,
) -> Result<Option<(u8, T)>, crate::spell::DataSpellCatalogResolutionError> {
    if requested_difficulty_id == 0 {
        return Ok(exact_value(0).map(|value| (0, value)));
    }
    crate::spell::resolve_spell_catalog_value(
        requested_difficulty_id,
        difficulty_store,
        true,
        |difficulty_id| match exact_value(difficulty_id) {
            Some(value) => Some(Some((difficulty_id, value))),
            None if difficulty_id == 0 => Some(None),
            None => None,
        },
    )
    .map(Option::flatten)
}

impl SpellStore {
    /// C++ `SpellInfo::HasHitDelay` depends on the selected `SpellMisc` row's
    /// launch delay and speed, independently of cast time.
    pub fn has_hit_delay_at_difficulty_like_cpp(
        &self,
        spell_id: u32,
        requested_difficulty_id: u8,
        difficulties: Option<&crate::difficulty::DifficultyStore>,
    ) -> Result<bool, SpellCategoryReadError> {
        if !self.spell_misc_loaded {
            return Err(SpellCategoryReadError::MetadataUnavailable);
        }
        let spell_key = spell_id as i32;
        let has_any_row = self
            .spell_hit_misc_by_difficulty
            .keys()
            .any(|(candidate_id, _)| *candidate_id == spell_key);
        let misc = crate::spell::resolve_spell_catalog_value(
            requested_difficulty_id,
            difficulties,
            has_any_row,
            |difficulty_id| {
                self.spell_hit_misc_by_difficulty
                    .get(&(spell_key, difficulty_id))
                    .map(|row| (row.speed, row.launch_delay))
            },
        )
        .map_err(SpellCategoryReadError::Catalog)?;
        let Some((speed, launch_delay)) = misc else {
            // With the complete SpellMisc catalog loaded, C++ leaves the
            // SpellInfo defaults intact when no row exists for this spell.
            return Ok(false);
        };
        Ok(speed > 0.0 || launch_delay > 0.0)
    }
    pub(super) fn resolve_cast_time_payload_like_cpp(
        &self,
        spell_id: i32,
        requested_difficulty_id: u8,
        difficulty_store: Option<&crate::difficulty::DifficultyStore>,
    ) -> Result<u32, crate::spell::SpellInfoPayloadResolutionErrorLikeCpp> {
        use crate::spell::{
            SpellInfoPayloadComponentLikeCpp as Component,
            SpellInfoPayloadResolutionErrorLikeCpp as Error,
        };

        if !self.cast_time_payload_loaded_like_cpp {
            return Err(Error::AuthorityUnavailable(Component::CastTime));
        }
        let selected = resolve_component_payload_like_cpp(
            requested_difficulty_id,
            difficulty_store,
            |difficulty_id| {
                self.cast_times_by_difficulty_like_cpp
                    .get(&(spell_id, difficulty_id))
                    .copied()
            },
        )
        .map_err(Error::Catalog)?;
        match selected {
            Some((_, (_, Some(cast_time)))) => Ok(cast_time),
            // C++ keeps the selected SpellMisc row's null CastTimeEntry. Its
            // CalcCastTime therefore returns zero and does not inherit a
            // parent SpellMisc row.
            Some((_, (_, None))) => Ok(0),
            None => Ok(0),
        }
    }

    pub(super) fn resolve_cooldown_payload_like_cpp(
        &self,
        spell_id: i32,
        requested_difficulty_id: u8,
        difficulty_store: Option<&crate::difficulty::DifficultyStore>,
    ) -> Result<(u32, u32, u32), crate::spell::SpellInfoPayloadResolutionErrorLikeCpp> {
        use crate::spell::{
            SpellInfoPayloadComponentLikeCpp as Component,
            SpellInfoPayloadResolutionErrorLikeCpp as Error,
        };

        if !self.cooldown_payload_loaded_like_cpp {
            return Err(Error::AuthorityUnavailable(Component::Cooldowns));
        }
        let selected = resolve_component_payload_like_cpp(
            requested_difficulty_id,
            difficulty_store,
            |difficulty_id| {
                self.cooldowns_by_difficulty_like_cpp
                    .get(&(spell_id, difficulty_id))
                    .copied()
            },
        )
        .map_err(Error::Catalog)?;
        Ok(selected.map_or((0, 0, 0), |(_, value)| value))
    }

    /// The source cooldown history keeps recovery and category recovery
    /// separate even though SpellInfo::GetRecoveryTime exposes their maximum.
    pub fn cooldown_components_at_difficulty_like_cpp(
        &self,
        spell_id: i32,
        requested_difficulty_id: u8,
        difficulty_store: Option<&crate::difficulty::DifficultyStore>,
    ) -> Result<(u32, u32, u32), crate::spell::SpellInfoPayloadResolutionErrorLikeCpp> {
        self.resolve_cooldown_payload_like_cpp(spell_id, requested_difficulty_id, difficulty_store)
    }

    pub(super) fn resolve_power_cost_payload_like_cpp(
        &self,
        spell_id: i32,
        requested_difficulty_id: u8,
        difficulty_store: Option<&crate::difficulty::DifficultyStore>,
    ) -> Result<Vec<SpellPowerCostInfoLikeCpp>, crate::spell::SpellInfoPayloadResolutionErrorLikeCpp>
    {
        use crate::spell::{
            SpellInfoPayloadComponentLikeCpp as Component,
            SpellInfoPayloadResolutionErrorLikeCpp as Error,
        };

        if !self.power_cost_payload_loaded_like_cpp {
            return Err(Error::AuthorityUnavailable(Component::PowerCosts));
        }
        // Validate the selected chain even when this spell has no power rows;
        // otherwise a cyclic/incomplete difficulty catalog would look free.
        resolve_component_payload_like_cpp(requested_difficulty_id, difficulty_store, |_| {
            Option::<()>::None
        })
        .map_err(Error::Catalog)?;

        let mut order_indices = self
            .power_costs_by_difficulty_like_cpp
            .iter()
            .filter(|((candidate_spell, _), _)| *candidate_spell == spell_id)
            .flat_map(|(_, costs)| costs.iter().map(|cost| cost.order_index))
            .collect::<BTreeSet<_>>();
        let mut resolved = Vec::with_capacity(order_indices.len());
        while let Some(order_index) = order_indices.pop_first() {
            let selected = resolve_component_payload_like_cpp(
                requested_difficulty_id,
                difficulty_store,
                |difficulty_id| {
                    self.power_costs_by_difficulty_like_cpp
                        .get(&(spell_id, difficulty_id))
                        .and_then(|costs| {
                            costs
                                .iter()
                                .find(|cost| cost.order_index == order_index)
                                .cloned()
                        })
                },
            )
            .map_err(Error::Catalog)?;
            if let Some((_, cost)) = selected {
                resolved.push(cost);
            }
        }
        Ok(resolved)
    }

    /// Resolve the `SpellMisc` attributes owned by the same difficulty-specific
    /// C++ `SpellInfo` selected by `SpellMgr::GetSpellInfo`.
    pub fn misc_attributes_for_difficulty_like_cpp(
        &self,
        spell_id: i32,
        requested_difficulty_id: u8,
        difficulty_store: Option<&crate::difficulty::DifficultyStore>,
    ) -> Option<[u32; 15]> {
        let mut difficulty_id = requested_difficulty_id;
        let mut visited = HashSet::new();
        loop {
            if let Some(attributes) = self
                .spell_misc_attributes_by_difficulty
                .get(&(spell_id, difficulty_id))
                .copied()
            {
                return Some(attributes);
            }
            if difficulty_id == 0 || !visited.insert(difficulty_id) {
                break;
            }
            difficulty_id = difficulty_store
                .and_then(|store| store.get(u32::from(difficulty_id)))
                .map_or(0, |difficulty| difficulty.fallback_difficulty_id);
        }
        self.spell_misc_attributes.get(&spell_id).copied()
    }

    /// Resolve all SpellInfo attribute words from the selected SpellMisc
    /// component. A loaded catalog with no row has the C++ default words;
    /// an unavailable catalog or an incomplete difficulty chain is an error.
    pub fn misc_attribute_words_at_difficulty_like_cpp(
        &self,
        spell_id: i32,
        requested_difficulty_id: u8,
        difficulty_store: Option<&crate::difficulty::DifficultyStore>,
    ) -> Result<[u32; 15], crate::spell::SpellInfoPayloadResolutionErrorLikeCpp> {
        use crate::spell::{
            SpellInfoPayloadComponentLikeCpp as Component,
            SpellInfoPayloadResolutionErrorLikeCpp as Error,
        };

        if !self.spell_misc_loaded {
            return Err(Error::AuthorityUnavailable(Component::Misc));
        }
        let selected = resolve_component_payload_like_cpp(
            requested_difficulty_id,
            difficulty_store,
            |difficulty_id| {
                self.spell_misc_attributes_by_difficulty
                    .get(&(spell_id, difficulty_id))
                    .copied()
            },
        )
        .map_err(Error::Catalog)?;
        Ok(selected.map_or([0; 15], |(_, attributes)| attributes))
    }
    /// Read only the effective SpellInfo attributes hydrated for this exact
    /// difficulty, without applying a fallback or base-difficulty lookup.
    pub fn misc_attributes_for_exact_difficulty(
        &self,
        spell_id: i32,
        difficulty_id: u8,
    ) -> Option<[u32; 15]> {
        self.spell_misc_attributes_by_difficulty
            .get(&(spell_id, difficulty_id))
            .copied()
    }
    pub fn has_attribute_for_difficulty_like_cpp(
        &self,
        spell_id: i32,
        requested_difficulty_id: u8,
        difficulty_store: Option<&crate::difficulty::DifficultyStore>,
        attribute_word: usize,
        attribute: u32,
    ) -> bool {
        self.misc_attributes_for_difficulty_like_cpp(
            spell_id,
            requested_difficulty_id,
            difficulty_store,
        )
        .and_then(|attributes| attributes.get(attribute_word).copied())
        .is_some_and(|attributes| attributes & attribute != 0)
    }
    /// C++ `SpellInfo::HasAttribute` for attributes hydrated from `SpellMisc.db2`.
    pub fn has_attribute0_like_cpp(&self, spell_id: i32, attribute: u32) -> bool {
        self.spell_misc_attributes
            .get(&spell_id)
            .is_some_and(|attributes| attributes[0] & attribute != 0)
    }
    /// C++ `SpellInfo::HasAttribute(SpellAttr1)` for attributes hydrated from `SpellMisc.db2`.
    pub fn has_attribute1_like_cpp(&self, spell_id: i32, attribute: u32) -> bool {
        self.spell_misc_attributes
            .get(&spell_id)
            .is_some_and(|attributes| attributes[1] & attribute != 0)
    }
    /// C++ `SpellInfo::HasAttribute(SpellAttr2)` for attributes hydrated from `SpellMisc.db2`.
    pub fn has_attribute2_like_cpp(&self, spell_id: i32, attribute: u32) -> bool {
        self.spell_misc_attributes
            .get(&spell_id)
            .is_some_and(|attributes| attributes[2] & attribute != 0)
    }
    /// C++ `SpellInfo::HasAttribute(SpellAttr4)` for attributes hydrated from `SpellMisc.db2`.
    pub fn has_attribute4_like_cpp(&self, spell_id: i32, attribute: u32) -> bool {
        self.spell_misc_attributes
            .get(&spell_id)
            .is_some_and(|attributes| attributes[4] & attribute != 0)
    }
    /// C++ `SpellInfo::HasAttribute(SpellAttr8)` for attributes hydrated from `SpellMisc.db2`.
    pub fn has_attribute8_like_cpp(&self, spell_id: i32, attribute: u32) -> bool {
        self.spell_misc_attributes
            .get(&spell_id)
            .is_some_and(|attributes| attributes[8] & attribute != 0)
    }
}
