//! Canonical state, catalog and transport adapter for Player cast requests.

use super::*;
use crate::player_cast::Runtime;
use wire::PlayerCastPublicationPhaseLikeCpp;
use wow_entities::{SpellCastMetadata, SpellCastState, SpellCastVisualLikeCpp};
use wow_packet::packets::spell::{CastFailed, SpellPreparePkt, SpellStartPkt};
use wow_world_core::session::SessionHubHostLikeCpp;
mod host;
mod state;
pub(in crate::session) mod wire;

impl Runtime for WorldSession {
    fn cast_time_multiplier(&self) -> f32 {
        wow_world_spell::represented_cast_speed_multiplier_like_cpp(self)
    }

    fn spell(&self, id: i32) -> Option<wow_data::SpellInfo> {
        self.spell_store().and_then(|store| store.get(id)).cloned()
    }

    fn spell_for_difficulty(
        &self,
        id: i32,
        difficulty_id: u8,
    ) -> Option<(wow_data::SpellInfo, u8)> {
        self.spell_for_difficulty_outcome(id, difficulty_id)
            .ok()
            .flatten()
    }

    fn spell_for_difficulty_outcome(
        &self,
        id: i32,
        difficulty_id: u8,
    ) -> Result<Option<(wow_data::SpellInfo, u8)>, &'static str> {
        let Some(store) = self.spell_store() else {
            tracing::warn!(
                spell_id = id,
                difficulty_id,
                "Normal server cast SpellStore unavailable"
            );
            return Err("normal server cast SpellStore unavailable");
        };
        let difficulties = self
            .hub_ref()
            .catalogs
            .difficulty_store()
            .map(AsRef::as_ref);
        match store.spell_info_for_difficulty_like_cpp(id, difficulty_id, difficulties) {
            Ok(selected) => Ok(selected),
            Err(error) => {
                tracing::warn!(
                    spell_id = id,
                    difficulty_id,
                    ?error,
                    "Normal server cast SpellInfo payload unavailable"
                );
                Err("normal server cast SpellInfo or difficulty authority unavailable")
            }
        }
    }

    fn server_normal_busy(&mut self) -> Option<bool> {
        #[cfg(test)]
        if self.test_fixtures_ref_like_cpp().is_some() && self.core.player_handle_like_cpp.is_none()
        {
            return self.mutate_cast_execution_like_cpp(|state| {
                state
                    .active
                    .as_ref()
                    .is_some_and(|active| active.cast_time_ms != 0)
            });
        }
        self.core.with_owned_player_like_cpp(|player| {
            player
                .unit()
                .subsystems()
                .spells
                .execution
                .active
                .as_ref()
                .is_some_and(|active| active.cast_time_ms != 0)
        })
    }

    fn known(&self, id: i32) -> bool {
        wow_world_spell::known_spells_like_cpp(self).contains(&id)
            || self
                .spell_store()
                .is_some_and(|store| store.has_attribute8_like_cpp(id, 0x0400_0000))
    }

    fn passive(&self, spell: i32) -> bool {
        self.spell_store()
            .is_some_and(|store| store.is_passive_like_cpp(spell))
    }

    fn resolve_override(&self, original: &wow_data::SpellInfo) -> wow_data::SpellInfo {
        wow_world_spell::represented_cast_spell_info_like_cpp(self, original)
    }

    fn remaining(&self, spell: &wow_data::SpellInfo) -> Option<(u32, u32)> {
        wow_world_application::remaining_global_cooldown_ms_like_cpp(self, spell)
            .zip(wow_world_application::remaining_active_spell_cast_ms_like_cpp(self))
    }

    fn replace_pending(&mut self, request: RepresentedPendingSpellCastRequestLikeCpp) {
        wow_world_application::request_represented_spell_cast_like_cpp(self, request);
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
        wow_world_spell::allocate_player_cast_identity_like_cpp(self, spell)
    }

    fn visual(&self, spell: &wow_data::SpellInfo) -> Option<SpellCastVisualLikeCpp> {
        let config = &self.config.creature_aggro_config_like_cpp;
        let Some(store) = config.spell_x_spell_visual_store.as_ref() else {
            #[cfg(test)]
            if self.core.player_handle_like_cpp.is_none() {
                return Some(SpellCastVisualLikeCpp::default());
            }
            return None;
        };
        let spell_id = u32::try_from(spell.spell_id).ok()?;
        for difficulty in creature_ai_spell_difficulty_chain_like_cpp(
            self.core.current_map_difficulty_id_like_cpp(),
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

    fn prepare_mapping(&mut self, client: ObjectGuid, server: ObjectGuid) {
        self.send_packet(&SpellPreparePkt {
            client_cast_id: client,
            server_cast_id: server,
        });
    }

    fn disabled(&self, spell: i32) -> bool {
        wow_world_spell::is_spell_disabled_for_player_like_cpp(self, spell)
    }

    fn check_caster_death_outcome(
        &mut self,
        spell: &wow_data::SpellInfo,
        cast: ObjectGuid,
        visual: &SpellCastVisualLikeCpp,
        metadata: &mut SpellCastMetadata,
    ) -> Result<bool, &'static str> {
        wow_world_spell::check_represented_caster_death_like_cpp(
            self,
            spell,
            cast,
            &crate::spell_cast_adapter::present_visual(visual.clone()),
            metadata,
        )
    }

    fn on_cooldown(&self, spell: &wow_data::SpellInfo) -> Option<bool> {
        if spell.recovery_time_ms == 0 {
            return Some(false);
        }
        let (state, hub) = crate::session::split_spell_state_ref(self);
        Some(
            state
                .spell_last_cast_time_like_cpp(hub, spell.spell_id)?
                .is_some_and(|at| at.elapsed().as_millis() < u128::from(spell.recovery_time_ms)),
        )
    }

    fn server_normal_on_cooldown(
        &self,
        spell: &wow_data::SpellInfo,
        actual_difficulty_id: u8,
        metadata: SpellCastMetadata,
    ) -> Option<bool> {
        let hub = self.hub_ref();
        let spell_store = hub.catalogs.spell_store()?;
        let serverside_spells = hub
            .catalogs
            .spell_catalogs
            .serverside_spell_store
            .as_deref()?;
        let ready = hub.core.with_owned_player_like_cpp(|player| {
            wow_world_spell::server_normal_cooldown_ready_like_cpp(
                player,
                spell,
                actual_difficulty_id,
                metadata,
                spell_store,
                serverside_spells,
                hub.catalogs.spell_class_options_store().map(AsRef::as_ref),
                hub.catalogs.difficulty_store().map(AsRef::as_ref),
                hub.catalogs.items.effect_store.as_deref(),
                hub.catalogs.spell_catalogs.spell_category_store.as_deref(),
            )
        })?;
        match ready {
            Ok(ready) => Some(!ready),
            Err(reason) => {
                tracing::warn!(
                    spell_id = spell.spell_id,
                    reason,
                    "Native Player SpellHistory readiness authority unavailable"
                );
                None
            }
        }
    }

    fn server_normal_global_cooldown_state(
        &self,
        spell: &wow_data::SpellInfo,
        actual_difficulty_id: u8,
    ) -> Option<(u32, bool)> {
        let hub = self.hub_ref();
        let spell_store = hub.catalogs.spell_store()?;
        let state = hub.core.with_owned_player_like_cpp(|player| {
            wow_world_spell::server_normal_global_cooldown_state_like_cpp(
                player,
                spell.spell_id,
                actual_difficulty_id,
                spell_store,
                hub.catalogs.difficulty_store().map(AsRef::as_ref),
                wow_world_core::session::unix_now_ms_like_cpp(),
            )
        })?;
        match state {
            Ok(state) => Some(state),
            Err(reason) => {
                tracing::warn!(
                    spell_id = spell.spell_id,
                    actual_difficulty_id,
                    reason,
                    "Native Player SpellHistory GCD authority unavailable"
                );
                None
            }
        }
    }

    fn server_normal_cooldown_at_cast_start(
        &mut self,
        spell: &wow_data::SpellInfo,
        _actual_difficulty_id: u8,
        metadata: SpellCastMetadata,
    ) -> Option<()> {
        wow_world_application::apply_server_normal_cooldown_at_cast_start_like_cpp(
            self, spell, metadata,
        )
    }

    fn check_power(
        &mut self,
        spell: &wow_data::SpellInfo,
        cast: ObjectGuid,
        visual: &SpellCastVisualLikeCpp,
    ) -> bool {
        let (s, mut h) = crate::session::split_spell_state_mut(self);
        s.check_spell_power_like_cpp(
            &mut h,
            spell,
            cast,
            spell.spell_id,
            &crate::spell_cast_adapter::present_visual(visual.clone()),
        )
    }

    fn check_server_normal_costs(
        &mut self,
        spell: &wow_data::SpellInfo,
        cast: ObjectGuid,
        visual: &SpellCastVisualLikeCpp,
        metadata: SpellCastMetadata,
    ) -> bool {
        self.check_server_normal_costs_outcome(spell, cast, visual, metadata)
            .unwrap_or(false)
    }

    fn check_server_normal_costs_outcome(
        &mut self,
        spell: &wow_data::SpellInfo,
        cast: ObjectGuid,
        visual: &SpellCastVisualLikeCpp,
        metadata: SpellCastMetadata,
    ) -> Result<bool, &'static str> {
        match wow_world_spell::server_normal_cost_result_like_cpp(
            wow_world_application::server_normal_cast_cost_facts_like_cpp(self, spell, metadata),
        ) {
            wow_world_spell::ServerNormalCostResultLikeCpp::Affordable => Ok(true),
            wow_world_spell::ServerNormalCostResultLikeCpp::MissingReagent(item_id) => {
                self.send_packet(&CastFailed {
                    cast_id: cast,
                    spell_id: spell.spell_id,
                    visual: crate::spell_cast_adapter::present_visual(visual.clone()),
                    reason: wow_constants::SpellCastResult::Reagents as i32,
                    fail_arg1: item_id as i32,
                    fail_arg2: 0,
                });
                Ok(false)
            }
            wow_world_spell::ServerNormalCostResultLikeCpp::MissingCurrency(currency_id) => {
                self.send_packet(&CastFailed {
                    cast_id: cast,
                    spell_id: spell.spell_id,
                    visual: crate::spell_cast_adapter::present_visual(visual.clone()),
                    reason: wow_constants::SpellCastResult::Reagents as i32,
                    fail_arg1: -1,
                    fail_arg2: i32::from(currency_id),
                });
                Ok(false)
            }
            wow_world_spell::ServerNormalCostResultLikeCpp::Unavailable(reason) => {
                tracing::warn!(
                    spell_id = spell.spell_id,
                    reason,
                    "Normal server cast cost authority unavailable"
                );
                Err(reason)
            }
        }
    }

    fn check_preconditions(
        &mut self,
        spell: &wow_data::SpellInfo,
        cast: ObjectGuid,
        visual: &SpellCastVisualLikeCpp,
        metadata: SpellCastMetadata,
    ) -> bool {
        self.check_preconditions_outcome(spell, cast, visual, metadata)
            .unwrap_or(false)
    }

    fn check_preconditions_outcome(
        &mut self,
        spell: &wow_data::SpellInfo,
        cast: ObjectGuid,
        visual: &SpellCastVisualLikeCpp,
        metadata: SpellCastMetadata,
    ) -> Result<bool, &'static str> {
        match wow_world_spell::check_represented_cast_preparation_like_cpp(
            self,
            spell,
            cast,
            &crate::spell_cast_adapter::present_visual(visual.clone()),
            metadata,
        ) {
            Ok(Some(_)) => Ok(true),
            Ok(None) => Ok(false),
            Err(reason) => {
                tracing::warn!(
                    spell_id = spell.spell_id,
                    reason,
                    "Cast preparation lost canonical inputs"
                );
                Err(reason)
            }
        }
    }

    fn install(&mut self, cast: SpellCastState) -> bool {
        #[cfg(test)]
        if self.core.player_handle_like_cpp.is_none()
            && cast.metadata.prepared_residence_revision.is_none()
        {
            return self.set_active_spell_cast_like_cpp(Some(cast));
        }
        let (Some(handle), Some(manager)) = (
            self.core.player_handle_like_cpp,
            self.core.canonical_map_manager.as_ref(),
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

    fn install_server_normal(&mut self, cast: SpellCastState) -> bool {
        #[cfg(test)]
        if self.core.player_handle_like_cpp.is_none() {
            return self
                .mutate_cast_execution_like_cpp(|state| {
                    if state
                        .active
                        .as_ref()
                        .is_some_and(|active| active.cast_time_ms != 0)
                    {
                        return false;
                    }
                    state.active = Some(cast);
                    true
                })
                .unwrap_or(false);
        }
        let (Some(handle), Some(manager)) = (
            self.core.player_handle_like_cpp,
            self.core.canonical_map_manager.as_ref(),
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
                let state = &mut player.unit_mut().subsystems_mut().spells.execution;
                if state
                    .active
                    .as_ref()
                    .is_some_and(|active| active.cast_time_ms != 0)
                {
                    return false;
                }
                state.active = Some(cast);
                true
            })
            .unwrap_or(false)
    }

    fn start(&mut self, cast: &SpellCastState, spell: &wow_data::SpellInfo) {
        let Some(actor_guid) = self.player_guid() else {
            return;
        };
        let caster = if cast.metadata.server_normal_cast_like_cpp {
            cast.metadata.cast_item_guid.unwrap_or(actor_guid)
        } else {
            actor_guid
        };
        // C++ `SendSpellStart` samples power before the debit; `m_timer != 0`
        // is the timed-cast gate for the immunity section.
        let phase = PlayerCastPublicationPhaseLikeCpp::Start {
            timed: cast.cast_time_ms != 0,
        };
        let (cast_data, cast_flags) =
            wow_world_spell::player_cast_publication_like_cpp(self, spell, &cast.metadata, phase);
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
        wow_world_spell::publish_player_cast_frame_like_cpp(
            self,
            cast.metadata,
            wow_packet::ServerPacket::to_bytes(&packet),
        );
        let starts_global_cooldown = if cast.metadata.server_normal_cast_like_cpp {
            cast.metadata.server_normal_started_global_cooldown
        } else {
            spell.cooldown_ms != 0
        };
        if starts_global_cooldown {
            let server_normal_gcd_category = if cast.metadata.server_normal_cast_like_cpp {
                cast.metadata
                    .actual_spell_difficulty_id_like_cpp
                    .and_then(|difficulty_id| {
                        self.hub_ref()
                            .catalogs
                            .spell_store()?
                            .start_recovery_category_at_difficulty_like_cpp(
                                spell.spell_id as u32,
                                difficulty_id,
                                self.hub_ref()
                                    .catalogs
                                    .difficulty_store()
                                    .map(AsRef::as_ref),
                            )
                            .ok()
                    })
            } else {
                None
            };
            #[cfg(test)]
            if self.core.player_handle_like_cpp.is_none() {
                let _ = self.mutate_cast_execution_like_cpp(|state| {
                    state.last_cast_time = Some(cast.cast_start_time);
                });
                return;
            }
            let (Some(handle), Some(manager)) = (
                self.core.player_handle_like_cpp,
                self.core.canonical_map_manager.as_ref(),
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
                if let Some(category_id) = server_normal_gcd_category {
                    if category_id != 0 {
                        wow_world_spell::add_server_normal_global_cooldown_like_cpp(
                            &mut player.unit_mut().subsystems_mut().spells.history,
                            category_id,
                            spell.cooldown_ms,
                            wow_world_core::session::unix_now_ms_like_cpp(),
                        );
                    }
                }
            });
        }
    }
}
