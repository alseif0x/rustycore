//! MariaDB adapter for CMSG_QUERY_PLAYER_NAMES.
//!
//! TrinityCore answers this request from `CharacterCache`, which is warmed
//! before sessions are accepted. The adapter therefore owns only the startup
//! projection and never performs a packet-time full character-row query or
//! substitutes the requesting session's account.

use std::sync::Arc;

use wow_persistence::{
    PersistenceFutureLikeCpp, PlayerNameQueryOutcomeLikeCpp, PlayerNameQueryPersistencePortLikeCpp,
    PlayerNameQueryRequestLikeCpp, PlayerNameQueryRowLikeCpp,
};

use crate::{
    CharStatements, CharacterDatabase, CharacterIdentityCacheEntryLikeCpp,
    CharacterIdentityCacheLikeCpp, LoginDatabase, LoginStatements,
};

/// MariaDB-backed C++ `CharacterCache` projection.
pub struct MariaDbPlayerNameQueryPersistenceAdapterLikeCpp {
    character_db: Arc<CharacterDatabase>,
    identity_cache: Arc<CharacterIdentityCacheLikeCpp>,
}

pub async fn build_player_name_query_port_like_cpp(
    character_db: Arc<CharacterDatabase>,
    login_db: &LoginDatabase,
) -> Result<
    (
        Arc<CharacterIdentityCacheLikeCpp>,
        Arc<dyn PlayerNameQueryPersistencePortLikeCpp>,
    ),
    crate::DatabaseError,
> {
    let identity_cache = Arc::new(CharacterIdentityCacheLikeCpp::default());
    let adapter = Arc::new(MariaDbPlayerNameQueryPersistenceAdapterLikeCpp::new(
        character_db,
        Arc::clone(&identity_cache),
    ));
    adapter
        .load_character_identity_cache_like_cpp(login_db)
        .await?;
    Ok((identity_cache, adapter))
}

impl MariaDbPlayerNameQueryPersistenceAdapterLikeCpp {
    pub fn new(
        character_db: Arc<CharacterDatabase>,
        identity_cache: Arc<CharacterIdentityCacheLikeCpp>,
    ) -> Self {
        Self {
            character_db,
            identity_cache,
        }
    }

    /// Load the same minimal identity projection as C++ `CharacterCache`.
    /// If either query fails, world-server startup fails rather than accepting
    /// sessions with an incomplete authority.
    pub async fn load_character_identity_cache_like_cpp(
        &self,
        login_db: &LoginDatabase,
    ) -> Result<usize, crate::DatabaseError> {
        let character_statement = self
            .character_db
            .prepare(CharStatements::SEL_CHARACTER_IDENTITY_CACHE);
        let mut characters = self.character_db.query(&character_statement).await?;
        let mut entries = Vec::with_capacity(characters.count());
        if !characters.is_empty() {
            loop {
                entries.push(CharacterIdentityCacheEntryLikeCpp {
                    guid_low: characters.read::<u64>(0),
                    name: characters.read_string(1),
                    account_id: characters.read::<u32>(2),
                    race: characters.read::<u8>(3),
                    sex: characters.read::<u8>(4),
                    class: characters.read::<u8>(5),
                    level: characters.read::<u8>(6),
                    is_deleted: characters.try_read::<u64>(7).unwrap_or(0) != 0,
                });
                if !characters.next_row() {
                    break;
                }
            }
        }

        let account_statement = login_db.prepare(LoginStatements::SEL_BNET_GAME_ACCOUNT_IDS);
        let mut accounts = login_db.query(&account_statement).await?;
        let mut bnet_by_game_account = Vec::with_capacity(accounts.count());
        if !accounts.is_empty() {
            loop {
                bnet_by_game_account.push((
                    accounts.read::<u32>(0),
                    accounts.try_read::<u32>(1).unwrap_or(0),
                ));
                if !accounts.next_row() {
                    break;
                }
            }
        }

        Ok(self
            .identity_cache
            .replace_all(entries, bnet_by_game_account))
    }
}

impl PlayerNameQueryPersistencePortLikeCpp for MariaDbPlayerNameQueryPersistenceAdapterLikeCpp {
    fn load_player_name_like_cpp<'a>(
        &'a self,
        request: PlayerNameQueryRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PlayerNameQueryOutcomeLikeCpp> {
        Box::pin(async move {
            let Some(entry) = self.identity_cache.get(request.player_guid_counter) else {
                return PlayerNameQueryOutcomeLikeCpp::Missing;
            };
            PlayerNameQueryOutcomeLikeCpp::Found(PlayerNameQueryRowLikeCpp {
                name: entry.name,
                race: entry.race,
                class: entry.class,
                sex: entry.sex,
                level: entry.level,
                account_id: entry.account_id,
                battlenet_account_id: self.identity_cache.battlenet_account_id(entry.account_id),
                is_deleted: entry.is_deleted,
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PreparedStatement, StatementDef};

    #[test]
    fn cache_statements_are_global_projections_without_packet_bindings() {
        assert_eq!(
            CharStatements::SEL_CHARACTER_IDENTITY_CACHE.sql(),
            "SELECT guid, name, account, race, gender, class, level, deleteDate FROM characters"
        );
        assert_eq!(
            LoginStatements::SEL_BNET_GAME_ACCOUNT_IDS.sql(),
            "SELECT id, battlenet_account FROM account"
        );
        assert!(
            PreparedStatement::for_statement(CharStatements::SEL_CHARACTER_IDENTITY_CACHE)
                .params()
                .is_empty()
        );
    }

    #[test]
    fn cache_maps_target_account_and_deleted_projection() {
        let cache = CharacterIdentityCacheLikeCpp::default();
        let count = cache.replace_all(
            [CharacterIdentityCacheEntryLikeCpp {
                guid_low: 41,
                name: "Target".into(),
                account_id: 22,
                race: 10,
                class: 3,
                sex: 1,
                level: 80,
                is_deleted: true,
            }],
            [(22, 77)],
        );
        assert_eq!(count, 1);
        let entry = cache.get(41).expect("cached target");
        assert_eq!(entry.account_id, 22);
        assert_eq!(cache.battlenet_account_id(22), 77);
        assert!(entry.is_deleted);
    }
}
