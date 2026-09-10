//! Creature template model and loader state definitions, part 1 of 2.
//!
//! Separated from the creature_template.rs root under #664. Behaviour is preserved.

use super::*;

pub const MAX_CREATURE_SPELLS_LIKE_CPP: usize = 8;

pub const MAX_SPELL_SCHOOL_LIKE_CPP: u8 = 7;

pub(super) const CREATURE_GROUND_MOVEMENT_TYPE_MAX_LIKE_CPP: u8 = 3;

pub(super) const CREATURE_FLIGHT_MOVEMENT_TYPE_MAX_LIKE_CPP: u8 = 3;

pub(super) const CREATURE_CHASE_MOVEMENT_TYPE_MAX_LIKE_CPP: u8 = 3;

pub(super) const CREATURE_RANDOM_MOVEMENT_TYPE_MAX_LIKE_CPP: u8 = 3;

pub const DEFAULT_CREATURE_INTERACTION_PAUSE_TIMER_MS_LIKE_CPP: u32 = 180_000;

pub const DEFAULT_INVISIBLE_CREATURE_DISPLAY_ID_LIKE_CPP: u32 = 11_686;

pub(super) const IDLE_MOTION_TYPE_LIKE_CPP: u8 = 0;

pub(super) const WAYPOINT_MOTION_TYPE_LIKE_CPP: u8 = 2;

pub(super) const AFLAG_NOCASTER_LIKE_CPP: u32 = 0x0001;

pub(super) const AFLAG_CANCELABLE_LIKE_CPP: u32 = 0x0002;

pub(super) const AFLAG_POSITIVE_LIKE_CPP: u32 = 0x0100;

pub(super) const AFLAG_PASSIVE_LIKE_CPP: u32 = 0x0200;

pub(super) const MAX_ANIM_TIER_LIKE_CPP: u8 = 5;

pub(super) const MAX_SHEATH_STATE_LIKE_CPP: u8 = 3;

pub(super) const MAX_EXPANSIONS_LIKE_CPP: u8 = 10;

pub(super) fn normalize_creature_ground_movement_type_like_cpp(ground_movement_type: u8) -> u8 {
    if ground_movement_type < CREATURE_GROUND_MOVEMENT_TYPE_MAX_LIKE_CPP {
        ground_movement_type
    } else {
        CreatureGroundMovementType::Run as u8
    }
}

pub(super) fn normalize_creature_flight_movement_type_like_cpp(flight_movement_type: u8) -> u8 {
    if flight_movement_type < CREATURE_FLIGHT_MOVEMENT_TYPE_MAX_LIKE_CPP {
        flight_movement_type
    } else {
        CreatureFlightMovementType::None as u8
    }
}

pub(super) fn normalize_creature_chase_movement_type_like_cpp(chase_movement_type: u8) -> u8 {
    if chase_movement_type < CREATURE_CHASE_MOVEMENT_TYPE_MAX_LIKE_CPP {
        chase_movement_type
    } else {
        CreatureChaseMovementType::Run as u8
    }
}

pub(super) fn normalize_creature_random_movement_type_like_cpp(random_movement_type: u8) -> u8 {
    if random_movement_type < CREATURE_RANDOM_MOVEMENT_TYPE_MAX_LIKE_CPP {
        random_movement_type
    } else {
        CreatureRandomMovementType::Walk as u8
    }
}

pub(super) fn normalize_unit_stand_state_like_cpp(stand_state: u8) -> UnitStandStateType {
    match stand_state {
        1 => UnitStandStateType::Sit,
        2 => UnitStandStateType::SitChair,
        3 => UnitStandStateType::Sleep,
        4 => UnitStandStateType::SitLowChair,
        5 => UnitStandStateType::SitMediumChair,
        6 => UnitStandStateType::SitHighChair,
        7 => UnitStandStateType::Dead,
        8 => UnitStandStateType::Kneel,
        9 => UnitStandStateType::Submerged,
        _ => UnitStandStateType::Stand,
    }
}

pub(super) fn normalize_anim_tier_like_cpp(anim_tier: u8) -> u8 {
    if anim_tier < MAX_ANIM_TIER_LIKE_CPP {
        anim_tier
    } else {
        0
    }
}

