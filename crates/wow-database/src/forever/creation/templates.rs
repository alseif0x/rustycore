//! CharacterTemplateDataStore.cpp:36-107 at Forever source 02245dcd.
//! A two-query complete read, not a SQL snapshot or empty-on-error fallback.
use super::{LoadError, WorldDatabase, field, rows};
use wow_persistence::forever::creation::templates::{TemplateClassRow, TemplateRow, TemplateRows};

const CLASSES: &str = "SELECT TemplateId, FactionGroup, Class FROM character_template_class";
const TEMPLATES: &str = "SELECT Id, Name, Description, Level FROM character_template";

pub(super) async fn load(world: &WorldDatabase) -> Result<TemplateRows, LoadError> {
    let classes = rows(world, CLASSES, |row| {
        Ok(TemplateClassRow {
            template_id: field(row, 0)?,
            faction_group: field(row, 1)?,
            class: field(row, 2)?,
        })
    })
    .await?;
    let templates = rows(world, TEMPLATES, |row| {
        Ok(TemplateRow {
            id: field(row, 0)?,
            name: field(row, 1)?,
            description: field(row, 2)?,
            level: field(row, 3)?,
        })
    })
    .await?;
    Ok(TemplateRows { classes, templates })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_template_queries_preserve_complete_shape_and_do_not_filter_by_request() {
        assert_eq!(
            CLASSES,
            "SELECT TemplateId, FactionGroup, Class FROM character_template_class"
        );
        assert_eq!(
            TEMPLATES,
            "SELECT Id, Name, Description, Level FROM character_template"
        );
        assert!(!CLASSES.contains("WHERE") && !TEMPLATES.contains("WHERE"));
    }
}
