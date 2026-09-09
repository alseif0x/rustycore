//! Represented use of scripted gameobjects: goobers, rituals, fishing nodes and meeting stones.
//!
//! Moved out of the Session root under #599. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(crate) fn use_represented_gameobject_ritual_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        source: wow_entities::RitualUseSource,
    ) -> bool {
        let (owner_guid, ritual_owner_guid, owner_current_channeled_spell_active) = {
            let state = self.represented_gameobject_use_states.get(&gameobject_guid);
            (
                state.and_then(|state| state.owner_guid),
                state.and_then(|state| state.ritual_owner_guid),
                state.and_then(|state| state.owner_current_channeled_spell_active),
            )
        };

        let ritual_spell_caster_guid = if let Some(owner_guid) = owner_guid {
            if owner_guid == player_guid {
                return false;
            }
            if source.casters_grouped
                && !self.represented_player_is_same_raid_with_like_cpp(player_guid, owner_guid)
            {
                return false;
            }
            if owner_current_channeled_spell_active == Some(false) {
                return false;
            }
            owner_guid
        } else {
            let ritual_owner_guid = ritual_owner_guid.unwrap_or_else(|| {
                self.represented_gameobject_use_states
                    .get(&gameobject_guid)
                    .and_then(|state| state.unique_users.first().copied())
                    .unwrap_or(player_guid)
            });
            if player_guid != ritual_owner_guid
                && source.casters_grouped
                && !self
                    .represented_player_is_same_raid_with_like_cpp(player_guid, ritual_owner_guid)
            {
                return false;
            }
            self.represented_gameobject_use_states
                .entry(gameobject_guid)
                .or_default()
                .ritual_owner_guid
                .get_or_insert(ritual_owner_guid);
            ritual_owner_guid
        };

        let unique_user_count = {
            let state = self
                .represented_gameobject_use_states
                .entry(gameobject_guid)
                .or_default();
            if !state.unique_users.contains(&player_guid) {
                state.unique_users.push(player_guid);
            }
            state.unique_users.len() as u32
        };

        if source.anim_spell_id != 0 {
            self.represented_gameobject_use_effects.push(
                RepresentedGameObjectUseEffect::CastSpell {
                    gameobject_guid,
                    player_guid,
                    spell_id: source.anim_spell_id,
                },
            );
        }

        if unique_user_count != source.casters_required {
            self.represented_gameobject_use_effects.push(
                RepresentedGameObjectUseEffect::RitualWaitingForParticipants {
                    gameobject_guid,
                    player_guid,
                    unique_user_count,
                    casters_required: source.casters_required,
                },
            );
            return false;
        }

        let mut final_spell_id = source.spell_id;
        let mut triggered = source.anim_spell_id != 0;
        if final_spell_id == 62330 {
            final_spell_id = 61993;
            triggered = true;
        }

        if source.caster_target_spell_id != 0 && source.caster_target_spell_id != 1 {
            let unique_users = self
                .represented_gameobject_use_states
                .get(&gameobject_guid)
                .map(|state| state.unique_users.clone())
                .unwrap_or_default();
            self.represented_gameobject_use_effects.push(
                RepresentedGameObjectUseEffect::RitualCasterTargetSpellRequested {
                    gameobject_guid,
                    player_guid,
                    spell_id: source.caster_target_spell_id,
                    target_count: source.caster_target_spell_targets,
                },
            );
            for _ in 0..source.caster_target_spell_targets {
                let Some(target_guid) = unique_users
                    .choose(&mut self.represented_runtime_rng_like_cpp)
                    .copied()
                else {
                    continue;
                };
                if !self
                    .player_registry
                    .as_ref()
                    .is_some_and(|registry| registry.group_presence(target_guid).is_some())
                {
                    continue;
                }
                self.represented_gameobject_use_effects.push(
                    RepresentedGameObjectUseEffect::RitualCasterTargetSpellCast {
                        gameobject_guid,
                        caster_guid: ritual_spell_caster_guid,
                        target_guid,
                        spell_id: source.caster_target_spell_id,
                        triggered: true,
                    },
                );
            }
        }

        if let Some(owner_guid) = owner_guid {
            self.represented_gameobject_use_effects.push(
                RepresentedGameObjectUseEffect::FinishChanneledSpell {
                    player_guid: owner_guid,
                },
            );
        }

        if source.persistent {
            let state = self
                .represented_gameobject_use_states
                .entry(gameobject_guid)
                .or_default();
            state.ritual_owner_guid = None;
            state.unique_users.clear();
            state.use_count = 0;
        } else {
            self.represented_gameobject_use_states
                .entry(gameobject_guid)
                .or_default()
                .loot_state = Some(wow_entities::LootState::JustDeactivated);
        }

        self.represented_gameobject_use_effects.push(
            RepresentedGameObjectUseEffect::RitualCompleted {
                gameobject_guid,
                player_guid,
                final_spell_id,
                triggered,
                persistent: source.persistent,
                unique_user_count,
            },
        );
        self.apply_represented_gameobject_post_use_spell_like_cpp(
            gameobject_guid,
            player_guid,
            gameobject_guid.entry(),
            wow_entities::GAMEOBJECT_TYPE_RITUAL,
            final_spell_id,
            triggered,
            RepresentedGameObjectSpellCaster::User,
            ritual_spell_caster_guid,
        );

        true
    }
    pub(crate) fn use_represented_gameobject_meeting_stone_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        gameobject_entry: u32,
        source: wow_entities::MeetingStoneUseSource,
    ) -> bool {
        let target_guid = self.selection_guid_like_cpp();
        if !target_guid.is_some_and(|target_guid| {
            target_guid != player_guid
                && self.represented_player_is_same_raid_with_like_cpp(player_guid, target_guid)
                && self
                    .player_registry
                    .as_ref()
                    .is_some_and(|registry| registry.group_presence(target_guid).is_some())
        }) {
            self.represented_gameobject_use_effects.push(
                RepresentedGameObjectUseEffect::MeetingStoneTargetRejected {
                    gameobject_guid,
                    player_guid,
                    target_guid,
                },
            );
            return false;
        }
        let target_guid = target_guid.expect("checked above");
        if let Some(content_tuning) = self
            .content_tuning_store
            .as_ref()
            .and_then(|store| store.get(source.content_tuning_id))
        {
            let player_level = self.player_level_like_cpp();
            let target_level = self
                .player_registry
                .as_ref()
                .and_then(|registry| {
                    registry
                        .group_presence(target_guid)
                        .map(|target| target.level)
                })
                .unwrap_or(0);
            if i32::from(player_level) < content_tuning.max_level
                || i32::from(target_level) < content_tuning.max_level
            {
                self.represented_gameobject_use_effects.push(
                    RepresentedGameObjectUseEffect::MeetingStoneLevelRejected {
                        gameobject_guid,
                        player_guid,
                        target_guid,
                        player_level,
                        target_level,
                        required_level: content_tuning.max_level,
                    },
                );
                return false;
            }
        }
        let spell_id = if gameobject_entry == 194097 {
            61994
        } else {
            59782
        };

        self.represented_gameobject_use_effects.push(
            RepresentedGameObjectUseEffect::MeetingStoneSummonRequested {
                gameobject_guid,
                player_guid,
                target_guid,
                gameobject_entry,
                spell_id,
                area_id: source.area_id,
                prevent_unfriendly_outside_instances: source.prevent_unfriendly_outside_instances,
            },
        );

        self.apply_represented_gameobject_post_use_spell_like_cpp(
            gameobject_guid,
            player_guid,
            gameobject_entry,
            wow_entities::GAMEOBJECT_TYPE_MEETINGSTONE,
            spell_id,
            false,
            RepresentedGameObjectSpellCaster::User,
            player_guid,
        );

        true
    }
    pub(crate) fn use_represented_gameobject_fishing_node_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
    ) -> bool {
        let known_owner = self
            .represented_gameobject_use_states
            .get(&gameobject_guid)
            .and_then(|state| state.owner_guid);
        if let Some(owner_guid) = known_owner {
            if owner_guid != player_guid {
                self.represented_gameobject_use_effects.push(
                    RepresentedGameObjectUseEffect::FishingNodeOwnerRejected {
                        gameobject_guid,
                        player_guid,
                        owner_guid,
                    },
                );
                return false;
            }
        }

        let current_loot_state = self
            .represented_gameobject_use_states
            .get(&gameobject_guid)
            .and_then(|state| state.loot_state)
            .unwrap_or(wow_entities::LootState::Ready);

        match current_loot_state {
            wow_entities::LootState::Ready => {
                let (
                    area_fishing_level,
                    player_fishing_level,
                    fishing_roll,
                    explicit_fishing_hole_guid,
                ) = {
                    let represented_area_fishing_level =
                        self.represented_fishing_base_skill_level_like_cpp(gameobject_guid);
                    let represented_player_fishing_level = self
                        .player_profession_skill_value_for_exp_like_cpp(SKILL_FISHING_LIKE_CPP, 0);
                    let state = self
                        .represented_gameobject_use_states
                        .entry(gameobject_guid)
                        .or_default();
                    state.loot_state = Some(wow_entities::LootState::Activated);
                    state.loot_state_unit_guid = player_guid;
                    state.go_state = Some(wow_entities::GoState::Active);
                    state.gameobject_flags = wow_entities::GO_FLAG_IN_MULTI_USE;
                    (
                        state.fishing_area_level.or(represented_area_fishing_level),
                        state
                            .player_fishing_level
                            .or(Some(represented_player_fishing_level)),
                        state.fishing_roll,
                        state.nearby_fishing_hole_guid,
                    )
                };
                let fishing_hole_guid = explicit_fishing_hole_guid.or_else(|| {
                    self.lookup_represented_fishing_hole_around_like_cpp(gameobject_guid)
                });
                self.represented_gameobject_use_effects.push(
                    RepresentedGameObjectUseEffect::FishingNodeActivated {
                        gameobject_guid,
                        player_guid,
                    },
                );
                if let Some(area_fishing_level) = area_fishing_level {
                    self.represented_gameobject_use_effects.push(
                        RepresentedGameObjectUseEffect::FishingSkillUpdated {
                            gameobject_guid,
                            player_guid,
                        },
                    );
                    let player_fishing_level = player_fishing_level.unwrap_or(0);
                    let roll = fishing_roll.unwrap_or(100).clamp(1, 100);
                    let chance =
                        if area_fishing_level > 0 && player_fishing_level < area_fishing_level {
                            (((player_fishing_level as f64 / area_fishing_level as f64).powi(2)
                                * 100.0) as i32)
                                .max(1)
                        } else {
                            100
                        };
                    self.represented_gameobject_use_effects.push(
                        RepresentedGameObjectUseEffect::FishingLootRoll {
                            gameobject_guid,
                            player_guid,
                            player_fishing_level,
                            area_fishing_level,
                            chance,
                            roll,
                        },
                    );

                    let fishing_success = chance >= roll || fishing_hole_guid.is_some();
                    if fishing_success {
                        self.record_represented_gameobject_owner_guid_like_cpp(
                            gameobject_guid,
                            player_guid,
                        );
                        self.set_canonical_gameobject_spell_id_like_cpp(gameobject_guid, 0);
                    }

                    if let Some(fishing_hole_guid) = fishing_hole_guid {
                        self.represented_gameobject_use_states
                            .entry(gameobject_guid)
                            .or_default()
                            .loot_state = Some(wow_entities::LootState::JustDeactivated);
                        self.represented_gameobject_use_effects.push(
                            RepresentedGameObjectUseEffect::FishingHoleDelegated {
                                gameobject_guid,
                                player_guid,
                                fishing_hole_guid,
                            },
                        );
                    } else {
                        let loot_type = if chance >= roll {
                            wow_packet::packets::loot::LOOT_TYPE_FISHING_LIKE_CPP
                        } else {
                            wow_packet::packets::loot::LOOT_TYPE_FISHING_JUNK_LIKE_CPP
                        };
                        self.represented_gameobject_use_effects.push(
                            RepresentedGameObjectUseEffect::FishingLootRequested {
                                gameobject_guid,
                                player_guid,
                                loot_type,
                            },
                        );
                    }
                }
            }
            wow_entities::LootState::JustDeactivated => {}
            _ => {
                self.represented_gameobject_use_states
                    .entry(gameobject_guid)
                    .or_default()
                    .loot_state = Some(wow_entities::LootState::JustDeactivated);
                self.represented_gameobject_use_effects.push(
                    RepresentedGameObjectUseEffect::FishNotHooked {
                        gameobject_guid,
                        player_guid,
                    },
                );
                self.send_packet(&wow_packet::packets::misc::FishNotHooked);
            }
        }

        self.represented_gameobject_use_effects
            .push(RepresentedGameObjectUseEffect::FinishChanneledSpell { player_guid });

        true
    }
    pub(crate) async fn use_represented_gameobject_goober_preamble_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        gameobject_guid: ObjectGuid,
        gameobject_entry: u32,
        gameobject_position: Position,
        player_guid: ObjectGuid,
        source: wow_entities::GooberUseSource,
    ) -> bool {
        if source.page_id != 0 {
            self.send_packet(&wow_packet::packets::misc::PageText { gameobject_guid });
            self.represented_gameobject_use_effects.push(
                RepresentedGameObjectUseEffect::ShowPageText {
                    gameobject_guid,
                    player_guid,
                    page_id: source.page_id,
                },
            );
        } else if source.gossip_id != 0 {
            self.represented_gameobject_use_effects.push(
                RepresentedGameObjectUseEffect::SendGossip {
                    gameobject_guid,
                    player_guid,
                    gossip_id: source.gossip_id,
                },
            );
        }

        if source.event_id != 0 {
            self.represented_gameobject_use_effects.push(
                RepresentedGameObjectUseEffect::TriggerGameEvent {
                    gameobject_guid,
                    player_guid,
                    event_id: source.event_id,
                },
            );
        }

        if source.quest_id != 0
            && self
                .quest_store
                .as_ref()
                .is_some_and(|store| store.get(source.quest_id).is_some())
            && self
                .represented_player_quest_status_like_cpp(source.quest_id)
                .map_or(true, |status| {
                    status != Some(crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP)
                })
        {
            self.represented_gameobject_use_effects.push(
                RepresentedGameObjectUseEffect::GooberQuestGateRejected {
                    gameobject_guid,
                    player_guid,
                    quest_id: source.quest_id,
                },
            );
            return false;
        }

        let mut credit_guids = Vec::new();
        if let (Some(group_guid), Some(group_registry)) =
            (self.resolved_group_guid_like_cpp(), &self.group_registry)
        {
            if let Some(group) = group_registry.get(&group_guid) {
                if group.members.contains(&player_guid) {
                    credit_guids.extend(group.members.iter().copied());
                }
            }
        }
        if credit_guids.is_empty() {
            credit_guids.push(player_guid);
        }

        for credit_guid in credit_guids {
            if self.represented_player_at_group_reward_distance_like_cpp(
                credit_guid,
                self.player_map_id_like_cpp(),
                gameobject_position,
            ) {
                self.represented_gameobject_use_effects.push(
                    RepresentedGameObjectUseEffect::KillCreditGo {
                        gameobject_guid,
                        player_guid: credit_guid,
                        entry: gameobject_entry,
                    },
                );
                if Some(credit_guid) == self.player_guid() {
                    self.kill_credit_gameobject_with_generator_like_cpp(
                        item_guid_generator,
                        gameobject_entry,
                        gameobject_guid,
                    )
                    .await;
                }
            }
        }

        if source.linked_trap_entry != 0 {
            self.represented_gameobject_use_effects.push(
                RepresentedGameObjectUseEffect::TriggerLinkedTrap {
                    gameobject_guid,
                    player_guid,
                    trap_entry: source.linked_trap_entry,
                },
            );
        }

        true
    }
    #[cfg(test)]
    pub(crate) async fn use_represented_gameobject_goober_preamble_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        gameobject_entry: u32,
        gameobject_position: Position,
        player_guid: ObjectGuid,
        source: wow_entities::GooberUseSource,
    ) -> bool {
        let generators = self.id_generators_for_test_like_cpp();
        self.use_represented_gameobject_goober_preamble_with_generator_like_cpp(
            generators.item.as_ref(),
            gameobject_guid,
            gameobject_entry,
            gameobject_position,
            player_guid,
            source,
        )
        .await
    }
    pub(crate) fn use_represented_gameobject_goober_state_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        gameobject_entry: u32,
        source: wow_entities::GooberUseSource,
    ) -> bool {
        if source.allow_multi_interact {
            if source.consumable {
                let default_map_id = self.player_map_id_like_cpp();
                let (despawn_secs, map_id) = {
                    let state = self
                        .represented_gameobject_use_states
                        .entry(gameobject_guid)
                        .or_default();
                    let despawn_secs = state
                        .despawn_delay_secs
                        .unwrap_or(wow_entities::DEFAULT_GAMEOBJECT_RESPAWN_DELAY_SECS);
                    state.per_player_despawn_secs = Some(despawn_secs);
                    state.per_player_despawn_until =
                        Some(Instant::now() + Duration::from_secs(u64::from(despawn_secs)));
                    state.per_player_state_player_guid = Some(player_guid);
                    (despawn_secs, state.map_id.unwrap_or(default_map_id))
                };
                self.represented_gameobject_use_effects.push(
                    RepresentedGameObjectUseEffect::GooberDespawnForPlayer {
                        gameobject_guid,
                        player_guid,
                        despawn_secs,
                    },
                );
                if self.player_guid() == Some(player_guid)
                    && self.client_visible_guids_like_cpp.remove(&gameobject_guid)
                {
                    self.send_represented_gameobject_out_of_range_for_player_like_cpp(
                        gameobject_guid,
                        map_id,
                    );
                }
            } else {
                let state = self
                    .represented_gameobject_use_states
                    .entry(gameobject_guid)
                    .or_default();
                let respawn_secs = state
                    .despawn_delay_secs
                    .unwrap_or(wow_entities::DEFAULT_GAMEOBJECT_RESPAWN_DELAY_SECS);
                state.per_player_state_player_guid = Some(player_guid);
                state.per_player_go_state = Some(wow_entities::GoState::Active);
                state.per_player_go_state_until =
                    Some(Instant::now() + Duration::from_secs(u64::from(respawn_secs)));
                self.represented_gameobject_use_effects.push(
                    RepresentedGameObjectUseEffect::GooberSetGoStateForPlayer {
                        gameobject_guid,
                        player_guid,
                        go_state: wow_entities::GoState::Active,
                    },
                );
                if self.player_guid() == Some(player_guid) {
                    self.send_packet(&wow_packet::packets::misc::GameObjectSetStateLocal {
                        object_guid: gameobject_guid,
                        state: wow_entities::GoState::Active as u8,
                    });
                }
            }
        } else {
            let (go_state, custom_anim_progress) = {
                let state = self
                    .represented_gameobject_use_states
                    .entry(gameobject_guid)
                    .or_default();
                state.gameobject_flags |= wow_entities::GO_FLAG_IN_USE;
                state.loot_state = Some(wow_entities::LootState::Activated);
                state.loot_state_unit_guid = player_guid;
                state.goober_use_source = Some(source);
                state.despawn_at_action = source.consumable;
                let custom_anim_progress = if source.custom_anim != 0 {
                    Some(u32::from(state.go_anim_progress))
                } else {
                    state.go_state = Some(wow_entities::GoState::Active);
                    None
                };
                state.cooldown_until =
                    Some(Instant::now() + Duration::from_millis(u64::from(source.auto_close_ms)));
                (
                    custom_anim_progress
                        .is_none()
                        .then_some(wow_entities::GoState::Active),
                    custom_anim_progress,
                )
            };
            if let Some(custom_anim) = custom_anim_progress {
                use wow_packet::ServerPacket;

                let packet = wow_packet::packets::misc::GameObjectCustomAnim {
                    object_guid: gameobject_guid,
                    custom_anim,
                    play_as_despawn: false,
                };
                self.send_packet(&packet);
                let _ = self.queue_visible_gameobject_packet_for_same_map_like_cpp(
                    gameobject_guid,
                    packet.to_bytes(),
                );
            }
            self.represented_gameobject_use_effects.push(
                RepresentedGameObjectUseEffect::GooberUsed {
                    gameobject_guid,
                    user_guid: player_guid,
                    custom_anim: source.custom_anim,
                    auto_close_ms: source.auto_close_ms,
                    go_state,
                },
            );
            let _ =
                self.queue_goober_gameobject_state_refresh_for_same_map_like_cpp(gameobject_guid);
        }

        if source.spell_id != 0 {
            self.apply_represented_gameobject_post_use_spell_like_cpp(
                gameobject_guid,
                player_guid,
                gameobject_entry,
                wow_entities::GAMEOBJECT_TYPE_GOOBER,
                source.spell_id,
                false,
                if source.player_cast {
                    RepresentedGameObjectSpellCaster::User
                } else {
                    RepresentedGameObjectSpellCaster::GameObject
                },
                if source.player_cast {
                    player_guid
                } else {
                    gameobject_guid
                },
            );
        }

        true
    }
}
