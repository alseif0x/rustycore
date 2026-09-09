//! Condition entry model and load reporting state definitions, part 3 of 3.
//!
//! Separated from the conditions.rs root under #638. Behaviour is preserved.

use super::*;

pub(super) fn validate_object_entry_guid_type_like_cpp(
    condition: &Condition,
) -> Result<(), ConditionTypeValidationErrorLikeCpp> {
    match TypeId::from_u32(condition.condition_value1) {
        Some(TypeId::Unit | TypeId::GameObject | TypeId::Player | TypeId::Corpse) => Ok(()),
        _ => Err(ConditionTypeValidationErrorLikeCpp::InvalidObjectTypeId(
            condition.condition_value1,
        )),
    }
}

pub(super) fn validate_type_mask_like_cpp(
    condition: &Condition,
) -> Result<(), ConditionTypeValidationErrorLikeCpp> {
    let allowed_mask =
        (TypeMask::UNIT | TypeMask::PLAYER | TypeMask::GAME_OBJECT | TypeMask::CORPSE).bits();
    if condition.condition_value1 == 0 || condition.condition_value1 & !allowed_mask != 0 {
        return Err(ConditionTypeValidationErrorLikeCpp::InvalidTypeMask(
            condition.condition_value1,
        ));
    }

    Ok(())
}

pub(super) fn validate_target_selector_like_cpp(
    condition: &Condition,
    field: u8,
) -> Result<(), ConditionTypeValidationErrorLikeCpp> {
    let value = match field {
        1 => condition.condition_value1,
        _ => unreachable!("unsupported condition target selector field"),
    };
    let max = condition.max_available_condition_targets_like_cpp();

    if value >= max {
        return Err(ConditionTypeValidationErrorLikeCpp::InvalidTargetSelector {
            field,
            value,
            max,
        });
    }

    if value == u32::from(condition.condition_target) {
        return Err(ConditionTypeValidationErrorLikeCpp::SelfTargetSelector { field, value });
    }

    Ok(())
}

pub type ConditionContainer = Vec<Condition>;

pub type ConditionsByEntryMap = HashMap<ConditionId, Arc<ConditionContainer>>;

#[derive(Debug, Clone, Default)]
pub struct ConditionsReference {
    pub(super) conditions: Weak<ConditionContainer>,
}

impl ConditionsReference {
    pub fn new(conditions: &Arc<ConditionContainer>) -> Self {
        Self {
            conditions: Arc::downgrade(conditions),
        }
    }

    pub fn upgrade(&self) -> Option<Arc<ConditionContainer>> {
        self.conditions.upgrade()
    }

    pub fn is_expired(&self) -> bool {
        self.conditions.strong_count() == 0
    }
}

#[derive(Debug, Clone, Default)]
pub struct ConditionEntriesByTypeStore {
    pub(super) entries: HashMap<ConditionSourceType, ConditionsByEntryMap>,
}

impl ConditionEntriesByTypeStore {
    pub fn from_conditions_like_cpp(conditions: impl IntoIterator<Item = Condition>) -> Self {
        let mut store = Self::default();
        for condition in conditions {
            store.add_condition_like_cpp(condition);
        }
        store
    }

    pub fn add_condition_like_cpp(&mut self, condition: Condition) {
        self.entries
            .entry(condition.source_type)
            .or_default()
            .entry(condition.id_like_cpp())
            .or_insert_with(|| Arc::new(Vec::new()));

        let bucket = self
            .entries
            .get_mut(&condition.source_type)
            .and_then(|by_id| by_id.get_mut(&condition.id_like_cpp()))
            .expect("condition bucket must exist after insertion");
        Arc::make_mut(bucket).push(condition);
    }

    pub fn conditions_for_like_cpp(
        &self,
        source_type: ConditionSourceType,
        id: ConditionId,
    ) -> Option<&Arc<ConditionContainer>> {
        self.entries
            .get(&source_type)
            .and_then(|by_id| by_id.get(&id))
    }

