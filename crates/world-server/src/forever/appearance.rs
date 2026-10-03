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
    opcode: u32,
    payload: &[u8],
) -> anyhow::Result<()> {
    if opcode != 0x440070 {
        return Ok(());
    }
    let create = wow_packet::forever::character_create::CharacterCreatePayload::decode(payload)?;
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
    Ok(())
}
