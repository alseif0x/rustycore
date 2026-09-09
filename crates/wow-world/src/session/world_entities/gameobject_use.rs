//! Represented gameobject use and interaction operations.
//!
//! Moved out of the Session root under #599. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(crate) fn add_use_and_get_canonical_gameobject_use_count_like_cpp(
        &mut self,
        guid: ObjectGuid,
    ) -> Option<u32> {
        self.mutate_canonical_gameobject_by_guid_like_cpp(guid, |gameobject| {
            gameobject.add_use_like_cpp();
            gameobject.use_times()
        })
    }
    pub(crate) fn record_represented_gameobject_interact_radius_override_like_cpp(
        &mut self,
        guid: ObjectGuid,
        interact_radius_override: u32,
    ) {
        self.represented_gameobject_use_states
            .entry(guid)
            .or_default()
            .interact_radius_override =
            (interact_radius_override != 0).then_some(interact_radius_override);
    }
    pub(crate) fn record_represented_gameobject_icon_interaction_like_cpp(
        &mut self,
        guid: ObjectGuid,
        allows_interaction: bool,
    ) {
        self.represented_gameobject_use_states
            .entry(guid)
            .or_default()
            .icon_name_allows_interaction_like_cpp = Some(allows_interaction);
    }
    pub(crate) fn represented_gameobject_can_interact_with_like_cpp(
        &self,
        guid: ObjectGuid,
        interaction_distance: f32,
    ) -> Option<RepresentedGameObjectAccessLikeCpp> {
        let access = self.canonical_gameobject_access_like_cpp(guid)?;
        let player_position = self.player_position_like_cpp()?;
        if access
            .position
            .is_within_dist(&player_position, interaction_distance)
        {
            Some(access)
        } else {
            None
        }
    }
    pub(crate) fn represented_gameobject_use_allowed_by_mover_like_cpp(
        &self,
        gameobject_usable_mounted: bool,
    ) -> bool {
        let Some(player_guid) = self.player_guid() else {
            return false;
        };
        if self.player_moved_unit_guid_like_cpp() == Some(player_guid) {
            return true;
        }

        self.player_vehicle_seat_state_like_cpp()
            .and_then(|(flags, _)| flags)
            .is_some()
            || self.resolved_player_mounted_like_cpp() == Some(true)
            || gameobject_usable_mounted
    }
    pub(crate) fn record_represented_gameobject_report_use_ai_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
    ) -> bool {
        let handled = self
            .represented_gameobject_use_states
            .get(&gameobject_guid)
            .map(|state| state.report_use_ai_returns_true)
            .unwrap_or(false);
        self.represented_gameobject_use_effects
            .push(RepresentedGameObjectUseEffect::ReportUseAi {
                gameobject_guid,
                player_guid,
                handled,
            });
        handled
    }
    pub(crate) fn apply_represented_gameobject_player_use_preamble_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        gameobject_usable_mounted: bool,
        no_damage_immune: bool,
    ) -> bool {
        let Some((player_unit_flags, _, _)) = self.player_unit_presentation_snapshot_like_cpp()
        else {
            return false;
        };
        if no_damage_immune && player_unit_flags.contains(UnitFlags::IMMUNE) {
            self.represented_gameobject_use_effects.push(
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
            self.represented_gameobject_use_effects.push(
                RepresentedGameObjectUseEffect::RemoveMountedAuras {
                    gameobject_guid,
                    player_guid,
                },
            );
        }

        self.represented_gameobject_use_effects.push(
            RepresentedGameObjectUseEffect::ClearPlayerTalkMenus {
                gameobject_guid,
                player_guid,
            },
        );

        let handled = self
            .represented_gameobject_use_states
            .get(&gameobject_guid)
            .map(|state| state.gossip_hello_ai_returns_true)
            .unwrap_or(false);
        self.represented_gameobject_use_effects.push(
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

        let spell_lookup_difficulty_id =
            self.represented_gameobject_spell_lookup_difficulty_id_like_cpp();
        let spell_info_missing = self
            .spell_store()
            .is_some_and(|store| store.get(spell_id as i32).is_none());

        self.represented_gameobject_use_effects.push(
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
            self.represented_gameobject_use_effects.push(
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

        self.represented_gameobject_use_effects.push(
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
            .represented_gameobject_use_states
            .get(&gameobject_guid)
            .and_then(|state| state.linked_trap_guid);
        if source.linked_trap_entry != 0 {
            if let Some(trap_guid) = linked_trap_guid {
                self.despawn_represented_linked_trap_by_guid_like_cpp(trap_guid);
            }
            self.represented_gameobject_use_effects.push(
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
                .represented_gameobject_use_states
                .entry(gameobject_guid)
                .or_default();
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
                self.represented_gameobject_use_effects.push(
                    RepresentedGameObjectUseEffect::GooberUniqueUserSpell {
                        gameobject_guid,
                        player_guid,
                        spell_id: source.spell_id,
                    },
                );
            }
        }

        if let Some(state) = self.represented_gameobject_use_states.get(&gameobject_guid) {
            self.represented_gameobject_use_effects.push(
                RepresentedGameObjectUseEffect::GooberCleared {
                    gameobject_guid,
                    loot_state: state
                        .loot_state
                        .unwrap_or(wow_entities::LootState::NotReady),
                    go_state: state.go_state,
                },
            );
        }

        true
    }
}
