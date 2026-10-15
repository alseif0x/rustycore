//! Represented effects that change progression state: quests, reputation and proficiency.
//!
//! Moved out of the Session root under #621. Behaviour is preserved. The raid
//! marker effect moved to `wow_world_spell` (#1363 slice 9d).

use super::*;

impl WorldSession {
    /// C++ `Spell::EffectReputation`.
    ///
    /// Represented boundary: current player target only. This follows the C++
    /// spell-source reputation formula and `ReputationMgr::ModifyReputation`
    /// packet path; generic player targets, persistence and script callbacks
    /// remain outside this bounded slice.
    pub(in crate::session) fn apply_reputation_effect_like_cpp(
        &mut self,
        damage: i32,
        misc_value: i32,
        target_guid: ObjectGuid,
    ) -> bool {
        let Some(player_guid) = self.player_guid() else {
            return false;
        };
        if target_guid != player_guid {
            return false;
        }
        let Ok(faction_id) = u32::try_from(misc_value) else {
            return false;
        };
        let Some(faction_store) = self.catalogs.faction_store().map(Arc::clone) else {
            return false;
        };
        let Some(faction_entry) = faction_store.get(faction_id).cloned() else {
            return false;
        };

        let reputation = wow_world_social::calculate_reputation_gain_like_cpp(
            self,
            ReputationGainSourceLikeCpp::Spell,
            0,
            damage,
            faction_id,
            false,
        );

        let reputation_spillover_template_store = self
            .catalogs
            .reputation_spillover_template_store()
            .map(Arc::clone);
        let friendship_rep_reaction_store = self
            .catalogs
            .friendship_rep_reaction_store()
            .map(Arc::clone);
        let paragon_reputation_store = self.catalogs.paragon_reputation_store().map(Arc::clone);
        let currency_types_store = self.currency_types_store().map(Arc::clone);
        let db_spillover_template = reputation_spillover_template_store
            .as_deref()
            .and_then(|store| store.get(faction_id));
        let options = wow_progression::mgr::SetReputationOptionsLikeCpp {
            incremental: true,
            spillover_only: false,
            no_spillover: false,
            reputation_gain_rate: self.config.reputation_rates_like_cpp().gain,
            paragon_reward_quest_status_none_like_cpp: true,
            renown_current_level_like_cpp: 0,
            renown_currency_increased_cap_quantity_like_cpp: 0,
            player_race: crate::session::hub_ref(self).player_race_like_cpp(),
            player_class: crate::session::hub_ref(self).player_class_like_cpp(),
        };
        let Some((outcome, packet)) =
            crate::session::hub_mut(self).mutate_reputation_mgr_like_cpp(|mgr| {
                let outcome = mgr.set_reputation_like_cpp(
                    &faction_entry,
                    reputation,
                    options,
                    &faction_store,
                    db_spillover_template,
                    friendship_rep_reaction_store.as_deref(),
                    paragon_reputation_store.as_deref(),
                    currency_types_store.as_deref(),
                );
                let packet = outcome
                    .send_state_rep_list_id
                    .map(|rep_list_id| mgr.set_faction_standing_packet_like_cpp(Some(rep_list_id)));
                (outcome, packet)
            })
        else {
            return false;
        };
        if let Some(packet) = packet {
            self.send_packet(&packet);
        }
        outcome.applied
    }
    pub(in crate::session) fn apply_proficiency_effect_like_cpp(
        &mut self,
        spell_id: i32,
    ) -> Result<(), &'static str> {
        let _ = self.player_guid().ok_or("No player GUID")?;
        let Some(equipped) = self
            .catalogs
            .spell_catalogs
            .spell_equipped_items_store
            .as_ref()
            .and_then(|store| store.entry_for_spell_id_like_cpp(spell_id))
            .cloned()
        else {
            return Ok(());
        };

        let sub_class_mask = u32::try_from(equipped.equipped_item_subclass).unwrap_or(0);
        if sub_class_mask == 0 {
            return Ok(());
        }

