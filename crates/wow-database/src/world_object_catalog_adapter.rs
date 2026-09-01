//! MariaDB adapter for canonical creature and gameobject startup catalogs.

use crate::{SqlResult, WorldDatabase};
use anyhow::Result;
use std::sync::Arc;
use wow_persistence::*;

pub struct MariaDbWorldObjectCatalogPersistenceAdapterLikeCpp {
    world_db: Arc<WorldDatabase>,
}

impl MariaDbWorldObjectCatalogPersistenceAdapterLikeCpp {
    pub fn new(world_db: Arc<WorldDatabase>) -> Self {
        Self { world_db }
    }

    async fn classifications(&self) -> Result<Vec<(u32, u32)>> {
        let mut result = self
            .world_db
            .direct_query("SELECT entry, Classification FROM creature_template")
            .await?;
        let mut rows = Vec::new();
        if !result.is_empty() {
            loop {
                rows.push((
                    result.try_read(0).unwrap_or(0),
                    result.try_read(1).unwrap_or(0),
                ));
                if !result.next_row() {
                    break;
                }
            }
        }
        Ok(rows)
    }

    async fn creature_templates(&self) -> Result<CreatureTemplateCatalogPersistenceRowsLikeCpp> {
        let mut result = self.world_db.direct_query("SELECT ct.entry, ct.name, ct.AIName, ct.ScriptName, ct.RequiredExpansion, ct.faction, ct.npcflag, ct.speed_walk, ct.speed_run, ct.scale, ct.Classification, ct.dmgschool, ct.unit_flags, ct.unit_flags2, ct.unit_flags3, ct.`type`, ct.family, ct.trainer_class, ct.unit_class, ct.VehicleId, ct.MovementType, COALESCE(ctm.Ground, 1), COALESCE(ctm.Swim, 1), COALESCE(ctm.Flight, 0), COALESCE(ctm.Rooted, 0), COALESCE(ctm.Chase, 0), COALESCE(ctm.Random, 0), COALESCE(ctm.InteractionPauseTimer, 180000), ct.flags_extra, ct.StringId, ct.RegenHealth FROM creature_template ct LEFT JOIN creature_template_movement ctm ON ct.entry = ctm.CreatureId").await?;
        let mut templates = Vec::new();
        if !result.is_empty() {
            loop {
                templates.push(CreatureTemplatePersistenceRowLikeCpp {
                    entry: result.try_read(0).unwrap_or(0),
                    name: result.try_read(1).unwrap_or_default(),
                    ai_name: result.try_read(2).unwrap_or_default(),
                    script_name: result.try_read(3).unwrap_or_default(),
                    required_expansion: result.try_read(4).unwrap_or(0),
                    faction: result.try_read(5).unwrap_or(0),
                    npc_flags: result.try_read(6).unwrap_or(0),
                    speed_walk: result.try_read(7).unwrap_or(0.0),
                    speed_run: result.try_read(8).unwrap_or(0.0),
                    scale: result.try_read(9).unwrap_or(1.0),
                    classification: result.try_read(10).unwrap_or(0),
                    damage_school: result.try_read(11).unwrap_or(0),
                    unit_flags: result.try_read(12).unwrap_or(0),
                    unit_flags2: result.try_read(13).unwrap_or(0),
                    unit_flags3: result.try_read(14).unwrap_or(0),
                    creature_type: result.try_read(15).unwrap_or(0),
                    family: result.try_read(16).unwrap_or(0),
                    trainer_class: result.try_read(17).unwrap_or(0),
                    unit_class: result.try_read(18).unwrap_or(0),
                    vehicle_id: result.try_read(19).unwrap_or(0),
                    movement_type: result.try_read(20).unwrap_or(0),
                    ground_movement_type: result.try_read::<Option<u8>>(21).flatten().unwrap_or(1),
                    swim_allowed: result.try_read::<Option<u8>>(22).flatten().unwrap_or(1) != 0,
                    flight_movement_type: result.try_read::<Option<u8>>(23).flatten().unwrap_or(0),
                    rooted: result.try_read::<Option<u8>>(24).flatten().unwrap_or(0) != 0,
                    chase_movement_type: result.try_read::<Option<u8>>(25).flatten().unwrap_or(0),
                    random_movement_type: result.try_read::<Option<u8>>(26).flatten().unwrap_or(0),
                    interaction_pause_timer_ms: result
                        .try_read::<Option<u32>>(27)
                        .flatten()
                        .unwrap_or(180_000),
                    flags_extra: result.try_read(28).unwrap_or(0),
                    string_id: result.try_read(29).unwrap_or_default(),
                    regen_health: result.try_read::<u8>(30).unwrap_or(0) != 0,
                });
                if !result.next_row() {
                    break;
                }
            }
        }
        let mut result = self
            .world_db
            .direct_query("SELECT CreatureID, `Index`, Spell FROM creature_template_spell")
            .await?;
        let mut spells = Vec::new();
        if !result.is_empty() {
            loop {
                spells.push(CreatureTemplateSpellPersistenceRowLikeCpp {
                    creature_id: result.try_read(0).unwrap_or(0),
                    index: result.try_read(1).unwrap_or(8),
                    spell_id: result.try_read(2).unwrap_or(0),
                });
                if !result.next_row() {
                    break;
                }
            }
        }
        let mut result = self.world_db.direct_query("SELECT CreatureID, CreatureDisplayID, DisplayScale, Probability FROM creature_template_model ORDER BY Idx ASC").await?;
        let mut models = Vec::new();
        if !result.is_empty() {
            loop {
                models.push(CreatureTemplateModelPersistenceRowLikeCpp {
                    creature_id: result.try_read(0).unwrap_or(0),
                    display_id: result.try_read(1).unwrap_or(0),
                    display_scale: result.try_read(2).unwrap_or(0.0),
                    probability: result.try_read(3).unwrap_or(0.0),
                });
                if !result.next_row() {
                    break;
                }
            }
        }
        Ok(CreatureTemplateCatalogPersistenceRowsLikeCpp {
            templates,
            spells,
            models,
        })
    }

