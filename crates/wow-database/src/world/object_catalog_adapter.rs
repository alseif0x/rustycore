//! MariaDB adapter for canonical creature and gameobject startup catalogs.
//!
//! Every column is read with the C++ `ObjectMgr` getter width through
//! [`FieldRowLikeCpp`]; a decode failure fails the catalog load (#1436).

use crate::WorldDatabase;
use crate::field_like_cpp::FieldRowLikeCpp;
use crate::result::collect_rows_like_cpp;
use anyhow::Result;
use std::sync::Arc;
use wow_persistence::*;

/// C++ `ObjectMgr::LoadCreatureTemplate` (ObjectMgr.cpp:403-470) columns of
/// the lifecycle catalog. `creature_template_movement` columns are selected
/// raw: C++ checks `IsNull()` and keeps the `CreatureMovementData` default
/// (Creature.cpp:58), while `COALESCE(TINYINT UNSIGNED, 1)` is DECIMAL.
const CREATURE_TEMPLATE_LIFECYCLE_SQL_LIKE_CPP: &str = concat!(
    "SELECT ct.entry, ct.name, ct.AIName, ct.ScriptName, ct.RequiredExpansion, ct.faction, ",
    "ct.npcflag, ct.speed_walk, ct.speed_run, ct.scale, ct.Classification, ct.dmgschool, ",
    "ct.unit_flags, ct.unit_flags2, ct.unit_flags3, ct.`type`, ct.family, ct.trainer_class, ",
    "ct.unit_class, ct.VehicleId, ct.MovementType, ctm.Ground, ctm.Swim, ctm.Flight, ",
    "ctm.Rooted, ctm.Chase, ctm.Random, ctm.InteractionPauseTimer, ct.flags_extra, ",
    "ct.StringId, ct.RegenHealth, ct.BaseVariance FROM creature_template ct ",
    "LEFT JOIN creature_template_movement ctm ON ct.entry = ctm.CreatureId"
);

/// C++ `ObjectMgr::LoadGameObjectTemplate` (ObjectMgr.cpp:7556-7612).
const GAMEOBJECT_TEMPLATE_LIFECYCLE_SQL_LIKE_CPP: &str = concat!(
    "SELECT entry, type, displayId, name, size, Data0, Data1, Data2, Data3, Data4, Data5, ",
    "Data6, Data7, Data8, Data9, Data10, Data11, Data12, Data13, Data14, Data15, Data16, ",
    "Data17, Data18, Data19, Data20, Data21, Data22, Data23, Data24, Data25, Data26, Data27, ",
    "Data28, Data29, Data30, Data31, Data32, Data33, Data34, ContentTuningId, AIName, ",
    "ScriptName, StringId FROM gameobject_template"
);

const CREATURE_ADDON_SQL_LIKE_CPP: &str = "SELECT guid, PathId, mount, StandState, AnimTier, VisFlags, SheathState, PvPFlags, emote, aiAnimKit, movementAnimKit, meleeAnimKit, visibilityDistanceType, auras FROM creature_addon";
const CREATURE_TEMPLATE_ADDON_SQL_LIKE_CPP: &str = "SELECT entry, PathId, mount, StandState, AnimTier, VisFlags, SheathState, PvPFlags, emote, aiAnimKit, movementAnimKit, meleeAnimKit, visibilityDistanceType, auras FROM creature_template_addon";

