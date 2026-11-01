use super::*;

#[test]
fn frozen_aura_state_serializes_exact_values_for_owner_caster_and_unrelated_viewer() {
    let owner = ObjectGuid::create_player(1, 701);
    let caster = ObjectGuid::create_player(1, 702);
    let other = ObjectGuid::create_player(1, 703);
    let mut unit = wow_entities::Unit::new(true);
    unit.world_mut().object_mut().create(owner);
    unit.set_max_health(100);
    unit.set_health(10);
    unit.subsystems_mut().auras.register_applied_aura(
        wow_entities::AppliedAuraRef::new(800, owner, 0, 1),
        Some(14),
        0,
        0,
    );
    unit.subsystems_mut().auras.register_applied_aura(
        wow_entities::AppliedAuraRef::new(801, caster, 1, 1),
        Some(16),
        0,
        0,
    );
    unit.clear_unit_data_changes();
    unit.update_health_aura_states_like_cpp();
    let frozen = unit.values_update();
    assert_eq!(frozen.unit_data.as_ref().unwrap().aura_state, 0x0090_3022);
    unit.subsystems_mut().auras.aura_state_mask = 0;
    unit.subsystems_mut().auras.aura_state_auras.clear();
    for (viewer, expected) in [
        (owner, 0x0090_3022),
        (caster, 0x0090_9022),
        (other, 0x0090_1022),
    ] {
        let mut update = frozen.clone();
        update
            .unit_data
            .as_mut()
            .unwrap()
            .project_aura_state_for_target_like_cpp(viewer);
        let bytes = unit_values_update_to_update_object(owner, 0, &update)
            .unwrap()
            .to_bytes();
        let mut wire = wow_packet::WorldPacket::from_bytes(&bytes);
        wire.read_uint16().unwrap();
        assert_eq!(wire.read_uint32().unwrap(), 1);
        assert_eq!(wire.read_uint16().unwrap(), 0);
        assert!(!wire.read_bit().unwrap());
        wire.reset_bits();
        wire.read_uint32().unwrap();
        assert_eq!(wire.read_uint8().unwrap(), 0);
        assert_eq!(wire.read_packed_guid().unwrap(), owner);
        assert_eq!(wire.read_uint32().unwrap(), 13);
        assert_eq!(wire.read_uint32().unwrap(), 1 << wow_entities::TYPEID_UNIT);
        assert_eq!(wire.read_bits(8).unwrap(), 2);
        assert_eq!(
            wire.read_bits(32).unwrap(),
            0x1001,
            "parent32 and AuraState44"
        );
        wire.reset_bits();
        assert_eq!(wire.read_uint32().unwrap(), expected);
        assert_eq!(wire.read_position(), bytes.len());
    }
}
