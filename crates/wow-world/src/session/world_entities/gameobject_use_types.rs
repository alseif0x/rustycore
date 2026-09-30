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
        let Some(next_go_state) = wow_entities::GameObjectUseValues::borrow(
            &mut state.loot_state,
            &mut state.loot_state_unit_guid,
            &mut state.go_state,
            &mut state.prev_go_state,
            &mut state.gameobject_flags,
            &mut state.cooldown_until,
            &mut state.go_type,
            &mut state.trap_use_source,
            &mut state.chair_slots,
        ).use_door_or_button(user_guid, restore_time_ms, now) else {
            self.represented_gameobject_use_effects.push(
                RepresentedGameObjectUseEffect::DoorOrButtonRejectedNotReady { gameobject_guid },
            );
            return false;
        };
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
        let Some(restored_go_state) = wow_entities::GameObjectUseValues::borrow(
            &mut state.loot_state,
            &mut state.loot_state_unit_guid,
            &mut state.go_state,
            &mut state.prev_go_state,
            &mut state.gameobject_flags,
            &mut state.cooldown_until,
            &mut state.go_type,
            &mut state.trap_use_source,
            &mut state.chair_slots,
        ).reset_door_or_button() else {
            return false;
        };
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
        let Some(state) = self
            .represented_gameobject_use_states
            .get_mut(&gameobject_guid)
        else {
            return false;
        };
        let Some(restored_go_state) = wow_entities::GameObjectUseValues::borrow(
            &mut state.loot_state,
            &mut state.loot_state_unit_guid,
            &mut state.go_state,
            &mut state.prev_go_state,
            &mut state.gameobject_flags,
            &mut state.cooldown_until,
            &mut state.go_type,
            &mut state.trap_use_source,
            &mut state.chair_slots,
        ).tick_door_or_button(now) else {
            return false;
        };
        self.represented_gameobject_use_effects.push(
            RepresentedGameObjectUseEffect::DoorOrButtonReset {
                gameobject_guid,
                go_state: restored_go_state,
            },
        );
        true
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
        let effects = &mut self.represented_gameobject_use_effects;
        wow_entities::GameObjectUseValues::borrow(
            &mut state.loot_state,
            &mut state.loot_state_unit_guid,
            &mut state.go_state,
            &mut state.prev_go_state,
            &mut state.gameobject_flags,
            &mut state.cooldown_until,
            &mut state.go_type,
            &mut state.trap_use_source,
            &mut state.chair_slots,
        ).use_trap(source, now, |effect| match effect {
            wow_entities::TrapUseEffect::CooldownRejected => effects.push(
                RepresentedGameObjectUseEffect::CooldownRejected { gameobject_guid },
            ),
            wow_entities::TrapUseEffect::CastSpell { spell_id } => effects.push(
                RepresentedGameObjectUseEffect::CastSpell {
                    gameobject_guid,
                    player_guid: user_guid,
                    spell_id,
                },
            ),
            wow_entities::TrapUseEffect::CooldownStarted { cooldown_secs } => effects.push(
                RepresentedGameObjectUseEffect::CooldownStarted {
                    gameobject_guid,
                    cooldown_secs,
                },
            ),
        })
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
        let placement = wow_entities::GameObjectUseValues::use_chair(
            player_guid,
            player_position,
            gameobject_position,
            gameobject_size,
            source,
            || {
                let state = self
                    .represented_gameobject_use_states
                    .entry(gameobject_guid)
                    .or_default();
                wow_entities::GameObjectUseValues::borrow(
                    &mut state.loot_state,
                    &mut state.loot_state_unit_guid,
                    &mut state.go_state,
                    &mut state.prev_go_state,
                    &mut state.gameobject_flags,
                    &mut state.cooldown_until,
                    &mut state.go_type,
                    &mut state.trap_use_source,
                    &mut state.chair_slots,
                )
            },
        );
        let Some(placement) = placement else {
            self.represented_gameobject_use_effects.push(
                RepresentedGameObjectUseEffect::ChairNoFreeSlot {
                    gameobject_guid,
                    player_guid,
                },
            );
            return false;
        };
        self.set_player_position_like_cpp(placement.teleport_position());
        self.set_player_stand_state_like_cpp(wow_entities::chair_stand_state_like_cpp(
            source.chair_height,
        ));
        self.represented_gameobject_use_effects
            .push(RepresentedGameObjectUseEffect::ChairUsed {
                gameobject_guid,
                player_guid,
                slot: placement.slot(),
                teleport_position: placement.teleport_position(),
                stand_state: placement.raw_stand_state(),
            });
        if let Some(event_id) = placement.trigger_event() {
            self.represented_gameobject_use_effects.push(
                RepresentedGameObjectUseEffect::TriggerGameEvent {
                    gameobject_guid,
                    player_guid,
                    event_id,
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
        self.set_player_stand_state_like_cpp(wow_entities::chair_stand_state_like_cpp(
            source.chair_height,
        ));

        true
    }
    pub(crate) fn use_represented_gameobject_ui_link_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        source: wow_entities::UiLinkUseSource,
    ) -> bool {
        let interaction_type =
            wow_entities::ui_link_player_interaction_type_like_cpp(source.ui_link_type);
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
