use super::*;

#[test]
fn alive_unit_phase_maintains_exact_health_thresholds_and_preserves_other_states() {
    let mut unit = Unit::new(true);
    unit.world_mut().object_mut().add_to_world();
    unit.set_max_health(100);
    unit.subsystems_mut().auras.modify_aura_state(1, true);
    let mut previous = 0;
    for (health, expected) in [
        (0, 0x0090_1022),
        (19, 0x0090_1022),
        (20, 0x0080_1020),
        (24, 0x0080_1020),
        (25, 0x0080_1000),
        (34, 0x0080_1000),
        (35, 0),
        (75, 0),
        (76, 0x0040_0000),
        (80, 0x0040_0000),
        (81, 0x00D0_0000),
        (100, 0x00D0_0000),
    ] {
        unit.set_health(health);
        unit.clear_unit_data_changes();
        unit.world_mut().object_mut().clear_update_mask(false);
        unit.update_health_aura_states_like_cpp();
        assert_eq!(unit.subsystems().auras.aura_state_mask, expected | 1);
        for bit in 0..UNIT_DATA_BITS {
            assert_eq!(
                unit.unit_data_changes_mask().is_set(bit),
                previous != expected && [32, 44].contains(&bit)
            );
        }
        assert_eq!(
            unit.world().object().is_object_updated(),
            previous != expected
        );
        previous = expected;
    }
    unit.set_death_state(DeathState::Dead);
    unit.set_health(1);
    unit.clear_unit_data_changes();
    unit.update_health_aura_states_like_cpp();
    assert_eq!(
        unit.subsystems().auras.aura_state_mask,
        0x00D0_0001,
        "dead Unit updates retain the last health states"
    );
    assert!(!unit.unit_data_changes_mask().is_any_set());
    unit.set_death_state(DeathState::Alive);
    unit.set_health(50);
    unit.clear_unit_data_changes();
    unit.update_health_aura_states_like_cpp();
    assert_eq!(unit.subsystems().auras.aura_state_mask, 1);
    assert!(unit.unit_data_changes_mask().is_set(44));
    unit.set_max_health(0);
    unit.clear_unit_data_changes();
    unit.update_health_aura_states_like_cpp();
    assert_eq!(unit.subsystems().auras.aura_state_mask, 0x00D0_0001);
    for bit in 0..UNIT_DATA_BITS {
        assert_eq!(
            unit.unit_data_changes_mask().is_set(bit),
            [32, 44].contains(&bit)
        );
    }
}

#[test]
fn health_state_changes_invalidate_previously_inert_spell_hit_authority() {
    let mut unit = Unit::new(true);
    unit.set_max_health(100);
    unit.set_health(50);
    unit.subsystems_mut()
        .auras
        .set_spell_hit_aura_authority_inert_like_cpp(true);
    assert!(
        unit.subsystems()
            .auras
            .has_complete_spell_hit_inert_aura_authority_like_cpp()
    );
    unit.set_health(10);
    unit.update_health_aura_states_like_cpp();
    unit.set_health(50);
    unit.update_health_aura_states_like_cpp();
    assert_eq!(unit.subsystems().auras.aura_state_mask, 0);
    assert!(
        !unit
            .subsystems()
            .auras
            .has_complete_spell_hit_inert_aura_authority_like_cpp()
    );
}

#[test]
fn aura_removal_rebuild_preserves_unit_health_and_reactive_states() {
    let mut unit = Unit::new(true);
    unit.set_max_health(100);
    unit.set_health(10);
    unit.update_health_aura_states_like_cpp();
    let caster = ObjectGuid::create_player(1, 71);
    let first = AppliedAuraRef::new(800, caster, 0, 1);
    let second = AppliedAuraRef::new(801, caster, 1, 1);
    let auras = &mut unit.subsystems_mut().auras;
    auras.modify_aura_state(1, true);
    auras.register_applied_aura(first, Some(4), 0, 0);
    auras.register_applied_aura(second, Some(4), 0, 0);
    assert!(auras.unapply_aura(first, 0));
    assert_eq!(auras.aura_state_mask, 0x0090_102B);
    assert!(auras.unapply_aura(second, 0));
    assert_eq!(auras.aura_state_mask, 0x0090_1023);
}

#[test]
fn frozen_aura_state_is_owner_projected_and_keeps_per_caster_contributions() {
    let owner = ObjectGuid::create_player(1, 71);
    let caster = ObjectGuid::create_player(1, 72);
    let other = ObjectGuid::create_player(1, 73);
    let mut unit = Unit::new(true);
    unit.world_mut().object_mut().create(owner);
    unit.set_max_health(100);
    unit.set_health(10);
    unit.subsystems_mut().auras.register_applied_aura(
        AppliedAuraRef::new(800, owner, 0, 1),
        Some(14),
        0,
        0,
    );
    unit.subsystems_mut().auras.register_applied_aura(
        AppliedAuraRef::new(801, caster, 1, 1),
        Some(16),
        0,
        0,
    );
    unit.update_health_aura_states_like_cpp();
    let mut frozen = unit.values_update().unit_data.unwrap();
    assert_eq!(frozen.aura_state, 0x0090_1022 | (1 << 13));
    unit.subsystems_mut().auras.aura_state_auras.clear();
    unit.subsystems_mut().auras.aura_state_mask = 0;
    frozen.project_aura_state_for_target_like_cpp(caster);
    assert_eq!(frozen.aura_state, 0x0090_1022 | (1 << 15));
    frozen.project_aura_state_for_target_like_cpp(other);
    assert_eq!(frozen.aura_state, 0x0090_1022);
    frozen.project_aura_state_for_target_like_cpp(owner);
    assert_eq!(frozen.aura_state, 0x0090_1022 | (1 << 13));
}
