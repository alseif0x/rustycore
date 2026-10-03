//! Target query-holder layout; do not use 3.4.3 statement indexes.
use super::{
    ForeverSessionRepository, LoadError, PreparedStatement, SqlResult, account_statement, field,
};
use wow_persistence::forever::selection::{
    CharacterRow, CustomizationRow, SelectionRows, VisualItemRow,
};

const BASE_COLUMNS: &str = "c.guid,c.name,c.race,c.class,c.gender,c.level,c.zone,c.map,c.position_x,c.position_y,c.position_z,gm.guildid,c.playerFlags,c.at_login,cp.entry,cp.modelid,cp.level,cb.guid,c.slot,c.createTime,c.logout_time,c.activeTalentGroup,c.lastLoginBuild,c.personalTabardEmblemStyle,c.personalTabardEmblemColor,c.personalTabardBorderStyle,c.personalTabardBorderColor,c.personalTabardBackgroundColor";
const SLOTS: [&str; 19] = [
    "head", "neck", "shoulder", "body", "chest", "waist", "legs", "feet", "wrists", "hands",
    "finger1", "finger2", "trinket1", "trinket2", "back", "mainHand", "offHand", "ranged",
    "tabard",
];
const VISUAL_FIELDS: [&str; 8] = [
    "EquippedItemID",
    "VisibleItemID",
    "Subclass",
    "InvType",
    "DisplayID",
    "DisplayEnchantID",
    "SecondaryItemModifiedAppearanceID",
    "SheatheCategory",
];
const CUSTOMIZATIONS: &str = "SELECT cc.guid,cc.chrCustomizationOptionID,cc.chrCustomizationChoiceID FROM character_customizations cc LEFT JOIN characters c ON cc.guid=c.guid WHERE c.account=? AND c.deleteInfos_Name IS NULL ORDER BY cc.guid,cc.chrCustomizationOptionID";

fn character_query(declined: bool) -> String {
    let mut sql = format!("SELECT {BASE_COLUMNS}");
    for slot in SLOTS {
        for column in VISUAL_FIELDS {
            sql.push_str(&format!(",ceq.{slot}{column}"));
        }
    }
    if declined {
        sql.push_str(",cd.genitive");
    }
    // Source obtains surname from CharacterCache. This isolated operation
    // reads the same persisted field, without adding a second mutable cache.
    sql.push_str(",c.surname FROM characters c LEFT JOIN character_pet cp ON c.summonedPetNumber=cp.id LEFT JOIN guild_member gm ON c.guid=gm.guid LEFT JOIN character_banned cb ON c.guid=cb.guid AND cb.active=1 LEFT JOIN character_select_screen_equipment_cache ceq ON c.guid=ceq.guid");
    if declined {
        sql.push_str(" LEFT JOIN character_declinedname cd ON c.guid=cd.guid");
    }
    sql.push_str(" WHERE c.account=? AND c.deleteInfos_Name IS NULL");
    sql
}

/// LEFT JOIN NULL is source zero. A type/width decoding error is NOT zero.
fn joined<T>(row: &SqlResult, index: usize) -> Result<T, LoadError>
where
    T: Default + for<'r> sqlx::Decode<'r, sqlx::MySql> + sqlx::Type<sqlx::MySql>,
{
    Ok(field::<Option<T>>(row, index)?.unwrap_or_default())
}

fn decode_character(row: &SqlResult, declined: bool) -> Result<CharacterRow, LoadError> {
    let mut equipment = [VisualItemRow::default(); 19];
    for (slot, value) in equipment.iter_mut().enumerate() {
        let base = 28 + slot * 8;
        *value = VisualItemRow {
            item_id: joined(row, base)?,
            visible_item_id: joined(row, base + 1)?,
            subclass: joined(row, base + 2)?,
            inventory_type: joined(row, base + 3)?,
            display_id: joined(row, base + 4)?,
            display_enchant_id: joined(row, base + 5)?,
            secondary_appearance_id: joined(row, base + 6)?,
            sheathe_category: joined(row, base + 7)?,
        };
    }
    Ok(CharacterRow {
        guid: field(row, 0)?,
        name: field(row, 1)?,
        race: field(row, 2)?,
        class: field(row, 3)?,
        gender: field(row, 4)?,
        level: field(row, 5)?,
        zone: field(row, 6)?,
        map: field(row, 7)?,
        position: [field(row, 8)?, field(row, 9)?, field(row, 10)?],
        guild: joined(row, 11)?,
        player_flags: field(row, 12)?,
        at_login: field(row, 13)?,
        pet_entry: joined(row, 14)?,
        pet_display: joined(row, 15)?,
        pet_level: joined(row, 16)?,
        active_ban_guid: joined(row, 17)?,
        slot: field(row, 18)?,
        create_time: field(row, 19)?,
        logout_time: field(row, 20)?,
        active_talent_group: field(row, 21)?,
        last_login_build: field(row, 22)?,
        personal_tabard: [
            field(row, 23)?,
            field(row, 24)?,
            field(row, 25)?,
            field(row, 26)?,
            field(row, 27)?,
        ],
        equipment,
        declined_genitive: if declined { field(row, 180)? } else { None },
        surname: field(row, 180 + usize::from(declined))?,
    })
}

