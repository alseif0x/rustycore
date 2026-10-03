//! Target 02245dcd WorldSession query holders, NOT legacy statement offsets.
//! Every query must succeed. A missing table is never a new/empty account.

use crate::{CharacterDatabase, LoginDatabase, PreparedStatement, SqlResult, WorldDatabase};
use std::{collections::BTreeMap, sync::Arc};
use wow_persistence::PersistenceFutureLikeCpp;
use wow_persistence::forever::{
    ACCOUNT_DATA_TYPES, AccountData, AccountSnapshot, AvailabilityRepository, AvailabilityRows,
    ClassRequirement, GLOBAL_CACHE_MASK, LoadError, RaceClassRequirement, RaceUnlockRequirement,
    SessionRepository,
};

pub struct ForeverSessionRepository {
    auth: Arc<LoginDatabase>,
    characters: Arc<CharacterDatabase>,
}

// LoginDatabase.cpp / AccountInfoQueryHolder::Initialize: exact columns.
// Currently only empty collections are admitted. Supporting persisted collections
// belongs to the same ongoing port; unsupported rows are not silently discarded.
const COLLECTION_QUERIES: &[&str] = &[
    "SELECT itemId,isFavourite,hasFanfare FROM battlenet_account_toys WHERE accountId=?",
    "SELECT itemId,flags FROM battlenet_account_heirlooms WHERE accountId=?",
    "SELECT mountSpellId,flags FROM battlenet_account_mounts WHERE battlenetAccountId=?",
    "SELECT blobIndex,appearanceMask FROM battlenet_item_appearances WHERE battlenetAccountId=? ORDER BY blobIndex DESC",
    "SELECT itemModifiedAppearanceId FROM battlenet_item_favorite_appearances WHERE battlenetAccountId=?",
    "SELECT blobIndex,illusionMask FROM battlenet_account_transmog_illusions WHERE battlenetAccountId=? ORDER BY blobIndex DESC",
    "SELECT transmogOutfitId FROM battlenet_account_transmog_outfits WHERE battlenetAccountId=?",
    "SELECT warbandSceneId,isFavorite,hasFanfare FROM battlenet_account_warband_scenes WHERE battlenetAccountId=?",
    "SELECT playerDataElementAccountId,floatValue,int64Value FROM battlenet_account_player_data_element WHERE battlenetAccountId=?",
    "SELECT storageIndex,mask FROM battlenet_account_player_data_flag WHERE battlenetAccountId=?",
    "SELECT id,battlePetGuid,locked FROM battle_pet_slots WHERE battlenetAccountId=?",
];

fn account_statement(sql: &'static str, account: u32) -> PreparedStatement {
    let mut statement = PreparedStatement::new(sql);
    statement.set_u32(0, account);
    statement
}

fn field<T: for<'r> sqlx::Decode<'r, sqlx::MySql> + sqlx::Type<sqlx::MySql>>(
    result: &SqlResult,
    index: usize,
) -> Result<T, LoadError> {
    result.try_read(index).ok_or(LoadError::InvalidRow)
}

impl ForeverSessionRepository {
    pub fn new(auth: Arc<LoginDatabase>, characters: Arc<CharacterDatabase>) -> Self {
        Self { auth, characters }
    }

    async fn load(
        &self,
        account: u32,
        battlenet: u32,
        realm: u32,
    ) -> Result<AccountSnapshot, LoadError> {
        let mut account_data = std::array::from_fn(|_| AccountData::default());
        let mut result = self
            .characters
            .query(&account_statement(
                "SELECT type,time,data FROM account_data WHERE accountId=?",
                account,
            ))
            .await
            .map_err(|_| LoadError::Database)?;
        if !result.is_empty() {
            loop {
                let kind = usize::from(field::<u8>(&result, 0)?);
                if kind < ACCOUNT_DATA_TYPES && GLOBAL_CACHE_MASK & (1 << kind) != 0 {
                    account_data[kind] = AccountData {
                        time: field(&result, 1)?,
                        data: field(&result, 2)?,
                    };
                }
                if !result.next_row() {
                    break;
                }
            }
        }
        let result = self.characters.query(&account_statement(
            "SELECT tut0,tut1,tut2,tut3,tut4,tut5,tut6,tut7 FROM account_tutorial WHERE accountId=?", account,
        )).await.map_err(|_| LoadError::Database)?;
        let mut tutorials = [0; 8];
        if !result.is_empty() {
            if result.count() != 1 {
                return Err(LoadError::InvalidRow);
            }
            for (index, value) in tutorials.iter_mut().enumerate() {
                *value = field(&result, index)?;
            }
        }
        let mut instance_release_times = BTreeMap::new();
        let mut result = self
            .characters
            .query(&account_statement(
                "SELECT instanceId,releaseTime FROM account_instance_times WHERE accountId=?",
                account,
            ))
            .await
            .map_err(|_| LoadError::Database)?;
        if !result.is_empty() {
            loop {
                instance_release_times.insert(field(&result, 0)?, field(&result, 1)?);
                if !result.next_row() {
                    break;
                }
            }
        }
        for sql in COLLECTION_QUERIES {
            let result = self
                .auth
                .query(&account_statement(sql, battlenet))
                .await
                .map_err(|_| LoadError::Database)?;
            if !result.is_empty() {
                return Err(LoadError::UnsupportedState);
            }
        }
        let mut pets = account_statement(
            "SELECT bp.guid,bp.species,bp.breed,bp.displayId,bp.level,bp.exp,bp.health,bp.quality,bp.flags,bp.name,bp.nameTimestamp,bp.owner,bp.ownerRealmId,dn.genitive,dn.dative,dn.accusative,dn.instrumental,dn.prepositional FROM battle_pets bp LEFT JOIN battle_pet_declinedname dn ON bp.guid=dn.guid WHERE bp.battlenetAccountId=? AND (bp.ownerRealmId IS NULL OR bp.ownerRealmId=?)",
            battlenet,
        );
        pets.set_i32(1, i32::try_from(realm).map_err(|_| LoadError::InvalidRow)?);
        if !self
            .auth
            .query(&pets)
            .await
            .map_err(|_| LoadError::Database)?
            .is_empty()
        {
            return Err(LoadError::UnsupportedState);
        }
        let mut realm_character_counts = BTreeMap::new();
        let mut result = self.auth.query(&account_statement(
            "SELECT rc.acctid,rc.numchars,r.id,r.Region,r.Battlegroup FROM realmcharacters rc INNER JOIN realmlist r ON rc.realmid=r.id WHERE rc.acctid=?", account,
        )).await.map_err(|_| LoadError::Database)?;
        if !result.is_empty() {
            loop {
                let id: u32 = field(&result, 2)?;
                let region: u8 = field(&result, 3)?;
                let group: u8 = field(&result, 4)?;
                if id > 0xFFFF {
                    return Err(LoadError::InvalidRow);
                }
                realm_character_counts.insert(
                    u32::from(region) << 24 | u32::from(group) << 16 | id,
                    field(&result, 1)?,
                );
                if !result.next_row() {
                    break;
                }
            }
        }
        Ok(AccountSnapshot {
            account_data,
            tutorials,
            instance_release_times,
            realm_character_counts,
        })
    }
}

