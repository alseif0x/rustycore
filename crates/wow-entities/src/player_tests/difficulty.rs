//! #773 — the canonical Player owns its difficulty preferences.
//!
//! C++ keeps the three preferences apart, one accessor pair each:
//! `GetDungeonDifficultyID` (`Player.h:1961`) with `SetDungeonDifficultyID`
//! (`:1964`), `GetRaidDifficultyID` (`:1962`) with `SetRaidDifficultyID`
//! (`:1965`), and `GetLegacyRaidDifficultyID` (`:1963`) with
//! `SetLegacyRaidDifficultyID` (`:1966`).

use crate::Player;

fn player() -> Player {
    Player::new(Some(1), false)
}

#[test]
fn each_setter_writes_only_its_own_preference_like_cpp() {
    let mut player = player();
    player.replace_difficulty_preferences_like_cpp(1, 14, 3);

    player.set_dungeon_difficulty_id_like_cpp(2);
    assert_eq!(player.difficulty_preferences_like_cpp(), (2, 14, 3));

    player.set_raid_difficulty_id_like_cpp(15);
    assert_eq!(player.difficulty_preferences_like_cpp(), (2, 15, 3));

    player.set_legacy_raid_difficulty_id_like_cpp(4);
    assert_eq!(player.difficulty_preferences_like_cpp(), (2, 15, 4));
}

#[test]
fn the_named_reads_answer_the_same_values_as_the_paired_read_like_cpp() {
    let mut player = player();
    player.replace_difficulty_preferences_like_cpp(2, 15, 4);

    assert_eq!(player.dungeon_difficulty_id_like_cpp(), 2);
    assert_eq!(player.raid_difficulty_id_like_cpp(), 15);
    assert_eq!(player.legacy_raid_difficulty_id_like_cpp(), 4);
    assert_eq!(
        player.difficulty_preferences_like_cpp(),
        (
            player.dungeon_difficulty_id_like_cpp(),
            player.raid_difficulty_id_like_cpp(),
            player.legacy_raid_difficulty_id_like_cpp(),
        )
    );
}

#[test]
fn the_load_time_replacement_installs_all_three_together_like_cpp() {
    let mut player = player();
    player.set_dungeon_difficulty_id_like_cpp(9);

    player.replace_difficulty_preferences_like_cpp(1, 14, 3);

    assert_eq!(player.difficulty_preferences_like_cpp(), (1, 14, 3));
}

#[test]
fn writing_the_same_difficulty_twice_is_idempotent_like_cpp() {
    let mut player = player();

    player.set_raid_difficulty_id_like_cpp(14);
    player.set_raid_difficulty_id_like_cpp(14);

    assert_eq!(player.raid_difficulty_id_like_cpp(), 14);
}
