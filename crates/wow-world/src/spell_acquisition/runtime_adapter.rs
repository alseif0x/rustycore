// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! `WorldSession` adapter for the generic spell-acquisition application.
//!
//! This module owns the representation boundary: canonical Session rows,
//! retained skill tombstones and packet construction. The application module
//! owns ordering and does not depend on these concrete representations.

use std::collections::{BTreeSet, HashMap};

use super::*;

impl PlayerSpellAcquisitionRuntimeLikeCpp for crate::session::WorldSession {
    fn character_guid(&self) -> Option<wow_core::ObjectGuid> {
        self.player_guid()
    }

    fn has_canonical_player(&self) -> bool {
        self.has_canonical_player_for_spell_acquisition_like_cpp()
    }

    fn install_snapshot(
        &mut self,
        runtime_snapshot: &PlayerSpellAcquisitionSnapshotLikeCpp,
        new_non_durable_skill_tombstone_ids: &BTreeSet<u16>,
    ) -> Result<(), PlayerSpellAcquisitionRuntimeApplyErrorLikeCpp> {
        let spell_rows = runtime_snapshot
            .spells
            .iter()
            .map(|spell| crate::session::RepresentedPlayerSpellLikeCpp {
                spell_id: i32::try_from(spell.spell_id)
                    .expect("prepared acquisition validated every spell ID"),
                active: spell.active,
                disabled: spell.disabled,
                dependent: spell.dependent,
                favorite: spell.favorite,
                state: match spell.state {
                    PlayerSpellPersistenceStateLikeCpp::Unchanged => {
                        crate::session::RepresentedPlayerSpellStateLikeCpp::Unchanged
                    }
                    PlayerSpellPersistenceStateLikeCpp::Changed => {
                        crate::session::RepresentedPlayerSpellStateLikeCpp::Changed
                    }
                    PlayerSpellPersistenceStateLikeCpp::New => {
                        crate::session::RepresentedPlayerSpellStateLikeCpp::New
                    }
                    PlayerSpellPersistenceStateLikeCpp::Removed => {
                        crate::session::RepresentedPlayerSpellStateLikeCpp::Removed
                    }
                    PlayerSpellPersistenceStateLikeCpp::Temporary => {
                        crate::session::RepresentedPlayerSpellStateLikeCpp::Temporary
                    }
                },
            })
            .collect::<Vec<_>>();
        let traits = runtime_snapshot
            .spells
            .iter()
            .filter_map(|spell| {
                if spell.state == PlayerSpellPersistenceStateLikeCpp::Removed {
                    return None;
                }
                Some((
                    i32::try_from(spell.spell_id).ok()?,
                    spell.trait_definition_id?,
                ))
            })
            .collect::<Vec<_>>();
        let overrides = runtime_snapshot
            .overrides
            .iter()
            .map(|&(overridden, overriding)| {
                (
                    i32::try_from(overridden).expect("validated overridden spell ID"),
                    i32::try_from(overriding).expect("validated overriding spell ID"),
                )
            })
            .collect::<Vec<_>>();
        let skill_records = runtime_snapshot
            .skills
            .iter()
            .map(|skill| {
                let skill_id = u16::try_from(skill.skill_id)
                    .expect("prepared acquisition validated every skill ID");
                (
                    skill_id,
                    crate::session::RepresentedPlayerSkillLikeCpp {
                        skill_id,
                        step: skill.step,
                        value: skill.value,
                        max: skill.maximum,
                        profession_slot: skill.profession_association.database_value_like_cpp(),
                        state: match skill.state {
                            PlayerSkillPersistenceStateLikeCpp::Unchanged => {
                                crate::session::RepresentedPlayerSkillStateLikeCpp::Unchanged
                            }
                            PlayerSkillPersistenceStateLikeCpp::Changed => {
                                crate::session::RepresentedPlayerSkillStateLikeCpp::Changed
                            }
                            PlayerSkillPersistenceStateLikeCpp::New => {
                                crate::session::RepresentedPlayerSkillStateLikeCpp::New
                            }
                            PlayerSkillPersistenceStateLikeCpp::Deleted => {
                                crate::session::RepresentedPlayerSkillStateLikeCpp::Deleted
                            }
                        },
                    },
                )
            })
            .collect::<HashMap<_, _>>();

        let mut non_durable_skill_tombstone_ids = self
            .resolved_player_skill_non_durable_tombstones_like_cpp()
            .ok_or(PlayerSpellAcquisitionRuntimeApplyErrorLikeCpp::InvalidPreparedRuntime)?;
        // C++ `Player::SetSkill` reactivates a `SKILL_DELETED` entry as
        // `SKILL_CHANGED`. A saved, non-durable tombstone therefore survives
        // only while the resulting row is still the zero-valued deleted shape.
        non_durable_skill_tombstone_ids.retain(|skill_id| {
            skill_records.get(skill_id).is_some_and(|skill| {
                skill.step == 0
                    && skill.value == 0
                    && skill.max == 0
                    && skill.profession_slot == -1
                    && matches!(
                        skill.state,
                        crate::session::RepresentedPlayerSkillStateLikeCpp::Unchanged
                            | crate::session::RepresentedPlayerSkillStateLikeCpp::Deleted
                    )
            })
        });
        non_durable_skill_tombstone_ids.extend(new_non_durable_skill_tombstone_ids.iter().copied());
        if !self.replace_complete_spell_acquisition_runtime_like_cpp(
            spell_rows,
            traits,
            overrides,
            skill_records,
            runtime_snapshot.occupied_skill_slots,
            non_durable_skill_tombstone_ids,
        ) {
            return Err(PlayerSpellAcquisitionRuntimeApplyErrorLikeCpp::InvalidPreparedRuntime);
        }
        Ok(())
    }

