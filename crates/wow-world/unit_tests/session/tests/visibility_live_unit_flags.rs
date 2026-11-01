//! #1356 review follow-up: a visibility CREATE block carries the creature's
//! live `UnitFlags`.
//!
//! C++ (`a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`): `Unit::BuildValuesCreate`
//! (`Unit.cpp:13228`) → `UnitData::WriteCreate` (`UpdateFields.cpp:650`) writes
//! `ViewerDependentValue<FlagsTag>` (`UpdateFields.cpp:717`), i.e. the live
//! `m_unitData->Flags` (`ViewerDependentValues.h:228`; only a GM receiver loses
//! `UNIT_FLAG_UNINTERACTIBLE`), the value the live `Unit::SetUnitFlag` changed:
//! `SKINNABLE` on a skinnable corpse (`Unit::Kill`, `Unit.cpp:10678`) and
//! `IN_COMBAT` (`CombatManager::UpdateOwnerCombatState`, `CombatManager.cpp:389`),
//! not the template copy. Rust: `send_nearby_creatures_with_catalogs_like_cpp`
//! (`handlers/character/visibility/creatures.rs`) builds the block from
//! `creature_projection::creature_create_data_like_cpp`.

use super::*;
use wow_constants::UnitFlags;

const CREATURE_ENTRY: u32 = 901;

/// The CREATE packet `send_nearby_creatures` publishes for one canonical
/// creature, with `live_flag` (if any) added to its live `UnitData::Flags`
/// through the canonical mutation root, plus that creature's template flags.
async fn nearby_creature_create_like_cpp(live_flag: Option<UnitFlags>) -> (Vec<u8>, u32, u32) {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let canonical = shared_map_manager();
    let player_guid = ObjectGuid::create_player(1, 74_360);
    let player_position = Position::new(10.0, 10.0, 0.0, 0.0);
    let creature_guid = test_creature_guid(74_361);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(canonical_player_transfer_test_map_store_like_cpp());
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "LiveUnitFlags".to_string(),
        player_position,
        571,
        1,
        1,
        80,
        0,
    ));
    session
        .ensure_canonical_world_map_for_current_player_like_cpp()
        .expect("canonical viewer map");
    canonical.lock().unwrap().add_creature(
        571,
        0,
        crate::creature_projection::new_runtime_creature_like_cpp(
            creature_guid,
            CREATURE_ENTRY,
            Position::new(20.0, 20.0, 0.0, 0.0),
            100,
            80,
            1,
            2,
            0.0,
            1,
            35,
            0,
            0,
        ),
    );
    let (live, template) = session
        .mutate_world_creature(creature_guid, |creature| {
            if let Some(flag) = live_flag {
                let flags = creature.unit().unit_flags_like_cpp() | flag;
                creature.unit_mut().set_unit_flags_like_cpp(flags);
            }
            (
                creature.unit().unit_flags_like_cpp().bits(),
                creature.ai_ownership().unit_flags,
            )
        })
        .expect("the canonical creature is mutated in place");
    let _ = drain_server_packet_bytes(&send_rx);

    session
        .send_nearby_creatures(571, &player_position, 0)
        .await;

    assert!(
        session
            .core
            .client_visible_guids_like_cpp
            .contains(&creature_guid)
    );
    let mut creates = drain_server_packet_bytes(&send_rx)
        .into_iter()
        .filter(|packet| {
            packet.len() >= 2
                && u16::from_le_bytes([packet[0], packet[1]]) == ServerOpcodes::UpdateObject as u16
        })
        .collect::<Vec<_>>();
    assert_eq!(creates.len(), 1, "one CREATE UpdateObject");
    (creates.remove(0), live, template)
}

/// The packets differ in exactly one little-endian `u32` field, which holds
/// `control_value` in the control and `live_value` in the live packet.
fn assert_single_u32_field_differs_like_cpp(
    control: &[u8],
    live: &[u8],
    control_value: u32,
    live_value: u32,
) {
    assert_eq!(control.len(), live.len(), "same CREATE layout");
    let differing = (0..live.len())
        .filter(|&index| control[index] != live[index])
        .collect::<Vec<_>>();
    let (Some(&first), Some(&last)) = (differing.first(), differing.last()) else {
        panic!("the live flag must change the CREATE bytes");
    };
    let read = |bytes: &[u8], at: usize| u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
    let field = (last.saturating_sub(3)..=first)
        .filter(|&at| at + 4 <= live.len())
        .find(|&at| read(control, at) == control_value && read(live, at) == live_value);
    assert!(
        field.is_some(),
        "the only change is the UnitFlags field {control_value:#x} -> {live_value:#x}, \
         differing bytes {differing:?}"
    );
}

async fn assert_create_carries_live_unit_flag_like_cpp(flag: UnitFlags) {
    let (control, control_live, control_template) = nearby_creature_create_like_cpp(None).await;
    let (live, live_flags, live_template) = nearby_creature_create_like_cpp(Some(flag)).await;

    // Negative control: the template copy never carries the live flag, so a
    // block built from it would equal the control block.
    assert_eq!(live_template, control_template);
    assert_eq!(live_template & flag.bits(), 0);
    assert_eq!(control_live & flag.bits(), 0);
    assert_eq!(live_flags, control_live | flag.bits());
    assert_single_u32_field_differs_like_cpp(&control, &live, control_live, live_flags);
}

#[tokio::test]
async fn skinnable_corpse_create_carries_live_skinnable_unit_flag_like_cpp() {
    assert_create_carries_live_unit_flag_like_cpp(UnitFlags::SKINNABLE).await;
}

#[tokio::test]
async fn in_combat_creature_create_carries_live_in_combat_unit_flag_like_cpp() {
    assert_create_carries_live_unit_flag_like_cpp(UnitFlags::IN_COMBAT).await;
}
