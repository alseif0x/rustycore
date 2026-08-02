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

const SPELL_AURA_EFFECT_IMMUNITY_LIKE_CPP: i64 = 37;
const SPELL_AURA_STATE_IMMUNITY_LIKE_CPP: i64 = 38;
const SPELL_AURA_MECHANIC_IMMUNITY_LIKE_CPP: i64 = 77;
const SPELL_AURA_MECHANIC_IMMUNITY_MASK_LIKE_CPP: i64 = 147;
const SPELL_AURA_MOD_IMMUNE_AURA_APPLY_SCHOOL_LIKE_CPP: i64 = 267;
const SPELL_EFFECT_ATTRIBUTE_NO_IMMUNITY_LIKE_CPP: i64 = 0x0000_0001;

fn trainer_target_restriction_admits_player_like_cpp(
    store: &wow_data::SpellTargetRestrictionsStore,
    spell_id: u32,
    difficulty_chain: impl IntoIterator<Item = u32>,
) -> bool {
    const CREATURE_TYPEMASK_HUMANOID_LIKE_CPP: u32 = 1 << (7 - 1);
    store
        .resolved_for_difficulty_chain_like_cpp(spell_id, difficulty_chain)
        .is_none_or(|restriction| {
            let mask = restriction.target_creature_type_mask_like_cpp();
            mask == 0 || mask & CREATURE_TYPEMASK_HUMANOID_LIKE_CPP != 0
        })
}

impl crate::session::WorldSession {
    /// Projects one trainer product from the complete current player snapshot.
    ///
    /// Static cast/craft safety and the current player's exact immediate/
    /// hit-target mask are both mandatory. Direct ordinary learns do not
    /// consult those authorities; wrapper paths fail closed if either proof is
    /// absent.
    pub(crate) fn project_trainer_spell_acquisition_like_cpp(
        &self,
        root: SpellAcquisitionRootLikeCpp,
    ) -> SpellAcquisitionOutcomeLikeCpp {
        self.project_player_spell_acquisition_like_cpp(root)
    }

    /// Projects any represented durable player acquisition from the same
    /// complete authority used by trainer offers. The root selects semantics;
    /// this adapter never grants a shallower fallback.
    pub(crate) fn project_player_spell_acquisition_like_cpp(
        &self,
        root: SpellAcquisitionRootLikeCpp,
    ) -> SpellAcquisitionOutcomeLikeCpp {
        self.project_player_spell_acquisition_with_policy_like_cpp(root, false)
    }

    /// C++ `Spell::EffectLearnSpell` always reaches `Player::LearnSpell` even
    /// when AddSpell's subsequent triggered cast cannot yet be represented by
    /// this immutable planner. Preserve that base mutation while recording a
    /// typed deferral for the unsupported cast-side work.
    pub(crate) fn project_effect_learn_spell_acquisition_like_cpp(
        &self,
        spell_id: u32,
    ) -> SpellAcquisitionOutcomeLikeCpp {
        self.project_player_spell_acquisition_with_policy_like_cpp(
            SpellAcquisitionRootLikeCpp::DirectLearn(spell_id),
            true,
        )
    }

