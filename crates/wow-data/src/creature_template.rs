use std::collections::HashMap;

use anyhow::Result;
use rand::Rng;
use wow_constants::{
    CreatureFlightMovementType, CreatureGroundMovementType, SheathState, UnitPvpFlags,
    UnitStandStateType,
};
use wow_database::WorldDatabase;
use wow_entities::{CreatureAddonLifecycleRecordLikeCpp, VisibilityDistanceTypeLikeCpp};

use crate::{AnimKitStore, CreatureDisplayInfoStore, EmotesStore};

pub const MAX_CREATURE_SPELLS_LIKE_CPP: usize = 8;
pub const MAX_SPELL_SCHOOL_LIKE_CPP: u8 = 7;
const CREATURE_GROUND_MOVEMENT_TYPE_MAX_LIKE_CPP: u8 = 3;
const CREATURE_FLIGHT_MOVEMENT_TYPE_MAX_LIKE_CPP: u8 = 3;
const IDLE_MOTION_TYPE_LIKE_CPP: u8 = 0;
const WAYPOINT_MOTION_TYPE_LIKE_CPP: u8 = 2;
const MAX_ANIM_TIER_LIKE_CPP: u8 = 5;
const MAX_SHEATH_STATE_LIKE_CPP: u8 = 3;

fn normalize_creature_ground_movement_type_like_cpp(ground_movement_type: u8) -> u8 {
    if ground_movement_type < CREATURE_GROUND_MOVEMENT_TYPE_MAX_LIKE_CPP {
        ground_movement_type
    } else {
        CreatureGroundMovementType::Run as u8
    }
}

fn normalize_creature_flight_movement_type_like_cpp(flight_movement_type: u8) -> u8 {
    if flight_movement_type < CREATURE_FLIGHT_MOVEMENT_TYPE_MAX_LIKE_CPP {
        flight_movement_type
    } else {
        CreatureFlightMovementType::None as u8
    }
}

fn normalize_unit_stand_state_like_cpp(stand_state: u8) -> UnitStandStateType {
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

fn normalize_anim_tier_like_cpp(anim_tier: u8) -> u8 {
    if anim_tier < MAX_ANIM_TIER_LIKE_CPP {
        anim_tier
    } else {
        0
    }
}

fn normalize_sheath_state_like_cpp(sheath_state: u8) -> SheathState {
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
    classifications: HashMap<u32, u32>,
}

impl CreatureTemplateClassificationStoreLikeCpp {
    pub fn from_entries(entries: impl IntoIterator<Item = (u32, u32)>) -> Self {
        Self {
            classifications: entries.into_iter().collect(),
        }
    }

    /// Loads the minimal `creature_template` classification dependency in the same
    /// order shape as C++ `ObjectMgr::LoadCreatureTemplates`/`LoadCreatureTemplate`.
    ///
    /// C++ anchors:
    /// - `/home/server/woltk-trinity-legacy/src/server/game/Globals/ObjectMgr.cpp:349-400`
    /// - `/home/server/woltk-trinity-legacy/src/server/game/Globals/ObjectMgr.cpp:403-482`
    ///
    /// The full template is intentionally not materialized in this data-only store;
    /// C++ field[0] is `entry` and field[15] is `Classification`.
    pub async fn load_like_cpp(db: &WorldDatabase) -> Result<Self> {
        let mut result = db
            .direct_query("SELECT entry, Classification FROM creature_template")
            .await?;

        if result.is_empty() {
            return Ok(Self::default());
        }

        let mut classifications = HashMap::new();
        loop {
            let entry = result.try_read::<u32>(0).unwrap_or(0);
            let classification = result.try_read::<u32>(1).unwrap_or(0);
            classifications.insert(entry, classification);

            if !result.next_row() {
                break;
            }
        }

        Ok(Self { classifications })
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
    pub unit_class: u8,
    pub vehicle_id: u32,
    pub movement_type: u8,
    pub ground_movement_type: u8,
    pub swim_allowed: bool,
    pub flight_movement_type: u8,
    pub flags_extra: u32,
    pub string_id: String,
    pub regen_health: bool,
    pub spells: [u32; MAX_CREATURE_SPELLS_LIKE_CPP],
    pub models: Vec<CreatureTemplateLifecycleModelLikeCpp>,
}

#[derive(Debug, Clone, Default)]
pub struct CreatureTemplateSparringStoreLikeCpp {
    values: HashMap<u32, Vec<f32>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
}

#[derive(Debug, Clone, Default)]
pub struct CreatureAddonStoreLikeCpp {
    spawn_addons: HashMap<u64, CreatureAddonLifecycleRecordLikeCpp>,
    template_addons: HashMap<u32, CreatureAddonLifecycleRecordLikeCpp>,
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
                    ),
                )
            })
            .collect();

        Self {
            spawn_addons,
            template_addons,
        }
    }

    /// Loads the represented subset of C++ `ObjectMgr::LoadCreatureAddons` and
    /// `ObjectMgr::LoadCreatureTemplateAddons`.
    ///
    /// C++ anchors:
    /// - `/home/server/woltk-trinity-legacy/src/server/game/Globals/ObjectMgr.cpp:766-897`
    /// - `/home/server/woltk-trinity-legacy/src/server/game/Globals/ObjectMgr.cpp:1224-1367`
    pub async fn load_like_cpp(
        db: &WorldDatabase,
        template_store: &CreatureTemplateLifecycleStoreLikeCpp,
        creature_spawn_store: &crate::WorldSpawnIdStore,
        display_store: &CreatureDisplayInfoStore,
        emotes_store: &EmotesStore,
        anim_kit_store: &AnimKitStore,
    ) -> Result<Self> {
        let spawn_rows = load_creature_addon_rows_like_cpp(
            db,
            "SELECT guid, PathId, mount, StandState, AnimTier, VisFlags, SheathState, PvPFlags, emote, aiAnimKit, movementAnimKit, meleeAnimKit, visibilityDistanceType, auras FROM creature_addon",
        )
        .await?;
        let template_rows = load_creature_addon_rows_like_cpp(
            db,
            "SELECT entry, PathId, mount, StandState, AnimTier, VisFlags, SheathState, PvPFlags, emote, aiAnimKit, movementAnimKit, meleeAnimKit, visibilityDistanceType, auras FROM creature_template_addon",
        )
        .await?;

        Ok(Self::from_rows_like_cpp(
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
        ))
    }

    /// Mirrors C++ `Creature::GetCreatureAddon`: spawn-specific addon wins over template addon.
    pub fn get_for_creature_like_cpp(
        &self,
        spawn_id: u64,
        entry: u32,
    ) -> Option<CreatureAddonLifecycleRecordLikeCpp> {
        if spawn_id != 0 {
            if let Some(addon) = self.spawn_addons.get(&spawn_id) {
                return Some(*addon);
            }
        }
        self.template_addons.get(&entry).copied()
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

async fn load_creature_addon_rows_like_cpp(
    db: &WorldDatabase,
    query: &str,
) -> Result<Vec<CreatureAddonRowLikeCpp>> {
    let mut result = db.direct_query(query).await?;
    if result.is_empty() {
        return Ok(Vec::new());
    }

    let mut rows = Vec::new();
    loop {
        rows.push(CreatureAddonRowLikeCpp {
            owner_id: result.try_read::<u64>(0).unwrap_or(0),
            path_id: result.try_read::<u32>(1).unwrap_or(0),
            mount: result.try_read::<u32>(2).unwrap_or(0),
            stand_state: result.try_read::<u8>(3).unwrap_or(0),
            anim_tier: result.try_read::<u8>(4).unwrap_or(0),
            vis_flags: result.try_read::<u8>(5).unwrap_or(0),
            sheath_state: result.try_read::<u8>(6).unwrap_or(0),
            pvp_flags: result.try_read::<u8>(7).unwrap_or(0),
            emote: result.try_read::<u32>(8).unwrap_or(0),
            ai_anim_kit: result.try_read::<u16>(9).unwrap_or(0),
            movement_anim_kit: result.try_read::<u16>(10).unwrap_or(0),
            melee_anim_kit: result.try_read::<u16>(11).unwrap_or(0),
            visibility_distance_type: result.try_read::<u8>(12).unwrap_or(0),
        });
        if !result.next_row() {
            break;
        }
    }

    Ok(rows)
}

fn addon_record_from_row_like_cpp(
    row: CreatureAddonRowLikeCpp,
    mount_display_exists: &impl Fn(u32) -> bool,
    emote_exists: &impl Fn(u32) -> bool,
    anim_kit_exists: &impl Fn(u32) -> bool,
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
    }
}

