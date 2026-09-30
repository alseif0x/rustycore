// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Group loot rolls: need/greed/disenchant voting and winner selection.

use super::*;

mod voting;
mod completion;


mod publication;

impl WorldSession {




















    pub(super) fn represented_start_group_loot_rolls_on_first_open_like_cpp(
        &mut self,
        item_valuation: &ItemValuationCatalogsLikeCpp,
        owner_guid: ObjectGuid,
        player_guid: ObjectGuid,
    ) {
        let Some(authority) = self.represented_owned_loot_authority_like_cpp(owner_guid) else {
            return;
        };
        let Some(authority_snapshot) = authority.snapshot_for_player_like_cpp(player_guid) else {
            return;
        };
        let authority_generation = authority_snapshot.generation;
        let authority_scope = authority_snapshot.scope;
        let current_map_id = self.player_map_id_like_cpp();
        let current_instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        let player_registry = self.player_registry().cloned();
        let mut packets = Vec::new();
        let mut auto_pass_packets = Vec::new();
        let mut pending_rolls = Vec::new();
        let mut unblocked_without_roll = Vec::new();
        let item_flags2_by_item_id: HashMap<u32, (Option<u32>, Option<u16>)> = self
            .loot_table
            .get(&owner_guid)
            .map(|loot| {
                loot.items
                    .iter()
                    .map(|entry| {
                        (
                            entry.item_id,
                            (
                                self.item_template_flags2(entry.item_id),
                                self.represented_loot_roll_disenchant_skill_required_like_cpp(
                                    item_valuation,
                                    entry.item_id,
                                ),
                            ),
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();
        let current_player_enchanting_skill = self.resolved_enchanting_skill_like_cpp();
        let Some(pass_on_group_loot) = self.resolved_pass_on_group_loot_like_cpp() else {
            return;
        };

        if let Some(loot) = self.loot_table.get_mut(&owner_guid) {
            for entry in &mut loot.items {
                if !entry.flags.blocked {
                    continue;
                }

                let eligible_looters = connected_roll_looters_like_cpp(
                    entry,
                    player_guid,
                    current_map_id,
                    current_instance_id,
                    player_registry.as_deref(),
                );
                let Some(ballots) = wow_loot::RollBallots::start(
                    entry,
                    eligible_looters.len(),
                    player_guid,
                    pass_on_group_loot,
                    |looter| {
                        player_registry.as_deref().and_then(|registry| {
                            registry.loot_pass_on_group_loot(
                                looter,
                                current_map_id,
                                current_instance_id,
                            )
                        })
                    },
                ) else {
                    unblocked_without_roll.push(entry.loot_list_id);
                    continue;
                };
                let command_identity = LootRollCommandIdentityLikeCpp::new_like_cpp(
                    loot.loot_guid,
                    entry.loot_list_id,
                    authority.clone(),
                    authority_generation,
                );
                let state = RepresentedLootRollState {
                    owner_guid,
                    loot_obj: loot.loot_guid,
                    loot_list_id: entry.loot_list_id,
                    authority: authority.clone(),
                    authority_generation,
                    authority_scope,
                    command_identity,
                    end_time: Instant::now()
                        + Duration::from_millis(u64::from(LOOT_ROLL_TIMEOUT_MS_LIKE_CPP)),
                    ballots,
                };
                let max_enchanting_skill = represented_max_enchanting_skill_like_cpp(
                    &eligible_looters,
                    player_guid,
                    current_player_enchanting_skill,
                    player_registry.as_deref(),
                );
                let (item_flags2, disenchant_skill_required) = item_flags2_by_item_id
                    .get(&entry.item_id)
                    .copied()
                    .unwrap_or((None, None));
                let valid_rolls = wow_loot::represented_loot_roll_valid_rolls_like_cpp(
                    item_flags2,
                    disenchant_skill_required,
                    max_enchanting_skill,
                );

                for (looter, vote) in state.ballots.votes() {
                    if vote.vote != ROLL_VOTE_NOT_EMITTED_YET_LIKE_CPP {
                        continue;
                    }

                    packets.push((
                        *looter,
                        start_loot_roll_packet_like_cpp(
                            loot.loot_guid,
                            current_map_id,
                            loot.loot_method,
                            entry,
                            valid_rolls,
                            loot.dungeon_encounter_id as i32,
                        ),
                    ));
                }

                for (looter, vote) in state.ballots.votes() {
                    if vote.vote != ROLL_VOTE_PASS_LIKE_CPP {
                        continue;
                    }

                    auto_pass_packets.push((
                        LootRollBroadcast {
                            loot_obj: loot.loot_guid,
                            player: *looter,
                            roll: -1,
                            roll_type: ROLL_VOTE_PASS_LIKE_CPP,
                            item: loot_roll_broadcast_item_like_cpp(
                                entry,
                                LOOT_SLOT_TYPE_ROLL_ONGOING_LIKE_CPP,
                            ),
                            autopassed: false,
                            off_spec: false,
                            dungeon_encounter_id: loot.dungeon_encounter_id as i32,
                        },
                        state.clone(),
                    ));
                }

                pending_rolls.push(state);
            }
        }

        if !unblocked_without_roll.is_empty()
            && let Some(authority) = self.represented_owned_loot_authority_like_cpp(owner_guid)
        {
            for loot_list_id in unblocked_without_roll {
                let _ = authority.finish_item_roll_like_cpp(
                    player_guid,
                    authority_generation,
                    loot_list_id,
                    true,
                    None,
                );
            }
            let _ = self.reconcile_represented_loot_cache_like_cpp(owner_guid, player_guid);
        }

        for roll in pending_rolls {
            self.represented_loot_rolls
                .insert((roll.loot_obj, roll.loot_list_id), roll);
        }
        self.publish_represented_loot_roll_ownership_like_cpp();

        for (looter, packet) in packets {
            if looter == player_guid {
                self.send_packet(&packet);
                continue;
            }

            let Some(registry) = self.player_registry() else {
                continue;
            };
            let Some(registration) = registry.loot_delivery_recipient(
                looter,
                self.player_map_id_like_cpp(),
                current_instance_id,
            ) else {
                continue;
            };

            let _ = registry.send_current_packet(registration, packet.to_bytes());
        }

        for (packet, state) in auto_pass_packets {
            self.broadcast_represented_loot_roll_packet_to_voters_like_cpp(&packet, &state, None);
        }
    }

    fn publish_represented_loot_roll_ownership_like_cpp(&self) {
        let Some(player_guid) = self.player_guid() else {
            return;
        };
        let Some(registry) = self.player_registry() else {
            return;
        };
        let identities = self
            .represented_loot_rolls
            .values()
            .map(|state| state.command_identity.clone())
            .collect();
        let _ = registry.replace_loot_rolls_for_control_channel(
            player_guid,
            &self.session_command_tx(),
            identities,
        );
    }

    #[cfg(test)]
    pub(in crate::handlers) async fn tick_represented_loot_rolls_like_cpp(&mut self) {
        let generators = self.id_generators_for_test_like_cpp();
        let item_valuation = self.item_valuation_catalogs_for_test_like_cpp();
        self.tick_represented_loot_rolls_with_generator_like_cpp(
            generators.item.as_ref(),
            &item_valuation,
        )
        .await;
    }

    pub(crate) async fn tick_represented_loot_rolls_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        item_valuation: &ItemValuationCatalogsLikeCpp,
    ) {
        let now = Instant::now();
        let roll_keys: Vec<(ObjectGuid, u8)> =
            self.represented_loot_rolls.keys().copied().collect();

        for (loot_obj, loot_list_id) in roll_keys {
            let Some(state) = self
                .represented_loot_rolls
                .get(&(loot_obj, loot_list_id))
                .cloned()
            else {
                continue;
            };
            if self
                .represented_current_loot_roll_authority_like_cpp(&state)
                .is_none()
            {
                self.cancel_represented_loot_roll_generation_mismatch_like_cpp(
                    (loot_obj, loot_list_id),
                    &state,
                );
                continue;
            }
            if state.end_time > now {
                continue;
            }

            let owner_guid = state.owner_guid;
            let Some(entry) = self.loot_table.get(&owner_guid).and_then(|loot| {
                loot.items
                    .iter()
                    .find(|entry| entry.loot_list_id == loot_list_id)
                    .cloned()
            }) else {
                self.represented_loot_rolls
                    .remove(&(loot_obj, loot_list_id));
                self.publish_represented_loot_roll_ownership_like_cpp();
                continue;
            };

            let winner = state.ballots.current_winner();
            self.finish_represented_loot_roll_like_cpp(
                item_guid_generator,
                item_valuation,
                loot_obj,
                loot_list_id,
                &entry,
                winner,
                Some(&state),
            )
            .await;
        }
    }

    fn represented_loot_roll_disenchant_skill_required_like_cpp(
        &self,
        item_valuation: &ItemValuationCatalogsLikeCpp,
        item_id: u32,
    ) -> Option<u16> {
        let template = self
            .item_stats_store()
            .and_then(|store| store.random_property_template(item_id))?;
        self.item_disenchant_loot_with_catalogs_like_cpp(
            item_valuation,
            item_id,
            template.quality as u32,
            u32::from(template.item_level),
            true,
        )
        .map(|(_, skill_required)| skill_required)
    }
}
