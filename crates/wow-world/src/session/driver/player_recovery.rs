// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The admitted Player timer owner shared by coordinated and standalone ticks.

use super::{SessionHandlerCatalogsLikeCpp, WorldSession};
use crate::session::mailbox::{
    PlayerRecoveryReply, PlayerRecoveryStatus, RunPlayerRecoveryRequest, SessionPassKind,
    SessionPhaseClaimLikeCpp, SessionPhasePassOutcomeLikeCpp, SessionPhaseRequestLikeCpp,
};
use wow_core::ObjectGuid;
use wow_map::{MapKey, PlayerHandle};
use wow_world_core::session::SessionState;
use wow_world_core::session::directory::PlayerRegistration;
use wow_world_entities::update_pvp_flag_like_cpp;

pub(super) async fn run_current_player_ticks(
    session: &mut WorldSession,
    diff_ms: u32,
    catalogs: &SessionHandlerCatalogsLikeCpp,
) {
    update_pvp_flag_like_cpp(session, wow_entities::game_time_secs_like_cpp());
    let _ = crate::session::hub_mut(session).set_represented_can_delay_teleport_like_cpp(true);
    session.world_entities.advance_creature_tick_like_cpp();

    let coordinated = session.is_map_phase_coordinated_like_cpp();
    let selected_result = if coordinated {
        Ok(())
    } else {
        let unix_time_ns = wow_core::GameTime::unix_time_nanos();
        let binding = capture_player_binding(session);
        run_selected_unit_update(session, diff_ms, unix_time_ns, binding)
    };
    let _ = crate::session::hub_mut(session).set_represented_can_delay_teleport_like_cpp(false);

    if let Err(error) = selected_result {
        tracing::debug!(
            account = session.core.account_id,
            %error,
            "Selected aura Unit update could not be completed"
        );
        return;
    }
    if !coordinated {
        run_regeneration_and_kill(session, diff_ms, catalogs);
    }

    run_legacy_aura_sweep(session);
    if !coordinated {
        advance_player_save_timer(session, diff_ms);
        let binding = capture_player_binding(session);
        process_pending_player_save(session, catalogs).await;
        if session.core.state != SessionState::LoggedIn
            || !binding_still_current(session, binding.as_ref())
        {
            return;
        }
        advance_player_death_timer(session, diff_ms, catalogs).await;
        if session.core.state != SessionState::LoggedIn
            || !binding_still_current(session, binding.as_ref())
        {
            return;
        }
    }

    // These Session-owned outputs remain at their existing point in the
    // coordinated World pass; Recovery owns only the Player timers above.
    session.revalidate_represented_tavern_resting_with_catalog_like_cpp(
        catalogs.area_triggers.db2.as_ref(),
    );
    session.tick_represented_online_xp_rest_bonus_with_policy_like_cpp(
        catalogs.player_rest_rates.as_ref(),
        wow_core::GameTime::now().as_secs(),
    );

    if !coordinated {
        process_delayed_teleport(session, catalogs).await;
    }
}

fn run_legacy_aura_sweep(session: &mut WorldSession) {
    if session.world_entities.creature_tick_like_cpp() % 4 != 0 {
        return;
    }
    let _ = crate::session::hub_mut(session).set_represented_can_delay_teleport_like_cpp(true);
    wow_world_spell::tick_auras_expiry_only(session);
    let _ = crate::session::hub_mut(session).set_represented_can_delay_teleport_like_cpp(false);
}

