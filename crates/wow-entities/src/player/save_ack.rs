//! Confirmation of a deferred save, not a second mutable Player.
//!
//! C++ Player::_SaveSpells/_SaveSkills/_SaveEquipmentSets and ReputationMgr::SaveToDB
//! consume the rows visited while preparing the transaction. Rust retains dirty state
//! until confirmed COMMIT (#169), so it must reconcile only that captured projection.
//! The application binds this single-use receipt to a generation-checked PlayerHandle.

use super::*;
use crate::{PlayerCurrency, PlayerCurrencyState, PlayerQuestStatusSaveTypeLikeCpp};

/// One objective value read using `_SaveQuestStatus`'s pinned native getter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerQuestObjectiveSaveValueLikeCpp {
    pub storage_index: u8,
    pub is_storing_flag: bool,
    pub value: u16,
}

/// Native update-field values that the selected status row's SQL projection reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerQuestNativeSaveProjectionLikeCpp {
    pub slot: u8,
    pub end_time_secs: i64,
    pub objectives: Vec<PlayerQuestObjectiveSaveValueLikeCpp>,
}

/// The immutable native projection selection tied to one dirty quest-status key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerQuestStatusSaveProjectionLikeCpp {
    pub quest_id: u32,
    pub native: Option<PlayerQuestNativeSaveProjectionLikeCpp>,
}

#[derive(Debug)]
struct SavedQuestStatusLikeCpp {
    quest_id: u32,
    save_type: PlayerQuestStatusSaveTypeLikeCpp,
    status: Option<PlayerQuestStatusRecord>,
    native_projection: Option<PlayerQuestNativeSaveProjectionLikeCpp>,
}

/// Owned, short-lived acknowledgement data. No ECS/SQL/Session dependency or whole Player copy.
/// Consumed exactly once by the application after confirmed commit for its exact incarnation.
#[derive(Debug)]
pub struct PlayerSaveAcknowledgementLikeCpp {
    guid: ObjectGuid,
    deferred_save: super::deferred_save::DeferredPlayerSave,
    resurrection: Option<crate::PlayerResurrectionContinuation>,
    spells: BTreeMap<i32, PlayerKnownSpellRecord>,
    fallback: BTreeMap<i32, PlayerKnownSpellRecord>,
    skills: Vec<PlayerSkillRecord>,
    equipment: BTreeMap<u64, PlayerEquipmentSetLikeCpp>,
    reputations: Vec<PlayerFactionStateLikeCpp>,
    inventory_items: Vec<(ObjectGuid, Item)>,
    currencies: Vec<(u32, PlayerCurrency)>,
    quest_statuses: Vec<SavedQuestStatusLikeCpp>,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct PlayerSavedGroupsLikeCpp {
    pub spells: bool,
    pub fallback_spells: bool,
    pub skills: bool,
    pub equipment: bool,
    pub reputations: bool,
    pub inventory_items: bool,
    /// Internal receipt proof for no-pickup resurrection costs; never acknowledges Items.
    pub resurrection_inventory_covered: bool,
    pub currencies: bool,
    pub quest_statuses: bool,
}

impl Player {
    /// Must run in the same admitted owner read as the complete save projection.
    pub fn capture_save_acknowledgement_like_cpp(&self) -> PlayerSaveAcknowledgementLikeCpp {
        self.capture_save_acknowledgement_with_quest_status_like_cpp(&[], &[], &[])
    }

    /// Capture only the item and currency rows selected for the same full-save
    /// request. These immutable values let the later receipt acknowledge the
    /// exact state written without clearing newer mutations.
    pub fn capture_save_acknowledgement_with_inventory_like_cpp(
        &self,
        item_guids: &[ObjectGuid],
        currency_ids: &[u32],
    ) -> PlayerSaveAcknowledgementLikeCpp {
        self.capture_save_acknowledgement_with_quest_status_like_cpp(item_guids, currency_ids, &[])
    }

