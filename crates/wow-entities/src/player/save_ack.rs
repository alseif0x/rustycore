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
    deferred_save: super::deferred_save::DeferredPlayerSave,
    spells: BTreeMap<i32, PlayerKnownSpellRecord>,
    fallback: BTreeMap<i32, PlayerKnownSpellRecord>,
    skills: Vec<PlayerSkillRecord>,
    equipment: BTreeMap<u64, PlayerEquipmentSetLikeCpp>,
    reputations: Vec<PlayerFactionStateLikeCpp>,
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
            deferred_save: self.deferred_save,
            spells: state.spells.rows.clone(),
            fallback: state.spells.fallback_rows.clone(),
            skills: state.skills.clone(),
            equipment: state.equipment_sets.snapshot_like_cpp(),
            // C++ FactionStateList is keyed by RepListID (ReputationMgr.h:63)
            // and the Player now owns it in that shape, so a shadowed duplicate
            // row can no longer exist to be acknowledged (#735).
            reputations: state.reputation.factions_like_cpp().cloned().collect(),
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
