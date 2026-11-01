// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Loot sources: the bodies moved to `wow_world_loot_ops` (#1363 slice 5a);
//! the World test entry points and the autostore facade stay here.

use super::*;

mod gameobject;
mod gameobject_authority;

impl WorldSession {
    #[cfg(test)]
    pub(crate) async fn open_represented_gameobject_chest_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        source: GameObjectLootSource,
    ) {
        let template_money = self
            .world_query_catalogs_like_cpp()
            .and_then(|catalogs| catalogs.gameobject.get(gameobject_guid.entry()))
            .map(|row| (row.min_money, row.max_money))
            .unwrap_or((0, 0));
        let generators = self.id_generators_for_test_like_cpp();
        let item_valuation = self.item_valuation_catalogs_for_test_like_cpp();
        wow_world_loot_ops::open_represented_gameobject_chest_with_template_money_like_cpp(
            self,
            generators.item.as_ref(),
            &item_valuation,
            gameobject_guid,
            source,
            template_money,
        )
        .await;
    }

    #[cfg(test)]
    pub(super) async fn generate_represented_gameobject_chest_loot_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        source: GameObjectLootSource,
        allowed_looters: &[ObjectGuid],
    ) -> Option<CreatureLoot> {
        let template_money = self
            .world_query_catalogs_like_cpp()
            .and_then(|catalogs| catalogs.gameobject.get(gameobject_guid.entry()))
            .map(|row| (row.min_money, row.max_money))
            .unwrap_or((0, 0));
        wow_world_loot_ops::generate_represented_gameobject_chest_loot_with_template_money_like_cpp(
            self,
            gameobject_guid,
            player_guid,
            source,
            allowed_looters,
            template_money,
        )
        .await
    }
}