/// One lifecycle `CreatureTemplate` row (ObjectMgr.cpp:405-466 getters).
fn creature_template_row_like_cpp<R: FieldRowLikeCpp + ?Sized>(
    row: &R,
) -> Result<CreatureTemplatePersistenceRowLikeCpp> {
    Ok(CreatureTemplatePersistenceRowLikeCpp {
        entry: row.get_like_cpp(0, "creature_template.entry")?,
        name: row.get_string_like_cpp(1, "creature_template.name")?,
        ai_name: row.get_string_like_cpp(2, "creature_template.AIName")?,
        script_name: row.get_string_like_cpp(3, "creature_template.ScriptName")?,
        // C++ `GetUInt32()` over a signed INT; the represented expansion is
        // a u8, so an out-of-range value fails instead of wrapping.
        required_expansion: row.get_like_cpp(4, "creature_template.RequiredExpansion")?,
        faction: u32::from(row.get_like_cpp::<u16>(5, "creature_template.faction")?),
        npc_flags: row.get_like_cpp(6, "creature_template.npcflag")?,
        speed_walk: row.get_like_cpp(7, "creature_template.speed_walk")?,
        speed_run: row.get_like_cpp(8, "creature_template.speed_run")?,
        scale: row.get_like_cpp(9, "creature_template.scale")?,
        classification: u32::from(row.get_like_cpp::<u8>(10, "creature_template.Classification")?),
        // `uint32(fields[16].GetInt8())` over a signed TINYINT.
        damage_school: row
            .get_like_cpp::<i8>(11, "creature_template.dmgschool")?
            .cast_unsigned(),
        unit_flags: row.get_like_cpp(12, "creature_template.unit_flags")?,
        unit_flags2: row.get_like_cpp(13, "creature_template.unit_flags2")?,
        unit_flags3: row.get_like_cpp(14, "creature_template.unit_flags3")?,
        creature_type: u32::from(row.get_like_cpp::<u8>(15, "creature_template.type")?),
        // `CreatureFamily(fields[25].GetInt32())` over a signed INT.
        family: row
            .get_like_cpp::<i32>(16, "creature_template.family")?
            .cast_unsigned(),
        trainer_class: row.get_like_cpp(17, "creature_template.trainer_class")?,
        unit_class: row.get_like_cpp(18, "creature_template.unit_class")?,
        vehicle_id: row.get_like_cpp(19, "creature_template.VehicleId")?,
        movement_type: row.get_like_cpp(20, "creature_template.MovementType")?,
        ground_movement_type: row
            .get_nullable_like_cpp(21, "creature_template_movement.Ground")?
            .unwrap_or(1),
        swim_allowed: row
            .get_nullable_bool_like_cpp(22, "creature_template_movement.Swim")?
            .unwrap_or(true),
        flight_movement_type: row
            .get_nullable_like_cpp(23, "creature_template_movement.Flight")?
            .unwrap_or(0),
        rooted: row
            .get_nullable_bool_like_cpp(24, "creature_template_movement.Rooted")?
            .unwrap_or(false),
        chase_movement_type: row
            .get_nullable_like_cpp(25, "creature_template_movement.Chase")?
            .unwrap_or(0),
        random_movement_type: row
            .get_nullable_like_cpp(26, "creature_template_movement.Random")?
            .unwrap_or(0),
        interaction_pause_timer_ms: row
            .get_nullable_like_cpp(27, "creature_template_movement.InteractionPauseTimer")?
            .unwrap_or(180_000),
        flags_extra: row.get_like_cpp(28, "creature_template.flags_extra")?,
        string_id: row.get_string_like_cpp(29, "creature_template.StringId")?,
        regen_health: row.get_bool_like_cpp(30, "creature_template.RegenHealth")?,
        base_variance: row.get_like_cpp(31, "creature_template.BaseVariance")?,
    })
}

/// One lifecycle `GameObjectTemplate` row (ObjectMgr.cpp:7591-7612 getters).
fn gameobject_template_row_like_cpp<R: FieldRowLikeCpp + ?Sized>(
    row: &R,
) -> Result<GameObjectTemplatePersistenceRowLikeCpp> {
    let mut data = [0_u32; MAX_GAMEOBJECT_DATA_PERSISTENCE_LIKE_CPP];
    for (index, slot) in data.iter_mut().enumerate() {
        // `got.raw.data[i] = fields[8 + i].GetUInt32()` over a signed INT:
        // sqlx rejected every Data column as u32 and they became 0 (#1436).
        *slot = row.get_like_cpp(5 + index, "gameobject_template.Data")?;
    }
    Ok(GameObjectTemplatePersistenceRowLikeCpp {
        entry: row.get_like_cpp(0, "gameobject_template.entry")?,
        go_type: u32::from(row.get_like_cpp::<u8>(1, "gameobject_template.type")?),
        display_id: row.get_like_cpp(2, "gameobject_template.displayId")?,
        name: row.get_string_like_cpp(3, "gameobject_template.name")?,
        size: row.get_like_cpp(4, "gameobject_template.size")?,
        data,
        // `got.ContentTuningId = fields[43].GetInt32()` over a signed INT.
        content_tuning_id: row
            .get_like_cpp::<i32>(40, "gameobject_template.ContentTuningId")?
            .cast_unsigned(),
        ai_name: row.get_string_like_cpp(41, "gameobject_template.AIName")?,
        script_name: row.get_string_like_cpp(42, "gameobject_template.ScriptName")?,
        string_id: row.get_string_like_cpp(43, "gameobject_template.StringId")?,
    })
}

