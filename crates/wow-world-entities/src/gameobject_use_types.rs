use std::time::{Duration, Instant};

use wow_core::ObjectGuid;

use crate::{RepresentedGameObjectUseEffect, WorldEntitiesState};

impl WorldEntitiesState {
    pub fn use_represented_gameobject_door_or_button_like_cpp(
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
    pub fn reset_represented_gameobject_door_or_button_like_cpp(
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
    pub fn tick_represented_gameobject_door_or_button_like_cpp(
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

    pub fn use_represented_gameobject_trap_like_cpp(
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

    pub fn use_represented_gameobject_ui_link_like_cpp(
        &mut self,
        hub: &mut wow_world_core::session::HubMut<'_>,
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        source: wow_entities::UiLinkUseSource,
    ) -> bool {
        let interaction_type =
            wow_entities::ui_link_player_interaction_type_like_cpp(source.ui_link_type);
        hub.core
            .send_packet(&wow_packet::packets::misc::GameObjectInteraction {
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

    pub fn use_represented_gameobject_spell_focus_like_cpp(
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
}
