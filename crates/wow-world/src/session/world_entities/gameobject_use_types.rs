//! Per-type represented gameobject use handlers.
//!
//! Moved out of the Session root under #599. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(crate) fn use_represented_gameobject_door_or_button_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        user_guid: ObjectGuid,
        restore_time_ms: u32,
    ) -> bool {
        let now = Instant::now();
        let state = self
            .represented_gameobject_use_states
            .entry(gameobject_guid)
            .or_default();
        if state
            .loot_state
            .is_some_and(|loot_state| loot_state != wow_entities::LootState::Ready)
        {
            self.represented_gameobject_use_effects.push(
                RepresentedGameObjectUseEffect::DoorOrButtonRejectedNotReady { gameobject_guid },
            );
            return false;
        }

        let current_go_state = state.go_state.unwrap_or(wow_entities::GoState::Ready);
        let next_go_state = if current_go_state == wow_entities::GoState::Ready {
            wow_entities::GoState::Active
        } else {
            wow_entities::GoState::Ready
        };
        state.prev_go_state = Some(current_go_state);
        state.go_state = Some(next_go_state);
        state.loot_state = Some(wow_entities::LootState::Activated);
        state.loot_state_unit_guid = user_guid;
        state.gameobject_flags |= wow_entities::GO_FLAG_IN_USE;
        state.cooldown_until = (restore_time_ms != 0)
            .then_some(now + Duration::from_millis(u64::from(restore_time_ms)));
        self.represented_gameobject_use_effects.push(
            RepresentedGameObjectUseEffect::DoorOrButtonUsed {
                gameobject_guid,
                user_guid,
                restore_time_ms,
                go_state: next_go_state,
            },
        );
        true
    }
    #[allow(dead_code)]
    pub(crate) fn reset_represented_gameobject_door_or_button_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
    ) -> bool {
        let Some(state) = self
            .represented_gameobject_use_states
            .get_mut(&gameobject_guid)
        else {
            return false;
        };
        if matches!(
            state.loot_state,
            Some(wow_entities::LootState::Ready | wow_entities::LootState::JustDeactivated)
        ) {
            return false;
        }

        state.gameobject_flags &= !wow_entities::GO_FLAG_IN_USE;
        let restored_go_state = state.prev_go_state.unwrap_or(wow_entities::GoState::Ready);
        state.go_state = Some(restored_go_state);
        state.loot_state = Some(wow_entities::LootState::JustDeactivated);
        state.cooldown_until = None;
        self.represented_gameobject_use_effects.push(
            RepresentedGameObjectUseEffect::DoorOrButtonReset {
                gameobject_guid,
                go_state: restored_go_state,
            },
        );
        true
    }
    #[allow(dead_code)]
    pub(crate) fn tick_represented_gameobject_door_or_button_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
    ) -> bool {
        let now = Instant::now();
        let expired = self
            .represented_gameobject_use_states
            .get(&gameobject_guid)
            .and_then(|state| state.cooldown_until)
            .is_some_and(|cooldown_until| cooldown_until <= now);
        if !expired {
            return false;
        }
        self.reset_represented_gameobject_door_or_button_like_cpp(gameobject_guid)
    }
    pub(crate) fn use_represented_gameobject_trap_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        user_guid: ObjectGuid,
        source: wow_entities::TrapUseSource,
    ) -> bool {
        let now = Instant::now();
        let state = self
            .represented_gameobject_use_states
            .entry(gameobject_guid)
            .or_default();
        if state
            .cooldown_until
            .is_some_and(|cooldown_until| cooldown_until > now)
        {
            self.represented_gameobject_use_effects
                .push(RepresentedGameObjectUseEffect::CooldownRejected { gameobject_guid });
            return false;
        }
        state.go_type = Some(wow_entities::GAMEOBJECT_TYPE_TRAP as u8);
        state.trap_use_source = Some(source);

        if source.spell_id != 0 {
            self.represented_gameobject_use_effects.push(
                RepresentedGameObjectUseEffect::CastSpell {
                    gameobject_guid,
                    player_guid: user_guid,
                    spell_id: source.spell_id,
                },
            );
        }

        let cooldown_secs = if source.cooldown_secs != 0 {
            source.cooldown_secs
        } else {
            4
        };
        state.cooldown_until =
            Some(now + Duration::from_millis(u64::from(cooldown_secs).saturating_mul(1000)));
        self.represented_gameobject_use_effects.push(
            RepresentedGameObjectUseEffect::CooldownStarted {
                gameobject_guid,
                cooldown_secs,
            },
        );

        if source.charges == 1 {
            state.loot_state = Some(wow_entities::LootState::JustDeactivated);
        }

        true
    }
    pub(crate) fn use_represented_gameobject_chair_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        player_position: Position,
        gameobject_position: Position,
        gameobject_size: f32,
        source: wow_entities::ChairUseSource,
    ) -> bool {
        let slot_count = source.chair_slots.max(1).min(5);
        let state = self
            .represented_gameobject_use_states
            .entry(gameobject_guid)
            .or_default();
        if state.chair_slots.is_empty() {
            state.chair_slots = vec![None; slot_count as usize];
        }

        let orthogonal_orientation = gameobject_position.orientation + std::f32::consts::PI * 0.5;
        let mut nearest_slot = None;
        let mut lowest_dist = f32::MAX;
        let mut nearest_position = gameobject_position;
        for slot in 0..state.chair_slots.len() {
            if state.chair_slots[slot].is_some() {
                continue;
            }

            let relative_distance =
                (gameobject_size * slot as f32) - (gameobject_size * (slot_count - 1) as f32 / 2.0);
            let candidate = Position::new(
                gameobject_position.x + relative_distance * orthogonal_orientation.cos(),
                gameobject_position.y + relative_distance * orthogonal_orientation.sin(),
                gameobject_position.z,
                gameobject_position.orientation,
            );
            let dist = player_position.distance_2d(&candidate);
            if dist <= lowest_dist {
                nearest_slot = Some(slot);
                lowest_dist = dist;
                nearest_position = candidate;
            }
        }

        let Some(slot) = nearest_slot else {
            self.represented_gameobject_use_effects.push(
                RepresentedGameObjectUseEffect::ChairNoFreeSlot {
                    gameobject_guid,
                    player_guid,
                },
            );
            return false;
        };

        state.chair_slots[slot] = Some(player_guid);
        let stand_state = 4_u32.saturating_add(source.chair_height);
        self.set_player_position_like_cpp(nearest_position);
        self.set_player_stand_state_like_cpp(Self::chair_stand_state_like_cpp(source.chair_height));
        self.represented_gameobject_use_effects
            .push(RepresentedGameObjectUseEffect::ChairUsed {
                gameobject_guid,
                player_guid,
                slot: slot as u32,
                teleport_position: nearest_position,
                stand_state,
            });
        if source.triggered_event_id != 0 {
            self.represented_gameobject_use_effects.push(
                RepresentedGameObjectUseEffect::TriggerGameEvent {
                    gameobject_guid,
                    player_guid,
                    event_id: source.triggered_event_id,
                },
            );
        }

        true
    }
    pub(crate) fn use_represented_gameobject_barber_chair_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        gameobject_position: Position,
        source: wow_entities::BarberChairUseSource,
    ) -> bool {
        self.send_packet(&wow_packet::packets::misc::EnableBarberShop {
            customization_scope: source.customization_scope.min(u32::from(u8::MAX)) as u8,
        });
        self.represented_gameobject_use_effects.push(
            RepresentedGameObjectUseEffect::BarberChairUsed {
                gameobject_guid,
                player_guid,
                customization_scope: source.customization_scope,
                teleport_position: gameobject_position,
                stand_state: 4_u32.saturating_add(source.chair_height),
                sit_anim_kit: source.sit_anim_kit,
            },
        );
        self.set_player_position_like_cpp(gameobject_position);
        self.set_player_stand_state_like_cpp(Self::chair_stand_state_like_cpp(source.chair_height));

        true
    }
    pub(crate) fn use_represented_gameobject_ui_link_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        source: wow_entities::UiLinkUseSource,
    ) -> bool {
        let interaction_type = Self::ui_link_player_interaction_type_like_cpp(source.ui_link_type);
        self.send_packet(&wow_packet::packets::misc::GameObjectInteraction {
            object_guid: gameobject_guid,
            interaction_type,
        });
        self.represented_gameobject_use_effects.push(
            RepresentedGameObjectUseEffect::UiLinkOpened {
                gameobject_guid,
                player_guid,
                ui_link_type: source.ui_link_type,
                interaction_type,
            },
        );

        true
    }
    pub(crate) fn use_represented_gameobject_spellcaster_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        gameobject_entry: u32,
        source: wow_entities::SpellcasterUseSource,
    ) -> bool {
        if source.party_only {
            let owner_guid =
                self.represented_or_canonical_gameobject_owner_guid_like_cpp(gameobject_guid);
            if !owner_guid.is_some_and(|owner_guid| {
                self.represented_player_is_same_raid_with_like_cpp(player_guid, owner_guid)
            }) {
                self.represented_gameobject_use_effects.push(
                    RepresentedGameObjectUseEffect::SpellcasterPartyOnlyRejected {
                        gameobject_guid,
                        player_guid,
                    },
                );
                return false;
            }
        }

        if !self.remove_represented_mounted_auras_by_type_like_cpp() {
            return false;
        }
        self.represented_gameobject_use_effects.push(
            RepresentedGameObjectUseEffect::RemoveMountedAuras {
                gameobject_guid,
                player_guid,
            },
        );

        let use_count = {
            let state = self
                .represented_gameobject_use_states
                .entry(gameobject_guid)
                .or_default();
            state.go_type = Some(wow_entities::GAMEOBJECT_TYPE_SPELLCASTER as u8);
            if source.charges != 0 {
                state.max_charges = Some(source.charges);
            }
            state.use_count = state.use_count.saturating_add(1);
            state.use_count
        };
        self.represented_gameobject_use_effects.push(
            RepresentedGameObjectUseEffect::GameObjectUseCountIncremented {
                gameobject_guid,
                use_count,
            },
        );

        self.apply_represented_gameobject_post_use_spell_like_cpp(
            gameobject_guid,
            player_guid,
            gameobject_entry,
            wow_entities::GAMEOBJECT_TYPE_SPELLCASTER,
            source.spell_id,
            false,
            RepresentedGameObjectSpellCaster::User,
            player_guid,
        );

        true
    }
    pub(crate) fn use_represented_gameobject_spell_focus_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        linked_trap_entry: u32,
    ) -> bool {
        if linked_trap_entry != 0 {
            self.represented_gameobject_use_effects.push(
                RepresentedGameObjectUseEffect::TriggerLinkedTrap {
                    gameobject_guid,
                    player_guid,
                    trap_entry: linked_trap_entry,
                },
            );
        }

        true
    }
    pub(crate) fn use_represented_gameobject_camera_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        source: wow_entities::CameraUseSource,
    ) -> bool {
        if source.cinematic_id != 0 {
            self.send_represented_cinematic_start_like_cpp(source.cinematic_id);
            self.represented_gameobject_use_effects.push(
                RepresentedGameObjectUseEffect::TriggerCinematic {
                    gameobject_guid,
                    player_guid,
                    cinematic_id: source.cinematic_id,
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

        true
    }
}