/// Consume one admitted Player timer pass from the existing Session phase rail.
pub(super) async fn run_requested_player_recovery(
    session: &mut WorldSession,
    request: RunPlayerRecoveryRequest,
    catalogs: &SessionHandlerCatalogsLikeCpp,
) {
    let admission = request.admission;
    if !session.core.admission.accept_phase_authority(
        SessionPassKind::PlayerRecovery,
        admission.coordinator_id,
        admission.tick_epoch,
    ) || !session
        .core
        .is_player_recovery_admission_current(&admission)
    {
        request.permit.refuse_before_start_like_cpp();
        let _ = request.response_tx.try_send(PlayerRecoveryReply {
            coordinator_id: admission.coordinator_id,
            tick_epoch: admission.tick_epoch,
            map_tick_epoch: admission.map_tick_epoch,
            outcome: SessionPhasePassOutcomeLikeCpp::RefusedBeforeStart,
            status: None,
            disconnecting: session.is_disconnecting(),
        });
        return;
    }

    match request.permit.claim_like_cpp() {
        SessionPhaseClaimLikeCpp::Claimed => {}
        SessionPhaseClaimLikeCpp::Revoked => {
            let _ = request.response_tx.try_send(PlayerRecoveryReply {
                coordinator_id: admission.coordinator_id,
                tick_epoch: admission.tick_epoch,
                map_tick_epoch: admission.map_tick_epoch,
                outcome: SessionPhasePassOutcomeLikeCpp::RevokedBeforeStart,
                status: None,
                disconnecting: session.is_disconnecting(),
            });
            return;
        }
        SessionPhaseClaimLikeCpp::AlreadyResolved(_) => {
            let _ = request.response_tx.try_send(PlayerRecoveryReply {
                coordinator_id: admission.coordinator_id,
                tick_epoch: admission.tick_epoch,
                map_tick_epoch: admission.map_tick_epoch,
                outcome: SessionPhasePassOutcomeLikeCpp::AlreadyResolved,
                status: None,
                disconnecting: session.is_disconnecting(),
            });
            return;
        }
    }

    let status = run_admitted_player_recovery(session, &admission, catalogs).await;
    if status == PlayerRecoveryStatus::Unavailable {
        request.permit.complete_unavailable();
    } else {
        request.permit.complete_like_cpp();
    }
    let _ = request.response_tx.try_send(PlayerRecoveryReply {
        coordinator_id: admission.coordinator_id,
        tick_epoch: admission.tick_epoch,
        map_tick_epoch: admission.map_tick_epoch,
        outcome: SessionPhasePassOutcomeLikeCpp::Ran,
        status: Some(status),
        disconnecting: session.is_disconnecting(),
    });
}

async fn run_admitted_player_recovery(
    session: &mut WorldSession,
    admission: &crate::session::mailbox::PlayerRecoveryAdmission,
    catalogs: &SessionHandlerCatalogsLikeCpp,
) -> PlayerRecoveryStatus {
    // C++ Unit::Update runs before Player regeneration, KillPlayer, SaveTimer,
    // deathTimer, and delayed teleport (`Player.cpp:935-942,1053-1066,1101-1111`).
    let _ = crate::session::hub_mut(session).set_represented_can_delay_teleport_like_cpp(true);
    let binding = PlayerBinding {
        guid: admission.registration.guid(),
        registration: Some(admission.registration),
        handle: admission.handle,
        map_key: admission.map_key,
        map_incarnation: admission.map_incarnation,
        residence_revision: admission.residence_revision,
    };
    let selected_result = run_selected_unit_update(
        session,
        admission.diff_ms,
        admission.unix_time_ns,
        Some(binding),
    );
    let _ = crate::session::hub_mut(session).set_represented_can_delay_teleport_like_cpp(false);
    if selected_result.is_err() {
        return recovery_status(session, admission).unwrap_or(PlayerRecoveryStatus::Unavailable);
    }
    if let Some(status) = recovery_status(session, admission) {
        return status;
    }

    run_regeneration_and_kill(session, admission.diff_ms, catalogs);
    if let Some(status) = recovery_status(session, admission) {
        return status;
    }

    advance_player_save_timer(session, admission.diff_ms);
    process_pending_player_save(session, catalogs).await;
    if session.core.state != SessionState::LoggedIn {
        return PlayerRecoveryStatus::Unavailable;
    }
    if !session.core.is_player_recovery_admission_current(admission) {
        return PlayerRecoveryStatus::PlayerLeft;
    }

    advance_player_death_timer(session, admission.diff_ms, catalogs).await;
    if let Some(status) = recovery_status(session, admission) {
        return status;
    }

    process_delayed_teleport(session, catalogs).await;
    recovery_status(session, admission).unwrap_or(PlayerRecoveryStatus::Completed)
}

