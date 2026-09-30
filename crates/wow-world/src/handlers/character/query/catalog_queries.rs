// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Typed catalog query projections and their fixture-only direct adapters.

use super::*;

impl WorldSession {
    /// Handle CMSG_DB_QUERY_BULK — client requests DB2 records.
    ///
    /// TrinityCore only sends a Valid `DBReply` when `sDB2Manager.GetStorage`
    /// returns typed storage and that storage can serialize the record through
    /// `DB2StorageBase::WriteRecord`. Rust's `HotfixBlobCache` stores raw
    /// WDC4/DB2 record bytes, which are not the same wire format. Only typed
    /// stores implemented here may answer Valid; missing typed storage follows
    /// the C++ Invalid branch and lets the client use its local DB2 cache.
    pub(in crate::handlers::character) async fn handle_db_query_bulk_with_tact_keys_like_cpp(
        &mut self,
        tact_keys: &wow_data::TactKeyStore,
        query: wow_packet::packets::misc::DbQueryBulk,
    ) {
        info!(
            "DbQueryBulk: table=0x{:08X}, {} records {:?} for account {}",
            query.table_hash,
            query.queries.len(),
            query.queries,
            self.account_id
        );
        for record_id in &query.queries {
            if query.table_hash == TACT_KEY_TABLE_HASH_LIKE_CPP {
                let tact_key = (*record_id)
                    .try_into()
                    .ok()
                    .and_then(|id| tact_keys.get(id));
                if let Some(entry) = tact_key {
                    debug!(
                        "DbQueryBulk: TactKey.db2 record={} -> Valid(1), 16-byte typed WriteRecord payload",
                        record_id
                    );
                    self.send_packet_realm(&DBReply::found(
                        query.table_hash,
                        *record_id,
                        entry.key.to_vec(),
                    ));
                    continue;
                }
                debug!(
                    "DbQueryBulk: NOT_FOUND TactKey.db2 record={} -> Invalid(3), client may use local DB2 cache",
                    record_id
                );
            } else {
                info!(
                    "DbQueryBulk: table=0x{:08X} record={} -> Invalid(3), no typed DB2 storage serializer",
                    query.table_hash, record_id
                );
            }
            // RecordRemoved(2) would tell the client to delete the record from its cache,
            // which is wrong for client-local DB2 rows missing from server typed storage.
            self.send_packet_realm(&DBReply::not_found(query.table_hash, *record_id));
        }
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub async fn handle_db_query_bulk(&mut self, query: wow_packet::packets::misc::DbQueryBulk) {
        let tact_keys = self
            .tact_key_store_for_test_like_cpp()
            .cloned()
            .unwrap_or_else(|| Arc::new(wow_data::TactKeyStore::from_entries([])));
        self.handle_db_query_bulk_with_tact_keys_like_cpp(tact_keys.as_ref(), query)
            .await;
    }

    /// Handle CMSG_QUERY_CREATURE — client requests creature template data.
    ///
    /// The client sends this automatically after receiving an UpdateObject with
    /// unknown creature entries. Without a response, NPC names don't display
    /// and interaction menus don't work.
    pub(in crate::handlers::character) async fn handle_query_creature_with_catalogs_like_cpp(
        &mut self,
        catalogs: &crate::session::ObjectMgrCatalogsLikeCpp,
        query: QueryCreature,
    ) {
        let row = match catalogs
            .creature
            .resolve_like_cpp(query.creature_id, &self.locale)
        {
            Some(row) => row,
            None => {
                self.send_packet(&QueryCreatureResponse {
                    creature_id: query.creature_id,
                    allow: false,
                    stats: None,
                });
                return;
            }
        };

        let total_probability = row.displays.iter().map(|display| display.probability).sum();
        let displays = row
            .displays
            .iter()
            .map(|display| CreatureXDisplay {
                creature_display_id: display.display_id,
                scale: display.scale,
                probability: display.probability,
            })
            .collect();

        let mut names: [String; 4] = Default::default();
        names[0] = row.name;

        let stats = CreatureStats {
            title: row.subname,
            title_alt: row.title_alt,
            cursor_name: row.icon_name,
            civilian: row.civilian,
            leader: row.racial_leader,
            names,
            name_alts: Default::default(),
            flags: row.type_flags,
            creature_type: row.creature_type,
            creature_family: row.creature_family,
            classification: row.classification,
            proxy_creature_ids: row.kill_credits,
            display: CreatureDisplayStats {
                displays,
                total_probability,
            },
            hp_multi: row.hp_multi,
            energy_multi: row.energy_multi,
            quest_items: Vec::new(),
            creature_movement_info_id: row.movement_id,
            health_scaling_expansion: 0,
            required_expansion: row.required_expansion,
            vignette_id: row.vignette_id,
            unit_class: row.unit_class,
            creature_difficulty_id: row.creature_difficulty_id,
            widget_set_id: row.widget_set_id,
            widget_set_unit_condition_id: row.widget_set_unit_condition_id,
        };

        self.send_packet(&QueryCreatureResponse {
            creature_id: query.creature_id,
            allow: true,
            stats: Some(stats),
        });
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub async fn handle_query_creature(&mut self, query: QueryCreature) {
        let catalogs = self
            .world_query_catalogs_like_cpp()
            .cloned()
            .unwrap_or_default();
        self.handle_query_creature_with_catalogs_like_cpp(&catalogs, query)
            .await;
    }

    /// Handle CMSG_QUERY_GAME_OBJECT — client requests gameobject template data.
    pub(in crate::handlers::character) async fn handle_query_game_object_with_catalogs_like_cpp(
        &mut self,
        catalogs: &crate::session::ObjectMgrCatalogsLikeCpp,
        query: wow_packet::packets::query::QueryGameObject,
    ) {
        let row = match catalogs
            .gameobject
            .resolve_like_cpp(query.game_object_id, &self.locale)
        {
            Some(row) => row,
            None => {
                self.send_packet(&QueryGameObjectResponse {
                    game_object_id: query.game_object_id,
                    guid: query.guid,
                    allow: false,
                    stats: None,
                });
                return;
            }
        };

        let mut names: [String; 4] = Default::default();
        names[0] = row.name;

        let stats = GameObjectStats {
            names,
            icon_name: row.icon_name,
            cast_bar_caption: row.cast_bar_caption,
            unk_string: row.unk_string,
            go_type: row.go_type,
            display_id: row.display_id,
            data: row.data,
            size: row.size,
            quest_items: catalogs
                .gameobject_quest_items
                .get_gameobject_quest_item_list_like_cpp(query.game_object_id)
                .into_iter()
                .flatten()
                .filter_map(|item| i32::try_from(*item).ok())
                .collect(),
            content_tuning_id: row.content_tuning_id,
        };

        self.send_packet(&QueryGameObjectResponse {
            game_object_id: query.game_object_id,
            guid: query.guid,
            allow: true,
            stats: Some(stats),
        });
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub async fn handle_query_game_object(
        &mut self,
        query: wow_packet::packets::query::QueryGameObject,
    ) {
        let catalogs = self
            .world_query_catalogs_like_cpp()
            .cloned()
            .unwrap_or_default();
        self.handle_query_game_object_with_catalogs_like_cpp(&catalogs, query)
            .await;
    }

    pub(in crate::handlers::character) async fn handle_query_page_text_with_catalogs_like_cpp(
        &mut self,
        catalogs: &crate::session::ObjectMgrCatalogsLikeCpp,
        query: QueryPageText,
    ) {
        let pages = catalogs
            .page_text
            .resolve_chain_like_cpp(query.page_text_id, &self.locale);

        let pages = pages
            .into_iter()
            .map(|page| PageTextInfo {
                id: page.id,
                next_page_id: page.next_page_id,
                player_condition_id: page.player_condition_id,
                flags: page.flags,
                text: page.text,
            })
            .collect::<Vec<_>>();

        self.send_packet(&QueryPageTextResponse {
            page_text_id: query.page_text_id,
            allow: !pages.is_empty(),
            pages,
        });
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub async fn handle_query_page_text(&mut self, query: QueryPageText) {
        let catalogs = self
            .world_query_catalogs_like_cpp()
            .cloned()
            .unwrap_or_default();
        self.handle_query_page_text_with_catalogs_like_cpp(&catalogs, query)
            .await;
    }

}