impl SessionRepository for ForeverSessionRepository {
    fn load_account(
        &self,
        account: u32,
        battlenet: u32,
        realm: u32,
    ) -> PersistenceFutureLikeCpp<'_, Result<AccountSnapshot, LoadError>> {
        Box::pin(self.load(account, battlenet, realm))
    }

    fn enumerate_empty(&self, account: u32) -> PersistenceFutureLikeCpp<'_, Result<(), LoadError>> {
        Box::pin(async move {
            self.characters.execute(&PreparedStatement::new("UPDATE character_banned SET active=0 WHERE unbandate<=UNIX_TIMESTAMP() AND unbandate<>bandate")).await.map_err(|_| LoadError::Database)?;
            // Target enum joins/identity projection, plus its customization
            // holder. Equipment-row decoding remains part of nonempty enum.
            let result = self.characters.query(&account_statement(
                "SELECT c.guid,c.name,c.race,c.class,c.gender,c.level,c.zone,c.map,c.position_x,c.position_y,c.position_z,gm.guildid,c.playerFlags,c.at_login,cp.entry,cp.modelid,cp.level,cb.guid,c.slot,c.createTime,c.logout_time,c.activeTalentGroup,c.lastLoginBuild,c.personalTabardEmblemStyle,c.personalTabardEmblemColor,c.personalTabardBorderStyle,c.personalTabardBorderColor,c.personalTabardBackgroundColor,ceq.guid FROM characters c LEFT JOIN character_pet cp ON c.summonedPetNumber=cp.id LEFT JOIN guild_member gm ON c.guid=gm.guid LEFT JOIN character_banned cb ON c.guid=cb.guid AND cb.active=1 LEFT JOIN character_select_screen_equipment_cache ceq ON c.guid=ceq.guid WHERE c.account=? AND c.deleteInfos_Name IS NULL", account,
            )).await.map_err(|_| LoadError::Database)?;
            let choices = self.characters.query(&account_statement("SELECT cc.guid,cc.chrCustomizationOptionID,cc.chrCustomizationChoiceID FROM character_customizations cc LEFT JOIN characters c ON cc.guid=c.guid WHERE c.account=? AND c.deleteInfos_Name IS NULL ORDER BY cc.guid,cc.chrCustomizationOptionID", account)).await.map_err(|_| LoadError::Database)?;
            if !result.is_empty() || !choices.is_empty() {
                return Err(LoadError::UnsupportedState);
            }
            Ok(())
        })
    }
}

pub struct ForeverAvailabilityRepository(Arc<WorldDatabase>);

impl ForeverAvailabilityRepository {
    pub fn new(world: Arc<WorldDatabase>) -> Self {
        Self(world)
    }
}

impl AvailabilityRepository for ForeverAvailabilityRepository {
    fn load_availability(
        &self,
    ) -> PersistenceFutureLikeCpp<'_, Result<AvailabilityRows, LoadError>> {
        Box::pin(async move {
            let mut classes = Vec::new();
            let mut result = self.0.direct_query("SELECT ClassID,RaceID,ActiveExpansionLevel,AccountExpansionLevel FROM class_expansion_requirement").await.map_err(|_| LoadError::Database)?;
            if !result.is_empty() {
                loop {
                    classes.push(RaceClassRequirement {
                        race_id: field(&result, 1)?,
                        class: ClassRequirement {
                            class_id: field(&result, 0)?,
                            active_expansion: field(&result, 2)?,
                            account_expansion: field(&result, 3)?,
                        },
                    });
                    if !result.next_row() {
                        break;
                    }
                }
            }
            let mut unlocks = Vec::new();
            let mut result = self
                .0
                .direct_query("SELECT raceID,expansion,achievementId FROM race_unlock_requirement")
                .await
                .map_err(|_| LoadError::Database)?;
            if !result.is_empty() {
                loop {
                    unlocks.push(RaceUnlockRequirement {
                        race_id: field(&result, 0)?,
                        expansion: field(&result, 1)?,
                        achievement_id: field(&result, 2)?,
                    });
                    if !result.next_row() {
                        break;
                    }
                }
            }
            Ok(AvailabilityRows { classes, unlocks })
        })
    }
}
