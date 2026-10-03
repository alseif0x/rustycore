//! SpellInfo.cpp:1325-1526 constructor projection, before corrections/learning.
//! The seed is deliberately not an executable SpellInfo or Player spell state.
use super::{POWER_SLOTS, SpellInputs};
use std::collections::BTreeSet;
use wow_data::forever_spells::*;
mod effects;
pub use effects::SpellEffectSeed;

/// Constructor-scoped scalar storage; owned definitions may later correct it.
/// Defaults are source defaults; signed
/// DB2 flags/IDs retain C++ assignment bits, rather than clamping or narrowing.
/// Returned through a shared reference, not a public mutable catalog field.
#[derive(Default)]
pub struct SpellConstructorFields {
    pub attributes: [u32; 17],
    pub speed: f32,
    pub launch_delay: f32,
    pub min_duration: f32,
    pub school_mask: u32,
    pub icon_file_data_id: u32,
    pub active_icon_file_data_id: u32,
    pub content_tuning_id: u32,
    pub show_future_spell_player_condition_id: u32,
    pub min_scaling_level: u32,
    pub max_scaling_level: u32,
    pub proc_flags: [u32; 2],
    pub proc_chance: u32,
    pub proc_charges: u32,
    pub proc_cooldown: u32,
    pub stack_amount: u32,
    pub caster_aura_state: u32,
    pub target_aura_state: u32,
    pub exclude_caster_aura_state: u32,
    pub exclude_target_aura_state: u32,
    pub caster_aura_spell: u32,
    pub target_aura_spell: u32,
    pub exclude_caster_aura_spell: u32,
    pub exclude_target_aura_spell: u32,
    pub caster_aura_type: u32,
    pub target_aura_type: u32,
    pub exclude_caster_aura_type: u32,
    pub exclude_target_aura_type: u32,
    pub requires_spell_focus: u32,
    pub facing_caster_flags: u32,
    pub required_areas_id: i32,
    pub category_id: u32,
    pub dispel: u32,
    pub mechanic: u32,
    pub start_recovery_category: u32,
    pub damage_class: u32,
    pub prevention_type: u32,
    pub charge_category_id: u32,
    pub spell_family_name: u32,
    pub spell_family_flags: [u32; 4],
    pub recovery_time: u32,
    pub category_recovery_time: u32,
    pub start_recovery_time: u32,
    pub cooldown_aura_spell_id: u32,
    pub equipped_item_class: i32,
    pub equipped_item_subclass_mask: i32,
    pub equipped_item_inventory_type_mask: i32,
    pub interrupt_flags: u32,
    pub aura_interrupt_flags: [u32; 2],
    pub channel_interrupt_flags: [u32; 2],
    pub max_level: u32,
    pub base_level: u32,
    pub spell_level: u32,
    pub reagents: [i32; 8],
    pub reagent_counts: [i16; 8],
    pub stances: u64,
    pub stances_not: u64,
    pub cone_angle: f32,
    pub width: f32,
    pub targets: u32,
    pub target_creature_type: u32,
    pub max_affected_targets: u32,
    pub max_target_level: u32,
    pub totem_category: [u16; 2],
    pub totems: [i32; 2],
    pub proc_base_ppm: f32,
}

/// Owned derived scalars, borrowed immutable dependencies. No raw record clones,
/// no parent pointer cycle and no implicit-target/immunity/correction claim.
pub struct SpellConstructorSeed<'a> {
    spell: u32,
    difficulty: i16,
    fields: SpellConstructorFields,
    name: &'a SpellNameRecord,
    effects: Vec<SpellEffectSeed<'a>>,
    cast_time: Option<&'a SpellCastTimesRecord>,
    duration: Option<&'a SpellDurationRecord>,
    range: Option<&'a SpellRangeRecord>,
    ppm_modifiers: Vec<&'a SpellProcsPerMinuteModRecord>,
    powers: [Option<&'a SpellPowerRecord>; POWER_SLOTS],
    reagent_currencies: Vec<&'a SpellReagentsCurrencyRecord>,
    visuals: Vec<&'a SpellXSpellVisualRecord>,
    labels: BTreeSet<u32>,
    // Source Milliseconds is signed: negative thresholds are not normalized.
    empower_thresholds_ms: Vec<i64>,
}

