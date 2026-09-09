//! Represented effects that change progression state: quests, reputation, proficiency and raid markers.
//!
//! Moved out of the Session root under #621. Behaviour is preserved.

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
        let Some(faction_store) = self.faction_store().map(Arc::clone) else {
            return false;
        };
        let Some(faction_entry) = faction_store.get(faction_id).cloned() else {
            return false;
        };

        let reputation = self.calculate_reputation_gain_like_cpp(
            ReputationGainSourceLikeCpp::Spell,
            0,
            damage,
            faction_id,
            false,
        );

        let reputation_spillover_template_store =
            self.reputation_spillover_template_store().map(Arc::clone);
        let friendship_rep_reaction_store = self.friendship_rep_reaction_store().map(Arc::clone);
        let paragon_reputation_store = self.paragon_reputation_store().map(Arc::clone);
        let currency_types_store = self.currency_types_store().map(Arc::clone);
        let db_spillover_template = reputation_spillover_template_store
            .as_deref()
            .and_then(|store| store.get(faction_id));
        let options = crate::reputation::mgr::SetReputationOptionsLikeCpp {
            incremental: true,
            spillover_only: false,
            no_spillover: false,
            reputation_gain_rate: self.reputation_rates_like_cpp().gain,
            paragon_reward_quest_status_none_like_cpp: true,
            renown_current_level_like_cpp: 0,
            renown_currency_increased_cap_quantity_like_cpp: 0,
            player_race: self.player_race_like_cpp(),
            player_class: self.player_class_like_cpp(),
        };
        let Some((outcome, packet)) = self.mutate_reputation_mgr_like_cpp(|mgr| {
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
        }) else {
            return false;
        };
        if let Some(packet) = packet {
            self.send_packet(&packet);
        }
        outcome.applied
    }
    /// C++ `Spell::EffectChangeRaidMarker`.
    ///
    /// Represented boundary: current in-memory HOME group only. This mirrors
    /// `Group::AddRaidMarker` storage and `SMSG_RAID_MARKERS_CHANGED` fanout to
    /// connected represented members. Full DB persistence, instance/original
    /// group category routing and `CMSG_CLEAR_RAID_MARKER` remain group-runtime
    /// follow-ups.
    pub(in crate::session) fn apply_change_raid_marker_effect_like_cpp(
        &mut self,
        damage: i32,
        target_data: &SpellTargetData,
    ) -> bool {
        let Some(player_guid) = self.player_guid() else {
            return false;
        };
        let Some(destination) = target_data.dst_location.as_ref() else {
            return false;
        };
        let Some(group_guid) = self.resolved_group_guid_like_cpp() else {
            return false;
        };
        let Some(group_registry) = self.group_registry().cloned() else {
            return false;
        };

        let outcome = match group_registry.add_raid_marker_transition_like_cpp(
            group_guid,
            player_guid,
            damage as u8,
            u32::from(self.player_map_id_like_cpp()),
            destination.position,
            destination.transport,
        ) {
            Ok(outcome) => outcome,
            Err(_) => return false,
        };
        let packet_bytes_and_members = {
            let group = outcome.group;
            use wow_packet::ServerPacket;
            let packet = wow_packet::packets::party::RaidMarkersChanged {
                party_index: group.group_category_like_cpp(),
                active_markers: group.active_raid_markers_mask_like_cpp(),
                raid_markers: group
                    .raid_marker_list_like_cpp()
                    .into_iter()
                    .map(|marker| wow_packet::packets::party::RaidMarker {
                        transport_guid: marker.transport_guid,
                        map_id: marker.map_id,
                        position: marker.position,
                    })
                    .collect(),
            }
            .to_bytes();
            (packet, group.members)
        };

        let (packet_bytes, members) = packet_bytes_and_members;
        let mut sent_to_self_via_registry = false;
        if let Some(registry) = self.player_registry().cloned() {
            for member_guid in members {
                let Some(member) = registry.group_presence(member_guid) else {
                    continue;
                };
                if member_guid == player_guid {
                    sent_to_self_via_registry = true;
                }
                let _ = registry.try_send_current_packet(member.registration, packet_bytes.clone());
            }
        }
        if !sent_to_self_via_registry {
            self.send_raw_packet(&packet_bytes);
        }
        true
    }
    pub(in crate::session) fn apply_proficiency_effect_like_cpp(
        &mut self,
        spell_id: i32,
    ) -> Result<(), &'static str> {
        let _ = self.player_guid().ok_or("No player GUID")?;
        let Some(equipped) = self
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

        let Some(update) = self.mutate_player_persistent_capability_state_like_cpp(|state| {
            if equipped.equipped_item_class == ItemClass::Weapon as i8
                && (state.weapon_proficiency & sub_class_mask) == 0
            {
                state.weapon_proficiency |= sub_class_mask;
                Some((ItemClass::Weapon as u8, state.weapon_proficiency))
            } else if equipped.equipped_item_class == ItemClass::Armor as i8
                && (state.armor_proficiency & sub_class_mask) == 0
            {
                state.armor_proficiency |= sub_class_mask;
                Some((ItemClass::Armor as u8, state.armor_proficiency))
            } else {
                None
            }
        }) else {
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
                account = self.account_id,
                "Skipping represented quest-complete spell effect with negative MiscValue"
            );
            return Ok(());
        };
        if quest_id == 0 {
            return Ok(());
        }

        let Some(quest) = self
            .quest_store
            .as_deref()
            .and_then(|store| store.get(quest_id))
            .cloned()
        else {
            return Ok(());
        };

        let Some((quest_is_in_log, should_send_event_complete)) = self
            .mutate_player_quest_gameplay_like_cpp(|quests| {
                let Some(status) = quests.statuses.get_mut(&quest_id) else {
                    return (false, false);
                };
                let should_send = !status.explored
                    && status.status != crate::conditions::QUEST_STATUS_FAILED_LIKE_CPP;
                if should_send {
                    status.explored = true;
                }
                (true, should_send)
            })
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
            if self
                .mutate_player_quest_gameplay_like_cpp(|quests| {
                    quests.rewarded_quest_ids.insert(quest_id)
                })
                .is_none()
            {
                return Ok(());
            }
            let quest_bit = self
                .quest_v2_store
                .as_deref()
                .map(|store| store.get_quest_unique_bit_flag_like_cpp(quest_id))
                .unwrap_or(0);
            if quest_bit != 0 {
                self.set_loaded_quest_completed_bit_like_cpp(quest_bit);
            }
            self.sync_player_registry_state_like_cpp();
        }

        Ok(())
    }
}
