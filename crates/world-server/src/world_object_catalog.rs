//! Composition boundary for canonical creature and gameobject catalogs.

use anyhow::{Result, bail};
use wow_persistence::*;

fn loaded<T>(outcome: WorldObjectRowsLoadOutcomeLikeCpp<T>) -> Result<T> {
    match outcome {
        WorldObjectRowsLoadOutcomeLikeCpp::Loaded(rows) => Ok(rows),
        WorldObjectRowsLoadOutcomeLikeCpp::Failed { reason } => bail!(reason),
    }
}

pub(super) async fn load_creature_classifications_like_cpp(
    port: &dyn WorldObjectCatalogPersistencePortLikeCpp,
) -> Result<wow_data::CreatureTemplateClassificationStoreLikeCpp> {
    Ok(
        wow_data::CreatureTemplateClassificationStoreLikeCpp::from_entries(loaded(
            port.load_creature_classification_rows_like_cpp().await,
        )?),
    )
}

pub(super) async fn load_creature_templates_like_cpp(
    port: &dyn WorldObjectCatalogPersistencePortLikeCpp,
) -> Result<wow_data::CreatureTemplateLifecycleStoreLikeCpp> {
    let rows = loaded(port.load_creature_template_rows_like_cpp().await)?;
    Ok(
        wow_data::CreatureTemplateLifecycleStoreLikeCpp::from_catalog_rows_like_cpp(
            rows.templates
                .into_iter()
                .map(|r| wow_data::CreatureTemplateLifecycleRecordLikeCpp {
                    entry: r.entry,
                    name: r.name,
                    ai_name: r.ai_name,
                    script_name: r.script_name,
                    required_expansion: r.required_expansion,
                    faction: r.faction,
                    npc_flags: r.npc_flags,
                    speed_walk: r.speed_walk,
                    speed_run: r.speed_run,
                    scale: r.scale,
                    classification: r.classification,
                    damage_school: r.damage_school,
                    unit_flags: r.unit_flags,
                    unit_flags2: r.unit_flags2,
                    unit_flags3: r.unit_flags3,
                    creature_type: r.creature_type,
                    family: r.family,
                    trainer_class: r.trainer_class,
                    unit_class: r.unit_class,
                    vehicle_id: r.vehicle_id,
                    movement_type: r.movement_type,
                    ground_movement_type: r.ground_movement_type,
                    swim_allowed: r.swim_allowed,
                    flight_movement_type: r.flight_movement_type,
                    rooted: r.rooted,
                    chase_movement_type: r.chase_movement_type,
                    random_movement_type: r.random_movement_type,
                    interaction_pause_timer_ms: r.interaction_pause_timer_ms,
                    flags_extra: r.flags_extra,
                    string_id: r.string_id,
                    regen_health: r.regen_health,
                    spells: [0; wow_data::MAX_CREATURE_SPELLS_LIKE_CPP],
                    models: Vec::new(),
                }),
            rows.spells
                .into_iter()
                .map(|r| (r.creature_id, r.index, r.spell_id)),
            rows.models.into_iter().map(|r| {
                (
                    r.creature_id,
                    wow_data::CreatureTemplateLifecycleModelLikeCpp {
                        creature_display_id: r.display_id,
                        display_scale: r.display_scale,
                        probability: r.probability,
                    },
                )
            }),
        ),
    )
}

pub(super) async fn load_creature_sparring_like_cpp(
    port: &dyn WorldObjectCatalogPersistencePortLikeCpp,
    templates: &wow_data::CreatureTemplateLifecycleStoreLikeCpp,
) -> Result<wow_data::CreatureTemplateSparringStoreLikeCpp> {
    Ok(
        wow_data::CreatureTemplateSparringStoreLikeCpp::from_rows_like_cpp(
            loaded(port.load_creature_sparring_rows_like_cpp().await)?,
            |entry| templates.get(entry).is_some(),
        ),
    )
}

