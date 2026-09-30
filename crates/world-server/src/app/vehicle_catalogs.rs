use anyhow::{Context, Result};
use crate::catalogs;
use std::sync::Arc;
use tracing::info;
use wow_database::{HotfixDatabase, WorldDatabase};

pub(super) struct VehicleCatalogs {
    pub(super) vehicle_accessory_store: Arc<wow_data::VehicleAccessoryStoreLikeCpp>,
    pub(super) _vehicle_template_store: Arc<wow_data::VehicleTemplateStoreLikeCpp>,
    pub(super) vehicle_seat_store: Arc<wow_data::VehicleSeatStore>,
    pub(super) vehicle_store: Arc<wow_data::VehicleStore>,
}

pub(super) async fn load(
    data_dir: &str,
    locale: &str,
    hotfix_db: &Arc<HotfixDatabase>,
    world_db: &Arc<WorldDatabase>,
) -> Result<VehicleCatalogs> {
    let vehicle_hotfix_persistence =
        wow_database::MariaDbVehicleHotfixPersistenceAdapterLikeCpp::new(Arc::clone(hotfix_db));
    let vehicle_store = Arc::new(
        catalogs::vehicle::load_vehicle_store_like_cpp(
            &data_dir,
            &locale,
            &vehicle_hotfix_persistence,
        )
        .await
        .context("Failed to load Vehicle.db2 / hotfix rows")?,
    );
    info!("Loaded {} vehicle rows", vehicle_store.len());
    let vehicle_seat_store = Arc::new(
        catalogs::vehicle::load_vehicle_seat_store_like_cpp(
            &data_dir,
            &locale,
            &vehicle_hotfix_persistence,
        )
        .await
        .context("Failed to load VehicleSeat.db2 / hotfix rows")?,
    );
    info!("Loaded {} vehicle seat rows", vehicle_seat_store.len());
    let vehicle_world_persistence =
        wow_database::MariaDbVehicleWorldCatalogPersistenceAdapterLikeCpp::new(Arc::clone(
            world_db,
        ));
    let _vehicle_template_store = Arc::new(
        catalogs::vehicle::load_vehicle_template_store_like_cpp(&vehicle_world_persistence)
            .await
            .context("Failed to load C++ vehicle_template rows")?,
    );
    let vehicle_accessory_store = Arc::new(
        catalogs::vehicle::load_vehicle_accessory_store_like_cpp(&vehicle_world_persistence)
            .await
            .context("Failed to load C++ vehicle accessory rows")?,
    );

    Ok(VehicleCatalogs {
        vehicle_store,
        vehicle_seat_store,
        _vehicle_template_store,
        vehicle_accessory_store,
    })
}