    async fn sparring(&self) -> Result<Vec<(u32, f32)>> {
        let mut result = self
            .world_db
            .direct_query("SELECT Entry, NoNPCDamageBelowHealthPct FROM creature_template_sparring")
            .await?;
        let mut rows = Vec::new();
        if !result.is_empty() {
            loop {
                rows.push((
                    result.try_read(0).unwrap_or(0),
                    result.try_read(1).unwrap_or(0.0),
                ));
                if !result.next_row() {
                    break;
                }
            }
        }
        Ok(rows)
    }

    async fn gameobject_templates(
        &self,
    ) -> Result<GameObjectTemplateCatalogPersistenceRowsLikeCpp> {
        let mut result = self.world_db.direct_query("SELECT entry, type, displayId, name, size, Data0, Data1, Data2, Data3, Data4, Data5, Data6, Data7, Data8, Data9, Data10, Data11, Data12, Data13, Data14, Data15, Data16, Data17, Data18, Data19, Data20, Data21, Data22, Data23, Data24, Data25, Data26, Data27, Data28, Data29, Data30, Data31, Data32, Data33, Data34, ContentTuningId, AIName, ScriptName, StringId FROM gameobject_template").await?;
        let mut templates = Vec::new();
        if !result.is_empty() {
            loop {
                let mut data = [0; MAX_GAMEOBJECT_DATA_PERSISTENCE_LIKE_CPP];
                for (index, slot) in data.iter_mut().enumerate() {
                    *slot = result.try_read(5 + index).unwrap_or(0);
                }
                templates.push(GameObjectTemplatePersistenceRowLikeCpp {
                    entry: result.try_read(0).unwrap_or(0),
                    go_type: result.try_read(1).unwrap_or(0),
                    display_id: result.try_read(2).unwrap_or(0),
                    name: result.try_read(3).unwrap_or_default(),
                    size: result.try_read(4).unwrap_or(1.0),
                    data,
                    content_tuning_id: result.try_read(40).unwrap_or(0),
                    ai_name: result.try_read(41).unwrap_or_default(),
                    script_name: result.try_read(42).unwrap_or_default(),
                    string_id: result.try_read(43).unwrap_or_default(),
                });
                if !result.next_row() {
                    break;
                }
            }
        }
        let mut result = self.world_db.direct_query("SELECT entry, faction, flags, WorldEffectID, AIAnimKitID FROM gameobject_template_addon").await?;
        let mut addons = Vec::new();
        if !result.is_empty() {
            loop {
                addons.push(GameObjectTemplateAddonPersistenceRowLikeCpp {
                    entry: result.try_read(0).unwrap_or(0),
                    faction: result.try_read(1).unwrap_or(0),
                    flags: result.try_read(2).unwrap_or(0),
                    world_effect_id: result.try_read(3).unwrap_or(0),
                    anim_kit_id: result.try_read(4).unwrap_or(0),
                });
                if !result.next_row() {
                    break;
                }
            }
        }
        Ok(GameObjectTemplateCatalogPersistenceRowsLikeCpp { templates, addons })
    }