pub(super) async fn load_gameobject_templates_like_cpp(
    port: &dyn WorldObjectCatalogPersistencePortLikeCpp,
) -> Result<wow_data::GameObjectTemplateLifecycleStoreLikeCpp> {
    let rows = loaded(port.load_gameobject_template_rows_like_cpp().await)?;
    Ok(
        wow_data::GameObjectTemplateLifecycleStoreLikeCpp::from_templates_and_addons_like_cpp(
            rows.templates.into_iter().map(|r| {
                wow_data::GameObjectTemplateLifecycleRecordLikeCpp {
                    entry: r.entry,
                    go_type: r.go_type,
                    display_id: r.display_id,
                    name: r.name,
                    size: r.size,
                    data: r.data,
                    content_tuning_id: r.content_tuning_id,
                    ai_name: r.ai_name,
                    script_name: r.script_name,
                    string_id: r.string_id,
                    addon: None,
                }
            }),
            rows.addons.into_iter().map(|r| {
                wow_data::GameObjectTemplateAddonLifecycleRecordLikeCpp {
                    entry: r.entry,
                    faction: r.faction,
                    flags: r.flags,
                    world_effect_id: r.world_effect_id,
                    anim_kit_id: r.anim_kit_id,
                }
            }),
        ),
    )
}

pub(super) async fn load_gameobject_overrides_like_cpp(
    port: &dyn WorldObjectCatalogPersistencePortLikeCpp,
) -> Result<wow_data::GameObjectOverrideLifecycleStoreLikeCpp> {
    Ok(
        wow_data::GameObjectOverrideLifecycleStoreLikeCpp::from_overrides(
            loaded(port.load_gameobject_override_rows_like_cpp().await)?
                .into_iter()
                .map(|r| wow_data::GameObjectOverrideLifecycleRecordLikeCpp {
                    spawn_id: r.spawn_id,
                    faction: r.faction,
                    flags: r.flags,
                }),
        ),
    )
}

pub(super) async fn load_creature_difficulties_like_cpp(
    port: &dyn WorldObjectCatalogPersistencePortLikeCpp,
    difficulty_store: &wow_data::DifficultyStore,
    modifier: impl Fn(u32) -> f32,
) -> Result<wow_data::CreatureDifficultyStoreLikeCpp> {
    let rows = loaded(port.load_creature_difficulty_rows_like_cpp().await)?;
    Ok(
        wow_data::CreatureDifficultyStoreLikeCpp::from_records_and_difficulty_store_like_cpp(
            rows.into_iter()
                .map(|r| wow_data::CreatureDifficultyRecordLikeCpp {
                    entry: r.entry,
                    difficulty_id: r.difficulty_id,
                    min_level: r.min_level,
                    max_level: r.max_level,
                    health_scaling_expansion: r.health_scaling_expansion,
                    health_modifier: r.health_modifier,
                    mana_modifier: r.mana_modifier,
                    armor_modifier: r.armor_modifier,
                    damage_modifier: r.damage_modifier,
                    creature_difficulty_id: r.creature_difficulty_id,
                    type_flags: r.type_flags,
                    type_flags2: r.type_flags2,
                    loot_id: r.loot_id,
                    pickpocket_loot_id: r.pickpocket_loot_id,
                    skin_loot_id: r.skin_loot_id,
                    gold_min: r.gold_min,
                    gold_max: r.gold_max,
                    static_flags: r.static_flags,
                }),
            difficulty_store,
            modifier,
        ),
    )
}

pub(super) async fn load_creature_base_stats_like_cpp(
    port: &dyn WorldObjectCatalogPersistencePortLikeCpp,
) -> Result<wow_data::CreatureBaseStatsStoreLikeCpp> {
    let rows = loaded(port.load_creature_base_stats_rows_like_cpp().await)?;
    Ok(wow_data::CreatureBaseStatsStoreLikeCpp::from_records(
        rows.into_iter().map(|r| {
            (
                r.level,
                r.unit_class,
                wow_data::CreatureBaseStatsRecordLikeCpp {
                    base_health: r.base_health,
                    base_mana: r.base_mana,
                    base_armor: r.base_armor,
                    attack_power: r.attack_power,
                    ranged_attack_power: r.ranged_attack_power,
                    base_damage: r.base_damage,
                },
            )
        }),
    ))
}

pub(super) async fn load_creature_mounts_like_cpp(
    port: &dyn WorldObjectCatalogPersistencePortLikeCpp,
) -> Result<wow_data::CreatureTemplateMountStoreLikeCpp> {
    Ok(
        wow_data::CreatureTemplateMountStoreLikeCpp::from_rows_like_cpp(
            loaded(port.load_creature_mount_rows_like_cpp().await)?
                .into_iter()
                .map(|r| {
                    (
                        r.entry,
                        r.vehicle_id,
                        r.display_id,
                        r.display_scale,
                        r.probability,
                    )
                }),
        ),
    )
}

