use std::collections::BTreeMap;

use wow_core::ObjectGuid;
use wow_data::{
    EffectiveCoreSpellDb2StoresLikeCpp, ServersideSpellEffectKeyLikeCpp,
    ServersideSpellEffectLikeCpp, ServersideSpellInfoLikeCpp, ServersideSpellRowLikeCpp,
    ServersideSpellStoreLikeCpp, SpellCastTimesStore, SpellCastingRequirementsStore,
    SpellCategoriesEntry, SpellCategoriesStore, SpellClassOptionsEntry, SpellClassOptionsStore,
    SpellCooldownsStore, SpellEffectDb2Entry, SpellEffectDb2Store, SpellInterruptsStore,
    SpellMiscStore, SpellPowerDifficultyStore, SpellPowerStore, SpellShapeshiftStore, SpellStore,
};
use wow_entities::{
    AuraApplicationLikeCpp, Player, PlayerKnownSpellRecord, PlayerSpellLoadState,
    PlayerSpellRuntimeState,
};
use wow_world_test_support::spell_catalog::canonical_spell_store;

pub const PASSIVE: i32 = 20_608;
pub const ACTIVE: u32 = 21_169;
pub const TWISTING_NETHER: u32 = 23_701;
pub const DUMMY_AURA: i32 = 4;
pub const IGNORE_COOLDOWN_AURA: i32 = 383;
pub const APPLY_AURA: u32 = wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA;

pub fn player_with_rows(
    rows: impl IntoIterator<Item = PlayerKnownSpellRecord>,
    complete: bool,
) -> Player {
    let mut player = Player::new(None, false);
    let rows = rows
        .into_iter()
        .map(|row| (row.spell_id, row))
        .collect::<BTreeMap<_, _>>();
    let mut runtime = PlayerSpellRuntimeState::default();
    runtime.replace_rows_like_cpp(rows, complete);
    player.replace_spell_runtime_like_cpp(runtime);
    player
        .unit_mut()
        .subsystems_mut()
        .auras
        .set_persisted_player_aura_authority_complete_like_cpp(true);
    player
}

pub fn spell_row(
    spell_id: i32,
    state: PlayerSpellLoadState,
    active: bool,
    disabled: bool,
) -> PlayerKnownSpellRecord {
    PlayerKnownSpellRecord {
        spell_id,
        state,
        active,
        disabled,
        favorite: false,
        dependent: false,
    }
}

pub fn player_with_passive(active: bool) -> Player {
    player_with_rows(
        [spell_row(
            PASSIVE,
            PlayerSpellLoadState::Changed,
            active,
            false,
        )],
        false,
    )
}

pub fn add_aura(player: &mut Player, spell_id: i32, difficulty_id: u8, slot: u8, mask: u32) {
    player
        .unit_mut()
        .subsystems_mut()
        .auras
        .insert_runtime_application_like_cpp(AuraApplicationLikeCpp {
            spell_id,
            difficulty_id,
            caster_guid: ObjectGuid::EMPTY,
            slot,
            duration_total: -1,
            duration_remaining: -1,
            stack_count: 1,
            aura_flags: 0,
            effect_mask: mask,
            aura_interrupt_flags: 0,
            aura_interrupt_flags2: 0,
            represented_effect: None,
            represented_amount: 0,
            represented_effect_amounts: Vec::new(),
            represented_misc_value: None,
            represented_multiplier: 1.0,
            applied_at: std::time::Instant::now(),
        });
}

pub fn set_history_loaded(player: &mut Player, loaded: bool) {
    player
        .unit_mut()
        .subsystems_mut()
        .spells
        .history
        .cooldowns_loaded = loaded;
}

