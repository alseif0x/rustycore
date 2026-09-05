//! Confirmation of a deferred save, not a second mutable Player.
//!
//! C++ Player::_SaveSpells/_SaveSkills/_SaveEquipmentSets and ReputationMgr::SaveToDB
//! consume the rows visited while preparing the transaction. Rust retains dirty state
//! until confirmed COMMIT (#169), so it must reconcile only that captured projection.
//! The application binds this single-use receipt to a generation-checked PlayerHandle.

use super::*;

/// Owned, short-lived acknowledgement data. No ECS/SQL/Session dependency or whole Player copy.
/// Consumed exactly once by the application after confirmed commit for its exact incarnation.
#[derive(Debug)]
pub struct PlayerSaveAcknowledgementLikeCpp {
    guid: ObjectGuid,
    spells: BTreeMap<i32, PlayerKnownSpellRecord>,
    fallback: BTreeMap<i32, PlayerKnownSpellRecord>,
    skills: Vec<PlayerSkillRecord>,
    equipment: BTreeMap<u64, PlayerEquipmentSetLikeCpp>,
    reputations: Vec<PlayerReputationRecord>,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct PlayerSavedGroupsLikeCpp {
    pub spells: bool,
    pub fallback_spells: bool,
    pub skills: bool,
    pub equipment: bool,
    pub reputations: bool,
}

impl Player {
    /// Must run in the same admitted owner read as the complete save projection.
    pub fn capture_save_acknowledgement_like_cpp(&self) -> PlayerSaveAcknowledgementLikeCpp {
        let state = self.gameplay_state();
        PlayerSaveAcknowledgementLikeCpp {
            guid: self.guid(),
            spells: state.spells.rows.clone(),
            fallback: state.spells.fallback_rows.clone(),
            skills: state.skills.clone(),
            equipment: state.equipment_sets.clone(),
            reputations: state.reputations.clone(),
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
        let state = &mut self.gameplay_state;
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
        if groups.equipment && state.equipment_sets_loaded {
            acknowledge_equipment(&mut state.equipment_sets, saved.equipment);
        }
        if groups.reputations {
            for current in &mut state.reputations {
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

fn acknowledge_equipment(
    current_sets: &mut BTreeMap<u64, PlayerEquipmentSetLikeCpp>,
    saved: BTreeMap<u64, PlayerEquipmentSetLikeCpp>,
) {
    use PlayerEquipmentSetUpdateStateLikeCpp::*;
    for (id, row) in saved {
        match current_sets.get_mut(&id) {
            Some(current) if *current == row => {
                if row.state == Deleted {
                    current_sets.remove(&id);
                } else {
                    current.state = Unchanged;
                }
            }
            Some(current) => {
                current.state = match (row.state, current.state) {
                    (Deleted, Deleted) => Deleted,
                    (Deleted, _) => New,
                    (_, Deleted) => Deleted,
                    (New | Changed | Unchanged, _) => Changed,
                };
            }
            None if row.state != Deleted => {
                current_sets.insert(
                    id,
                    PlayerEquipmentSetLikeCpp {
                        state: Deleted,
                        ..row
                    },
                );
            }
            None => {}
        }
    }
}

#[cfg(test)]
mod tests;
