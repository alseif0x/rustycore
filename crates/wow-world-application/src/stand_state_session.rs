// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The represented->live stand-state intent (#1363 slice 12b7), moved from the
//! World `session/stand_state_adapter.rs`, and the cast-interruption
//! publication it uses, moved from `session/player_cast/publication.rs`.
//!
//! C++ `Unit::SetStandState` (`Unit.cpp:9966-9977`). The bodies run over
//! [`SpellExecutionHostLikeCpp`]; the canonical `Player::Unit` stays the live
//! owner and the packet order is unchanged.

use std::sync::Arc;

use tracing::debug;
use wow_constants::UnitStandStateType;
use wow_entities::SpellCastState;
use wow_world_core::session::{
    RepresentedLiveIntentAppliedLikeCpp, RepresentedLiveIntentApplyOutcomeLikeCpp,
    RepresentedLiveIntentLikeCpp, RepresentedStandChannelCancellationBoundary,
    RepresentedStandStateChangedLikeCpp,
};
use wow_world_spell::SpellExecutionHostLikeCpp;

/// Session-owned represented->live boundary.
///
/// Packet handlers construct a typed intent only after completing their
/// C++ validation. The bridge applies against the authoritative live owner
/// first and records evidence only when that application succeeds.
pub fn apply_represented_live_intent_like_cpp<H: SpellExecutionHostLikeCpp>(
    host: &mut H,
    intent: RepresentedLiveIntentLikeCpp,
) -> RepresentedLiveIntentApplyOutcomeLikeCpp {
    let outcome = match &intent {
        RepresentedLiveIntentLikeCpp::StandStateChanged(change) => {
            apply_represented_stand_state_changed_live_like_cpp(host, change)
        }
    };

    if matches!(
        outcome,
        RepresentedLiveIntentApplyOutcomeLikeCpp::Applied(_)
    ) {
        debug!(?intent, ?outcome, "represented->live intent applied");
        #[cfg(any(test, feature = "test-fixtures"))]
        if let Some(fixtures) = host.test_fixtures_like_cpp() {
            fixtures
                .presentation
                .represented_live_applications_like_cpp
                .push(wow_world_core::session::RepresentedLiveApplicationLikeCpp {
                    intent,
                    outcome,
                });
        }
    }

    outcome
}

