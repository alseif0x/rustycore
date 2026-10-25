//! Restore the Player-owned corpse projection during login.
//!
//! C++ anchors: `Player::LoadCorpse` (`Player.cpp:18157`) and its following
//! ghost-aura restoration (`CharacterHandler.cpp:1227-1243`).

use tracing::warn;
use wow_core::Position;

use crate::{
    CorpseHostLikeCpp, PLAYER_LOCAL_FLAG_RELEASE_TIMER, ResurrectionOperationCatalogs,
    SPELL_GHOST_LIKE_CPP, SPELL_WISP_SPIRIT_AURA_LIKE_CPP, resurrect_player_like_cpp,
    spawn_corpse_bones_like_cpp,
};

const AT_LOGIN_RESURRECT: u16 = 0x0100;
const NIGHT_ELF_RACE: u8 = 4;

/// Apply C++ `Player::LoadCorpse` to the already placed, canonical Player.
/// The query result remains an ephemeral input; `_corpseLocation` stays owned
/// by `PlayerResurrectionStateLikeCpp`.
pub async fn restore_player_corpse_after_login(
    host: &mut impl CorpseHostLikeCpp,
    catalogs: ResurrectionOperationCatalogs<'_>,
    row: Option<wow_persistence::PlayerCorpseLocationLoadRow>,
) {
    let Some((alive, resurrect_at_login)) = host.core().with_owned_player_like_cpp(|player| {
        (
            player.unit().is_alive(),
            player.persistent_capability_state_like_cpp().at_login_flags & AT_LOGIN_RESURRECT != 0,
        )
    }) else {
        return;
    };

    if alive || resurrect_at_login {
        spawn_corpse_bones_like_cpp(host, catalogs.item_guid_generator, false).await;
    }

    if !alive {
        if resurrect_at_login {
            resurrect_player_like_cpp(host, catalogs, 0.5, false).await;
        } else if let Some((map_id, position, instanceable)) = corpse_location_from_row(host, row) {
            let installed = host.core().with_owned_player_mut_like_cpp(|player| {
                player.resurrection_state_mut_like_cpp().corpse_location = Some((map_id, position));
            });
            if installed.is_some() {
                host.hub_mut()
                    .mutate_active_player_update_state_like_cpp(|state| {
                        if instanceable {
                            state.active_local_flags &= !PLAYER_LOCAL_FLAG_RELEASE_TIMER;
                        } else {
                            state.active_local_flags |= PLAYER_LOCAL_FLAG_RELEASE_TIMER;
                        }
                    });
            }
        }
    }

    host.core().with_owned_player_mut_like_cpp(|player| {
        player.remove_at_login_flags_like_cpp(AT_LOGIN_RESURRECT);
    });
    restore_ghost_auras(host, catalogs).await;
}

fn corpse_location_from_row(
    host: &impl CorpseHostLikeCpp,
    row: Option<wow_persistence::PlayerCorpseLocationLoadRow>,
) -> Option<(u32, Position, bool)> {
    let row = row?;
    let (Some(map_id), Some(x), Some(y), Some(z), Some(orientation)) =
        (row.map_id, row.x, row.y, row.z, row.orientation)
    else {
        warn!("Ignoring incomplete persisted Player corpse location during login");
        return None;
    };
    let position = Position::new(x, y, z, orientation);
    if !position.is_valid_map_coord_like_cpp() {
        warn!(
            map_id,
            "Ignoring invalid persisted Player corpse coordinates during login"
        );
        return None;
    }
    let Some(map) = host
        .hub_ref()
        .catalogs
        .maps
        .store
        .as_ref()
        .and_then(|store| store.get(u32::from(map_id)).copied())
    else {
        warn!(
            map_id,
            "Ignoring Player corpse location for unknown map during login"
        );
        return None;
    };
    Some((u32::from(map_id), position, map.is_instanceable_like_cpp()))
}

async fn restore_ghost_auras(
    host: &mut impl CorpseHostLikeCpp,
    catalogs: ResurrectionOperationCatalogs<'_>,
) {
    let Some((is_dead, race, has_wisp)) = host.core().with_owned_player_like_cpp(|player| {
        (
            player.unit().death_state() == wow_constants::DeathState::Dead,
            player.race_like_cpp(),
            aura_spell_present(player, SPELL_WISP_SPIRIT_AURA_LIKE_CPP),
        )
    }) else {
        return;
    };
    if !is_dead {
        return;
    }
    if race == NIGHT_ELF_RACE && !has_wisp {
        if !host
            .cast_triggered_self_spell_like_cpp(
                catalogs,
                SPELL_WISP_SPIRIT_AURA_LIKE_CPP,
                None,
                None,
            )
            .await
        {
            warn!("Failed to restore the persisted Wisp Spirit aura during login");
        }
    }
    // Re-read after the optional Wisp Spirit cast: its triggered script may
    // have installed Ghost, and C++ checks HasAura(8326) after that cast.
    let needs_ghost = host
        .core()
        .with_owned_player_like_cpp(|player| !aura_spell_present(player, SPELL_GHOST_LIKE_CPP));
    if needs_ghost == Some(true)
        && !host
            .cast_triggered_self_spell_like_cpp(catalogs, SPELL_GHOST_LIKE_CPP, None, None)
            .await
    {
        warn!("Failed to restore the persisted ghost aura during login");
    }
    // C++ `CharacterHandler::HandlePlayerLogin`: water-walk is restored after
    // both ghost auras have been applied.
    host.hub_mut().set_player_water_walking(true);
}

fn aura_spell_present(player: &wow_entities::Player, spell_id: i32) -> bool {
    let auras = &player.unit().subsystems().auras;
    auras.has_aura_spell_like_cpp(spell_id as u32)
        || auras
            .runtime_applications_like_cpp()
            .values()
            .any(|application| application.spell_id == spell_id)
}

#[cfg(test)]
mod tests {
    use super::aura_spell_present;
    use std::time::Instant;
    use wow_core::ObjectGuid;
    use wow_entities::{AuraApplicationLikeCpp, Player};

    #[test]
    fn runtime_aura_application_counts_as_present_during_login() {
        let mut player = Player::new(Some(1), false);
        player.insert_player_visible_aura_like_cpp(AuraApplicationLikeCpp {
            spell_id: 8_326,
            difficulty_id: 0,
            caster_guid: ObjectGuid::EMPTY,
            slot: 0,
            duration_total: -1,
            duration_remaining: -1,
            stack_count: 1,
            aura_flags: 0,
            effect_mask: 0,
            aura_interrupt_flags: 0,
            aura_interrupt_flags2: 0,
            represented_effect: None,
            represented_amount: 0,
            represented_effect_amounts: Vec::new(),
            represented_misc_value: None,
            represented_multiplier: 1.0,
            applied_at: Instant::now(),
        });

        assert!(aura_spell_present(&player, 8_326));
        assert!(!aura_spell_present(&player, 8_327));
    }
}