impl ForeverSessionRepository {
    pub(super) async fn selection(
        &self,
        account: u32,
        declined: bool,
    ) -> Result<SelectionRows, LoadError> {
        self.characters.execute(&PreparedStatement::new("UPDATE character_banned SET active=0 WHERE unbandate<=UNIX_TIMESTAMP() AND unbandate<>bandate")).await.map_err(|_|LoadError::Database)?;
        static QUERIES: std::sync::OnceLock<[String; 2]> = std::sync::OnceLock::new();
        let queries = QUERIES.get_or_init(|| [character_query(false), character_query(true)]);
        let mut statement = PreparedStatement::new(&queries[usize::from(declined)]);
        statement.set_u32(0, account);
        let mut characters = self
            .characters
            .query(&statement)
            .await
            .map_err(|_| LoadError::Database)?;
        let mut choices = self
            .characters
            .query(&account_statement(CUSTOMIZATIONS, account))
            .await
            .map_err(|_| LoadError::Database)?;
        let mut rows = SelectionRows::default();
        if !characters.is_empty() {
            loop {
                rows.characters
                    .push(decode_character(&characters, declined)?);
                if !characters.next_row() {
                    break;
                }
            }
        }
        if !choices.is_empty() {
            loop {
                rows.customizations.push(CustomizationRow {
                    guid: field(&choices, 0)?,
                    option: field(&choices, 1)?,
                    choice: field(&choices, 2)?,
                });
                if !choices.next_row() {
                    break;
                }
            }
        }
        Ok(rows)
    }

    pub(super) async fn recustomize(&self, account: u32, guid: u64) -> Result<(), LoadError> {
        // CHAR_UPD_ADD_AT_LOGIN_FLAG uses OR, not assignment. The account
        // predicate is an ownership fence for this isolated session adapter.
        let mut statement = PreparedStatement::new(
            "UPDATE characters SET at_login=at_login|8 WHERE guid=? AND account=? AND deleteInfos_Name IS NULL",
        );
        statement.set_u64(0, guid);
        statement.set_u32(1, account);
        let affected = self
            .characters
            .execute(&statement)
            .await
            .map_err(|_| LoadError::Database)?;
        if affected == 0 {
            let mut confirm = PreparedStatement::new(
                "SELECT guid FROM characters WHERE guid=? AND account=? AND deleteInfos_Name IS NULL AND (at_login&8)<>0",
            );
            confirm.set_u64(0, guid);
            confirm.set_u32(1, account);
            if self
                .characters
                .query(&confirm)
                .await
                .map_err(|_| LoadError::Database)?
                .count()
                != 1
            {
                return Err(LoadError::InvalidRow);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn target_holder_offsets_and_optional_cache_projection_are_explicit() {
        for declined in [false, true] {
            let sql = character_query(declined);
            let fields: Vec<_> = sql
                .strip_prefix("SELECT ")
                .unwrap()
                .split(" FROM ")
                .next()
                .unwrap()
                .split(',')
                .collect();
            assert_eq!(fields.len(), 181 + usize::from(declined));
            assert_eq!(fields[28], "ceq.headEquippedItemID");
            assert_eq!(fields[179], "ceq.tabardSheatheCategory");
            if declined {
                assert_eq!(fields[180], "cd.genitive");
            }
            assert_eq!(fields.last().copied(), Some("c.surname"));
            assert_eq!(sql.contains("LEFT JOIN character_declinedname"), declined);
            assert!(!sql.contains("LIMIT"));
            assert!(!sql.contains("ORDER BY"));
            assert!(sql.ends_with("WHERE c.account=? AND c.deleteInfos_Name IS NULL"));
        }
    }
    #[test]
    fn customization_holder_retains_source_order_and_deleted_predicate() {
        assert!(CUSTOMIZATIONS.ends_with("ORDER BY cc.guid,cc.chrCustomizationOptionID"));
        assert!(CUSTOMIZATIONS.contains("c.account=? AND c.deleteInfos_Name IS NULL"));
    }
}