fn run_selected_unit_update(
    session: &mut WorldSession,
    diff_ms: u32,
    unix_time_ns: u128,
    binding: Option<PlayerBinding>,
) -> Result<(), &'static str> {
    let Some(handle) = session.core.player_handle_like_cpp else {
        #[cfg(any(test, feature = "test-fixtures"))]
        {
            wow_world_spell::flush_pending_selected_aura_updates(session);
            return Ok(());
        }
        #[cfg(not(any(test, feature = "test-fixtures")))]
        return Err("Selected aura Unit update has no canonical Player handle");
    };

    let binding = binding.ok_or("Selected aura Unit update has no current Player binding")?;
    if binding.handle != handle || session.player_guid() != Some(binding.guid) {
        return Err("Selected aura Unit update Player binding is no longer current");
    }

    let manager = session
        .core
        .canonical_map_manager
        .as_ref()
        .ok_or("Selected aura Unit update has no canonical map manager")?
        .clone();
    {
        let mut manager = manager
            .lock()
            .map_err(|_| "Selected aura map manager lock is unavailable")?;
        if !wow_world_spell::update_player_spell_history(
            &mut manager,
            handle,
            binding.map_key,
            binding.map_incarnation,
            binding.residence_revision,
            unix_time_ns,
        ) {
            return Err("Native Player spell history has no current admitted residence");
        }
    }

    wow_world_spell::tick_selected_aura_lifetimes(session, diff_ms)?;

    let catalogs = wow_world_spell::SelectedAuraPublicationCatalogs {
        races: session.catalogs.chr.races_store.as_deref(),
        spells: session.catalogs.spell_catalogs.spell_store.as_deref(),
        aura_options: session
            .catalogs
            .spell_catalogs
            .spell_aura_options_store
            .as_deref(),
        difficulties: session.catalogs.difficulty_store.as_deref(),
    };
    let prepared = {
        let mut manager = manager
            .lock()
            .map_err(|_| "Selected aura map manager lock is unavailable")?;
        wow_world_spell::collect_player_pending_selected_aura_updates(
            &mut manager,
            handle,
            catalogs,
        )?
    };
    for update in &prepared {
        if !session
            .core
            .packet_publication_access_like_cpp()
            .publish_prepared_selected_aura_update(update)
        {
            return Err("Prepared selected aura source is no longer current");
        }
    }
    Ok(())
}

#[derive(Clone, Copy)]
struct PlayerBinding {
    guid: ObjectGuid,
    registration: Option<PlayerRegistration>,
    handle: PlayerHandle,
    map_key: MapKey,
    map_incarnation: u64,
    residence_revision: u64,
}

fn capture_player_binding(session: &WorldSession) -> Option<PlayerBinding> {
    let guid = session.player_guid()?;
    let registration = match session.player_registry() {
        Some(registry) => Some(
            registry
                .session_phase_address_like_cpp(guid)?
                .registration(),
        ),
        None => None,
    };
    let handle = session.core.player_handle_like_cpp?;
    let manager = session.core.canonical_map_manager.as_ref()?;
    let manager = manager.lock().ok()?;
    let (map_key, residence_revision) =
        manager.player_active_residence_revision_like_cpp(handle)?;
    let map_incarnation = manager.map_incarnation_like_cpp(map_key)?;
    manager
        .with_player_like_cpp(handle, |player| {
            let world = player.unit().world();
            (player.guid() == guid
                && world.object().is_in_world()
                && world.has_current_map()
                && MapKey::new(world.map_id(), world.instance_id()) == map_key)
                .then_some(PlayerBinding {
                    guid,
                    registration,
                    handle,
                    map_key,
                    map_incarnation,
                    residence_revision,
                })
        })
        .flatten()
}