fn normalize_anim_kit_like_cpp(anim_kit_id: u16, exists: &impl Fn(u32) -> bool) -> u16 {
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

    /// Loads C++ `ObjectMgr::LoadCreatureTemplateSparring`.
    ///
    /// C++ anchors:
    /// - `/home/server/woltk-trinity-legacy/src/server/game/Globals/ObjectMgr.cpp:899-937`
    /// - `/home/server/woltk-trinity-legacy/src/server/game/Globals/ObjectMgr.cpp:1468-1471`
    pub async fn load_like_cpp(
        db: &WorldDatabase,
        template_store: &CreatureTemplateLifecycleStoreLikeCpp,
    ) -> Result<Self> {
        let mut result = db
            .direct_query("SELECT Entry, NoNPCDamageBelowHealthPct FROM creature_template_sparring")
            .await?;

        if result.is_empty() {
            return Ok(Self::default());
        }

        let mut rows = Vec::new();
        loop {
            rows.push((
                result.try_read::<u32>(0).unwrap_or(0),
                result.try_read::<f32>(1).unwrap_or(0.0),
            ));

            if !result.next_row() {
                break;
            }
        }

        Ok(Self::from_rows_like_cpp(rows, |entry| {
            template_store.get(entry).is_some()
        }))
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
    templates: HashMap<u32, CreatureTemplateLifecycleRecordLikeCpp>,
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

    /// Loads C++ `ObjectMgr::LoadCreatureTemplates` input rows for future
    /// `Creature::InitEntry`/`Creature::LoadFromDB` wiring.
    ///
    /// C++ anchors:
    /// - `/home/server/woltk-trinity-legacy/src/server/game/Globals/ObjectMgr.cpp:349-400`
    /// - `/home/server/woltk-trinity-legacy/src/server/game/Globals/ObjectMgr.cpp:403-482`
    /// - `/home/server/woltk-trinity-legacy/src/server/game/Globals/ObjectMgr.cpp:575-617`
    /// - `/home/server/woltk-trinity-legacy/src/server/game/Globals/ObjectMgr.cpp:620+`
    pub async fn load_like_cpp(db: &WorldDatabase) -> Result<Self> {
        let mut templates = HashMap::new();
        let mut result = db
            .direct_query(
                "SELECT ct.entry, ct.name, ct.AIName, ct.ScriptName, ct.faction, ct.npcflag, ct.speed_walk, ct.speed_run, ct.scale, ct.Classification, ct.dmgschool, ct.unit_flags, ct.unit_flags2, ct.unit_flags3, ct.`type`, ct.unit_class, ct.VehicleId, ct.MovementType, COALESCE(ctm.Ground, 1), COALESCE(ctm.Swim, 1), COALESCE(ctm.Flight, 0), ct.flags_extra, ct.StringId, ct.RegenHealth FROM creature_template ct LEFT JOIN creature_template_movement ctm ON ct.entry = ctm.CreatureId",
            )
            .await?;
        if !result.is_empty() {
            loop {
                let record = CreatureTemplateLifecycleRecordLikeCpp {
                    entry: result.try_read::<u32>(0).unwrap_or(0),
                    name: result.try_read::<String>(1).unwrap_or_default(),
                    ai_name: result.try_read::<String>(2).unwrap_or_default(),
                    script_name: result.try_read::<String>(3).unwrap_or_default(),
                    faction: result.try_read::<u32>(4).unwrap_or(0),
                    npc_flags: result.try_read::<u64>(5).unwrap_or(0),
                    speed_walk: result.try_read::<f32>(6).unwrap_or(0.0),
                    speed_run: result.try_read::<f32>(7).unwrap_or(0.0),
                    scale: result.try_read::<f32>(8).unwrap_or(1.0),
                    classification: result.try_read::<u32>(9).unwrap_or(0),
                    damage_school: result.try_read::<u8>(10).unwrap_or(0),
                    unit_flags: result.try_read::<u32>(11).unwrap_or(0),
                    unit_flags2: result.try_read::<u32>(12).unwrap_or(0),
                    unit_flags3: result.try_read::<u32>(13).unwrap_or(0),
                    creature_type: result.try_read::<u32>(14).unwrap_or(0),
                    unit_class: result.try_read::<u8>(15).unwrap_or(0),
                    vehicle_id: result.try_read::<u32>(16).unwrap_or(0),
                    movement_type: result.try_read::<u8>(17).unwrap_or(0),
                    ground_movement_type: result
                        .try_read::<Option<u8>>(18)
                        .flatten()
                        .unwrap_or(CreatureGroundMovementType::Run as u8),
                    swim_allowed: result.try_read::<Option<u8>>(19).flatten().unwrap_or(1) != 0,
                    flight_movement_type: result.try_read::<Option<u8>>(20).flatten().unwrap_or(0),
                    flags_extra: result.try_read::<u32>(21).unwrap_or(0),
                    string_id: result.try_read::<String>(22).unwrap_or_default(),
                    regen_health: result.try_read::<u8>(23).unwrap_or(0) != 0,
                    spells: [0; MAX_CREATURE_SPELLS_LIKE_CPP],
                    models: Vec::new(),
                };
                templates.insert(record.entry, record);
                if !result.next_row() {
                    break;
                }
            }
        }

        let mut spell_result = db
            .direct_query("SELECT CreatureID, `Index`, Spell FROM creature_template_spell")
            .await?;
        if !spell_result.is_empty() {
            loop {
                let creature_id = spell_result.try_read::<u32>(0).unwrap_or(0);
                let index = spell_result
                    .try_read::<u8>(1)
                    .map(usize::from)
                    .unwrap_or(MAX_CREATURE_SPELLS_LIKE_CPP);
                let spell = spell_result.try_read::<u32>(2).unwrap_or(0);
                if index < MAX_CREATURE_SPELLS_LIKE_CPP {
                    if let Some(template) = templates.get_mut(&creature_id) {
                        template.spells[index] = spell;
                    }
                }
                if !spell_result.next_row() {
                    break;
                }
            }
        }

        let mut model_result = db
            .direct_query(
                "SELECT CreatureID, CreatureDisplayID, DisplayScale, Probability FROM creature_template_model ORDER BY Idx ASC",
            )
            .await?;
        if !model_result.is_empty() {
            loop {
                let creature_id = model_result.try_read::<u32>(0).unwrap_or(0);
                let model = CreatureTemplateLifecycleModelLikeCpp {
                    creature_display_id: model_result.try_read::<u32>(1).unwrap_or(0),
                    display_scale: model_result.try_read::<f32>(2).unwrap_or(0.0),
                    probability: model_result.try_read::<f32>(3).unwrap_or(0.0),
                }
                .normalize_like_cpp();
                if model.creature_display_id != 0 {
                    if let Some(template) = templates.get_mut(&creature_id) {
                        template.models.push(model);
                    }
                }
                if !model_result.next_row() {
                    break;
                }
            }
        }

        Ok(Self::from_templates(templates.into_values()))
    }

    pub fn get(&self, entry: u32) -> Option<&CreatureTemplateLifecycleRecordLikeCpp> {
        self.templates.get(&entry)
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
        if self.damage_school >= MAX_SPELL_SCHOOL_LIKE_CPP {
            self.damage_school = wow_constants::spell::SpellSchools::Normal as u8;
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CreatureTemplateMountModelLikeCpp {
    pub display_id: u32,
    pub display_scale: f32,
    pub probability: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CreatureTemplateMountEntryLikeCpp {
    pub entry: u32,
    pub vehicle_id: u32,
    pub models: Vec<CreatureTemplateMountModelLikeCpp>,
}

#[derive(Debug, Clone, Default)]
pub struct CreatureTemplateMountStoreLikeCpp {
    entries: HashMap<u32, CreatureTemplateMountEntryLikeCpp>,
}

impl CreatureTemplateMountStoreLikeCpp {
    pub fn from_entries(
        entries: impl IntoIterator<Item = CreatureTemplateMountEntryLikeCpp>,
    ) -> Self {
        Self {
            entries: entries
                .into_iter()
                .map(|entry| (entry.entry, entry))
                .collect(),
        }
    }

    pub async fn load_like_cpp(db: &WorldDatabase) -> Result<Self> {
        let mut result = db
            .direct_query(
                "SELECT ct.entry, ct.VehicleId, ctm.CreatureDisplayID, ctm.DisplayScale, ctm.Probability \
                 FROM creature_template ct \
                 LEFT JOIN creature_template_model ctm ON ct.entry = ctm.CreatureID \
                 ORDER BY ct.entry, ctm.Idx",
            )
            .await?;

        if result.is_empty() {
            return Ok(Self::default());
        }

        let mut entries = HashMap::new();
        loop {
            let entry_id = result.read::<u32>(0);
            let vehicle_id = result.try_read::<u32>(1).unwrap_or(0);
            let display_id = result.try_read::<u32>(2).unwrap_or(0);
            let display_scale = result.try_read::<f32>(3).unwrap_or(0.0);
            let probability = result.try_read::<f32>(4).unwrap_or(0.0);

            let entry =
                entries
                    .entry(entry_id)
                    .or_insert_with(|| CreatureTemplateMountEntryLikeCpp {
                        entry: entry_id,
                        vehicle_id,
                        models: Vec::new(),
                    });
            entry.vehicle_id = vehicle_id;
            if display_id != 0 {
                entry.models.push(CreatureTemplateMountModelLikeCpp {
                    display_id,
                    display_scale,
                    probability,
                });
            }

            if !result.next_row() {
                break;
            }
        }

        Ok(Self { entries })
    }

    pub fn get(&self, entry: u32) -> Option<&CreatureTemplateMountEntryLikeCpp> {
        self.entries.get(&entry)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl CreatureTemplateMountEntryLikeCpp {
    pub fn choose_display_id_like_cpp<R: Rng + ?Sized>(&self, rng: &mut R) -> Option<u32> {
        match self.models.as_slice() {
            [] => None,
            [model] => Some(model.display_id),
            models => {
                let total: f32 = models.iter().map(|model| model.probability.max(0.0)).sum();
                if total <= f32::EPSILON {
                    return models.first().map(|model| model.display_id);
                }

                let mut roll = rng.gen_range(0.0..total);
                for model in models {
                    roll -= model.probability.max(0.0);
                    if roll <= 0.0 {
                        return Some(model.display_id);
                    }
                }

                models.last().map(|model| model.display_id)
            }
        }
    }
}

/// C++ `CURRENT_EXPANSION` for this 3.4.3/TDB442 port.
///
/// Anchor: `SharedDefines.h:87-105` defines Wrath of the Lich King as 2 and
/// `CURRENT_EXPANSION` as `EXPANSION_WRATH_OF_THE_LICH_KING`.
pub const CREATURE_CURRENT_EXPANSION_LIKE_CPP: usize = 2;

/// C++ sentinel `EXPANSION_LEVEL_CURRENT` used by `CreatureDifficulty`.
pub const CREATURE_EXPANSION_LEVEL_CURRENT_LIKE_CPP: i32 = -1;

#[derive(Debug, Clone, PartialEq)]
pub struct CreatureDifficultyRecordLikeCpp {
    pub entry: u32,
    pub difficulty_id: u8,
    pub min_level: u8,
    pub max_level: u8,
    pub health_scaling_expansion: i32,
    pub health_modifier: f32,
    pub mana_modifier: f32,
    pub armor_modifier: f32,
    pub damage_modifier: f32,
    pub creature_difficulty_id: i32,
    pub type_flags: u32,
    pub type_flags2: u32,
    pub loot_id: u32,
    pub pickpocket_loot_id: u32,
    pub skin_loot_id: u32,
    pub gold_min: u32,
    pub gold_max: u32,
    pub static_flags: [u32; 8],
}

impl CreatureDifficultyRecordLikeCpp {
    /// Applies the C++ `ObjectMgr::LoadCreatureTemplateDifficulty` row fixes.
    ///
    /// `classification_damage_modifier` represents
    /// `Creature::GetDamageMod(template.Classification)`. The full creature
    /// template classification lookup remains a future integration slice; this
    /// pure data normalizer only applies the caller-provided multiplier.
    pub fn normalize_like_cpp(mut self, classification_damage_modifier: f32) -> Self {
        self.damage_modifier *= classification_damage_modifier;

        if self.min_level == 0 {
            self.min_level = 1;
        }
        if self.max_level == 0 {
            self.max_level = 1;
        }
        if self.min_level > self.max_level {
            self.min_level = self.max_level;
        }
        if self.health_scaling_expansion < CREATURE_EXPANSION_LEVEL_CURRENT_LIKE_CPP
            || self.health_scaling_expansion > CREATURE_CURRENT_EXPANSION_LIKE_CPP as i32
        {
            self.health_scaling_expansion = 0;
        }
        if self.gold_min > self.gold_max {
            self.gold_max = self.gold_min;
        }

        self
    }

    /// Matches `CreatureDifficulty::GetHealthScalingExpansion`: `-1` maps to
    /// C++ `CURRENT_EXPANSION`, otherwise the normalized DB value is used.
    pub fn health_scaling_expansion_index_like_cpp(&self) -> usize {
        if self.health_scaling_expansion == CREATURE_EXPANSION_LEVEL_CURRENT_LIKE_CPP {
            CREATURE_CURRENT_EXPANSION_LIKE_CPP
        } else {
            self.health_scaling_expansion as usize
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CreatureBaseStatsRecordLikeCpp {
    pub base_health: [u32; CREATURE_CURRENT_EXPANSION_LIKE_CPP + 1],
    pub base_mana: u32,
    pub base_armor: u32,
    pub attack_power: u32,
    pub ranged_attack_power: u32,
    pub base_damage: [f32; CREATURE_CURRENT_EXPANSION_LIKE_CPP + 1],
}

impl Default for CreatureBaseStatsRecordLikeCpp {
    fn default() -> Self {
        Self {
            base_health: [0; CREATURE_CURRENT_EXPANSION_LIKE_CPP + 1],
            base_mana: 0,
            base_armor: 0,
            attack_power: 0,
            ranged_attack_power: 0,
            base_damage: [0.0; CREATURE_CURRENT_EXPANSION_LIKE_CPP + 1],
        }
    }
}

impl CreatureBaseStatsRecordLikeCpp {
    /// Applies C++ `LoadCreatureClassLevelStats` fixes: loaded row HP zero -> 1,
    /// negative base damage -> 0. Missing rows are handled by the store default
    /// and intentionally retain all-zero health/damage arrays like C++ static
    /// zero-initialized fallback stats.
    pub fn normalize_loaded_row_like_cpp(mut self) -> Self {
        for hp in &mut self.base_health {
            if *hp == 0 {
                *hp = 1;
            }
        }
        for damage in &mut self.base_damage {
            if *damage < 0.0 {
                *damage = 0.0;
            }
        }
        self
    }

    pub fn generate_health_like_cpp(&self, difficulty: &CreatureDifficultyRecordLikeCpp) -> u32 {
        (self.base_health[difficulty.health_scaling_expansion_index_like_cpp()] as f32
            * difficulty.health_modifier)
            .ceil() as u32
    }

    pub fn generate_mana_like_cpp(&self, difficulty: &CreatureDifficultyRecordLikeCpp) -> u32 {
        if self.base_mana == 0 {
            return 0;
        }

        (self.base_mana as f32 * difficulty.mana_modifier).ceil() as u32
    }

    pub fn generate_armor_like_cpp(&self, difficulty: &CreatureDifficultyRecordLikeCpp) -> u32 {
        (self.base_armor as f32 * difficulty.armor_modifier).ceil() as u32
    }

    pub fn generate_base_damage_like_cpp(
        &self,
        difficulty: &CreatureDifficultyRecordLikeCpp,
    ) -> f32 {
        self.base_damage[difficulty.health_scaling_expansion_index_like_cpp()]
    }
}

#[derive(Debug, Clone, Default)]
pub struct CreatureBaseStatsStoreLikeCpp {
    records: HashMap<(u8, u8), CreatureBaseStatsRecordLikeCpp>,
    default_record: CreatureBaseStatsRecordLikeCpp,
}

impl CreatureBaseStatsStoreLikeCpp {
    pub fn from_records(
        records: impl IntoIterator<Item = (u8, u8, CreatureBaseStatsRecordLikeCpp)>,
    ) -> Self {
        Self {
            records: records
                .into_iter()
                .map(|(level, unit_class, record)| {
                    ((level, unit_class), record.normalize_loaded_row_like_cpp())
                })
                .collect(),
            default_record: CreatureBaseStatsRecordLikeCpp::default(),
        }
    }

    pub async fn load_like_cpp(db: &WorldDatabase) -> Result<Self> {
        let mut result = db
            .direct_query(
                "SELECT level, class, basehp0, basehp1, basehp2, basemana, basearmor, attackpower, rangedattackpower, damage_base, damage_exp1, damage_exp2 FROM creature_classlevelstats",
            )
            .await?;

        if result.is_empty() {
            return Ok(Self::default());
        }

        let mut records = Vec::new();
        loop {
            let level = result.try_read::<u8>(0).unwrap_or(0);
            let unit_class = result.try_read::<u8>(1).unwrap_or(0);
            let record = CreatureBaseStatsRecordLikeCpp {
                base_health: [
                    result.try_read::<u16>(2).map(u32::from).unwrap_or(0),
                    result.try_read::<u16>(3).map(u32::from).unwrap_or(0),
                    result.try_read::<u16>(4).map(u32::from).unwrap_or(0),
                ],
                base_mana: result.try_read::<u16>(5).map(u32::from).unwrap_or(0),
                base_armor: result.try_read::<u16>(6).map(u32::from).unwrap_or(0),
                attack_power: result.try_read::<u16>(7).map(u32::from).unwrap_or(0),
                ranged_attack_power: result.try_read::<u16>(8).map(u32::from).unwrap_or(0),
                base_damage: [
                    result.try_read::<f32>(9).unwrap_or(0.0),
                    result.try_read::<f32>(10).unwrap_or(0.0),
                    result.try_read::<f32>(11).unwrap_or(0.0),
                ],
            };
            records.push((level, unit_class, record));

            if !result.next_row() {
                break;
            }
        }

        Ok(Self::from_records(records))
    }

    pub fn get_like_cpp(&self, level: u8, unit_class: u8) -> &CreatureBaseStatsRecordLikeCpp {
        self.records
            .get(&(level, unit_class))
            .unwrap_or(&self.default_record)
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }
}

#[derive(Debug, Clone, Default)]
pub struct CreatureDifficultyStoreLikeCpp {
    records: HashMap<(u32, u8), CreatureDifficultyRecordLikeCpp>,
}

impl CreatureDifficultyStoreLikeCpp {
    pub fn from_records(
        records: impl IntoIterator<Item = CreatureDifficultyRecordLikeCpp>,
        classification_damage_modifier_for_entry: impl Fn(u32) -> f32,
    ) -> Self {
        Self {
            records: records
                .into_iter()
                .map(|record| {
                    let key = (record.entry, record.difficulty_id);
                    let classification_damage_modifier =
                        classification_damage_modifier_for_entry(record.entry);
                    let normalized = record.normalize_like_cpp(classification_damage_modifier);
                    (key, normalized)
                })
                .collect(),
        }
    }

    pub async fn load_like_cpp(
        db: &WorldDatabase,
        classification_damage_modifier_for_entry: impl Fn(u32) -> f32,
    ) -> Result<Self> {
        let mut result = db
            .direct_query(
                "SELECT Entry, DifficultyID, MinLevel, MaxLevel, HealthScalingExpansion, HealthModifier, ManaModifier, ArmorModifier, DamageModifier, CreatureDifficultyID, TypeFlags, TypeFlags2, LootID, PickPocketLootID, SkinLootID, GoldMin, GoldMax, StaticFlags1, StaticFlags2, StaticFlags3, StaticFlags4, StaticFlags5, StaticFlags6, StaticFlags7, StaticFlags8 FROM creature_template_difficulty ORDER BY Entry",
            )
            .await?;

        if result.is_empty() {
            return Ok(Self::default());
        }

        let mut records = Vec::new();
        loop {
            records.push(CreatureDifficultyRecordLikeCpp {
                entry: result.try_read::<u32>(0).unwrap_or(0),
                difficulty_id: result.try_read::<u8>(1).unwrap_or(0),
                min_level: result.try_read::<u8>(2).unwrap_or(0),
                max_level: result.try_read::<u8>(3).unwrap_or(0),
                health_scaling_expansion: result.try_read::<i32>(4).unwrap_or(0),
                health_modifier: result.try_read::<f32>(5).unwrap_or(0.0),
                mana_modifier: result.try_read::<f32>(6).unwrap_or(0.0),
                armor_modifier: result.try_read::<f32>(7).unwrap_or(0.0),
                damage_modifier: result.try_read::<f32>(8).unwrap_or(0.0),
                creature_difficulty_id: result.try_read::<i32>(9).unwrap_or(0),
                type_flags: result.try_read::<u32>(10).unwrap_or(0),
                type_flags2: result.try_read::<u32>(11).unwrap_or(0),
                loot_id: result.try_read::<u32>(12).unwrap_or(0),
                pickpocket_loot_id: result.try_read::<u32>(13).unwrap_or(0),
                skin_loot_id: result.try_read::<u32>(14).unwrap_or(0),
                gold_min: result.try_read::<u32>(15).unwrap_or(0),
                gold_max: result.try_read::<u32>(16).unwrap_or(0),
                static_flags: [
                    result.try_read::<u32>(17).unwrap_or(0),
                    result.try_read::<u32>(18).unwrap_or(0),
                    result.try_read::<u32>(19).unwrap_or(0),
                    result.try_read::<u32>(20).unwrap_or(0),
                    result.try_read::<u32>(21).unwrap_or(0),
                    result.try_read::<u32>(22).unwrap_or(0),
                    result.try_read::<u32>(23).unwrap_or(0),
                    result.try_read::<u32>(24).unwrap_or(0),
                ],
            });

            if !result.next_row() {
                break;
            }
        }

        Ok(Self::from_records(
            records,
            classification_damage_modifier_for_entry,
        ))
    }

    pub fn get_like_cpp(
        &self,
        entry: u32,
        difficulty_id: u8,
    ) -> Option<&CreatureDifficultyRecordLikeCpp> {
        self.records.get(&(entry, difficulty_id))
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use rand::{SeedableRng, rngs::StdRng};

    use super::*;

    #[test]
    fn creature_classification_store_maps_template_entries_like_cpp() {
        let store = CreatureTemplateClassificationStoreLikeCpp::from_entries([(100, 0), (101, 4)]);
        assert_eq!(store.len(), 2);
        assert!(!store.is_empty());
        assert_eq!(store.classification_for_entry(100), Some(0));
        assert_eq!(store.classification_for_entry(101), Some(4));
        assert_eq!(store.classification_for_entry(999), None);
    }

    #[test]
    fn creature_template_lifecycle_store_preserves_cpp_field_mapping_and_vehicle_id() {
        let store = CreatureTemplateLifecycleStoreLikeCpp::from_templates([
            CreatureTemplateLifecycleRecordLikeCpp {
                entry: 42,
                name: "C++ Template".to_string(),
                ai_name: "SmartAI".to_string(),
                script_name: "npc_cpp_template".to_string(),
                faction: 35,
                npc_flags: 0x1_0000_0040,
                speed_walk: 1.0,
                speed_run: 1.14286,
                scale: 1.25,
                classification: 4,
                damage_school: wow_constants::spell::SpellSchools::Fire as u8,
                unit_flags: 0x0000_0200,
                unit_flags2: 0x0000_0800,
                unit_flags3: 0x0000_0002,
                creature_type: 7,
                unit_class: 2,
                vehicle_id: 900,
                movement_type: 1,
                ground_movement_type: CreatureGroundMovementType::Hover as u8,
                swim_allowed: false,
                flight_movement_type: CreatureFlightMovementType::CanFly as u8,
                flags_extra: 0x20,
                string_id: "template_string".to_string(),
                regen_health: true,
                spells: [0; MAX_CREATURE_SPELLS_LIKE_CPP],
                models: Vec::new(),
            },
        ]);

        let template = store.get(42).expect("template row retained");
        assert_eq!(template.name, "C++ Template");
        assert_eq!(template.ai_name, "SmartAI");
        assert_eq!(template.script_name, "npc_cpp_template");
        assert_eq!(template.faction, 35);
        assert_eq!(template.npc_flags, 0x1_0000_0040);
        assert_eq!(template.speed_walk, 1.0);
        assert_eq!(template.speed_run, 1.14286);
        assert_eq!(template.scale, 1.25);
        assert_eq!(template.classification, 4);
        assert_eq!(
            template.damage_school,
            wow_constants::spell::SpellSchools::Fire as u8
        );
        assert_eq!(template.unit_flags, 0x0000_0200);
        assert_eq!(template.unit_flags2, 0x0000_0800);
        assert_eq!(template.unit_flags3, 0x0000_0002);
        assert_eq!(template.creature_type, 7);
        assert_eq!(template.unit_class, 2);
        assert_eq!(template.vehicle_id, 900);
        assert_eq!(template.movement_type, 1);
        assert_eq!(
            template.ground_movement_type,
            CreatureGroundMovementType::Hover as u8
        );
        assert!(!template.swim_allowed);
        assert_eq!(
            template.flight_movement_type,
            CreatureFlightMovementType::CanFly as u8
        );
        assert_eq!(template.flags_extra, 0x20);
        assert_eq!(template.string_id, "template_string");
        assert!(template.regen_health);
    }

    #[test]
    fn creature_template_lifecycle_normalizes_invalid_flight_like_cpp() {
        let mut invalid = CreatureTemplateLifecycleRecordLikeCpp {
            entry: 43,
            name: "invalid flight".to_string(),
            ai_name: String::new(),
            script_name: String::new(),
            faction: 35,
            npc_flags: 0,
            speed_walk: 1.0,
            speed_run: 1.0,
            scale: 1.0,
            classification: 0,
            damage_school: wow_constants::spell::SpellSchools::Normal as u8,
            unit_flags: 0,
            unit_flags2: 0,
            unit_flags3: 0,
            creature_type: 0,
            unit_class: 1,
            vehicle_id: 0,
            movement_type: 0,
            ground_movement_type: 0,
            swim_allowed: true,
            flight_movement_type: CREATURE_FLIGHT_MOVEMENT_TYPE_MAX_LIKE_CPP,
            flags_extra: 0,
            string_id: String::new(),
            regen_health: true,
            spells: [0; MAX_CREATURE_SPELLS_LIKE_CPP],
            models: Vec::new(),
        };
        invalid = invalid.normalize_like_cpp();

        assert_eq!(
            invalid.flight_movement_type,
            CreatureFlightMovementType::None as u8
        );
    }

    #[test]
    fn creature_template_lifecycle_spells_skip_oob_and_missing_template_like_cpp() {
        let mut present = CreatureTemplateLifecycleRecordLikeCpp {
            entry: 7,
            name: String::new(),
            ai_name: String::new(),
            script_name: String::new(),
            faction: 0,
            npc_flags: 0,
            speed_walk: 0.0,
            speed_run: 0.0,
            scale: 1.0,
            classification: 0,
            damage_school: wow_constants::spell::SpellSchools::Normal as u8,
            unit_flags: 0,
            unit_flags2: 0,
            unit_flags3: 0,
            creature_type: 0,
            unit_class: 0,
            vehicle_id: 0,
            movement_type: 0,
            ground_movement_type: CreatureGroundMovementType::Run as u8,
            swim_allowed: true,
            flight_movement_type: 0,
            flags_extra: 0,
            string_id: String::new(),
            regen_health: false,
            spells: [0; MAX_CREATURE_SPELLS_LIKE_CPP],
            models: Vec::new(),
        };
        present.apply_spell_row_like_cpp(0, 100);
        present.apply_spell_row_like_cpp(7, 700);
        present.apply_spell_row_like_cpp(8, 800);

        let store = CreatureTemplateLifecycleStoreLikeCpp::from_templates([present]);
        let template = store.get(7).expect("present template");
        assert_eq!(template.spells, [100, 0, 0, 0, 0, 0, 0, 700]);
        assert!(store.get(999).is_none());
    }

    #[test]
    fn creature_template_lifecycle_models_preserve_order_and_first_valid_like_cpp() {
        let mut template = CreatureTemplateLifecycleRecordLikeCpp {
            entry: 8,
            name: String::new(),
            ai_name: String::new(),
            script_name: String::new(),
            faction: 0,
            npc_flags: 0,
            speed_walk: 0.0,
            speed_run: 0.0,
            scale: 1.0,
            classification: 0,
            damage_school: wow_constants::spell::SpellSchools::Normal as u8,
            unit_flags: 0,
            unit_flags2: 0,
            unit_flags3: 0,
            creature_type: 0,
            unit_class: 0,
            vehicle_id: 0,
            movement_type: 0,
            ground_movement_type: CreatureGroundMovementType::Run as u8,
            swim_allowed: true,
            flight_movement_type: 0,
            flags_extra: 0,
            string_id: String::new(),
            regen_health: false,
            spells: [0; MAX_CREATURE_SPELLS_LIKE_CPP],
            models: Vec::new(),
        };
        template.push_model_like_cpp(CreatureTemplateLifecycleModelLikeCpp {
            creature_display_id: 0,
            display_scale: 9.9,
            probability: 100.0,
        });
        template.push_model_like_cpp(CreatureTemplateLifecycleModelLikeCpp {
            creature_display_id: 111,
            display_scale: 1.0,
            probability: 25.0,
        });
        template.push_model_like_cpp(CreatureTemplateLifecycleModelLikeCpp {
            creature_display_id: 222,
            display_scale: 2.0,
            probability: 75.0,
        });

        assert_eq!(template.models.len(), 2);
        assert_eq!(template.models[0].creature_display_id, 111);
        assert_eq!(template.models[1].creature_display_id, 222);
        assert_eq!(
            template.first_model_like_cpp(),
            Some(CreatureTemplateLifecycleModelLikeCpp {
                creature_display_id: 111,
                display_scale: 1.0,
                probability: 25.0,
            })
        );
    }

    #[test]
    fn creature_template_lifecycle_models_normalize_non_positive_display_scale_like_cpp() {
        let mut template = CreatureTemplateLifecycleRecordLikeCpp {
            entry: 9,
            name: String::new(),
            ai_name: String::new(),
            script_name: String::new(),
            faction: 0,
            npc_flags: 0,
            speed_walk: 0.0,
            speed_run: 0.0,
            scale: 1.0,
            classification: 0,
            damage_school: MAX_SPELL_SCHOOL_LIKE_CPP,
            unit_flags: 0,
            unit_flags2: 0,
            unit_flags3: 0,
            creature_type: 0,
            unit_class: 0,
            vehicle_id: 0,
            movement_type: 0,
            ground_movement_type: CreatureGroundMovementType::Run as u8,
            swim_allowed: true,
            flight_movement_type: 0,
            flags_extra: 0,
            string_id: String::new(),
            regen_health: false,
            spells: [0; MAX_CREATURE_SPELLS_LIKE_CPP],
            models: vec![CreatureTemplateLifecycleModelLikeCpp {
                creature_display_id: 333,
                display_scale: -2.0,
                probability: 1.0,
            }],
        };
        let normalized_store =
            CreatureTemplateLifecycleStoreLikeCpp::from_templates([template.clone()]);
        assert_eq!(
            normalized_store.get(9).expect("template").models[0].display_scale,
            1.0
        );
        assert_eq!(
            normalized_store.get(9).expect("template").damage_school,
            wow_constants::spell::SpellSchools::Normal as u8,
            "C++ ObjectMgr clamps invalid creature_template.dmgschool to SPELL_SCHOOL_NORMAL"
        );

        template.models.clear();
        template.push_model_like_cpp(CreatureTemplateLifecycleModelLikeCpp {
            creature_display_id: 444,
            display_scale: 0.0,
            probability: 1.0,
        });
        assert_eq!(template.models[0].display_scale, 1.0);
    }

    #[test]
    fn creature_template_sparring_store_validates_rows_like_cpp() {
        let store = CreatureTemplateSparringStoreLikeCpp::from_rows_like_cpp(
            [
                (10, 35.5),
                (10, 75.0),
                (11, 20.0),
                (12, 0.0),
                (13, -5.0),
                (14, 100.1),
            ],
            |entry| matches!(entry, 10 | 12 | 13 | 14),
        );

        assert_eq!(store.len(), 2);
        assert_eq!(store.values_for_entry_like_cpp(10), Some(&[35.5, 75.0][..]));
        assert_eq!(store.values_for_entry_like_cpp(11), None);
        assert_eq!(store.values_for_entry_like_cpp(12), None);
        assert_eq!(store.values_for_entry_like_cpp(13), None);
        assert_eq!(store.values_for_entry_like_cpp(14), None);
    }

    #[test]
    fn creature_template_sparring_selection_preserves_float_percent_like_cpp() {
        let store = CreatureTemplateSparringStoreLikeCpp::from_rows_like_cpp(
            [(10, 35.5), (10, 75.25)],
            |entry| entry == 10,
        );

        assert_eq!(store.select_for_entry_by_index_like_cpp(10, 0), Some(35.5));
        assert_eq!(store.select_for_entry_by_index_like_cpp(10, 1), Some(75.25));
        assert_eq!(store.select_for_entry_by_index_like_cpp(10, 2), Some(35.5));
        assert_eq!(store.select_for_entry_by_index_like_cpp(999, 0), None);
    }

    #[test]
    fn creature_classification_damage_rates_match_cpp_switch_and_default_elite() {
        let rates = CreatureClassificationDamageRatesLikeCpp {
            normal: 1.0,
            elite: 2.0,
            rare_elite: 3.0,
            obsolete: 4.0,
            rare: 5.0,
            trivial: 6.0,
            minus_mob: 7.0,
        };

        assert_eq!(rates.modifier_for_classification_like_cpp(0), 1.0);
        assert_eq!(rates.modifier_for_classification_like_cpp(1), 2.0);
        assert_eq!(rates.modifier_for_classification_like_cpp(2), 3.0);
        assert_eq!(rates.modifier_for_classification_like_cpp(3), 4.0);
        assert_eq!(rates.modifier_for_classification_like_cpp(4), 5.0);
        assert_eq!(rates.modifier_for_classification_like_cpp(5), 6.0);
        assert_eq!(rates.modifier_for_classification_like_cpp(6), 7.0);
        assert_eq!(rates.modifier_for_classification_like_cpp(99), 2.0);
    }

    #[test]
    fn creature_classification_health_rates_match_cpp_switch_and_default_elite() {
        let rates = CreatureClassificationHealthRatesLikeCpp {
            normal: 1.0,
            elite: 2.0,
            rare_elite: 3.0,
            obsolete: 4.0,
            rare: 5.0,
            trivial: 6.0,
            minus_mob: 7.0,
        };

        assert_eq!(rates.modifier_for_classification_like_cpp(0), 1.0);
        assert_eq!(rates.modifier_for_classification_like_cpp(1), 2.0);
        assert_eq!(rates.modifier_for_classification_like_cpp(2), 3.0);
        assert_eq!(rates.modifier_for_classification_like_cpp(3), 4.0);
        assert_eq!(rates.modifier_for_classification_like_cpp(4), 5.0);
        assert_eq!(rates.modifier_for_classification_like_cpp(5), 6.0);
        assert_eq!(rates.modifier_for_classification_like_cpp(6), 7.0);
        assert_eq!(rates.modifier_for_classification_like_cpp(99), 2.0);
    }

    fn base_difficulty_record() -> CreatureDifficultyRecordLikeCpp {
        CreatureDifficultyRecordLikeCpp {
            entry: 1,
            difficulty_id: 0,
            min_level: 1,
            max_level: 1,
            health_scaling_expansion: 0,
            health_modifier: 1.0,
            mana_modifier: 1.0,
            armor_modifier: 1.0,
            damage_modifier: 1.0,
            creature_difficulty_id: 0,
            type_flags: 0,
            type_flags2: 0,
            loot_id: 0,
            pickpocket_loot_id: 0,
            skin_loot_id: 0,
            gold_min: 0,
            gold_max: 0,
            static_flags: [0; 8],
        }
    }

    #[test]
    fn creature_difficulty_health_scaling_current_and_invalid_match_cpp() {
        let current = CreatureDifficultyRecordLikeCpp {
            health_scaling_expansion: CREATURE_EXPANSION_LEVEL_CURRENT_LIKE_CPP,
            ..base_difficulty_record()
        }
        .normalize_like_cpp(1.0);
        assert_eq!(
            current.health_scaling_expansion_index_like_cpp(),
            CREATURE_CURRENT_EXPANSION_LIKE_CPP
        );

        let invalid_low = CreatureDifficultyRecordLikeCpp {
            health_scaling_expansion: CREATURE_EXPANSION_LEVEL_CURRENT_LIKE_CPP - 1,
            ..base_difficulty_record()
        }
        .normalize_like_cpp(1.0);
        assert_eq!(invalid_low.health_scaling_expansion, 0);
        assert_eq!(invalid_low.health_scaling_expansion_index_like_cpp(), 0);

        let invalid_high = CreatureDifficultyRecordLikeCpp {
            health_scaling_expansion: CREATURE_CURRENT_EXPANSION_LIKE_CPP as i32 + 1,
            ..base_difficulty_record()
        }
        .normalize_like_cpp(1.0);
        assert_eq!(invalid_high.health_scaling_expansion, 0);
    }

    #[test]
    fn creature_difficulty_normalizes_min_max_gold_and_damage_modifier_like_cpp() {
        let normalized = CreatureDifficultyRecordLikeCpp {
            min_level: 0,
            max_level: 0,
            health_scaling_expansion: 1,
            damage_modifier: 3.0,
            gold_min: 50,
            gold_max: 10,
            ..base_difficulty_record()
        }
        .normalize_like_cpp(2.0);
        assert_eq!(normalized.min_level, 1);
        assert_eq!(normalized.max_level, 1);
        assert_eq!(normalized.gold_max, 50);
        assert_eq!(normalized.damage_modifier, 6.0);

        let inverted = CreatureDifficultyRecordLikeCpp {
            min_level: 60,
            max_level: 55,
            ..base_difficulty_record()
        }
        .normalize_like_cpp(1.0);
        assert_eq!(inverted.min_level, 55);
        assert_eq!(inverted.max_level, 55);
    }

    #[test]
    fn creature_base_stats_normalize_loaded_rows_but_missing_fallback_stays_zero_like_cpp() {
        let store = CreatureBaseStatsStoreLikeCpp::from_records([(
            10,
            2,
            CreatureBaseStatsRecordLikeCpp {
                base_health: [0, 25, 0],
                base_mana: 30,
                base_armor: 40,
                attack_power: 50,
                ranged_attack_power: 60,
                base_damage: [-1.0, 2.5, -0.25],
            },
        )]);

        let loaded = store.get_like_cpp(10, 2);
        assert_eq!(loaded.base_health, [1, 25, 1]);
        assert_eq!(loaded.base_damage, [0.0, 2.5, 0.0]);
        assert_eq!(loaded.base_mana, 30);
        assert_eq!(loaded.attack_power, 50);
        assert_eq!(loaded.ranged_attack_power, 60);

        let missing = store.get_like_cpp(99, 2);
        assert_eq!(missing.base_health, [0, 0, 0]);
        assert_eq!(missing.base_damage, [0.0, 0.0, 0.0]);
        assert_eq!(missing.base_mana, 0);
        assert_eq!(missing.attack_power, 0);
        assert_eq!(missing.ranged_attack_power, 0);
    }

    #[test]
    fn creature_base_stats_generate_helpers_match_cpp_ceil_and_expansion_index() {
        let stats = CreatureBaseStatsRecordLikeCpp {
            base_health: [100, 200, 300],
            base_mana: 50,
            base_armor: 80,
            attack_power: 0,
            ranged_attack_power: 0,
            base_damage: [1.25, 2.5, 3.75],
        };
        let difficulty = CreatureDifficultyRecordLikeCpp {
            health_scaling_expansion: CREATURE_EXPANSION_LEVEL_CURRENT_LIKE_CPP,
            health_modifier: 1.25,
            mana_modifier: 1.01,
            armor_modifier: 1.1,
            ..base_difficulty_record()
        }
        .normalize_like_cpp(1.0);

        assert_eq!(stats.generate_health_like_cpp(&difficulty), 375);
        assert_eq!(stats.generate_mana_like_cpp(&difficulty), 51);
        assert_eq!(stats.generate_armor_like_cpp(&difficulty), 88);
        assert_eq!(stats.generate_base_damage_like_cpp(&difficulty), 3.75);

        let no_mana = CreatureBaseStatsRecordLikeCpp {
            base_mana: 0,
            ..stats
        };
        assert_eq!(no_mana.generate_mana_like_cpp(&difficulty), 0);
    }

    #[test]
    fn creature_difficulty_store_keys_by_entry_and_difficulty_after_normalization() {
        let store = CreatureDifficultyStoreLikeCpp::from_records(
            [CreatureDifficultyRecordLikeCpp {
                entry: 7,
                difficulty_id: 3,
                min_level: 4,
                max_level: 5,
                damage_modifier: 2.0,
                ..base_difficulty_record()
            }],
            |entry| if entry == 7 { 1.5 } else { 1.0 },
        );

        let record = store.get_like_cpp(7, 3).expect("difficulty row exists");
        assert_eq!(record.min_level, 4);
        assert_eq!(record.max_level, 5);
        assert_eq!(record.damage_modifier, 3.0);
        assert!(store.get_like_cpp(7, 0).is_none());
    }

    #[test]
    fn creature_template_mount_model_selection_matches_cpp_shape() {
        let entry = CreatureTemplateMountEntryLikeCpp {
            entry: 10,
            vehicle_id: 77,
            models: vec![CreatureTemplateMountModelLikeCpp {
                display_id: 1234,
                display_scale: 1.0,
                probability: 0.0,
            }],
        };

        assert_eq!(
            entry.choose_display_id_like_cpp(&mut StdRng::seed_from_u64(1)),
            Some(1234)
        );

        let entry = CreatureTemplateMountEntryLikeCpp {
            entry: 11,
            vehicle_id: 0,
            models: vec![
                CreatureTemplateMountModelLikeCpp {
                    display_id: 1,
                    display_scale: 1.0,
                    probability: 0.0,
                },
                CreatureTemplateMountModelLikeCpp {
                    display_id: 2,
                    display_scale: 1.0,
                    probability: 100.0,
                },
            ],
        };

        assert_eq!(
            entry.choose_display_id_like_cpp(&mut StdRng::seed_from_u64(2)),
            Some(2)
        );
    }

    fn addon_row(owner_id: u64) -> CreatureAddonRowLikeCpp {
        CreatureAddonRowLikeCpp {
            owner_id,
            path_id: 0,
            mount: 0,
            stand_state: 0,
            anim_tier: 0,
            vis_flags: 0,
            sheath_state: 0,
            pvp_flags: 0,
            emote: 0,
            ai_anim_kit: 0,
            movement_anim_kit: 0,
            melee_anim_kit: 0,
            visibility_distance_type: 0,
        }
    }

    #[test]
    fn creature_addon_store_uses_spawn_addon_before_template_like_cpp() {
        let spawn = CreatureAddonRowLikeCpp {
            mount: 1234,
            stand_state: UnitStandStateType::Kneel as u8,
            pvp_flags: UnitPvpFlags::PVP.bits(),
            emote: 77,
            ai_anim_kit: 11,
            movement_anim_kit: 22,
            melee_anim_kit: 33,
            ..addon_row(44)
        };
        let template = CreatureAddonRowLikeCpp {
            mount: 5678,
            stand_state: UnitStandStateType::Sleep as u8,
            pvp_flags: UnitPvpFlags::SANCTUARY.bits(),
            emote: 88,
            ai_anim_kit: 44,
            movement_anim_kit: 55,
            melee_anim_kit: 66,
            ..addon_row(1001)
        };

        let store = CreatureAddonStoreLikeCpp::from_rows_like_cpp(
            [spawn],
            [template],
            |spawn_id| spawn_id == 44,
            |entry| entry == 1001,
            |display_id| matches!(display_id, 1234 | 5678),
            |emote| matches!(emote, 77 | 88),
            |anim_kit_id| matches!(anim_kit_id, 11 | 22 | 33 | 44 | 55 | 66),
        );

        assert_eq!(
            store.get_for_creature_like_cpp(44, 1001),
            Some(CreatureAddonLifecycleRecordLikeCpp {
                path_id: 0,
                mount_display_id: 1234,
                stand_state: UnitStandStateType::Kneel,
                vis_flags: 0,
                anim_tier: 0,
                sheath_state: SheathState::Unarmed,
                pvp_flags: UnitPvpFlags::PVP,
                emote: 77,
                ai_anim_kit_id: 11,
                movement_anim_kit_id: 22,
                melee_anim_kit_id: 33,
                visibility_distance_type: VisibilityDistanceTypeLikeCpp::Normal,
            }),
            "C++ Creature::GetCreatureAddon checks spawn id before template entry"
        );
        assert_eq!(
            store.get_for_creature_like_cpp(0, 1001),
            Some(CreatureAddonLifecycleRecordLikeCpp {
                path_id: 0,
                mount_display_id: 5678,
                stand_state: UnitStandStateType::Sleep,
                vis_flags: 0,
                anim_tier: 0,
                sheath_state: SheathState::Unarmed,
                pvp_flags: UnitPvpFlags::SANCTUARY,
                emote: 88,
                ai_anim_kit_id: 44,
                movement_anim_kit_id: 55,
                melee_anim_kit_id: 66,
                visibility_distance_type: VisibilityDistanceTypeLikeCpp::Normal,
            })
        );
    }

    #[test]
    fn creature_addon_store_normalizes_supported_fields_like_cpp() {
        let row = CreatureAddonRowLikeCpp {
            mount: 9999,
            stand_state: UnitStandStateType::Max as u8,
            anim_tier: MAX_ANIM_TIER_LIKE_CPP,
            vis_flags: 0xff,
            sheath_state: MAX_SHEATH_STATE_LIKE_CPP,
            pvp_flags: 0xff,
            emote: 333,
            ai_anim_kit: 11,
            movement_anim_kit: 22,
            melee_anim_kit: 33,
            visibility_distance_type: VisibilityDistanceTypeLikeCpp::MAX_LIKE_CPP,
            ..addon_row(44)
        };

        let store = CreatureAddonStoreLikeCpp::from_rows_like_cpp(
            [row],
            [],
            |spawn_id| spawn_id == 44,
            |_| false,
            |_| false,
            |_| false,
            |_| false,
        );

        assert_eq!(
            store.get_for_creature_like_cpp(44, 1001),
            Some(CreatureAddonLifecycleRecordLikeCpp {
                path_id: 0,
                mount_display_id: 0,
                stand_state: UnitStandStateType::Stand,
                vis_flags: 0xff,
                anim_tier: 0,
                sheath_state: SheathState::Unarmed,
                pvp_flags: UnitPvpFlags::from_bits_retain(0xff),
                emote: 0,
                ai_anim_kit_id: 0,
                movement_anim_kit_id: 0,
                melee_anim_kit_id: 0,
                visibility_distance_type: VisibilityDistanceTypeLikeCpp::Normal,
            }),
            "C++ invalid mount/emote/stand/anim/sheath/anim-kit/visibility rows are truncated; VisFlags/PvPFlags cover the full byte"
        );
    }

    #[test]
    fn creature_addon_store_mutates_waypoint_spawn_without_path_to_idle_like_cpp() {
        let spawn_without_path = CreatureAddonRowLikeCpp {
            path_id: 0,
            ..addon_row(44)
        };
        let spawn_with_path = CreatureAddonRowLikeCpp {
            path_id: 9001,
            ..addon_row(45)
        };
        let template_without_path = CreatureAddonRowLikeCpp {
            path_id: 0,
            ..addon_row(1001)
        };

        let store = CreatureAddonStoreLikeCpp::from_rows_like_cpp(
            [spawn_without_path, spawn_with_path],
            [template_without_path],
            |spawn_id| matches!(spawn_id, 44 | 45),
            |entry| entry == 1001,
            |_| true,
            |_| true,
            |_| true,
        );

        assert_eq!(
            store.movement_type_after_spawn_addon_load_like_cpp(44, WAYPOINT_MOTION_TYPE_LIKE_CPP),
            IDLE_MOTION_TYPE_LIKE_CPP,
            "C++ LoadCreatureAddons mutates spawn WAYPOINT_MOTION_TYPE to IDLE_MOTION_TYPE when spawn PathId is zero"
        );
        assert_eq!(
            store.movement_type_after_spawn_addon_load_like_cpp(45, WAYPOINT_MOTION_TYPE_LIKE_CPP),
            WAYPOINT_MOTION_TYPE_LIKE_CPP,
            "non-zero spawn PathId preserves C++ waypoint movement"
        );
        assert_eq!(
            store.movement_type_after_spawn_addon_load_like_cpp(0, WAYPOINT_MOTION_TYPE_LIKE_CPP),
            WAYPOINT_MOTION_TYPE_LIKE_CPP,
            "template addon PathId never triggers the spawn CreatureData movementType mutation"
        );
        assert_eq!(
            store
                .get_for_creature_like_cpp(45, 1001)
                .map(|addon| addon.path_id),
            Some(9001),
            "PathId is retained for the future path runtime seam"
        );
    }
}
