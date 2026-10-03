//! Startup consumes SQL projections into the single immutable data catalog.
use wow_data::forever_appearance::*;
use wow_persistence::forever::appearance::AppearanceRows;

pub(super) fn records(rows: AppearanceRows) -> AppearanceRecords {
    AppearanceRecords {
        models: rows
            .models
            .into_iter()
            .map(|(id, display)| Model { id, display })
            .collect(),
        races: rows
            .races
            .into_iter()
            .map(|(id, visual_parent)| Race { id, visual_parent })
            .collect(),
        race_models: rows
            .race_models
            .into_iter()
            .map(|(id, race, model, sex)| RaceModel {
                id,
                race,
                model,
                sex,
            })
            .collect(),
        options: rows
            .options
            .into_iter()
            .map(|(id, model, requirement)| OptionRecord {
                id,
                model,
                requirement,
            })
            .collect(),
        choices: rows
            .choices
            .into_iter()
            .map(|(id, option, requirement)| Choice {
                id,
                option,
                requirement,
            })
            .collect(),
        requirements: rows
            .requirements
            .into_iter()
            .map(|row| Requirement {
                id: row.id,
                flags: row.flags,
                class_mask: row.class_mask,
                race_mask: row.race_mask,
                achievement: row.achievement,
                quest: row.quest,
                item_appearance: row.item_appearance,
            })
            .collect(),
        required_choices: rows
            .required_choices
            .into_iter()
            .map(|(id, choice, requirement)| RequiredChoice {
                id,
                choice,
                requirement,
            })
            .collect(),
    }
}

pub(super) fn observe(
    catalog: &AppearanceCatalog,
    sources: &wow_world::forever::creation::WorldSources,
    initialization: &wow_data::forever_initialization::InitializationCatalog,
    game_tables: &wow_data::forever_game_tables::InitialGameTables,
    starting: &wow_world::forever::creation::StartingPolicy,
    permissions: &wow_world::forever::permissions::DefaultAccountPermissions,
    opcode: u32,
    payload: &[u8],
) -> anyhow::Result<()> {
    if opcode != 0x440070 {
        return Ok(());
    }
    let create = wow_packet::forever::character_create::CharacterCreatePayload::decode(payload)?;
    // The complete template startup batch now distinguishes a missing server
    // row from a failed lookup. A client template ID is never used as a level.
    let selected = starting.select(
        create.race,
        create.class,
        initialization,
        permissions,
        create.template_set,
    );
    let level = selected.as_ref().ok().map(|value| value.level());
    println!(
        "Private native creation reference start policy checked: resolved={}, template_requested={}; no Player/admission/save inferred.",
        selected.is_ok(),
        create.template_set.is_some()
    );
    // The admitted account loader rejects nonempty item-appearance collections.
    // This is valid only inside that current explicit empty-account boundary.
    let valid = wow_world::forever::appearance::validate_creation_appearance(
        catalog,
        &create,
        &Default::default(),
    );
    println!(
        "Private native creation appearance checked: valid={valid}, choice_count={}; no persistence/success inferred.",
        create.customizations.len()
    );
    println!(
        "Private native creation normal start checked: valid={}, selected_level_GT_rows_present={}, base_mana_admitted={}, next_level_XP_nonzero={}; NPE/intros/full Player and save still required.",
        sources
            .normal_start(create.race, create.class, initialization, catalog)
            .is_ok(),
        level.is_some_and(|level| game_tables.base_mp(u32::from(level)).is_some()
            && game_tables.hp_per_sta(u32::from(level)).is_some()
            && game_tables.xp(u32::from(level)).is_some()),
        level.is_some_and(|level| sources.base_mana(game_tables, create.class, level).is_ok()),
        level.is_some_and(|level| sources.experience_for_level(level) != 0),
    );
    println!(
        "Private native creation unmodified vitals checked: valid={}; target cap/default spells/equipment/full Player/save still required.",
        level.is_some_and(|level| sources
            .pre_equipment_vitals(
                create.race,
                create.class,
                level,
                initialization,
                game_tables
            )
            .is_ok())
    );
    println!(
        "Private native creation world inputs checked: definition_present={}, selected_level_stats_present={}; map/DB2/Player validation and persistence still required.",
        sources.definition(create.race, create.class).is_some(),
        level.is_some_and(|level| sources
            .primary_stats(create.race, create.class, level)
            .is_ok()),
    );
    Ok(())
}
