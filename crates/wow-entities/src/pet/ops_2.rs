//! Pet lifecycle, aura and persistence state operations, part 2 of 2.
//!
//! The inherent `Pet` impl is divided by responsibility under
//! #636; every method keeps its original body.

use super::*;

impl Pet {
    pub fn remove_spell_like_cpp(
        &mut self,
        spell_id: u32,
        mut learn_prev: bool,
        clear_action_bar: bool,
        action_bar: &mut [u32; MAX_UNIT_ACTION_BAR_INDEX],
        mut prev_spell_in_chain_like_cpp: impl FnMut(u32) -> u32,
        mut first_spell_in_chain_like_cpp: impl FnMut(u32) -> u32,
    ) -> PetRemoveSpellOutcomeLikeCpp {
        let Some(spell) = self.spells.get(&spell_id).copied() else {
            return PetRemoveSpellOutcomeLikeCpp::not_removed();
        };
        if spell.state == PetSpellState::Removed {
            return PetRemoveSpellOutcomeLikeCpp::not_removed();
        }

        let erased_new_spell = spell.state == PetSpellState::New;
        if erased_new_spell {
            self.spells.remove(&spell_id);
        } else if let Some(spell) = self.spells.get_mut(&spell_id) {
            spell.state = PetSpellState::Removed;
        }
        self.autospells.retain(|known| *known != spell_id);

        let learned_prev_spell_id = if learn_prev {
            let prev_id = prev_spell_in_chain_like_cpp(spell_id);
            if prev_id != 0 {
                self.add_spell(
                    prev_id,
                    ActiveState::Decide,
                    PetSpellState::New,
                    PetSpellType::Normal,
                );
                Some(prev_id)
            } else {
                learn_prev = false;
                None
            }
        } else {
            None
        };

        let cleared_action_bar_slot = if clear_action_bar && !learn_prev {
            self.remove_spell_from_action_bar_like_cpp(
                spell_id,
                action_bar,
                &mut first_spell_in_chain_like_cpp,
            )
        } else {
            None
        };

        PetRemoveSpellOutcomeLikeCpp {
            removed: true,
            erased_new_spell,
            marked_removed: !erased_new_spell,
            remove_auras_due_to_spell: true,
            learned_prev_spell_id,
            cleared_action_bar_slot,
            pet_spell_initialize: cleared_action_bar_slot.is_some() && !self.loading,
        }
    }
    pub fn toggle_autocast(&mut self, spell_id: u32, apply: bool) -> bool {
        self.toggle_autocast_like_cpp(spell_id, apply, true)
            .spell_found
    }
    pub fn toggle_autocast_like_cpp(
        &mut self,
        spell_id: u32,
        apply: bool,
        spell_is_autocastable_like_cpp: bool,
    ) -> PetToggleAutocastOutcomeLikeCpp {
        if !spell_is_autocastable_like_cpp {
            return PetToggleAutocastOutcomeLikeCpp {
                spell_found: false,
                autocastable: false,
                autospell_added: false,
                autospell_removed: false,
                active_changed: false,
                marked_changed: false,
            };
        }

        let Some(existing_spell) = self.spells.get(&spell_id).copied() else {
            return PetToggleAutocastOutcomeLikeCpp {
                spell_found: false,
                autocastable: true,
                autospell_added: false,
                autospell_removed: false,
                active_changed: false,
                marked_changed: false,
            };
        };

        let autospell_index = self
            .autospells
            .iter()
            .position(|known_spell_id| *known_spell_id == spell_id);
        let mut autospell_added = false;
        let mut autospell_removed = false;
        let mut active_changed = false;
        let mut marked_changed = false;

        if apply {
            if autospell_index.is_none() {
                self.autospells.push(spell_id);
                autospell_added = true;

                if existing_spell.active != ActiveState::Enabled {
                    active_changed = true;
                    marked_changed = existing_spell.state != PetSpellState::New;
                    if let Some(spell) = self.spells.get_mut(&spell_id) {
                        spell.active = ActiveState::Enabled;
                        if marked_changed {
                            spell.state = PetSpellState::Changed;
                        }
                    }
                }
            }
        } else if let Some(index) = autospell_index {
            self.autospells.remove(index);
            autospell_removed = true;

            if existing_spell.active != ActiveState::Disabled {
                active_changed = true;
                marked_changed = existing_spell.state != PetSpellState::New;
                if let Some(spell) = self.spells.get_mut(&spell_id) {
                    spell.active = ActiveState::Disabled;
                    if marked_changed {
                        spell.state = PetSpellState::Changed;
                    }
                }
            }
        }

        PetToggleAutocastOutcomeLikeCpp {
            spell_found: true,
            autocastable: true,
            autospell_added,
            autospell_removed,
            active_changed,
            marked_changed,
        }
    }
    pub fn save_spells_plan_like_cpp(
        &mut self,
        pet_number: u32,
    ) -> Vec<PetSpellSaveOperationLikeCpp> {
        let spell_ids = self.spells.keys().copied().collect::<Vec<_>>();
        let mut operations = Vec::new();

        for spell_id in spell_ids {
            let Some(spell) = self.spells.get(&spell_id).copied() else {
                continue;
            };

            if spell.spell_type == PetSpellType::Family {
                continue;
            }

            match spell.state {
                PetSpellState::Removed => {
                    operations.push(PetSpellSaveOperationLikeCpp::DeleteBySpell {
                        pet_number,
                        spell_id,
                    });
                    self.spells.remove(&spell_id);
                }
                PetSpellState::Changed => {
                    operations.push(PetSpellSaveOperationLikeCpp::DeleteBySpell {
                        pet_number,
                        spell_id,
                    });
                    operations.push(PetSpellSaveOperationLikeCpp::Insert {
                        pet_number,
                        spell_id,
                        active: spell.active,
                    });
                    if let Some(saved_spell) = self.spells.get_mut(&spell_id) {
                        saved_spell.state = PetSpellState::Unchanged;
                    }
                }
                PetSpellState::New => {
                    operations.push(PetSpellSaveOperationLikeCpp::Insert {
                        pet_number,
                        spell_id,
                        active: spell.active,
                    });
                    if let Some(saved_spell) = self.spells.get_mut(&spell_id) {
                        saved_spell.state = PetSpellState::Unchanged;
                    }
                }
                PetSpellState::Unchanged => {}
            }
        }

        operations
    }
    pub fn save_auras_plan_like_cpp(
        pet_number: u32,
        pet_guid: ObjectGuid,
        auras: &[PetAuraSaveRefLikeCpp],
    ) -> Vec<PetAuraSaveOperationLikeCpp> {
        let mut operations = vec![
            PetAuraSaveOperationLikeCpp::DeleteAuraEffects { pet_number },
            PetAuraSaveOperationLikeCpp::DeleteAuras { pet_number },
        ];

        for aura in auras {
            if !aura.can_be_saved || aura.is_pet_aura {
                continue;
            }

            let caster_guid = if aura.caster_guid == pet_guid {
                ObjectGuid::EMPTY
            } else {
                aura.caster_guid
            };

            operations.push(PetAuraSaveOperationLikeCpp::InsertAura {
                pet_number,
                caster_guid,
                spell_id: aura.spell_id,
                effect_mask: aura.effect_mask,
                recalculate_mask: aura.recalculate_mask,
                difficulty: aura.difficulty,
                stack_count: aura.stack_count,
                max_duration_ms: aura.max_duration_ms,
                duration_ms: aura.duration_ms,
                charges: aura.charges,
            });

            for effect in &aura.effects {
                operations.push(PetAuraSaveOperationLikeCpp::InsertAuraEffect {
                    pet_number,
                    caster_guid,
                    spell_id: aura.spell_id,
                    effect_mask: aura.effect_mask,
                    effect_index: effect.effect_index,
                    amount: effect.amount,
                    base_amount: effect.base_amount,
                });
            }
        }

        operations
    }
    pub fn is_permanent_pet_for(&self, owner_guid: ObjectGuid, pet_number: u32) -> bool {
        self.owner_guid == owner_guid && self.pet_type == PetType::Hunter && pet_number != 0
    }
    pub fn pet_next_level_xp_for_owner_level(owner_next_level_xp: u32) -> u32 {
        (owner_next_level_xp as f32 * PET_XP_FACTOR) as u32
    }
    pub fn generate_action_bar_data_like_cpp(action_bar: &[u32]) -> String {
        let mut data = String::new();
        for packed in action_bar.iter().take(10) {
            data.push_str(&format!(
                "{} {} ",
                unit_action_button_type_like_cpp(*packed),
                unit_action_button_action_like_cpp(*packed)
            ));
        }
        data
    }
    pub fn cleanup_action_bar_like_cpp(
        &mut self,
        action_bar: &mut [u32; MAX_UNIT_ACTION_BAR_INDEX],
        mut spell_exists_like_cpp: impl FnMut(u32) -> bool,
    ) -> PetCleanupActionBarOutcomeLikeCpp {
        let mut operations = Vec::new();

        for (index, packed) in action_bar.iter_mut().enumerate() {
            let action = unit_action_button_action_like_cpp(*packed);
            let active_type = unit_action_button_type_like_cpp(*packed);
            if action == 0 || !Self::is_action_bar_for_spell_like_cpp(active_type) {
                continue;
            }

            if !self.has_spell(action) {
                *packed = make_unit_action_button_like_cpp(0, ACT_PASSIVE_LIKE_CPP);
                operations.push(PetCleanupActionBarOperationLikeCpp::ClearSlot { index });
            } else if active_type == ACT_ENABLED_LIKE_CPP && spell_exists_like_cpp(action) {
                self.toggle_autocast(action, true);
                operations.push(PetCleanupActionBarOperationLikeCpp::EnableAutocast {
                    index,
                    spell_id: action,
                });
            }
        }

        PetCleanupActionBarOutcomeLikeCpp {
            action_bar: *action_bar,
            operations,
        }
    }
    pub fn learn_spell_high_rank_like_cpp(
        &mut self,
        spell_id: u32,
        mut next_spell_in_chain_like_cpp: impl FnMut(u32) -> u32,
    ) -> PetLearnSpellHighRankOutcomeLikeCpp {
        let mut attempted_spell_ids = Vec::new();
        let mut learned_spell_ids = Vec::new();
        let mut current_spell_id = spell_id;

        while current_spell_id != 0 {
            attempted_spell_ids.push(current_spell_id);

            if self.add_spell(
                current_spell_id,
                ActiveState::Decide,
                PetSpellState::New,
                PetSpellType::Normal,
            ) {
                learned_spell_ids.push(current_spell_id);
            }

            current_spell_id = next_spell_in_chain_like_cpp(current_spell_id);
        }

        PetLearnSpellHighRankOutcomeLikeCpp {
            attempted_spell_ids,
            learned_spell_ids,
        }
    }
    pub fn learn_pet_passives_like_cpp(
        &mut self,
        creature_family_id: Option<u32>,
        creature_family_exists_like_cpp: impl FnOnce(u32) -> bool,
        pet_family_spell_ids_like_cpp: impl FnOnce(u32) -> Vec<u32>,
    ) -> PetLearnPetPassivesOutcomeLikeCpp {
        let Some(creature_family_id) = creature_family_id else {
            return PetLearnPetPassivesOutcomeLikeCpp {
                creature_template_missing: true,
                creature_family_missing: false,
                attempted_spell_ids: Vec::new(),
                learned_spell_ids: Vec::new(),
            };
        };

        if !creature_family_exists_like_cpp(creature_family_id) {
            return PetLearnPetPassivesOutcomeLikeCpp {
                creature_template_missing: false,
                creature_family_missing: true,
                attempted_spell_ids: Vec::new(),
                learned_spell_ids: Vec::new(),
            };
        }

        let attempted_spell_ids = pet_family_spell_ids_like_cpp(creature_family_id)
            .into_iter()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let mut learned_spell_ids = Vec::new();

        for spell_id in attempted_spell_ids.iter().copied() {
            if self.add_spell(
                spell_id,
                ActiveState::Decide,
                PetSpellState::New,
                PetSpellType::Family,
            ) {
                learned_spell_ids.push(spell_id);
            }
        }

        PetLearnPetPassivesOutcomeLikeCpp {
            creature_template_missing: false,
            creature_family_missing: false,
            attempted_spell_ids,
            learned_spell_ids,
        }
    }
    pub fn learn_pet_talent_like_cpp(&self, talent_id: u32) -> PetLearnPetTalentOutcomeLikeCpp {
        PetLearnPetTalentOutcomeLikeCpp {
            talent_id,
            debug_log_only: true,
        }
    }
    pub fn cast_pet_aura_like_cpp(
        &self,
        owner_pet_aura_index: usize,
        aura: &PetAuraLikeCpp,
        pet_stamina_like_cpp: f32,
        pet_intellect_like_cpp: f32,
    ) -> Option<PetCastPetAuraPlanLikeCpp> {
        let aura_id = aura.aura_for_pet_entry_like_cpp(self.creature.entry());
        if aura_id == 0 {
            return None;
        }

        let spell_value_base_point0 = (aura_id == DEMONIC_KNOWLEDGE_AURA_LIKE_CPP).then(|| {
            (aura.damage as f32 * (pet_stamina_like_cpp + pet_intellect_like_cpp) / 100.0) as i32
        });

        Some(PetCastPetAuraPlanLikeCpp {
            owner_pet_aura_index,
            aura_id,
            spell_value_base_point0,
        })
    }
    pub fn cast_pet_auras_like_cpp(
        &self,
        current: bool,
        owner_class: Class,
        creature_type: CreatureType,
        owner_pet_auras: &[PetAuraLikeCpp],
        pet_stamina_like_cpp: f32,
        pet_intellect_like_cpp: f32,
    ) -> PetCastPetAurasOutcomeLikeCpp {
        if !Self::is_permanent_pet_for_like_cpp(self.pet_type, owner_class, creature_type) {
            return PetCastPetAurasOutcomeLikeCpp {
                skipped_not_permanent: true,
                removed_owner_pet_aura_indices: Vec::new(),
                removed_pet_aura_spell_ids: Vec::new(),
                cast_auras: Vec::new(),
            };
        }

        let mut removed_owner_pet_aura_indices = Vec::new();
        let mut removed_pet_aura_spell_ids = Vec::new();
        let mut cast_auras = Vec::new();

        for (index, aura) in owner_pet_auras.iter().enumerate() {
            if !current && aura.remove_on_change_pet {
                removed_owner_pet_aura_indices.push(index);
                let aura_id = aura.aura_for_pet_entry_like_cpp(self.creature.entry());
                if aura_id != 0 {
                    removed_pet_aura_spell_ids.push(aura_id);
                }
            } else if let Some(plan) = self.cast_pet_aura_like_cpp(
                index,
                aura,
                pet_stamina_like_cpp,
                pet_intellect_like_cpp,
            ) {
                cast_auras.push(plan);
            }
        }

        PetCastPetAurasOutcomeLikeCpp {
            skipped_not_permanent: false,
            removed_owner_pet_aura_indices,
            removed_pet_aura_spell_ids,
            cast_auras,
        }
    }
    pub fn is_pet_aura_like_cpp(
        &self,
        owner_pet_auras: &[PetAuraLikeCpp],
        aura_spell_id: u32,
    ) -> bool {
        owner_pet_auras
            .iter()
            .any(|aura| aura.aura_for_pet_entry_like_cpp(self.creature.entry()) == aura_spell_id)
    }
    pub fn fill_pet_info_like_cpp(
        &self,
        pet_number: u32,
        action_bar: &[u32],
        forced_react_state: Option<ReactState>,
        can_be_renamed: bool,
        last_save_time: u32,
        created_by_spell_id: u32,
    ) -> PetStableInfo {
        let unit = self.creature.unit();
        PetStableInfo {
            name: unit.world().name().to_string(),
            action_bar: Self::generate_action_bar_data_like_cpp(action_bar),
            pet_number,
            creature_id: self.creature.entry(),
            display_id: unit.data().native_display_id as u32,
            experience: self.pet_experience,
            health: unit.data().health as u32,
            mana: unit.get_power(PowerType::Mana) as u32,
            last_save_time,
            created_by_spell_id,
            specialization_id: self.pet_specialization,
            level: self.creature.level(),
            react_state: forced_react_state.unwrap_or_else(|| self.creature.react_state()),
            pet_type: self.pet_type,
            was_renamed: !can_be_renamed,
        }
    }
    pub fn prepare_save_pet_to_db_like_cpp(
        &self,
        mut mode: i16,
        pet_number: u32,
        temporary_unsummoned_pet_number: Option<u32>,
        current_active_pet_index: Option<u32>,
    ) -> Result<PetSaveToDbPlan, PetSaveToDbSkipReason> {
        if self.creature.entry() == 0 {
            return Err(PetSaveToDbSkipReason::ZeroEntry);
        }

        if !self.is_controlled() {
            return Err(PetSaveToDbSkipReason::NotControlled);
        }

        if !self.owner_guid.is_player() {
            return Err(PetSaveToDbSkipReason::OwnerNotPlayer);
        }

        if mode == PetSaveMode::AsCurrent as i16 {
            if temporary_unsummoned_pet_number.is_some_and(|number| number != pet_number) {
                if self.pet_type == PetType::Hunter {
                    return Err(PetSaveToDbSkipReason::TemporaryUnsummonedHunterCurrent);
                }
                mode = PetSaveMode::NotInSlot as i16;
            }
        }

        if mode == PetSaveMode::AsCurrent as i16 {
            if let Some(active_slot) = current_active_pet_index {
                mode = active_slot as i16;
            }
        }

        let delete_path = mode == PetSaveMode::AsDeleted as i16;
        let remove_all_auras_before_spell_save = !PetSaveMode::is_active_slot(mode);
        let insert_slot = if delete_path {
            None
        } else {
            Some(
                current_active_pet_index
                    .map(|slot| slot as i16)
                    .unwrap_or(PetSaveMode::NotInSlot as i16),
            )
        };

        Ok(PetSaveToDbPlan {
            pet_number,
            effective_mode: mode,
            save_auras_before_cleanup: true,
            remove_all_auras_before_spell_save,
            save_spells: true,
            save_spell_history: true,
            delete_existing_pet_row: !delete_path,
            fill_pet_info: !delete_path,
            insert_pet_row: !delete_path,
            insert_slot,
            remove_all_auras_before_delete: delete_path,
            delete_from_db_pet_number: delete_path.then_some(pet_number),
        })
    }
    pub fn delete_from_db_plan_like_cpp(pet_number: u32) -> Vec<PetDeleteFromDbOperationLikeCpp> {
        vec![
            PetDeleteFromDbOperationLikeCpp::BeginTransaction,
            PetDeleteFromDbOperationLikeCpp::DeleteCharacterPetById { pet_number },
            PetDeleteFromDbOperationLikeCpp::DeleteCharacterPetDeclinedName { pet_number },
            PetDeleteFromDbOperationLikeCpp::DeletePetAuraEffects { pet_number },
            PetDeleteFromDbOperationLikeCpp::DeletePetAuras { pet_number },
            PetDeleteFromDbOperationLikeCpp::DeletePetSpells { pet_number },
            PetDeleteFromDbOperationLikeCpp::DeletePetSpellCooldowns { pet_number },
            PetDeleteFromDbOperationLikeCpp::DeletePetSpellCharges { pet_number },
            PetDeleteFromDbOperationLikeCpp::CommitTransaction,
        ]
    }
    pub fn get_load_pet_info(
        stable: &PetStable,
        pet_entry: u32,
        pet_number: u32,
        slot: Option<i16>,
    ) -> Option<PetLoadSelection> {
        Self::get_load_pet_info_result_like_cpp(stable, pet_entry, pet_number, slot).selection()
    }
    pub fn get_load_pet_info_result_like_cpp(
        stable: &PetStable,
        pet_entry: u32,
        pet_number: u32,
        slot: Option<i16>,
    ) -> PetLoadInfoResult {
        if pet_number != 0 {
            for (index, pet) in stable.active_pets.iter().enumerate() {
                if let Some(pet) = pet {
                    if pet.pet_number == pet_number {
                        return PetLoadInfoResult::Found(PetLoadSelection {
                            pet_number: pet.pet_number,
                            creature_id: pet.creature_id,
                            slot: index as i16,
                        });
                    }
                }
            }
            for (index, pet) in stable.stabled_pets.iter().enumerate() {
                if let Some(pet) = pet {
                    if pet.pet_number == pet_number {
                        return PetLoadInfoResult::Found(PetLoadSelection {
                            pet_number: pet.pet_number,
                            creature_id: pet.creature_id,
                            slot: PetSaveMode::stable_slot(index as u16),
                        });
                    }
                }
            }
            for pet in &stable.unslotted_pets {
                if pet.pet_number == pet_number {
                    return PetLoadInfoResult::Found(PetLoadSelection {
                        pet_number: pet.pet_number,
                        creature_id: pet.creature_id,
                        slot: PetSaveMode::NotInSlot as i16,
                    });
                }
            }
        } else if let Some(slot) = slot {
            if slot == PetSaveMode::AsCurrent as i16 {
                if let Some(index) = stable.current_pet_index {
                    if let Some(Some(pet)) = stable.active_pets.get(index as usize) {
                        return PetLoadInfoResult::Found(PetLoadSelection {
                            pet_number: pet.pet_number,
                            creature_id: pet.creature_id,
                            slot: index as i16,
                        });
                    }
                }
            } else if PetSaveMode::is_active_slot(slot) {
                if let Some(Some(pet)) = stable.active_pets.get(slot as usize) {
                    return PetLoadInfoResult::Found(PetLoadSelection {
                        pet_number: pet.pet_number,
                        creature_id: pet.creature_id,
                        slot,
                    });
                }
            } else if PetSaveMode::is_stabled_slot(slot) {
                let index = (slot - PetSaveMode::stable_slot(0)) as usize;
                if let Some(Some(pet)) = stable.stabled_pets.get(index) {
                    return PetLoadInfoResult::Found(PetLoadSelection {
                        pet_number: pet.pet_number,
                        creature_id: pet.creature_id,
                        slot,
                    });
                }
            }
        } else if pet_entry != 0 {
            for pet in &stable.unslotted_pets {
                if pet.creature_id == pet_entry {
                    return PetLoadInfoResult::Found(PetLoadSelection {
                        pet_number: pet.pet_number,
                        creature_id: pet.creature_id,
                        slot: PetSaveMode::NotInSlot as i16,
                    });
                }
            }
        } else {
            if let Some(Some(pet)) = stable.active_pets.first() {
                return PetLoadInfoResult::Found(PetLoadSelection {
                    pet_number: pet.pet_number,
                    creature_id: pet.creature_id,
                    slot: PetSaveMode::FirstActiveSlot as i16,
                });
            }
            if let Some(pet) = stable.unslotted_pets.first() {
                return PetLoadInfoResult::Found(PetLoadSelection {
                    pet_number: pet.pet_number,
                    creature_id: pet.creature_id,
                    slot: PetSaveMode::NotInSlot as i16,
                });
            }
        }

        PetLoadInfoResult::Deleted
    }
    pub(super) fn sync_autospell(&mut self, spell_id: u32, active: ActiveState) {
        if active == ActiveState::Enabled {
            if !self.autospells.contains(&spell_id) {
                self.autospells.push(spell_id);
            }
        } else {
            self.autospells.retain(|known| *known != spell_id);
        }
    }
    pub(super) fn is_action_bar_for_spell_like_cpp(active_type: u8) -> bool {
        matches!(
            active_type,
            ACT_DISABLED_LIKE_CPP | ACT_ENABLED_LIKE_CPP | ACT_PASSIVE_LIKE_CPP
        )
    }
    pub(super) fn remove_spell_from_action_bar_like_cpp(
        &self,
        spell_id: u32,
        action_bar: &mut [u32; MAX_UNIT_ACTION_BAR_INDEX],
        first_spell_in_chain_like_cpp: &mut impl FnMut(u32) -> u32,
    ) -> Option<usize> {
        let first_id = first_spell_in_chain_like_cpp(spell_id);

        for (index, packed) in action_bar.iter_mut().enumerate() {
            let action = unit_action_button_action_like_cpp(*packed);
            if action == 0 {
                continue;
            }
            if Self::is_action_bar_for_spell_like_cpp(unit_action_button_type_like_cpp(*packed))
                && first_spell_in_chain_like_cpp(action) == first_id
            {
                *packed = make_unit_action_button_like_cpp(0, ACT_PASSIVE_LIKE_CPP);
                return Some(index);
            }
        }

        None
    }
}