/// One `CreatureAddon` row (ObjectMgr.cpp:779-796 and 1242-1270 getters).
fn creature_addon_row_like_cpp<R: FieldRowLikeCpp + ?Sized>(
    row: &R,
) -> Result<CreatureAddonPersistenceRowLikeCpp> {
    Ok(CreatureAddonPersistenceRowLikeCpp {
        owner_id: row.get_like_cpp(0, "creature_addon.guid/entry")?,
        path_id: row.get_like_cpp(1, "creature_addon.PathId")?,
        mount: row.get_like_cpp(2, "creature_addon.mount")?,
        stand_state: row.get_like_cpp(3, "creature_addon.StandState")?,
        anim_tier: row.get_like_cpp(4, "creature_addon.AnimTier")?,
        vis_flags: row.get_like_cpp(5, "creature_addon.VisFlags")?,
        sheath_state: row.get_like_cpp(6, "creature_addon.SheathState")?,
        pvp_flags: row.get_like_cpp(7, "creature_addon.PvPFlags")?,
        emote: row.get_like_cpp(8, "creature_addon.emote")?,
        // `GetUInt16()` over a signed SMALLINT.
        ai_anim_kit: row.get_like_cpp(9, "creature_addon.aiAnimKit")?,
        movement_anim_kit: row.get_like_cpp(10, "creature_addon.movementAnimKit")?,
        melee_anim_kit: row.get_like_cpp(11, "creature_addon.meleeAnimKit")?,
        visibility_distance_type: row.get_like_cpp(12, "creature_addon.visibilityDistanceType")?,
        auras: row.get_string_like_cpp(13, "creature_addon.auras")?,
    })
}

pub struct MariaDbWorldObjectCatalogPersistenceAdapterLikeCpp {
    world_db: Arc<WorldDatabase>,
}

impl MariaDbWorldObjectCatalogPersistenceAdapterLikeCpp {
    pub fn new(world_db: Arc<WorldDatabase>) -> Self {
        Self { world_db }
    }

    async fn classifications(&self) -> Result<Vec<(u32, u32)>> {
        collect_rows_like_cpp(
            self.world_db
                .direct_query("SELECT entry, Classification FROM creature_template")
                .await?,
            "creature_template",
            |row| {
                Ok((
                    row.get_like_cpp(0, "creature_template.entry")?,
                    u32::from(row.get_like_cpp::<u8>(1, "creature_template.Classification")?),
                ))
            },
        )
    }

    async fn creature_templates(&self) -> Result<CreatureTemplateCatalogPersistenceRowsLikeCpp> {
        let templates = collect_rows_like_cpp(
            self.world_db
                .direct_query(CREATURE_TEMPLATE_LIFECYCLE_SQL_LIKE_CPP)
                .await?,
            "creature_template",
            creature_template_row_like_cpp,
        )?;
        // ObjectMgr::LoadCreatureTemplateSpells (ObjectMgr.cpp:575-618).
        let spells = collect_rows_like_cpp(
            self.world_db
                .direct_query("SELECT CreatureID, `Index`, Spell FROM creature_template_spell")
                .await?,
            "creature_template_spell",
            |row| {
                Ok(CreatureTemplateSpellPersistenceRowLikeCpp {
                    creature_id: row.get_like_cpp(0, "creature_template_spell.CreatureID")?,
                    index: row.get_like_cpp(1, "creature_template_spell.Index")?,
                    spell_id: row.get_like_cpp(2, "creature_template_spell.Spell")?,
                })
            },
        )?;
        // ObjectMgr::LoadCreatureTemplateModels (ObjectMgr.cpp:620-650).
        let models = collect_rows_like_cpp(
            self.world_db.direct_query("SELECT CreatureID, CreatureDisplayID, DisplayScale, Probability FROM creature_template_model ORDER BY Idx ASC").await?,
            "creature_template_model",
            |row| {
                Ok(CreatureTemplateModelPersistenceRowLikeCpp {
                    creature_id: row.get_like_cpp(0, "creature_template_model.CreatureID")?,
                    display_id: row.get_like_cpp(1, "creature_template_model.CreatureDisplayID")?,
                    display_scale: row.get_like_cpp(2, "creature_template_model.DisplayScale")?,
                    probability: row.get_like_cpp(3, "creature_template_model.Probability")?,
                })
            },
        )?;
        Ok(CreatureTemplateCatalogPersistenceRowsLikeCpp {
            templates,
            spells,
            models,
        })
    }

    /// ObjectMgr::LoadCreatureTemplateSparring (ObjectMgr.cpp:899-938).
    async fn sparring(&self) -> Result<Vec<(u32, f32)>> {
        collect_rows_like_cpp(
            self.world_db
                .direct_query(
                    "SELECT Entry, NoNPCDamageBelowHealthPct FROM creature_template_sparring",
                )
                .await?,
            "creature_template_sparring",
            |row| {
                Ok((
                    row.get_like_cpp(0, "creature_template_sparring.Entry")?,
                    row.get_like_cpp(1, "creature_template_sparring.NoNPCDamageBelowHealthPct")?,
                ))
            },
        )
    }