pub fn insert_direct_cooldown(player: &mut Player, on_hold: bool, end_ms: u64) {
    let history = &mut player.unit_mut().subsystems_mut().spells.history;
    history.cooldowns.insert(
        ACTIVE,
        wow_entities::SpellCooldown {
            spell_id: ACTIVE,
            item_id: 0,
            cooldown_end_ms: end_ms,
            category_id: 0,
            category_end_ms: 0,
            on_hold,
        },
    );
}

pub fn regular_spell_store(
    keys: impl IntoIterator<Item = (u32, u8)>,
    effects: impl IntoIterator<Item = SpellEffectDb2Entry>,
    categories: impl IntoIterator<Item = SpellCategoriesEntry>,
) -> SpellStore {
    canonical_spell_store(keys).hydrate_effective_core_db2_like_cpp(
        EffectiveCoreSpellDb2StoresLikeCpp::new(
            SpellCategoriesStore::from_entries(categories),
            SpellMiscStore::from_entries([]),
            SpellEffectDb2Store::from_entries(effects),
            SpellShapeshiftStore::from_entries([]),
            SpellInterruptsStore::from_entries([]),
            SpellCastTimesStore::from_entries([]),
            SpellCooldownsStore::from_entries([]),
            SpellCastingRequirementsStore::from_entries([]),
            SpellPowerStore::from_entries([]),
            SpellPowerDifficultyStore::from_entries([]),
        ),
    )
}

pub fn regular_effect(
    record_id: u32,
    spell_id: u32,
    difficulty: u8,
    index: i32,
    effect_type: u32,
    aura_type: i16,
    class_mask: [u32; 4],
) -> SpellEffectDb2Entry {
    SpellEffectDb2Entry {
        id: record_id,
        difficulty_id: i32::from(difficulty),
        effect_index: index,
        effect: effect_type,
        effect_amplitude: 0.0,
        effect_attributes: 0,
        effect_aura: aura_type,
        effect_aura_period: 0,
        effect_base_points: 0,
        effect_bonus_coefficient: 0.0,
        effect_chain_amplitude: 0.0,
        effect_chain_targets: 0,
        effect_die_sides: 0,
        effect_item_type: 0,
        effect_mechanic: 0,
        effect_points_per_resource: 0.0,
        effect_pos_facing: 0.0,
        effect_real_points_per_level: 0.0,
        effect_trigger_spell: 0,
        bonus_coefficient_from_ap: 0.0,
        pvp_multiplier: 0.0,
        coefficient: 0.0,
        variance: 0.0,
        resource_coefficient: 0.0,
        group_size_base_points_coefficient: 0.0,
        effect_misc_value: [0; 2],
        effect_radius_index: [0; 2],
        effect_spell_class_mask: class_mask,
        implicit_target: [0; 2],
        spell_id,
    }
}

pub fn category_row(id: u32, spell_id: u32, difficulty: u8, category: i16) -> SpellCategoriesEntry {
    SpellCategoriesEntry {
        id,
        difficulty_id: difficulty,
        category,
        defense_type: 0,
        dispel_type: 0,
        mechanic: 0,
        prevention_type: 0,
        start_recovery_category: 0,
        charge_category: 0,
        spell_id,
    }
}

pub fn class_options(
    entries: impl IntoIterator<Item = (u32, u32, [u32; 4])>,
) -> SpellClassOptionsStore {
    SpellClassOptionsStore::from_entries(entries.into_iter().map(
        |(spell_id, family_name, family_flags)| SpellClassOptionsEntry {
            id: spell_id,
            spell_id: spell_id as i32,
            modal_next_spell: 0,
            spell_class_set: family_name as u8,
            spell_class_mask: family_flags,
        },
    ))
}

