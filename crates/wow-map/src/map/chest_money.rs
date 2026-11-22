use super::*;

#[derive(Debug, Clone, Copy)]
pub struct ChestMoneyRestock {
    pub realm_id: u16,
    pub spawn_id: u64,
    pub generation: u64,
    pub deadline: i64,
}

impl<Terrain: TerrainGridLoader, Lifecycle: GridLifecycle> Map<Terrain, Lifecycle> {
    pub fn take_chest_money_restocks(&mut self) -> Vec<ChestMoneyRestock> {
        std::mem::take(&mut self.pending_chest_money_restocks)
    }

    pub(super) fn record_chest_money_restock(
        &mut self,
        guid: ObjectGuid,
        armed: bool,
        deadline: Option<i64>,
    ) {
        if !armed && deadline.is_none() {
            return;
        }
        let Some(chest) = self.get_typed_game_object_mut(guid) else {
            return;
        };
        let Some(deadline) = deadline.or_else(|| armed.then(|| chest.restock_time())) else {
            return;
        };
        if let Some((realm_id, generation)) = chest.take_chest_money_generation() {
            let spawn_id = chest.spawn_id();
            self.pending_chest_money_restocks.push(ChestMoneyRestock {
                realm_id,
                spawn_id,
                generation,
                deadline,
            });
        }
    }
}