    async fn gameobject_overrides(&self) -> Result<Vec<GameObjectOverridePersistenceRowLikeCpp>> {
        let mut result = self
            .world_db
            .direct_query("SELECT spawnId, faction, flags FROM gameobject_overrides")
            .await?;
        let mut rows = Vec::new();
        if !result.is_empty() {
            loop {
                rows.push(GameObjectOverridePersistenceRowLikeCpp {
                    spawn_id: result.try_read(0).unwrap_or(0),
                    faction: result.try_read(1).unwrap_or(0),
                    flags: result.try_read(2).unwrap_or(0),
                });
                if !result.next_row() {
                    break;
                }
            }
        }
        Ok(rows)
    }

    async fn difficulty(&self) -> Result<Vec<CreatureDifficultyPersistenceRowLikeCpp>> {
        let mut result = self.world_db.direct_query("SELECT Entry, DifficultyID, MinLevel, MaxLevel, HealthScalingExpansion, HealthModifier, ManaModifier, ArmorModifier, DamageModifier, CreatureDifficultyID, TypeFlags, TypeFlags2, LootID, PickPocketLootID, SkinLootID, GoldMin, GoldMax, StaticFlags1, StaticFlags2, StaticFlags3, StaticFlags4, StaticFlags5, StaticFlags6, StaticFlags7, StaticFlags8 FROM creature_template_difficulty ORDER BY Entry").await?;
        let mut rows = Vec::new();
        if !result.is_empty() {
            loop {
                rows.push(CreatureDifficultyPersistenceRowLikeCpp {
                    entry: result.try_read(0).unwrap_or(0),
                    difficulty_id: read_u8(&result, 1),
                    min_level: read_u8(&result, 2),
                    max_level: read_u8(&result, 3),
                    health_scaling_expansion: result.try_read(4).unwrap_or(0),
                    health_modifier: result.try_read(5).unwrap_or(0.0),
                    mana_modifier: result.try_read(6).unwrap_or(0.0),
                    armor_modifier: result.try_read(7).unwrap_or(0.0),
                    damage_modifier: result.try_read(8).unwrap_or(0.0),
                    creature_difficulty_id: result.try_read(9).unwrap_or(0),
                    type_flags: result.try_read(10).unwrap_or(0),
                    type_flags2: result.try_read(11).unwrap_or(0),
                    loot_id: result.try_read(12).unwrap_or(0),
                    pickpocket_loot_id: result.try_read(13).unwrap_or(0),
                    skin_loot_id: result.try_read(14).unwrap_or(0),
                    gold_min: result.try_read(15).unwrap_or(0),
                    gold_max: result.try_read(16).unwrap_or(0),
                    static_flags: std::array::from_fn(|index| {
                        result.try_read(17 + index).unwrap_or(0)
                    }),
                });
                if !result.next_row() {
                    break;
                }
            }
        }
        Ok(rows)
    }

