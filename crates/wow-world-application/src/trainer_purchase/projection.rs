// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Current-player acquisition projection for the trainer's `NeedsProjection`
//! branch. Each selected owner is queried here, after offer preflight.

use std::collections::{BTreeMap, HashMap};

use super::controller::TrainerOfferCxLikeCpp;
use wow_spell_acquisition::{
    PlayerAcquisitionLifecycleLikeCpp, PlayerSkillAcquisitionRowLikeCpp,
    PlayerSkillPersistenceStateLikeCpp, PlayerSpellAcquisitionRowLikeCpp,
    PlayerSpellPersistenceStateLikeCpp,
    PlayerSpellAcquisitionSnapshotLikeCpp, ProfessionAssociationInputLikeCpp,
    SpellAcquisitionSnapshotAdapterErrorLikeCpp,
};
use wow_world_core::session::RepresentedPlayerSkillStateLikeCpp;
use wow_world_spell::RepresentedPlayerSpellStateLikeCpp as SpellState;

/// Inert selected stores for wrapper resolution and acquisition planning.
/// Missing stores are checked at their original operation phases.
pub struct TrainerProjectionCatalogsLikeCpp<'a> {
    chains: Option<&'a wow_data::SpellChainStoreLikeCpp>,
    learn_skills: Option<&'a wow_data::SpellLearnSkillStoreLikeCpp>,
    learn_spells: Option<&'a wow_data::SpellLearnSpellStoreLikeCpp>,
    required: Option<&'a wow_data::SpellRequiredStoreLikeCpp>,
    custom_attributes: Option<&'a wow_data::SpellCustomAttributeStoreLikeCpp>,
    traits: Option<&'a wow_data::TraitDefinitionStore>,
    skill_tiers: Option<&'a wow_data::SkillTiersStoreLikeCpp>,
    mounts: Option<&'a wow_data::MountStore>,
    difficulties: Option<&'a wow_data::DifficultyStore>,
    maps: Option<&'a wow_data::MapStore>,
    disables: Option<&'a wow_data::DisableMgrLikeCpp>,
    targets: Option<&'a wow_data::SpellTargetRestrictionsStore>,
    aura_restrictions: Option<&'a wow_data::SpellAuraRestrictionsStore>,
    pet_auras: Option<&'a wow_data::SpellPetAuraStoreLikeCpp>,
    linked: Option<&'a wow_data::SpellLinkedStoreLikeCpp>,
}

impl<'a> TrainerProjectionCatalogsLikeCpp<'a> {
    pub fn new(
        chains: Option<&'a wow_data::SpellChainStoreLikeCpp>,
        learn_skills: Option<&'a wow_data::SpellLearnSkillStoreLikeCpp>,
        learn_spells: Option<&'a wow_data::SpellLearnSpellStoreLikeCpp>,
        required: Option<&'a wow_data::SpellRequiredStoreLikeCpp>,
        custom_attributes: Option<&'a wow_data::SpellCustomAttributeStoreLikeCpp>,
        traits: Option<&'a wow_data::TraitDefinitionStore>,
        skill_tiers: Option<&'a wow_data::SkillTiersStoreLikeCpp>,
        mounts: Option<&'a wow_data::MountStore>,
        difficulties: Option<&'a wow_data::DifficultyStore>,
        maps: Option<&'a wow_data::MapStore>,
        disables: Option<&'a wow_data::DisableMgrLikeCpp>,
        targets: Option<&'a wow_data::SpellTargetRestrictionsStore>,
        aura_restrictions: Option<&'a wow_data::SpellAuraRestrictionsStore>,
        pet_auras: Option<&'a wow_data::SpellPetAuraStoreLikeCpp>,
        linked: Option<&'a wow_data::SpellLinkedStoreLikeCpp>,
    ) -> Self {
        Self {
            chains, learn_skills, learn_spells, required, custom_attributes,
            traits, skill_tiers, mounts, difficulties, maps, disables,
            targets, aura_restrictions, pet_auras, linked,
        }
    }
}