/// C++ `Unit::SetStandState` (`Unit.cpp:9966-9977`).
///
/// The canonical `Player::Unit` is the live owner. Session-local stand and
/// aura state remain mirrors until their remaining consumers migrate.
fn apply_represented_stand_state_changed_live_like_cpp<H: SpellExecutionHostLikeCpp>(
    host: &mut H,
    change: &RepresentedStandStateChangedLikeCpp,
) -> RepresentedLiveIntentApplyOutcomeLikeCpp {
    let Some(player_guid) = host.player_guid() else {
        return RepresentedLiveIntentApplyOutcomeLikeCpp::RejectedMissingPlayer;
    };

    let state = change.state;
    let spell_store = host
        .hub_ref()
        .catalogs
        .spell_catalogs
        .spell_store
        .as_ref()
        .map(Arc::clone);
    let difficulty_store = host
        .hub_ref()
        .catalogs
        .difficulty_store
        .as_ref()
        .map(Arc::clone);
    let Some(represented_visible_auras) = host.hub_ref().resolved_player_visible_auras_like_cpp()
    else {
        return RepresentedLiveIntentApplyOutcomeLikeCpp::RejectedMissingCanonicalPlayer;
    };
    let spell_difficulty_id = host
        .core()
        .current_canonical_player_map_difficulty_id_like_cpp()
        .unwrap_or(0);
    let standing_flag = wow_entities::SPELL_AURA_INTERRUPT_FLAG_STANDING_LIKE_CPP;
    let Some((
        canonical_field_changed,
        canonical_removed_auras,
        canonical_removed_visible_slots,
        mut channel_cancellation_boundary,
        canonical_interrupted_spell_ids,
    )) = host.core().mutate_canonical_player_like_cpp(|player| {
        let previous = player.unit().stand_state_like_cpp();
        let unit = player.unit_mut();
        unit.set_stand_state_like_cpp(state);

        let mut removed_auras = Vec::new();
        let mut removed_visible_slots = Vec::new();
        if unit.is_stand_state_like_cpp() {
            // Keep the locally registered masks as a
            // fallback for transitional/test state, and use the real
            // SpellInterrupts.db2 metadata for live auras whose older
            // represented materializers still carry zero masks.
            let metadata_matches: Vec<_> = {
                let auras = &unit.subsystems().auras;
                auras
                    .applied_auras
                    .iter()
                    .copied()
                    .filter(|aura| {
                        if let Some(known) = auras.aura_interrupt_flags.get(aura) {
                            // Presence means this applied aura already
                            // carries resolved metadata, including a known
                            // non-Standing mask. Never override it with a
                            // DB2 row selected from the current map.
                            return known.0 & standing_flag != 0;
                        }
                        let store_matches = i32::try_from(aura.spell_id)
                            .ok()
                            .and_then(|spell_id| {
                                spell_store
                                    .as_ref()?
                                    .aura_interrupt_flags_for_difficulty_like_cpp(
                                        spell_id,
                                        spell_difficulty_id,
                                        difficulty_store.as_deref(),
                                    )
                            })
                            .is_some_and(|known| known[0] & standing_flag != 0);
                        // No canonical entry means this older materializer
                        // supplied no interrupt metadata; hydrate only that
                        // missing case through the current difficulty's C++
                        // fallback chain.
                        store_matches
                    })
                    .collect()
            };
            for aura in metadata_matches {
                let was_visible = unit
                    .subsystems()
                    .auras
                    .visible_auras
                    .get(&aura.slot)
                    .is_some_and(|visible| *visible == aura.aura_ref());
                if unit
                    .subsystems_mut()
                    .auras
                    .unapply_aura(aura, wow_entities::AURA_REMOVE_BY_INTERRUPT_LIKE_CPP)
                {
                    // C++ Unit::RemoveAura removes the locally owned Aura
                    // base after unapplying its application when this Unit
                    // is the owner. Presence in the local owned store is
                    // the bounded owner identity available to this model.
                    // Cross-Unit applications still require the full Aura
                    // runtime/fanout tracked by the represented boundary.
                    unit.subsystems_mut()
                        .auras
                        .remove_owned_by_aura_ref_like_cpp(aura.aura_ref());
                    removed_auras.push(aura);
                    if was_visible {
                        unit.subsystems_mut().auras.clear_visible(aura.slot);
                        removed_visible_slots.push(aura.slot);
                    }
                }
            }
        } else {
            debug_assert!(removed_auras.is_empty());
        }

        // C++ evaluates the current channel only after the Standing aura
        // removal loop and calls InterruptNonMeleeSpells(false). Reuse the
        // canonical Unit implementation so generic, autorepeat, and
        // channeled slots follow the same interruption ordering. Packet
        // and owned-effect cleanup still remains a typed boundary.
        let mut interrupted_spell_ids = Vec::new();
        let channel_cancellation_boundary = if unit.is_stand_state_like_cpp()
            && let Some(current) = unit.current_spell(wow_entities::CurrentSpellSlot::Channeled)
            && current.state == wow_constants::SpellState::Casting
        {
            let channel_flags = i32::try_from(current.spell_id).ok().and_then(|spell_id| {
                spell_store
                    .as_ref()?
                    .channel_interrupt_flags_for_difficulty_like_cpp(
                        spell_id,
                        spell_difficulty_id,
                        difficulty_store.as_deref(),
                    )
            });
            match channel_flags {
                Some(flags) if flags[0] & standing_flag != 0 => {
                    let interrupted = unit.interrupt_non_melee_spells(None, false, true);
                    interrupted_spell_ids
                        .extend(interrupted.iter().map(|(_, spell)| spell.spell_id));
                    Some(RepresentedStandChannelCancellationBoundary::Interrupted {
                        spell_id: current.spell_id,
                        canonical_spells_interrupted: interrupted.len(),
                        session_cast_interrupted: false,
                    })
                }
                Some(_) => None,
                None => Some(
                    RepresentedStandChannelCancellationBoundary::UnknownInterruptMetadata {
                        spell_id: current.spell_id,
                    },
                ),
            }
        } else {
            None
        };
        (
            previous != state,
            removed_auras,
            removed_visible_slots,
            channel_cancellation_boundary,
            interrupted_spell_ids,
        )
    })
    else {
        return RepresentedLiveIntentApplyOutcomeLikeCpp::RejectedMissingCanonicalPlayer;
    };

    let interrupted_cast = host
        .mutate_cast_execution_like_cpp(|execution| {
            let interrupted = execution.active.as_ref().is_some_and(|active| {
                u32::try_from(active.spell_id)
                    .ok()
                    .is_some_and(|spell_id| canonical_interrupted_spell_ids.contains(&spell_id))
            });
            if interrupted {
                execution.take_interrupted_cast(None)
            } else {
                None
            }
        })
        .flatten();
    let session_cast_interrupted = interrupted_cast.is_some();
    if let Some(cast) = interrupted_cast {
        publish_player_cast_interruption_like_cpp(host, cast);
    }
    if let Some(RepresentedStandChannelCancellationBoundary::Interrupted {
        session_cast_interrupted: recorded,
        ..
    }) = &mut channel_cancellation_boundary
    {
        *recorded = session_cast_interrupted;
    }

    let mut represented_removed_slots = Vec::new();
    if state == UnitStandStateType::Stand {
        represented_removed_slots.extend(
            represented_visible_auras
                .values()
                .filter(|aura| {
                    let snapshot_matches = aura.aura_interrupt_flags & standing_flag != 0;
                    if aura.aura_interrupt_flags != 0 || aura.aura_interrupt_flags2 != 0 {
                        return snapshot_matches;
                    }
                    let store_matches = spell_store
                        .as_ref()
                        .and_then(|store| {
                            store.aura_interrupt_flags_for_difficulty_like_cpp(
                                aura.spell_id,
                                spell_difficulty_id,
                                difficulty_store.as_deref(),
                            )
                        })
                        .is_some_and(|known| known[0] & standing_flag != 0);
                    store_matches
                })
                .map(|aura| aura.slot),
        );
        represented_removed_slots.sort_unstable();
        for slot in represented_removed_slots.iter().copied() {
            let _ = host.remove_aura(slot);
        }
    }

    host.hub_mut().set_player_stand_state_like_cpp(state);

    use wow_packet::ServerPacket;
    let mut removed_slots: Vec<u8> = canonical_removed_visible_slots
        .into_iter()
        .chain(represented_removed_slots.iter().copied())
        .collect();
    removed_slots.sort_unstable();
    removed_slots.dedup();
    for slot in removed_slots {
        let aura_update = wow_packet::packets::misc::AuraUpdate {
            unit_guid: player_guid,
            update_all: false,
            auras: vec![wow_packet::packets::misc::AuraInfoLikeCpp {
                slot,
                aura_data: None,
            }],
        };
        if !represented_removed_slots.contains(&slot) {
            host.send_packet(&aura_update);
        }
        host.hub_ref()
            .broadcast_to_movement_set_like_cpp(aura_update.to_bytes(), false);
    }

    // Opcodes.cpp registers SMSG_STAND_STATE_UPDATE on
    // CONNECTION_TYPE_REALM even though the CMSG is handled in-world.
    host.core()
        .send_packet_realm(&wow_packet::packets::misc::StandStateUpdate {
            anim_kit_id: 0,
            stand_state: state as u8,
        });

    // `UnitData::StandState` is `UpdateField<uint8, 32, 56>` in the C++
    // generated fields. Until map-owned SendObjectUpdates has real fanout,
    // mirror this one live delta through the existing visibility registry.
    // Keep the canonical dirty bit set: only canonical object-update
    // processing may consume it, including when session routing is absent.
    if canonical_field_changed {
        let mut values = wow_packet::packets::update::UnitDataValuesDeltaUpdate::default();
        values.unit_data_mask[1] = (1 << (32 - 32)) | (1 << (56 - 32));
        values.stand_state = state as u8;
        let update = wow_packet::packets::update::UpdateObject::unit_values_update(
            player_guid,
            host.core().player_map_id_like_cpp(),
            values,
        );
        let bytes = update.to_bytes();
        host.send_raw_packet(&bytes);
        host.hub_ref()
            .broadcast_to_movement_set_like_cpp(bytes, false);
    }

    RepresentedLiveIntentApplyOutcomeLikeCpp::Applied(
        RepresentedLiveIntentAppliedLikeCpp::StandStateChanged {
            canonical_field_changed,
            canonical_auras_removed: canonical_removed_auras.len(),
            represented_auras_removed: represented_removed_slots.len(),
            channel_cancellation_boundary,
        },
    )
}