    fn project_player_spell_acquisition_with_policy_like_cpp(
        &self,
        root: SpellAcquisitionRootLikeCpp,
        defer_unavailable_cast_side_effects: bool,
    ) -> SpellAcquisitionOutcomeLikeCpp {
        let cast_resolutions = match root {
            SpellAcquisitionRootLikeCpp::DirectLearn(_) => BTreeMap::new(),
            SpellAcquisitionRootLikeCpp::TrainerWrapperCast(spell_id) => {
                let Some(resolution) =
                    self.resolve_trainer_wrapper_cast_acquisition_like_cpp(spell_id)
                else {
                    return SpellAcquisitionOutcomeLikeCpp::Indeterminate(
                        SpellAcquisitionIndeterminateLikeCpp::MissingCastResolution { spell_id },
                    );
                };
                BTreeMap::from([(spell_id, resolution)])
            }
        };
        let snapshot = match self.spell_acquisition_snapshot_like_cpp(
            PlayerAcquisitionLifecycleLikeCpp::InWorld,
            Vec::new(),
            cast_resolutions,
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
        let cast_authority = self
            .spell_acquisition_cast_authority_like_cpp
            .as_deref()
            .unwrap_or(&FAIL_CLOSED_CAST_AUTHORITY_LIKE_CPP);
        let craft_validity_authority = self
            .spell_acquisition_craft_authority_like_cpp
            .as_deref()
            .unwrap_or(&FAIL_CLOSED_CRAFT_AUTHORITY_LIKE_CPP);
        let metadata = SpellAcquisitionMetadataLikeCpp {
            catalog,
            spell_chains,
            spell_learn_skills,
            spell_learn_spells,
            spell_required,
            spell_custom_attributes,
            trait_definitions,
            cast_authority,
            craft_validity_authority,
            mounts: self.mount_store().map(AsRef::as_ref),
            skills,
            skill_lines,
            skill_tiers,
        };
        if defer_unavailable_cast_side_effects {
            let SpellAcquisitionRootLikeCpp::DirectLearn(spell_id) = root else {
                unreachable!("cast-side deferral is exclusive to EffectLearnSpell")
            };
            project_effect_learn_spell_acquisition_like_cpp(&snapshot, metadata, spell_id)
        } else {
            project_spell_acquisition_like_cpp(&snapshot, metadata, root)
        }
    }

    /// Resolve the bounded normal-trainer wrapper against current represented
    /// player state. C++ checks effect immunity per target in `AddUnitTarget`;
    /// ordinary buffs do not suppress unrelated trainer effects. Until the
    /// canonical Unit immunity containers exist, derive the bounded blockers
    /// from complete active-aura effects and negative `SPELL_LINK_AURA` rows.
    /// Missing aura/link authority still fails closed.
    fn resolve_trainer_wrapper_cast_acquisition_like_cpp(
        &self,
        spell_id: u32,
    ) -> Option<PlayerCastAcquisitionResolutionLikeCpp> {
        let catalog = self.spell_acquisition_catalog()?;
        let difficulty_chain = self.current_map_difficulty_chain_for_acquisition_like_cpp();
        let effective_effects = match catalog.resolved_effects_for_difficulty_chain_like_cpp(
            spell_id,
            difficulty_chain.iter().copied(),
        ) {
            SpellAcquisitionResolvedEffectsLookupLikeCpp::Covered(effects) => {
                effects.into_iter().cloned().collect::<Vec<_>>()
            }
            SpellAcquisitionResolvedEffectsLookupLikeCpp::MissingCoverage { .. }
            | SpellAcquisitionResolvedEffectsLookupLikeCpp::Indeterminate(_) => return None,
        };
        // Re-audit the active variant because startup's immutable authority
        // proves the difficulty-none closure. This prevents a heroic/custom
        // override from smuggling an unsupported or non-player effect into
        // the reduced cast path.
        for effect in &effective_effects {
            let effect_type = effect.effect_type_checked().ok()?;
            match effect_type {
                SPELL_EFFECT_LEARN_SPELL | SPELL_EFFECT_SKILL_STEP | SPELL_EFFECT_DUAL_WIELD => {
                    if effect.effect_mechanic_raw != 0
                        || effect.effect_aura_raw != 0
                        || !effect.targets_player_like_cpp()
                    {
                        return None;
                    }
                }
                // C++ `Trainer::TeachSpell` invokes castable wrappers as
                // `player->CastSpell(player, ...)`. EffectSkill is HANDLE_HIT,
                // has no implicit target, and mutates that player caster.
                SPELL_EFFECT_SKILL => {
                    if effect.effect_mechanic_raw != 0 || effect.effect_aura_raw != 0 {
                        return None;
                    }
                }
                0 => {}
                3 if matches!(spell_id, 33_388 | 34_090) => {}
                other if wow_data::spell::spell_effect_types::is_cpp_null_or_unused_noop(other) => {
                }
                _ => return None,
            }
        }
        // C++ copies this single SpellInfo field from the first row in the
        // active difficulty/fallback chain, then checks the player target's
        // HUMANOID mask. Do not combine sibling difficulty rows: a heroic
        // restriction cannot reject a normal cast (or vice versa).
        if !trainer_target_restriction_admits_player_like_cpp(
            self.spell_target_restrictions_store()?,
            spell_id,
            difficulty_chain.iter().copied(),
        ) {
            return None;
        }
        let map_id = u32::from(self.player_map_id_like_cpp());
        let (_, area_id) = self.player_zone_area_like_cpp();
        let map_instance_type = self
            .map_store()
            .and_then(|store| store.get(map_id))
            .map(|entry| entry.instance_type);
        if self.disable_mgr()?.is_disabled_for_like_cpp(
            wow_data::DISABLE_TYPE_SPELL,
            spell_id,
            Some(wow_data::DisableWorldObjectRefLikeCpp {
                // The C++ trainer path casts from the player, not the NPC.
                type_id: wow_constants::TypeId::Player,
                map_id,
                area_id,
                is_pet: false,
                is_battle_arena: map_instance_type == Some(wow_data::MAP_ARENA_LIKE_CPP),
                is_battleground: map_instance_type == Some(wow_data::MAP_BATTLEGROUND_LIKE_CPP),
                player_map_difficulty: None,
            }),
            0,
            self.map_store().map(AsRef::as_ref),
        ) {
            return None;
        }
        let no_immunities = match catalog
            .resolved_misc_for_difficulty_chain_like_cpp(spell_id, difficulty_chain.iter().copied())
        {
            SpellAcquisitionResolvedMetadataLookupLikeCpp::Present(misc) => {
                misc.no_immunities_checked().ok()?
            }
            SpellAcquisitionResolvedMetadataLookupLikeCpp::CoveredWithoutRow => false,
            SpellAcquisitionResolvedMetadataLookupLikeCpp::MissingCoverage { .. }
            | SpellAcquisitionResolvedMetadataLookupLikeCpp::Indeterminate(_) => return None,
        };
        let immunized_effect_mask = self.active_auras_immunized_trainer_effect_mask_like_cpp(
            catalog,
            spell_id,
            &effective_effects,
            no_immunities,
            &difficulty_chain,
        )?;
        let mut executed_hit_target_effect_mask = 0_u32;
        let mut executed_dual_wield_effects = Vec::new();
        for effect in &effective_effects {
            let effect_type = effect.effect_type_checked().ok()?;
            if !matches!(
                effect_type,
                SPELL_EFFECT_LEARN_SPELL
                    | SPELL_EFFECT_SKILL_STEP
                    | SPELL_EFFECT_SKILL
                    | SPELL_EFFECT_DUAL_WIELD
            ) {
                continue;
            }
            let effect_index = effect.effect_index_checked().ok()?;
            let effect_bit = 1_u32.checked_shl(u32::from(effect_index))?;
            if immunized_effect_mask & effect_bit != 0 {
                continue;
            }
            executed_hit_target_effect_mask |= effect_bit;
            if effect_type == SPELL_EFFECT_DUAL_WIELD {
                executed_dual_wield_effects.push(PlayerExecutedDualWieldEffectLikeCpp {
                    effect_record_id: effect.record_id,
                    effect_index,
                });
            }
        }
        Some(PlayerCastAcquisitionResolutionLikeCpp {
            reached_immediate_phase: true,
            executed_hit_target_effect_mask,
            effective_effects,
            executed_dual_wield_effects,
        })
    }

    /// C++ `WorldObject::CastSpell` resolves both the cast and every active
    /// aura's `SpellInfo` from `Map::GetDifficultyID`, walking
    /// `FallbackDifficultyID` exactly as `SpellInfoLoadHelper` does.
    fn current_map_difficulty_chain_for_acquisition_like_cpp(&self) -> Vec<u32> {
        let requested = u32::from(self.current_map_difficulty_id_like_cpp());
        let mut chain = vec![requested];
        let mut visited = BTreeSet::from([requested]);
        let mut current = requested;
        while let Some(difficulty) = self.difficulty_store().and_then(|store| store.get(current)) {
            let fallback = u32::from(difficulty.fallback_difficulty_id);
            if !visited.insert(fallback) {
                break;
            }
            chain.push(fallback);
            current = fallback;
        }
        chain
    }

    fn active_auras_immunized_trainer_effect_mask_like_cpp(
        &self,
        catalog: &SpellAcquisitionCatalogLikeCpp,
        trainer_spell_id: u32,
        trainer_effects: &[SpellAcquisitionEffectLikeCpp],
        no_immunities: bool,
        difficulty_chain: &[u32],
    ) -> Option<u32> {
        let linked = self.spell_linked_store_like_cpp()?;
        let mut immunized_effect_mask = 0_u32;
        for aura in self.visible_auras.values() {
            let aura_spell_id = u32::try_from(aura.spell_id).ok().filter(|id| *id != 0)?;
            if linked
                .get_spell_linked_like_cpp(SpellLinkedTypeLikeCpp::Aura, aura_spell_id)
                .is_some_and(|effects| {
                    effects
                        .iter()
                        .any(|effect| *effect < 0 && effect.unsigned_abs() == trainer_spell_id)
                })
            {
                // C++ `IMMUNITY_ID` rejects the spell rather than one effect.
                return Some(u32::MAX);
            }
            if no_immunities {
                // C++ checks IMMUNITY_ID before SPELL_ATTR0_NO_IMMUNITIES,
                // then bypasses all remaining spell/effect immunities.
                continue;
            }

            let effects = match catalog.resolved_effects_for_difficulty_chain_like_cpp(
                aura_spell_id,
                difficulty_chain.iter().copied(),
            ) {
                SpellAcquisitionResolvedEffectsLookupLikeCpp::Covered(effects) => effects,
                SpellAcquisitionResolvedEffectsLookupLikeCpp::MissingCoverage { .. }
                | SpellAcquisitionResolvedEffectsLookupLikeCpp::Indeterminate(_) => return None,
            };
            let mut unresolved_effect_mask = aura.effect_mask;
            for effect in effects {
                let effect_index = effect.effect_index_checked().ok()?;
                let effect_bit = 1_u32.checked_shl(u32::from(effect_index))?;
                if unresolved_effect_mask & effect_bit == 0 {
                    continue;
                }
                unresolved_effect_mask &= !effect_bit;
                match effect.effect_aura_raw {
                    SPELL_AURA_EFFECT_IMMUNITY_LIKE_CPP => {
                        for trainer_effect in trainer_effects {
                            let trainer_effect_type = trainer_effect.effect_type_checked().ok()?;
                            if trainer_effect.effect_attributes_raw
                                & SPELL_EFFECT_ATTRIBUTE_NO_IMMUNITY_LIKE_CPP
                                == 0
                                && effect.effect_misc_value_raw[0] == i64::from(trainer_effect_type)
                            {
                                let trainer_effect_index =
                                    trainer_effect.effect_index_checked().ok()?;
                                immunized_effect_mask |=
                                    1_u32.checked_shl(u32::from(trainer_effect_index))?;
                            }
                        }
                    }
                    SPELL_AURA_STATE_IMMUNITY_LIKE_CPP => {
                        for trainer_effect in trainer_effects {
                            if trainer_effect.effect_aura_raw != 0
                                && effect.effect_misc_value_raw[0] == trainer_effect.effect_aura_raw
                            {
                                let trainer_effect_index =
                                    trainer_effect.effect_index_checked().ok()?;
                                immunized_effect_mask |=
                                    1_u32.checked_shl(u32::from(trainer_effect_index))?;
                            }
                        }
                    }
                    SPELL_AURA_MECHANIC_IMMUNITY_LIKE_CPP
                    | SPELL_AURA_MECHANIC_IMMUNITY_MASK_LIKE_CPP
                        if trainer_effects
                            .iter()
                            .any(|trainer_effect| trainer_effect.effect_mechanic_raw != 0) =>
                    {
                        // Startup authority excludes this shape. If a stale or
                        // injected catalog violates that invariant, do not
                        // approximate C++'s mechanic-mask switch table.
                        return None;
                    }
                    SPELL_AURA_MOD_IMMUNE_AURA_APPLY_SCHOOL_LIKE_CPP
                        if trainer_effects
                            .iter()
                            .any(|trainer_effect| trainer_effect.effect_aura_raw != 0) =>
                    {
                        return None;
                    }
                    _ => {}
                }
            }
            if unresolved_effect_mask != 0 {
                return None;
            }
        }
        Some(immunized_effect_mask)
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

        let mut primary_profession_skill_ids = skills
            .iter()
            .filter(|skill| {
                skill.state != PlayerSkillPersistenceStateLikeCpp::Deleted && skill.value != 0
            })
            .filter_map(|skill| {
                self.skill_line_store()
                    .and_then(|store| {
                        store
                            .is_primary_profession_skill_like_cpp(skill.skill_id)
                            .map(|is_primary| (skill.skill_id, is_primary))
                    })
                    .and_then(|(skill_id, is_primary)| is_primary.then_some(skill_id))
            })
            .collect::<Vec<_>>();
        primary_profession_skill_ids.sort_unstable();
        let non_durable_skill_tombstone_ids = self
            .player_skill_non_durable_tombstones_like_cpp()
            .iter()
            .map(|skill_id| u32::from(*skill_id))
            .collect();

        Ok(PlayerSpellAcquisitionSnapshotLikeCpp {
            character_guid: self.player_guid(),
            spells,
            skills,
            occupied_skill_slots,
            overrides,
            primary_profession_skill_ids,
            non_durable_skill_tombstone_ids,
            race: self.player_race_like_cpp(),
            class: self.player_class_like_cpp(),
            level: self.player_level_like_cpp(),
            lifecycle,
            future_player_condition_resolutions,
            cast_resolutions,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::trainer_target_restriction_admits_player_like_cpp;
    use wow_data::{SpellTargetRestrictionsEntry, SpellTargetRestrictionsStore};

    fn row(id: u32, difficulty_id: u8, target_creature_type: i16) -> SpellTargetRestrictionsEntry {
        SpellTargetRestrictionsEntry {
            id,
            difficulty_id,
            cone_degrees: 0.0,
            max_targets: 0,
            max_target_level: 0,
            target_creature_type,
            targets: 0,
            width: 0.0,
            spell_id: 100,
        }
    }

    #[test]
    fn trainer_target_restriction_uses_active_row_without_merging_siblings_like_cpp() {
        let store = SpellTargetRestrictionsStore::from_entries([
            row(1, 0, 1 << (3 - 1)),
            row(2, 2, 1 << (7 - 1)),
        ]);

        assert!(trainer_target_restriction_admits_player_like_cpp(
            &store,
            100,
            [2, 0]
        ));
        assert!(!trainer_target_restriction_admits_player_like_cpp(
            &store,
            100,
            [0]
        ));
        assert!(trainer_target_restriction_admits_player_like_cpp(
            &store,
            200,
            [2, 0]
        ));
    }
}