    fn begin_action_batch(&mut self) {
        self.begin_spell_acquisition_post_commit_action_batch_like_cpp();
    }

    fn record_action(&mut self, action: SpellAcquisitionPostCommitActionLikeCpp) {
        self.record_spell_acquisition_post_commit_action_like_cpp(action);
    }

    fn grant_dual_wield(&mut self) -> bool {
        self.grant_dual_wield_after_spell_acquisition_like_cpp()
    }

    fn publish_action(
        &mut self,
        action: &SpellAcquisitionPostCommitActionLikeCpp,
        trait_definition_id: Option<i32>,
    ) {
        match action {
            SpellAcquisitionPostCommitActionLikeCpp::LearnedSpell {
                spell_id,
                favorite,
                suppress_messaging,
            } => {
                self.send_packet(&wow_packet::packets::trainer::LearnedSpells {
                    spells: vec![wow_packet::packets::trainer::LearnedSpellEntry {
                        spell_id: i32::try_from(*spell_id).expect("validated learned spell ID"),
                        is_favorite: *favorite,
                        field_8: None,
                        superceded: None,
                        trait_definition_id,
                    }],
                    suppress_messaging: *suppress_messaging,
                });
            }
            SpellAcquisitionPostCommitActionLikeCpp::SupersededSpell {
                old_spell_id,
                new_spell_id,
            } => {
                self.send_packet(&wow_packet::packets::trainer::SupercededSpells::single(
                    i32::try_from(*old_spell_id).expect("validated old spell ID"),
                    i32::try_from(*new_spell_id).expect("validated new spell ID"),
                ));
            }
            SpellAcquisitionPostCommitActionLikeCpp::UnlearnedSpell { spell_id } => {
                self.send_packet(&wow_packet::packets::trainer::UnlearnedSpells::single(
                    *spell_id, false,
                ));
            }
            SpellAcquisitionPostCommitActionLikeCpp::GrantDualWield { .. }
            | SpellAcquisitionPostCommitActionLikeCpp::RefreshPassive { .. }
            | SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnSpellQuestObjective { .. }
            | SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnTradeskillSkillLineCriteria {
                ..
            }
            | SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnSpellFromSkillLineCriteria {
                ..
            }
            | SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnOrKnowSpellCriteria { .. }
            | SpellAcquisitionPostCommitActionLikeCpp::UpdateMountCapability { .. }
            | SpellAcquisitionPostCommitActionLikeCpp::UpdateSkillRaisedCriteria { .. }
            | SpellAcquisitionPostCommitActionLikeCpp::UpdateAchieveSkillStepCriteria { .. } => {}
        }
    }
}