pub fn server_effect(
    index: i32,
    effect_type: i32,
    aura_type: i32,
    class_mask: [i32; 4],
) -> ServersideSpellEffectLikeCpp {
    ServersideSpellEffectLikeCpp {
        effect_index: index,
        difficulty_id: 0,
        effect: effect_type,
        effect_aura: aura_type,
        effect_amplitude: 0.0,
        effect_attributes: 0,
        effect_aura_period: 0,
        effect_bonus_coefficient: 0.0,
        effect_chain_amplitude: 0.0,
        effect_chain_targets: 0,
        effect_item_type: 0,
        effect_mechanic: 0,
        effect_points_per_resource: 0.0,
        effect_pos_facing: 0.0,
        effect_real_points_per_level: 0.0,
        effect_trigger_spell: 0,
        bonus_coefficient_from_ap: 0.0,
        pvp_multiplier: 0.0,
        coefficient: 0.0,
        variance: 0.0,
        resource_coefficient: 0.0,
        group_size_base_points_coefficient: 0.0,
        effect_base_points: 0,
        effect_misc_value: [0; 2],
        effect_radius_index: [0; 2],
        effect_spell_class_mask: class_mask,
        implicit_target: [0; 2],
    }
}

pub fn server_row(
    spell_id: u32,
    difficulty: u32,
    category_id: u32,
    family_name: u32,
    family_flags: [u32; 4],
) -> ServersideSpellRowLikeCpp {
    ServersideSpellRowLikeCpp {
        spell_id,
        difficulty_id: difficulty,
        category_id,
        dispel: 0,
        mechanic: 0,
        attributes: 0,
        attributes_ex: [0; 14],
        stances: 0,
        stances_not: 0,
        targets: 0,
        target_creature_type: 0,
        requires_spell_focus: 0,
        facing_caster_flags: 0,
        caster_aura_state: 0,
        target_aura_state: 0,
        exclude_caster_aura_state: 0,
        exclude_target_aura_state: 0,
        caster_aura_spell: 0,
        target_aura_spell: 0,
        exclude_caster_aura_spell: 0,
        exclude_target_aura_spell: 0,
        caster_aura_type: 0,
        target_aura_type: 0,
        exclude_caster_aura_type: 0,
        exclude_target_aura_type: 0,
        casting_time_index: 0,
        recovery_time: 0,
        category_recovery_time: 0,
        start_recovery_category: 0,
        start_recovery_time: 0,
        interrupt_flags: 0,
        aura_interrupt_flags: [0; 2],
        channel_interrupt_flags: [0; 2],
        proc_flags: [0; 2],
        proc_chance: 0,
        proc_charges: 0,
        proc_cooldown: 0,
        proc_base_ppm: 0.0,
        max_level: 0,
        base_level: 0,
        spell_level: 0,
        duration_index: 0,
        range_index: 0,
        speed: 0.0,
        launch_delay: 0.0,
        stack_amount: 0,
        equipped_item_class: 0,
        equipped_item_sub_class_mask: 0,
        equipped_item_inventory_type_mask: 0,
        content_tuning_id: 0,
        spell_name: String::new(),
        cone_angle: 0.0,
        cone_width: 0.0,
        max_target_level: 0,
        max_affected_targets: 0,
        spell_family_name: family_name,
        spell_family_flags: family_flags,
        dmg_class: 0,
        prevention_type: 0,
        area_group_id: 0,
        school_mask: 0,
        charge_category_id: 0,
    }
}

pub fn insert_server_spell(
    store: &mut ServersideSpellStoreLikeCpp,
    spell_id: u32,
    difficulty: u8,
    category_id: u32,
    family_name: u32,
    family_flags: [u32; 4],
    effects: Vec<ServersideSpellEffectLikeCpp>,
) {
    let difficulty = u32::from(difficulty);
    store.spell_infos_by_spell_and_difficulty.insert(
        ServersideSpellEffectKeyLikeCpp {
            spell_id,
            difficulty_id: difficulty,
        },
        ServersideSpellInfoLikeCpp {
            row: server_row(spell_id, difficulty, category_id, family_name, family_flags),
            effects: effects
                .into_iter()
                .map(|mut effect| {
                    effect.difficulty_id = difficulty;
                    effect
                })
                .collect(),
        },
    );
}