    async fn base_stats(&self) -> Result<Vec<CreatureBaseStatsPersistenceRowLikeCpp>> {
        let mut result = self.world_db.direct_query("SELECT level, class, basehp0, basehp1, basehp2, basemana, basearmor, attackpower, rangedattackpower, damage_base, damage_exp1, damage_exp2 FROM creature_classlevelstats").await?;
        let mut rows = Vec::new();
        if !result.is_empty() {
            loop {
                rows.push(CreatureBaseStatsPersistenceRowLikeCpp {
                    level: result.try_read(0).unwrap_or(0),
                    unit_class: result.try_read(1).unwrap_or(0),
                    base_health: std::array::from_fn(|index| {
                        result
                            .try_read::<u16>(2 + index)
                            .map(u32::from)
                            .unwrap_or(0)
                    }),
                    base_mana: result.try_read::<u16>(5).map(u32::from).unwrap_or(0),
                    base_armor: result.try_read::<u16>(6).map(u32::from).unwrap_or(0),
                    attack_power: result.try_read::<u16>(7).map(u32::from).unwrap_or(0),
                    ranged_attack_power: result.try_read::<u16>(8).map(u32::from).unwrap_or(0),
                    base_damage: std::array::from_fn(|index| {
                        result.try_read(9 + index).unwrap_or(0.0)
                    }),
                });
                if !result.next_row() {
                    break;
                }
            }
        }
        Ok(rows)
    }

    async fn mounts(&self) -> Result<Vec<CreatureMountPersistenceRowLikeCpp>> {
        let mut result = self.world_db.direct_query("SELECT ct.entry, ct.VehicleId, ctm.CreatureDisplayID, ctm.DisplayScale, ctm.Probability FROM creature_template ct LEFT JOIN creature_template_model ctm ON ct.entry = ctm.CreatureID ORDER BY ct.entry, ctm.Idx").await?;
        let mut rows = Vec::new();
        if !result.is_empty() {
            loop {
                rows.push(CreatureMountPersistenceRowLikeCpp {
                    entry: result.read(0),
                    vehicle_id: result.try_read(1).unwrap_or(0),
                    display_id: result.try_read(2).unwrap_or(0),
                    display_scale: result.try_read(3).unwrap_or(0.0),
                    probability: result.try_read(4).unwrap_or(0.0),
                });
                if !result.next_row() {
                    break;
                }
            }
        }
        Ok(rows)
    }

    async fn model_info(&self) -> Result<Vec<CreatureModelInfoPersistenceRowLikeCpp>> {
        let mut result = self.world_db.direct_query("SELECT DisplayID, BoundingRadius, CombatReach, DisplayID_Other_Gender FROM creature_model_info").await?;
        let mut rows = Vec::new();
        if !result.is_empty() {
            loop {
                rows.push(CreatureModelInfoPersistenceRowLikeCpp {
                    display_id: result.try_read(0).unwrap_or(0),
                    bounding_radius: result.try_read(1).unwrap_or(0.0),
                    combat_reach: result.try_read(2).unwrap_or(0.0),
                    display_id_other_gender: result.try_read(3).unwrap_or(0),
                });
                if !result.next_row() {
                    break;
                }
            }
        }
        Ok(rows)
    }