pub(super) fn normalize_sheath_state_like_cpp(sheath_state: u8) -> SheathState {
    match if sheath_state < MAX_SHEATH_STATE_LIKE_CPP {
        sheath_state
    } else {
        0
    } {
        1 => SheathState::Melee,
        2 => SheathState::Ranged,
        _ => SheathState::Unarmed,
    }
}

#[derive(Debug, Clone, Default)]
pub struct CreatureTemplateClassificationStoreLikeCpp {
    pub(super) classifications: HashMap<u32, u32>,
}

impl CreatureTemplateClassificationStoreLikeCpp {
    pub fn from_entries(entries: impl IntoIterator<Item = (u32, u32)>) -> Self {
        Self {
            classifications: entries.into_iter().collect(),
        }
    }

    pub fn classification_for_entry(&self, entry: u32) -> Option<u32> {
        self.classifications.get(&entry).copied()
    }

    pub fn len(&self) -> usize {
        self.classifications.len()
    }

    pub fn is_empty(&self) -> bool {
        self.classifications.is_empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CreatureTemplateLifecycleModelLikeCpp {
    pub creature_display_id: u32,
    pub display_scale: f32,
    pub probability: f32,
}

pub trait CreatureModelSelectionRandomLikeCpp {
    fn weighted_model_roll_like_cpp(&mut self, total_weight: f32) -> f32;
    fn other_gender_roll_zero_like_cpp(&mut self) -> bool;
}

impl<R: Rng + ?Sized> CreatureModelSelectionRandomLikeCpp for R {
    fn weighted_model_roll_like_cpp(&mut self, total_weight: f32) -> f32 {
        self.gen_range(0.0..total_weight)
    }

    fn other_gender_roll_zero_like_cpp(&mut self) -> bool {
        self.gen_range(0..=1) == 0
    }
}

impl CreatureTemplateLifecycleModelLikeCpp {
    /// C++ `ObjectMgr::LoadCreatureTemplateModel` normalizes non-positive display scale
    /// before inserting the model into the template model list.
    ///
    /// C++ anchor: `/home/server/woltk-trinity-legacy/src/server/game/Globals/ObjectMgr.cpp:661-662`.
    pub fn normalize_like_cpp(mut self) -> Self {
        if self.display_scale <= 0.0 {
            self.display_scale = 1.0;
        }
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CreatureTemplateLifecycleRecordLikeCpp {
    pub entry: u32,
    pub name: String,
    pub ai_name: String,
    pub script_name: String,
    pub required_expansion: u8,
    pub faction: u32,
    pub npc_flags: u64,
    pub speed_walk: f32,
    pub speed_run: f32,
    pub scale: f32,
    pub classification: u32,
    pub damage_school: u8,
    pub unit_flags: u32,
    pub unit_flags2: u32,
    pub unit_flags3: u32,
    pub creature_type: u32,
    pub family: u32,
    pub trainer_class: u8,
    pub unit_class: u8,
    pub vehicle_id: u32,
    pub movement_type: u8,
    pub ground_movement_type: u8,
    pub swim_allowed: bool,
    pub flight_movement_type: u8,
    pub rooted: bool,
    pub chase_movement_type: u8,
    pub random_movement_type: u8,
    pub interaction_pause_timer_ms: u32,
    pub flags_extra: u32,
    pub string_id: String,
    pub regen_health: bool,
    pub spells: [u32; MAX_CREATURE_SPELLS_LIKE_CPP],
    pub models: Vec<CreatureTemplateLifecycleModelLikeCpp>,
}

#[derive(Debug, Clone, Default)]
pub struct CreatureTemplateSparringStoreLikeCpp {
    pub(super) values: HashMap<u32, Vec<f32>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatureAddonRowLikeCpp {
    pub owner_id: u64,
    pub path_id: u32,
    pub mount: u32,
    pub stand_state: u8,
    pub anim_tier: u8,
    pub vis_flags: u8,
    pub sheath_state: u8,
    pub pvp_flags: u8,
    pub emote: u32,
    pub ai_anim_kit: u16,
    pub movement_anim_kit: u16,
    pub melee_anim_kit: u16,
    pub visibility_distance_type: u8,
    pub auras: String,
}

#[derive(Debug, Clone, Default)]
pub struct CreatureAddonStoreLikeCpp {
    pub(super) spawn_addons: HashMap<u64, CreatureAddonLifecycleRecordLikeCpp>,
    pub(super) template_addons: HashMap<u32, CreatureAddonLifecycleRecordLikeCpp>,
}

impl CreatureAddonStoreLikeCpp {
    pub fn from_rows_like_cpp(
        spawn_rows: impl IntoIterator<Item = CreatureAddonRowLikeCpp>,
        template_rows: impl IntoIterator<Item = CreatureAddonRowLikeCpp>,
        creature_spawn_exists: impl Fn(u64) -> bool,
        creature_template_exists: impl Fn(u32) -> bool,
        mount_display_exists: impl Fn(u32) -> bool,
        emote_exists: impl Fn(u32) -> bool,
        anim_kit_exists: impl Fn(u32) -> bool,
        spell_exists: impl Fn(u32) -> bool,
        spell_has_control_vehicle_aura: impl Fn(u32) -> bool,
        spell_duration_ms: impl Fn(u32) -> i32,
        spell_unit_owned_aura_effect_mask: impl Fn(u32) -> u32,
        spell_addon_aura_flags: impl Fn(u32) -> u32,
    ) -> Self {
        let spawn_addons = spawn_rows
            .into_iter()
            .filter(|row| creature_spawn_exists(row.owner_id))
            .map(|row| {
                (
                    row.owner_id,
                    addon_record_from_row_like_cpp(
                        row,
                        &mount_display_exists,
                        &emote_exists,
                        &anim_kit_exists,
                        &spell_exists,
                        &spell_has_control_vehicle_aura,
                        &spell_duration_ms,
                        &spell_unit_owned_aura_effect_mask,
                        &spell_addon_aura_flags,
                    ),
                )
            })
            .collect();
        let template_addons = template_rows
            .into_iter()
            .filter(|row| creature_template_exists(row.owner_id as u32))
            .map(|row| {
                (
                    row.owner_id as u32,
                    addon_record_from_row_like_cpp(
                        row,
                        &mount_display_exists,
                        &emote_exists,
                        &anim_kit_exists,
                        &spell_exists,
                        &spell_has_control_vehicle_aura,
                        &spell_duration_ms,
                        &spell_unit_owned_aura_effect_mask,
                        &spell_addon_aura_flags,
                    ),
                )
            })
            .collect();

        Self {
            spawn_addons,
            template_addons,
        }
    }

    pub fn from_catalog_rows_with_stores_like_cpp(
        spawn_rows: impl IntoIterator<Item = CreatureAddonRowLikeCpp>,
        template_rows: impl IntoIterator<Item = CreatureAddonRowLikeCpp>,
        template_store: &CreatureTemplateLifecycleStoreLikeCpp,
        creature_spawn_store: &crate::WorldSpawnIdStore,
        display_store: &CreatureDisplayInfoStore,
        emotes_store: &EmotesStore,
        anim_kit_store: &AnimKitStore,
        spell_store: &SpellStore,
        spell_misc_store: &SpellMiscStore,
        spell_duration_store: &SpellDurationStore,
    ) -> Self {
        Self::from_rows_like_cpp(
            spawn_rows,
            template_rows,
            |spawn_id| {
                u32::try_from(spawn_id)
                    .ok()
                    .and_then(|spawn_id| creature_spawn_store.entry_for_guid(spawn_id))
                    .is_some()
            },
            |entry| template_store.get(entry).is_some(),
            |display_id| display_store.get(display_id).is_some(),
            |emote| emotes_store.get(emote).is_some(),
            |anim_kit_id| anim_kit_store.get(anim_kit_id).is_some(),
            |spell_id| {
                i32::try_from(spell_id)
                    .ok()
                    .and_then(|id| spell_store.get(id))
                    .is_some()
            },
            |spell_id| {
                i32::try_from(spell_id)
                    .ok()
                    .and_then(|id| spell_store.get(id))
                    .is_some_and(|spell| {
                        spell.has_aura_like_cpp(aura_types::SPELL_AURA_CONTROL_VEHICLE)
                    })
            },
            |spell_id| {
                let duration_index = spell_misc_store
                    .get_by_spell_id(spell_id)
                    .map(|entry| u32::from(entry.duration_index))
                    .unwrap_or(0);
                spell_duration_ms_like_cpp(duration_index, Some(spell_duration_store))
            },
            |spell_id| creature_addon_aura_effect_mask_like_cpp(spell_store, spell_id),
            |spell_id| creature_addon_aura_flags_like_cpp(spell_store, spell_id),
        )
    }

    /// Mirrors C++ `Creature::GetCreatureAddon`: spawn-specific addon wins over template addon.
    pub fn get_for_creature_like_cpp(
        &self,
        spawn_id: u64,
        entry: u32,
    ) -> Option<CreatureAddonLifecycleRecordLikeCpp> {
        if spawn_id != 0 {
            if let Some(addon) = self.spawn_addons.get(&spawn_id) {
                return Some(addon.clone());
            }
        }
        self.template_addons.get(&entry).cloned()
    }

    /// Mirrors the spawn-addon side effect in C++ `ObjectMgr::LoadCreatureAddons`.
    ///
    /// If a concrete spawn uses `WAYPOINT_MOTION_TYPE` but its spawn-specific
    /// `creature_addon` row has no `PathId`, C++ mutates `CreatureData::movementType`
    /// to `IDLE_MOTION_TYPE`. Template addon rows do not apply this mutation.
    pub fn movement_type_after_spawn_addon_load_like_cpp(
        &self,
        spawn_id: u64,
        movement_type: u8,
    ) -> u8 {
        if movement_type == WAYPOINT_MOTION_TYPE_LIKE_CPP
            && self
                .spawn_addons
                .get(&spawn_id)
                .is_some_and(|addon| addon.path_id == 0)
        {
            IDLE_MOTION_TYPE_LIKE_CPP
        } else {
            movement_type
        }
    }

    pub fn len(&self) -> usize {
        self.spawn_addons.len() + self.template_addons.len()
    }

    pub fn is_empty(&self) -> bool {
        self.spawn_addons.is_empty() && self.template_addons.is_empty()
    }
}

pub(super) fn addon_record_from_row_like_cpp(
    row: CreatureAddonRowLikeCpp,
    mount_display_exists: &impl Fn(u32) -> bool,
    emote_exists: &impl Fn(u32) -> bool,
    anim_kit_exists: &impl Fn(u32) -> bool,
    spell_exists: &impl Fn(u32) -> bool,
    spell_has_control_vehicle_aura: &impl Fn(u32) -> bool,
    spell_duration_ms: &impl Fn(u32) -> i32,
    spell_unit_owned_aura_effect_mask: &impl Fn(u32) -> u32,
    spell_addon_aura_flags: &impl Fn(u32) -> u32,
) -> CreatureAddonLifecycleRecordLikeCpp {
    let mount_display_id = if row.mount != 0 && !mount_display_exists(row.mount) {
        0
    } else {
        row.mount
    };
    let stand_state = normalize_unit_stand_state_like_cpp(row.stand_state);
    let anim_tier = normalize_anim_tier_like_cpp(row.anim_tier);
    let sheath_state = normalize_sheath_state_like_cpp(row.sheath_state);
    let emote = if emote_exists(row.emote) {
        row.emote
    } else {
        0
    };
    let ai_anim_kit_id = normalize_anim_kit_like_cpp(row.ai_anim_kit, anim_kit_exists);
    let movement_anim_kit_id = normalize_anim_kit_like_cpp(row.movement_anim_kit, anim_kit_exists);
    let melee_anim_kit_id = normalize_anim_kit_like_cpp(row.melee_anim_kit, anim_kit_exists);
    let visibility_distance_type =
        VisibilityDistanceTypeLikeCpp::from_u8_like_cpp(row.visibility_distance_type);
    let auras = normalize_creature_addon_auras_like_cpp(
        &row.auras,
        spell_exists,
        spell_has_control_vehicle_aura,
        spell_duration_ms,
    );
    let aura_applications = auras
        .iter()
        .copied()
        .filter_map(|spell_id| {
            let effect_mask = spell_unit_owned_aura_effect_mask(spell_id);
            (effect_mask != 0).then(|| CreatureAddonAuraApplicationLikeCpp {
                spell_id,
                effect_mask,
                flags: spell_addon_aura_flags(spell_id),
            })
        })
        .collect();

    CreatureAddonLifecycleRecordLikeCpp {
        path_id: row.path_id,
        mount_display_id,
        stand_state,
        vis_flags: row.vis_flags,
        anim_tier,
        sheath_state,
        pvp_flags: UnitPvpFlags::from_bits_retain(row.pvp_flags),
        emote,
        ai_anim_kit_id,
        movement_anim_kit_id,
        melee_anim_kit_id,
        visibility_distance_type,
        auras,
        aura_applications,
    }
}

pub(super) fn normalize_creature_addon_auras_like_cpp(
    auras: &str,
    spell_exists: &impl Fn(u32) -> bool,
    spell_has_control_vehicle_aura: &impl Fn(u32) -> bool,
    spell_duration_ms: &impl Fn(u32) -> i32,
) -> Vec<u32> {
    let mut normalized = Vec::new();
    for token in auras.split_whitespace() {
        let Ok(spell_id) = token.parse::<u32>() else {
            continue;
        };
        if !spell_exists(spell_id) {
            continue;
        }
        let _control_vehicle_warn_only = spell_has_control_vehicle_aura(spell_id);
        if normalized.contains(&spell_id) {
            continue;
        }
        if spell_duration_ms(spell_id) > 0 {
            continue;
        }
        normalized.push(spell_id);
    }
    normalized
}

pub(super) fn creature_addon_aura_effect_mask_like_cpp(
    spell_store: &SpellStore,
    spell_id: u32,
) -> u32 {
    use crate::spell::spell_effect_types::{
        SPELL_EFFECT_APPLY_AREA_AURA_ENEMY, SPELL_EFFECT_APPLY_AREA_AURA_FRIEND,
        SPELL_EFFECT_APPLY_AREA_AURA_OWNER, SPELL_EFFECT_APPLY_AREA_AURA_PARTY,
        SPELL_EFFECT_APPLY_AREA_AURA_PET, SPELL_EFFECT_APPLY_AREA_AURA_RAID,
        SPELL_EFFECT_APPLY_AURA, SPELL_EFFECT_APPLY_AURA_ON_PET,
    };

    const SPELL_EFFECT_APPLY_AREA_AURA_SUMMONS: u32 = 202;
    const SPELL_EFFECT_APPLY_AREA_AURA_PARTY_NONRANDOM: u32 = 271;

    let Some(spell) = i32::try_from(spell_id)
        .ok()
        .and_then(|spell_id| spell_store.get(spell_id))
    else {
        return 0;
    };

    spell.effects().iter().fold(0, |mask, effect| {
        let unit_owned = matches!(
            effect.effect,
            SPELL_EFFECT_APPLY_AURA
                | SPELL_EFFECT_APPLY_AURA_ON_PET
                | SPELL_EFFECT_APPLY_AREA_AURA_PARTY
                | SPELL_EFFECT_APPLY_AREA_AURA_RAID
                | SPELL_EFFECT_APPLY_AREA_AURA_FRIEND
                | SPELL_EFFECT_APPLY_AREA_AURA_ENEMY
                | SPELL_EFFECT_APPLY_AREA_AURA_PET
                | SPELL_EFFECT_APPLY_AREA_AURA_OWNER
                | SPELL_EFFECT_APPLY_AREA_AURA_SUMMONS
                | SPELL_EFFECT_APPLY_AREA_AURA_PARTY_NONRANDOM
        );
        if unit_owned && effect.effect_index < u32::BITS {
            mask | (1u32 << effect.effect_index)
        } else {
            mask
        }
    })
}

pub(super) fn creature_addon_aura_flags_like_cpp(spell_store: &SpellStore, spell_id: u32) -> u32 {
    let Ok(spell_id_i32) = i32::try_from(spell_id) else {
        return AFLAG_NOCASTER_LIKE_CPP | AFLAG_POSITIVE_LIKE_CPP;
    };
    let passive = spell_store.is_passive_like_cpp(spell_id_i32);
    let no_aura_icon = spell_store.has_attribute1_like_cpp(
        spell_id_i32,
        crate::spell::attributes::SPELL_ATTR1_NO_AURA_ICON,
    );
    let mut flags = AFLAG_NOCASTER_LIKE_CPP | AFLAG_POSITIVE_LIKE_CPP;
    if !passive && !no_aura_icon {
        flags |= AFLAG_CANCELABLE_LIKE_CPP;
    }
    if passive {
        flags |= AFLAG_PASSIVE_LIKE_CPP;
    }
    flags
}

pub(super) fn normalize_anim_kit_like_cpp(anim_kit_id: u16, exists: &impl Fn(u32) -> bool) -> u16 {
    if anim_kit_id != 0 && !exists(u32::from(anim_kit_id)) {
        0
    } else {
        anim_kit_id
    }
}

impl CreatureTemplateSparringStoreLikeCpp {
    pub fn from_rows_like_cpp(
        rows: impl IntoIterator<Item = (u32, f32)>,
        template_exists: impl Fn(u32) -> bool,
    ) -> Self {
        let mut values: HashMap<u32, Vec<f32>> = HashMap::new();
        for (entry, no_npc_damage_below_health_pct) in rows {
            if !template_exists(entry)
                || no_npc_damage_below_health_pct <= 0.0
                || no_npc_damage_below_health_pct > 100.0
            {
                continue;
            }
            values
                .entry(entry)
                .or_default()
                .push(no_npc_damage_below_health_pct);
        }
        Self { values }
    }

    pub fn values_for_entry_like_cpp(&self, entry: u32) -> Option<&[f32]> {
        self.values.get(&entry).map(Vec::as_slice)
    }

    pub fn select_for_entry_like_cpp<R: Rng + ?Sized>(
        &self,
        entry: u32,
        rng: &mut R,
    ) -> Option<f32> {
        let values = self.values_for_entry_like_cpp(entry)?;
        if values.is_empty() {
            return None;
        }
        Some(values[rng.gen_range(0..values.len())])
    }

    pub fn select_for_entry_by_index_like_cpp(&self, entry: u32, index: usize) -> Option<f32> {
        let values = self.values_for_entry_like_cpp(entry)?;
        if values.is_empty() {
            return None;
        }
        Some(values[index % values.len()])
    }

    pub fn len(&self) -> usize {
        self.values.values().map(Vec::len).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

#[derive(Debug, Clone, Default)]
pub struct CreatureTemplateLifecycleStoreLikeCpp {
    pub(super) templates: HashMap<u32, CreatureTemplateLifecycleRecordLikeCpp>,
}

impl CreatureTemplateLifecycleStoreLikeCpp {
    pub fn from_templates(
        templates: impl IntoIterator<Item = CreatureTemplateLifecycleRecordLikeCpp>,
    ) -> Self {
        Self {
            templates: templates
                .into_iter()
                .map(CreatureTemplateLifecycleRecordLikeCpp::normalize_like_cpp)
                .map(|template| (template.entry, template))
                .collect(),
        }
    }

    pub fn from_catalog_rows_like_cpp(
        templates: impl IntoIterator<Item = CreatureTemplateLifecycleRecordLikeCpp>,
        spells: impl IntoIterator<Item = (u32, u8, u32)>,
        models: impl IntoIterator<Item = (u32, CreatureTemplateLifecycleModelLikeCpp)>,
    ) -> Self {
        let mut templates: HashMap<_, _> = templates
            .into_iter()
            .map(|template| (template.entry, template))
            .collect();
        for (creature_id, index, spell_id) in spells {
            if let Some(template) = templates.get_mut(&creature_id)
                && usize::from(index) < MAX_CREATURE_SPELLS_LIKE_CPP
            {
                template.spells[usize::from(index)] = spell_id;
            }
        }
        for (creature_id, model) in models {
            let model = model.normalize_like_cpp();
            if model.creature_display_id != 0
                && let Some(template) = templates.get_mut(&creature_id)
            {
                template.models.push(model);
            }
        }
        Self::from_templates(templates.into_values())
    }

    pub fn get(&self, entry: u32) -> Option<&CreatureTemplateLifecycleRecordLikeCpp> {
        self.templates.get(&entry)
    }

    pub fn entries_like_cpp(
        &self,
    ) -> impl Iterator<Item = &CreatureTemplateLifecycleRecordLikeCpp> {
        self.templates.values()
    }

    /// Applies C++ `ObjectMgr::LoadNPCSpellClickSpells` post-load fixup for
    /// templates that carry `UNIT_NPC_FLAG_SPELLCLICK` without spellclick data.
    pub fn remove_npc_flag_for_entries_like_cpp(
        &mut self,
        entries: impl IntoIterator<Item = u32>,
        flag: u64,
    ) -> usize {
        let mut removed = 0;
        for entry in entries {
            let Some(template) = self.templates.get_mut(&entry) else {
                continue;
            };
            if (template.npc_flags & flag) == 0 {
                continue;
            }
            template.npc_flags &= !flag;
            removed += 1;
        }
        removed
    }

    pub fn len(&self) -> usize {
        self.templates.len()
    }

    pub fn is_empty(&self) -> bool {
        self.templates.is_empty()
    }
}

impl CreatureTemplateLifecycleRecordLikeCpp {
    pub fn normalize_like_cpp(mut self) -> Self {
        self.ground_movement_type =
            normalize_creature_ground_movement_type_like_cpp(self.ground_movement_type);
        self.flight_movement_type =
            normalize_creature_flight_movement_type_like_cpp(self.flight_movement_type);
        self.chase_movement_type =
            normalize_creature_chase_movement_type_like_cpp(self.chase_movement_type);
        self.random_movement_type =
            normalize_creature_random_movement_type_like_cpp(self.random_movement_type);
        if self.required_expansion >= MAX_EXPANSIONS_LIKE_CPP {
            self.required_expansion = 0;
        }
        if self.damage_school >= MAX_SPELL_SCHOOL_LIKE_CPP {
            self.damage_school = wow_constants::spell::SpellSchools::Normal as u8;
        }
        if self.speed_walk == 0.0 {
            self.speed_walk = 1.0;
        }
        if self.speed_run == 0.0 {
            self.speed_run = 1.14286;
        }
        self.models = self
            .models
            .into_iter()
            .map(CreatureTemplateLifecycleModelLikeCpp::normalize_like_cpp)
            .collect();
        self
    }

    pub fn first_model_like_cpp(&self) -> Option<CreatureTemplateLifecycleModelLikeCpp> {
        self.models.first().copied()
    }

    /// Mirrors C++ `CreatureTemplate::GetFirstValidModel`.
    ///
    /// C++ anchor: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Creature/Creature.cpp:130-136`.
    pub fn first_valid_model_like_cpp(&self) -> Option<CreatureTemplateLifecycleModelLikeCpp> {
        self.models
            .iter()
            .copied()
            .find(|model| model.creature_display_id != 0)
    }

    /// Mirrors C++ `CreatureTemplate::GetRandomValidModel`.
    ///
    /// C++ anchor: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Creature/Creature.cpp:113-128`.
    pub fn random_valid_model_like_cpp(
        &self,
        random: &mut impl CreatureModelSelectionRandomLikeCpp,
    ) -> Option<CreatureTemplateLifecycleModelLikeCpp> {
        match self.models.as_slice() {
            [] => None,
            [model] => Some(*model),
            models => {
                let total: f32 = models.iter().map(|model| model.probability.max(0.0)).sum();
                if total <= f32::EPSILON {
                    return models.first().copied();
                }

                let mut roll = random.weighted_model_roll_like_cpp(total).clamp(0.0, total);
                for model in models {
                    roll -= model.probability.max(0.0);
                    if roll <= 0.0 {
                        return Some(*model);
                    }
                }

                models.last().copied()
            }
        }
    }

    /// Mirrors C++ `CreatureTemplate::GetModelWithDisplayId`.
    ///
    /// C++ anchor: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Creature/Creature.cpp:139-146`.
    pub fn model_with_display_id_like_cpp(
        &self,
        display_id: u32,
    ) -> Option<CreatureTemplateLifecycleModelLikeCpp> {
        self.models
            .iter()
            .copied()
            .find(|model| model.creature_display_id == display_id)
    }

    /// Mirrors C++ `CreatureTemplate::GetFirstInvisibleModel`.
    ///
    /// C++ anchor: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Creature/Creature.cpp:148-156`.
    pub fn first_invisible_model_like_cpp(
        &self,
        model_info_store: &CreatureModelInfoStoreLikeCpp,
    ) -> CreatureTemplateLifecycleModelLikeCpp {
        self.models
            .iter()
            .copied()
            .find(|model| {
                model_info_store
                    .get(model.creature_display_id)
                    .is_some_and(|model_info| model_info.is_trigger)
            })
            .unwrap_or(CreatureTemplateLifecycleModelLikeCpp {
                creature_display_id: DEFAULT_INVISIBLE_CREATURE_DISPLAY_ID_LIKE_CPP,
                display_scale: 1.0,
                probability: 1.0,
            })
    }

    /// Mirrors C++ `ObjectMgr::ChooseDisplayId` plus `ObjectMgr::GetCreatureModelRandomGender`.
    ///
    /// C++ anchors:
    /// - `/home/server/woltk-trinity-legacy/src/server/game/Globals/ObjectMgr.cpp:1669-1680`
    /// - `/home/server/woltk-trinity-legacy/src/server/game/Globals/ObjectMgr.cpp:1702-1728`
    pub fn choose_display_model_like_cpp(
        &self,
        model_info_store: &CreatureModelInfoStoreLikeCpp,
        spawn_model: Option<CreatureTemplateLifecycleModelLikeCpp>,
        random: &mut impl CreatureModelSelectionRandomLikeCpp,
    ) -> Option<CreatureTemplateLifecycleModelLikeCpp> {
        self.first_valid_model_like_cpp()?;

        let flags_extra = CreatureFlagsExtra::from_bits_truncate(self.flags_extra);
        let mut model = if let Some(model) = spawn_model {
            model
        } else if !flags_extra.contains(CreatureFlagsExtra::TRIGGER) {
            self.random_valid_model_like_cpp(random)?
        } else {
            self.first_invisible_model_like_cpp(model_info_store)
        };

        let model_info = model_info_store.get(model.creature_display_id)?;
        if model_info.display_id_other_gender != 0 && random.other_gender_roll_zero_like_cpp() {
            let other_gender_display_id = model_info.display_id_other_gender;
            if model_info_store.get(other_gender_display_id).is_some() {
                model.creature_display_id = other_gender_display_id;
                if let Some(template_model) =
                    self.model_with_display_id_like_cpp(other_gender_display_id)
                {
                    model = template_model;
                }
            }
        }

        Some(model)
    }

    pub fn apply_spell_row_like_cpp(&mut self, index: usize, spell: u32) {
        if index < MAX_CREATURE_SPELLS_LIKE_CPP {
            self.spells[index] = spell;
        }
    }

    pub fn push_model_like_cpp(&mut self, model: CreatureTemplateLifecycleModelLikeCpp) {
        if model.creature_display_id != 0 {
            self.models.push(model.normalize_like_cpp());
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CreatureClassificationHealthRatesLikeCpp {
    pub normal: f32,
    pub elite: f32,
    pub rare_elite: f32,
    pub obsolete: f32,
    pub rare: f32,
    pub trivial: f32,
    pub minus_mob: f32,
}

impl Default for CreatureClassificationHealthRatesLikeCpp {
    fn default() -> Self {
        Self {
            normal: 1.0,
            elite: 1.0,
            rare_elite: 1.0,
            obsolete: 1.0,
            rare: 1.0,
            trivial: 1.0,
            minus_mob: 1.0,
        }
    }
}

impl CreatureClassificationHealthRatesLikeCpp {
    /// C++ `Creature::GetHealthMod(CreatureClassifications)` switch. Unknown
    /// classifications fall through to the elite rate, matching the C++ default.
    ///
    /// C++ anchor: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Creature/Creature.cpp:1646-1666`.
    pub fn modifier_for_classification_like_cpp(&self, classification: u32) -> f32 {
        match classification {
            0 => self.normal,
            1 => self.elite,
            2 => self.rare_elite,
            3 => self.obsolete,
            4 => self.rare,
            5 => self.trivial,
            6 => self.minus_mob,
            _ => self.elite,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CreatureClassificationDamageRatesLikeCpp {
    pub normal: f32,
    pub elite: f32,
    pub rare_elite: f32,
    pub obsolete: f32,
    pub rare: f32,
    pub trivial: f32,
    pub minus_mob: f32,
}

impl Default for CreatureClassificationDamageRatesLikeCpp {
    fn default() -> Self {
        Self {
            normal: 1.0,
            elite: 1.0,
            rare_elite: 1.0,
            obsolete: 1.0,
            rare: 1.0,
            trivial: 1.0,
            minus_mob: 1.0,
        }
    }
}

impl CreatureClassificationDamageRatesLikeCpp {
    /// C++ `Creature::GetDamageMod(CreatureClassifications)` switch. Unknown
    /// classifications fall through to the elite rate, matching the C++ default.
    ///
    /// C++ anchor: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Creature/Creature.cpp:1675-1695`.
    pub fn modifier_for_classification_like_cpp(&self, classification: u32) -> f32 {
        match classification {
            0 => self.normal,
            1 => self.elite,
            2 => self.rare_elite,
            3 => self.obsolete,
            4 => self.rare,
            5 => self.trivial,
            6 => self.minus_mob,
            _ => self.elite,
        }
    }
}
