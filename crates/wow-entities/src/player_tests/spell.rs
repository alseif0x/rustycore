//! Spell scenarios for [`super`].
//!
//! Split out of player_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn native_known_spell_commands_preserve_order_prune_metadata_and_keep_row_state() {
    let mut runtime = PlayerSpellRuntimeState::default();
    runtime.dependent_known_spells = BTreeSet::from([10, 30]);
    runtime.favorite_known_spells = BTreeSet::from([10, 30]);
    runtime.trait_definition_ids = BTreeMap::from([(10, 100), (30, 300)]);
    runtime.removed_known_spells.insert(99);
    runtime.replace_known_spell_ids_like_cpp(vec![20, 10, 20, -1]);
    assert_eq!(runtime.known_spells, vec![20, 10, 20, -1]);
    assert_eq!(runtime.dependent_known_spells, BTreeSet::from([10]));
    assert_eq!(runtime.favorite_known_spells, BTreeSet::from([10]));
    assert_eq!(runtime.trait_definition_ids, BTreeMap::from([(10, 100)]));
    assert!(runtime.removed_known_spells.is_empty());
    runtime.learn_known_spell_id_like_cpp(20);
    runtime.removed_known_spells.insert(0);
    runtime.learn_known_spell_id_like_cpp(0);
    assert_eq!(runtime.known_spells, vec![20, 10, 20, -1, 0]);
    assert!(runtime.removed_known_spells.is_empty());
    runtime.rows.insert(
        10,
        PlayerKnownSpellRecord {
            spell_id: 10,
            state: PlayerSpellLoadState::New,
            active: true,
            disabled: false,
            favorite: true,
            dependent: false,
        },
    );
    runtime.mark_known_spell_dependent_like_cpp(10);
    assert!(!runtime.rows[&10].dependent);
    runtime.rows_complete = true;
    runtime.mark_known_spell_dependent_like_cpp(10);
    assert!(runtime.rows[&10].dependent && !runtime.rows[&10].favorite);
    assert_eq!(runtime.rows[&10].state, PlayerSpellLoadState::New);
}
#[test]
fn native_spell_save_finalization_preserves_temporary_rows_and_source_proofs() {
    let mut runtime = PlayerSpellRuntimeState::default();
    for (id, state) in [
        (10, PlayerSpellLoadState::New),
        (20, PlayerSpellLoadState::Removed),
        (30, PlayerSpellLoadState::Temporary),
    ] {
        runtime.rows.insert(
            id,
            PlayerKnownSpellRecord {
                spell_id: id,
                state,
                active: false,
                disabled: id == 30,
                favorite: true,
                dependent: true,
            },
        );
        runtime.trait_definition_ids.insert(id, id + 100);
    }
    runtime.rows_complete = true;
    runtime.fallback_rows = runtime.rows.clone();
    let fallback = runtime.fallback_rows.clone();
    runtime.mark_spell_rows_saved_like_cpp();
    assert_eq!(runtime.rows[&10].state, PlayerSpellLoadState::Unchanged);
    assert!(!runtime.rows.contains_key(&20));
    assert_eq!(runtime.rows[&30].state, PlayerSpellLoadState::Temporary);
    assert_eq!(runtime.known_spells, vec![10]);
    assert_eq!(runtime.favorite_known_spells, BTreeSet::from([10, 30]));
    assert_eq!(runtime.dependent_known_spells, BTreeSet::from([10, 30]));
    assert_eq!(
        runtime.trait_definition_ids,
        BTreeMap::from([(10, 110), (30, 130)])
    );
    assert_eq!(runtime.fallback_rows, fallback);
    assert!(runtime.rows_complete);
}
#[test]
fn native_loaded_spell_reconciliation_preserves_pending_grants_and_unrelated_state() {
    let mut runtime = PlayerSpellRuntimeState::default();
    let pending = PlayerKnownSpellRecord {
        spell_id: 10,
        state: PlayerSpellLoadState::New,
        active: true,
        disabled: false,
        favorite: false,
        dependent: true,
    };
    runtime.fallback_rows.insert(10, pending.clone());
    runtime.known_spells = vec![99];
    runtime.trait_definition_ids_complete = true;
    let fallback_address = runtime.fallback_rows.get(&10).unwrap() as *const _;
    runtime.replace_loaded_spell_rows_like_cpp(
        BTreeMap::from([(
            10,
            PlayerKnownSpellRecord {
                active: false,
                disabled: true,
                favorite: true,
                dependent: false,
                state: PlayerSpellLoadState::Unchanged,
                ..pending.clone()
            },
        )]),
        false,
    );
    let loaded = runtime.rows.get(&10).unwrap();
    assert!(!loaded.active && !loaded.disabled && loaded.favorite && loaded.dependent);
    assert_eq!(loaded.state, PlayerSpellLoadState::Changed);
    assert!(runtime.rows_loaded && !runtime.rows_complete);
    assert_eq!(runtime.known_spells, vec![99]);
    assert!(runtime.trait_definition_ids_complete);
    assert_eq!(
        runtime.fallback_rows.get(&10).unwrap() as *const _,
        fallback_address
    );
    runtime.replace_loaded_spell_rows_like_cpp(BTreeMap::new(), true);
    assert_eq!(runtime.rows.get(&10), Some(&pending));
    assert!(runtime.rows_loaded && runtime.rows_complete);
}
#[test]
fn native_spell_metadata_keeps_cpp_override_set_and_fail_closed_trait_semantics() {
    let mut state = PlayerSpellRuntimeState::default();
    assert!(state.replace_complete_trait_definition_ids_like_cpp(vec![(10, 20)]));
    assert!(!state.replace_complete_trait_definition_ids_like_cpp(vec![(30, 40), (30, 40)]));
    assert!(state.trait_definition_ids.is_empty());
    assert!(!state.trait_definition_ids_complete);
    assert!(state.replace_complete_trait_definition_ids_like_cpp(vec![]));
    assert!(state.trait_definition_ids_complete);
    state.add_override_spell_like_cpp(50, 60);
    state.add_override_spell_like_cpp(50, 60);
    state.add_override_spell_like_cpp(50, 70);
    state.add_override_spell_like_cpp(-1, 70);
    assert_eq!(
        state.override_spells,
        BTreeMap::from([(50, BTreeSet::from([60, 70]))])
    );
    state.remove_override_spell_like_cpp(50, 60);
    assert_eq!(
        state.override_spells,
        BTreeMap::from([(50, BTreeSet::from([70]))])
    );
    state.remove_override_spell_like_cpp(50, 70);
    state.remove_override_spell_like_cpp(50, 70);
    state.override_spells.insert(0, BTreeSet::new());
    state.remove_override_spell_like_cpp(0, 99);
    assert!(state.override_spells.is_empty());
}
#[test]
fn apply_enchantment_effect_actions_match_cpp_deferred_noop_and_spell_cases() {
    let player = Player::new(None, false);
    let mut item = item_with_guid_entry(12491, 7464);
    item.set_slot(EQUIPMENT_SLOT_CHEST);

    let effects = [
        ApplyEnchantmentEffectRef::known(ItemEnchantmentType::None, 0, 0),
        ApplyEnchantmentEffectRef::known(ItemEnchantmentType::CombatSpell, 0, 0),
        ApplyEnchantmentEffectRef::known(ItemEnchantmentType::UseSpell, 0, 0),
        ApplyEnchantmentEffectRef::known(ItemEnchantmentType::EquipSpell, 0, 1234),
        ApplyEnchantmentEffectRef::known(ItemEnchantmentType::EquipSpell, 0, 0),
        ApplyEnchantmentEffectRef::known(ItemEnchantmentType::PrismaticSocket, 0, 0),
        ApplyEnchantmentEffectRef::unknown(99, 0, 0),
    ];

    assert_eq!(
        player.apply_enchantment_effect_actions(
            &item,
            None,
            EnchantmentSlot::EnhancementTemporary,
            true,
            &effects,
        ),
        vec![
            ApplyEnchantmentEffectAction::Noop,
            ApplyEnchantmentEffectAction::DeferredCombatSpell,
            ApplyEnchantmentEffectAction::DeferredUseSpell,
            ApplyEnchantmentEffectAction::CastEquipSpell {
                spell_id: 1234,
                item_guid: item.object().guid(),
            },
            ApplyEnchantmentEffectAction::Noop,
            ApplyEnchantmentEffectAction::Noop,
            ApplyEnchantmentEffectAction::Unknown { effect_type: 99 },
        ]
    );
    assert_eq!(
        player.apply_enchantment_effect_actions(
            &item,
            None,
            EnchantmentSlot::EnhancementTemporary,
            false,
            &[ApplyEnchantmentEffectRef::known(
                ItemEnchantmentType::EquipSpell,
                0,
                1234,
            )],
        ),
        vec![ApplyEnchantmentEffectAction::RemoveEquipSpellAura {
            spell_id: 1234,
            item_guid: item.object().guid(),
        }]
    );
}
