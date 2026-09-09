//! Object update-block regressions, part 5 of 5.
//!
//! Moved out of the update_tests.rs root under #640; every test is unchanged.

use super::*;

#[test]
fn active_player_movement_block_adds_721_bytes() {
    // Self-view packets include a 721-byte ActivePlayer block in
    // BuildMovementUpdate: 1 byte (3 bits + flush) + 180 action buttons (720 bytes).
    // Non-self packets don't have this block.
    let guid = ObjectGuid::create_player(1, 42);
    let pos = Position::new(0.0, 0.0, 0.0, 0.0);
    let self_pkt = UpdateObject::create_player(
        guid,
        1,
        1,
        0,
        1,
        49,
        &pos,
        0,
        12,
        true,
        [(0, 0, 0); 19],
        [ObjectGuid::EMPTY; 141],
        PlayerCombatStats::default(),
        Vec::new(),
        0,
        Vec::new(),
    );
    let other_pkt = UpdateObject::create_player(
        guid,
        1,
        1,
        0,
        1,
        49,
        &pos,
        0,
        12,
        false,
        [(0, 0, 0); 19],
        [ObjectGuid::EMPTY; 141],
        PlayerCombatStats::default(),
        Vec::new(),
        0,
        Vec::new(),
    );
    let self_bytes = self_pkt.to_bytes();
    let other_bytes = other_pkt.to_bytes();

    // The difference between self and non-self should include:
    // - 721 bytes from ActivePlayer movement block
    // - plus the ActivePlayerData values block difference
    // The ActivePlayer movement block alone is 721 bytes.
    let diff = self_bytes.len() - other_bytes.len();
    assert!(
        diff > 721,
        "Self/non-self difference ({}) should be > 721 (ActivePlayer block)",
        diff
    );
}

#[test]
fn active_player_movement_block_writes_loaded_action_buttons_like_cpp() {
    let guid = ObjectGuid::create_player(1, 42);
    let pos = Position::new(0.0, 0.0, 0.0, 0.0);
    let mut pkt = UpdateObject::create_player(
        guid,
        1,
        1,
        0,
        1,
        49,
        &pos,
        0,
        12,
        true,
        [(0, 0, 0); 19],
        [ObjectGuid::EMPTY; 141],
        PlayerCombatStats::default(),
        Vec::new(),
        0,
        Vec::new(),
    );
    let sentinel = 0xA1B2_C3D4u32;
    let mut action_buttons = [0; MAX_ACTION_BUTTONS];
    action_buttons[17] = sentinel;
    pkt.set_player_action_buttons_like_cpp(action_buttons);

    let bytes = pkt.to_bytes();
    let sentinel_bytes = sentinel.to_le_bytes();
    assert!(
        bytes.windows(4).any(|window| window == sentinel_bytes),
        "C++ ActivePlayer movement block writes Player::m_actionButtons into self CREATE"
    );
}
