//! Threat forwarding and synchronisation raised by represented effects.
//!
//! Moved out of the Session root under #621. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    /// C++ `ThreatManager::ForwardThreatForAssistingMe` for direct healing.
    ///
    /// `Spell::DoAllEffectOnTarget` forwards half of the effective heal, split
    /// evenly across non-controlled creatures that already threaten the healed
    /// unit. Controlled owners receive a zero-threat combat reference instead.
    pub(in crate::session) fn forward_heal_threat_like_cpp(
        &mut self,
        spell_id: Option<i32>,
        healer_guid: ObjectGuid,
        target_guid: ObjectGuid,
        effective_heal: u32,
    ) {
        if effective_heal == 0 {
            return;
        }
        let difficulty = self.current_map_difficulty_id_like_cpp();
        let difficulty_store = self.difficulty_store().cloned();
        if spell_id.is_some_and(|spell_id| {
            self.spell_store().is_some_and(|store| {
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
                    4,
                    wow_data::spell::attributes::SPELL_ATTR4_NO_HELPFUL_THREAT,
                )
            })
        }) {
            return;
        }
        let spell_threat_entry = spell_id
            .and_then(|spell_id| u32::try_from(spell_id).ok())
            .and_then(|spell_id| self.spell_threat_entry_like_cpp(spell_id))
            .copied();
        let spell_threat_pct_mod = spell_threat_entry.map_or(1.0, |entry| entry.pct_mod);
        let spell_school_mask = spell_id
            .and_then(|spell_id| u32::try_from(spell_id).ok())
            .map_or(1, |spell_id| {
                self.spell_school_mask_for_difficulty_like_cpp(
                    spell_id,
                    self.current_map_difficulty_id_like_cpp(),
                )
            });
        let caster_school_threat_mod = if self.player_guid() == Some(healer_guid) {
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
        let no_initial_threat = spell_id.is_some_and(|spell_id| {
            self.spell_store().is_some_and(|store| {
                store.has_attribute_for_difficulty_like_cpp(
                    spell_id,
                    difficulty,
                    difficulty_store.as_deref(),
                    2,
                    wow_data::spell::attributes::SPELL_ATTR2_NO_INITIAL_THREAT,
                )
            })
        });
        let owner_guids = self.canonical_threatened_by_me_owner_guids_like_cpp(target_guid);
        if owner_guids.is_empty() {
            return;
        }

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
        let per_owner = if eligible_count == 0 {
            0.0
        } else {
            effective_heal as f32 * 0.5 * spell_threat_pct_mod * caster_school_threat_mod
                / eligible_count as f32
        };

        for owner_guid in owner_guids {
            let threat_value = self
                .mutate_world_creature(owner_guid, |creature| {
                    if !creature.is_alive() {
                        return None;
                    }
                    let controlled = creature.creature.unit().has_unit_state(controlled_mask);
                    if no_initial_threat && !creature.creature.is_in_combat() {
                        return None;
                    }
                    if !controlled && creature.creature.ai_ownership().combat_target.is_none() {
                        creature.enter_combat(healer_guid);
                    }
                    let combat = &mut creature.creature.unit_mut().subsystems_mut().combat;
                    combat.add_threat(healer_guid, if controlled { 0.0 } else { per_owner });
                    combat.threat_value(healer_guid)
                })
                .flatten();
            if let Some(threat_value) = threat_value {
                self.sync_represented_creature_threat_to_canonical_like_cpp(
                    owner_guid,
                    healer_guid,
                    threat_value,
                );
            }
        }
    }
    pub(in crate::session) fn sync_represented_creature_threat_to_canonical_like_cpp(
        &mut self,
        creature_guid: ObjectGuid,
        attacker_guid: ObjectGuid,
        threat_value: f32,
    ) {
        // C++ `AddThreat(..., 0.0f)` still creates the reciprocal combat
        // reference (notably for controlled heal-threat owners).
        if attacker_guid.is_player() {
            let _ = self.begin_canonical_player_combat_ref_like_cpp(
                attacker_guid,
                creature_guid,
                false,
                false,
                false,
            );
        }

        let Some(map_key) = self.current_canonical_player_map_key_like_cpp() else {
            return;
        };
        let Some(manager) = self.canonical_map_manager.as_ref().cloned() else {
            return;
        };
        let Ok(mut manager) = manager.lock() else {
            return;
        };
        let Some(managed) = manager.find_map_mut(map_key.map_id, map_key.instance_id) else {
            return;
        };
        let map = managed.map_mut();

        if attacker_guid.is_creature() {
            let both_player_controlled = map
                .with_creature_like_cpp(attacker_guid, |creature| {
                    creature.is_charmed_owned_by_player_or_player_like_cpp()
                })
                .unwrap_or(false)
                && map
                    .with_creature_like_cpp(creature_guid, |creature| {
                        creature.is_charmed_owned_by_player_or_player_like_cpp()
                    })
                    .unwrap_or(false);
            if let Some(attacker) = map.get_typed_creature_mut(attacker_guid) {
                attacker
                    .unit_mut()
                    .subsystems_mut()
                    .combat
                    .set_in_combat_with(creature_guid, both_player_controlled, false);
            }
            if let Some(creature) = map.get_typed_creature_mut(creature_guid) {
                creature
                    .unit_mut()
                    .subsystems_mut()
                    .combat
                    .set_in_combat_with(attacker_guid, both_player_controlled, false);
            }
        }

        let threat_ref = {
            let Some(creature) = map.get_typed_creature_mut(creature_guid) else {
                return;
            };
            creature
                .unit_mut()
                .subsystems_mut()
                .combat
                .set_threat(attacker_guid, threat_value);
            creature
                .unit()
                .subsystems()
                .combat
                .threat_ref(attacker_guid)
                .copied()
        };

        if let Some(threat_ref) = threat_ref {
            if let Some(attacker) = map.get_typed_player_mut(attacker_guid) {
                attacker
                    .unit_mut()
                    .subsystems_mut()
                    .combat
                    .put_threatened_by_me_ref(creature_guid, threat_ref);
            } else if let Some(attacker) = map.get_typed_creature_mut(attacker_guid) {
                attacker
                    .unit_mut()
                    .subsystems_mut()
                    .combat
                    .put_threatened_by_me_ref(creature_guid, threat_ref);
            }
        }
    }
    /// C++ `Spell::EffectThreat`.
    pub(in crate::session) fn apply_threat_effect_like_cpp(
        &mut self,
        damage: i32,
        target_guid: ObjectGuid,
    ) -> Result<(), &'static str> {
        let player_guid = self.player_guid().ok_or("No player GUID")?;
        if self.resolved_player_is_alive_like_cpp() != Some(true) {
            return Ok(());
        }

        let Some(threat_value) = self
            .mutate_world_creature(target_guid, |creature| {
                creature
                    .creature
                    .unit_mut()
                    .subsystems_mut()
                    .combat
                    .add_threat(player_guid, damage as f32);
                creature
                    .creature
                    .unit()
                    .subsystems()
                    .combat
                    .threat_value(player_guid)
            })
            .flatten()
        else {
            return Ok(());
        };

        self.sync_represented_creature_threat_to_canonical_like_cpp(
            target_guid,
            player_guid,
            threat_value,
        );
        Ok(())
    }
    /// C++ `Spell::EffectModifyThreatPercent`.
    pub(in crate::session) fn apply_modify_threat_percent_effect_like_cpp(
        &mut self,
        damage: i32,
        target_guid: ObjectGuid,
    ) -> Result<(), &'static str> {
        let player_guid = self.player_guid().ok_or("No player GUID")?;
        let Some(threat_value) = self
            .mutate_world_creature(target_guid, |creature| {
                creature
                    .creature
                    .unit_mut()
                    .subsystems_mut()
                    .combat
                    .modify_threat_by_percent(player_guid, damage)
            })
            .flatten()
        else {
            return Ok(());
        };

        self.sync_represented_creature_threat_to_canonical_like_cpp(
            target_guid,
            player_guid,
            threat_value,
        );
        Ok(())
    }
    pub(in crate::session) fn canonical_threatened_by_me_owner_guids_like_cpp(
        &self,
        target_guid: ObjectGuid,
    ) -> Vec<ObjectGuid> {
        let Some(map_key) = self.current_canonical_player_map_key_like_cpp() else {
            return Vec::new();
        };
        let Some(manager) = self.canonical_map_manager.as_ref().cloned() else {
            return Vec::new();
        };
        let Ok(manager) = manager.lock() else {
            return Vec::new();
        };
        let Some(managed) = manager.find_map(map_key.map_id, map_key.instance_id) else {
            return Vec::new();
        };
        let map = managed.map();
        if let Some(player) = map.get_typed_player(target_guid) {
            return player
                .unit()
                .subsystems()
                .combat
                .threatened_by_me_owner_guids();
        }
        if let Some(owner_guids) = map.with_creature_like_cpp(target_guid, |creature| {
            creature
                .unit()
                .subsystems()
                .combat
                .threatened_by_me_owner_guids()
        }) {
            return owner_guids;
        }
        Vec::new()
    }
    pub(in crate::session) fn scale_canonical_owner_threat_to_target_zero_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
        target_guid: ObjectGuid,
    ) {
        let Some(map_key) = self.current_canonical_player_map_key_like_cpp() else {
            return;
        };
        let Some(manager) = self.canonical_map_manager.as_ref().cloned() else {
            return;
        };
        let Ok(mut manager) = manager.lock() else {
            return;
        };
        let Some(managed) = manager.find_map_mut(map_key.map_id, map_key.instance_id) else {
            return;
        };
        let map = managed.map_mut();

        let threat_ref = if let Some(owner) = map.get_typed_creature_mut(owner_guid) {
            owner
                .unit_mut()
                .subsystems_mut()
                .combat
                .scale_threat(target_guid, 0.0);
            owner
                .unit()
                .subsystems()
                .combat
                .threat_ref(target_guid)
                .copied()
        } else if let Some(owner) = map.get_typed_player_mut(owner_guid) {
            owner
                .unit_mut()
                .subsystems_mut()
                .combat
                .scale_threat(target_guid, 0.0);
            owner
                .unit()
                .subsystems()
                .combat
                .threat_ref(target_guid)
                .copied()
        } else {
            None
        };

        let Some(threat_ref) = threat_ref else {
            return;
        };
        if let Some(target) = map.get_typed_player_mut(target_guid) {
            target
                .unit_mut()
                .subsystems_mut()
                .combat
                .put_threatened_by_me_ref(owner_guid, threat_ref);
        } else if let Some(target) = map.get_typed_creature_mut(target_guid) {
            target
                .unit_mut()
                .subsystems_mut()
                .combat
                .put_threatened_by_me_ref(owner_guid, threat_ref);
        }
    }
}
