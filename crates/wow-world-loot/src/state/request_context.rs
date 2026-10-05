use super::LootState;
use wow_world_core::session::HubRef;

impl LootState {
    pub fn item_template_flags2_like_cpp(&self, hub: HubRef<'_>, item_id: u32) -> Option<u32> {
        hub.catalogs
            .item_stats_store()
            .and_then(|store| store.sparse_template(item_id))
            .map(|template| template.flags[1])
    }
}
