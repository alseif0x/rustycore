//! Unit values, visibility and health-revision state regression scenarios, part 3 of 3.
//!
//! Moved out of the unit.rs root under #636; every test is unchanged.

use super::*;

#[test]
fn values_update_sets_unit_object_type_bit() {
    let mut unit = Unit::new(true);

    unit.set_level(12);
    let update = unit.values_update();

    assert!(update.has_data());
    assert_eq!(update.changed_object_type_mask, 1 << TYPEID_UNIT);
    let unit_data = update.unit_data.unwrap();
    assert_eq!(unit_data.values.level, 12);
    assert!(unit_data.mask.is_set(UNIT_DATA_LEVEL_BIT));
}

#[test]
fn emote_state_marks_cpp_nested_unit_data_bits() {
    let mut unit = Unit::new(true);
    unit.clear_unit_data_changes();

    unit.set_emote_state_like_cpp(10);

    assert_eq!(unit.emote_state_like_cpp(), 10);
    assert_eq!(
        unit.values_update().unit_data.unwrap().values.emote_state,
        10
    );
    assert!(
        unit.unit_data_changes_mask()
            .is_set(UNIT_DATA_MODS_PARENT_BIT)
    );
    assert!(
        unit.unit_data_changes_mask()
            .is_set(UNIT_DATA_EMOTE_STATE_BIT)
    );
}

#[test]
fn pvp_flags_match_cpp_unit_pvp_state_helpers() {
    let mut unit = Unit::new(true);

    unit.set_pvp_flag_like_cpp(UnitPvpFlags::PVP | UnitPvpFlags::FFA_PVP);
    assert!(unit.is_pvp_like_cpp());
    assert!(unit.is_ffa_pvp_like_cpp());
    assert!(!unit.is_in_sanctuary_like_cpp());
    assert!(
        unit.unit_data_changes_mask()
            .is_set(UNIT_DATA_PVP_FLAGS_BIT)
    );

    unit.remove_pvp_flag_like_cpp(UnitPvpFlags::FFA_PVP);
    assert!(unit.is_pvp_like_cpp());
    assert!(!unit.is_ffa_pvp_like_cpp());

    unit.replace_all_pvp_flags_like_cpp(UnitPvpFlags::SANCTUARY | UnitPvpFlags::UNK1);
    assert!(!unit.is_pvp_like_cpp());
    assert!(unit.is_in_sanctuary_like_cpp());
    assert!(unit.has_pvp_flag_like_cpp(UnitPvpFlags::UNK1));
}

#[test]
fn health_state_revision_advances_only_for_real_health_or_death_changes_like_cpp() {
    let mut unit = Unit::new(true);
    unit.set_max_health(100);
    assert_eq!(unit.health_state_revision_like_cpp(), 1);

    unit.set_health(100);
    assert_eq!(unit.health_state_revision_like_cpp(), 2);
    unit.set_health(100);
    unit.set_death_state(DeathState::Alive);
    assert_eq!(unit.health_state_revision_like_cpp(), 2);

    unit.set_death_state(DeathState::JustDied);
    assert_eq!(unit.health_state_revision_like_cpp(), 3);
    unit.set_health(50);
    assert_eq!(unit.data().health, 0);
    assert_eq!(unit.health_state_revision_like_cpp(), 4);
    unit.set_health(50);
    assert_eq!(unit.health_state_revision_like_cpp(), 4);
}

#[test]
fn committed_mirror_replay_adopts_revision_without_rewinding_shared_allocator_like_cpp() {
    let mut base = Unit::new(true);
    base.set_max_health(100);
    base.set_health(100);
    let authority = base.health_state_revision_authority_like_cpp();
    let mut canonical = base.clone();
    let mut mirror = base;

    canonical.set_health(90);
    assert_eq!(canonical.health_state_revision_like_cpp(), 3);
    mirror.set_health(90);
    assert_eq!(mirror.health_state_revision_like_cpp(), 4);
    assert!(mirror.shares_health_state_revision_authority_like_cpp(&authority));

    mirror.adopt_committed_health_state_revision_for_mirror_like_cpp(3);
    assert_eq!(mirror.health_state_revision_like_cpp(), 3);
    canonical.set_health(80);
    assert_eq!(
        canonical.health_state_revision_like_cpp(),
        5,
        "the mirror replay reservation remains consumed"
    );
}

#[test]
fn snapshot_preservation_copies_exact_authoritative_health_timeline_like_cpp() {
    let mut canonical = Unit::new(true);
    canonical.set_max_health(100);
    canonical.set_health(100);
    canonical.set_health(0);
    canonical.set_death_state(DeathState::Corpse);

    let mut snapshot = Unit::new(true);
    snapshot.set_max_health(200);
    snapshot.set_health(150);
    snapshot.preserve_authoritative_health_state_for_snapshot_like_cpp(&canonical);

    assert_eq!(snapshot.data().max_health, 100);
    assert_eq!(snapshot.data().health, 0);
    assert_eq!(snapshot.death_state(), DeathState::Corpse);
    assert_eq!(
        snapshot.health_state_revision_like_cpp(),
        canonical.health_state_revision_like_cpp()
    );
    assert!(snapshot.shares_health_state_revision_authority_like_cpp(
        &canonical.health_state_revision_authority_like_cpp()
    ));
}