fn publish_player_cast_interruption_like_cpp<H: SpellExecutionHostLikeCpp>(
    host: &mut H,
    cast: SpellCastState,
) {
    let (state, mut hub) = host.split_spell_state_mut();
    state.publish_player_cast_interruption_like_cpp(&mut hub, cast)
}

/// The `CanFreeMove` block of C++ `HandleLogoutRequestOpcode`
/// (`MiscHandler.cpp:276-284`), run only for an admitted timed logout:
/// sit down if standing, root, and set `UNIT_FLAG_STUNNED`. "Not set flags if
/// player can't free move to prevent lost state at logout cancel."
pub fn begin_logout_wait_movement_like_cpp<H: SpellExecutionHostLikeCpp>(host: &mut H) {
    let Some(snapshot) = host.hub_ref().logout_admission_snapshot_like_cpp() else {
        return;
    };
    if !snapshot.can_free_move {
        return;
    }
    if snapshot.stand_state == UnitStandStateType::Stand {
        let _ = apply_represented_live_intent_like_cpp(
            host,
            RepresentedLiveIntentLikeCpp::StandStateChanged(RepresentedStandStateChangedLikeCpp {
                state: UnitStandStateType::Sit,
            }),
        );
    }
    host.hub_mut().set_player_rooted_like_cpp(true);
    host.hub_mut().set_player_unit_flag_stunned_like_cpp(true);
}

/// The `CanFreeMove` block of C++ `HandleLogoutCancelOpcode`
/// (`MiscHandler.cpp:300-310`): unroot, stand up, clear `UNIT_FLAG_STUNNED`.
/// "Not remove flags if can't free move - its not set in Logout request code."
pub fn restore_logout_wait_movement_like_cpp<H: SpellExecutionHostLikeCpp>(host: &mut H) {
    let Some(snapshot) = host.hub_ref().logout_admission_snapshot_like_cpp() else {
        return;
    };
    if !snapshot.can_free_move {
        return;
    }
    host.hub_mut().set_player_rooted_like_cpp(false);
    let _ = apply_represented_live_intent_like_cpp(
        host,
        RepresentedLiveIntentLikeCpp::StandStateChanged(RepresentedStandStateChangedLikeCpp {
            state: UnitStandStateType::Stand,
        }),
    );
    host.hub_mut().set_player_unit_flag_stunned_like_cpp(false);
}
