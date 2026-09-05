#![cfg(test)]

use super::*;

fn spell(id: i32, state: PlayerSpellLoadState) -> PlayerKnownSpellRecord {
    PlayerKnownSpellRecord {
        spell_id: id,
        state,
        active: true,
        disabled: false,
        favorite: false,
        dependent: false,
    }
}

fn player() -> Player {
    let mut p = Player::new(Some(1), false);
    let state = p.gameplay_state_mut();
    state.spells.rows_loaded = true;
    state.spells.rows_complete = true;
    state.skills_complete = true;
    state.equipment_sets_loaded = true;
    p
}

fn all() -> PlayerSavedGroupsLikeCpp {
    PlayerSavedGroupsLikeCpp {
        spells: true,
        fallback_spells: true,
        skills: true,
        equipment: true,
        reputations: true,
    }
}

#[test]
fn save_ack_marks_only_rows_captured_and_preserves_later_new_and_changed_spells() {
    use PlayerSpellLoadState::*;
    let mut p = player();
    p.gameplay_state_mut().spells.rows =
        BTreeMap::from([(10, spell(10, New)), (20, spell(20, Changed))]);
    let receipt = p.capture_save_acknowledgement_like_cpp();
    let spells = &mut p.gameplay_state_mut().spells;
    spells.rows.insert(30, spell(30, New));
    spells.rows.get_mut(&20).unwrap().favorite = true;
    p.acknowledge_saved_projection_like_cpp(receipt, all());
    let spells = &p.gameplay_state().spells;
    assert_eq!(spells.rows[&10].state, Unchanged);
    assert_eq!(spells.rows[&20].state, Changed);
    assert!(spells.rows[&20].favorite);
    assert_eq!(spells.rows[&30].state, New);
    assert_eq!(spells.known_spells, [10, 20, 30]);
}

#[test]
fn save_ack_rebases_insert_delete_relearn_without_retrying_duplicate_primary_keys() {
    use PlayerSpellLoadState::*;
    for previous in [New, Changed, Unchanged, Removed] {
        for later in [
            None,
            Some(New),
            Some(Changed),
            Some(Removed),
            Some(Temporary),
        ] {
            let mut p = player();
            p.gameplay_state_mut()
                .spells
                .rows
                .insert(10, spell(10, previous));
            let receipt = p.capture_save_acknowledgement_like_cpp();
            p.gameplay_state_mut().spells.rows.clear();
            if let Some(state) = later {
                let mut row = spell(10, state);
                row.favorite = true; // Distinct from the captured row, even when flags match.
                p.gameplay_state_mut().spells.rows.insert(10, row);
            }
            p.acknowledge_saved_projection_like_cpp(receipt, all());
            let actual = p.gameplay_state().spells.rows.get(&10).map(|row| row.state);
            let expected = match (previous, later) {
                (Removed, None) => None,
                (_, None) => Some(Removed),
                (_, Some(Temporary)) => Some(Temporary),
                (_, Some(Removed)) => Some(Removed),
                (Removed, _) => Some(New),
                _ => Some(Changed),
            };
            assert_eq!(actual, expected, "{previous:?} -> {later:?}");
        }
    }
}

#[test]
fn save_ack_preserves_new_fallback_and_unsaved_reputation_values() {
    let mut p = player();
    p.gameplay_state_mut()
        .spells
        .fallback_rows
        .insert(10, spell(10, PlayerSpellLoadState::New));
    p.gameplay_state_mut().reputations = vec![
        PlayerReputationRecord {
            faction_id: 1,
            need_save: true,
            standing: 10,
            ..Default::default()
        },
        PlayerReputationRecord {
            faction_id: 2,
            need_save: true,
            standing: 20,
            ..Default::default()
        },
    ];
    let receipt = p.capture_save_acknowledgement_like_cpp();
    p.gameplay_state_mut()
        .spells
        .fallback_rows
        .insert(20, spell(20, PlayerSpellLoadState::New));
    p.gameplay_state_mut().reputations[0].need_send = true;
    p.gameplay_state_mut().reputations[1].standing = 30;
    p.acknowledge_saved_projection_like_cpp(receipt, all());
    assert_eq!(
        p.gameplay_state()
            .spells
            .fallback_rows
            .keys()
            .copied()
            .collect::<Vec<_>>(),
        [20]
    );
    assert!(!p.gameplay_state().reputations[0].need_save);
    assert!(p.gameplay_state().reputations[0].need_send);
    assert!(p.gameplay_state().reputations[1].need_save);
}

#[test]
fn save_ack_rebases_equipment_new_edit_and_new_remove() {
    use PlayerEquipmentSetUpdateStateLikeCpp::*;
    for removed in [false, true] {
        let mut p = player();
        p.gameplay_state_mut()
            .equipment_sets
            .insert(1, PlayerEquipmentSetLikeCpp::equipment(1, 0, New));
        let receipt = p.capture_save_acknowledgement_like_cpp();
        if removed {
            p.gameplay_state_mut().equipment_sets.clear();
        } else {
            p.gameplay_state_mut()
                .equipment_sets
                .get_mut(&1)
                .unwrap()
                .set_name = "later".into();
        }
        p.acknowledge_saved_projection_like_cpp(receipt, all());
        let row = &p.gameplay_state().equipment_sets[&1];
        assert_eq!(row.state, if removed { Deleted } else { Changed });
        if !removed {
            assert_eq!(row.set_name, "later");
        }
    }
}

#[test]
fn save_ack_skills_preserves_later_edits_and_only_tombstones_confirmed_deletions() {
    use PlayerSkillLoadState::*;
    let mut p = player();
    p.gameplay_state_mut().skills = (1..=3)
        .map(|id| PlayerSkillRecord {
            skill_line_id: id,
            current_value: 10,
            max_value: 75,
            step: 1,
            profession_slot: -1,
            state: if id == 1 { New } else { Deleted },
        })
        .collect();
    let receipt = p.capture_save_acknowledgement_like_cpp();
    p.gameplay_state_mut().skills[0].current_value = 20;
    p.gameplay_state_mut().skills[2].state = New;
    p.acknowledge_saved_projection_like_cpp(receipt, all());
    let state = p.gameplay_state();
    // The adapter replaces the complete skill set, unlike incremental spell INSERTs.
    assert_eq!(state.skills[0].state, New);
    assert_eq!(state.skills[0].current_value, 20);
    assert_eq!(state.skills[1].state, Unchanged);
    assert_eq!(state.skills[2].state, New);
    assert_eq!(
        state
            .non_durable_skill_tombstones
            .iter()
            .copied()
            .collect::<Vec<_>>(),
        [2]
    );
}

#[test]
fn save_ack_uncommitted_groups_and_different_player_are_inert() {
    let mut p = player();
    p.gameplay_state_mut()
        .spells
        .rows
        .insert(10, spell(10, PlayerSpellLoadState::New));
    let receipt = p.capture_save_acknowledgement_like_cpp();
    p.acknowledge_saved_projection_like_cpp(receipt, PlayerSavedGroupsLikeCpp::default());
    assert_eq!(
        p.gameplay_state().spells.rows[&10].state,
        PlayerSpellLoadState::New
    );
    let receipt = p.capture_save_acknowledgement_like_cpp();
    p.unit_mut()
        .world_mut()
        .object_mut()
        .create(ObjectGuid::create_player(1, 42));
    p.acknowledge_saved_projection_like_cpp(receipt, all());
    assert_eq!(
        p.gameplay_state().spells.rows[&10].state,
        PlayerSpellLoadState::New
    );
}
