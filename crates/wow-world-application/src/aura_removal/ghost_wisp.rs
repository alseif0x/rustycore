// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Canonical participants for the two resurrection ghost auras.
//!
//! C++ anchors: `SpellAuraEffects.cpp:1718-1742` (ghost flag and server-side
//! visibility), `:1935-2131` (transform ownership/display), `:3159-3174` and
//! `:3210-3218` (run/swim speed updates), and `Unit.cpp:10038-10100`,
//! `:11763-11870` (display restoration and form selection).

use super::{AuraRemovalCxLikeCpp, RemovedAuraLikeCpp};
use wow_entities::AuraApplicationLikeCpp;

const GHOST_SPELL: i32 = 8326;
const WISP_SPELL: i32 = 20584;
const AURA_FLAG_POSITIVE: u32 = 0x0100;

struct DisplayRestorePlan {
    display_id: u32,
    transform: Option<AuraApplicationLikeCpp>,
}

pub(super) struct PreparedGhostWispRemoval {
    slot: u8,
    spell_id: i32,
    effect_mask: u32,
    effect_types: Vec<i32>,
    clear_ghost_flag: bool,
    display_restore: Option<DisplayRestorePlan>,
}

impl AuraRemovalCxLikeCpp<'_> {
    pub fn apply_ghost_wisp_aura(
        &mut self,
        spell_id: i32,
        caster_guid: wow_core::ObjectGuid,
        spell_info: &wow_data::SpellInfo,
        duration_ms: u32,
        aura_flags: u32,
        effect_mask: u32,
        provenance: wow_entities::AuraCastProvenanceLikeCpp,
    ) -> Result<(), &'static str> {
        let expected = match spell_id {
            GHOST_SPELL => [
                wow_data::spell::aura_types::SPELL_AURA_GHOST,
                wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SPEED,
                wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SWIM_SPEED,
            ],
            WISP_SPELL => [
                wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SPEED,
                wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SWIM_SPEED,
                wow_data::spell::aura_types::SPELL_AURA_TRANSFORM,
            ],
            _ => return Err("Spell is not a selected resurrection ghost aura"),
        };
        if duration_ms != 0 || effect_mask != 0b111 {
            return Err("Resurrection ghost aura is not the expected permanent three-effect aura");
        }
        let mut selected = spell_info
            .effects()
            .iter()
            .filter(|effect| {
                effect.effect == wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA
                    && effect_mask & 1u32.checked_shl(effect.effect_index).unwrap_or(0) != 0
            })
            .collect::<Vec<_>>();
        selected.sort_by_key(|effect| effect.effect_index);
        if selected.len() != expected.len()
            || selected
                .iter()
                .zip(expected)
                .any(|(effect, aura_type)| effect.effect_aura != aura_type)
        {
            return Err("Resurrection ghost aura effects differ from the selected DB2 contract");
        }

        let aura_snapshot = self
            .player
            .aura_subsystem_snapshot_like_cpp()
            .ok_or("Missing canonical Player aura subsystem while applying resurrection aura")?;
        let mut same_caster = aura_snapshot
            .runtime_applications_like_cpp()
            .values()
            .filter(|aura| aura.spell_id == spell_id && aura.caster_guid == caster_guid)
            .collect::<Vec<_>>();
        same_caster.sort_by_key(|aura| aura.slot);
        if same_caster.len() > 1 {
            return Err("Multiple canonical resurrection aura applications share this caster");
        }

        let effect_updates = selected
            .iter()
            .map(|effect| {
                let bit = 1_u32
                    .checked_shl(effect.effect_index)
                    .ok_or("Invalid selected resurrection aura effect index")?;
                Ok((
                    bit,
                    effect.effect_aura,
                    effect.calc_value_no_caster_like_cpp(),
                    effect.effect_misc_value_1,
                ))
            })
            .collect::<Result<Vec<_>, &'static str>>()?;
        let base_amounts = selected
            .iter()
            .map(|effect| {
                Ok((
                    u8::try_from(effect.effect_index)
                        .map_err(|_| "Invalid selected resurrection aura base index")?,
                    effect.effect_base_points,
                ))
            })
            .collect::<Result<Vec<_>, &'static str>>()?;
        if let Some(existing) = same_caster.first() {
            if existing.effect_mask != effect_mask {
                return Err("Existing resurrection aura effect mask differs from selected contract");
            }
            let (run_amount_changed, swim_amount_changed) = self
                .player
                .refresh_resurrection_aura_application(
                    spell_id,
                    caster_guid,
                    existing.slot,
                    effect_mask,
                    &effect_updates,
                    &base_amounts,
                )
                .ok_or("Canonical Player disappeared while refreshing resurrection aura")??;
            if run_amount_changed {
                self.mount
                    .recompute_represented_run_speed_rate_like_cpp(&self.player);
            }
            if swim_amount_changed {
                self.mount
                    .recompute_represented_swim_speed_rate_like_cpp(&self.player);
            }
            return Ok(());
        }

        let slot = (0..u8::MAX)
            .find(|slot| !aura_snapshot.runtime_applications_like_cpp().contains_key(slot))
            .ok_or("No free aura slot for resurrection ghost aura")?;

        // Validate the Wisp template before installing the aura so missing
        // selected data cannot leave a partially applied transform.
        let transform_display = if spell_id == WISP_SPELL {
            let effect = selected
                .iter()
                .find(|effect| {
                    effect.effect_aura == wow_data::spell::aura_types::SPELL_AURA_TRANSFORM
                })
                .ok_or("Wisp Spirit transform effect is missing")?;
            Some(self.choose_creature_display(effect.effect_misc_value_1)?)
        } else {
            None
        };

        self.apply_aura_with_effect_mask_provenance_and_update_like_cpp(
            spell_id,
            caster_guid,
            duration_ms,
            aura_flags,
            effect_mask,
            provenance,
            false,
        )?;

        for effect in selected {
            match effect.effect_aura {
                wow_data::spell::aura_types::SPELL_AURA_GHOST => {
                    if !self.player.set_ghost_aura(true) {
                        return Err("Canonical Player disappeared while applying ghost aura");
                    }
                }
                wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SPEED => self
                    .mount
                    .recompute_represented_run_speed_rate_like_cpp(&self.player),
                wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SWIM_SPEED => self
                    .mount
                    .recompute_represented_swim_speed_rate_like_cpp(&self.player),
                wow_data::spell::aura_types::SPELL_AURA_TRANSFORM => {}
                _ => unreachable!("effect contract checked above"),
            }
        }
        if let Some(display) = transform_display {
            let aura = self
                .player
                .aura_subsystem_snapshot_like_cpp()
                .and_then(|auras| auras.runtime_application_like_cpp(slot).cloned())
                .ok_or("Canonical Wisp AuraApplication disappeared after insertion")?;
            if !self.spell.apply_represented_transform_aura_with_access_like_cpp(
                &mut self.player,
                self.spell_store.map(|store| store.as_ref()),
                &aura,
            ) {
                return Err("Wisp transform was not admitted by the canonical transform owner");
            }
            let selected = self
                .player
                .aura_subsystem_snapshot_like_cpp()
                .is_some_and(|auras| auras.transform_spell_like_cpp() == spell_id);
            if selected {
                self.player
                    .set_player_display_model(display)
                    .ok_or("Canonical Player disappeared while setting Wisp display")?;
            }
        }
        self.player
            .set_visible_aura_update_pending(slot, true)
            .ok_or("Canonical Player disappeared while queueing aura update")?;
        Ok(())
    }

    pub fn apply_loaded_ghost_wisp_aura(
        &mut self,
        aura: &AuraApplicationLikeCpp,
    ) -> Result<(), &'static str> {
        match aura.spell_id {
            GHOST_SPELL => {
                if !self.player.set_ghost_aura(true) {
                    return Err("Canonical Player disappeared while restoring ghost aura");
                }
                self.mount
                    .recompute_represented_run_speed_rate_like_cpp(&self.player);
                self.mount
                    .recompute_represented_swim_speed_rate_like_cpp(&self.player);
            }
            WISP_SPELL => {
                self.mount
                    .recompute_represented_run_speed_rate_like_cpp(&self.player);
                self.mount
                    .recompute_represented_swim_speed_rate_like_cpp(&self.player);
                let snapshot = self
                    .player
                    .aura_subsystem_snapshot_like_cpp()
                    .ok_or("Missing canonical Player aura subsystem while restoring Wisp")?;
                let plan = self.prepare_display_restore(&snapshot, None)?;
                self.apply_display_restore(&plan)?;
            }
            _ => return Err("Aura is not a selected resurrection ghost aura"),
        }
        self.player
            .set_visible_aura_update_pending(aura.slot, true)
            .ok_or("Canonical Player disappeared while queueing restored aura update")?;
        Ok(())
    }

    pub(super) fn prepare_ghost_wisp_removal(
        &self,
        slot: u8,
    ) -> Result<Option<PreparedGhostWispRemoval>, &'static str> {
        let Some(visible_auras) = self.player.visible_auras_snapshot_like_cpp() else {
            return Ok(None);
        };
        let Some(visible_aura) = visible_auras.get(&slot) else {
            return Ok(None);
        };
        if !matches!(visible_aura.spell_id, GHOST_SPELL | WISP_SPELL) {
            return Ok(None);
        }
        let snapshot = self
            .player
            .aura_subsystem_snapshot_like_cpp()
            .ok_or("Missing canonical Player aura subsystem during aura removal")?;
        let aura = snapshot
            .runtime_application_like_cpp(slot)
            .ok_or("Aura slot not found")?;
        let spell = self
            .spell_store
            .and_then(|store| store.get(aura.spell_id))
            .ok_or("Missing resurrection ghost aura SpellInfo during removal")?;
        let effect_types = spell
            .effects()
            .iter()
            .filter(|effect| {
                effect.effect == wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA
                    && aura.effect_mask & 1u32.checked_shl(effect.effect_index).unwrap_or(0) != 0
            })
            .map(|effect| effect.effect_aura)
            .collect::<Vec<_>>();
        let expected = if aura.spell_id == GHOST_SPELL {
            [
                wow_data::spell::aura_types::SPELL_AURA_GHOST,
                wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SPEED,
                wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SWIM_SPEED,
            ]
        } else {
            [
                wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SPEED,
                wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SWIM_SPEED,
                wow_data::spell::aura_types::SPELL_AURA_TRANSFORM,
            ]
        };
        let mut actual = effect_types.clone();
        actual.sort_unstable();
        let mut expected_sorted = expected;
        expected_sorted.sort_unstable();
        if actual != expected_sorted {
            return Err("Resurrection ghost aura effects differ from the selected DB2 contract");
        }
        let clear_ghost_flag = aura.spell_id == GHOST_SPELL
            && !snapshot
                .applied_aura_types
                .get(&wow_data::spell::aura_types::SPELL_AURA_GHOST)
                .into_iter()
                .flatten()
                .any(|active| active.slot != slot);
        let display_restore = if aura.spell_id == WISP_SPELL {
            Some(self.prepare_display_restore(&snapshot, Some(slot))?)
        } else {
            None
        };
        Ok(Some(PreparedGhostWispRemoval {
            slot,
            spell_id: aura.spell_id,
            effect_mask: aura.effect_mask,
            effect_types,
            clear_ghost_flag,
            display_restore,
        }))
    }

    pub(super) fn remove_ghost_wisp_effects(
        &mut self,
        removed: &RemovedAuraLikeCpp,
        prepared: Option<PreparedGhostWispRemoval>,
    ) -> Result<(), &'static str> {
        let Some(prepared) = prepared else {
            return Ok(());
        };
        if prepared.slot != removed.aura.slot
            || prepared.spell_id != removed.aura.spell_id
            || prepared.effect_mask != removed.aura.effect_mask
        {
            return Err("Resurrection aura changed during canonical removal");
        }
        self.player
            .set_visible_aura_update_pending(removed.aura.slot, false)
            .ok_or("Canonical Player disappeared while clearing aura update")?;
        for aura_type in prepared.effect_types {
            match aura_type {
                wow_data::spell::aura_types::SPELL_AURA_GHOST => {
                    if prepared.clear_ghost_flag && !self.player.set_ghost_aura(false) {
                        return Err("Canonical Player disappeared while removing ghost aura");
                    }
                }
                wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SPEED => self
                    .mount
                    .recompute_represented_run_speed_rate_like_cpp(&self.player),
                wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SWIM_SPEED => self
                    .mount
                    .recompute_represented_swim_speed_rate_like_cpp(&self.player),
                wow_data::spell::aura_types::SPELL_AURA_TRANSFORM => {
                    self.apply_display_restore(
                        prepared
                            .display_restore
                            .as_ref()
                            .ok_or("Wisp display restoration was not prepared")?,
                    )?;
                }
                _ => return Err("Unsupported effect in resurrection aura removal"),
            }
        }
        Ok(())
    }

    fn choose_creature_display(&self, entry: i32) -> Result<u32, &'static str> {
        let entry = u32::try_from(entry)
            .ok()
            .filter(|entry| *entry != 0)
            .ok_or("Transform creature entry is invalid")?;
        let template = self
            .creature_template_store
            .and_then(|store| store.get(entry))
            .ok_or("Transform creature template is unavailable")?;
        let model_info = self
            .creature_model_info_store
            .ok_or("Creature model info catalog is unavailable")?;
        let mut random = rand::thread_rng();
        choose_transform_template_display(template, model_info, &mut random)
            .filter(|display| *display != 0)
            .ok_or("Transform creature template has no valid display model")
    }

    fn prepare_display_restore(
        &self,
        snapshot: &wow_entities::AuraSubsystem,
        excluded_slot: Option<u8>,
    ) -> Result<DisplayRestorePlan, &'static str> {
        let mounted = self
            .player
            .resolved_player_mounted_like_cpp()
            .ok_or("Missing canonical mounted state during display restoration")?;
        let (race, sex, form, _, native_display) = self
            .player
            .player_shape_display_state()
            .ok_or("Canonical Player disappeared during display restoration")?;
        let mut transforms = snapshot
            .applied_aura_types
            .get(&wow_data::spell::aura_types::SPELL_AURA_TRANSFORM)
            .into_iter()
            .flatten()
            .rev()
            .filter(|applied| Some(applied.slot) != excluded_slot)
            .filter_map(|applied| {
                snapshot
                    .runtime_application_like_cpp(applied.slot)
                    .filter(|aura| aura.effect_mask & applied.effect_mask != 0)
                    .cloned()
            })
            .collect::<Vec<_>>();

        let mut selected_transform: Option<AuraApplicationLikeCpp> = None;
        for transform in transforms {
            let positive = transform.aura_flags & AURA_FLAG_POSITIVE != 0;
            if selected_transform.is_none() {
                if !mounted {
                    selected_transform = Some(transform.clone());
                } else if positive {
                    // C++ `RestoreDisplayId(true)` admits positive transform
                    // effects only when their template model can be mounted.
                    if let Some(entry) = transform_misc_value(snapshot, &transform)
                        && let Some(template_store) = self.creature_template_store
                    {
                        let Ok(entry) = u32::try_from(entry) else {
                            continue;
                        };
                        if entry == 0 || template_store.get(entry).is_none() {
                            continue;
                        }
                        let display = self.choose_creature_display(entry as i32)?;
                        if !self.transform_display_disallowed_while_mounted(
                            &transform,
                            display,
                            native_display,
                        )? {
                            selected_transform = Some(transform.clone());
                        }
                    }
                } else {
                    selected_transform = Some(transform.clone());
                }
            }
            if !positive {
                selected_transform = Some(transform);
                break;
            }
        }

        if let Some(transform) = selected_transform {
            // The mount filter makes its own ChooseDisplayId call. The selected
            // aura applies again below using AuraEffect's ChooseDisplayId path.
            let display = self.transform_display(snapshot, &transform)?;
            return Ok(DisplayRestorePlan {
                display_id: display,
                transform: Some(transform),
            });
        }

        let shapeshift = snapshot
            .applied_aura_types
            .get(&wow_data::spell::aura_types::SPELL_AURA_MOD_SHAPESHIFT)
            .and_then(|auras| {
                auras
                    .iter()
                    .rev()
                    .find(|aura| Some(aura.slot) != excluded_slot)
            })
            .and_then(|applied| snapshot.runtime_application_like_cpp(applied.slot));
        let display = if let Some(shapeshift) = shapeshift {
            if self.shapeshift_display_links_absent != Some(true) {
                return Err(
                    "Shapeshift display links are not certified absent; full customization resolution belongs to #32",
                );
            }
            let Some(display) = self.model_for_form(form, shapeshift.spell_id, race, sex, snapshot)?
            else {
                return Ok(DisplayRestorePlan {
                    display_id: native_display,
                    transform: None,
                });
            };
            if mounted
                && !self.transform_or_form_display_allowed_while_mounted(
                    0,
                    form,
                    display,
                    native_display,
                )?
            {
                native_display
            } else {
                display
            }
        } else {
            native_display
        };
        Ok(DisplayRestorePlan {
            display_id: display,
            transform: None,
        })
    }

    fn transform_display(
        &self,
        snapshot: &wow_entities::AuraSubsystem,
        transform: &AuraApplicationLikeCpp,
    ) -> Result<u32, &'static str> {
        let entry = transform_misc_value(snapshot, transform)
            .ok_or("Transform aura has no represented misc value")?;
        if entry == 0 {
            return Err("Transform aura with misc value zero has no selected spell model");
        }
        self.choose_creature_display(entry)
    }

    fn transform_display_disallowed_while_mounted(
        &self,
        transform: &AuraApplicationLikeCpp,
        display: u32,
        native_display: u32,
    ) -> Result<bool, &'static str> {
        if self
            .spell_store
            .is_some_and(|store| {
                store.has_attribute_for_difficulty_like_cpp(
                    transform.spell_id,
                    self.map_difficulty_id,
                    self.difficulty_store,
                    0,
                    wow_data::spell::attributes::SPELL_ATTR0_ALLOW_WHILE_MOUNTED,
                )
            })
        {
            return Ok(false);
        }
        self.transform_or_form_display_allowed_while_mounted(
            transform.spell_id,
            0,
            display,
            native_display,
        )
        .map(|allowed| !allowed)
    }

    fn transform_or_form_display_allowed_while_mounted(
        &self,
        spell_id: i32,
        form: u32,
        display: u32,
        native_display: u32,
    ) -> Result<bool, &'static str> {
        if spell_id != 0
            && self
                .spell_store
                .is_some_and(|store| {
                    store.has_attribute_for_difficulty_like_cpp(
                        spell_id,
                        self.map_difficulty_id,
                        self.difficulty_store,
                        0,
                        wow_data::spell::attributes::SPELL_ATTR0_ALLOW_WHILE_MOUNTED,
                    )
                })
        {
            return Ok(true);
        }
        wow_world_spell::disallowed_mount_form_from_catalogs(
            form,
            display,
            native_display,
            self.form_store,
            self.display_info_store,
            self.display_info_extra_store,
            self.model_data_store,
            self.races_store,
        )
        .map(|disallowed| !disallowed)
        .ok_or("Mount form metadata is unavailable during display restoration")
    }

    fn apply_display_restore(
        &mut self,
        plan: &DisplayRestorePlan,
    ) -> Result<(), &'static str> {
        if let Some(transform) = plan.transform.as_ref()
            && !self.spell.apply_represented_transform_aura_with_access_like_cpp(
                &mut self.player,
                self.spell_store.map(|store| store.as_ref()),
                transform,
            )
        {
            return Err("Selected transform aura was not applied by its canonical owner");
        }
        self.player
            .set_player_display_model(plan.display_id)
            .ok_or("Canonical Player disappeared while restoring display")?;
        Ok(())
    }

    fn model_for_form(
        &self,
        form: u32,
        spell_id: i32,
        race: u8,
        sex: u8,
        auras: &wow_entities::AuraSubsystem,
    ) -> Result<Option<u32>, &'static str> {
        if spell_id == 7090 {
            return Ok(Some(29_414));
        }
        if spell_id == 35_200 {
            return Ok(Some(4_877));
        }
        if spell_id == 24_858 && aura_snapshot_has_spell(auras, 114_301) {
            // C++ GetModelForForm returns zero for Glyph of Stars; RestoreDisplayId
            // falls through to the native display.
            return Ok(None);
        }
        if form == 16 && aura_snapshot_has_spell(auras, 58_135) {
            return Ok(Some(60_247));
        }
        let Some(form_store) = self.form_store else {
            return Err("SpellShapeshiftForm catalog is unavailable during display restoration");
        };
        let Some(entry) = form_store.get(form) else {
            return Ok(None);
        };
        let team = wow_world_core::session::player_team_for_race_cpp(race);
        let model_index = if team == wow_constants::Team::Horde {
            1
        } else {
            0
        };
        let model = entry.creature_display_id[model_index];
        if model == 0 && team == wow_constants::Team::Horde {
            Ok((entry.creature_display_id[0] != 0).then_some(entry.creature_display_id[0]))
        } else {
            let _ = sex;
            Ok((model != 0).then_some(model))
        }
    }
}