    /// Capture quest status and native values selected by the same full-save projection.
    pub fn capture_save_acknowledgement_with_quest_status_like_cpp(
        &self,
        item_guids: &[ObjectGuid],
        currency_ids: &[u32],
        quest_statuses: &[PlayerQuestStatusSaveProjectionLikeCpp],
    ) -> PlayerSaveAcknowledgementLikeCpp {
        let state = self.gameplay_state();
        PlayerSaveAcknowledgementLikeCpp {
            guid: self.guid(),
            deferred_save: self.deferred_save,
            resurrection: self
                .resurrection_state_like_cpp()
                .delayed_after_teleport
                .filter(|continuation| {
                    continuation.phase == crate::PlayerResurrectionPhase::SavingPlayer
                }),
            spells: state.spells.rows.clone(),
            fallback: state.spells.fallback_rows.clone(),
            skills: state.skills.clone(),
            equipment: state.equipment_sets.snapshot_like_cpp(),
            // C++ FactionStateList is keyed by RepListID (ReputationMgr.h:63)
            // and the Player now owns it in that shape, so a shadowed duplicate
            // row can no longer exist to be acknowledged (#735).
            reputations: state.reputation.factions_like_cpp().cloned().collect(),
            inventory_items: item_guids
                .iter()
                .filter_map(|guid| {
                    self.inventory_runtime_like_cpp()
                        .item_objects()
                        .get(guid)
                        .cloned()
                        .map(|item| (*guid, item))
                })
                .collect(),
            currencies: currency_ids
                .iter()
                .filter_map(|id| state.currencies.get(id).copied().map(|row| (*id, row)))
                .collect(),
            quest_statuses: quest_statuses
                .iter()
                .filter_map(|projection| {
                    Some(SavedQuestStatusLikeCpp {
                        quest_id: projection.quest_id,
                        save_type: state
                            .quests
                            .status_save_type_like_cpp(projection.quest_id)?,
                        status: state.quests.status_like_cpp(projection.quest_id).cloned(),
                        native_projection: projection.native.clone(),
                    })
                })
                .collect(),
        }
    }

