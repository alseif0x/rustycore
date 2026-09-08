// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Adapt represented EffectLearnSpell to canonical Player access and catalogs.
use super::*;
use crate::spell_acquisition::{EffectLearningRuntimeLikeCpp, *};

impl WorldSession {
    pub(super) async fn apply_learn_spell_effect_like_cpp(
        &mut self,
        spell: i32,
        target: ObjectGuid,
    ) -> bool {
        execute_effect_learning_like_cpp(self, spell, target)
    }
}

impl EffectLearningRuntimeLikeCpp for WorldSession {
    fn admits_learning_target(&self, spell: i32, target: ObjectGuid) -> bool {
        if self.player_guid() != Some(target) || spell <= 0 {
            return false;
        }
        if !self.represented_spell_valid_for_learning_like_cpp(spell) {
            return false;
        }
        // Existing represented limit: account mount acquisition has a richer
        // owner; neither planned nor fallback learning may install half of it.
        self.mount_store().is_some_and(|mounts| {
            mounts
                .get_by_source_spell_id_like_cpp(spell as u32)
                .is_none()
        })
    }

    fn project_learning(&self, spell: u32) -> SpellAcquisitionOutcomeLikeCpp {
        self.project_effect_learn_spell_acquisition_like_cpp(spell)
    }

    fn acquisition_snapshot(
        &self,
    ) -> Result<PlayerSpellAcquisitionSnapshotLikeCpp, SpellAcquisitionSnapshotAdapterErrorLikeCpp>
    {
        self.spell_acquisition_snapshot_like_cpp(
            PlayerAcquisitionLifecycleLikeCpp::InWorld,
            Vec::new(),
            BTreeMap::new(),
        )
    }

    fn profession_capacity(
        &self,
        skills: &[u32],
    ) -> Result<
        crate::profession::PrimaryProfessionCapacityPlanLikeCpp,
        crate::profession::PrimaryProfessionCapacityPlanErrorLikeCpp,
    > {
        self.plan_primary_profession_capacity_like_cpp(skills.iter().copied())
    }

    fn fallback_snapshot(&self) -> Option<wow_entities::PlayerSpellRuntimeState> {
        self.with_player_spell_runtime_like_cpp(Clone::clone)
    }

    fn fallback_chains(&self) -> Option<&SpellChainStoreLikeCpp> {
        self.spell_chain_store.as_deref()
    }

    fn fallback_requirements(&self) -> Option<&SpellRequiredStoreLikeCpp> {
        self.spell_required_store.as_deref()
    }

    fn fallback_traits(&self) -> Option<&wow_data::trait_tree::TraitDefinitionStore> {
        self.trait_definition_store().map(AsRef::as_ref)
    }

    fn install_fallback_row(
        &mut self,
        row: wow_entities::PlayerKnownSpellRecord,
        source: &wow_entities::PlayerSpellRuntimeState,
    ) {
        let spell = row.spell_id;
        if let Some(trait_id) = self
            .mutate_player_spell_runtime_like_cpp(|runtime| {
                runtime.trait_definition_ids.remove(&spell)
            })
            .flatten()
            && let Some(overridden) = u32::try_from(trait_id)
                .ok()
                .and_then(|id| {
                    self.trait_definition_store()
                        .and_then(|store| store.get(id))
                })
                .map(|definition| definition.overrides_spell_id)
                .filter(|id| *id > 0)
        {
            self.remove_represented_override_spell_like_cpp(overridden, spell);
        }
        let complete_rows = source.rows_complete.then(|| source.rows.clone());
        self.learn_known_spell_like_cpp(spell);
        let _ = self.mutate_player_spell_runtime_like_cpp(|runtime| {
            if let Some(mut rows) = complete_rows {
                rows.insert(spell, row);
                runtime.rows = rows;
                runtime.rows_loaded = true;
                runtime.rows_complete = true;
            } else {
                runtime.fallback_rows.insert(spell, row);
            }
        });
        self.sync_player_registry_state_like_cpp();
    }
}