    async fn gameobject_templates(
        &self,
    ) -> Result<GameObjectTemplateCatalogPersistenceRowsLikeCpp> {
        let templates = collect_rows_like_cpp(
            self.world_db
                .direct_query(GAMEOBJECT_TEMPLATE_LIFECYCLE_SQL_LIKE_CPP)
                .await?,
            "gameobject_template",
            gameobject_template_row_like_cpp,
        )?;
        // ObjectMgr::LoadGameObjectTemplateAddons (ObjectMgr.cpp:7770-7805).
        let addons = collect_rows_like_cpp(
            self.world_db.direct_query("SELECT entry, faction, flags, WorldEffectID, AIAnimKitID FROM gameobject_template_addon").await?,
            "gameobject_template_addon",
            |row| {
                Ok(GameObjectTemplateAddonPersistenceRowLikeCpp {
                    entry: row.get_like_cpp(0, "gameobject_template_addon.entry")?,
                    faction: u32::from(
                        row.get_like_cpp::<u16>(1, "gameobject_template_addon.faction")?,
                    ),
                    flags: row.get_like_cpp(2, "gameobject_template_addon.flags")?,
                    world_effect_id: row.get_like_cpp(3, "gameobject_template_addon.WorldEffectID")?,
                    // C++ `GetUInt32()`; the represented anim kit is a u16.
                    anim_kit_id: row.get_like_cpp(4, "gameobject_template_addon.AIAnimKitID")?,
                })
            },
        )?;
        Ok(GameObjectTemplateCatalogPersistenceRowsLikeCpp { templates, addons })
    }

    /// ObjectMgr::LoadGameObjectOverrides (ObjectMgr.cpp:7856-7890).
    async fn gameobject_overrides(&self) -> Result<Vec<GameObjectOverridePersistenceRowLikeCpp>> {
        collect_rows_like_cpp(
            self.world_db
                .direct_query("SELECT spawnId, faction, flags FROM gameobject_overrides")
                .await?,
            "gameobject_overrides",
            |row| {
                Ok(GameObjectOverridePersistenceRowLikeCpp {
                    spawn_id: row.get_like_cpp(0, "gameobject_overrides.spawnId")?,
                    faction: u32::from(row.get_like_cpp::<u16>(1, "gameobject_overrides.faction")?),
                    flags: row.get_like_cpp(2, "gameobject_overrides.flags")?,
                })
            },
        )
    }

    /// ObjectMgr::LoadCreatureTemplateDifficulty (ObjectMgr.cpp:940-1035).
    async fn difficulty(&self) -> Result<Vec<CreatureDifficultyPersistenceRowLikeCpp>> {
        collect_rows_like_cpp(
            self.world_db.direct_query("SELECT Entry, DifficultyID, MinLevel, MaxLevel, HealthScalingExpansion, HealthModifier, ManaModifier, ArmorModifier, DamageModifier, CreatureDifficultyID, TypeFlags, TypeFlags2, LootID, PickPocketLootID, SkinLootID, GoldMin, GoldMax, StaticFlags1, StaticFlags2, StaticFlags3, StaticFlags4, StaticFlags5, StaticFlags6, StaticFlags7, StaticFlags8 FROM creature_template_difficulty ORDER BY Entry").await?,
            "creature_template_difficulty",
            |row| {
                let mut static_flags = [0; 8];
                for (index, flags) in static_flags.iter_mut().enumerate() {
                    *flags = row.get_like_cpp(17 + index, "creature_template_difficulty.StaticFlags")?;
                }
                Ok(CreatureDifficultyPersistenceRowLikeCpp {
                    entry: row.get_like_cpp(0, "creature_template_difficulty.Entry")?,
                    difficulty_id: row.get_like_cpp(1, "creature_template_difficulty.DifficultyID")?,
                    // `GetUInt8()` over a signed TINYINT.
                    min_level: row.get_like_cpp(2, "creature_template_difficulty.MinLevel")?,
                    max_level: row.get_like_cpp(3, "creature_template_difficulty.MaxLevel")?,
                    health_scaling_expansion: row
                        .get_like_cpp(4, "creature_template_difficulty.HealthScalingExpansion")?,
                    health_modifier: row.get_like_cpp(5, "creature_template_difficulty.HealthModifier")?,
                    mana_modifier: row.get_like_cpp(6, "creature_template_difficulty.ManaModifier")?,
                    armor_modifier: row.get_like_cpp(7, "creature_template_difficulty.ArmorModifier")?,
                    damage_modifier: row.get_like_cpp(8, "creature_template_difficulty.DamageModifier")?,
                    creature_difficulty_id: row
                        .get_like_cpp(9, "creature_template_difficulty.CreatureDifficultyID")?,
                    type_flags: row.get_like_cpp(10, "creature_template_difficulty.TypeFlags")?,
                    type_flags2: row.get_like_cpp(11, "creature_template_difficulty.TypeFlags2")?,
                    loot_id: row.get_like_cpp(12, "creature_template_difficulty.LootID")?,
                    pickpocket_loot_id: row
                        .get_like_cpp(13, "creature_template_difficulty.PickPocketLootID")?,
                    skin_loot_id: row.get_like_cpp(14, "creature_template_difficulty.SkinLootID")?,
                    gold_min: row.get_like_cpp(15, "creature_template_difficulty.GoldMin")?,
                    gold_max: row.get_like_cpp(16, "creature_template_difficulty.GoldMax")?,
                    static_flags,
                })
            },
        )
    }

