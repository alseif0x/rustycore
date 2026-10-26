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
    state.equipment_sets.mark_loaded_like_cpp();
    p
}

fn all() -> PlayerSavedGroupsLikeCpp {
    PlayerSavedGroupsLikeCpp {
        spells: true,
        fallback_spells: true,
        skills: true,
        equipment: true,
        reputations: true,
        inventory_items: true,
        currencies: true,
    }
}

fn dirty_item(player: &mut Player, guid: ObjectGuid) {
    let mut item = Item::new(0);
    item.object_mut().create(guid);
    item.object_mut().set_entry(6948);
    item.set_owner_guid(player.guid());
    item.set_count(1);
    item.set_durability(20);
    item.force_state(ItemUpdateState::New);
    player
        .inventory_runtime_mut_like_cpp()
        .store_item_object_like_cpp(item);
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
fn save_ack_promotes_only_the_exact_saving_resurrection_continuation() {
    use crate::{
        PlayerResurrectionContinuation, PlayerResurrectionOperation, PlayerResurrectionPhase,
    };

    let continuation = PlayerResurrectionContinuation {
        operation: PlayerResurrectionOperation::ReclaimCorpse {
            restore_percent: 0.5,
        },
        phase: PlayerResurrectionPhase::SavingPlayer,
    };
    let mut p = player();
    p.resurrection_state_mut_like_cpp().delayed_after_teleport = Some(continuation);
    let receipt = p.capture_save_acknowledgement_like_cpp();
    p.acknowledge_saved_projection_like_cpp(receipt, all());
    assert_eq!(
        p.resurrection_state_like_cpp().delayed_after_teleport,
        Some(PlayerResurrectionContinuation {
            phase: PlayerResurrectionPhase::PlayerSaveApplied,
            ..continuation
        }),
        "the receipt confirms the save without clearing post-save work"
    );

    p.resurrection_state_mut_like_cpp().delayed_after_teleport = Some(continuation);
    let stale = p.capture_save_acknowledgement_like_cpp();
    p.resurrection_state_mut_like_cpp()
        .delayed_after_teleport
        .as_mut()
        .unwrap()
        .phase = PlayerResurrectionPhase::RecoveryCorpseBonesInFlight;
    p.acknowledge_saved_projection_like_cpp(stale, all());
    assert_eq!(
        p.resurrection_state_like_cpp()
            .delayed_after_teleport
            .unwrap()
            .phase,
        PlayerResurrectionPhase::RecoveryCorpseBonesInFlight,
        "a stale receipt cannot advance a changed operation phase"
    );

    p.resurrection_state_mut_like_cpp().delayed_after_teleport = None;
    let ordinary_save = p.capture_save_acknowledgement_like_cpp();
    p.acknowledge_saved_projection_like_cpp(ordinary_save, all());
    assert!(
        p.resurrection_state_like_cpp()
            .delayed_after_teleport
            .is_none()
    );
}

#[test]
fn applied_save_with_missing_resource_groups_keeps_costs_dirty_and_recovery_saving() {
    use crate::{
        PlayerCurrency, PlayerCurrencyState, PlayerResurrectionContinuation,
        PlayerResurrectionOperation, PlayerResurrectionPhase,
    };

    let mut p = player();
    let item_guid = ObjectGuid::create_item(1, 99);
    dirty_item(&mut p, item_guid);
    p.gameplay_state_mut().currencies.insert(
        395,
        PlayerCurrency {
            state: PlayerCurrencyState::Changed,
            quantity: 7,
            weekly_quantity: 0,
            tracked_quantity: 0,
            increased_cap_quantity: 0,
            earned_quantity: 0,
            flags: 0,
        },
    );
    let continuation = PlayerResurrectionContinuation {
        operation: PlayerResurrectionOperation::ReclaimCorpse {
            restore_percent: 0.5,
        },
        phase: PlayerResurrectionPhase::SavingPlayer,
    };
    p.resurrection_state_mut_like_cpp().delayed_after_teleport = Some(continuation);
    let receipt = p.capture_save_acknowledgement_with_inventory_like_cpp(&[item_guid], &[395]);

    p.acknowledge_saved_projection_like_cpp(
        receipt,
        PlayerSavedGroupsLikeCpp {
            inventory_items: true,
            currencies: false,
            ..all()
        },
    );

    assert_eq!(
        p.inventory_runtime_like_cpp().item_objects()[&item_guid].update_state(),
        ItemUpdateState::Unchanged,
        "the independently committed inventory group acknowledges its captured item"
    );
    assert_eq!(
        p.gameplay_state().currencies[&395].state,
        PlayerCurrencyState::Changed,
        "an absent committed currency group cannot acknowledge the native currency cost"
    );
    assert_eq!(
        p.resurrection_state_like_cpp().delayed_after_teleport,
        Some(continuation),
        "Applied with an incomplete resource receipt must remain in SavingPlayer"
    );
}

#[test]
fn item_save_ack_ignores_wire_masks_and_retains_new_persistent_values() {
    let mut p = player();
    let guid = ObjectGuid::create_item(1, 101);
    dirty_item(&mut p, guid);
    p.inventory_runtime_mut_like_cpp()
        .item_objects_mut()
        .get_mut(&guid)
        .unwrap()
        .force_state(ItemUpdateState::Changed);
    let receipt = p.capture_save_acknowledgement_with_inventory_like_cpp(&[guid], &[]);

    let item = p
        .inventory_runtime_mut_like_cpp()
        .item_objects_mut()
        .get_mut(&guid)
        .unwrap();
    item.set_durability(item.data().durability);
    p.acknowledge_saved_projection_like_cpp(
        receipt,
        PlayerSavedGroupsLikeCpp {
            inventory_items: true,
            ..Default::default()
        },
    );
    assert_eq!(
        p.inventory_runtime_like_cpp().item_objects()[&guid].update_state(),
        ItemUpdateState::Unchanged,
        "the captured persistent row is durable despite a wire-only update mask"
    );

    let receipt = p.capture_save_acknowledgement_with_inventory_like_cpp(&[guid], &[]);
    p.inventory_runtime_mut_like_cpp()
        .item_objects_mut()
        .get_mut(&guid)
        .unwrap()
        .set_durability(19);
    p.acknowledge_saved_projection_like_cpp(
        receipt,
        PlayerSavedGroupsLikeCpp {
            inventory_items: true,
            ..Default::default()
        },
    );
    assert_eq!(
        p.inventory_runtime_like_cpp().item_objects()[&guid].update_state(),
        ItemUpdateState::Changed,
        "a persistent write made during the save remains dirty"
    );
}

#[test]
fn consumed_new_item_survives_inflight_insert_until_delete_acknowledges() {
    let mut p = player();
    let item_guid = ObjectGuid::create_item(1, 6949);
    let slot = INVENTORY_SLOT_ITEM_START;
    p.set_inventory_slot_count(2);
    p.inventory_runtime_mut_like_cpp()
        .set_equipment_inventory_authority_complete_like_cpp(true);
    p.store_top_level_item(slot, item_guid).unwrap();

    let mut item = Item::new(0);
    item.object_mut().create(item_guid);
    item.object_mut().set_entry(6948);
    item.set_owner_guid(p.guid());
    item.set_contained_in(p.guid());
    item.set_slot(slot);
    item.set_count(1);
    item.force_state(ItemUpdateState::New);
    p.inventory_runtime_mut_like_cpp()
        .store_item_object_like_cpp(item);

    let in_flight_insert =
        p.capture_save_acknowledgement_with_inventory_like_cpp(&[item_guid], &[]);
    let debit = p
        .plan_spell_cast_reagent_step_like_cpp(6948, 1)
        .expect("the new physical stack can be consumed");
    p.apply_spell_cast_reagent_step_like_cpp(debit)
        .expect("canonical stack is still at the planned position");
    assert_eq!(
        p.inventory_runtime_like_cpp().item_objects()[&item_guid].update_state(),
        ItemUpdateState::Removed,
        "consumption retains a delete tombstone even while the insert save is pending"
    );

    p.acknowledge_saved_projection_like_cpp(
        in_flight_insert,
        PlayerSavedGroupsLikeCpp {
            inventory_items: true,
            ..Default::default()
        },
    );
    assert_eq!(
        p.inventory_runtime_like_cpp().item_objects()[&item_guid].update_state(),
        ItemUpdateState::Removed,
        "the in-flight NEW receipt cannot retire a newer removal"
    );

    let confirmed_delete =
        p.capture_save_acknowledgement_with_inventory_like_cpp(&[item_guid], &[]);
    p.acknowledge_saved_projection_like_cpp(
        confirmed_delete,
        PlayerSavedGroupsLikeCpp {
            inventory_items: true,
            ..Default::default()
        },
    );
    assert!(
        !p.inventory_runtime_like_cpp()
            .item_objects()
            .contains_key(&item_guid),
        "only the matching DELETE receipt retires the tombstone"
    );
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
    let reputation = p.reputation_mut_like_cpp();
    reputation.insert_faction_like_cpp(PlayerFactionStateLikeCpp {
        faction_id: 1,
        reputation_list_id: 1,
        need_save: true,
        standing: 10,
        ..Default::default()
    });
    reputation.insert_faction_like_cpp(PlayerFactionStateLikeCpp {
        faction_id: 2,
        reputation_list_id: 2,
        need_save: true,
        standing: 20,
        ..Default::default()
    });
    let receipt = p.capture_save_acknowledgement_like_cpp();
    p.gameplay_state_mut()
        .spells
        .fallback_rows
        .insert(20, spell(20, PlayerSpellLoadState::New));
    p.reputation_mut_like_cpp()
        .faction_mut_like_cpp(1)
        .unwrap()
        .need_send = true;
    p.reputation_mut_like_cpp()
        .faction_mut_like_cpp(2)
        .unwrap()
        .standing = 30;
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
    let reputation = p.reputation_like_cpp();
    assert!(!reputation.faction_like_cpp(1).unwrap().need_save);
    assert!(reputation.faction_like_cpp(1).unwrap().need_send);
    assert!(reputation.faction_like_cpp(2).unwrap().need_save);
}

#[test]
fn save_ack_rebases_equipment_new_edit_and_new_remove() {
    use PlayerEquipmentSetUpdateStateLikeCpp::*;
    for removed in [false, true] {
        let mut p = player();
        let mut stored = PlayerEquipmentSetLikeCpp::equipment(1, 0, New);
        stored.guid = 1;
        p.gameplay_state_mut()
            .equipment_sets
            .install_loaded_set_like_cpp(stored.clone());
        let receipt = p.capture_save_acknowledgement_like_cpp();
        if removed {
            assert!(p.gameplay_state_mut().equipment_sets.delete_set_like_cpp(1));
        } else {
            let mut edited = stored.clone();
            edited.set_name = "later".into();
            assert!(
                p.gameplay_state_mut()
                    .equipment_sets
                    .update_set_like_cpp(edited)
            );
        }
        p.acknowledge_saved_projection_like_cpp(receipt, all());
        let row = p
            .gameplay_state()
            .equipment_sets
            .set_like_cpp(1)
            .expect("stored equipment set");
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
