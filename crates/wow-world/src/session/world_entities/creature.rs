//! Represented creature state owned by the Session boundary.
//!
//! Moved out of the Session root under #599. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(crate) fn mutate_canonical_creature_by_guid_like_cpp<R>(
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
    pub fn set_canonical_creature_private_object_owner_like_cpp(
        &mut self,
        guid: ObjectGuid,
        owner: ObjectGuid,
    ) -> bool {
        let mut updated = self
            .mutate_canonical_creature_by_guid_like_cpp(guid, |creature| {
                creature.unit_mut().set_private_object_owner_like_cpp(owner);
            })
            .is_some();

        if let Some(manager) = self.map_manager.as_ref() {
            let mut manager = manager
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if let Some(creature) =
                manager.find_creature_mut(self.player_map_id_like_cpp(), 0, guid)
            {
                creature
                    .creature
                    .unit_mut()
                    .set_private_object_owner_like_cpp(owner);
                updated = true;
            }
        }

        updated
    }
    pub(crate) fn world_creature_guids(&self) -> Vec<ObjectGuid> {
        let (map_id, instance_id) = self.current_legacy_runtime_map_key_like_cpp();
        if let Some(manager) = &self.map_manager {
            return manager
                .read()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .creature_guids(map_id, instance_id);
        }

        Vec::new()
    }
    pub(crate) fn active_world_creature_guids_for_update_like_cpp(&self) -> Vec<ObjectGuid> {
        let Some(player_position) = self.player_position_like_cpp() else {
            return Vec::new();
        };
        let Some(manager) = &self.map_manager else {
            return Vec::new();
        };
        let Some(player_phase_shift) = self.represented_player_phase_shift_like_cpp() else {
            return Vec::new();
        };
        let (map_id, instance_id) = self.current_legacy_runtime_map_key_like_cpp();
        manager
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .active_creature_guids_for_player_update_like_cpp(
                map_id,
                instance_id,
                player_position,
                &player_phase_shift,
            )
    }
    pub(in crate::session) fn represented_can_see_or_detect_world_creature_like_cpp(
        &self,
        creature: &crate::map_manager::WorldCreature,
    ) -> bool {
        let expected = wow_map::MapKey::new(
            u32::from(self.player_map_id_like_cpp()),
            creature.instance_id(),
        );
        if self.current_canonical_player_map_key_like_cpp() != Some(expected) {
            return false;
        }
        self.with_owned_player_like_cpp(|player| {
            player.unit().can_see_or_detect_unit_like_cpp(
                creature.creature.unit(),
                false,
                true,
                false,
            )
        })
        .unwrap_or(false)
    }
    pub(in crate::session) fn represented_can_receive_creature_message_to_set_like_cpp(
        &self,
        guid: ObjectGuid,
        creature: &crate::map_manager::WorldCreature,
        required_3d: bool,
    ) -> bool {
        if !self.client_visible_guids_like_cpp.contains(&guid) {
            return false;
        }

        let Some(player_position) = self.player_position_like_cpp() else {
            return false;
        };
        let player_map_id = u32::from(self.player_map_id_like_cpp());
        let player_instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        let Some(player_phase_shift) = self.represented_player_phase_shift_like_cpp() else {
            return false;
        };

        crate::session_rules::creature_message_to_set_target_allows_like_cpp(
            creature,
            // The HaveAtClient membership was proven above.
            true,
            player_map_id,
            player_instance_id,
            &player_position,
            &player_phase_shift,
            required_3d,
        )
    }
    pub(crate) fn represented_can_receive_creature_message_to_set_by_guid_like_cpp(
        &self,
        guid: ObjectGuid,
        map_id: u16,
        instance_id: u32,
        required_3d: bool,
    ) -> Option<bool> {
        self.represented_can_receive_creature_message_to_set_by_guid_with_legacy_fallback_like_cpp(
            guid,
            map_id,
            instance_id,
            required_3d,
            false,
        )
    }
    pub(crate) fn represented_can_receive_creature_message_to_set_by_guid_with_legacy_fallback_like_cpp(
        &self,
        guid: ObjectGuid,
        map_id: u16,
        instance_id: u32,
        required_3d: bool,
        allow_legacy_fallback: bool,
    ) -> Option<bool> {
        if let Some(manager) = &self.canonical_map_manager {
            let creature = {
                let manager = manager.lock().ok()?;
                manager
                    .find_map(u32::from(map_id), instance_id)
                    .and_then(|map| {
                        map.map().with_creature_like_cpp(guid, |creature| {
                            let create_data =
                            crate::map_manager::WorldCreature::create_data_from_canonical_like_cpp(
                                creature,
                            );
                            crate::map_manager::WorldCreature::from_canonical(
                                creature.clone(),
                                create_data,
                            )
                        })
                    })
            };
            if let Some(creature) = creature {
                if creature.map_id() != u32::from(map_id) || creature.instance_id() != instance_id {
                    return Some(false);
                }
                return Some(
                    self.represented_can_receive_creature_message_to_set_like_cpp(
                        guid,
                        &creature,
                        required_3d,
                    ),
                );
            }
            if !allow_legacy_fallback {
                return None;
            }
        }

        // The two map managers coexist during the incremental runtime
        // migration. A canonical manager being installed does not prove that
        // it owns a source already validated from the legacy map. Only the
        // provenance-marked command path may cross this fallback boundary.
        let creature = {
            let manager = self.map_manager.as_ref()?;
            manager
                .read()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .find_creature(map_id, instance_id, guid)
                .cloned()
        }?;
        Some(
            self.represented_can_receive_creature_message_to_set_like_cpp(
                guid,
                &creature,
                required_3d,
            ),
        )
    }
    #[cfg(test)]
    pub fn set_creature_health_rates_like_cpp(
        &mut self,
        rates: CreatureClassificationHealthRatesLikeCpp,
    ) {
        self.creature_health_rates_like_cpp = rates;
    }
    pub(crate) fn creature_create_model_scalars_like_cpp(
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
    pub(crate) fn choose_creature_display_like_cpp(
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
    #[cfg(test)]
    pub(crate) fn creature_create_stats_like_cpp(
        &self,
        entry: u32,
        level: u8,
        unit_class: u8,
        classification: u32,
        regen_health: bool,
        db_cur_health: u32,
        db_cur_mana: u32,
    ) -> CreatureCreateStatsLikeCpp {
        let catalogs = self.creature_spawn_catalogs_for_test_like_cpp();
        self.creature_create_stats_with_catalogs_like_cpp(
            &catalogs,
            entry,
            level,
            unit_class,
            classification,
            regen_health,
            db_cur_health,
            db_cur_mana,
        )
    }
    pub(crate) fn creature_display_power_for_class_like_cpp(&self, unit_class: u8) -> u8 {
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
    pub(crate) async fn talked_to_creature_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        creature_entry: u32,
        creature_guid: wow_core::ObjectGuid,
    ) {
        // C++ QUEST_OBJECTIVE_TALKTO.
        self.update_represented_storing_value_quest_objective_progress_like_cpp(
            item_guid_generator,
            3,
            creature_entry as i32,
            1,
            creature_guid,
        )
        .await;
    }
    #[cfg(test)]
    pub(crate) async fn talked_to_creature_like_cpp(
        &mut self,
        creature_entry: u32,
        creature_guid: wow_core::ObjectGuid,
    ) {
        let Some(generator) = self.item_guid_generator_like_cpp_for_bridge() else {
            return;
        };
        self.talked_to_creature_with_generator_like_cpp(
            generator.as_ref(),
            creature_entry,
            creature_guid,
        )
        .await;
    }
    /// Present a health transition already committed by the canonical map
    /// owner without replaying the delayed value into canonical state.
    ///
    /// The command revision is a presentation high-water mark. The health and
    /// death tuple itself is always reread from the current canonical Player,
    /// so a heal, later hit, death, or resurrection that won after the queued
    /// swing cannot be rolled back by session delivery.
    pub(crate) fn present_committed_creature_melee_health_like_cpp(
        &mut self,
        committed_revision: u64,
    ) -> Option<u64> {
        if committed_revision == 0
            || committed_revision
                <= self.last_presented_creature_melee_health_state_revision_like_cpp
        {
            return None;
        }

        let canonical = self.with_owned_player_like_cpp(|player| {
            (
                player.unit().health_state_revision_like_cpp(),
                player.unit().data().health,
                player.unit().data().max_health,
                player.unit().is_alive(),
            )
        });
        #[cfg(test)]
        let canonical = canonical.or_else(|| {
            if self.player_handle_like_cpp.is_some() {
                return None;
            }
            self.mutate_canonical_player_like_cpp(|player| {
                (
                    player.unit().health_state_revision_like_cpp(),
                    player.unit().data().health,
                    player.unit().data().max_health,
                    player.unit().is_alive(),
                )
            })
        });
        let (canonical_revision, canonical_health, _canonical_max_health, _canonical_alive) =
            canonical?;
        if canonical_revision < committed_revision {
            return None;
        }

        #[cfg(test)]
        {
            self.player_max_health_like_cpp =
                _canonical_max_health.clamp(1, u64::from(u32::MAX)) as u32;
            self.player_health_like_cpp =
                canonical_health.min(u64::from(self.player_max_health_like_cpp)) as u32;
            self.player_alive_like_cpp = _canonical_alive && self.player_health_like_cpp > 0;
        }
        self.last_presented_creature_melee_health_state_revision_like_cpp = committed_revision;
        self.sync_player_registry_state_like_cpp();
        Some(canonical_health)
    }
}
