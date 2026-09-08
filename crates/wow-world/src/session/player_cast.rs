//! Canonical state, catalog and transport adapter for Player cast requests.

use super::*;
use crate::player_cast::Runtime;
use wow_entities::{SpellCastState, SpellCastVisualLikeCpp};
use wire::{PlayerCastPublicationPhaseLikeCpp, UnrepresentedPlayerCastPublicationLikeCpp};
use wow_packet::packets::spell::{CastFailed, SpellPreparePkt, SpellStartPkt};
mod checks;
mod identity;
mod power;
mod publication;
mod state;
pub(in crate::session) mod wire;

impl WorldSession {
    pub(crate) fn cancel_client_cast_request_like_cpp(&mut self, spell_id: Option<i32>) {
        let Some((had_active, cast)) = self.mutate_cast_execution_like_cpp(|state| {
            (
                state.active.is_some(),
                state.take_interrupted_cast(spell_id),
            )
        }) else {
            return;
        };
        if let Some(cast) = cast {
            self.publish_player_cast_interruption_like_cpp(cast);
        }
        if had_active {
            self.cancel_pending_spell_cast_request_like_cpp();
        }
    }

    pub(crate) fn interrupt_player_cast_like_cpp(&mut self, spell_id: Option<i32>) -> bool {
        let Some(cast) = self
            .mutate_cast_execution_like_cpp(|state| state.take_interrupted_cast(spell_id))
            .flatten()
        else {
            return false;
        };
        self.publish_player_cast_interruption_like_cpp(cast);
        true
    }

    pub(crate) fn take_ready_player_cast_like_cpp(&mut self) -> Option<SpellCastState> {
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            return self
                .mutate_cast_execution_like_cpp(
                    wow_entities::CastExecutionStateLikeCpp::take_ready_cast,
                )
                .flatten();
        }
        let handle = self.player_handle_like_cpp?;
        if Some(handle.guid()) != self.player_guid() {
            return None;
        }
        let mut manager = self.canonical_map_manager.as_ref()?.lock().ok()?;
        let revision = manager
            .player_active_residence_revision_like_cpp(handle)
            .map(|(_, revision)| revision);
        manager
            .with_player_mut_like_cpp(handle, |player| {
                let state = &mut player.unit_mut().subsystems_mut().spells.execution;
                if state.active.as_ref().is_some_and(|cast| {
                    cast.metadata.client_cast_id.is_some()
                        && cast.metadata.prepared_residence_revision != revision
                }) {
                    state.active = None;
                    return None;
                }
                state.take_ready_cast()
            })
            .flatten()
    }
}

impl Runtime for WorldSession {
    fn spell(&self, id: i32) -> Option<wow_data::SpellInfo> {
        self.spell_store().and_then(|store| store.get(id)).cloned()
    }

    fn known(&self, id: i32) -> bool {
        self.known_spells_like_cpp().contains(&id)
            || self
                .spell_store()
                .is_some_and(|store| store.has_attribute8_like_cpp(id, 0x0400_0000))
    }

    fn passive(&self, spell: i32) -> bool {
        self.spell_store()
            .is_some_and(|store| store.is_passive_like_cpp(spell))
    }

    fn resolve_override(&self, original: &wow_data::SpellInfo) -> wow_data::SpellInfo {
        self.represented_cast_spell_info_like_cpp(original)
    }

    fn remaining(&self, spell: &wow_data::SpellInfo) -> Option<(u32, u32)> {
        self.remaining_global_cooldown_ms_like_cpp(spell)
            .zip(self.remaining_active_spell_cast_ms_like_cpp())
    }

    fn replace_pending(&mut self, request: RepresentedPendingSpellCastRequestLikeCpp) {
        self.request_represented_spell_cast_like_cpp(request);
    }

    fn failure(&mut self, id: ObjectGuid, spell: i32, visual: SpellCastVisualLikeCpp, reason: i32) {
        self.send_packet(&CastFailed {
            cast_id: id,
            spell_id: spell,
            visual: crate::spell_cast_adapter::present_visual(visual),
            reason,
            fail_arg1: 0,
            fail_arg2: 0,
        });
    }