impl<'a> SpellInputs<'a> {
    pub fn constructor_seed(&self) -> SpellConstructorSeed<'a> {
        let mut fields = SpellConstructorFields {
            required_areas_id: -1,
            equipped_item_class: -1,
            ..Default::default()
        };
        if let Some(row) = self.misc() {
            fields.attributes = row.attributes.map(|word| word as u32);
            fields.speed = row.speed;
            fields.launch_delay = row.launch_delay;
            fields.min_duration = row.min_duration;
            fields.school_mask = u32::from(row.school_mask);
            fields.icon_file_data_id = row.spell_icon_file_data_id as u32;
            fields.active_icon_file_data_id = row.active_icon_file_data_id as u32;
            fields.content_tuning_id = row.content_tuning_id as u32;
            fields.show_future_spell_player_condition_id =
                row.show_future_spell_player_condition_id as u32;
        }
        if let Some(row) = self.scaling() {
            fields.min_scaling_level = row.min_scaling_level;
            fields.max_scaling_level = row.max_scaling_level;
        }
        if let Some(row) = self.aura_options() {
            fields.proc_flags = row.proc_type_mask.map(|word| word as u32);
            fields.proc_chance = u32::from(row.proc_chance);
            fields.proc_charges = row.proc_charges as u32;
            fields.proc_cooldown = row.proc_category_recovery as u32;
            fields.stack_amount = u32::from(row.cumulative_aura);
        }
        if let Some(row) = self.aura_restrictions() {
            fields.caster_aura_state = row.caster_aura_state as u32;
            fields.target_aura_state = row.target_aura_state as u32;
            fields.exclude_caster_aura_state = row.exclude_caster_aura_state as u32;
            fields.exclude_target_aura_state = row.exclude_target_aura_state as u32;
            fields.caster_aura_spell = row.caster_aura_spell as u32;
            fields.target_aura_spell = row.target_aura_spell as u32;
            fields.exclude_caster_aura_spell = row.exclude_caster_aura_spell as u32;
            fields.exclude_target_aura_spell = row.exclude_target_aura_spell as u32;
            fields.caster_aura_type = row.caster_aura_type as u32;
            fields.target_aura_type = row.target_aura_type as u32;
            fields.exclude_caster_aura_type = row.exclude_caster_aura_type as u32;
            fields.exclude_target_aura_type = row.exclude_target_aura_type as u32;
        }
        if let Some(row) = self.casting_requirements() {
            fields.requires_spell_focus = u32::from(row.requires_spell_focus);
            fields.facing_caster_flags = row.facing_caster_flags as u32;
            fields.required_areas_id = i32::from(row.required_areas_id);
        }
        if let Some(row) = self.categories() {
            fields.category_id = row.category as u32;
            fields.dispel = row.dispel_type as u32;
            fields.mechanic = row.mechanic as u32;
            fields.start_recovery_category = row.start_recovery_category as u32;
            fields.damage_class = row.defense_type as u32;
            fields.prevention_type = row.prevention_type as u32;
            fields.charge_category_id = row.charge_category as u32;
        }
        if let Some(row) = self.class_options() {
            fields.spell_family_name = row.spell_class_set as u32;
            fields.spell_family_flags = row.spell_class_mask;
        }
        if let Some(row) = self.cooldowns() {
            fields.recovery_time = row.recovery_time as u32;
            fields.category_recovery_time = row.category_recovery_time as u32;
            fields.start_recovery_time = row.start_recovery_time as u32;
            fields.cooldown_aura_spell_id = row.aura_spell_id as u32;
        }
        if let Some(row) = self.equipped_items() {
            fields.equipped_item_class = row.equipped_item_class;
            fields.equipped_item_subclass_mask = row.equipped_item_subclass;
            fields.equipped_item_inventory_type_mask = row.equipped_item_inv_types;
        }
        if let Some(row) = self.interrupts() {
            fields.interrupt_flags = row.interrupt_flags as u32;
            fields.aura_interrupt_flags = row.aura_interrupt_flags.map(|word| word as u32);
            fields.channel_interrupt_flags = row.channel_interrupt_flags.map(|word| word as u32);
        }
        if let Some(row) = self.levels() {
            fields.max_level = row.max_level as u32;
            fields.base_level = row.base_level as u32;
            fields.spell_level = row.spell_level as u32;
        }
        if let Some(row) = self.reagents() {
            fields.reagents = row.reagent;
            fields.reagent_counts = row.reagent_count;
        }
        if let Some(row) = self.shapeshift() {
            fields.stances = pair64(row.shapeshift_mask);
            fields.stances_not = pair64(row.shapeshift_exclude);
        }
        if let Some(row) = self.target_restrictions() {
            fields.cone_angle = row.cone_degrees;
            fields.width = row.width;
            fields.targets = row.targets as u32;
            fields.target_creature_type = row.target_creature_type as u32;
            fields.max_affected_targets = u32::from(row.max_targets);
            fields.max_target_level = row.max_target_level;
        }
        if let Some(row) = self.totems() {
            fields.totem_category = row.required_totem_category_id;
            fields.totems = row.totem;
        }
        let (cast_time, duration, range) = self.misc().map_or((None, None, None), |row| {
            (
                self.catalog
                    .spell_cast_times(u32::from(row.casting_time_index)),
                self.catalog.spell_duration(u32::from(row.duration_index)),
                self.catalog.spell_range(u32::from(row.range_index)),
            )
        });
        let mut ppm_modifiers = Vec::new();
        if let Some(ppm) = self.aura_options().and_then(|row| {
            self.catalog
                .spell_procs_per_minute(u32::from(row.spell_procs_per_minute_id))
        }) {
            fields.proc_base_ppm = ppm.base_proc_rate;
            // DB2Stores.cpp:1562-1563,3082-3088: ascending ID, no sorting
            // by modifier type and no orphan modifier use if PPM is absent.
            ppm_modifiers.extend(self.ppm_modifiers_for(ppm.id));
        }
        let slots = self.effect_slots();
        let effect_count = slots
            .iter()
            .rposition(Option::is_some)
            .map_or(0, |index| index + 1);
        let effects = slots[..effect_count]
            .iter()
            .enumerate()
            .map(|(index, row)| effects::seed(self.catalog, index as u32, *row))
            .collect();
        SpellConstructorSeed {
            spell: self.spell_id(),
            difficulty: self.difficulty(),
            fields,
            name: self.name(),
            effects,
            cast_time,
            duration,
            range,
            ppm_modifiers,
            powers: self.power_slots(),
            reagent_currencies: self.reagent_currencies().collect(),
            visuals: self.visuals().collect(),
            labels: self.labels().map(|row| row.label_id).collect(),
            empower_thresholds_ms: self
                .empower_stages()
                .map(|row| i64::from(row.duration_ms))
                .collect(),
        }
    }
}

