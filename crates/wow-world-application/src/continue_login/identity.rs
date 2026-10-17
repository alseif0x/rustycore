// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The loaded-character identity write, moved from `wow-world`
//! `session/persistence/load.rs` with its gender decode
//! (`session/player_load_values.rs`) (#1363 final cleanup).

use wow_constants::unit::Gender;
use wow_world_core::session::PlayerIdentityBootstrapLikeCpp;
use wow_world_spell::TalentHostLikeCpp;

pub fn gender_from_u8(value: u8) -> Gender {
    match value {
        1 => Gender::Female,
        2 => Gender::None,
        _ => Gender::Male,
    }
}

pub fn set_loaded_player_identity_like_cpp<H: TalentHostLikeCpp + ?Sized>(
    host: &mut H,
    map_id: u16,
    race: u8,
    class: u8,
    level: u8,
    gender: u8,
) {
    let initialize_reputation = host.hub_ref().player_race_like_cpp() != race
        || host.hub_ref().player_class_like_cpp() != class
        || host
            .core()
            .with_owned_player_like_cpp(|player| {
                player.reputation_like_cpp().faction_count_like_cpp() == 0
            })
            .unwrap_or(true);
    if host.core().player_map_id_like_cpp() != map_id
        || host.hub_ref().player_race_like_cpp() != race
        || host.hub_ref().player_class_like_cpp() != class
        || host.hub_ref().player_gender_like_cpp() != gender
    {
        host.core()
            .invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
    }
    host.core_mut().current_map_id = map_id;
    let gray_level = host.hub_ref().gray_level(level);
    let canonical = host
        .core()
        .with_owned_player_mut_like_cpp(|player| {
            player.set_race_class_gender(race, class, gender_from_u8(gender));
            player.set_level_and_gray_level_like_cpp(level, gray_level);
        })
        .is_some();
    if canonical {
        host.core_mut().player_identity_bootstrap_like_cpp = None;
    } else {
        #[cfg(any(test, feature = "test-fixtures"))]
        let world_test = host.test_fixtures_ref_like_cpp().is_some();
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let world_test = false;
        if !world_test && host.core().player_handle_like_cpp.is_some() {
            return;
        }
        host.core_mut().player_identity_bootstrap_like_cpp = Some(PlayerIdentityBootstrapLikeCpp {
            name: host.hub_ref().player_name_like_cpp(),
            race,
            class,
            level,
            gender,
        });
        #[cfg(any(test, feature = "test-fixtures"))]
        {
            host.hub_mut().fixtures.identity.player_race = race;
            host.hub_mut().fixtures.identity.player_class = class;
            host.hub_mut().fixtures.identity.player_level = level;
            host.hub_mut().fixtures.identity.player_gender = gender;
        }
    }
    host.hub_mut().set_player_faction_for_race_like_cpp(race);
    if initialize_reputation {
        host.hub_mut().initialize_reputation_mgr_like_cpp();
    }
    host.refresh_represented_talent_points_like_cpp();
}