    /// ObjectMgr::LoadCreatureClassLevelStats (ObjectMgr.cpp:9996-10045).
    async fn base_stats(&self) -> Result<Vec<CreatureBaseStatsPersistenceRowLikeCpp>> {
        collect_rows_like_cpp(
            self.world_db.direct_query("SELECT level, class, basehp0, basehp1, basehp2, basemana, basearmor, attackpower, rangedattackpower, damage_base, damage_exp1, damage_exp2 FROM creature_classlevelstats").await?,
            "creature_classlevelstats",
            |row| {
                let uint16 = |column, field| -> Result<u32> {
                    Ok(u32::from(row.get_like_cpp::<u16>(column, field)?))
                };
                let mut base_health = [0; 3];
                for (index, health) in base_health.iter_mut().enumerate() {
                    *health = uint16(2 + index, "creature_classlevelstats.basehp")?;
                }
                let mut base_damage = [0.0; 3];
                for (index, damage) in base_damage.iter_mut().enumerate() {
                    *damage = row.get_like_cpp(9 + index, "creature_classlevelstats.damage")?;
                }
                Ok(CreatureBaseStatsPersistenceRowLikeCpp {
                    level: row.get_like_cpp(0, "creature_classlevelstats.level")?,
                    unit_class: row.get_like_cpp(1, "creature_classlevelstats.class")?,
                    base_health,
                    base_mana: uint16(5, "creature_classlevelstats.basemana")?,
                    base_armor: uint16(6, "creature_classlevelstats.basearmor")?,
                    attack_power: uint16(7, "creature_classlevelstats.attackpower")?,
                    ranged_attack_power: uint16(8, "creature_classlevelstats.rangedattackpower")?,
                    base_damage,
                })
            },
        )
    }

    async fn mounts(&self) -> Result<Vec<CreatureMountPersistenceRowLikeCpp>> {
        // LEFT JOIN: a template without a model row keeps 0 like `GetUInt32()` on NULL.
        collect_rows_like_cpp(
            self.world_db.direct_query("SELECT ct.entry, ct.VehicleId, ctm.CreatureDisplayID, ctm.DisplayScale, ctm.Probability FROM creature_template ct LEFT JOIN creature_template_model ctm ON ct.entry = ctm.CreatureID ORDER BY ct.entry, ctm.Idx").await?,
            "creature_template",
            |row| {
                Ok(CreatureMountPersistenceRowLikeCpp {
                    entry: row.get_like_cpp(0, "creature_template.entry")?,
                    vehicle_id: row.get_like_cpp(1, "creature_template.VehicleId")?,
                    display_id: row.get_like_cpp(2, "creature_template_model.CreatureDisplayID")?,
                    display_scale: row.get_like_cpp(3, "creature_template_model.DisplayScale")?,
                    probability: row.get_like_cpp(4, "creature_template_model.Probability")?,
                })
            },
        )
    }

    /// ObjectMgr::LoadCreatureModelInfo (ObjectMgr.cpp:1734-1800).
    async fn model_info(&self) -> Result<Vec<CreatureModelInfoPersistenceRowLikeCpp>> {
        collect_rows_like_cpp(
            self.world_db.direct_query("SELECT DisplayID, BoundingRadius, CombatReach, DisplayID_Other_Gender FROM creature_model_info").await?,
            "creature_model_info",
            |row| {
                Ok(CreatureModelInfoPersistenceRowLikeCpp {
                    display_id: row.get_like_cpp(0, "creature_model_info.DisplayID")?,
                    bounding_radius: row.get_like_cpp(1, "creature_model_info.BoundingRadius")?,
                    combat_reach: row.get_like_cpp(2, "creature_model_info.CombatReach")?,
                    display_id_other_gender: row
                        .get_like_cpp(3, "creature_model_info.DisplayID_Other_Gender")?,
                })
            },
        )
    }

    async fn addons(&self) -> Result<CreatureAddonCatalogPersistenceRowsLikeCpp> {
        Ok(CreatureAddonCatalogPersistenceRowsLikeCpp {
            spawn_addons: self
                .addon_query(CREATURE_ADDON_SQL_LIKE_CPP, "creature_addon")
                .await?,
            template_addons: self
                .addon_query(
                    CREATURE_TEMPLATE_ADDON_SQL_LIKE_CPP,
                    "creature_template_addon",
                )
                .await?,
        })
    }

    async fn addon_query(
        &self,
        sql: &str,
        table: &str,
    ) -> Result<Vec<CreatureAddonPersistenceRowLikeCpp>> {
        collect_rows_like_cpp(
            self.world_db.direct_query(sql).await?,
            table,
            creature_addon_row_like_cpp,
        )
    }

