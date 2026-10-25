// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Existing LoggedIn Session-owned Player tick body.

use super::{SessionHandlerCatalogsLikeCpp, WorldSession};
use wow_world_entities::update_pvp_flag_like_cpp;

pub(super) async fn run_current_player_ticks(
    session: &mut WorldSession,
    diff_ms: u32,
    catalogs: &SessionHandlerCatalogsLikeCpp,
) {
    update_pvp_flag_like_cpp(session, wow_entities::game_time_secs_like_cpp());
    let _ = crate::session::hub_mut(session).set_represented_can_delay_teleport_like_cpp(true);
    session.world_entities.advance_creature_tick_like_cpp();
    // C++ `Player::Update` runs `RegenerateAll()` after
    // `DoMeleeAttackIfReady` for every living in-world player. The
    // canonical Player owns the accumulators; this session pass is the
    // single writer and consumes the published stat snapshot.
    let regen_game_tables = session.catalogs.regen_game_tables.clone();
    wow_world_entities::tick_player_regeneration_like_cpp(
        session,
        diff_ms,
        catalogs.creature_spawns.power_types.as_ref(),
        regen_game_tables.as_deref(),
        catalogs.player_regeneration_rates.as_ref(),
    );
    // C++ `Player::Update` returns at its entry when !IsInWorld and
    // calls KillPlayer for JUST_DIED immediately after RegenerateAll
    // and before the save timer (`Player.cpp:909`, `:1053`). The
    // retained canonical Player can be detached during a far transfer.
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
    // Aura expiry tick every 4 ticks (~200ms) — always, regardless of owner.
    if session.world_entities.creature_tick_like_cpp() % 4 == 0 {
        wow_world_spell::tick_auras_expiry_only(session);
    }
    // Selected Ghost/Wisp publication has one production owner: the
    // resumed canonical map tick. Keep the standalone Session path for
    // callers that do not participate in coordinated map phases.
    if !session.is_map_phase_coordinated_like_cpp() {
        wow_world_spell::flush_pending_selected_aura_updates(session);
    }
    // C++ Player::Update closes the delayed-teleport window immediately
    // after Unit::Update (`Player.cpp:935-942`). The save and death
    // timers run with can_delay=false, so graveyard release teleports
    // publish immediately (`Player.cpp:1101`).
    let _ = crate::session::hub_mut(session).set_represented_can_delay_teleport_like_cpp(false);
    session.lifecycle.update_player_save_timer_like_cpp(diff_ms);
    // C++ `Player::Update` advances m_deathTimer after SaveTimer and
    // before its delayed teleport (`Player.cpp:1101`), only while its
    // Player remains in world. The lifecycle operation owns expiry's
    // BuildPlayerRepop → RepopAtGraveyard.
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
    session.revalidate_represented_tavern_resting_with_catalog_like_cpp(
        catalogs.area_triggers.db2.as_ref(),
    );
    session.tick_represented_online_xp_rest_bonus_with_policy_like_cpp(
        catalogs.player_rest_rates.as_ref(),
        wow_core::GameTime::now().as_secs(),
    );
    wow_world_entities::process_represented_delayed_teleport_after_update_like_cpp(
        session,
        wow_world_entities::ResurrectionOperationCatalogs {
            item_guid_generator: catalogs.id_generators.item.as_ref(),
            creature_spawns: catalogs.creature_spawns.as_ref(),
        },
    )
    .await;
}