    pub fn entries_for_source_type_like_cpp(
        &self,
        source_type: ConditionSourceType,
    ) -> Option<&ConditionsByEntryMap> {
        self.entries.get(&source_type)
    }

    pub fn reference_for_like_cpp(
        &self,
        source_type: ConditionSourceType,
        id: ConditionId,
    ) -> Option<ConditionsReference> {
        self.conditions_for_like_cpp(source_type, id)
            .map(ConditionsReference::new)
    }

    pub fn bucket_count(&self) -> usize {
        self.entries.values().map(HashMap::len).sum()
    }

    pub fn condition_count(&self) -> usize {
        self.entries
            .values()
            .flat_map(HashMap::values)
            .map(|conditions| conditions.len())
            .sum()
    }

    /// C++ `SpellsUsedInSpellClickConditions` load-time index.
    pub fn spells_used_in_spell_click_conditions_like_cpp(&self) -> std::collections::HashSet<u32> {
        self.entries_for_source_type_like_cpp(ConditionSourceType::SpellClickEvent)
            .into_iter()
            .flat_map(HashMap::values)
            .flat_map(|conditions| conditions.iter())
            .filter(|condition| condition.condition_type == ConditionType::Aura)
            .map(|condition| condition.condition_value1)
            .collect()
    }

    /// C++ `ConditionMgr::IsSpellUsedInSpellClickConditions`.
    pub fn is_spell_used_in_spell_click_conditions_like_cpp(&self, spell_id: u32) -> bool {
        self.entries_for_source_type_like_cpp(ConditionSourceType::SpellClickEvent)
            .into_iter()
            .flat_map(HashMap::values)
            .flat_map(|conditions| conditions.iter())
            .any(|condition| {
                condition.condition_type == ConditionType::Aura
                    && condition.condition_value1 == spell_id
            })
    }

