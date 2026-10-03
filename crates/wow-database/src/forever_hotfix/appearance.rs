//! 02245dcd HotfixDatabase.cpp:382-445 / DB2DatabaseLoader::Load. Project
//! only fields read by ValidateAppearance, with identical build predicates.
use super::*;
use wow_persistence::forever::appearance::{AppearanceOverlays, AppearanceRows, RequirementRow};

const QUERIES: [&str; 7] = [
    "SELECT ID,DisplayID FROM chr_model WHERE (VerifiedBuild>0)=?",
    "SELECT ID,UnalteredVisualRaceID FROM chr_races WHERE (VerifiedBuild>0)=?",
    "SELECT ID,ChrRacesID,ChrModelID,Sex FROM chr_race_x_chr_model WHERE (VerifiedBuild>0)=?",
    "SELECT ID,ChrModelID,ChrCustomizationReqID FROM chr_customization_option WHERE (VerifiedBuild>0)=?",
    "SELECT ID,ChrCustomizationOptionID,ChrCustomizationReqID FROM chr_customization_choice WHERE (VerifiedBuild>0)=?",
    "SELECT ID,Flags,ClassMask,AchievementID,QuestID,ItemModifiedAppearanceID,RaceMask1,RaceMask2 FROM chr_customization_req WHERE (VerifiedBuild>0)=?",
    "SELECT ID,ChrCustomizationChoiceID,ChrCustomizationReqID FROM chr_customization_req_choice WHERE (VerifiedBuild>0)=?",
];

impl ForeverHotfixRepository {
    pub async fn load_appearance_overlays(&self) -> Result<AppearanceOverlays, LoadError> {
        Ok(AppearanceOverlays {
            official: self.appearance_batch(false).await?,
            custom: self.appearance_batch(true).await?,
        })
    }

    async fn appearance_batch(&self, custom: bool) -> Result<AppearanceRows, LoadError> {
        let mut rows = AppearanceRows::default();
        for (table, sql) in QUERIES.iter().enumerate() {
            let mut query = PreparedStatement::new(*sql);
            query.set_bool(0, !custom);
            let mut result = self
                .0
                .query(&query)
                .await
                .map_err(|_| LoadError::Database)?;
            if result.is_empty() {
                continue;
            }
            let mut seen = BTreeSet::new();
            loop {
                let unsigned = |column| result.try_read::<u32>(column).ok_or(LoadError::InvalidRow);
                let signed = |column| result.try_read::<i32>(column).ok_or(LoadError::InvalidRow);
                let byte = |column| result.try_read::<u8>(column).ok_or(LoadError::InvalidRow);
                let id = unsigned(0)?;
                if !seen.insert(id) {
                    return Err(LoadError::InvalidRow);
                }
                match table {
                    0 => rows.models.push((id, unsigned(1)?)),
                    1 => rows.races.push((id, byte(1)?)),
                    2 => rows.race_models.push((
                        id,
                        u32::from(byte(1)?),
                        signed(2)? as u32,
                        i32::from(result.try_read::<i8>(3).ok_or(LoadError::InvalidRow)?),
                    )),
                    3 => rows.options.push((id, unsigned(1)?, signed(2)? as u32)),
                    4 => rows.choices.push((id, unsigned(1)?, signed(2)? as u32)),
                    5 => rows.requirements.push(RequirementRow {
                        id,
                        flags: signed(1)?,
                        class_mask: signed(2)?,
                        achievement: signed(3)?,
                        quest: signed(4)?,
                        item_appearance: signed(5)?,
                        race_mask: [signed(6)? as u32, signed(7)? as u32],
                    }),
                    6 => rows
                        .required_choices
                        .push((id, signed(1)? as u32, unsigned(2)?)),
                    _ => unreachable!(),
                }
                if !result.next_row() {
                    break;
                }
            }
        }
        Ok(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_target_projections_are_explicit_and_use_official_custom_predicate() {
        for sql in QUERIES {
            assert!(sql.starts_with("SELECT ID,"));
            assert!(sql.ends_with("WHERE (VerifiedBuild>0)=?"));
            assert!(!sql.contains('*') && !sql.contains("locale"));
        }
        assert!(QUERIES[5].contains("RaceMask1,RaceMask2"));
    }
}
