//! SQL-free CharacterTemplateDataStore startup batch; never Debug/string logs.
#[derive(Default)]
pub struct TemplateRows {
    pub classes: Vec<TemplateClassRow>,
    pub templates: Vec<TemplateRow>,
}
pub struct TemplateClassRow {
    pub template_id: u32,
    pub faction_group: u8,
    pub class: u8,
}
pub struct TemplateRow {
    pub id: u32,
    pub name: String,
    pub description: String,
    pub level: u8,
}
