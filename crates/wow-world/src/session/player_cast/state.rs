//! Canonical Player/Unit cast state adapters and existing driver entry points.

use super::*;

impl WorldSession {
    /// C++ `Player::SendLoot` calls `InterruptNonMeleeSpells(false)`, i.e.
    /// `withDelayed = false`, `spell_id = 0`, `withInstant = true`.
    ///
    /// That single C++ call reaches the caster's current-spell slots, so the
    /// represented equivalent must interrupt both the prepared cast execution
    /// state and the canonical Unit slots, exactly as the far-teleport path
    /// does. Reaching only one of the two would let a canonical generic or
    /// channeled spell survive looting.
    pub(crate) fn interrupt_non_melee_spell_cast_for_loot_like_cpp(&mut self) -> bool {
        let session_cast_interrupted = self.interrupt_player_cast_like_cpp(None);
        let canonical_spells_interrupted = self
            .mutate_canonical_player_like_cpp(|player| {
                let unit = player.unit_mut();
                if !unit.is_non_melee_spell_cast_like_cpp(false, false, false, true) {
                    return false;
                }
                !unit.interrupt_non_melee_spells(None, false, true).is_empty()
            })
            .unwrap_or(false);

        session_cast_interrupted || canonical_spells_interrupted
    }

    pub(in crate::session) fn with_cast_execution_like_cpp<R>(
        &self,
        f: impl FnOnce(&wow_entities::CastExecutionStateLikeCpp) -> R,
    ) -> Option<R> {
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            return Some(f(&wow_entities::CastExecutionStateLikeCpp {
                active: self.active_spell_cast.clone(),
                last_cast_time: self.last_spell_cast_time,
                last_cast_time_per_spell: self.last_spell_cast_time_per_spell.clone(),
            }));
        }
        self.with_owned_player_like_cpp(|player| f(&player.unit().subsystems().spells.execution))
    }

    pub(crate) fn mutate_cast_execution_like_cpp<R>(
        &mut self,
        f: impl FnOnce(&mut wow_entities::CastExecutionStateLikeCpp) -> R,
    ) -> Option<R> {
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            let mut state = wow_entities::CastExecutionStateLikeCpp {
                active: self.active_spell_cast.clone(),
                last_cast_time: self.last_spell_cast_time,
                last_cast_time_per_spell: self.last_spell_cast_time_per_spell.clone(),
            };
            let result = f(&mut state);
            self.active_spell_cast = state.active;
            self.last_spell_cast_time = state.last_cast_time;
            self.last_spell_cast_time_per_spell = state.last_cast_time_per_spell;
            return Some(result);
        }
        self.with_owned_player_mut_like_cpp(|player| {
            f(&mut player.unit_mut().subsystems_mut().spells.execution)
        })
    }

    #[cfg(test)]
    pub(crate) fn active_spell_cast_snapshot_like_cpp(&self) -> Option<SpellCastState> {
        self.with_cast_execution_like_cpp(|state| state.active.clone())
            .flatten()
    }

    pub(crate) fn set_active_spell_cast_like_cpp(&mut self, cast: Option<SpellCastState>) -> bool {
        self.mutate_cast_execution_like_cpp(|state| state.active = cast)
            .is_some()
    }

    pub(crate) fn last_spell_cast_time_like_cpp(&self) -> Option<Option<Instant>> {
        self.with_cast_execution_like_cpp(|state| state.last_cast_time)
    }

    pub(crate) fn spell_last_cast_time_like_cpp(&self, spell_id: i32) -> Option<Option<Instant>> {
        self.with_cast_execution_like_cpp(|state| {
            state.last_cast_time_per_spell.get(&spell_id).copied()
        })
    }

    pub(crate) fn cancel_pending_spell_cast_request_like_cpp(&mut self) -> bool {
        let Some(request) = self
            .mutate_pending_spell_cast_like_cpp(Option::take)
            .flatten()
        else {
            return false;
        };

        self.send_packet(&wow_packet::packets::spell::CastFailed {
            cast_id: request.cast_id,
            spell_id: request.spell_id,
            visual: Default::default(),
            reason: SPELL_FAILED_DONT_REPORT_LIKE_CPP,
            fail_arg1: 0,
            fail_arg2: 0,
        });
        true
    }

    pub(in crate::session) fn pending_spell_cast_snapshot_like_cpp(
        &self,
    ) -> Option<Option<RepresentedPendingSpellCastRequestLikeCpp>> {
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            return Some(self.represented_pending_spell_cast_request_like_cpp.clone());
        }
        self.with_owned_player_like_cpp(|player| player.gameplay_state().pending_spell_cast.clone())
    }

    #[cfg(test)]
    pub(crate) fn pending_spell_cast_for_test_like_cpp(
        &self,
    ) -> Option<RepresentedPendingSpellCastRequestLikeCpp> {
        self.pending_spell_cast_snapshot_like_cpp().flatten()
    }

    pub(in crate::session) fn mutate_pending_spell_cast_like_cpp<R>(
        &mut self,
        f: impl FnOnce(&mut Option<RepresentedPendingSpellCastRequestLikeCpp>) -> R,
    ) -> Option<R> {
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            return Some(f(&mut self.represented_pending_spell_cast_request_like_cpp));
        }
        self.with_owned_player_mut_like_cpp(|player| {
            f(&mut player.gameplay_state_mut().pending_spell_cast)
        })
    }

    pub(crate) fn remaining_global_cooldown_ms_like_cpp(
        &self,
        spell_info: &wow_data::SpellInfo,
    ) -> Option<u32> {
        self.with_cast_execution_like_cpp(|state| {
            state.remaining_global_cooldown_ms(spell_info.cooldown_ms)
        })
    }

    pub(crate) fn remaining_active_spell_cast_ms_like_cpp(&self) -> Option<u32> {
        self.with_cast_execution_like_cpp(
            wow_entities::CastExecutionStateLikeCpp::remaining_cast_ms,
        )
    }

    pub(crate) fn request_represented_spell_cast_like_cpp(
        &mut self,
        request: RepresentedPendingSpellCastRequestLikeCpp,
    ) {
        self.cancel_pending_spell_cast_request_like_cpp();
        let _ = self.mutate_pending_spell_cast_like_cpp(|pending| *pending = Some(request));
    }

    pub(crate) async fn tick_pending_spell_cast_request_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        creature_spawn_catalogs: &CreatureSpawnCatalogsLikeCpp,
    ) {
        let Some(request) = self.pending_spell_cast_snapshot_like_cpp().flatten() else {
            return;
        };
        // C++ `Player::CanExecutePendingSpellCastRequest` cancels the request
        // when the casting unit is missing, is no longer in world, or is no
        // longer `GetUnitBeingMoved()`. A detached or replaced handle must not
        // execute a queued request against the current player.
        let casting_unit_is_current = Some(request.casting_unit_guid) == self.player_guid()
            && self
                .player_moved_unit_guid_like_cpp()
                .is_none_or(|moved| moved == request.casting_unit_guid)
            && self
                .player_handle_like_cpp
                .is_none_or(|handle| handle.guid() == request.casting_unit_guid);
        if !casting_unit_is_current {
            self.cancel_pending_spell_cast_request_like_cpp();
            return;
        }
        let Some(spell_info) = self
            .spell_store()
            .and_then(|store| store.get(request.spell_id))
            .cloned()
        else {
            self.cancel_pending_spell_cast_request_like_cpp();
            return;
        };

        if self.remaining_global_cooldown_ms_like_cpp(&spell_info) != Some(0) {
            return;
        }
        if self.remaining_active_spell_cast_ms_like_cpp() != Some(0) {
            return;
        }

        let Some(request) = self
            .mutate_pending_spell_cast_like_cpp(|pending| {
                if pending.as_ref().is_some_and(|current| {
                    current.cast_id == request.cast_id
                        && current.spell_id == request.spell_id
                        && current.casting_unit_guid == request.casting_unit_guid
                }) {
                    pending.take()
                } else {
                    None
                }
            })
            .flatten()
        else {
            return;
        };
        if crate::player_cast::prepare(self, request) {
            self.tick_active_spell_cast_with_generator_like_cpp(
                item_guid_generator,
                creature_spawn_catalogs,
            )
            .await;
        }
    }

    #[cfg(test)]
    pub(crate) async fn tick_pending_spell_cast_request_like_cpp(&mut self) {
        let generators = self.id_generators_for_test_like_cpp();
        let creature_spawn_catalogs = self.creature_spawn_catalogs_for_test_like_cpp();
        self.tick_pending_spell_cast_request_with_generator_like_cpp(
            generators.item.as_ref(),
            &creature_spawn_catalogs,
        )
        .await;
    }

    /// Called every ~100ms. Checks if an in-progress spell cast has completed.
    ///
    /// If `active_spell_cast` is set and its cast time has elapsed, this method
    /// executes the spell (applies effects, cooldowns, etc.) and clears the cast state.
    pub(crate) async fn tick_active_spell_cast_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        creature_spawn_catalogs: &CreatureSpawnCatalogsLikeCpp,
    ) {
        let Some(cast_state) = self.take_ready_player_cast_like_cpp() else {
            return;
        };

        let spell_id = cast_state.spell_id;
        let target = cast_state.target_guid;
        let target_data = crate::spell_cast_adapter::present_targets(cast_state.target_data);
        let cast_id = cast_state.cast_id;
        let spell_visual = crate::spell_cast_adapter::present_visual(cast_state.spell_visual);
        let metadata = cast_state.metadata;

        // The owner guard is released before effect execution and failure publication.
        if let Err(e) = self
            .execute_spell_with_visual_and_target_data_with_metadata_and_generator_like_cpp(
                item_guid_generator,
                creature_spawn_catalogs,
                spell_id,
                target,
                cast_id,
                spell_visual.clone(),
                target_data,
                metadata,
            )
            .await
        {
            warn!(account = self.account_id, "Spell execution failed: {}", e);
            // Send CastFailed so client cancels cast animation
            use wow_packet::packets::spell::CastFailed;
            self.send_packet(&CastFailed {
                cast_id,
                spell_id,
                visual: spell_visual,
                reason: 2, // SpellCastResult::NotKnown
                fail_arg1: 0,
                fail_arg2: 0,
            });
        }
    }

    #[cfg(test)]
    pub(crate) async fn tick_active_spell_cast(&mut self) {
        let generators = self.id_generators_for_test_like_cpp();
        let creature_spawn_catalogs = self.creature_spawn_catalogs_for_test_like_cpp();
        self.tick_active_spell_cast_with_generator_like_cpp(
            generators.item.as_ref(),
            &creature_spawn_catalogs,
        )
        .await;
    }
}