    fn allocate(&self, spell: i32) -> Option<(ObjectGuid, Option<u64>)> {
        self.allocate_player_cast_identity_like_cpp(spell)
    }

    fn visual(&self, spell: &wow_data::SpellInfo) -> Option<SpellCastVisualLikeCpp> {
        let config = &self.legacy_creature_aggro_config_like_cpp;
        let Some(store) = config.spell_x_spell_visual_store.as_ref() else {
            #[cfg(test)]
            if self.player_handle_like_cpp.is_none() {
                return Some(SpellCastVisualLikeCpp::default());
            }
            return None;
        };
        let spell_id = u32::try_from(spell.spell_id).ok()?;
        for difficulty in creature_ai_spell_difficulty_chain_like_cpp(
            self.current_map_difficulty_id_like_cpp(),
            config,
        ) {
            let mut rows: Vec<_> = store
                .entries_like_cpp()
                .filter(|row| row.spell_id == spell_id && row.difficulty_id == difficulty)
                .collect();
            if rows.is_empty() {
                continue;
            }
            // SpellMgr inserts ascending DB2 IDs at lower_bound ordered by
            // descending CasterPlayerConditionID; equal-condition IDs reverse.
            rows.sort_by_key(|row| std::cmp::Reverse((row.caster_player_condition_id, row.id)));
            for row in rows {
                if !self
                    .represented_meets_player_condition_id_like_cpp(row.caster_player_condition_id)
                {
                    continue;
                }
                if row.caster_unit_condition_id != 0 {
                    // Rejected, not defaulted: visual zero is not a silent
                    // substitute for an unevaluated `CasterUnitConditionID`.
                    tracing::warn!(
                        spell_id,
                        condition = row.caster_unit_condition_id,
                        reason = UnrepresentedPlayerCastPublicationLikeCpp::VisualUnitCondition
                            .reason_like_cpp(),
                        "Player cast visual requires unrepresented UnitCondition evaluation"
                    );
                    return None;
                }
                return Some(SpellCastVisualLikeCpp {
                    spell_visual_id: row.id,
                    script_visual_id: 0,
                });
            }
            return Some(SpellCastVisualLikeCpp::default());
        }
        Some(SpellCastVisualLikeCpp::default())
    }

    fn publication_supported(
        &mut self,
        spell: &wow_data::SpellInfo,
        cast: ObjectGuid,
        visual: &SpellCastVisualLikeCpp,
        metadata: &SpellCastMetadata,
    ) -> bool {
        let Some(unrepresented) =
            self.player_cast_unrepresented_publication_like_cpp(spell, metadata)
        else {
            return true;
        };
        // The rejection precedes allocation and the SpellPrepare mapping, so no
        // server cast identity is consumed for a cast that cannot publish a
        // faithful Start/Go pair.
        self.send_packet(&CastFailed {
            cast_id: cast,
            spell_id: spell.spell_id,
            visual: crate::spell_cast_adapter::present_visual(visual.clone()),
            reason: SpellCastResult::Error as i32,
            fail_arg1: 0,
            fail_arg2: 0,
        });
        warn!(
            account = self.account_id,
            spell_id = spell.spell_id,
            reason = unrepresented.reason_like_cpp(),
            "Rejecting represented Player cast because its C++ publication payload is unrepresented"
        );
        false
    }

    fn prepare_mapping(&mut self, client: ObjectGuid, server: ObjectGuid) {
        self.send_packet(&SpellPreparePkt {
            client_cast_id: client,
            server_cast_id: server,
        });
    }

    fn disabled(&self, spell: i32) -> bool {
        self.is_spell_disabled_for_player_like_cpp(spell)
    }

    fn on_cooldown(&self, spell: &wow_data::SpellInfo) -> Option<bool> {
        if spell.recovery_time_ms == 0 {
            return Some(false);
        }
        Some(
            self.spell_last_cast_time_like_cpp(spell.spell_id)?
                .is_some_and(|at| at.elapsed().as_millis() < u128::from(spell.recovery_time_ms)),
        )
    }