    pub fn acknowledge_saved_projection_like_cpp(
        &mut self,
        saved: PlayerSaveAcknowledgementLikeCpp,
        groups: PlayerSavedGroupsLikeCpp,
    ) {
        if self.guid() != saved.guid {
            return;
        }
        self.deferred_save.acknowledge(saved.deferred_save);
        let native_quest_rows = self.quest_log_values_like_cpp();
        let state = &mut self.gameplay_state;
        if let Some(captured) = saved.resurrection
            && captured.phase == crate::PlayerResurrectionPhase::SavingPlayer
            && (groups.inventory_items
                || (groups.resurrection_inventory_covered
                    && matches!(
                        captured.operation,
                        crate::PlayerResurrectionOperation::ReclaimCorpse { .. }
                            | crate::PlayerResurrectionOperation::SpiritHealer { .. }
                            | crate::PlayerResurrectionOperation::RequestData(
                                crate::PlayerResurrectionRequestLikeCpp { aura: 0, .. }
                            )
                    )))
            && groups.currencies
            && let Some(current) = state.resurrection.delayed_after_teleport.as_mut()
            && *current == captured
        {
            current.phase = crate::PlayerResurrectionPhase::PlayerSaveApplied;
        }
        if groups.spells && state.spells.rows_loaded && state.spells.rows_complete {
            acknowledge_spells(&mut state.spells, saved.spells);
        }
        if groups.fallback_spells {
            for (id, row) in saved.fallback {
                if state.spells.fallback_rows.get(&id) == Some(&row) {
                    state.spells.fallback_rows.remove(&id);
                }
            }
        }
        if groups.skills && state.skills_complete {
            for current in &mut state.skills {
                if saved.skills.iter().any(|row| row == current) {
                    if current.state == PlayerSkillLoadState::Deleted {
                        if let Ok(id) = u16::try_from(current.skill_line_id) {
                            state.non_durable_skill_tombstones.insert(id);
                        }
                    }
                    current.state = PlayerSkillLoadState::Unchanged;
                }
            }
        }
        if groups.equipment && state.equipment_sets.is_loaded_like_cpp() {
            state
                .equipment_sets
                .acknowledge_saved_sets_like_cpp(saved.equipment);
        }
        if groups.reputations {
            for current in state.reputation.factions_mut_like_cpp() {
                if saved.reputations.iter().any(|row| {
                    row.need_save
                        && row.faction_id == current.faction_id
                        && row.reputation_list_id == current.reputation_list_id
                        && row.standing == current.standing
                        && row.flags == current.flags
                }) {
                    // Send/visual state is independent of durable standing/flags.
                    current.need_save = false;
                }
            }
        }
        if groups.inventory_items {
            for (guid, captured) in saved.inventory_items {
                let Some(current) = self
                    .inventory_runtime_like_cpp()
                    .item_objects()
                    .get(&guid)
                    .filter(|current| current.has_same_persistent_save_state_like_cpp(&captured))
                    .map(|current| current.update_state())
                else {
                    if let Some(current) = self
                        .inventory_runtime_mut_like_cpp()
                        .item_objects_mut()
                        .get_mut(&guid)
                        && captured.update_state() == ItemUpdateState::New
                        && current.update_state() == ItemUpdateState::New
                    {
                        current.force_state(ItemUpdateState::Changed);
                    }
                    continue;
                };
                if current == ItemUpdateState::Removed {
                    self.inventory_runtime_mut_like_cpp()
                        .remove_item_object_like_cpp(guid);
                } else {
                    self.inventory_runtime_mut_like_cpp()
                        .item_objects_mut()
                        .get_mut(&guid)
                        .expect("captured inventory item remains owned")
                        .set_state(ItemUpdateState::Unchanged);
                }
            }
        }
        if groups.currencies {
            for (currency_id, captured) in saved.currencies {
                if let Some(current) = self
                    .gameplay_state_mut()
                    .currencies
                    .get_mut(&currency_id)
                    .filter(|current| **current == captured)
                    && matches!(
                        current.state,
                        PlayerCurrencyState::New | PlayerCurrencyState::Changed
                    )
                {
                    current.state = PlayerCurrencyState::Unchanged;
                }
            }
        }
        if groups.quest_statuses {
            let quests = &mut self.gameplay_state.quests;
            for captured in saved.quest_statuses {
                if quests.status_save_type_like_cpp(captured.quest_id) != Some(captured.save_type)
                    || quests.status_like_cpp(captured.quest_id) != captured.status.as_ref()
                {
                    continue;
                }
                let native_matches = captured
                    .native_projection
                    .as_ref()
                    .is_none_or(|projection| {
                        let Some(row) = native_quest_rows.get(usize::from(projection.slot)) else {
                            return false;
                        };
                        row.end_time() == projection.end_time_secs
                            && projection.objectives.iter().all(|objective| {
                                row.objective_data_for_save_like_cpp(
                                    objective.storage_index,
                                    objective.is_storing_flag,
                                ) == Some(objective.value)
                            })
                    });
                if native_matches {
                    quests.clear_status_save_if_like_cpp(captured.quest_id, captured.save_type);
                }
            }
        }
    }
}

fn acknowledge_spells(
    runtime: &mut PlayerSpellRuntimeState,
    saved: BTreeMap<i32, PlayerKnownSpellRecord>,
) {
    use PlayerSpellLoadState::*;
    for (id, row) in saved {
        if row.state == Temporary {
            continue;
        }
        match runtime.rows.get_mut(&id) {
            Some(current) if *current == row => {
                if row.state == Removed {
                    runtime.rows.remove(&id);
                    runtime.removed_known_spells.remove(&id);
                } else {
                    current.state = Unchanged;
                }
            }
            Some(current) if current.state != Temporary => {
                // A confirmed INSERT is now durable even if this row was changed
                // while awaiting it. Retrying NEW would duplicate the primary key.
                // A confirmed DELETE instead requires INSERT for a later relearn.
                current.state = match (row.state, current.state) {
                    (Removed, Removed) => Removed,
                    (Removed, _) => New,
                    (_, Removed) => Removed,
                    (New | Changed | Unchanged, _) => Changed,
                    (_, other) => other,
                };
            }
            None if row.state != Removed => {
                // Removing an in-flight NEW row can erase its in-memory entry.
                // The confirmed insert still needs a later durable delete.
                runtime.rows.insert(
                    id,
                    PlayerKnownSpellRecord {
                        state: Removed,
                        ..row
                    },
                );
            }
            _ => {}
        }
    }
    runtime
        .trait_definition_ids
        .retain(|id, _| runtime.rows.contains_key(id));
    // Rebuild only derived indices, retaining later canonical changes/tombstones.
    runtime.dependent_known_spells = runtime
        .rows
        .values()
        .filter(|row| row.state != Removed && row.dependent)
        .map(|row| row.spell_id)
        .collect();
    runtime.favorite_known_spells = runtime
        .rows
        .values()
        .filter(|row| row.state != Removed && row.favorite)
        .map(|row| row.spell_id)
        .collect();
    runtime.known_spells = runtime
        .rows
        .values()
        .filter(|row| row.state != Removed && !row.disabled)
        .map(|row| row.spell_id)
        .collect();
}

#[cfg(test)]
mod tests;
