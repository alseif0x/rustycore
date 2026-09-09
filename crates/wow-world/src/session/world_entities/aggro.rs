//! Represented aggro, threat and engagement for observed creatures.
//!
//! Moved out of the Session root under #599. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(crate) fn creature_aggro_radius_for_faction_template_like_cpp(
        &self,
        faction_template_id: u32,
        default_radius: f32,
    ) -> f32 {
        if self.creature_faction_template_is_neutral_to_all_like_cpp(faction_template_id) {
            0.0
        } else {
            default_radius
        }
    }
    pub(in crate::session) fn canonical_creature_threat_value_like_cpp(
        &self,
        creature_guid: ObjectGuid,
        attacker_guid: ObjectGuid,
    ) -> Option<f32> {
        let map_key = self.current_canonical_player_map_key_like_cpp()?;
        let manager = self.canonical_map_manager.as_ref()?.clone();
        let manager = manager.lock().ok()?;
        let managed = manager.find_map(map_key.map_id, map_key.instance_id)?;
        creature_threat_value_on_map_like_cpp(managed.map(), creature_guid, attacker_guid)
    }
    pub(in crate::session) fn mirror_canonical_creature_threat_from_attacker_like_cpp(
        &mut self,
        creature_guid: ObjectGuid,
        attacker_guid: ObjectGuid,
        threat_value: f32,
    ) -> bool {
        let Some(map_key) = self.current_canonical_player_map_key_like_cpp() else {
            return false;
        };
        let Some(manager) = self.canonical_map_manager.as_ref().cloned() else {
            return false;
        };
        let Ok(mut manager) = manager.lock() else {
            return false;
        };
        let Some(managed) = manager.find_map_mut(map_key.map_id, map_key.instance_id) else {
            return false;
        };
        mirror_creature_threat_from_attacker_on_map_like_cpp(
            managed.map_mut(),
            creature_guid,
            attacker_guid,
            threat_value,
        )
    }
    pub fn set_legacy_creature_aggro_config_like_cpp(
        &mut self,
        config: LegacyCreatureAggroConfigLikeCpp,
    ) {
        self.legacy_creature_aggro_config_like_cpp = config;
    }
    pub(crate) fn canonical_player_combat_reach_snapshot_like_cpp(&self) -> f32 {
        self.canonical_player_snapshot_like_cpp(|player| player.unit().data().combat_reach)
            .unwrap_or(0.0)
    }
    pub(in crate::session) fn player_interaction_combat_reach_like_cpp(&self) -> f32 {
        let canonical_reach = self.canonical_player_combat_reach_snapshot_like_cpp();
        if canonical_reach > 0.0 {
            canonical_reach
        } else {
            DEFAULT_PLAYER_COMBAT_REACH_LIKE_CPP
        }
    }
    pub fn set_spell_threat_store(&mut self, store: Arc<SpellThreatStoreLikeCpp>) {
        self.spell_catalogs.spell_threat_store = Some(store);
    }
    pub(crate) fn spell_threat_entry_like_cpp(
        &self,
        spell_id: u32,
    ) -> Option<&SpellThreatEntryLikeCpp> {
        let store = self.spell_catalogs.spell_threat_store.as_ref()?;
        store.get_spell_threat_entry_like_cpp(spell_id, |lookup_spell_id| {
            self.spell_catalogs
                .spell_chain_store
                .as_ref()
                .map(|spell_chains| spell_chains.first_spell_in_chain_like_cpp(lookup_spell_id))
                .unwrap_or(lookup_spell_id)
        })
    }
    /// C++ `Spell::HandleThreatSpells` additive threat before target-count
    /// distribution. Explicit `spell_threat` rows replace the SpellLevel
    /// fallback. This returns the unmodified flat/AP amount: the positive
    /// `ForwardThreatForAssistingMe` path applies spell modifiers, while the
    /// harmful path calls `AddThreat(..., ignoreModifiers = true)`.
    fn spell_initial_threat_like_cpp(
        &self,
        spell_id: u32,
        threat_entry: Option<SpellThreatEntryLikeCpp>,
        caster_guid: ObjectGuid,
    ) -> Option<f32> {
        if let Some(entry) = threat_entry {
            let caster_attack_power = if self.player_guid() == Some(caster_guid) {
                self.resolved_item_bonus_state_like_cpp()?
                    .attack_power_total
                    .max(0)
            } else {
                0
            };
            return Some(entry.flat_mod as f32 + entry.ap_pct_mod * caster_attack_power as f32);
        }

        let difficulty = self.current_map_difficulty_id_like_cpp();
        if self.spell_custom_attributes_for_difficulty_like_cpp(spell_id, u32::from(difficulty))
            & wow_data::SPELL_ATTR0_CU_NO_INITIAL_THREAT_LIKE_CPP
            != 0
        {
            return Some(0.0);
        }

        Some(
            self.spell_catalogs
                .spell_levels_store
                .as_deref()
                .and_then(|store| {
                    let mut difficulty_id = difficulty;
                    let mut visited = HashSet::new();
                    loop {
                        if let Some(entry) =
                            store.entry_for_spell_difficulty_like_cpp(spell_id, difficulty_id)
                        {
                            break Some(entry);
                        }
                        if difficulty_id == 0 || !visited.insert(difficulty_id) {
                            break None;
                        }
                        difficulty_id = self
                            .difficulty_store()
                            .and_then(|difficulties| difficulties.get(u32::from(difficulty_id)))
                            .map_or(0, |difficulty| difficulty.fallback_difficulty_id);
                    }
                })
                .map_or(0.0, |entry| f32::from(entry.spell_level.max(0))),
        )
    }
    /// Represented single-target `Spell::HandleThreatSpells` pass. This runs
    /// once after all spell effects, unlike damage/heal threat which is
    /// generated by each individual effect.
    pub(in crate::session) fn apply_spell_initial_threat_like_cpp(
        &mut self,
        spell_id: i32,
        caster_guid: ObjectGuid,
        target_guid: ObjectGuid,
        is_positive: bool,
    ) {
        let Ok(spell_id_u32) = u32::try_from(spell_id) else {
            return;
        };
        let difficulty = self.current_map_difficulty_id_like_cpp();
        let difficulty_store = self.difficulty_store().cloned();
        // C++ `Spell::HandleThreatSpells` performs this unconditional
        // `!SpellInfo::HasInitialAggro()` return before calling
        // `ThreatManager::AddThreat`. The latter's engaged-owner exception
        // applies to per-effect damage threat, not this cast-level bonus.
        if self.spell_store().is_some_and(|store| {
            store.has_attribute_for_difficulty_like_cpp(
                spell_id,
                difficulty,
                difficulty_store.as_deref(),
                1,
                wow_data::spell::attributes::SPELL_ATTR1_NO_THREAT,
            ) || store.has_attribute_for_difficulty_like_cpp(
                spell_id,
                difficulty,
                difficulty_store.as_deref(),
                2,
                wow_data::spell::attributes::SPELL_ATTR2_NO_INITIAL_THREAT,
            )
        }) {
            return;
        }
        if !is_positive
            && self.spell_store().is_some_and(|store| {
                store.has_attribute_for_difficulty_like_cpp(
                    spell_id,
                    difficulty,
                    difficulty_store.as_deref(),
                    4,
                    wow_data::spell::attributes::SPELL_ATTR4_NO_HARMFUL_THREAT,
                )
            })
        {
            return;
        }

        let threat_entry = self.spell_threat_entry_like_cpp(spell_id_u32).copied();
        let Some(base_amount) =
            self.spell_initial_threat_like_cpp(spell_id_u32, threat_entry, caster_guid)
        else {
            return;
        };
        if base_amount == 0.0 {
            return;
        }
        if !is_positive {
            let threat_outcome = self
                .mutate_world_creature(target_guid, |creature| {
                    if !creature.is_alive() {
                        return None;
                    }
                    if !creature
                        .creature
                        .unit()
                        .subsystems()
                        .combat
                        .owner_can_have_threat_list
                    {
                        return None;
                    }
                    let newly_engaged = !creature.creature.is_in_combat();
                    if newly_engaged {
                        creature.enter_combat(caster_guid);
                    }
                    let combat = &mut creature.creature.unit_mut().subsystems_mut().combat;
                    combat.add_threat(caster_guid, base_amount);
                    combat
                        .threat_value(caster_guid)
                        .map(|threat_value| (threat_value, newly_engaged))
                })
                .flatten();
            if let Some((threat_value, newly_engaged)) = threat_outcome {
                self.sync_represented_creature_threat_to_canonical_like_cpp(
                    target_guid,
                    caster_guid,
                    threat_value,
                );
                if newly_engaged {
                    self.publish_spell_pull_attack_start_like_cpp(target_guid, caster_guid);
                }
            }
            return;
        }

        // C++ positive-spell path:
        // `target->GetThreatManager().ForwardThreatForAssistingMe`.
        if self.spell_store().is_some_and(|store| {
            store.has_attribute_for_difficulty_like_cpp(
                spell_id,
                difficulty,
                difficulty_store.as_deref(),
                4,
                wow_data::spell::attributes::SPELL_ATTR4_NO_HELPFUL_THREAT,
            )
        }) {
            return;
        }
        let spell_school_mask = self.spell_school_mask_for_difficulty_like_cpp(
            spell_id_u32,
            self.current_map_difficulty_id_like_cpp(),
        );
        let caster_school_threat_mod = if self.player_guid() == Some(caster_guid) {
            self.hydrate_canonical_threat_relevant_auras_like_cpp();
            self.mutate_canonical_player_like_cpp(|player| {
                player
                    .unit()
                    .subsystems()
                    .auras
                    .total_aura_multiplier_by_misc_mask_like_cpp(
                        wow_data::spell::aura_types::SPELL_AURA_MOD_THREAT,
                        spell_school_mask,
                    )
            })
            .unwrap_or(1.0)
        } else {
            1.0
        };
        let amount = base_amount
            * threat_entry.map_or(1.0, |entry| entry.pct_mod)
            * caster_school_threat_mod;
        if amount == 0.0 {
            return;
        }
        let owner_guids = self.canonical_threatened_by_me_owner_guids_like_cpp(target_guid);
        let controlled_mask = UnitState::CONTROLLED.bits();
        let eligible_count = owner_guids
            .iter()
            .filter(|owner_guid| {
                self.mutate_world_creature(**owner_guid, |creature| {
                    creature.is_alive() && !creature.creature.unit().has_unit_state(controlled_mask)
                })
                .unwrap_or(false)
            })
            .count();
        let per_owner = (eligible_count != 0).then(|| amount / eligible_count as f32);
        for owner_guid in owner_guids {
            let threat_value = self
                .mutate_world_creature(owner_guid, |creature| {
                    if !creature.is_alive() {
                        return None;
                    }
                    let controlled = creature.creature.unit().has_unit_state(controlled_mask);
                    let combat = &mut creature.creature.unit_mut().subsystems_mut().combat;
                    combat.add_threat(
                        caster_guid,
                        if controlled {
                            0.0
                        } else {
                            per_owner.unwrap_or(0.0)
                        },
                    );
                    combat.threat_value(caster_guid)
                })
                .flatten();
            if let Some(threat_value) = threat_value {
                self.sync_represented_creature_threat_to_canonical_like_cpp(
                    owner_guid,
                    caster_guid,
                    threat_value,
                );
            }
        }
    }
    pub(in crate::session) fn canonical_threat_aura_snapshot_for_difficulty_like_cpp(
        &self,
        spell_id: i32,
        difficulty: u8,
        effect_mask: u32,
        represented_effect_amounts: &[RepresentedAuraEffectAmountLikeCpp],
    ) -> CanonicalThreatAuraSnapshotLikeCpp {
        let interrupt_flags = self
            .spell_store()
            .and_then(|store| {
                store.aura_interrupt_flags_for_difficulty_like_cpp(
                    spell_id,
                    difficulty,
                    self.difficulty_store().map(AsRef::as_ref),
                )
            })
            .unwrap_or([0; 2]);
        let effects = self
            .spell_store()
            .and_then(|store| {
                store.effects_for_difficulty_like_cpp(
                    spell_id,
                    difficulty,
                    self.difficulty_store().map(AsRef::as_ref),
                )
            })
            .map(|effects| {
                effects
                    .iter()
                    .filter_map(|effect| {
                        let bit = 1u32.checked_shl(effect.effect_index)?;
                        let aura_type = effect.effect_aura;
                        (effect_mask & bit != 0
                            && matches!(
                                aura_type,
                                wow_data::spell::aura_types::SPELL_AURA_MOD_THREAT
                                    | wow_data::spell::aura_types::SPELL_AURA_SCHOOL_IMMUNITY
                                    | wow_data::spell::aura_types::SPELL_AURA_DAMAGE_IMMUNITY
                                    | wow_data::spell::aura_types::SPELL_AURA_MOD_CONFUSE
                                    | wow_data::spell::aura_types::SPELL_AURA_MOD_STUN
                            ))
                        .then(|| {
                            let amount = represented_effect_amounts
                                .iter()
                                .find(|represented| {
                                    represented.effect_index == effect.effect_index as u8
                                })
                                .map_or_else(
                                    || effect.calc_value_no_caster_like_cpp(),
                                    |represented| represented.amount,
                                );
                            (bit, aura_type, amount, effect.effect_misc_value_1)
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        CanonicalThreatAuraSnapshotLikeCpp::new(interrupt_flags, effects)
    }
    pub(in crate::session) fn sync_canonical_threat_relevant_aura_like_cpp(
        &mut self,
        spell_id: i32,
        caster_guid: ObjectGuid,
        slot: u8,
        effect_mask: u32,
        represented_effect_amounts: &[RepresentedAuraEffectAmountLikeCpp],
        apply: bool,
    ) {
        if !apply {
            let Ok(spell_id) = u32::try_from(spell_id) else {
                return;
            };
            let _ = self.mutate_player_aura_subsystem_like_cpp(|auras| {
                auras.remove_threat_snapshot_like_cpp(slot);
                for effect_index in 0..u32::BITS {
                    let effect_bit = 1u32 << effect_index;
                    if effect_mask & effect_bit != 0 {
                        auras.remove_applied(wow_entities::AppliedAuraRef::new(
                            spell_id,
                            caster_guid,
                            slot,
                            effect_bit,
                        ));
                    }
                }
            });
            return;
        }
        let snapshot = self
            .player_aura_subsystem_snapshot_like_cpp()
            .and_then(|auras| auras.threat_snapshot_like_cpp(slot).cloned())
            .unwrap_or_else(|| {
                let difficulty = self.current_map_difficulty_id_like_cpp();
                self.canonical_threat_aura_snapshot_for_difficulty_like_cpp(
                    spell_id,
                    difficulty,
                    effect_mask,
                    represented_effect_amounts,
                )
            });
        let Ok(spell_id) = u32::try_from(spell_id) else {
            return;
        };
        let _ = self.mutate_player_aura_subsystem_like_cpp(|auras| {
            auras.insert_threat_snapshot_like_cpp(slot, snapshot.clone());
            let interrupt_flags = snapshot.interrupt_flags();
            for &(effect_bit, aura_type, amount, misc_value) in snapshot.effects() {
                let aura =
                    wow_entities::AppliedAuraRef::new(spell_id, caster_guid, slot, effect_bit);
                auras.register_applied_aura(aura, None, interrupt_flags[0], interrupt_flags[1]);
                auras.register_applied_aura_effect_like_cpp(aura, aura_type, amount, misc_value);
            }
        });
    }
    pub(in crate::session) fn hydrate_canonical_threat_relevant_auras_like_cpp(&mut self) {
        let Some(auras) = self
            .resolved_player_visible_auras_like_cpp()
            .map(|auras| auras.into_values().collect::<Vec<_>>())
        else {
            return;
        };
        for aura in auras {
            self.sync_canonical_threat_relevant_aura_like_cpp(
                aura.spell_id,
                aura.caster_guid,
                aura.slot,
                aura.effect_mask,
                &aura.represented_effect_amounts,
                true,
            );
        }
    }
    pub(in crate::session) fn represented_visibility_source_combat_reach_like_cpp(&self) -> f32 {
        let Some(player_guid) = self.player_guid() else {
            return 0.0;
        };
        let seer_guid = self
            .represented_seer_guid_like_cpp
            .filter(|guid| !guid.is_empty())
            .unwrap_or(player_guid);

        if let (Some(key), Some(manager)) = (
            self.current_canonical_player_map_key_like_cpp(),
            self.canonical_map_manager.as_ref(),
        ) && let Ok(manager) = manager.lock()
            && let Some(reach) = manager
                .find_map(key.map_id, key.instance_id)
                .and_then(|managed| {
                    managed.map().with_world_object_by_kinds_like_cpp(
                        seer_guid,
                        Self::represented_seer_kinds_like_cpp(),
                        |object| object.combat_reach(),
                    )
                })
        {
            return reach.max(0.0);
        }

        self.canonical_player_combat_reach_snapshot_like_cpp()
            .max(0.0)
    }
}
