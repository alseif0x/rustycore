// Copyright (c) 2026 alseif0x
// RustyCore - WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 - https://www.gnu.org/licenses/gpl-3.0.html

//! C++ `ObjectMgr` gossip menu condition attachment primitives.

use std::collections::HashMap;

use anyhow::Result;
use wow_constants::ConditionSourceType;
use wow_constants::shared::Locale;
use wow_database::{WorldDatabase, WorldStatements};

use crate::{ConditionEntriesByTypeStore, ConditionId, ConditionsReference};

#[derive(Debug, Clone, Default)]
pub struct GossipMenu {
    pub menu_id: u32,
    pub text_id: u32,
    pub conditions: ConditionsReference,
}

#[derive(Debug, Clone, Default)]
pub struct GossipMenuItem {
    pub menu_id: u32,
    pub gossip_option_id: i32,
    pub order_index: u32,
    pub option_npc: u8,
    pub option_text: String,
    pub option_broadcast_text_id: u32,
    pub language: u32,
    pub flags: i32,
    pub action_menu_id: u32,
    pub action_poi_id: u32,
    pub gossip_npc_option_id: Option<i32>,
    pub box_coded: bool,
    pub box_money: u32,
    pub box_text: String,
    pub box_broadcast_text_id: u32,
    pub spell_id: Option<i32>,
    pub override_icon_id: Option<i32>,
    pub conditions: ConditionsReference,
}