    async fn addons(&self) -> Result<CreatureAddonCatalogPersistenceRowsLikeCpp> {
        Ok(CreatureAddonCatalogPersistenceRowsLikeCpp { spawn_addons: self.addon_query("SELECT guid, PathId, mount, StandState, AnimTier, VisFlags, SheathState, PvPFlags, emote, aiAnimKit, movementAnimKit, meleeAnimKit, visibilityDistanceType, auras FROM creature_addon").await?, template_addons: self.addon_query("SELECT entry, PathId, mount, StandState, AnimTier, VisFlags, SheathState, PvPFlags, emote, aiAnimKit, movementAnimKit, meleeAnimKit, visibilityDistanceType, auras FROM creature_template_addon").await? })
    }
    async fn addon_query(&self, sql: &str) -> Result<Vec<CreatureAddonPersistenceRowLikeCpp>> {
        let mut result = self.world_db.direct_query(sql).await?;
        let mut rows = Vec::new();
        if !result.is_empty() {
            loop {
                rows.push(CreatureAddonPersistenceRowLikeCpp {
                    owner_id: result.try_read(0).unwrap_or(0),
                    path_id: result.try_read(1).unwrap_or(0),
                    mount: result.try_read(2).unwrap_or(0),
                    stand_state: result.try_read(3).unwrap_or(0),
                    anim_tier: result.try_read(4).unwrap_or(0),
                    vis_flags: result.try_read(5).unwrap_or(0),
                    sheath_state: result.try_read(6).unwrap_or(0),
                    pvp_flags: result.try_read(7).unwrap_or(0),
                    emote: result.try_read(8).unwrap_or(0),
                    ai_anim_kit: result.try_read(9).unwrap_or(0),
                    movement_anim_kit: result.try_read(10).unwrap_or(0),
                    melee_anim_kit: result.try_read(11).unwrap_or(0),
                    visibility_distance_type: result.try_read(12).unwrap_or(0),
                    auras: result.try_read(13).unwrap_or_default(),
                });
                if !result.next_row() {
                    break;
                }
            }
        }
        Ok(rows)
    }

    async fn equipment(&self) -> Result<Vec<CreatureEquipmentPersistenceRowLikeCpp>> {
        let mut result = self.world_db.direct_query("SELECT CreatureID, ID, ItemID1, AppearanceModID1, ItemVisual1, ItemID2, AppearanceModID2, ItemVisual2, ItemID3, AppearanceModID3, ItemVisual3 FROM creature_equip_template").await?;
        let mut rows = Vec::new();
        if !result.is_empty() {
            loop {
                rows.push(CreatureEquipmentPersistenceRowLikeCpp {
                    creature_id: result.try_read(0).unwrap_or(0),
                    id: result.try_read(1).unwrap_or(0),
                    items: std::array::from_fn(|slot| {
                        let base = 2 + slot * 3;
                        CreatureEquipmentItemPersistenceLikeCpp {
                            item_id: result.try_read(base).unwrap_or(0),
                            appearance_mod_id: result.try_read(base + 1).unwrap_or(0),
                            item_visual: result.try_read(base + 2).unwrap_or(0),
                        }
                    }),
                });
                if !result.next_row() {
                    break;
                }
            }
        }
        Ok(rows)
    }
}

fn read_u8(result: &SqlResult, column: usize) -> u8 {
    u8_from_candidates_like_cpp(
        result.try_read(column),
        result.try_read(column),
        result.try_read(column),
        result.try_read(column),
        result.try_read(column),
        result.try_read(column),
    )
}

fn u8_from_candidates_like_cpp(
    u8_value: Option<u8>,
    i8_value: Option<i8>,
    u16_value: Option<u16>,
    i16_value: Option<i16>,
    u32_value: Option<u32>,
    i32_value: Option<i32>,
) -> u8 {
    u8_value
        .or_else(|| i8_value.and_then(|value| u8::try_from(value).ok()))
        .or_else(|| u16_value.and_then(|value| u8::try_from(value).ok()))
        .or_else(|| i16_value.and_then(|value| u8::try_from(value).ok()))
        .or_else(|| u32_value.and_then(|value| u8::try_from(value).ok()))
        .or_else(|| i32_value.and_then(|value| u8::try_from(value).ok()))
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signed_tinyint_levels_decode_like_cpp_get_uint8() {
        assert_eq!(
            u8_from_candidates_like_cpp(None, Some(75), None, None, None, None),
            75
        );
        assert_eq!(
            u8_from_candidates_like_cpp(None, Some(-1), None, None, None, None),
            0
        );
        assert_eq!(
            u8_from_candidates_like_cpp(None, None, Some(75), None, None, None),
            75
        );
        assert_eq!(
            u8_from_candidates_like_cpp(None, None, Some(300), None, None, None),
            0
        );
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
