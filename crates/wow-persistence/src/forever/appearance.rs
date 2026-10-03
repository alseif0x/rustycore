//! Transient hotfix projections used by target creation. No strings, SQL,
//! packet bytes or private account fields cross this startup boundary.

#[derive(Default)]
pub struct AppearanceRows {
    /// ID, DisplayID
    pub models: Vec<(u32, u32)>,
    /// ID, UnalteredVisualRaceID
    pub races: Vec<(u32, u8)>,
    /// ID, ChrRacesID, ChrModelID, Sex
    pub race_models: Vec<(u32, u32, u32, i32)>,
    /// ID, ChrModelID, ChrCustomizationReqID
    pub options: Vec<(u32, u32, u32)>,
    /// ID, ChrCustomizationOptionID, ChrCustomizationReqID
    pub choices: Vec<(u32, u32, u32)>,
    pub requirements: Vec<RequirementRow>,
    /// ID, ChrCustomizationChoiceID, ChrCustomizationReqID
    pub required_choices: Vec<(u32, u32, u32)>,
}

pub struct RequirementRow {
    pub id: u32,
    pub flags: i32,
    pub class_mask: i32,
    pub race_mask: [u32; 2],
    pub achievement: i32,
    pub quest: i32,
    pub item_appearance: i32,
}

pub struct AppearanceOverlays {
    pub official: AppearanceRows,
    pub custom: AppearanceRows,
}