static FAIL_CLOSED_CAST_AUTHORITY_LIKE_CPP: std::sync::LazyLock<
    wow_spell_acquisition::SpellAcquisitionCastAuthorityLikeCpp,
> = std::sync::LazyLock::new(Default::default);
static FAIL_CLOSED_CRAFT_AUTHORITY_LIKE_CPP: std::sync::LazyLock<
    wow_spell_acquisition::SpellAcquisitionCraftValidityAuthorityLikeCpp,
> = std::sync::LazyLock::new(Default::default);

impl TrainerOfferCxLikeCpp<'_> {
    pub(super) fn finish_trainer_offer_projection_like_cpp(
        &self,
        projection: super::TrainerOfferProjectionLikeCpp,
    ) -> super::TrainerOfferDecisionLikeCpp {
        let outcome = self.project_trainer_spell_acquisition_like_cpp(projection.root);
        let capacity = match &outcome {
            wow_spell_acquisition::SpellAcquisitionOutcomeLikeCpp::Deterministic(plan) => {
                Some(self.plan_primary_profession_capacity_like_cpp(
                    plan.root_primary_profession_skill_ids.iter().copied(),
                ))
            }
            wow_spell_acquisition::SpellAcquisitionOutcomeLikeCpp::Indeterminate(_) => None,
        };
        super::finish_trainer_offer_after_projection_like_cpp(
            projection.source_spell_id,
            projection.effective_price,
            projection.battle_pet_species_id,
            outcome,
            capacity,
        )
    }

    fn project_trainer_spell_acquisition_like_cpp(
        &self,
        root: wow_spell_acquisition::SpellAcquisitionRootLikeCpp,
    ) -> wow_spell_acquisition::SpellAcquisitionOutcomeLikeCpp {
        use wow_spell_acquisition::{
            SpellAcquisitionIndeterminateLikeCpp, SpellAcquisitionMetadataLikeCpp,
            SpellAcquisitionOutcomeLikeCpp, SpellAcquisitionRootLikeCpp,
        };
        let inputs = &self.catalogs.projection;
        let cast_resolutions = match root {
            SpellAcquisitionRootLikeCpp::DirectLearn(_) => BTreeMap::new(),
            SpellAcquisitionRootLikeCpp::TrainerWrapperCast(spell_id) => {
                let Some(resolution) = self.spell_state
                    .resolve_trainer_wrapper_cast_acquisition_with_access_like_cpp(
                        &self.spell_access,
                        self.player_conditions.player_access_like_cpp(),
                        self.catalogs.spell_acquisition,
                        inputs.difficulties,
                        inputs.maps,
                        inputs.disables,
                        inputs.targets,
                        inputs.aura_restrictions,
                        inputs.pet_auras,
                        inputs.linked,
                        spell_id,
                    ) else {
                        return SpellAcquisitionOutcomeLikeCpp::Indeterminate(
                            SpellAcquisitionIndeterminateLikeCpp::MissingCastResolution { spell_id },
                        );
                    };
                BTreeMap::from([(spell_id, resolution)])
            }
        };
        let snapshot = match self.spell_acquisition_snapshot_like_cpp(cast_resolutions) {
            Ok(snapshot) => snapshot,
            Err(error) => return SpellAcquisitionOutcomeLikeCpp::Indeterminate(
                SpellAcquisitionIndeterminateLikeCpp::SnapshotAdapter(error),
            ),
        };
        let (
            Some(catalog), Some(spell_chains), Some(spell_learn_skills),
            Some(spell_learn_spells), Some(spell_required), Some(spell_custom_attributes),
            Some(trait_definitions), Some(skills), Some(skill_lines), Some(skill_tiers),
        ) = (
            self.catalogs.spell_acquisition, inputs.chains, inputs.learn_skills,
            inputs.learn_spells, inputs.required, inputs.custom_attributes,
            inputs.traits, self.catalogs.skills, self.catalogs.skill_lines, inputs.skill_tiers,
        ) else {
            return SpellAcquisitionOutcomeLikeCpp::Indeterminate(
                SpellAcquisitionIndeterminateLikeCpp::MissingTrainerProjectionMetadata,
            );
        };
        let cast_authority = self.spell_state.spell_acquisition_cast_authority_like_cpp()
            .map(AsRef::as_ref).unwrap_or(&FAIL_CLOSED_CAST_AUTHORITY_LIKE_CPP);
        let craft_validity_authority = self.spell_state.spell_acquisition_craft_authority_like_cpp()
            .map(AsRef::as_ref).unwrap_or(&FAIL_CLOSED_CRAFT_AUTHORITY_LIKE_CPP);
        wow_spell_acquisition::project_spell_acquisition_like_cpp(
            &snapshot,
            SpellAcquisitionMetadataLikeCpp {
                catalog, spell_chains, spell_learn_skills, spell_learn_spells,
                spell_required, spell_custom_attributes, trait_definitions,
                cast_authority, craft_validity_authority, mounts: inputs.mounts,
                skills, skill_lines, skill_tiers,
            },
            root,
        )
    }

    fn plan_primary_profession_capacity_like_cpp(
        &self,
        requested_skill_ids: impl IntoIterator<Item = u32>,
    ) -> Result<crate::PrimaryProfessionCapacityPlanLikeCpp, crate::PrimaryProfessionCapacityPlanErrorLikeCpp> {
        use crate::PrimaryProfessionCapacityPlanErrorLikeCpp as Error;
        let Some(skill_lines) = self.catalogs.skill_lines else {
            return Err(Error::MissingSkillLineStore);
        };
        let Some(skills_loaded) = self.spell_access.player_skill_records_loaded_with_fixture_like_cpp(
            #[cfg(any(test, feature = "test-fixtures"))]
            self.skill_fixture_loaded,
        ) else {
            return Err(Error::MissingPlayerSkillSnapshot);
        };
        if !skills_loaded {
            return Err(Error::MissingPlayerSkillSnapshot);
        }
        let Some(skill_records) = self.player_conditions.player_access_like_cpp()
            .resolved_player_skill_records_like_cpp() else {
                return Err(Error::MissingPlayerSkillSnapshot);
            };
        let current_skills = skill_records.values().map(|skill| crate::PlayerSkillProfessionSnapshotLikeCpp {
            skill_id: u32::from(skill.skill_id),
            value: skill.value,
            profession_slot: skill.profession_slot,
        });
        let analysis = crate::analyze_primary_professions_like_cpp(
            *self.max_primary_trade_skills,
            skill_lines,
            current_skills,
        )?;
        crate::plan_primary_professions_like_cpp(&analysis, skill_lines, requested_skill_ids)
    }

    /// Snapshot acquisition inputs only after this row has survived every
    /// trainer-offer preflight gate. The ordering matches the World adapter:
    /// spell rows, skill rows, occupied slots, traits, then overrides.
    pub(super) fn spell_acquisition_snapshot_like_cpp(
        &self,
        cast_resolutions: BTreeMap<u32, wow_spell_acquisition::PlayerCastAcquisitionResolutionLikeCpp>,
    ) -> Result<PlayerSpellAcquisitionSnapshotLikeCpp, SpellAcquisitionSnapshotAdapterErrorLikeCpp>
    {
        #[cfg(any(test, feature = "test-fixtures"))]
        let consumer_test = self.player_conditions.consumer_test;
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let consumer_test = false;
        let spell_rows = super::controller::complete_represented_player_spell_rows_like_cpp(
            &self.spell_access,
            self.spell_state,
            consumer_test,
        )
        .ok_or(SpellAcquisitionSnapshotAdapterErrorLikeCpp::IncompleteSpellRows)?;
        let skill_rows = self
            .player_conditions
            .complete_player_skill_records_like_cpp()
            .ok_or(SpellAcquisitionSnapshotAdapterErrorLikeCpp::IncompleteSkillRows)?;
        let occupied_skill_slots = self
            .spell_access
            .complete_player_skill_occupied_slots_like_cpp(
                #[cfg(any(test, feature = "test-fixtures"))]
                self.skill_fixture_complete,
                #[cfg(any(test, feature = "test-fixtures"))]
                self.skill_fixture_occupied,
            )
            .ok_or(SpellAcquisitionSnapshotAdapterErrorLikeCpp::MissingSkillSlotOccupancy)?;
        let traits = self.complete_spell_trait_definitions_like_cpp()
            .ok_or(SpellAcquisitionSnapshotAdapterErrorLikeCpp::IncompleteTraitDefinitions)?;
        let represented_overrides = self.complete_spell_overrides_like_cpp()
            .ok_or(SpellAcquisitionSnapshotAdapterErrorLikeCpp::IncompleteOverrides)?;

        let mut trait_spell_ids = traits.keys().copied().collect::<Vec<_>>();
        trait_spell_ids.sort_unstable();
        for spell_id in trait_spell_ids {
            if !spell_rows.contains_key(&spell_id) {
                return Err(SpellAcquisitionSnapshotAdapterErrorLikeCpp::OrphanTraitDefinition {
                    spell_id,
                });
            }
        }

        let spells = spell_rows
            .values()
            .map(|row| {
                let spell_id = u32::try_from(row.spell_id).map_err(|_| {
                    SpellAcquisitionSnapshotAdapterErrorLikeCpp::InvalidSpellId(row.spell_id)
                })?;
                let trait_definition_id = traits.get(&row.spell_id).copied();
                if trait_definition_id.is_some_and(|id| id <= 0) {
                    return Err(
                        SpellAcquisitionSnapshotAdapterErrorLikeCpp::InvalidTraitDefinitionId {
                            spell_id: row.spell_id,
                            trait_definition_id: trait_definition_id.unwrap_or_default(),
                        },
                    );
                }
                Ok(PlayerSpellAcquisitionRowLikeCpp {
                    spell_id,
                    active: row.active,
                    disabled: row.disabled,
                    dependent: row.dependent,
                    favorite: row.favorite,
                    trait_definition_id,
                    state: match row.state {
                        SpellState::Unchanged => PlayerSpellPersistenceStateLikeCpp::Unchanged,
                        SpellState::Changed => PlayerSpellPersistenceStateLikeCpp::Changed,
                        SpellState::New => PlayerSpellPersistenceStateLikeCpp::New,
                        SpellState::Removed => PlayerSpellPersistenceStateLikeCpp::Removed,
                        SpellState::Temporary => PlayerSpellPersistenceStateLikeCpp::Temporary,
                    },
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        let mut skills = skill_rows
            .values()
            .map(|row| PlayerSkillAcquisitionRowLikeCpp {
                skill_id: u32::from(row.skill_id),
                step: row.step,
                value: row.value,
                maximum: row.max,
                profession_association: ProfessionAssociationInputLikeCpp::from_database_value_like_cpp(
                    row.profession_slot,
                ),
                state: match row.state {
                    RepresentedPlayerSkillStateLikeCpp::Unchanged => {
                        PlayerSkillPersistenceStateLikeCpp::Unchanged
                    }
                    RepresentedPlayerSkillStateLikeCpp::Changed => {
                        PlayerSkillPersistenceStateLikeCpp::Changed
                    }
                    RepresentedPlayerSkillStateLikeCpp::New => {
                        PlayerSkillPersistenceStateLikeCpp::New
                    }
                    RepresentedPlayerSkillStateLikeCpp::Deleted => {
                        PlayerSkillPersistenceStateLikeCpp::Deleted
                    }
                },
            })
            .collect::<Vec<_>>();
        skills.sort_by_key(|skill| skill.skill_id);

        let mut represented_override_pairs = represented_overrides
            .iter()
            .flat_map(|(&overridden_spell_id, overriding_spell_ids)| {
                overriding_spell_ids
                    .iter()
                    .map(move |&overriding_spell_id| (overridden_spell_id, overriding_spell_id))
            })
            .collect::<Vec<_>>();
        represented_override_pairs.sort_unstable();
        let mut overrides = Vec::with_capacity(represented_override_pairs.len());
        for (overridden_spell_id, overriding_spell_id) in represented_override_pairs {
            let (Ok(overridden_spell_id_u32), Ok(overriding_spell_id_u32)) = (
                u32::try_from(overridden_spell_id),
                u32::try_from(overriding_spell_id),
            ) else {
                return Err(SpellAcquisitionSnapshotAdapterErrorLikeCpp::InvalidOverride {
                    overridden_spell_id,
                    overriding_spell_id,
                });
            };
            overrides.push((overridden_spell_id_u32, overriding_spell_id_u32));
        }

        let mut primary_profession_skill_ids = skills
            .iter()
            .filter(|skill| {
                skill.state != PlayerSkillPersistenceStateLikeCpp::Deleted && skill.value != 0
            })
            .filter_map(|skill| {
                self.catalogs
                    .skill_lines
                    .and_then(|store| {
                        store.is_primary_profession_skill_like_cpp(skill.skill_id)
                            .map(|is_primary| (skill.skill_id, is_primary))
                    })
                    .and_then(|(skill_id, is_primary)| is_primary.then_some(skill_id))
            })
            .collect::<Vec<_>>();
        primary_profession_skill_ids.sort_unstable();
        let non_durable_skill_tombstone_ids = self
            .spell_access
            .skill_non_durable_tombstones_with_fixture_like_cpp(
                #[cfg(any(test, feature = "test-fixtures"))]
                self.skill_fixture_tombstones,
            )
            .ok_or(SpellAcquisitionSnapshotAdapterErrorLikeCpp::IncompleteSkillRows)?
            .iter()
            .map(|skill_id| u32::from(*skill_id))
            .collect();

        let player = self.player_conditions.player_access_like_cpp();
        Ok(PlayerSpellAcquisitionSnapshotLikeCpp {
            character_guid: self.spell_access.player_guid_like_cpp(),
            spells,
            skills,
            occupied_skill_slots,
            overrides,
            primary_profession_skill_ids,
            non_durable_skill_tombstone_ids,
            race: player.player_race_like_cpp(),
            class: player.player_class_like_cpp(),
            level: player.player_level_like_cpp(),
            lifecycle: PlayerAcquisitionLifecycleLikeCpp::InWorld,
            future_player_condition_resolutions: Vec::new(),
            cast_resolutions,
        })
    }

    fn complete_spell_trait_definitions_like_cpp(&self) -> Option<HashMap<i32, i32>> {
        fn complete(runtime: &wow_entities::PlayerSpellRuntimeState) -> Option<HashMap<i32, i32>> {
            runtime.trait_definition_ids_complete_like_cpp().then(|| {
                runtime
                    .trait_definition_ids_like_cpp()
                    .iter()
                    .map(|(&id, &value)| (id, value))
                    .collect()
            })
        }
        let canonical = self
            .spell_access
            .with_player_spell_runtime_like_cpp(complete);
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none()
            && self.player_conditions.consumer_test
            && self.spell_access.player_handle_absent_like_cpp()
        {
            let runtime = wow_world_spell::canonical_player_spell_runtime_like_cpp(
                self.spell_state.represented_spell_runtime_fixture_like_cpp(),
            );
            return complete(&runtime);
        }
        canonical.flatten()
    }

    fn complete_spell_overrides_like_cpp(
        &self,
    ) -> Option<HashMap<i32, std::collections::BTreeSet<i32>>> {
        fn complete(
            runtime: &wow_entities::PlayerSpellRuntimeState,
        ) -> Option<HashMap<i32, std::collections::BTreeSet<i32>>> {
            runtime.override_spells_complete_like_cpp().then(|| {
                runtime
                    .override_spells_like_cpp()
                    .iter()
                    .map(|(&id, values)| (id, values.clone()))
                    .collect()
            })
        }
        let canonical = self
            .spell_access
            .with_player_spell_runtime_like_cpp(complete);
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none()
            && self.player_conditions.consumer_test
            && self.spell_access.player_handle_absent_like_cpp()
        {
            let runtime = wow_world_spell::canonical_player_spell_runtime_like_cpp(
                self.spell_state.represented_spell_runtime_fixture_like_cpp(),
            );
            return complete(&runtime);
        }
        canonical.flatten()
    }
}