    /// ObjectMgr::LoadEquipmentTemplates (ObjectMgr.cpp:1504-1590).
    async fn equipment(&self) -> Result<Vec<CreatureEquipmentPersistenceRowLikeCpp>> {
        collect_rows_like_cpp(
            self.world_db.direct_query("SELECT CreatureID, ID, ItemID1, AppearanceModID1, ItemVisual1, ItemID2, AppearanceModID2, ItemVisual2, ItemID3, AppearanceModID3, ItemVisual3 FROM creature_equip_template").await?,
            "creature_equip_template",
            |row| {
                let mut items = [CreatureEquipmentItemPersistenceLikeCpp::default(); 3];
                for (slot, item) in items.iter_mut().enumerate() {
                    let base = 2 + slot * 3;
                    *item = CreatureEquipmentItemPersistenceLikeCpp {
                        item_id: row.get_like_cpp(base, "creature_equip_template.ItemID")?,
                        appearance_mod_id: row
                            .get_like_cpp(base + 1, "creature_equip_template.AppearanceModID")?,
                        item_visual: row.get_like_cpp(base + 2, "creature_equip_template.ItemVisual")?,
                    };
                }
                Ok(CreatureEquipmentPersistenceRowLikeCpp {
                    creature_id: row.get_like_cpp(0, "creature_equip_template.CreatureID")?,
                    id: row.get_like_cpp(1, "creature_equip_template.ID")?,
                    items,
                })
            },
        )
    }
}

fn outcome<T>(result: Result<T>) -> WorldObjectRowsLoadOutcomeLikeCpp<T> {
    match result {
        Ok(rows) => WorldObjectRowsLoadOutcomeLikeCpp::Loaded(rows),
        Err(error) => WorldObjectRowsLoadOutcomeLikeCpp::Failed {
            reason: error.to_string(),
        },
    }
}

