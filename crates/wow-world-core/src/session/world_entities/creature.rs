use std::sync::Arc;

use crate::session::{
    CreatureCreateDisplaySelectionLikeCpp, CreatureCreateModelScalarsLikeCpp,
    state::SessionCore,
};
use rand::Rng;
use wow_constants::{CreatureFlagsExtra, PowerType};
use wow_core::ObjectGuid;

impl SessionCore {
    pub fn mutate_canonical_creature_by_guid_like_cpp<R>(
        &mut self,
        guid: ObjectGuid,
        f: impl FnOnce(&mut wow_entities::Creature) -> R,
    ) -> Option<R> {
        let map_key = self
            .canonical_object_lookup_map_key_like_cpp(u32::from(self.player_map_id_like_cpp()))?;
        let manager = Arc::clone(self.canonical_map_manager.as_ref()?);
        let mut manager = manager.lock().ok()?;
        let managed = manager.find_map_mut(map_key.map_id, map_key.instance_id)?;
        let creature = managed.map_mut().get_typed_creature_mut(guid)?;
        Some(f(creature))
    }

    pub fn world_creature_guids(&self) -> Vec<ObjectGuid> {
        let (map_id, instance_id) = self.current_legacy_runtime_map_key_like_cpp();
        if let Some(manager) = &self.map_manager {
            return manager
                .read()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .creature_guids(map_id, instance_id);
        }

        Vec::new()
    }
}

impl crate::session::state::SessionCatalogs {
    pub fn creature_create_model_scalars_like_cpp(
        &self,
        display_id: u32,
        object_scale: f32,
        display_scale: f32,
    ) -> Option<CreatureCreateModelScalarsLikeCpp> {
        let model = self.creatures.model_info_store.as_ref()?.get(display_id)?;
        let display_scale = if display_scale <= 0.0 {
            1.0
        } else {
            display_scale
        };
        let hover_height = self
            .creatures
            .display_info_store
            .as_ref()
            .and_then(|display_store| display_store.get(display_id))
            .and_then(|display| {
                self.creatures
                    .model_data_store
                    .as_ref()
                    .and_then(|model_store| model_store.get(u32::from(display.model_id)))
                    .map(|model_data| {
                        model_data.hover_height
                            * model_data.model_scale
                            * display.creature_model_scale
                            * display_scale
                    })
            })
            .filter(|height| *height > 0.0)
            .unwrap_or(1.0);
        Some(CreatureCreateModelScalarsLikeCpp {
            display_scale,
            native_x_display_scale: display_scale,
            bounding_radius: model.bounding_radius * object_scale * display_scale,
            combat_reach: model.combat_reach * object_scale * display_scale,
            hover_height,
        })
    }

    pub fn choose_creature_display_like_cpp(
        &self,
        entry: u32,
        spawn_display_id: u32,
        template_flags_extra: u32,
        fallback_template_display_id: u32,
        fallback_template_display_scale: f32,
    ) -> Option<CreatureCreateDisplaySelectionLikeCpp> {
        // C++ `ObjectMgr::LoadCreatures` stores creature.modelid as
        // CreatureData::display with DEFAULT_PLAYER_DISPLAY_SCALE.
        if spawn_display_id != 0 {
            return Some(CreatureCreateDisplaySelectionLikeCpp {
                display_id: spawn_display_id,
                display_scale: 1.0,
            });
        }

        let template = self
            .creatures
            .template_lifecycle_store_like_cpp
            .as_ref()
            .and_then(|store| store.get(entry));
        let mut selected = template.and_then(|template| {
            if template_flags_extra & CreatureFlagsExtra::TRIGGER.bits() != 0 {
                let model_info_store = self.creatures.model_info_store.as_ref()?;
                template
                    .models
                    .iter()
                    .copied()
                    .find(|model| {
                        model_info_store
                            .get(model.creature_display_id)
                            .is_some_and(|info| info.is_trigger)
                    })
                    .or(Some(wow_data::CreatureTemplateLifecycleModelLikeCpp {
                        creature_display_id: 11686,
                        display_scale: 1.0,
                        probability: 1.0,
                    }))
            } else {
                match template.models.as_slice() {
                    [] => None,
                    [model] => Some(*model),
                    models => {
                        let total: f32 =
                            models.iter().map(|model| model.probability.max(0.0)).sum();
                        if total <= f32::EPSILON {
                            models.first().copied()
                        } else {
                            let mut roll = rand::thread_rng().gen_range(0.0..total);
                            let mut picked = *models.last()?;
                            for model in models {
                                roll -= model.probability.max(0.0);
                                if roll <= 0.0 {
                                    picked = *model;
                                    break;
                                }
                            }
                            Some(picked)
                        }
                    }
                }
            }
        });

        if selected.is_none() && fallback_template_display_id != 0 {
            selected = Some(wow_data::CreatureTemplateLifecycleModelLikeCpp {
                creature_display_id: fallback_template_display_id,
                display_scale: if fallback_template_display_scale <= 0.0 {
                    1.0
                } else {
                    fallback_template_display_scale
                },
                probability: 1.0,
            });
        }

        let mut selected = selected?;
        if let Some(other_gender) = self
            .creatures
            .model_info_store
            .as_ref()
            .and_then(|store| store.get(selected.creature_display_id))
            .map(|info| info.display_id_other_gender)
            .filter(|id| *id != 0)
        {
            if rand::thread_rng().gen_range(0..=1) == 0 {
                selected.creature_display_id = other_gender;
                if let Some(template_model) = template.and_then(|template| {
                    template
                        .models
                        .iter()
                        .copied()
                        .find(|model| model.creature_display_id == other_gender)
                }) {
                    selected = template_model;
                }
            }
        }

        Some(CreatureCreateDisplaySelectionLikeCpp {
            display_id: selected.creature_display_id,
            display_scale: selected.display_scale,
        })
    }
}

impl crate::session::state::SessionCatalogs {
    pub fn creature_display_power_for_class_like_cpp(&self, unit_class: u8) -> u8 {
        self.chr
            .classes_store
            .as_ref()
            .and_then(|store| store.get(u32::from(unit_class)))
            .map(|entry| entry.display_power)
            .unwrap_or_else(|| match unit_class {
                1 => PowerType::Rage as u8,
                4 => PowerType::Energy as u8,
                6 => PowerType::RunicPower as u8,
                _ => PowerType::Mana as u8,
            })
    }
}
