//! Realm identity and account-owned admission resources.

use std::{collections::HashMap, sync::Arc};

use anyhow::Result;
use tracing::info;
use wow_constants::ClientOpcodes;
use wow_database::{LoginBattlePetPersistenceLikeCpp, LoginDatabase};
use wow_network::world_socket::AccountLookup;
use wow_world::{BattlePetAccountRegistryLikeCpp, session::registry::PacketHandlerEntry};

use super::{realm_startup, stat_tables_startup::StatTables};
use crate::DbAccountLookup;

pub(super) async fn load(
    login_db_slot: &mut Option<LoginDatabase>,
    realm_availability: &realm_startup::RealmAvailability,
    stat_tables: &StatTables,
) -> Result<(
    realm_startup::RealmIdentity,
    Arc<LoginDatabase>,
    Arc<BattlePetAccountRegistryLikeCpp>,
    HashMap<ClientOpcodes, &'static PacketHandlerEntry>,
    Arc<dyn AccountLookup>,
)> {
    let realm_identity = realm_startup::load_identity(
        &realm_availability.realm_list,
        realm_availability.realm_id,
        login_db_slot
            .as_ref()
            .expect("Login database retained until account composition"),
    )
    .await?;
    // Share the Login DB only with account-owned composition adapters.
    let login_db = Arc::new(
        login_db_slot
            .take()
            .expect("Login database retained until account composition"),
    );
    let battle_pet_account_registry = Arc::new(BattlePetAccountRegistryLikeCpp::new(
        Arc::new(LoginBattlePetPersistenceLikeCpp::new(Arc::clone(&login_db))),
        Arc::clone(&stat_tables.battle_pet_species_entry_store),
        Arc::clone(&stat_tables.battle_pet_breed_quality_store),
        Arc::clone(&stat_tables.battle_pet_breed_state_store),
        Arc::clone(&stat_tables.battle_pet_species_state_store),
        Arc::clone(&stat_tables.battle_pet_xp_game_table),
        realm_availability.realm_id,
        realm_identity.active_realm.id.address_like_cpp(),
    ));

    // Build handler dispatch table
    let table = wow_world::session::registry::build_dispatch_table();
    info!("Loaded {} packet handlers", table.len());

    // Build account lookup
    let account_lookup: Arc<dyn AccountLookup> = Arc::new(DbAccountLookup {
        login_db: Arc::clone(&login_db),
        realm_id: realm_availability.realm_id,
        win64_auth_seed: realm_identity.win64_auth_seed,
    });

    Ok((
        realm_identity,
        login_db,
        battle_pet_account_registry,
        table,
        account_lookup,
    ))
}
