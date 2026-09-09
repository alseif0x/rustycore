//! Pet scenarios for [`super`].
//!
//! Split out of player_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn set_battle_pet_data_marks_cpp_player_active_and_unit_fields() {
    let mut player = Player::new(None, false);
    let pet_guid = ObjectGuid::create_global(wow_core::guid::HighGuid::BattlePet, 0, 42);
    player.clear_data_changes();

    player.set_battle_pet_data_like_cpp(pet_guid, 3, 17);

    assert_eq!(player.summoned_battle_pet_guid_like_cpp(), Some(pet_guid));
    assert_eq!(player.active_data().summoned_battle_pet_guid, pet_guid);
    assert_eq!(player.data().current_battle_pet_breed_quality, 3);
    assert_eq!(player.unit().data().wild_battle_pet_level, 17);
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_PARENT_BIT)
    );
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_SUMMONED_BATTLE_PET_GUID_BIT)
    );
    assert!(
        player
            .player_data_changes_mask()
            .is_set(PLAYER_DATA_PARENT_BIT)
    );
    assert!(
        player
            .player_data_changes_mask()
            .is_set(PLAYER_DATA_CURRENT_BATTLE_PET_BREED_QUALITY_BIT)
    );
    assert!(
        player
            .unit()
            .unit_data_changes_mask()
            .is_set(crate::UNIT_DATA_WILD_BATTLE_PET_LEVEL_BIT)
    );
}
#[test]
fn clear_battle_pet_data_clears_canonical_summoned_guid_like_cpp() {
    let mut player = Player::new(None, false);
    let pet_guid = ObjectGuid::create_global(wow_core::guid::HighGuid::BattlePet, 0, 43);
    player.set_battle_pet_data_like_cpp(pet_guid, 3, 17);

    player.clear_battle_pet_data_like_cpp();

    assert_eq!(player.summoned_battle_pet_guid_like_cpp(), None);
    assert_eq!(player.data().current_battle_pet_breed_quality, 0);
    assert_eq!(player.unit().data().wild_battle_pet_level, 0);
}
