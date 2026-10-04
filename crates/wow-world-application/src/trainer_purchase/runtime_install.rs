// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use std::collections::{BTreeSet, HashMap};
#[cfg(any(test, feature = "test-fixtures"))]
use std::collections::BTreeMap;

use crate::spell_acquisition::{
    PlayerSpellAcquisitionRuntimeApplyErrorLikeCpp as ApplyError,
    PlayerSpellAcquisitionRuntimeLikeCpp,
};
use super::AppTrainerCx;
use wow_spell_acquisition::{
    PlayerSkillPersistenceStateLikeCpp, PlayerSpellAcquisitionSnapshotLikeCpp,
    PlayerSpellPersistenceStateLikeCpp, SpellAcquisitionPostCommitActionLikeCpp,
};
use wow_world_core::session::{
    PlayerAcquisitionOwnerAccessLikeCpp, RepresentedPlayerSkillLikeCpp,
    RepresentedPlayerSkillStateLikeCpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
use wow_world_core::session::PlayerSkillTestFixtureLikeCpp;
use wow_world_spell::{
    RepresentedPlayerSpellLikeCpp, RepresentedPlayerSpellStateLikeCpp, SessionSpellState,
    canonical_player_spell_record_like_cpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
use wow_world_spell::represented_player_spell_runtime_like_cpp;
use wow_core::ObjectGuid;
use wow_packet::packets::trainer::{
    LearnedSpellEntry, LearnedSpells, SupercededSpells, UnlearnedSpells,
};

/// Shared full-runtime installer used by the trainer application and the
/// existing World spell-effect adapter. The canonical query runs before the
/// handle-less fixture fallback, and the represented/canonical conversions
/// remain the same ones used by the old fixture mutation path.
#[allow(clippy::too_many_arguments)]
pub fn install_player_spell_acquisition_runtime_snapshot_like_cpp(
    owner: &PlayerAcquisitionOwnerAccessLikeCpp<'_>,
    spell_state: &mut SessionSpellState,
    runtime_snapshot: &PlayerSpellAcquisitionSnapshotLikeCpp,
    new_non_durable_skill_tombstone_ids: &BTreeSet<u16>,
    consumer_test: bool,
    #[cfg(any(test, feature = "test-fixtures"))]
    fixture_inputs: (&mut PlayerSkillTestFixtureLikeCpp, &mut u16),
) -> Result<(), ApplyError> {
    let spell_rows = runtime_snapshot
        .spells
        .iter()
        .map(|spell| RepresentedPlayerSpellLikeCpp {
            spell_id: i32::try_from(spell.spell_id)
                .expect("prepared acquisition validated every spell ID"),
            active: spell.active,
            disabled: spell.disabled,
            dependent: spell.dependent,
            favorite: spell.favorite,
            state: match spell.state {
                PlayerSpellPersistenceStateLikeCpp::Unchanged => {
                    RepresentedPlayerSpellStateLikeCpp::Unchanged
                }
                PlayerSpellPersistenceStateLikeCpp::Changed => {
                    RepresentedPlayerSpellStateLikeCpp::Changed
                }
                PlayerSpellPersistenceStateLikeCpp::New => {
                    RepresentedPlayerSpellStateLikeCpp::New
                }
                PlayerSpellPersistenceStateLikeCpp::Removed => {
                    RepresentedPlayerSpellStateLikeCpp::Removed
                }
                PlayerSpellPersistenceStateLikeCpp::Temporary => {
                    RepresentedPlayerSpellStateLikeCpp::Temporary
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
                RepresentedPlayerSkillLikeCpp {
                    skill_id,
                    step: skill.step,
                    value: skill.value,
                    max: skill.maximum,
                    profession_slot: skill.profession_association.database_value_like_cpp(),
                    state: match skill.state {
                        PlayerSkillPersistenceStateLikeCpp::Unchanged => {
                            RepresentedPlayerSkillStateLikeCpp::Unchanged
                        }
                        PlayerSkillPersistenceStateLikeCpp::Changed => {
                            RepresentedPlayerSkillStateLikeCpp::Changed
                        }
                        PlayerSkillPersistenceStateLikeCpp::New => {
                            RepresentedPlayerSkillStateLikeCpp::New
                        }
                        PlayerSkillPersistenceStateLikeCpp::Deleted => {
                            RepresentedPlayerSkillStateLikeCpp::Deleted
                        }
                    },
                },
            )
        })
        .collect::<HashMap<_, _>>();

    let canonical_tombstones = owner
        .spell_acquisition()
        .skill_non_durable_tombstones_snapshot_like_cpp();
    #[cfg(any(test, feature = "test-fixtures"))]
    let canonical_tombstones = match canonical_tombstones {
        Some(tombstones) => Some(tombstones),
        None if owner.player_handle_absent_like_cpp() => Some(
            fixture_inputs
                .0
                .player_skill_non_durable_tombstones_like_cpp
                .clone(),
        ),
        None => None,
    };
    #[cfg(not(any(test, feature = "test-fixtures")))]
    let canonical_tombstones = canonical_tombstones;
    let mut non_durable_skill_tombstone_ids =
        canonical_tombstones.ok_or(ApplyError::InvalidPreparedRuntime)?;
    non_durable_skill_tombstone_ids.retain(|skill_id| {
        skill_records.get(skill_id).is_some_and(|skill| {
            skill.step == 0
                && skill.value == 0
                && skill.max == 0
                && skill.profession_slot == -1
                && matches!(
                    skill.state,
                    RepresentedPlayerSkillStateLikeCpp::Unchanged
                        | RepresentedPlayerSkillStateLikeCpp::Deleted
                )
        })
    });
    non_durable_skill_tombstone_ids.extend(new_non_durable_skill_tombstone_ids.iter().copied());

    install_represented_spell_acquisition_runtime_like_cpp(
        owner,
        spell_state,
        spell_rows,
        traits,
        overrides,
        skill_records,
        runtime_snapshot.occupied_skill_slots,
        non_durable_skill_tombstone_ids,
        consumer_test,
        #[cfg(any(test, feature = "test-fixtures"))]
        fixture_inputs,
    )
}

/// Install already represented Session rows through the shared canonical or
/// handle-less runtime path. World compatibility callers and the acquisition
/// snapshot adapter use this same validation and mutation provider.
#[allow(clippy::too_many_arguments)]
pub fn install_represented_spell_acquisition_runtime_like_cpp(
    owner: &PlayerAcquisitionOwnerAccessLikeCpp<'_>,
    spell_state: &mut SessionSpellState,
    spell_rows: impl IntoIterator<Item = RepresentedPlayerSpellLikeCpp>,
    traits: impl IntoIterator<Item = (i32, i32)>,
    overrides: impl IntoIterator<Item = (i32, i32)>,
    skill_records: HashMap<u16, RepresentedPlayerSkillLikeCpp>,
    occupied_skill_slots: u16,
    non_durable_skill_tombstone_ids: BTreeSet<u16>,
    consumer_test: bool,
    #[cfg(any(test, feature = "test-fixtures"))]
    fixture_inputs: (&mut PlayerSkillTestFixtureLikeCpp, &mut u16),
) -> Result<(), ApplyError> {
    #[cfg(not(any(test, feature = "test-fixtures")))]
    let _ = spell_state;

    #[cfg(any(test, feature = "test-fixtures"))]
    if consumer_test && owner.player_handle_absent_like_cpp() {
        if !install_handleless_fixture_snapshot_like_cpp(
            owner,
            spell_state,
            spell_rows,
            traits,
            overrides,
            &skill_records,
            occupied_skill_slots,
            &non_durable_skill_tombstone_ids,
        ) {
            return Err(ApplyError::InvalidPreparedRuntime);
        }
        if !owner.replace_skill_runtime_exact_like_cpp(
            skill_records,
            true,
            true,
            Some(occupied_skill_slots),
            non_durable_skill_tombstone_ids,
            fixture_inputs,
        ) {
            return Err(ApplyError::InvalidPreparedRuntime);
        }
        return Ok(());
    }
    #[cfg(not(any(test, feature = "test-fixtures")))]
    let _ = consumer_test;

    if !owner.spell_acquisition().install_complete_spell_acquisition_like_cpp(
        spell_rows.into_iter().map(canonical_player_spell_record_like_cpp),
        traits,
        overrides,
        skill_records,
        occupied_skill_slots,
        non_durable_skill_tombstone_ids,
    ) {
        return Err(ApplyError::InvalidPreparedRuntime);
    }
    Ok(())
}

#[cfg(any(test, feature = "test-fixtures"))]
fn install_handleless_fixture_snapshot_like_cpp(
    owner: &PlayerAcquisitionOwnerAccessLikeCpp<'_>,
    spell_state: &mut SessionSpellState,
    spell_rows: impl IntoIterator<Item = RepresentedPlayerSpellLikeCpp>,
    traits: impl IntoIterator<Item = (i32, i32)>,
    overrides: impl IntoIterator<Item = (i32, i32)>,
    skill_records: &HashMap<u16, RepresentedPlayerSkillLikeCpp>,
    occupied_skill_slots: u16,
    non_durable_skill_tombstones: &BTreeSet<u16>,
) -> bool {
    let mut exact_spells = BTreeMap::new();
    for spell in spell_rows {
        if spell.spell_id <= 0 || exact_spells.insert(spell.spell_id, spell).is_some() {
            return false;
        }
    }

    let mut exact_traits = HashMap::new();
    for (spell_id, trait_definition_id) in traits {
        if trait_definition_id <= 0
            || !exact_spells
                .get(&spell_id)
                .is_some_and(|spell| spell.state != RepresentedPlayerSpellStateLikeCpp::Removed)
            || exact_traits.insert(spell_id, trait_definition_id).is_some()
        {
            return false;
        }
    }

    let mut exact_overrides = HashMap::<i32, BTreeSet<i32>>::new();
    for (overridden_spell_id, overriding_spell_id) in overrides {
        if overridden_spell_id <= 0 || overriding_spell_id <= 0 {
            return false;
        }
        exact_overrides
            .entry(overridden_spell_id)
            .or_default()
            .insert(overriding_spell_id);
    }

    if usize::from(occupied_skill_slots) != skill_records.len()
        || occupied_skill_slots > 256
        || !skill_records.iter().all(|(skill_id, skill)| {
            *skill_id != 0
                && *skill_id == skill.skill_id
                && (skill.state != RepresentedPlayerSkillStateLikeCpp::Deleted
                    || (skill.step == 0
                        && skill.value == 0
                        && skill.max == 0
                        && skill.profession_slot == -1))
        })
        || !non_durable_skill_tombstones.iter().all(|skill_id| {
            skill_records
                .get(skill_id)
                .is_some_and(wow_world_core::session::is_non_durable_skill_tombstone_like_cpp)
        })
    {
        return false;
    }

    let mut known_spells = exact_spells
        .values()
        .filter(|spell| {
            spell.state != RepresentedPlayerSpellStateLikeCpp::Removed && !spell.disabled
        })
        .map(|spell| spell.spell_id)
        .collect::<Vec<_>>();
    known_spells.sort_unstable();
    let dependent_spells = exact_spells
        .values()
        .filter(|spell| {
            spell.state != RepresentedPlayerSpellStateLikeCpp::Removed && spell.dependent
        })
        .map(|spell| spell.spell_id)
        .collect();
    let favorite_spells = exact_spells
        .values()
        .filter(|spell| {
            spell.state != RepresentedPlayerSpellStateLikeCpp::Removed && spell.favorite
        })
        .map(|spell| spell.spell_id)
        .collect();
    let removed_spells = exact_spells
        .values()
        .filter(|spell| spell.state == RepresentedPlayerSpellStateLikeCpp::Removed)
        .map(|spell| spell.spell_id)
        .collect();
    owner.invalidate_spell_hit_aura_authority_like_cpp();
    let fixture_runtime = owner
        .with_player_spell_runtime_like_cpp(represented_player_spell_runtime_like_cpp);
    let fixture_runtime = match fixture_runtime {
        Some(runtime) => runtime,
        None if owner.player_handle_absent_like_cpp() => {
            spell_state.represented_spell_runtime_fixture_like_cpp()
        }
        None => return false,
    };

    spell_state.install_complete_spell_acquisition_fixture_like_cpp(
        fixture_runtime,
        wow_entities::PlayerSpellAcquisitionSnapshotLikeCpp {
            known_spells,
            rows: exact_spells
                .into_iter()
                .map(|(id, row)| (id, canonical_player_spell_record_like_cpp(row)))
                .collect(),
            dependent_known_spells: dependent_spells,
            removed_known_spells: removed_spells,
            favorite_known_spells: favorite_spells,
            trait_definition_ids: exact_traits.into_iter().collect(),
            override_spells: exact_overrides.into_iter().collect(),
        },
    );
    true
}

impl PlayerSpellAcquisitionRuntimeLikeCpp for AppTrainerCx<'_> {
    fn character_guid(&self) -> Option<ObjectGuid> {
        self.owner.player_guid()
    }

    fn has_canonical_player(&self) -> bool {
        self.owner.spell_acquisition().has_canonical_player_like_cpp()
    }

    fn install_snapshot(
        &mut self,
        runtime_snapshot: &PlayerSpellAcquisitionSnapshotLikeCpp,
        new_non_durable_skill_tombstone_ids: &BTreeSet<u16>,
    ) -> Result<(), ApplyError> {
        let installed = install_player_spell_acquisition_runtime_snapshot_like_cpp(
            &self.owner,
            self.spell_state,
            runtime_snapshot,
            new_non_durable_skill_tombstone_ids,
            self.consumer_test,
            #[cfg(any(test, feature = "test-fixtures"))]
            (
                &mut *self.fixtures.player_skill_fixture,
                &mut *self.fixtures.represented_enchanting_skill,
            ),
        );
        if installed.is_ok()
            && let Some(control) = self.owner.registry_control_binding()
        {
            let position = self.owner.registry_sync(
                #[cfg(any(test, feature = "test-fixtures"))]
                self.fixtures.registry_position,
                #[cfg(any(test, feature = "test-fixtures"))]
                self.fixtures.registry_level,
                #[cfg(any(test, feature = "test-fixtures"))]
                self.fixtures.registry_transport,
            );
            let sync = crate::PlayerRegistrySyncContext::new(
                position,
                control,
                self.loot,
                #[cfg(any(test, feature = "test-fixtures"))]
                wow_world_core::session::RegistrySyncInputs::new_like_cpp(
                    self.fixtures.registry_health,
                    self.fixtures.registry_max_health,
                    self.fixtures.registry_alive,
                ),
            );
            #[cfg(any(test, feature = "test-fixtures"))]
            let sync = if self.consumer_test {
                sync.with_fixture_hydration(crate::PlayerRegistryHydrationContext::new(
                    self.owner.registry_hydration(),
                    self.spell_state,
                    self.quest_state,
                    (
                        self.fixtures.registry_mount_vehicle_kit,
                        self.fixtures.registry_vehicle_seat_flags,
                        self.fixtures.registry_vehicle_seat_id,
                        self.fixtures.registry_pet_guid,
                    ),
                    self.consumer_test,
                ))
            } else {
                sync
            };
            sync.sync();
        }
        installed
    }

    fn begin_action_batch(&mut self) {
        self.spell_state
            .begin_spell_acquisition_post_commit_action_batch_like_cpp();
    }

    fn record_action(&mut self, action: SpellAcquisitionPostCommitActionLikeCpp) {
        self.spell_state
            .record_spell_acquisition_post_commit_action_like_cpp(action);
    }

    fn grant_dual_wield(&mut self) -> bool {
        self.owner
            .spell_acquisition()
            .grant_dual_wield_after_acquisition_like_cpp()
    }

    fn publish_action(
        &mut self,
        action: &SpellAcquisitionPostCommitActionLikeCpp,
        trait_definition_id: Option<i32>,
    ) {
        publish_spell_acquisition_action_like_cpp(
            &self.owner,
            action,
            trait_definition_id,
        );
    }
}

/// Publish the packet-bearing subset of one post-commit action through the
/// shared Core connection owner. World and application adapters use the same
/// packet construction and ignore the same non-packet actions.
pub fn publish_spell_acquisition_action_like_cpp(
    owner: &PlayerAcquisitionOwnerAccessLikeCpp<'_>,
    action: &SpellAcquisitionPostCommitActionLikeCpp,
    trait_definition_id: Option<i32>,
) {
    let publication = owner.packet_publication();
    match action {
        SpellAcquisitionPostCommitActionLikeCpp::LearnedSpell {
            spell_id,
            favorite,
            suppress_messaging,
        } => {
            publication.send_packet(&LearnedSpells {
                spells: vec![LearnedSpellEntry {
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
            publication.send_packet(&SupercededSpells::single(
                i32::try_from(*old_spell_id).expect("validated old spell ID"),
                i32::try_from(*new_spell_id).expect("validated new spell ID"),
            ));
        }
        SpellAcquisitionPostCommitActionLikeCpp::UnlearnedSpell { spell_id } => {
            publication.send_packet(&UnlearnedSpells::single(*spell_id, false));
        }
        SpellAcquisitionPostCommitActionLikeCpp::GrantDualWield { .. }
        | SpellAcquisitionPostCommitActionLikeCpp::RefreshPassive { .. }
        | SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnSpellQuestObjective { .. }
        | SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnTradeskillSkillLineCriteria { .. }
        | SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnSpellFromSkillLineCriteria { .. }
        | SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnOrKnowSpellCriteria { .. }
        | SpellAcquisitionPostCommitActionLikeCpp::UpdateMountCapability { .. }
        | SpellAcquisitionPostCommitActionLikeCpp::UpdateSkillRaisedCriteria { .. }
        | SpellAcquisitionPostCommitActionLikeCpp::UpdateAchieveSkillStepCriteria { .. } => {}
    }
}
