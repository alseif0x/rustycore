//! Actual catalog-port awaits interleaved with canonical Map mutations.
use std::sync::{Arc, Mutex, atomic::{AtomicUsize, Ordering}};
use wow_map::MapManager;
use wow_core::ObjectGuid;
use wow_persistence::{PersistenceFutureLikeCpp, LootTemplateCatalogPersistencePortLikeCpp,
    LootTemplateTablePersistenceLikeCpp, LootTemplateCatalogOutcomeLikeCpp,
    LootTemplatePersistenceRowLikeCpp, LootConditionPersistenceRowLikeCpp};

#[derive(Clone, Copy)]
pub(super) enum Mutation { None, Health, HealthSecond, Generation, TargetAba, RootAba(ObjectGuid), Map }
pub(super) struct CatalogPort {
    manager: Arc<Mutex<MapManager>>,
    victim: ObjectGuid,
    mutation: Mutation,
    calls: AtomicUsize,
}
impl CatalogPort {
    pub fn new(manager: Arc<Mutex<MapManager>>, victim: ObjectGuid, mutation: Mutation) -> Self {
        Self { manager, victim, mutation, calls: AtomicUsize::new(0) }
    }
    pub fn calls(&self) -> usize { self.calls.load(Ordering::SeqCst) }
}
impl LootTemplateCatalogPersistencePortLikeCpp for CatalogPort {
    fn load_loot_template_rows_like_cpp(&self, _: LootTemplateTablePersistenceLikeCpp, _: u32)
        -> PersistenceFutureLikeCpp<'_, LootTemplateCatalogOutcomeLikeCpp<LootTemplatePersistenceRowLikeCpp>> {
        Box::pin(async { LootTemplateCatalogOutcomeLikeCpp::Loaded(Vec::new()) })
    }
    fn load_loot_condition_rows_like_cpp(&self, _: i32, _: u32, _: u32)
        -> PersistenceFutureLikeCpp<'_, LootTemplateCatalogOutcomeLikeCpp<LootConditionPersistenceRowLikeCpp>> {
        Box::pin(async move {
            let ordinal = self.calls.fetch_add(1, Ordering::SeqCst);
            tokio::task::yield_now().await;
            let mutate_now = if matches!(self.mutation, Mutation::HealthSecond) {
                ordinal == 1
            } else { ordinal == 0 };
            if mutate_now {
                let mut manager = self.manager.lock().unwrap();
                match self.mutation {
                    Mutation::None => {}
                    Mutation::Health | Mutation::HealthSecond => {
                        manager.find_map_mut(1, 0).unwrap().map_mut().get_typed_creature_mut(self.victim)
                            .unwrap().unit_mut().set_max_health(101);
                    }
                    Mutation::Generation => {
                        manager.find_map(1, 0).unwrap().map().get_typed_creature(self.victim).unwrap()
                            .loot_authority_like_cpp().retire_like_cpp();
                    }
                    Mutation::Map => {
                        assert!(manager.destroy_map(1, 0));
                        manager.create_world_map(1, 0);
                    }
                    Mutation::TargetAba => super::support::replace_actor(&mut manager, self.victim),
                    Mutation::RootAba(root) => super::support::replace_actor(&mut manager, root),
                }
            }
            LootTemplateCatalogOutcomeLikeCpp::Loaded(Vec::new())
        })
    }
}
