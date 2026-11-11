//! Persistent evidence for a character committed before LoginDB acknowledgement.
//! The journal row shares the character transaction. It is removed only after
//! LoginDB commits. A pending row requires reconciliation, including on restart.
use super::{CreateError, PreparedStatement, SqlTransaction};
use crate::{CharacterDatabase, LoginDatabase};

#[cfg(test)]
#[path = "../../../unit_tests/forever/creation_journal.rs"]
mod tests;

const INSERT: &str = "INSERT INTO rustycore_forever_creation_journal (player_guid, account_id, realm_address, save_time, format_version) VALUES (?,?,?,?,?)";
const DELETE: &str = "DELETE FROM rustycore_forever_creation_journal WHERE player_guid=? AND account_id=? AND realm_address=? AND save_time=? AND format_version=?";

#[derive(Clone, Copy)]
pub(super) struct Entry {
    pub guid: u64,
    pub account: u32,
    pub realm: u32,
    pub save_time: u32,
}

impl Entry {
    pub(super) fn insert(self) -> PreparedStatement {
        let mut statement = PreparedStatement::new(INSERT);
        self.bind(&mut statement);
        statement
    }

    fn delete(self) -> PreparedStatement {
        let mut statement = PreparedStatement::new(DELETE);
        self.bind(&mut statement);
        statement
    }

    fn bind(self, statement: &mut PreparedStatement) {
        statement.set_u64(0, self.guid);
        statement.set_u32(1, self.account);
        statement.set_u32(2, self.realm);
        statement.set_u32(3, self.save_time);
        statement.set_u8(4, 1);
    }

    pub(super) fn cleanup(self) -> SqlTransaction {
        let mut transaction = SqlTransaction::new();
        // A missing or different marker is not a confirmed acknowledgement.
        transaction.append_expect_rows_affected(self.delete(), 1);
        transaction
    }
}

pub(super) async fn ensure_schema(
    database: &CharacterDatabase,
    auth: &LoginDatabase,
) -> Result<(), CreateError> {
    let metadata = database.direct_query(
        "SELECT CAST(COUNT(*) AS UNSIGNED) FROM information_schema.TABLES WHERE TABLE_SCHEMA=DATABASE() AND TABLE_TYPE='BASE TABLE' AND ENGINE='InnoDB' AND TABLE_NAME IN ('characters','character_customizations','item_instance','character_inventory','character_spell','character_spell_favorite','character_action','character_skills','character_select_screen_equipment_cache','rustycore_forever_creation_journal','item_instance_gems','item_instance_transmog','item_instance_artifact','item_instance_artifact_powers','item_instance_modifiers','item_instance_azerite','item_instance_azerite_milestone_power','item_instance_azerite_unlocked_essence','item_instance_azerite_empowered','character_fishingsteps','character_battleground_data','character_queststatus','character_queststatus_objectives','character_queststatus_objectives_spawn_tracking','character_queststatus_rewarded','character_queststatus_daily','character_queststatus_weekly','character_queststatus_seasonal','character_queststatus_monthly','character_glyphs','character_talent','character_pvp_talent','character_trait_config','character_trait_entry','character_spell_cooldown','character_spell_charges','character_aura','character_aura_effect','character_aura_stored_location','character_achievement','character_achievement_progress','character_reputation','character_queststatus_objectives_criteria','character_queststatus_objectives_criteria_progress','character_equipmentsets','character_transmog_outfits','character_transmog_outfit','character_transmog_outfit_situation','character_transmog_outfit_slot','account_tutorial','account_instance_times','character_currency','character_cuf_profiles','character_player_data_element','character_player_data_flag','character_bank_tab_settings','character_garrison','character_garrison_blueprints','character_garrison_buildings','character_garrison_followers','character_garrison_follower_abilities','character_stats','mail','mail_items','character_gifts')",
    ).await.map_err(|_| CreateError::Database)?;
    // Every represented write must participate in the character transaction.
    // A missing/nontransactional table cannot masquerade as an empty journal.
    if metadata.try_read::<u64>(0) != Some(65) {
        return Err(CreateError::InvalidRow);
    }
    let metadata = auth.direct_query(
        "SELECT CAST(COUNT(*) AS UNSIGNED) FROM information_schema.TABLES WHERE TABLE_SCHEMA=DATABASE() AND TABLE_TYPE='BASE TABLE' AND ENGINE='InnoDB' AND TABLE_NAME IN ('account','account_last_played_character','realmcharacters')",
    ).await.map_err(|_| CreateError::Database)?;
    if metadata.try_read::<u64>(0) != Some(3) {
        return Err(CreateError::InvalidRow);
    }
    Ok(())
}

pub(super) async fn ensure_empty(
    writer: &mut super::writer::CreationWriter,
) -> Result<(), CreateError> {
    let pending = writer
        .query(&PreparedStatement::new(
            "SELECT player_guid FROM rustycore_forever_creation_journal LIMIT 1",
        ))
        .await?;
    if !pending.is_empty() {
        return Err(CreateError::RecoveryRequired);
    }
    Ok(())
}
