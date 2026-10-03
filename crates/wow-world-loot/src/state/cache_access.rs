use super::*;

impl LootState {
    pub fn clear_cached_loot_like_cpp(&mut self) {
        self.loot_table.clear();
    }

    pub fn cached_loot_for_owner_like_cpp(&self, owner: ObjectGuid) -> Option<&CreatureLoot> {
        self.loot_table.get(&owner)
    }

    pub fn cached_loot_for_owner_mut_like_cpp(
        &mut self,
        owner: ObjectGuid,
    ) -> Option<&mut CreatureLoot> {
        self.loot_table.get_mut(&owner)
    }

    pub fn cached_loot_contains_owner_like_cpp(&self, owner: ObjectGuid) -> bool {
        self.loot_table.contains_key(&owner)
    }

    pub fn cached_loot_values_like_cpp(&self) -> impl Iterator<Item = &CreatureLoot> {
        self.loot_table.values()
    }

    pub fn insert_cached_loot_for_owner_like_cpp(
        &mut self,
        owner: ObjectGuid,
        loot: CreatureLoot,
    ) -> Option<CreatureLoot> {
        self.loot_table.insert(owner, loot)
    }

    pub fn remove_cached_loot_for_owner_like_cpp(
        &mut self,
        owner: ObjectGuid,
    ) -> Option<CreatureLoot> {
        self.loot_table.remove(&owner)
    }

    pub fn remove_cached_loot_generation_like_cpp(&mut self, owner: ObjectGuid) -> Option<u64> {
        self.represented_loot_cache_generations_like_cpp
            .remove(&owner)
    }

    pub fn insert_cached_loot_generation_like_cpp(
        &mut self,
        owner: ObjectGuid,
        generation: u64,
    ) -> Option<u64> {
        self.represented_loot_cache_generations_like_cpp
            .insert(owner, generation)
    }

    pub fn is_personal_loot_owner_like_cpp(&self, owner: ObjectGuid) -> bool {
        self.represented_personal_loot_owners.contains(&owner)
    }

    pub fn personal_loot_money_for_owner_and_player_like_cpp(
        &self,
        owner: ObjectGuid,
        player: ObjectGuid,
    ) -> Option<&u32> {
        self.represented_personal_loot_money.get(&(owner, player))
    }

    pub fn insert_personal_loot_owner_like_cpp(&mut self, owner: ObjectGuid) -> bool {
        self.represented_personal_loot_owners.insert(owner)
    }

    pub fn remove_personal_loot_owner_like_cpp(&mut self, owner: ObjectGuid) -> bool {
        self.represented_personal_loot_owners.remove(&owner)
    }

    pub fn record_personal_loot_money_or_clear_cached_coins_for_owner_like_cpp(
        &mut self,
        owner: ObjectGuid,
        player: ObjectGuid,
    ) -> Option<&mut CreatureLoot> {
        let personal_money_owner = self.represented_personal_loot_owners.contains(&owner);
        let loot = self.loot_table.get_mut(&owner)?;
        if personal_money_owner {
            self.represented_personal_loot_money
                .insert((owner, player), 0);
        } else {
            loot.coins = 0;
        }
        Some(loot)
    }

    pub fn insert_personal_loot_money_like_cpp(
        &mut self,
        owner: ObjectGuid,
        player: ObjectGuid,
        amount: u32,
    ) -> Option<u32> {
        self.represented_personal_loot_money
            .insert((owner, player), amount)
    }

    pub fn remove_personal_loot_money_for_owner_like_cpp(&mut self, owner: ObjectGuid) {
        self.represented_personal_loot_money
            .retain(|(entry_owner, _), _| *entry_owner != owner);
    }

    pub fn retain_personal_loot_money_for_owner_with_pools_like_cpp(
        &mut self,
        owner: ObjectGuid,
        personal_pools: &HashMap<ObjectGuid, CreatureLoot>,
    ) {
        self.represented_personal_loot_money
            .retain(|(entry_owner, player), _| {
                *entry_owner != owner || personal_pools.contains_key(player)
            });
    }

    pub fn represented_unique_gameobject_use_contains_like_cpp(
        &self,
        owner: ObjectGuid,
    ) -> bool {
        self.represented_unique_gameobject_uses.contains(&owner)
    }

    pub fn insert_represented_unique_gameobject_use_like_cpp(
        &mut self,
        owner: ObjectGuid,
    ) -> bool {
        self.represented_unique_gameobject_uses.insert(owner)
    }

    pub fn record_accepted_loot_open_mirrors_like_cpp(
        &mut self,
        owner: ObjectGuid,
        loot: CreatureLoot,
        cache_generation: u64,
        view_generation: u64,
        authority: &OwnedLootAuthority,
    ) {
        self.loot_table.insert(owner, loot);
        self.represented_loot_cache_generations_like_cpp
            .insert(owner, cache_generation);
        self.active_loot_view_generations_like_cpp
            .insert(owner, view_generation);
        self.active_loot_view_authorities_like_cpp
            .insert(owner, authority.clone());
    }

    pub fn ensure_active_loot_view_authority_like_cpp(
        &mut self,
        owner: ObjectGuid,
        authority: &OwnedLootAuthority,
    ) {
        if !self
            .active_loot_view_authorities_like_cpp
            .get(&owner)
            .is_some_and(|opened| opened.shares_storage_like_cpp(authority))
        {
            self.active_loot_view_authorities_like_cpp
                .insert(owner, authority.clone());
        }
    }

    pub fn mark_cached_loot_first_open_like_cpp(&mut self, owner: ObjectGuid) -> bool {
        match self.loot_table.get_mut(&owner) {
            Some(loot) if !loot.looted_by_player => {
                loot.looted_by_player = true;
                true
            }
            _ => false,
        }
    }
}