    /// C++ `ConditionMgr::GetSearcherTypeMaskForConditionList`.
    pub fn get_searcher_type_mask_for_condition_list_like_cpp(
        &self,
        conditions: &[Condition],
    ) -> u32 {
        if conditions.is_empty() {
            return GRID_MAP_TYPE_MASK_ALL;
        }

        let mut else_group_searcher_type_masks = std::collections::BTreeMap::<u32, u32>::new();
        for condition in conditions {
            assert!(
                condition.is_loaded_like_cpp(),
                "ConditionMgr::GetSearcherTypeMaskForConditionList - not yet loaded condition found in list"
            );

            let group_mask = else_group_searcher_type_masks
                .entry(condition.else_group)
                .or_insert(GRID_MAP_TYPE_MASK_ALL);
            if *group_mask == 0 {
                continue;
            }

            if condition.reference_id != 0 {
                let reference_conditions = self
                    .conditions_for_like_cpp(
                        ConditionSourceType::ReferenceCondition,
                        ConditionId::new(condition.reference_id, 0, 0),
                    )
                    .expect(
                        "ConditionMgr::GetSearcherTypeMaskForConditionList - incorrect reference",
                    );
                *group_mask &= self.get_searcher_type_mask_for_condition_list_like_cpp(
                    reference_conditions.as_slice(),
                );
            } else {
                *group_mask &= condition.get_searcher_type_mask_for_condition_like_cpp();
            }
        }

        else_group_searcher_type_masks
            .values()
            .fold(0, |mask, group_mask| mask | group_mask)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConditionDbRowLikeCpp {
    pub source_type_or_reference_id: i32,
    pub source_group: u32,
    pub source_entry: i32,
    pub source_id: u32,
    pub else_group: u32,
    pub condition_type_or_reference: i32,
    pub condition_target: u8,
    pub condition_value1: u32,
    pub condition_value2: u32,
    pub condition_value3: u32,
    pub condition_string_value1: String,
    pub negative_condition: bool,
    pub error_type: u32,
    pub error_text_id: u32,
    pub script_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConditionRowSkipReason {
    SelfReference(i32),
    InvalidConditionType(i32),
    InvalidSourceType(i32),
    SourceGroupNotAllowed {
        source_type: ConditionSourceType,
        source_group: u32,
    },
    SourceIdNotAllowed {
        source_type: ConditionSourceType,
        source_id: u32,
    },
    ConditionTargetOutOfRange {
        source_type: ConditionSourceType,
        condition_target: u8,
        max_available_targets: u32,
    },
    ConditionTypeValidationFailed(ConditionTypeValidationErrorLikeCpp),
    ConditionSourceValidationFailed(ConditionSourceValidationErrorLikeCpp),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkippedConditionRow {
    pub row: ConditionDbRowLikeCpp,
    pub reason: ConditionRowSkipReason,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConditionLoadWarningLikeCpp {
    ReferenceUselessConditionTarget {
        source_type_or_reference_id: i32,
        condition_target: u8,
    },
    ReferenceUselessValue {
        source_type_or_reference_id: i32,
        field: u8,
        value: u32,
    },
    ReferenceUselessNegativeCondition {
        source_type_or_reference_id: i32,
    },
    ReferenceTemplateUselessSourceGroup {
        source_type_or_reference_id: i32,
        source_group: u32,
    },
    ReferenceTemplateUselessSourceEntry {
        source_type_or_reference_id: i32,
        source_entry: i32,
    },
    ReferenceTemplateUselessSourceId {
        source_type_or_reference_id: i32,
        source_id: u32,
    },
    ErrorTypeResetForNonSpell {
        source_type: ConditionSourceType,
        error_type: u32,
    },
    ErrorTextIdResetWithoutErrorType {
        source_type: ConditionSourceType,
        error_text_id: u32,
    },
    UselessConditionValue {
        condition_type: ConditionType,
        field: u8,
        value: u32,
    },
    UselessConditionStringValue {
        condition_type: ConditionType,
        field: u8,
        value: String,
    },
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConditionLoadReport {
    pub conditions: Vec<Condition>,
    pub skipped: Vec<SkippedConditionRow>,
    pub warnings: Vec<ConditionLoadWarningLikeCpp>,
}

impl ConditionLoadReport {
    pub fn parsed_count(&self) -> usize {
        self.conditions.len()
    }

    pub fn into_store_like_cpp(self) -> ConditionEntriesByTypeStore {
        ConditionEntriesByTypeStore::from_conditions_like_cpp(self.conditions)
    }
}

/// C++ `ConditionMgr::LoadConditions` row-to-`Condition` conversion before source/type validation.
pub fn parse_condition_row_like_cpp(
    row: ConditionDbRowLikeCpp,
    mut script_id_for_name: impl FnMut(&str) -> u32,
) -> Result<Condition, SkippedConditionRow> {
    let mut condition = Condition {
        source_group: row.source_group,
        source_entry: row.source_entry,
        source_id: row.source_id,
        else_group: row.else_group,
        condition_target: row.condition_target,
        condition_value1: row.condition_value1,
        condition_value2: row.condition_value2,
        condition_value3: row.condition_value3,
        condition_string_value1: row.condition_string_value1.clone(),
        negative_condition: row.negative_condition,
        error_type: row.error_type,
        error_text_id: row.error_text_id,
        script_id: script_id_for_name(&row.script_name),
        ..Condition::default()
    };

    if row.condition_type_or_reference >= 0 {
        condition.condition_type = ConditionType::from_i32(row.condition_type_or_reference)
            .filter(|condition_type| *condition_type != ConditionType::Max)
            .ok_or_else(|| SkippedConditionRow {
                row: row.clone(),
                reason: ConditionRowSkipReason::InvalidConditionType(
                    row.condition_type_or_reference,
                ),
            })?;
    }

    if row.source_type_or_reference_id >= 0 {
        condition.source_type = ConditionSourceType::from_i32(row.source_type_or_reference_id)
            .ok_or_else(|| SkippedConditionRow {
                row: row.clone(),
                reason: ConditionRowSkipReason::InvalidSourceType(row.source_type_or_reference_id),
            })?;
    }

    if row.condition_type_or_reference < 0 {
        if row.condition_type_or_reference == row.source_type_or_reference_id {
            return Err(SkippedConditionRow {
                row: row.clone(),
                reason: ConditionRowSkipReason::SelfReference(row.source_type_or_reference_id),
            });
        }

        condition.reference_id = u32::try_from(-row.condition_type_or_reference).unwrap_or(0);
    }

    if row.source_type_or_reference_id < 0 {
        condition.source_type = ConditionSourceType::ReferenceCondition;
        condition.source_group = u32::try_from(-row.source_type_or_reference_id).unwrap_or(0);
    }

    Ok(condition)
}

pub fn condition_load_warnings_like_cpp(
    row: &ConditionDbRowLikeCpp,
) -> Vec<ConditionLoadWarningLikeCpp> {
    let mut warnings = Vec::new();

    if row.condition_type_or_reference < 0 {
        if row.condition_target != 0 {
            warnings.push(
                ConditionLoadWarningLikeCpp::ReferenceUselessConditionTarget {
                    source_type_or_reference_id: row.source_type_or_reference_id,
                    condition_target: row.condition_target,
                },
            );
        }
        if row.condition_value1 != 0 {
            warnings.push(ConditionLoadWarningLikeCpp::ReferenceUselessValue {
                source_type_or_reference_id: row.source_type_or_reference_id,
                field: 1,
                value: row.condition_value1,
            });
        }
        if row.condition_value2 != 0 {
            warnings.push(ConditionLoadWarningLikeCpp::ReferenceUselessValue {
                source_type_or_reference_id: row.source_type_or_reference_id,
                field: 2,
                value: row.condition_value2,
            });
        }
        if row.condition_value3 != 0 {
            warnings.push(ConditionLoadWarningLikeCpp::ReferenceUselessValue {
                source_type_or_reference_id: row.source_type_or_reference_id,
                field: 3,
                value: row.condition_value3,
            });
        }
        if row.negative_condition {
            warnings.push(
                ConditionLoadWarningLikeCpp::ReferenceUselessNegativeCondition {
                    source_type_or_reference_id: row.source_type_or_reference_id,
                },
            );
        }
    }

    if row.source_type_or_reference_id < 0 {
        if row.source_group != 0 {
            warnings.push(
                ConditionLoadWarningLikeCpp::ReferenceTemplateUselessSourceGroup {
                    source_type_or_reference_id: row.source_type_or_reference_id,
                    source_group: row.source_group,
                },
            );
        }
        if row.source_entry != 0 {
            warnings.push(
                ConditionLoadWarningLikeCpp::ReferenceTemplateUselessSourceEntry {
                    source_type_or_reference_id: row.source_type_or_reference_id,
                    source_entry: row.source_entry,
                },
            );
        }
        if row.source_id != 0 {
            warnings.push(
                ConditionLoadWarningLikeCpp::ReferenceTemplateUselessSourceId {
                    source_type_or_reference_id: row.source_type_or_reference_id,
                    source_id: row.source_id,
                },
            );
        }
    }

    warnings
}

pub(super) fn condition_normalization_warnings_like_cpp(
    condition: &Condition,
) -> Vec<ConditionLoadWarningLikeCpp> {
    let mut warnings = Vec::new();

    if condition.error_type != 0 && condition.source_type != ConditionSourceType::Spell {
        warnings.push(ConditionLoadWarningLikeCpp::ErrorTypeResetForNonSpell {
            source_type: condition.source_type,
            error_type: condition.error_type,
        });
    }

    if condition.error_text_id != 0
        && (condition.error_type == 0 || condition.source_type != ConditionSourceType::Spell)
    {
        warnings.push(
            ConditionLoadWarningLikeCpp::ErrorTextIdResetWithoutErrorType {
                source_type: condition.source_type,
                error_text_id: condition.error_text_id,
            },
        );
    }

    warnings
}

pub(super) fn condition_type_useless_value_warnings_like_cpp(
    condition: &Condition,
) -> Vec<ConditionLoadWarningLikeCpp> {
    useless_condition_value_fields_like_cpp(condition)
        .into_iter()
        .map(|field| match field {
            1 => ConditionLoadWarningLikeCpp::UselessConditionValue {
                condition_type: condition.condition_type,
                field,
                value: condition.condition_value1,
            },
            2 => ConditionLoadWarningLikeCpp::UselessConditionValue {
                condition_type: condition.condition_type,
                field,
                value: condition.condition_value2,
            },
            3 => ConditionLoadWarningLikeCpp::UselessConditionValue {
                condition_type: condition.condition_type,
                field,
                value: condition.condition_value3,
            },
            4 => ConditionLoadWarningLikeCpp::UselessConditionStringValue {
                condition_type: condition.condition_type,
                field,
                value: condition.condition_string_value1.clone(),
            },
            _ => unreachable!("condition value field must be 1..=4"),
        })
        .collect()
}

pub fn normalize_loaded_condition_shape_like_cpp(
    condition: &mut Condition,
) -> Result<(), ConditionRowSkipReason> {
    normalize_loaded_condition_shape_inner_like_cpp(condition, false)
}

pub(super) fn normalize_loaded_condition_shape_for_row_like_cpp(
    condition: &mut Condition,
    row: &ConditionDbRowLikeCpp,
) -> Result<(), ConditionRowSkipReason> {
    normalize_loaded_condition_shape_inner_like_cpp(condition, row.source_type_or_reference_id < 0)
}

pub(super) fn normalize_loaded_condition_shape_inner_like_cpp(
    condition: &mut Condition,
    is_reference_template: bool,
) -> Result<(), ConditionRowSkipReason> {
    if condition.reference_id == 0 {
        validate_condition_type_static_like_cpp(condition)
            .map_err(ConditionRowSkipReason::ConditionTypeValidationFailed)?;

        let max_available_targets = condition.max_available_condition_targets_like_cpp();
        if u32::from(condition.condition_target) >= max_available_targets {
            return Err(ConditionRowSkipReason::ConditionTargetOutOfRange {
                source_type: condition.source_type,
                condition_target: condition.condition_target,
                max_available_targets,
            });
        }
    }

    if !is_reference_template {
        validate_condition_source_static_like_cpp(condition)
            .map_err(ConditionRowSkipReason::ConditionSourceValidationFailed)?;
    }

    if condition.source_group != 0
        && !condition_source_can_have_group_set_like_cpp(condition.source_type)
    {
        return Err(ConditionRowSkipReason::SourceGroupNotAllowed {
            source_type: condition.source_type,
            source_group: condition.source_group,
        });
    }

    if condition.source_id != 0 && !condition_source_can_have_id_set_like_cpp(condition.source_type)
    {
        return Err(ConditionRowSkipReason::SourceIdNotAllowed {
            source_type: condition.source_type,
            source_id: condition.source_id,
        });
    }

    if condition.error_type != 0 && condition.source_type != ConditionSourceType::Spell {
        condition.error_type = 0;
    }

    if condition.error_text_id != 0 && condition.error_type == 0 {
        condition.error_text_id = 0;
    }

    Ok(())
}

pub fn parse_condition_rows_like_cpp(
    rows: impl IntoIterator<Item = ConditionDbRowLikeCpp>,
    mut script_id_for_name: impl FnMut(&str) -> u32,
) -> ConditionLoadReport {
    let mut report = ConditionLoadReport::default();
    for row in rows {
        report
            .warnings
            .extend(condition_load_warnings_like_cpp(&row));
        match parse_condition_row_like_cpp(row.clone(), &mut script_id_for_name) {
            Ok(mut condition) => {
                report
                    .warnings
                    .extend(condition_normalization_warnings_like_cpp(&condition));
                match normalize_loaded_condition_shape_for_row_like_cpp(&mut condition, &row) {
                    Ok(()) => {
                        if condition.reference_id == 0 {
                            report
                                .warnings
                                .extend(condition_type_useless_value_warnings_like_cpp(&condition));
                        }
                        report.conditions.push(condition);
                    }
                    Err(reason) => report.skipped.push(SkippedConditionRow { row, reason }),
                }
            }
            Err(skipped) => report.skipped.push(skipped),
        }
    }
    report
}