    fn check_power(
        &mut self,
        spell: &wow_data::SpellInfo,
        cast: ObjectGuid,
        visual: &SpellCastVisualLikeCpp,
    ) -> bool {
        self.check_spell_power_like_cpp(
            spell,
            cast,
            spell.spell_id,
            &crate::spell_cast_adapter::present_visual(visual.clone()),
        )
    }

    fn check_preconditions(
        &mut self,
        spell: &wow_data::SpellInfo,
        cast: ObjectGuid,
        visual: &SpellCastVisualLikeCpp,
        metadata: SpellCastMetadata,
    ) -> bool {
        match self.check_represented_cast_preparation_like_cpp(
            spell,
            cast,
            &crate::spell_cast_adapter::present_visual(visual.clone()),
            metadata,
        ) {
            Ok(Some(_)) => true,
            Ok(None) => false,
            Err(reason) => {
                tracing::warn!(
                    spell_id = spell.spell_id,
                    reason,
                    "Cast preparation lost canonical inputs"
                );
                false
            }
        }
    }

    fn install(&mut self, cast: SpellCastState) -> bool {
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none()
            && cast.metadata.prepared_residence_revision.is_none()
        {
            return self.set_active_spell_cast_like_cpp(Some(cast));
        }
        let (Some(handle), Some(manager)) = (
            self.player_handle_like_cpp,
            self.canonical_map_manager.as_ref(),
        ) else {
            return false;
        };
        let Ok(mut manager) = manager.lock() else {
            return false;
        };
        let Some((_, revision)) = manager.player_active_residence_revision_like_cpp(handle) else {
            return false;
        };
        if Some(revision) != cast.metadata.prepared_residence_revision {
            return false;
        }
        manager
            .with_player_mut_like_cpp(handle, |player| {
                player.unit_mut().subsystems_mut().spells.execution.active = Some(cast);
            })
            .is_some()
    }

    fn start(&mut self, cast: &SpellCastState, spell: &wow_data::SpellInfo) {
        let Some(caster) = self.player_guid() else {
            return;
        };
        // C++ `SendSpellStart` samples power before the debit; `m_timer != 0`
        // is the timed-cast gate for the immunity section.
        let phase = PlayerCastPublicationPhaseLikeCpp::Start {
            timed: cast.cast_time_ms != 0,
        };
        let cast_data = self.player_cast_wire_data_for_phase_like_cpp(spell, phase);
        let cast_flags = self.player_cast_flags_like_cpp(spell, &cast_data, phase);
        let packet = SpellStartPkt {
            cast_data,
            caster,
            cast_id: cast.cast_id,
            original_cast_id: ObjectGuid::EMPTY,
            spell_id: cast.spell_id,
            visual: crate::spell_cast_adapter::present_visual(cast.spell_visual.clone()),
            cast_flags,
            cast_flags_ex: cast.metadata.cast_flags_ex,
            cast_time_ms: cast.cast_time_ms,
            target: crate::spell_cast_adapter::present_targets(cast.target_data.clone()),
        };
        self.publish_player_cast_frame_like_cpp(
            cast.metadata,
            wow_packet::ServerPacket::to_bytes(&packet),
        );
        if spell.cooldown_ms != 0 {
            #[cfg(test)]
            if self.player_handle_like_cpp.is_none() {
                let _ = self.mutate_cast_execution_like_cpp(|state| {
                    state.last_cast_time = Some(cast.cast_start_time);
                });
                return;
            }
            let (Some(handle), Some(manager)) = (
                self.player_handle_like_cpp,
                self.canonical_map_manager.as_ref(),
            ) else {
                return;
            };
            let Ok(mut manager) = manager.lock() else {
                return;
            };
            let Some((_, revision)) = manager.player_active_residence_revision_like_cpp(handle)
            else {
                return;
            };
            if Some(revision) != cast.metadata.prepared_residence_revision {
                return;
            }
            let _ = manager.with_player_mut_like_cpp(handle, |player| {
                let state = &mut player.unit_mut().subsystems_mut().spells.execution;
                if state
                    .active
                    .as_ref()
                    .is_some_and(|active| active.cast_id == cast.cast_id)
                {
                    state.last_cast_time = Some(cast.cast_start_time);
                }
            });
        }
    }
}
