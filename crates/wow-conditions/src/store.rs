use std::sync::{Arc, OnceLock};

use parking_lot::RwLock;
use wow_data::ConditionEntriesByTypeStore;

static CONDITION_MGR_STORE_LIKE_CPP: OnceLock<RwLock<Option<Arc<ConditionEntriesByTypeStore>>>> =
    OnceLock::new();

fn condition_mgr_store_slot_like_cpp() -> &'static RwLock<Option<Arc<ConditionEntriesByTypeStore>>>
{
    CONDITION_MGR_STORE_LIKE_CPP.get_or_init(|| RwLock::new(None))
}

/// Install the process-wide C++ `sConditionMgr` condition store.
///
/// This keeps the access pattern close to C++ while storing the actual data in an `Arc`, so a
/// future reload can atomically replace the active store without changing call sites.
pub fn set_condition_mgr_store_like_cpp(store: Arc<ConditionEntriesByTypeStore>) {
    *condition_mgr_store_slot_like_cpp().write() = Some(store);
}

/// Return the active C++ `sConditionMgr` store, if startup loaded it.
pub fn condition_mgr_store_like_cpp() -> Option<Arc<ConditionEntriesByTypeStore>> {
    condition_mgr_store_slot_like_cpp().read().as_ref().cloned()
}

/// Clear the process-wide condition store. Used by tests and future reload wiring.
pub fn clear_condition_mgr_store_like_cpp() {
    *condition_mgr_store_slot_like_cpp().write() = None;
}
