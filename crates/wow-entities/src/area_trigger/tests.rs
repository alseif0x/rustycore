use super::*;
use wow_core::guid::HighGuid;

fn caster_guid() -> ObjectGuid {
    ObjectGuid::create_global(HighGuid::Player, 0, 1)
}

fn spell_guid() -> ObjectGuid {
    ObjectGuid::create_world_object(HighGuid::DynamicObject, 0, 1, 530, 123, 0, 99)
}

#[test]
fn box_containment_rotates_into_trigger_local_axes() {
    let center = Position::new(10.0, 20.0, 3.0, std::f32::consts::FRAC_PI_2);
    assert!(position_is_within_area_trigger_box_like_cpp(
        &Position::new(10.0, 20.5, 3.0, 0.0),
        &center,
        1.0,
        0.5,
        1.0,
    ));
    assert!(!position_is_within_area_trigger_box_like_cpp(
        &Position::new(11.0, 20.0, 3.0, 0.0),
        &center,
        1.0,
        0.5,
        1.0,
    ));
}

#[test]
fn areatrigger_constructor_matches_cpp_base_state() {
    let area_trigger = AreaTrigger::new();

    assert!(!area_trigger.world().is_world_object());
    assert_eq!(area_trigger.world().object().type_id(), TypeId::AreaTrigger);
    assert_eq!(
        area_trigger.world().object().type_mask(),
        TypeMask::OBJECT | TypeMask::AREA_TRIGGER
    );
    assert!(
        area_trigger
            .world()
            .object()
            .create_flags()
            .contains(CreateObjectFlags::STATIONARY | CreateObjectFlags::AREA_TRIGGER)
    );
    assert_eq!(area_trigger.spawn_id(), 0);
    assert!(!area_trigger.is_static_spawn());
    assert_eq!(area_trigger.duration_ms(), 0);
    assert_eq!(area_trigger.total_duration_ms(), 0);
    assert_eq!(area_trigger.time_since_created_ms(), 0);
    assert!(
        area_trigger
            .vertices_update_previous_orientation()
            .is_infinite()
    );
    assert!(!area_trigger.is_removed());
    assert!(area_trigger.reached_destination());
    assert_eq!(area_trigger.last_spline_index(), 0);
    assert_eq!(area_trigger.movement_time_ms(), 0);
    assert_eq!(area_trigger.create_properties_id(), None);
    assert_eq!(area_trigger.template_id(), None);
    assert!(!area_trigger.is_custom());
    assert!(!area_trigger.is_server_side());
    assert!(!area_trigger.is_aura_effect_bound());
    assert!(!area_trigger.is_ai_initialized());
    assert!(area_trigger.inside_units().is_empty());
}

#[test]
fn areatrigger_data_setters_mark_cpp_bits() {
    let mut area_trigger = AreaTrigger::new();
    area_trigger.set_caster_guid(caster_guid());
    area_trigger.set_duration(1_500);
    area_trigger.set_time_to_target(11);
    area_trigger.set_time_to_target_scale(12);
    area_trigger.set_time_to_target_extra_scale(13);
    area_trigger.set_time_to_target_pos(14);
    area_trigger.set_spell_id(123);
    area_trigger.set_spell_for_visuals(124);
    area_trigger.set_spell_visual_id(125);
    area_trigger.set_bounds_radius_2d(10.5);
    area_trigger.set_decal_properties_id(24);
    area_trigger.set_creating_effect_guid(spell_guid());
    area_trigger.set_orbit_path_target(caster_guid());
    area_trigger.set_visual_anim(VisualAnimValues {
        field_c: true,
        animation_data_id: 1,
        anim_kit_id: 2,
        anim_progress: 3,
    });
    area_trigger.set_override_scale_constant(2.0);
    area_trigger.set_extra_scale_constant(3.0);
    area_trigger.set_override_move_constant(1.0, 2.0, 3.0);

    let mask = area_trigger.area_trigger_data_changes_mask();
    for bit in 0..AREA_TRIGGER_DATA_BITS {
        assert!(mask.is_set(bit), "bit {bit} should be set");
    }
    assert_eq!(area_trigger.caster_guid(), caster_guid());
    assert_eq!(area_trigger.creator_guid(), caster_guid());
    assert_eq!(area_trigger.owner_guid(), caster_guid());
    assert_eq!(area_trigger.spell_id(), 123);
    assert_eq!(area_trigger.data().duration, 1_500);
    assert!(area_trigger.data().override_scale_curve.override_active);
    assert_eq!(
        area_trigger.data().override_scale_curve.parameter_curve,
        2.0f32.to_bits() | 1
    );
}

#[test]
fn areatrigger_duration_and_static_state_follow_cpp_shape() {
    let mut area_trigger = AreaTrigger::new();
    area_trigger.set_duration(-1);
    assert_eq!(area_trigger.duration_ms(), -1);
    assert_eq!(area_trigger.total_duration_ms(), -1);
    assert_eq!(area_trigger.data().duration, 0);
    assert!(!area_trigger.update_time_and_duration(10_000));
    assert_eq!(area_trigger.time_since_created_ms(), 10_000);

    area_trigger.set_duration(100);
    area_trigger.clear_area_trigger_data_changes();
    assert!(!area_trigger.update_time_and_duration(40));
    assert_eq!(area_trigger.duration_ms(), 60);
    assert_eq!(area_trigger.data().duration, 60);
    assert!(!area_trigger.area_trigger_data_changes_mask().is_any_set());
    assert!(area_trigger.update_time_and_duration(60));
    assert!(area_trigger.is_removed());

    area_trigger.set_spawn_id(42);
    assert!(area_trigger.is_static_spawn());
    area_trigger.set_template(
        AreaTriggerId {
            id: 7,
            is_custom: true,
        },
        AREA_TRIGGER_FLAG_IS_SERVER_SIDE,
    );
    assert!(area_trigger.is_custom());
    assert!(area_trigger.is_server_side());
}

#[test]
fn areatrigger_values_update_sets_type_bit() {
    let mut area_trigger = AreaTrigger::new();
    area_trigger.set_spell_id(1);

    let update = area_trigger.values_update();
    assert_eq!(update.changed_object_type_mask, 1 << TYPEID_AREA_TRIGGER);
    assert!(update.object_data.is_none());
    assert!(update.area_trigger_data.is_some());
}
