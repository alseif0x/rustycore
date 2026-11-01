//! Condition snapshot borrows regression scenarios, part 3 of 3.
//!
//! Moved out of the conditions.rs root under #656; every test is unchanged.

use super::*;

#[test]
fn specialized_condition_lookups_default_to_true_when_missing_like_cpp() {
    let store = ConditionEntriesByTypeStore::default();

    assert!(is_object_meeting_spell_click_conditions_like_cpp(
        &store,
        1,
        2,
        None,
        None,
        |_, _| false,
    ));
    assert!(is_object_meeting_vehicle_spell_conditions_like_cpp(
        &store,
        1,
        2,
        None,
        None,
        |_, _| false,
    ));
    assert!(is_object_meeting_smart_event_conditions_like_cpp(
        &store,
        1,
        2,
        3,
        None,
        None,
        |_, _| false,
    ));
    assert!(is_object_meeting_vendor_item_conditions_like_cpp(
        &store,
        1,
        2,
        None,
        None,
        |_, _| false,
    ));
    assert!(is_object_meeting_trainer_spell_conditions_like_cpp(
        &store,
        1,
        2,
        None,
        |_, _| false,
    ));
    assert!(
        is_object_meeting_visibility_by_object_id_conditions_like_cpp(
            &store,
            1,
            2,
            None,
            |_, _| false,
        )
    );
    assert!(conditions_for_area_trigger_like_cpp(&store, 1, false).is_none());
}