impl WorldObjectCatalogPersistencePortLikeCpp
    for MariaDbWorldObjectCatalogPersistenceAdapterLikeCpp
{
    fn load_creature_classification_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<'_, WorldObjectRowsLoadOutcomeLikeCpp<Vec<(u32, u32)>>> {
        Box::pin(async move { outcome(self.classifications().await) })
    }
    fn load_creature_template_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        WorldObjectRowsLoadOutcomeLikeCpp<CreatureTemplateCatalogPersistenceRowsLikeCpp>,
    > {
        Box::pin(async move { outcome(self.creature_templates().await) })
    }
    fn load_creature_sparring_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<'_, WorldObjectRowsLoadOutcomeLikeCpp<Vec<(u32, f32)>>> {
        Box::pin(async move { outcome(self.sparring().await) })
    }
    fn load_gameobject_template_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        WorldObjectRowsLoadOutcomeLikeCpp<GameObjectTemplateCatalogPersistenceRowsLikeCpp>,
    > {
        Box::pin(async move { outcome(self.gameobject_templates().await) })
    }
    fn load_gameobject_override_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        WorldObjectRowsLoadOutcomeLikeCpp<Vec<GameObjectOverridePersistenceRowLikeCpp>>,
    > {
        Box::pin(async move { outcome(self.gameobject_overrides().await) })
    }
    fn load_creature_difficulty_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        WorldObjectRowsLoadOutcomeLikeCpp<Vec<CreatureDifficultyPersistenceRowLikeCpp>>,
    > {
        Box::pin(async move { outcome(self.difficulty().await) })
    }
    fn load_creature_base_stats_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        WorldObjectRowsLoadOutcomeLikeCpp<Vec<CreatureBaseStatsPersistenceRowLikeCpp>>,
    > {
        Box::pin(async move { outcome(self.base_stats().await) })
    }
    fn load_creature_mount_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        WorldObjectRowsLoadOutcomeLikeCpp<Vec<CreatureMountPersistenceRowLikeCpp>>,
    > {
        Box::pin(async move { outcome(self.mounts().await) })
    }
    fn load_creature_model_info_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        WorldObjectRowsLoadOutcomeLikeCpp<Vec<CreatureModelInfoPersistenceRowLikeCpp>>,
    > {
        Box::pin(async move { outcome(self.model_info().await) })
    }
    fn load_creature_addon_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        WorldObjectRowsLoadOutcomeLikeCpp<CreatureAddonCatalogPersistenceRowsLikeCpp>,
    > {
        Box::pin(async move { outcome(self.addons().await) })
    }
    fn load_creature_equipment_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        WorldObjectRowsLoadOutcomeLikeCpp<Vec<CreatureEquipmentPersistenceRowLikeCpp>>,
    > {
        Box::pin(async move { outcome(self.equipment().await) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field_like_cpp::NativeFieldLikeCpp as Native;
    use crate::world::live_tests::live_world_probe_rows_like_cpp;

    /// A lifecycle `creature_template` row as MariaDB types it (TDB schema).
    fn creature_template_row(movement: [Native; 7]) -> Vec<Native> {
        let mut row = vec![
            Native::UInt32(575),                   // entry INT UNSIGNED
            Native::Text("Fire Elemental".into()), // name MEDIUMTEXT
            Native::Text(String::new()),           // AIName VARCHAR
            Native::Text(String::new()),           // ScriptName VARCHAR
            Native::Int32(2),                      // RequiredExpansion INT
            Native::UInt16(91),                    // faction SMALLINT UNSIGNED
            Native::UInt64(1),                     // npcflag BIGINT UNSIGNED
            Native::Float(1.0),                    // speed_walk FLOAT
            Native::Float(1.142_857),              // speed_run FLOAT
            Native::Float(1.0),                    // scale FLOAT
            Native::UInt8(1),                      // Classification TINYINT UNSIGNED
            Native::Int8(2),                       // dmgschool TINYINT
            Native::UInt32(0x8000),                // unit_flags INT UNSIGNED
            Native::UInt32(0x800),                 // unit_flags2 INT UNSIGNED
            Native::UInt32(0),                     // unit_flags3 INT UNSIGNED
            Native::UInt8(4),                      // type TINYINT UNSIGNED
            Native::Int32(1),                      // family INT
            Native::UInt8(0),                      // trainer_class TINYINT UNSIGNED
            Native::UInt8(8),                      // unit_class TINYINT UNSIGNED
            Native::UInt32(0),                     // VehicleId INT UNSIGNED
            Native::UInt8(1),                      // MovementType TINYINT UNSIGNED
        ];
        row.extend(movement);
        row.extend([
            Native::UInt32(0),   // flags_extra INT UNSIGNED
            Native::Null,        // StringId VARCHAR NULL
            Native::UInt8(1),    // RegenHealth TINYINT UNSIGNED
            Native::Float(0.75), // BaseVariance FLOAT
        ]);
        row
    }

    #[test]
    fn creature_template_row_reads_signed_and_unsigned_columns_like_cpp() {
        let row = creature_template_row([
            Native::UInt8(0),       // Ground
            Native::UInt8(0),       // Swim
            Native::UInt8(1),       // Flight
            Native::UInt8(1),       // Rooted
            Native::UInt8(0),       // Chase
            Native::UInt8(0),       // Random
            Native::UInt32(20_000), // InteractionPauseTimer
        ]);
        let template = creature_template_row_like_cpp(&row[..]).unwrap();
        assert_eq!(template.entry, 575);
        assert_eq!(template.required_expansion, 2);
        assert_eq!(template.faction, 91);
        assert_eq!(template.classification, 1);
        assert_eq!(template.damage_school, 2);
        assert_eq!(template.creature_type, 4);
        assert_eq!(template.family, 1);
        assert_eq!(template.unit_class, 8);
        assert_eq!(template.string_id, "");
        assert!(template.regen_health);
        assert_eq!(template.base_variance, 0.75);
        // A creature_template_movement row overrides every C++ default.
        assert_eq!(template.ground_movement_type, 0);
        assert!(!template.swim_allowed);
        assert_eq!(template.flight_movement_type, 1);
        assert!(template.rooted);
        assert_eq!(template.interaction_pause_timer_ms, 20_000);
    }

    #[test]
    fn creature_template_without_movement_row_keeps_cpp_movement_defaults() {
        let row = creature_template_row(std::array::from_fn(|_| Native::Null));
        let template = creature_template_row_like_cpp(&row[..]).unwrap();
        assert_eq!(template.ground_movement_type, 1);
        assert!(template.swim_allowed);
        assert_eq!(template.flight_movement_type, 0);
        assert!(!template.rooted);
        assert_eq!(
            (template.chase_movement_type, template.random_movement_type),
            (0, 0)
        );
        assert_eq!(template.interaction_pause_timer_ms, 180_000);
    }

    #[test]
    fn creature_template_row_fails_closed_on_decimal_movement_columns() {
        // The former COALESCE(ctm.Ground, 1) projection is DECIMAL.
        let row = creature_template_row(std::array::from_fn(|_| {
            Native::Unsupported("DECIMAL".into())
        }));
        let error = creature_template_row_like_cpp(&row[..]).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("creature_template_movement.Ground"),
            "{error}"
        );
        for sql in [
            CREATURE_TEMPLATE_LIFECYCLE_SQL_LIKE_CPP,
            GAMEOBJECT_TEMPLATE_LIFECYCLE_SQL_LIKE_CPP,
        ] {
            assert!(!sql.contains("COALESCE"), "{sql}");
        }
    }

    #[test]
    fn gameobject_template_row_reads_signed_data_and_content_tuning_like_cpp() {
        let mut row = vec![
            Native::UInt32(1721),
            Native::UInt8(10),
            Native::UInt32(259),
            Native::Text("Locked ball and chain".into()),
            Native::Float(1.0),
        ];
        row.extend(
            (0..MAX_GAMEOBJECT_DATA_PERSISTENCE_LIKE_CPP).map(|index| match index {
                0 => Native::Int32(36),
                1 => Native::Int32(-1),
                2 => Native::Int32(312),
                _ => Native::Int32(0),
            }),
        );
        row.extend([
            Native::Int32(42),
            Native::Text(String::new()),
            Native::Text("go_ball_and_chain".into()),
            Native::Null,
        ]);
        let template = gameobject_template_row_like_cpp(&row[..]).unwrap();
        assert_eq!((template.entry, template.go_type), (1721, 10));
        // `GetUInt32()` over a signed INT keeps the C++ bit pattern.
        assert_eq!(template.data[..3], [36, u32::MAX, 312]);
        assert_eq!(template.content_tuning_id, 42);
        assert_eq!(template.script_name, "go_ball_and_chain");
        assert_eq!(template.string_id, "");
    }

    #[test]
    fn creature_addon_row_reads_signed_smallint_anim_kits_like_cpp() {
        let row = [
            Native::UInt32(648), // creature_template_addon.entry INT UNSIGNED
            Native::UInt32(0),
            Native::UInt32(0),
            Native::UInt8(0),
            Native::UInt8(0),
            Native::UInt8(0),
            Native::UInt8(1),
            Native::UInt8(0),
            Native::UInt32(0),
            Native::Int16(726), // aiAnimKit SMALLINT
            Native::Int16(0),
            Native::Int16(-1),
            Native::UInt8(0),
            Native::Null,
        ];
        let addon = creature_addon_row_like_cpp(&row[..]).unwrap();
        assert_eq!(addon.owner_id, 648);
        assert_eq!(addon.ai_anim_kit, 726);
        assert_eq!(addon.melee_anim_kit, u16::MAX);
        assert_eq!(addon.auras, "");
    }

    /// #1436: run the lifecycle row mappers over real rows inside a READ ONLY
    /// transaction.
    #[tokio::test]
    #[ignore = "read-only MariaDB probe; set RUSTYCORE_DB_IT_USER, RUSTYCORE_DB_IT_WORLD_DB and optional HOST/PORT/PASS/SOCKET"]
    async fn live_world_object_catalog_rows_keep_cpp_widths() -> anyhow::Result<()> {
        let rows = live_world_probe_rows_like_cpp(&format!(
            "{CREATURE_TEMPLATE_LIFECYCLE_SQL_LIKE_CPP} WHERE ct.entry IN (1, 299, 575, 14823) ORDER BY ct.entry"
        ))
        .await?;
        assert_eq!(rows.len(), 4);
        // Waypoint: creature_template_movement Ground 0, Swim 0, Flight 1.
        let waypoint = creature_template_row_like_cpp(&rows[0][..])?;
        assert_eq!(waypoint.entry, 1);
        assert_eq!(waypoint.ground_movement_type, 0);
        assert!(!waypoint.swim_allowed);
        assert_eq!(waypoint.flight_movement_type, 1);
        assert_eq!(waypoint.interaction_pause_timer_ms, 180_000);
        // Diseased Young Wolf: beast (type 1), wolf family 1.
        let wolf = creature_template_row_like_cpp(&rows[1][..])?;
        assert_eq!((wolf.entry, wolf.creature_type, wolf.family), (299, 1, 1));
        // Fire Elemental: signed dmgschool 2 (fire).
        let elemental = creature_template_row_like_cpp(&rows[2][..])?;
        assert_eq!((elemental.entry, elemental.damage_school), (575, 2));
        let paused = creature_template_row_like_cpp(&rows[3][..])?;
        assert_eq!(
            (paused.entry, paused.interaction_pause_timer_ms),
            (14823, 20_000)
        );

        let rows = live_world_probe_rows_like_cpp(&format!(
            "{GAMEOBJECT_TEMPLATE_LIFECYCLE_SQL_LIKE_CPP} WHERE entry = 1721"
        ))
        .await?;
        let chain = gameobject_template_row_like_cpp(&rows[0][..])?;
        assert_eq!((chain.entry, chain.go_type), (1721, 10));
        assert_eq!(chain.data[..2], [36, u32::MAX]);

        let rows = live_world_probe_rows_like_cpp(&format!(
            "{CREATURE_TEMPLATE_ADDON_SQL_LIKE_CPP} WHERE entry = 648"
        ))
        .await?;
        let addon = creature_addon_row_like_cpp(&rows[0][..])?;
        assert_eq!((addon.owner_id, addon.ai_anim_kit), (648, 726));
        Ok(())
    }
}