fn binding_still_current(session: &WorldSession, binding: Option<&PlayerBinding>) -> bool {
    let Some(binding) = binding else {
        if session.core.player_handle_like_cpp.is_some() {
            return false;
        }
        // The unowned compatibility path exists only for the explicit
        // handleless Session fixtures; production never treats it as current.
        #[cfg(any(test, feature = "test-fixtures"))]
        return true;
        #[cfg(not(any(test, feature = "test-fixtures")))]
        return false;
    };
    if session.core.state != SessionState::LoggedIn
        || session.player_guid() != Some(binding.guid)
        || session.core.player_handle_like_cpp != Some(binding.handle)
    {
        return false;
    }
    match (binding.registration, session.player_registry()) {
        (Some(registration), Some(registry)) => {
            if registry.lookup_current(registration).is_none()
                || registry
                    .session_phase_address_like_cpp(binding.guid)
                    .is_none_or(|address| address.registration() != registration)
            {
                return false;
            }
        }
        (None, None) => {}
        _ => return false,
    }
    let Some(manager) = session.core.canonical_map_manager.as_ref() else {
        return false;
    };
    let Ok(manager) = manager.lock() else {
        return false;
    };
    if manager.map_incarnation_like_cpp(binding.map_key) != Some(binding.map_incarnation)
        || manager.player_active_residence_revision_like_cpp(binding.handle)
            != Some((binding.map_key, binding.residence_revision))
    {
        return false;
    }
    manager
        .with_player_like_cpp(binding.handle, |player| {
            let world = player.unit().world();
            player.guid() == binding.guid
                && world.object().is_in_world()
                && world.has_current_map()
                && MapKey::new(world.map_id(), world.instance_id()) == binding.map_key
        })
        .unwrap_or(false)
}

fn recovery_status(
    session: &WorldSession,
    admission: &crate::session::mailbox::PlayerRecoveryAdmission,
) -> Option<PlayerRecoveryStatus> {
    if session.core.state != SessionState::LoggedIn {
        Some(PlayerRecoveryStatus::Unavailable)
    } else if !session.core.is_player_recovery_admission_current(admission) {
        Some(PlayerRecoveryStatus::PlayerLeft)
    } else {
        None
    }
}

fn run_regeneration_and_kill(
    session: &mut WorldSession,
    diff_ms: u32,
    catalogs: &SessionHandlerCatalogsLikeCpp,
) {
    // C++ `Player::Update` runs RegenerateAll then KillPlayer before SaveTimer
    // (`Player.cpp:909,1053-1055`). The canonical Player owns the accumulators.
    let regen_game_tables = session.catalogs.regen_game_tables.clone();
    wow_world_entities::tick_player_regeneration_like_cpp(
        session,
        diff_ms,
        catalogs.creature_spawns.power_types.as_ref(),
        regen_game_tables.as_deref(),
        catalogs.player_regeneration_rates.as_ref(),
    );
    if session.core.player_is_strictly_in_world_like_cpp() {
        let just_died = session
            .core
            .with_owned_player_like_cpp(|player| {
                player.unit().death_state() == wow_constants::DeathState::JustDied
            })
            .unwrap_or(false);
        if just_died {
            wow_world_entities::kill_player_like_cpp(session);
        }
    }
}

fn advance_player_save_timer(session: &mut WorldSession, diff_ms: u32) {
    session.lifecycle.update_player_save_timer_like_cpp(diff_ms);
}

async fn process_pending_player_save(
    session: &mut WorldSession,
    catalogs: &SessionHandlerCatalogsLikeCpp,
) {
    session
        .core
        .record_driver_phase_like_cpp(super::phases::SessionDriverPhaseLikeCpp::PeriodicPlayerSave);
    wow_world_application::process_pending_periodic_player_save_with_generator_like_cpp(
        session,
        catalogs.id_generators.item.as_ref(),
    )
    .await;
}

async fn advance_player_death_timer(
    session: &mut WorldSession,
    diff_ms: u32,
    catalogs: &SessionHandlerCatalogsLikeCpp,
) {
    if session.core.player_is_strictly_in_world_like_cpp() {
        wow_world_entities::tick_player_death_timer(
            session,
            diff_ms,
            wow_world_entities::CorpseLifecycleCatalogsLikeCpp {
                item_guid_generator: catalogs.id_generators.item.as_ref(),
                creature_spawns: catalogs.creature_spawns.as_ref(),
                graveyards: catalogs.graveyards.as_ref(),
            },
        )
        .await;
    }
}

async fn process_delayed_teleport(
    session: &mut WorldSession,
    catalogs: &SessionHandlerCatalogsLikeCpp,
) {
    wow_world_entities::process_represented_delayed_teleport_after_update_like_cpp(
        session,
        wow_world_entities::ResurrectionOperationCatalogs {
            item_guid_generator: catalogs.id_generators.item.as_ref(),
            creature_spawns: catalogs.creature_spawns.as_ref(),
        },
    )
    .await;
}
