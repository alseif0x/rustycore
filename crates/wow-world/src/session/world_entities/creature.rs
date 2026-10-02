//! Represented creature state owned by the Session boundary.
//!
//! Moved out of the Session root under #599. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

pub(crate) fn creature_message_to_set_target_allows_like_cpp(
    creature: &crate::map_manager::WorldCreature,
    source_is_visible_like_cpp: bool,
    player_map_id: u32,
    player_instance_id: u32,
    player_position: &wow_core::Position,
    player_phase_shift: &wow_entities::PhaseShift,
    required_3d: bool,
) -> bool {
    if !source_is_visible_like_cpp {
        return false;
    }
    if creature.map_id() != player_map_id || creature.instance_id() != player_instance_id {
        return false;
    }
    if !player_phase_shift.can_see(creature.phase_shift()) {
        return false;
    }

    let range = creature.visibility_range_like_cpp();
    if required_3d {
        wow_core::position_is_in_dist_strict_3d_like_cpp(
            &creature.position(),
            player_position,
            range,
        )
    } else {
        wow_core::position_is_in_dist_strict_2d_like_cpp(
            &creature.position(),
            player_position,
            range,
        )
    }
}

impl WorldSession {
    pub fn set_canonical_creature_private_object_owner_like_cpp(
        &mut self,
        guid: ObjectGuid,
        owner: ObjectGuid,
    ) -> bool {
        let mut updated = self
            .core
            .mutate_canonical_creature_by_guid_like_cpp(guid, |creature| {
                creature.unit_mut().set_private_object_owner_like_cpp(owner);
            })
            .is_some();

        if let Some(manager) = self.core.map_manager.as_ref() {
            let mut manager = manager
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if let Some(creature) =
                manager.find_creature_mut(self.core.player_map_id_like_cpp(), 0, guid)
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
    pub(crate) fn active_world_creature_guids_for_update_like_cpp(&self) -> Vec<ObjectGuid> {
        let Some(player_position) = crate::session::hub_ref(self).player_position_like_cpp() else {
            return Vec::new();
        };
        let Some(manager) = &self.core.map_manager else {
            return Vec::new();
        };
        let Some(player_phase_shift) = self.represented_player_phase_shift_like_cpp() else {
            return Vec::new();
        };
        let (map_id, instance_id) = self.core.current_legacy_runtime_map_key_like_cpp();
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
    pub(in crate::session) fn represented_can_receive_creature_message_to_set_like_cpp(
        &self,
        guid: ObjectGuid,
        creature: &crate::map_manager::WorldCreature,
        required_3d: bool,
    ) -> bool {
        if !self.core.client_visible_guids_like_cpp.contains(&guid) {
            return false;
        }

        let Some(player_position) = crate::session::hub_ref(self).player_position_like_cpp() else {
            return false;
        };
        let player_map_id = u32::from(self.core.player_map_id_like_cpp());
        let player_instance_id = self
            .core
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        let Some(player_phase_shift) = self.represented_player_phase_shift_like_cpp() else {
            return false;
        };

        crate::session::creature_message_to_set_target_allows_like_cpp(
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
        if let Some(manager) = &self.core.canonical_map_manager {
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
            let manager = self.core.map_manager.as_ref()?;
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
        self.config.creature_health_rates_like_cpp = rates;
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
                <= self
                    .view
                    .last_presented_creature_melee_health_state_revision_like_cpp
        {
            return None;
        }

        let canonical = self.core.with_owned_player_like_cpp(|player| {
            (
                player.unit().health_state_revision_like_cpp(),
                player.unit().data().health,
                player.unit().data().max_health,
                player.unit().is_alive(),
            )
        });
        #[cfg(test)]
        let canonical = canonical.or_else(|| {
            if self.core.player_handle_like_cpp.is_some() {
                return None;
            }
            self.core.mutate_canonical_player_like_cpp(|player| {
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
            self.fixtures.combat.player_max_health_like_cpp =
                _canonical_max_health.clamp(1, u64::from(u32::MAX)) as u32;
            self.fixtures.combat.player_health_like_cpp = canonical_health
                .min(u64::from(self.fixtures.combat.player_max_health_like_cpp))
                as u32;
            self.fixtures.combat.player_alive_like_cpp =
                _canonical_alive && self.fixtures.combat.player_health_like_cpp > 0;
        }
        self.view
            .last_presented_creature_melee_health_state_revision_like_cpp = committed_revision;
        self.sync_player_registry_state_like_cpp();
        Some(canonical_health)
    }
}

impl crate::session::state::WorldEntitiesState {
    pub(in crate::session) fn represented_can_see_or_detect_world_creature_like_cpp(
        &self,
        hub: crate::session::HubRef<'_>,
        creature: &crate::map_manager::WorldCreature,
    ) -> bool {
        let expected = wow_map::MapKey::new(
            u32::from(hub.core.player_map_id_like_cpp()),
            creature.instance_id(),
        );
        if hub.core.current_canonical_player_map_key_like_cpp() != Some(expected) {
            return false;
        }
        hub.core
            .with_owned_player_like_cpp(|player| {
                player.unit().can_see_or_detect_unit_like_cpp(
                    creature.creature.unit(),
                    false,
                    true,
                    false,
                )
            })
            .unwrap_or(false)
    }
}

/// One creature aura this session applied, with the wall-clock deadline its
/// represented duration expires at.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::session) struct RepresentedCreatureAuraLikeCpp {
    pub target_guid: ObjectGuid,
    pub spell_id: i32,
    pub caster_guid: ObjectGuid,
    pub slot: u8,
    pub effect_mask: u32,
    pub applied_at: Instant,
    pub duration_ms: u32,
}

impl WorldSession {
    /// C++ `Spell::EffectApplyAura` for a creature target: create the canonical
    /// `AuraApplication` with its represented effect data, misc values and
    /// duration, and publish `SMSG_AURA_UPDATE` to the caster and the
    /// creature's observers.
    ///
    /// Boundaries of this slice: the represented model has no stack/refresh
    /// rule (a second application of the same spell by the same caster is
    /// refused, like the spawn-addon path), the duration comes from the
    /// caller's represented value rather than `SpellDuration.db2`, and the
    /// periodic effect amount is registered but not yet ticked.
    pub(crate) fn apply_creature_aura_like_cpp(
        &mut self,
        spell_id: i32,
        caster_guid: ObjectGuid,
        target_guid: ObjectGuid,
        effect_mask: u32,
        duration_ms: u32,
    ) -> Result<(), &'static str> {
        self.apply_creature_aura_with_provenance_like_cpp(
            spell_id,
            caster_guid,
            target_guid,
            effect_mask,
            duration_ms,
            wow_entities::AuraCastProvenanceLikeCpp::default(),
        )
    }
    pub(crate) fn apply_creature_aura_with_provenance_like_cpp(
        &mut self,
        spell_id: i32,
        caster_guid: ObjectGuid,
        target_guid: ObjectGuid,
        effect_mask: u32,
        duration_ms: u32,
        provenance: wow_entities::AuraCastProvenanceLikeCpp,
    ) -> Result<(), &'static str> {
        let Ok(spell_key) = u32::try_from(spell_id) else {
            return Err("invalid spell id");
        };
        let spell_store = self
            .spell_store()
            .cloned()
            .ok_or("Spell store not loaded")?;
        let Some(spell) = spell_store.get(spell_id) else {
            return Err("Spell not found");
        };
        let effects: Vec<(i32, i32, i32, u8)> = spell
            .effects()
            .iter()
            .filter(|effect| {
                1u32.checked_shl(effect.effect_index)
                    .is_some_and(|bit| effect_mask & bit != 0)
            })
            .map(|effect| {
                (
                    effect.effect_aura,
                    effect.calc_value_no_caster_like_cpp(),
                    effect.effect_misc_value_1,
                    u8::try_from(effect.effect_index).unwrap_or(0),
                )
            })
            .collect();
        if effects.is_empty() {
            return Err("no represented effects for this aura");
        }
        let applied = self
            .core
            .mutate_canonical_creature_by_guid_like_cpp(target_guid, |creature| {
                let auras = &mut creature.unit_mut().subsystems_mut().auras;
                if auras
                    .applied_auras
                    .iter()
                    .any(|aura| aura.spell_id == spell_key && aura.caster_guid == caster_guid)
                {
                    return None;
                }
                let slot = (0..u8::MAX).find(|slot| !auras.visible_auras.contains_key(slot))?;
                auras.add_owned(wow_entities::OwnedAuraRef::new(
                    spell_key,
                    caster_guid,
                    None,
                ));
                // C++ `Aura::Create` builds one `AuraEffect` per applied slot
                // and `GetAuraEffectsByType` reads those effects individually.
                // The per-slot `AppliedAuraRef` keeps each slot's own amount
                // and misc value, the convention the pet-load and
                // threat-snapshot paths already use.
                for (aura_type, amount, misc_value, effect_index) in &effects {
                    let effect_ref = wow_entities::AppliedAuraRef::new(
                        spell_key,
                        caster_guid,
                        slot,
                        1_u32 << u32::from(*effect_index),
                    );
                    auras.register_applied_aura_effect_like_cpp(
                        effect_ref,
                        *aura_type,
                        *amount,
                        *misc_value,
                    );
                }
                auras.set_loaded_aura_state_like_cpp(
                    wow_entities::AuraRef::new(spell_key, caster_guid),
                    wow_entities::LoadedAuraStateLikeCpp::new(
                        i32::try_from(duration_ms).unwrap_or(i32::MAX),
                        i32::try_from(duration_ms).unwrap_or(i32::MAX),
                        0,
                        1,
                        0,
                    ),
                );
                auras.set_visible_with_application_like_cpp(
                    slot,
                    wow_entities::AuraRef::new(spell_key, caster_guid),
                    wow_entities::VisibleAuraApplicationLikeCpp::new(
                        0,
                        effects
                            .iter()
                            .map(|(_, amount, _, effect_index)| {
                                wow_entities::VisibleAuraEffectAmountLikeCpp {
                                    effect_index: *effect_index,
                                    amount: *amount,
                                }
                            })
                            .collect(),
                    ),
                );
                auras.set_aura_cast_provenance_like_cpp(slot, provenance);
                Some(())
            })
            .flatten()
            .ok_or("creature target unavailable or aura already applied")?;
        let _ = applied;
        let slot = self
            .canonical_creature_aura_slot_like_cpp(target_guid, spell_key, caster_guid)
            .ok_or("creature aura slot missing after application")?;
        self.world_entities
            .represented_creature_auras_like_cpp
            .push(RepresentedCreatureAuraLikeCpp {
                target_guid,
                spell_id,
                caster_guid,
                slot,
                effect_mask,
                applied_at: Instant::now(),
                duration_ms,
            });
        self.publish_creature_aura_slot_update_like_cpp(target_guid, slot, true);
        Ok(())
    }

    fn canonical_creature_aura_slot_like_cpp(
        &mut self,
        target_guid: ObjectGuid,
        spell_id: u32,
        caster_guid: ObjectGuid,
    ) -> Option<u8> {
        let (state, mut hub) = crate::session::split_world_entities_mut(self);
        state.canonical_creature_aura_slot_like_cpp(&mut hub, target_guid, spell_id, caster_guid)
    }

    /// C++ `AuraApplication` publication: the slot's current state when
    /// `applied`, or a removal entry (`AuraInfoLikeCpp { aura_data: None }`)
    /// when not. The caster's session receives it directly and the creature's
    /// nearby observers through the existing creature visibility rail.
    pub(in crate::session) fn publish_creature_aura_slot_update_like_cpp(
        &self,
        target_guid: ObjectGuid,
        slot: u8,
        applied: bool,
    ) {
        use wow_packet::ServerPacket;
        let Some(map_key) = self
            .core
            .canonical_object_lookup_map_key_like_cpp(u32::from(
                self.core.player_map_id_like_cpp(),
            ))
        else {
            return;
        };
        let Some(manager) = self.core.canonical_map_manager.as_ref().cloned() else {
            return;
        };
        let Some(aura_info) = ({
            let mut manager = match manager.lock() {
                Ok(manager) => manager,
                Err(_) => return,
            };
            let Some(managed) = manager.find_map_mut(map_key.map_id, map_key.instance_id) else {
                return;
            };
            managed
                .map()
                .with_creature_like_cpp(target_guid, |creature| {
                    if !applied {
                        return wow_packet::packets::misc::AuraInfoLikeCpp {
                            slot,
                            aura_data: None,
                        };
                    }
                    let level = creature.unit().data().level.clamp(0, i32::from(u8::MAX)) as u8;
                    super::creature_publication::represented_creature_aura_info_like_cpp(
                        &creature.unit().subsystems().auras,
                        slot,
                        level,
                    )
                })
        }) else {
            return;
        };
        let packet = wow_packet::packets::misc::AuraUpdate {
            unit_guid: target_guid,
            update_all: false,
            auras: vec![aura_info],
        };
        self.send_packet(&packet);
        self.broadcast_creature_packet_to_visible_set_like_cpp(target_guid, packet.to_bytes());
    }

    /// C++ `Creature::Update`'s aura lifetime for the auras this session
    /// applied: a duration that elapsed removes the canonical application and
    /// publishes the removal.
    pub(in crate::session) fn tick_represented_creature_auras_like_cpp(&mut self) {
        if self
            .world_entities
            .represented_creature_auras_like_cpp
            .is_empty()
        {
            return;
        }
        let expired: Vec<RepresentedCreatureAuraLikeCpp> = self
            .world_entities
            .represented_creature_auras_like_cpp
            .iter()
            .filter(|aura| {
                aura.duration_ms > 0
                    && aura.applied_at.elapsed().as_millis() as u32 >= aura.duration_ms
            })
            .copied()
            .collect();
        for aura in expired {
            let removed = self
                .core
                .mutate_canonical_creature_by_guid_like_cpp(aura.target_guid, |creature| {
                    let auras = &mut creature.unit_mut().subsystems_mut().auras;
                    let spell_id = u32::try_from(aura.spell_id).unwrap_or(0);
                    // One per-slot `AppliedAuraRef` per applied effect slot;
                    // removing every covered slot is the C++
                    // `AuraApplication` removal. `unapply_aura` also removes
                    // the loaded state and `clear_visible` the published slot.
                    let mut removed = false;
                    for effect_index in 0..u32::BITS {
                        let bit = 1_u32 << effect_index;
                        if aura.effect_mask & bit == 0 {
                            continue;
                        }
                        let applied = wow_entities::AppliedAuraRef::new(
                            spell_id,
                            aura.caster_guid,
                            aura.slot,
                            bit,
                        );
                        removed |= auras.unapply_aura(applied, 0);
                    }
                    let _ = auras.clear_visible(aura.slot);
                    removed
                })
                .unwrap_or(false);
            self.world_entities
                .represented_creature_auras_like_cpp
                .retain(|tracked| *tracked != aura);
            if removed {
                self.publish_creature_aura_slot_update_like_cpp(aura.target_guid, aura.slot, false);
            }
        }
    }
}

impl crate::session::state::WorldEntitiesState {
    /// The canonical creature aura slot of one `(spell, caster)` application.
    fn canonical_creature_aura_slot_like_cpp(
        &mut self,
        hub: &mut crate::session::HubMut<'_>,
        target_guid: ObjectGuid,
        spell_id: u32,
        caster_guid: ObjectGuid,
    ) -> Option<u8> {
        hub.core
            .mutate_canonical_creature_by_guid_like_cpp(target_guid, |creature| {
                creature
                    .unit()
                    .subsystems()
                    .auras
                    .applied_auras
                    .iter()
                    .find(|aura| aura.spell_id == spell_id && aura.caster_guid == caster_guid)
                    .map(|aura| aura.slot)
            })
            .flatten()
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/world_entities/creature/f3_shims.rs"]
mod f3_shims;