pub(super) async fn load_creature_model_info_like_cpp(
    port: &dyn WorldObjectCatalogPersistencePortLikeCpp,
    displays: &wow_data::CreatureDisplayInfoStore,
    models: &wow_data::CreatureModelDataStore,
) -> Result<wow_data::CreatureModelInfoStoreLikeCpp> {
    Ok(wow_data::CreatureModelInfoStoreLikeCpp::from_rows_like_cpp(
        loaded(port.load_creature_model_info_rows_like_cpp().await)?
            .into_iter()
            .map(|r| wow_data::CreatureModelInfoRowLikeCpp {
                display_id: r.display_id,
                bounding_radius: r.bounding_radius,
                combat_reach: r.combat_reach,
                display_id_other_gender: r.display_id_other_gender,
            }),
        displays,
        models,
    ))
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn load_creature_addons_like_cpp(
    port: &dyn WorldObjectCatalogPersistencePortLikeCpp,
    templates: &wow_data::CreatureTemplateLifecycleStoreLikeCpp,
    spawns: &wow_data::WorldSpawnIdStore,
    displays: &wow_data::CreatureDisplayInfoStore,
    emotes: &wow_data::EmotesStore,
    anim_kits: &wow_data::AnimKitStore,
    spells: &wow_data::SpellStore,
    spell_misc: &wow_data::SpellMiscStore,
    spell_durations: &wow_data::SpellDurationStore,
) -> Result<wow_data::CreatureAddonStoreLikeCpp> {
    let rows = loaded(port.load_creature_addon_rows_like_cpp().await)?;
    let convert = |r: CreatureAddonPersistenceRowLikeCpp| wow_data::CreatureAddonRowLikeCpp {
        owner_id: r.owner_id,
        path_id: r.path_id,
        mount: r.mount,
        stand_state: r.stand_state,
        anim_tier: r.anim_tier,
        vis_flags: r.vis_flags,
        sheath_state: r.sheath_state,
        pvp_flags: r.pvp_flags,
        emote: r.emote,
        ai_anim_kit: r.ai_anim_kit,
        movement_anim_kit: r.movement_anim_kit,
        melee_anim_kit: r.melee_anim_kit,
        visibility_distance_type: r.visibility_distance_type,
        auras: r.auras,
    };
    Ok(
        wow_data::CreatureAddonStoreLikeCpp::from_catalog_rows_with_stores_like_cpp(
            rows.spawn_addons.into_iter().map(convert),
            rows.template_addons.into_iter().map(convert),
            templates,
            spawns,
            displays,
            emotes,
            anim_kits,
            spells,
            spell_misc,
            spell_durations,
        ),
    )
}

pub(super) async fn load_creature_equipment_like_cpp(
    port: &dyn WorldObjectCatalogPersistencePortLikeCpp,
    creature_template_exists: impl FnMut(u32) -> bool,
    item_inventory_type: impl FnMut(u32) -> Option<u8>,
    item_modified_appearance_exists: impl FnMut(u32, u32) -> bool,
    default_item_appearance_mod_id: impl FnMut(u32) -> Option<u16>,
) -> Result<wow_data::CreatureEquipmentStoreLikeCpp> {
    let rows = loaded(port.load_creature_equipment_rows_like_cpp().await)?;
    Ok(wow_data::CreatureEquipmentStoreLikeCpp::from_rows_like_cpp(
        rows.into_iter()
            .map(|r| wow_data::CreatureEquipmentRowLikeCpp {
                creature_id: r.creature_id,
                id: r.id,
                items: r.items.map(|item| wow_data::CreatureEquipmentItemLikeCpp {
                    item_id: item.item_id,
                    appearance_mod_id: item.appearance_mod_id,
                    item_visual: item.item_visual,
                }),
            }),
        creature_template_exists,
        item_inventory_type,
        item_modified_appearance_exists,
        default_item_appearance_mod_id,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn failed_catalog_is_not_published_as_empty() {
        assert!(
            loaded::<Vec<(u32, u32)>>(WorldObjectRowsLoadOutcomeLikeCpp::Failed {
                reason: "boom".into()
            })
            .is_err()
        );
    }
}