fn aura_snapshot_has_spell(snapshot: &wow_entities::AuraSubsystem, spell_id: u32) -> bool {
    snapshot.has_aura_spell_like_cpp(spell_id)
        || snapshot
            .runtime_applications_like_cpp()
            .values()
            .any(|aura| u32::try_from(aura.spell_id).ok() == Some(spell_id))
}

fn transform_misc_value(
    snapshot: &wow_entities::AuraSubsystem,
    aura: &AuraApplicationLikeCpp,
) -> Option<i32> {
    snapshot
        .threat_snapshot_like_cpp(aura.slot)?
        .effects()
        .iter()
        .find(|(effect_bit, aura_type, _, _)| {
            *aura_type == wow_data::spell::aura_types::SPELL_AURA_TRANSFORM
                && aura.effect_mask & *effect_bit != 0
        })
        .map(|(_, _, _, misc)| *misc)
}

fn choose_transform_template_display(
    template: &wow_data::CreatureTemplateLifecycleRecordLikeCpp,
    model_info: &wow_data::CreatureModelInfoStoreLikeCpp,
    random: &mut impl wow_data::CreatureModelSelectionRandomLikeCpp,
) -> Option<u32> {
    if template.flags_extra & wow_constants::CreatureFlagsExtra::TRIGGER.bits() != 0 {
        Some(
            template
                .first_invisible_model_like_cpp(model_info)
                .creature_display_id,
        )
    } else {
        template
            .random_valid_model_like_cpp(random)
            .or_else(|| Some(template.first_invisible_model_like_cpp(model_info)))
            .map(|model| model.creature_display_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FixedModelSelection {
        other_gender: bool,
    }

    impl wow_data::CreatureModelSelectionRandomLikeCpp for FixedModelSelection {
        fn weighted_model_roll_like_cpp(&mut self, _total_weight: f32) -> f32 {
            0.0
        }

        fn other_gender_roll_zero_like_cpp(&mut self) -> bool {
            self.other_gender
        }
    }

    #[test]
    fn transform_display_uses_choose_display_id_without_gender_substitution() {
        let template = wow_data::CreatureTemplateLifecycleRecordLikeCpp {
            entry: 12_861,
            models: vec![wow_data::CreatureTemplateLifecycleModelLikeCpp {
                creature_display_id: 1_825,
                display_scale: 1.0,
                probability: 100.0,
            }],
            ..Default::default()
        };
        let model_info = wow_data::CreatureModelInfoStoreLikeCpp::from_entries([
            wow_data::CreatureModelInfoLikeCpp {
                display_id: 1_825,
                bounding_radius: 0.5,
                combat_reach: 1.5,
                display_id_other_gender: 1_826,
                is_trigger: false,
            },
            wow_data::CreatureModelInfoLikeCpp {
                display_id: 1_826,
                bounding_radius: 0.5,
                combat_reach: 1.5,
                display_id_other_gender: 0,
                is_trigger: false,
            },
        ]);
        let mut random = FixedModelSelection { other_gender: true };

        assert_eq!(
            choose_transform_template_display(&template, &model_info, &mut random),
            Some(1_825),
            "ObjectMgr::ChooseDisplayId does not apply spawn gender randomization"
        );
    }

    #[test]
    fn transform_effect_snapshot_field_is_a_mask_not_an_index() {
        let mut snapshot = wow_entities::AuraSubsystem::default();
        snapshot.insert_threat_snapshot_like_cpp(
            0,
            wow_entities::AuraThreatSnapshotLikeCpp::new(
                [0, 0],
                vec![(0b100, wow_data::spell::aura_types::SPELL_AURA_TRANSFORM, 0, 12_861)],
            ),
        );
        let aura = AuraApplicationLikeCpp {
            spell_id: WISP_SPELL,
            difficulty_id: 0,
            caster_guid: wow_core::ObjectGuid::EMPTY,
            slot: 0,
            duration_total: 0,
            duration_remaining: 0,
            stack_count: 1,
            aura_flags: 0,
            effect_mask: 0b111,
            aura_interrupt_flags: 0,
            aura_interrupt_flags2: 0,
            represented_effect: None,
            represented_amount: 0,
            represented_effect_amounts: Vec::new(),
            represented_misc_value: None,
            represented_multiplier: 1.0,
            applied_at: std::time::Instant::now(),
        };

        assert_eq!(transform_misc_value(&snapshot, &aura), Some(12_861));
    }

    #[test]
    fn transform_display_falls_back_to_first_invisible_model_when_no_random_model_exists() {
        let template = wow_data::CreatureTemplateLifecycleRecordLikeCpp::default();
        let model_info = wow_data::CreatureModelInfoStoreLikeCpp::from_entries([]);
        let mut random = FixedModelSelection { other_gender: false };

        assert_eq!(
            choose_transform_template_display(&template, &model_info, &mut random),
            Some(wow_data::creature::DEFAULT_INVISIBLE_CREATURE_DISPLAY_ID_LIKE_CPP)
        );
    }
}
