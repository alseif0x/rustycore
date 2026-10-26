//! Restore the Player-owned corpse projection during login.
//!
//! C++ anchors: `Player::LoadCorpse` (`Player.cpp:18157-18180`) and the
//! following ghost-aura restoration (`CharacterHandler.cpp:1227-1243`).

use tracing::warn;

use crate::{
    CorpseHostLikeCpp, ResurrectionOperationCatalogs, SPELL_GHOST_LIKE_CPP,
    SPELL_WISP_SPIRIT_AURA_LIKE_CPP,
};

const AT_LOGIN_RESURRECT: u16 = 0x0100;
const NIGHT_ELF_RACE: u8 = 4;

/// Apply `Player::LoadCorpse` to the already placed canonical Player. Only a
/// validated row projection is retained across cancellation; the DB row stays
/// at the persistence boundary.
pub async fn restore_player_corpse_after_login(
    host: &mut impl CorpseHostLikeCpp,
    catalogs: ResurrectionOperationCatalogs<'_>,
    row: Option<wow_persistence::PlayerCorpseLocationLoadRow>,
) -> Result<(), &'static str> {
    let Some((alive, at_login_resurrect)) = host.core().with_owned_player_like_cpp(|player| {
        (
            player.unit().is_alive(),
            player.persistent_capability_state_like_cpp().at_login_flags & AT_LOGIN_RESURRECT != 0,
        )
    }) else {
        return Err("Canonical Player is unavailable during corpse restoration");
    };
    let validated_corpse_location = validated_corpse_location(host, row);
    let phase = if alive || at_login_resurrect {
        wow_entities::PlayerResurrectionPhase::RecoveryBonesReady
    } else {
        // No bones are requested on the ordinary dead-login path.
        wow_entities::PlayerResurrectionPhase::RecoveryBonesConfirmed
    };
    if !crate::recovery_fences::begin(
        host,
        wow_entities::PlayerResurrectionOperation::CorpseLogin {
            validated_corpse_location,
        },
        phase,
    ) {
        return Err("Another canonical Player recovery is already pending during login");
    }
    match crate::recovery_fences::continue_operation(host, catalogs).await {
        crate::recovery_fences::Progress::Complete => Ok(()),
        crate::recovery_fences::Progress::Waiting
        | crate::recovery_fences::Progress::Continue
        | crate::recovery_fences::Progress::Held => {
            Err("Login corpse recovery remains held on its canonical Player")
        }
    }
}

fn validated_corpse_location(
    host: &impl CorpseHostLikeCpp,
    row: Option<wow_persistence::PlayerCorpseLocationLoadRow>,
) -> Option<(u32, wow_core::Position, bool)> {
    let row = row?;
    let (Some(map_id), Some(x), Some(y), Some(z), Some(orientation)) =
        (row.map_id, row.x, row.y, row.z, row.orientation)
    else {
        warn!("Ignoring incomplete persisted Player corpse location during login");
        return None;
    };
    let position = wow_core::Position::new(x, y, z, orientation);
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

pub(super) async fn restore_ghost_auras(
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