#[derive(Debug, Clone, Default)]
pub struct GossipMenuItemsLocaleLikeCpp {
    option_text: HashMap<Locale, String>,
    box_text: HashMap<Locale, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GossipMenuItemsLocaleRowLikeCpp {
    pub menu_id: u32,
    pub option_id: u32,
    pub locale: String,
    pub option_text: String,
    pub box_text: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GossipMenuAddonLikeCpp {
    pub friendship_faction_id: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GossipMenuAddonRowLikeCpp {
    pub menu_id: u32,
    pub friendship_faction_id: i32,
}

impl GossipMenuItemsLocaleLikeCpp {
    pub fn option_text_like_cpp(&self, locale: Locale) -> Option<&str> {
        self.option_text.get(&locale).map(String::as_str)
    }

    pub fn box_text_like_cpp(&self, locale: Locale) -> Option<&str> {
        self.box_text.get(&locale).map(String::as_str)
    }
}

#[derive(Debug, Clone, Default)]
pub struct GossipStore {
    menus_by_id: HashMap<u32, Vec<GossipMenu>>,
    items_by_menu_id: HashMap<u32, Vec<GossipMenuItem>>,
    locales_by_menu_option: HashMap<(u32, u32), GossipMenuItemsLocaleLikeCpp>,
    addons_by_menu_id: HashMap<u32, GossipMenuAddonLikeCpp>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GossipLoadReport {
    pub menu_rows: usize,
    pub menu_item_rows: usize,
    pub locale_rows_seen: usize,
    pub locale_entries: usize,
    pub addon_rows: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GossipConditionAttachmentReport {
    pub attached_condition_count: usize,
    pub missing_menus: Vec<ConditionId>,
    pub missing_menu_items: Vec<ConditionId>,
}

impl GossipStore {
    /// C++ `ObjectMgr::LoadGossipMenu` + `LoadGossipMenuItems` +
    /// `LoadGossipMenuItemsLocales` + `LoadGossipMenuAddon`, represented
    /// without cross-store validation.
    pub async fn load_like_cpp(db: &WorldDatabase) -> Result<(Self, GossipLoadReport)> {
        let mut store = Self::default();
        let mut report = GossipLoadReport::default();

        let stmt = db.prepare(WorldStatements::SEL_GOSSIP_MENUS);
        let mut result = db.query(&stmt).await?;
        if !result.is_empty() {
            loop {
                store.add_menu_like_cpp(result.read(0), result.read(1));
                report.menu_rows += 1;
                if !result.next_row() {
                    break;
                }
            }
        }

        let stmt = db.prepare(WorldStatements::SEL_GOSSIP_MENU_OPTIONS_ALL);
        let mut result = db.query(&stmt).await?;
        if !result.is_empty() {
            loop {
                store.add_menu_item_full_like_cpp(GossipMenuItem {
                    menu_id: result.read(0),
                    gossip_option_id: result.read(1),
                    order_index: result.read(2),
                    option_npc: result.read(3),
                    option_text: result.read_string(4),
                    option_broadcast_text_id: result.read(5),
                    language: result.read(6),
                    flags: result.read(7),
                    action_menu_id: result.read(8),
                    action_poi_id: result.read(9),
                    gossip_npc_option_id: result.try_read(10),
                    box_coded: result.try_read::<u8>(11).unwrap_or(0) != 0,
                    box_money: result.read(12),
                    box_text: result.read_string(13),
                    box_broadcast_text_id: result.read(14),
                    spell_id: result.try_read(15),
                    override_icon_id: result.try_read(16),
                    conditions: ConditionsReference::default(),
                });
                report.menu_item_rows += 1;
                if !result.next_row() {
                    break;
                }
            }
        }

        let stmt = db.prepare(WorldStatements::SEL_GOSSIP_MENU_OPTION_LOCALES);
        let mut result = db.query(&stmt).await?;
        if !result.is_empty() {
            loop {
                report.locale_rows_seen += 1;
                store.add_menu_item_locale_like_cpp(GossipMenuItemsLocaleRowLikeCpp {
                    menu_id: result.read(0),
                    option_id: result.read(1),
                    locale: result.read_string(2),
                    option_text: result.read_string(3),
                    box_text: result.read_string(4),
                });
                if !result.next_row() {
                    break;
                }
            }
        }
        report.locale_entries = store.locales_by_menu_option.len();

        let stmt = db.prepare(WorldStatements::SEL_GOSSIP_MENU_ADDON);
        let mut result = db.query(&stmt).await?;
        if !result.is_empty() {
            loop {
                store.add_menu_addon_like_cpp(GossipMenuAddonRowLikeCpp {
                    menu_id: result.read(0),
                    friendship_faction_id: result.read(1),
                });
                report.addon_rows += 1;
                if !result.next_row() {
                    break;
                }
            }
        }

        Ok((store, report))
    }

    pub fn menu_row_count(&self) -> usize {
        self.menus_by_id.values().map(Vec::len).sum()
    }

    pub fn menu_item_row_count(&self) -> usize {
        self.items_by_menu_id.values().map(Vec::len).sum()
    }

    pub fn menu_item_locale_count(&self) -> usize {
        self.locales_by_menu_option.len()
    }

    pub fn menu_addon_count(&self) -> usize {
        self.addons_by_menu_id.len()
    }

    pub fn add_menu_like_cpp(&mut self, menu_id: u32, text_id: u32) {
        self.menus_by_id
            .entry(menu_id)
            .or_default()
            .push(GossipMenu {
                menu_id,
                text_id,
                conditions: ConditionsReference::default(),
            });
    }

    pub fn add_menu_item_like_cpp(&mut self, menu_id: u32, order_index: u32) {
        self.add_menu_item_full_like_cpp(GossipMenuItem {
            menu_id,
            order_index,
            ..GossipMenuItem::default()
        });
    }

    pub fn add_menu_item_full_like_cpp(&mut self, item: GossipMenuItem) {
        self.items_by_menu_id
            .entry(item.menu_id)
            .or_default()
            .push(item);
    }

    pub fn add_menu_item_locale_like_cpp(&mut self, row: GossipMenuItemsLocaleRowLikeCpp) {
        let Some(locale) = locale_from_name_like_cpp(&row.locale) else {
            return;
        };
        if locale == Locale::EnUS {
            return;
        }

        let locale_entry = self
            .locales_by_menu_option
            .entry((row.menu_id, row.option_id))
            .or_default();
        locale_entry.option_text.insert(locale, row.option_text);
        locale_entry.box_text.insert(locale, row.box_text);
    }

    pub fn add_menu_addon_like_cpp(&mut self, row: GossipMenuAddonRowLikeCpp) {
        self.addons_by_menu_id.insert(
            row.menu_id,
            GossipMenuAddonLikeCpp {
                friendship_faction_id: row.friendship_faction_id,
            },
        );
    }

    pub fn menus_for_id(&self, menu_id: u32) -> Option<&[GossipMenu]> {
        self.menus_by_id.get(&menu_id).map(Vec::as_slice)
    }

    pub fn menu_items_for_id(&self, menu_id: u32) -> Option<&[GossipMenuItem]> {
        self.items_by_menu_id.get(&menu_id).map(Vec::as_slice)
    }

    /// C++ `ObjectMgr::GetGossipMenuItemsLocale`.
    pub fn menu_item_locale_like_cpp(
        &self,
        menu_id: u32,
        option_id: u32,
    ) -> Option<&GossipMenuItemsLocaleLikeCpp> {
        self.locales_by_menu_option.get(&(menu_id, option_id))
    }

    /// C++ `ObjectMgr::GetGossipMenuAddon`.
    pub fn menu_addon_like_cpp(&self, menu_id: u32) -> Option<&GossipMenuAddonLikeCpp> {
        self.addons_by_menu_id.get(&menu_id)
    }

    /// C++ `ConditionMgr::addToGossipMenus`.
    pub fn attach_gossip_menu_conditions_like_cpp(
        &mut self,
        conditions: &ConditionEntriesByTypeStore,
    ) -> GossipConditionAttachmentReport {
        let mut report = GossipConditionAttachmentReport::default();
        let Some(menu_conditions) =
            conditions.entries_for_source_type_like_cpp(ConditionSourceType::GossipMenu)
        else {
            return report;
        };

        for (id, condition_bucket) in menu_conditions {
            let Some(menus) = self.menus_by_id.get_mut(&id.source_group) else {
                report.missing_menus.push(*id);
                continue;
            };

            for menu in menus {
                if id.source_entry == 0 || menu.text_id == id.source_entry as u32 {
                    menu.conditions = ConditionsReference::new(condition_bucket);
                    report.attached_condition_count += condition_bucket.len();
                }
            }
        }

        report
    }

    /// C++ `ConditionMgr::addToGossipMenuItems`.
    pub fn attach_gossip_menu_item_conditions_like_cpp(
        &mut self,
        conditions: &ConditionEntriesByTypeStore,
    ) -> GossipConditionAttachmentReport {
        let mut report = GossipConditionAttachmentReport::default();
        let Some(item_conditions) =
            conditions.entries_for_source_type_like_cpp(ConditionSourceType::GossipMenuOption)
        else {
            return report;
        };

        for (id, condition_bucket) in item_conditions {
            let Some(items) = self.items_by_menu_id.get_mut(&id.source_group) else {
                report.missing_menu_items.push(*id);
                continue;
            };

            let Some(item) = items
                .iter_mut()
                .find(|item| item.order_index == id.source_entry as u32)
            else {
                report.missing_menu_items.push(*id);
                continue;
            };

            item.conditions = ConditionsReference::new(condition_bucket);
            report.attached_condition_count += condition_bucket.len();
        }

        report
    }
}

fn locale_from_name_like_cpp(name: &str) -> Option<Locale> {
    match name {
        "enUS" => Some(Locale::EnUS),
        "koKR" => Some(Locale::KoKR),
        "frFR" => Some(Locale::FrFR),
        "deDE" => Some(Locale::DeDE),
        "zhCN" => Some(Locale::ZhCN),
        "zhTW" => Some(Locale::ZhTW),
        "esES" => Some(Locale::EsES),
        "esMX" => Some(Locale::EsMX),
        "ruRU" => Some(Locale::RuRU),
        "none" => Some(Locale::None),
        "ptBR" => Some(Locale::PtBR),
        "itIT" => Some(Locale::ItIT),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Condition;
    use wow_constants::ConditionType;

    fn condition(source_type: ConditionSourceType, menu_id: u32, source_entry: i32) -> Condition {
        Condition {
            source_type,
            source_group: menu_id,
            source_entry,
            condition_type: ConditionType::Aura,
            condition_value1: 100,
            ..Condition::default()
        }
    }

    #[test]
    fn gossip_menu_conditions_attach_to_matching_text_or_all_texts_like_cpp() {
        let mut gossip = GossipStore::default();
        gossip.add_menu_like_cpp(7, 10);
        gossip.add_menu_like_cpp(7, 20);
        let store = ConditionEntriesByTypeStore::from_conditions_like_cpp([
            condition(ConditionSourceType::GossipMenu, 7, 10),
            condition(ConditionSourceType::GossipMenu, 7, 0),
        ]);

        let report = gossip.attach_gossip_menu_conditions_like_cpp(&store);

        assert_eq!(report.attached_condition_count, 3);
        assert!(report.missing_menus.is_empty());
        let menus = gossip.menus_for_id(7).unwrap();
        assert_eq!(menus[0].conditions.upgrade().unwrap().len(), 1);
        assert_eq!(menus[1].conditions.upgrade().unwrap().len(), 1);
    }

    #[test]
    fn gossip_menu_conditions_do_not_report_text_mismatch_when_menu_exists_like_cpp() {
        let mut gossip = GossipStore::default();
        gossip.add_menu_like_cpp(7, 10);
        let store = ConditionEntriesByTypeStore::from_conditions_like_cpp([condition(
            ConditionSourceType::GossipMenu,
            7,
            99,
        )]);

        let report = gossip.attach_gossip_menu_conditions_like_cpp(&store);

        assert_eq!(report.attached_condition_count, 0);
        assert!(report.missing_menus.is_empty());
        assert!(gossip.menus_for_id(7).unwrap()[0].conditions.is_expired());
    }

    #[test]
    fn gossip_menu_item_conditions_attach_by_menu_and_order_index_like_cpp() {
        let mut gossip = GossipStore::default();
        gossip.add_menu_item_like_cpp(7, 2);
        let store = ConditionEntriesByTypeStore::from_conditions_like_cpp([condition(
            ConditionSourceType::GossipMenuOption,
            7,
            2,
        )]);

        let report = gossip.attach_gossip_menu_item_conditions_like_cpp(&store);

        assert_eq!(report.attached_condition_count, 1);
        assert!(report.missing_menu_items.is_empty());
        assert_eq!(
            gossip.menu_items_for_id(7).unwrap()[0]
                .conditions
                .upgrade()
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn gossip_menu_item_conditions_report_missing_item_like_cpp() {
        let mut gossip = GossipStore::default();
        gossip.add_menu_item_like_cpp(7, 2);
        let missing = ConditionId::new(7, 3, 0);
        let store = ConditionEntriesByTypeStore::from_conditions_like_cpp([condition(
            ConditionSourceType::GossipMenuOption,
            7,
            3,
        )]);

        let report = gossip.attach_gossip_menu_item_conditions_like_cpp(&store);

        assert_eq!(report.attached_condition_count, 0);
        assert_eq!(report.missing_menu_items, vec![missing]);
    }

    #[test]
    fn gossip_menu_item_full_rows_preserve_object_mgr_fields_like_cpp() {
        let mut gossip = GossipStore::default();
        gossip.add_menu_item_full_like_cpp(GossipMenuItem {
            menu_id: 7,
            gossip_option_id: 11,
            order_index: 2,
            option_npc: 3,
            option_text: "Train me".to_string(),
            option_broadcast_text_id: 44,
            language: 1,
            flags: 5,
            action_menu_id: 8,
            action_poi_id: 9,
            gossip_npc_option_id: Some(10),
            box_coded: true,
            box_money: 123,
            box_text: "Pay?".to_string(),
            box_broadcast_text_id: 45,
            spell_id: Some(46),
            override_icon_id: Some(47),
            conditions: ConditionsReference::default(),
        });

        let item = &gossip.menu_items_for_id(7).unwrap()[0];
        assert_eq!(item.gossip_option_id, 11);
        assert_eq!(item.option_text, "Train me");
        assert_eq!(item.gossip_npc_option_id, Some(10));
        assert!(item.box_coded);
        assert_eq!(item.spell_id, Some(46));
    }

    #[test]
    fn gossip_menu_item_locales_skip_enus_like_cpp() {
        let mut gossip = GossipStore::default();
        gossip.add_menu_item_locale_like_cpp(GossipMenuItemsLocaleRowLikeCpp {
            menu_id: 7,
            option_id: 2,
            locale: "enUS".to_string(),
            option_text: "Hello".to_string(),
            box_text: "Box".to_string(),
        });
        gossip.add_menu_item_locale_like_cpp(GossipMenuItemsLocaleRowLikeCpp {
            menu_id: 7,
            option_id: 2,
            locale: "esES".to_string(),
            option_text: "Hola".to_string(),
            box_text: "Caja".to_string(),
        });

        let locale = gossip.menu_item_locale_like_cpp(7, 2).unwrap();
        assert_eq!(locale.option_text_like_cpp(Locale::EsES), Some("Hola"));
        assert_eq!(locale.box_text_like_cpp(Locale::EsES), Some("Caja"));
        assert_eq!(locale.option_text_like_cpp(Locale::EnUS), None);
    }

    #[test]
    fn gossip_menu_addon_overwrites_by_menu_id_like_cpp() {
        let mut gossip = GossipStore::default();
        gossip.add_menu_addon_like_cpp(GossipMenuAddonRowLikeCpp {
            menu_id: 7,
            friendship_faction_id: 100,
        });
        gossip.add_menu_addon_like_cpp(GossipMenuAddonRowLikeCpp {
            menu_id: 7,
            friendship_faction_id: 200,
        });

        assert_eq!(gossip.menu_addon_count(), 1);
        assert_eq!(
            gossip.menu_addon_like_cpp(7).unwrap().friendship_faction_id,
            200
        );
    }
}
