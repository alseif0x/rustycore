//! Represented gameobject use and interaction operations.
//!
//! Moved out of the Session root under #599. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(crate) fn represented_gameobject_can_interact_with_like_cpp(
        &self,
        guid: ObjectGuid,
        interaction_distance: f32,
    ) -> Option<RepresentedGameObjectAccessLikeCpp> {
        let (state, hub) = crate::session::split_world_entities_ref(self);
        state.represented_gameobject_can_interact_with_like_cpp(hub, guid, interaction_distance)
    }
    pub(crate) fn represented_gameobject_use_allowed_by_mover_like_cpp(
        &self,
        gameobject_usable_mounted: bool,
    ) -> bool {
        let (state, hub) = crate::session::split_world_entities_ref(self);
        state.represented_gameobject_use_allowed_by_mover_like_cpp(hub, gameobject_usable_mounted)
    }
    pub(crate) fn apply_represented_gameobject_player_use_preamble_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        gameobject_usable_mounted: bool,
        no_damage_immune: bool,
    ) -> bool {
        let Some((player_unit_flags, _, _)) =
            crate::session::hub_ref(self).player_unit_presentation_snapshot_like_cpp()
        else {
            return false;
        };
        if no_damage_immune && player_unit_flags.contains(UnitFlags::IMMUNE) {
            self.world_entities
                .record_represented_gameobject_use_effect_like_cpp(
                    RepresentedGameObjectUseEffect::UseRejectedNoDamageImmune {
                        gameobject_guid,
                        player_guid,
                    },
                );
            return false;
        }

        if !gameobject_usable_mounted {
            if !self.remove_represented_mounted_auras_by_type_like_cpp() {
                return false;
            }
            self.world_entities
                .record_represented_gameobject_use_effect_like_cpp(
                    RepresentedGameObjectUseEffect::RemoveMountedAuras {
                        gameobject_guid,
                        player_guid,
                    },
                );
        }

        self.world_entities
            .record_represented_gameobject_use_effect_like_cpp(
                RepresentedGameObjectUseEffect::ClearPlayerTalkMenus {
                    gameobject_guid,
                    player_guid,
                },
            );

        let handled = self
            .world_entities
            .represented_gameobject_use_state_like_cpp(gameobject_guid)
            .map(|state| state.gossip_hello_ai_returns_true)
            .unwrap_or(false);
        self.world_entities
            .record_represented_gameobject_use_effect_like_cpp(
                RepresentedGameObjectUseEffect::GossipHelloAi {
                    gameobject_guid,
                    player_guid,
                    handled,
                },
            );
        !handled
    }
    pub(crate) fn apply_represented_gameobject_post_use_spell_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        gameobject_entry: u32,
        go_type: u32,
        spell_id: u32,
        triggered: bool,
        caster: RepresentedGameObjectSpellCaster,
        caster_guid: ObjectGuid,
    ) -> bool {
        self.apply_represented_gameobject_post_use_spell_like_cpp_internal(
            gameobject_guid,
            player_guid,
            gameobject_entry,
            go_type,
            spell_id,
            triggered,
            caster,
            caster_guid,
            true,
        )
    }
    pub(in crate::session) fn apply_represented_gameobject_post_use_spell_like_cpp_internal(
        &mut self,
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        gameobject_entry: u32,
        go_type: u32,
        spell_id: u32,
        triggered: bool,
        caster: RepresentedGameObjectSpellCaster,
        caster_guid: ObjectGuid,
        apply_new_flag_taken_state: bool,
    ) -> bool {
        if spell_id == 0 {
            return false;
        }

        let spell_lookup_difficulty_id = {
            let (s, h) = crate::session::split_world_entities_ref(self);
            s.represented_gameobject_spell_lookup_difficulty_id_like_cpp(h)
        };
        let spell_info_missing = self
            .spell_store()
            .is_some_and(|store| store.get(spell_id as i32).is_none());

        self.world_entities
            .record_represented_gameobject_use_effect_like_cpp(
                RepresentedGameObjectUseEffect::OutdoorPvpCustomSpellRequested {
                    gameobject_guid,
                    player_guid,
                    gameobject_entry,
                    spell_id,
                    go_type,
                    spell_lookup_difficulty_id,
                    spell_info_missing,
                },
            );

        if spell_info_missing {
            self.world_entities
                .record_represented_gameobject_use_effect_like_cpp(
                    RepresentedGameObjectUseEffect::GameObjectPostUseSpellMissing {
                        gameobject_guid,
                        player_guid,
                        gameobject_entry,
                        spell_id,
                        go_type,
                        spell_lookup_difficulty_id,
                    },
                );
            return false;
        }

        self.world_entities
            .record_represented_gameobject_use_effect_like_cpp(
                RepresentedGameObjectUseEffect::GameObjectPostUseSpellCast {
                    gameobject_guid,
                    target_guid: player_guid,
                    caster_guid,
                    spell_id,
                    triggered,
                    caster,
                    spell_lookup_difficulty_id,
                },
            );

        if apply_new_flag_taken_state
            && go_type == wow_entities::GAMEOBJECT_TYPE_NEW_FLAG
            && caster == RepresentedGameObjectSpellCaster::GameObject
        {
            self.apply_represented_new_flag_state_command_like_cpp(
                gameobject_guid,
                Some(player_guid),
                RepresentedNewFlagStateRequest::Taken,
                0,
            );
        }
        true
    }
    #[allow(dead_code)]
    pub(crate) fn apply_represented_gameobject_goober_just_deactivated_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        source: wow_entities::GooberUseSource,
    ) -> bool {
        let linked_trap_guid = self
            .world_entities
            .represented_gameobject_use_state_like_cpp(gameobject_guid)
            .and_then(|state| state.linked_trap_guid);
        if source.linked_trap_entry != 0 {
            if let Some(trap_guid) = linked_trap_guid {
                self.despawn_represented_linked_trap_by_guid_like_cpp(trap_guid);
            }
            self.world_entities
                .record_represented_gameobject_use_effect_like_cpp(
                    RepresentedGameObjectUseEffect::GooberLinkedTrapDespawn {
                        gameobject_guid,
                        trap_entry: source.linked_trap_entry,
                    },
                );
        }

        let mut unique_users = Vec::new();
        let mut send_despawn_at_action = false;
        {
            let state = self
                .world_entities
                .ensure_represented_gameobject_use_state_like_cpp(gameobject_guid);
            if source.spell_id != 0 {
                unique_users = std::mem::take(&mut state.unique_users);
            } else {
                state.unique_users.clear();
            }

            state.loot_state_unit_guid = wow_core::ObjectGuid::EMPTY;
            state.gameobject_flags &= !wow_entities::GO_FLAG_IN_USE;
            state.cooldown_until = None;
            state.goober_use_source = None;
            if source.lock_id != 0 || source.auto_close_ms != 0 {
                state.go_state = Some(wow_entities::GoState::Ready);
            }
            state.loot_state = if source.consumable {
                Some(wow_entities::LootState::NotReady)
            } else {
                Some(wow_entities::LootState::Ready)
            };
            let has_represented_owner_or_summon = state.owner_guid.is_some();
            if source.consumable && !has_represented_owner_or_summon {
                send_despawn_at_action = true;
            }
        }

        if send_despawn_at_action {
            self.send_represented_gameobject_despawn_to_visible_set_like_cpp(gameobject_guid);
        }

        if source.spell_id != 0 {
            for player_guid in unique_users {
                self.world_entities
                    .record_represented_gameobject_use_effect_like_cpp(
                        RepresentedGameObjectUseEffect::GooberUniqueUserSpell {
                            gameobject_guid,
                            player_guid,
                            spell_id: source.spell_id,
                        },
                    );
            }
        }

        let cleared_effect = self
            .world_entities
            .represented_gameobject_use_state_like_cpp(gameobject_guid)
            .map(|state| RepresentedGameObjectUseEffect::GooberCleared {
                gameobject_guid,
                loot_state: state
                    .loot_state
                    .unwrap_or(wow_entities::LootState::NotReady),
                go_state: state.go_state,
            });
        if let Some(effect) = cleared_effect {
            self.world_entities
                .record_represented_gameobject_use_effect_like_cpp(effect);
        }

        true
    }
}