        let update = if equipped.equipped_item_class == ItemClass::Weapon as i8 {
            let canonical = self.core.with_owned_player_mut_like_cpp(|player| {
                player
                    .add_weapon_proficiency_like_cpp(sub_class_mask)
                    .map(|mask| (ItemClass::Weapon as u8, mask))
            });
            #[cfg(test)]
            let canonical = canonical.or_else(|| {
                self.core.player_handle_like_cpp.is_none().then(|| {
                    self.mutate_player_persistent_capability_state_like_cpp(|state| {
                        if state.weapon_proficiency & sub_class_mask == 0 {
                            state.weapon_proficiency |= sub_class_mask;
                            Some((ItemClass::Weapon as u8, state.weapon_proficiency))
                        } else {
                            None
                        }
                    })
                    .expect("handle-less fixture capability owner")
                })
            });
            canonical
        } else if equipped.equipped_item_class == ItemClass::Armor as i8 {
            let canonical = self.core.with_owned_player_mut_like_cpp(|player| {
                player
                    .add_armor_proficiency_like_cpp(sub_class_mask)
                    .map(|mask| (ItemClass::Armor as u8, mask))
            });
            #[cfg(test)]
            let canonical = canonical.or_else(|| {
                self.core.player_handle_like_cpp.is_none().then(|| {
                    self.mutate_player_persistent_capability_state_like_cpp(|state| {
                        if state.armor_proficiency & sub_class_mask == 0 {
                            state.armor_proficiency |= sub_class_mask;
                            Some((ItemClass::Armor as u8, state.armor_proficiency))
                        } else {
                            None
                        }
                    })
                    .expect("handle-less fixture capability owner")
                })
            });
            canonical
        } else {
            Some(None)
        };
        let Some(update) = update else {
            return Err("No canonical Player persistent-capability owner");
        };
        if let Some((proficiency_class, proficiency_mask)) = update {
            self.send_packet(&SetProficiency {
                proficiency_mask,
                proficiency_class,
            });
        }

        Ok(())
    }
    pub(in crate::session) async fn apply_quest_complete_effect_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        target_guid: ObjectGuid,
        quest_id: i32,
    ) -> Result<(), &'static str> {
        self.invalidate_player_quest_status_authority_like_cpp();
        const QUEST_FLAGS_TRACKING_EVENT_LIKE_CPP_LOCAL: u32 = 0x0000_0400;

        let player_guid = self.player_guid().ok_or("No player GUID")?;
        if target_guid != player_guid {
            return Ok(());
        }

        let Ok(quest_id) = u32::try_from(quest_id) else {
            debug!(
                account = self.core.account_id,
                "Skipping represented quest-complete spell effect with negative MiscValue"
            );
            return Ok(());
        };
        if quest_id == 0 {
            return Ok(());
        }

        let Some(quest) = self
            .catalogs
            .quests
            .store
            .as_deref()
            .and_then(|store| store.get(quest_id))
            .cloned()
        else {
            return Ok(());
        };

        let Some((quest_is_in_log, should_send_event_complete)) =
            wow_world_quest::mark_represented_quest_explored_like_cpp(
                self,
                quest_id,
                wow_conditions::QUEST_STATUS_FAILED_LIKE_CPP,
            )
        else {
            return Ok(());
        };

        if quest_is_in_log {
            if should_send_event_complete {
                self.send_packet(&wow_packet::packets::quest::QuestUpdateComplete { quest_id });
            }
            self.complete_represented_quest_after_add_with_generator_like_cpp(
                item_guid_generator,
                &quest,
            )
            .await;
        } else if (quest.flags & QUEST_FLAGS_TRACKING_EVENT_LIKE_CPP_LOCAL) != 0 {
            if wow_world_quest::set_represented_quest_rewarded_like_cpp(self, quest_id, true)
                == false
            {
                return Ok(());
            }
            let quest_bit = self
                .catalogs
                .quests
                .v2_store
                .as_deref()
                .map(|store| store.get_quest_unique_bit_flag_like_cpp(quest_id))
                .unwrap_or(0);
            if quest_bit != 0 {
                wow_world_quest::set_loaded_quest_completed_bit_like_cpp(self, quest_bit);
            }
            self.sync_player_registry_state_like_cpp();
        }

        Ok(())
    }
}