fn pair64(words: [i32; 2]) -> u64 {
    u64::from(words[0] as u32) | (u64::from(words[1] as u32) << 32)
}

impl<'a> SpellConstructorSeed<'a> {
    pub fn spell_id(&self) -> u32 {
        self.spell
    }
    pub fn difficulty(&self) -> i16 {
        self.difficulty
    }
    pub fn fields(&self) -> &SpellConstructorFields {
        &self.fields
    }
    pub fn name(&self) -> &'a SpellNameRecord {
        self.name
    }
    pub fn effects(&self) -> &[SpellEffectSeed<'a>] {
        &self.effects
    }
    pub fn cast_time(&self) -> Option<&'a SpellCastTimesRecord> {
        self.cast_time
    }
    pub fn duration(&self) -> Option<&'a SpellDurationRecord> {
        self.duration
    }
    pub fn range(&self) -> Option<&'a SpellRangeRecord> {
        self.range
    }
    pub fn ppm_modifiers(&self) -> &[&'a SpellProcsPerMinuteModRecord] {
        &self.ppm_modifiers
    }
    pub fn powers(&self) -> &[Option<&'a SpellPowerRecord>; POWER_SLOTS] {
        &self.powers
    }
    pub fn reagent_currencies(&self) -> &[&'a SpellReagentsCurrencyRecord] {
        &self.reagent_currencies
    }
    pub fn visuals(&self) -> &[&'a SpellXSpellVisualRecord] {
        &self.visuals
    }
    pub fn labels(&self) -> &BTreeSet<u32> {
        &self.labels
    }
    pub fn empower_thresholds_ms(&self) -> &[i64] {
        &self.empower_thresholds_ms
    }
}

impl SpellConstructorSeed<'_> {
    pub(super) fn into_owned_definition(self) -> super::definitions::Definition {
        super::definitions::Definition::from_constructor_parts(
            self.spell,
            self.fields,
            self.effects
                .into_iter()
                .map(|effect| effect.into_owned_values())
                .collect(),
            self.cast_time.map(|row| row.id),
            self.duration.map(|row| row.id),
            self.range.map(|row| row.id),
            self.ppm_modifiers.into_iter().map(|row| row.id).collect(),
            self.powers.map(|row| row.map(|row| row.id)),
            self.reagent_currencies
                .into_iter()
                .map(|row| row.id)
                .collect(),
            self.visuals.into_iter().map(|row| row.id).collect(),
            self.labels,
            self.empower_thresholds_ms,
        )
    }
}
