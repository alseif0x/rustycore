// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use super::*;

static FAIL_CLOSED_CAST_AUTHORITY_LIKE_CPP: std::sync::LazyLock<
    SpellAcquisitionCastAuthorityLikeCpp,
> = std::sync::LazyLock::new(Default::default);
static FAIL_CLOSED_CRAFT_AUTHORITY_LIKE_CPP: std::sync::LazyLock<
    SpellAcquisitionCraftValidityAuthorityLikeCpp,
> = std::sync::LazyLock::new(Default::default);

impl crate::session::WorldSession {
    /// Projects one trainer product from the complete current player snapshot.
    ///
    /// Static cast/craft safety is intentionally fail-closed until a later
    /// runtime owner supplies audited authority. Direct ordinary learns do not
    /// consult those authorities; wrapper/craft paths that need them remain
    /// unavailable instead of being guessed.
    pub(crate) fn project_trainer_spell_acquisition_like_cpp(
        &self,
        root: SpellAcquisitionRootLikeCpp,
    ) -> SpellAcquisitionOutcomeLikeCpp {
        let snapshot = match self.spell_acquisition_snapshot_like_cpp(
            PlayerAcquisitionLifecycleLikeCpp::InWorld,
            Vec::new(),
            BTreeMap::new(),
        ) {
            Ok(snapshot) => snapshot,
            Err(error) => {
                return SpellAcquisitionOutcomeLikeCpp::Indeterminate(
                    SpellAcquisitionIndeterminateLikeCpp::SnapshotAdapter(error),
                );
            }
        };
        let (
            Some(catalog),
            Some(spell_chains),
            Some(spell_learn_skills),
            Some(spell_learn_spells),
            Some(spell_required),
            Some(spell_custom_attributes),
            Some(trait_definitions),
            Some(skills),
            Some(skill_lines),
            Some(skill_tiers),
        ) = (
            self.spell_acquisition_catalog(),
            self.spell_chain_store(),
            self.spell_learn_skill_store_like_cpp(),
            self.spell_learn_spell_store_like_cpp(),
            self.spell_required_store_like_cpp(),
            self.spell_custom_attribute_store_like_cpp(),
            self.trait_definition_store(),
            self.skill_store(),
            self.skill_line_store(),
            self.skill_tiers_store(),
        )
        else {
            return SpellAcquisitionOutcomeLikeCpp::Indeterminate(
                SpellAcquisitionIndeterminateLikeCpp::MissingTrainerProjectionMetadata,
            );
        };
        project_spell_acquisition_like_cpp(
            &snapshot,
            SpellAcquisitionMetadataLikeCpp {
                catalog,
                spell_chains,
                spell_learn_skills,
                spell_learn_spells,
                spell_required,
                spell_custom_attributes,
                trait_definitions,
                cast_authority: &FAIL_CLOSED_CAST_AUTHORITY_LIKE_CPP,
                craft_validity_authority: &FAIL_CLOSED_CRAFT_AUTHORITY_LIKE_CPP,
                mounts: self.mount_store().map(AsRef::as_ref),
                skills,
                skill_lines,
                skill_tiers,
            },
            root,
        )
    }

    pub(crate) fn spell_acquisition_snapshot_like_cpp(
        &self,
        lifecycle: PlayerAcquisitionLifecycleLikeCpp,
        future_player_condition_resolutions: Vec<PlayerFuturePlayerConditionResolutionLikeCpp>,
        cast_resolutions: BTreeMap<u32, PlayerCastAcquisitionResolutionLikeCpp>,
    ) -> Result<PlayerSpellAcquisitionSnapshotLikeCpp, SpellAcquisitionSnapshotAdapterErrorLikeCpp>
    {
        let spell_rows = self
            .complete_represented_player_spell_rows_like_cpp()
            .ok_or(SpellAcquisitionSnapshotAdapterErrorLikeCpp::IncompleteSpellRows)?;
        let skill_rows = self
            .complete_player_skill_records_like_cpp()
            .ok_or(SpellAcquisitionSnapshotAdapterErrorLikeCpp::IncompleteSkillRows)?;
        let occupied_skill_slots = self
            .complete_player_skill_occupied_slots_like_cpp()
            .ok_or(SpellAcquisitionSnapshotAdapterErrorLikeCpp::MissingSkillSlotOccupancy)?;
        let traits = self
            .complete_represented_spell_trait_definition_ids_like_cpp()
            .ok_or(SpellAcquisitionSnapshotAdapterErrorLikeCpp::IncompleteTraitDefinitions)?;
        let represented_overrides = self
            .complete_represented_override_spells_like_cpp()
            .ok_or(SpellAcquisitionSnapshotAdapterErrorLikeCpp::IncompleteOverrides)?;
        let mut trait_spell_ids = traits.keys().copied().collect::<Vec<_>>();
        trait_spell_ids.sort_unstable();
        for spell_id in trait_spell_ids {
            if !spell_rows.contains_key(&spell_id) {
                return Err(
                    SpellAcquisitionSnapshotAdapterErrorLikeCpp::OrphanTraitDefinition { spell_id },
                );
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
                        crate::session::RepresentedPlayerSpellStateLikeCpp::Unchanged => {
                            PlayerSpellPersistenceStateLikeCpp::Unchanged
                        }
                        crate::session::RepresentedPlayerSpellStateLikeCpp::Changed => {
                            PlayerSpellPersistenceStateLikeCpp::Changed
                        }
                        crate::session::RepresentedPlayerSpellStateLikeCpp::New => {
                            PlayerSpellPersistenceStateLikeCpp::New
                        }
                        crate::session::RepresentedPlayerSpellStateLikeCpp::Removed => {
                            PlayerSpellPersistenceStateLikeCpp::Removed
                        }
                        crate::session::RepresentedPlayerSpellStateLikeCpp::Temporary => {
                            PlayerSpellPersistenceStateLikeCpp::Temporary
                        }
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
                profession_association:
                    ProfessionAssociationInputLikeCpp::from_database_value_like_cpp(
                        row.profession_slot,
                    ),
                state: match row.state {
                    crate::session::RepresentedPlayerSkillStateLikeCpp::Unchanged => {
                        PlayerSkillPersistenceStateLikeCpp::Unchanged
                    }
                    crate::session::RepresentedPlayerSkillStateLikeCpp::Changed => {
                        PlayerSkillPersistenceStateLikeCpp::Changed
                    }
                    crate::session::RepresentedPlayerSkillStateLikeCpp::New => {
                        PlayerSkillPersistenceStateLikeCpp::New
                    }
                    crate::session::RepresentedPlayerSkillStateLikeCpp::Deleted => {
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
                return Err(
                    SpellAcquisitionSnapshotAdapterErrorLikeCpp::InvalidOverride {
                        overridden_spell_id,
                        overriding_spell_id,
                    },
                );
            };
            overrides.push((overridden_spell_id_u32, overriding_spell_id_u32));
        }

        Ok(PlayerSpellAcquisitionSnapshotLikeCpp {
            spells,
            skills,
            occupied_skill_slots,
            overrides,
            race: self.player_race_like_cpp(),
            class: self.player_class_like_cpp(),
            level: self.player_level_like_cpp(),
            lifecycle,
            future_player_condition_resolutions,
            cast_resolutions,
        })
    }
}
