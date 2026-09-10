//! Immutable C++ `ObjectMgr` query projections loaded before listeners start.
//!
//! C++ ownership anchors: `ObjectMgr::LoadCreatureTemplates` /
//! `LoadCreatureLocales`, `ObjectMgr::LoadGameObjectTemplate` /
//! `LoadGameObjectTemplateAddons` / `LoadGameObjectLocales`, and
//! `ObjectMgr::LoadPageTexts` / `LoadPageTextLocales`.

use std::collections::{HashMap, HashSet};

pub const WORLD_QUERY_GAMEOBJECT_DATA_COUNT_LIKE_CPP: usize = 35;
pub const PAGE_TEXT_QUERY_CHAIN_DEFENSIVE_LIMIT_LIKE_CPP: usize = 100;

#[derive(Debug, Clone, PartialEq)]
pub struct CreatureQueryTemplateLikeCpp {
    pub entry: u32,
    pub name: String,
    pub subname: String,
    pub title_alt: String,
    pub icon_name: String,
    pub creature_type: i32,
    pub creature_family: i32,
    pub classification: i32,
    pub kill_credits: [i32; 2],
    pub civilian: bool,
    pub racial_leader: bool,
    pub movement_id: i32,
    pub required_expansion: i32,
    pub vignette_id: i32,
    pub unit_class: i32,
    pub widget_set_id: i32,
    pub widget_set_unit_condition_id: i32,
    pub hp_multi: f32,
    pub energy_multi: f32,
    pub creature_difficulty_id: i32,
    pub type_flags: [u32; 2],
    pub displays: Vec<CreatureQueryDisplayLikeCpp>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CreatureQueryDisplayLikeCpp {
    pub display_id: u32,
    pub scale: f32,
    pub probability: f32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatureQueryLocaleLikeCpp {
    pub entry: u32,
    pub locale: String,
    pub name: String,
    pub subname: String,
    pub title_alt: String,
}

#[derive(Debug, Clone, Default)]
pub struct CreatureQueryCatalogLikeCpp {
    templates: HashMap<u32, CreatureQueryTemplateLikeCpp>,
    locales: HashMap<(u32, String), CreatureQueryLocaleLikeCpp>,
}

impl CreatureQueryCatalogLikeCpp {
    pub fn from_rows_like_cpp(
        templates: impl IntoIterator<Item = CreatureQueryTemplateLikeCpp>,
        locales: impl IntoIterator<Item = CreatureQueryLocaleLikeCpp>,
    ) -> Self {
        Self {
            templates: templates.into_iter().map(|row| (row.entry, row)).collect(),
            locales: locales
                .into_iter()
                .map(|row| ((row.entry, row.locale.clone()), row))
                .collect(),
        }
    }

    pub fn resolve_like_cpp(
        &self,
        entry: u32,
        locale: &str,
    ) -> Option<CreatureQueryTemplateLikeCpp> {
        let mut row = self.templates.get(&entry)?.clone();
        if !locale.is_empty()
            && locale != "enUS"
            && let Some(localized) = self.locales.get(&(entry, locale.to_owned()))
        {
            if !localized.name.is_empty() {
                row.name.clone_from(&localized.name);
            }
            if !localized.subname.is_empty() {
                row.subname.clone_from(&localized.subname);
            }
            if !localized.title_alt.is_empty() {
                row.title_alt.clone_from(&localized.title_alt);
            }
        }
        Some(row)
    }

    pub fn len(&self) -> usize {
        self.templates.len()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GameObjectQueryTemplateLikeCpp {
    pub entry: u32,
    pub go_type: i32,
    pub display_id: i32,
    pub name: String,
    pub icon_name: String,
    pub cast_bar_caption: String,
    pub unk_string: String,
    pub size: f32,
    pub data: [i32; WORLD_QUERY_GAMEOBJECT_DATA_COUNT_LIKE_CPP],
    pub content_tuning_id: i32,
    pub min_money: u32,
    pub max_money: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameObjectQueryLocaleLikeCpp {
    pub entry: u32,
    pub locale: String,
    pub name: String,
    pub cast_bar_caption: String,
    pub unk_string: String,
}

#[derive(Debug, Clone, Default)]
pub struct GameObjectQueryCatalogLikeCpp {
    templates: HashMap<u32, GameObjectQueryTemplateLikeCpp>,
    locales: HashMap<(u32, String), GameObjectQueryLocaleLikeCpp>,
}

impl GameObjectQueryCatalogLikeCpp {
    pub fn from_rows_like_cpp(
        templates: impl IntoIterator<Item = GameObjectQueryTemplateLikeCpp>,
        locales: impl IntoIterator<Item = GameObjectQueryLocaleLikeCpp>,
    ) -> Self {
        Self {
            templates: templates.into_iter().map(|row| (row.entry, row)).collect(),
            locales: locales
                .into_iter()
                .map(|row| ((row.entry, row.locale.clone()), row))
                .collect(),
        }
    }

    pub fn resolve_like_cpp(
        &self,
        entry: u32,
        locale: &str,
    ) -> Option<GameObjectQueryTemplateLikeCpp> {
        let mut row = self.templates.get(&entry)?.clone();
        if !locale.is_empty()
            && locale != "enUS"
            && let Some(localized) = self.locales.get(&(entry, locale.to_owned()))
        {
            if !localized.name.is_empty() {
                row.name.clone_from(&localized.name);
            }
            if !localized.cast_bar_caption.is_empty() {
                row.cast_bar_caption.clone_from(&localized.cast_bar_caption);
            }
            if !localized.unk_string.is_empty() {
                row.unk_string.clone_from(&localized.unk_string);
            }
        }
        Some(row)
    }

    pub fn get(&self, entry: u32) -> Option<&GameObjectQueryTemplateLikeCpp> {
        self.templates.get(&entry)
    }

    pub fn len(&self) -> usize {
        self.templates.len()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageTextLikeCpp {
    pub id: u32,
    pub text: String,
    pub next_page_id: u32,
    pub player_condition_id: i32,
    pub flags: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageTextLocaleLikeCpp {
    pub id: u32,
    pub locale: String,
    pub text: String,
}

#[derive(Debug, Clone, Default)]
pub struct PageTextCatalogLikeCpp {
    pages: HashMap<u32, PageTextLikeCpp>,
    locales: HashMap<(u32, String), String>,
}

impl PageTextCatalogLikeCpp {
    pub fn from_rows_like_cpp(
        pages: impl IntoIterator<Item = PageTextLikeCpp>,
        locales: impl IntoIterator<Item = PageTextLocaleLikeCpp>,
    ) -> Self {
        Self {
            pages: pages.into_iter().map(|row| (row.id, row)).collect(),
            locales: locales
                .into_iter()
                .map(|row| ((row.id, row.locale), row.text))
                .collect(),
        }
    }

    pub fn resolve_chain_like_cpp(&self, first_id: u32, locale: &str) -> Vec<PageTextLikeCpp> {
        let mut pages = Vec::new();
        let mut visited = HashSet::new();
        let mut id = first_id;
        while id != 0
            && visited.insert(id)
            && pages.len() < PAGE_TEXT_QUERY_CHAIN_DEFENSIVE_LIMIT_LIKE_CPP
        {
            let Some(mut page) = self.pages.get(&id).cloned() else {
                break;
            };
            if !locale.is_empty()
                && locale != "enUS"
                && let Some(text) = self.locales.get(&(id, locale.to_owned()))
                && !text.is_empty()
            {
                page.text.clone_from(text);
            }
            id = page.next_page_id;
            pages.push(page);
        }
        pages
    }

    pub fn len(&self) -> usize {
        self.pages.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn creature_template() -> CreatureQueryTemplateLikeCpp {
        CreatureQueryTemplateLikeCpp {
            entry: 7,
            name: "base name".into(),
            subname: "base title".into(),
            title_alt: "base title alt".into(),
            icon_name: "talk".into(),
            creature_type: 1,
            creature_family: 2,
            classification: 3,
            kill_credits: [4, 5],
            civilian: true,
            racial_leader: false,
            movement_id: 6,
            required_expansion: 7,
            vignette_id: 8,
            unit_class: 9,
            widget_set_id: 10,
            widget_set_unit_condition_id: 11,
            hp_multi: 1.5,
            energy_multi: 2.0,
            creature_difficulty_id: 12,
            type_flags: [13, 14],
            displays: vec![CreatureQueryDisplayLikeCpp {
                display_id: 15,
                scale: 1.25,
                probability: 0.75,
            }],
        }
    }

    #[test]
    fn creature_locale_overlay_replaces_only_nonempty_strings_like_cpp() {
        let base = creature_template();
        let catalog = CreatureQueryCatalogLikeCpp::from_rows_like_cpp(
            [base.clone()],
            [CreatureQueryLocaleLikeCpp {
                entry: base.entry,
                locale: "esES".into(),
                name: "nombre".into(),
                subname: String::new(),
                title_alt: "titulo alternativo".into(),
            }],
        );

        let localized = catalog.resolve_like_cpp(base.entry, "esES").unwrap();
        assert_eq!(localized.name, "nombre");
        assert_eq!(localized.subname, base.subname);
        assert_eq!(localized.title_alt, "titulo alternativo");
        assert_eq!(localized.icon_name, base.icon_name);
        assert_eq!(localized.displays, base.displays);
        assert_eq!(catalog.resolve_like_cpp(base.entry, "enUS"), Some(base));
        assert!(catalog.resolve_like_cpp(999, "esES").is_none());
    }

    #[test]
    fn gameobject_locale_overlay_preserves_template_and_money_like_cpp() {
        let base = GameObjectQueryTemplateLikeCpp {
            entry: 9,
            go_type: 3,
            display_id: 4,
            name: "base name".into(),
            icon_name: "loot".into(),
            cast_bar_caption: "base caption".into(),
            unk_string: "base unknown".into(),
            size: 1.25,
            data: [17; WORLD_QUERY_GAMEOBJECT_DATA_COUNT_LIKE_CPP],
            content_tuning_id: 18,
            min_money: 19,
            max_money: 20,
        };
        let catalog = GameObjectQueryCatalogLikeCpp::from_rows_like_cpp(
            [base.clone()],
            [GameObjectQueryLocaleLikeCpp {
                entry: base.entry,
                locale: "esES".into(),
                name: "nombre".into(),
                cast_bar_caption: String::new(),
                unk_string: "desconocido".into(),
            }],
        );

        let localized = catalog.resolve_like_cpp(base.entry, "esES").unwrap();
        assert_eq!(localized.name, "nombre");
        assert_eq!(localized.cast_bar_caption, base.cast_bar_caption);
        assert_eq!(localized.unk_string, "desconocido");
        assert_eq!(localized.data, base.data);
        assert_eq!((localized.min_money, localized.max_money), (19, 20));
        assert_eq!(catalog.resolve_like_cpp(base.entry, "enUS"), Some(base));
        assert!(catalog.resolve_like_cpp(999, "esES").is_none());
    }

    #[test]
    fn locale_overlays_are_sparse_and_page_cycles_stop_like_cpp() {
        let catalog = PageTextCatalogLikeCpp::from_rows_like_cpp(
            [
                PageTextLikeCpp {
                    id: 1,
                    text: "one".into(),
                    next_page_id: 2,
                    player_condition_id: 0,
                    flags: 0,
                },
                PageTextLikeCpp {
                    id: 2,
                    text: "two".into(),
                    next_page_id: 1,
                    player_condition_id: 0,
                    flags: 0,
                },
            ],
            [PageTextLocaleLikeCpp {
                id: 1,
                locale: "esES".into(),
                text: "uno".into(),
            }],
        );
        let pages = catalog.resolve_chain_like_cpp(1, "esES");
        assert_eq!(pages.len(), 2);
        assert_eq!(pages[0].text, "uno");
        assert_eq!(pages[1].text, "two");
    }
}
