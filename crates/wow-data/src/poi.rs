// Copyright (c) 2026 alseif0x
// RustyCore - WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 - https://www.gnu.org/licenses/gpl-3.0.html

//! C++ `ObjectMgr::LoadPointsOfInterest` data model.

use std::collections::HashMap;

use anyhow::Result;
use tracing::info;
use wow_constants::shared::Locale;
use wow_core::Position;
use wow_database::{WorldDatabase, WorldStatements};

#[derive(Debug, Clone, PartialEq)]
pub struct PointOfInterestLikeCpp {
    pub id: u32,
    pub position: Position,
    pub icon: u32,
    pub flags: u32,
    pub importance: u32,
    pub name: String,
    pub wmo_group_id: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PointOfInterestRowLikeCpp {
    pub id: u32,
    pub position_x: f32,
    pub position_y: f32,
    pub position_z: f32,
    pub icon: u32,
    pub flags: u32,
    pub importance: u32,
    pub name: String,
    pub wmo_group_id: i32,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct PointOfInterestLoadReportLikeCpp {
    pub rows_seen: usize,
    pub loaded_rows: usize,
    pub skipped_invalid_coordinates: Vec<(u32, f32, f32, f32)>,
}

#[derive(Debug, Default, Clone)]
pub struct PointOfInterestStoreLikeCpp {
    entries: HashMap<u32, PointOfInterestLikeCpp>,
}

pub struct PointOfInterestLoadOutcomeLikeCpp {
    pub store: PointOfInterestStoreLikeCpp,
    pub report: PointOfInterestLoadReportLikeCpp,
}

impl PointOfInterestStoreLikeCpp {
    pub fn from_rows_like_cpp(
        rows: impl IntoIterator<Item = PointOfInterestRowLikeCpp>,
    ) -> PointOfInterestLoadOutcomeLikeCpp {
        let mut report = PointOfInterestLoadReportLikeCpp::default();
        let mut entries = HashMap::new();

        for row in rows {
            report.rows_seen += 1;

            let position = Position::xyz(row.position_x, row.position_y, row.position_z);
            if !position.is_valid_map_coord_like_cpp() {
                report.skipped_invalid_coordinates.push((
                    row.id,
                    row.position_x,
                    row.position_y,
                    row.position_z,
                ));
                continue;
            }

            entries.insert(
                row.id,
                PointOfInterestLikeCpp {
                    id: row.id,
                    position,
                    icon: row.icon,
                    flags: row.flags,
                    importance: row.importance,
                    name: row.name,
                    wmo_group_id: row.wmo_group_id,
                },
            );
            report.loaded_rows += 1;
        }

        PointOfInterestLoadOutcomeLikeCpp {
            store: Self { entries },
            report,
        }
    }

    /// C++ `ObjectMgr::LoadPointsOfInterest`.
    pub async fn load_like_cpp(db: &WorldDatabase) -> Result<PointOfInterestLoadOutcomeLikeCpp> {
        let stmt = db.prepare(WorldStatements::SEL_POINTS_OF_INTEREST);
        let mut result = db.query(&stmt).await?;
        let mut rows = Vec::new();

        if !result.is_empty() {
            loop {
                rows.push(PointOfInterestRowLikeCpp {
                    id: result.read(0),
                    position_x: result.read(1),
                    position_y: result.read(2),
                    position_z: result.read(3),
                    icon: result.read(4),
                    flags: result.read(5),
                    importance: result.read(6),
                    name: result.read(7),
                    wmo_group_id: result.read(8),
                });

                if !result.next_row() {
                    break;
                }
            }
        }

        let outcome = Self::from_rows_like_cpp(rows);
        info!(
            "Loaded {} Points of Interest definitions",
            outcome.report.loaded_rows
        );
        Ok(outcome)
    }

    /// C++ `ObjectMgr::GetPointOfInterest`.
    pub fn get_point_of_interest_like_cpp(&self, id: u32) -> Option<&PointOfInterestLikeCpp> {
        self.entries.get(&id)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PointOfInterestLocaleRowLikeCpp {
    pub id: u32,
    pub locale: String,
    pub name: String,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct PointOfInterestLocaleLikeCpp {
    names: HashMap<Locale, String>,
}

impl PointOfInterestLocaleLikeCpp {
    pub fn name_like_cpp(&self, locale: Locale) -> Option<&str> {
        self.names.get(&locale).map(String::as_str)
    }

    pub fn len(&self) -> usize {
        self.names.len()
    }

    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }
}

#[derive(Debug, Default, Clone)]
pub struct PointOfInterestLocaleStoreLikeCpp {
    entries: HashMap<u32, PointOfInterestLocaleLikeCpp>,
}

impl PointOfInterestLocaleStoreLikeCpp {
    pub fn from_rows_like_cpp(
        rows: impl IntoIterator<Item = PointOfInterestLocaleRowLikeCpp>,
    ) -> Self {
        let mut entries = HashMap::<u32, PointOfInterestLocaleLikeCpp>::new();

        for row in rows {
            let Some(locale) = locale_from_name_like_cpp(&row.locale) else {
                continue;
            };
            if locale == Locale::EnUS {
                continue;
            }

            entries
                .entry(row.id)
                .or_default()
                .names
                .insert(locale, row.name);
        }

        Self { entries }
    }

    /// C++ `ObjectMgr::LoadPointOfInterestLocales`.
    pub async fn load_like_cpp(db: &WorldDatabase) -> Result<Self> {
        let stmt = db.prepare(WorldStatements::SEL_POINTS_OF_INTEREST_LOCALES);
        let mut result = db.query(&stmt).await?;
        let mut rows = Vec::new();

        if !result.is_empty() {
            loop {
                rows.push(PointOfInterestLocaleRowLikeCpp {
                    id: result.read(0),
                    locale: result.read(1),
                    name: result.read(2),
                });

                if !result.next_row() {
                    break;
                }
            }
        }

        let store = Self::from_rows_like_cpp(rows);
        info!("Loaded {} points_of_interest locale strings", store.len());
        Ok(store)
    }

    /// C++ `ObjectMgr::GetPointOfInterestLocale`.
    pub fn get_point_of_interest_locale_like_cpp(
        &self,
        id: u32,
    ) -> Option<&PointOfInterestLocaleLikeCpp> {
        self.entries.get(&id)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
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

    fn poi_row(id: u32, x: f32, y: f32, z: f32) -> PointOfInterestRowLikeCpp {
        PointOfInterestRowLikeCpp {
            id,
            position_x: x,
            position_y: y,
            position_z: z,
            icon: 7,
            flags: 8,
            importance: 9,
            name: format!("poi-{id}"),
            wmo_group_id: -1,
        }
    }

    fn locale_row(id: u32, locale: &str, name: &str) -> PointOfInterestLocaleRowLikeCpp {
        PointOfInterestLocaleRowLikeCpp {
            id,
            locale: locale.to_string(),
            name: name.to_string(),
        }
    }

    #[test]
    fn points_of_interest_skip_invalid_coordinates_like_cpp() {
        let limit = Position::MAP_HALFSIZE_LIKE_CPP - 0.5;
        let outcome = PointOfInterestStoreLikeCpp::from_rows_like_cpp([
            poi_row(1, limit, -limit, 0.0),
            poi_row(2, limit + 1.0, 0.0, 0.0),
            poi_row(3, 0.0, f32::NAN, 0.0),
        ]);

        assert_eq!(outcome.report.rows_seen, 3);
        assert_eq!(outcome.report.loaded_rows, 1);
        assert_eq!(outcome.report.skipped_invalid_coordinates.len(), 2);
        assert!(outcome.store.get_point_of_interest_like_cpp(1).is_some());
        assert!(outcome.store.get_point_of_interest_like_cpp(2).is_none());
    }

    #[test]
    fn points_of_interest_duplicate_id_overwrites_like_cpp() {
        let outcome =
            PointOfInterestStoreLikeCpp::from_rows_like_cpp([poi_row(1, 1.0, 2.0, 3.0), {
                let mut row = poi_row(1, 4.0, 5.0, 6.0);
                row.name = "replacement".to_string();
                row
            }]);

        let poi = outcome
            .store
            .get_point_of_interest_like_cpp(1)
            .expect("duplicate ID should keep one map entry");

        assert_eq!(poi.position.x, 4.0);
        assert_eq!(poi.name, "replacement");
        assert_eq!(outcome.report.loaded_rows, 2);
        assert_eq!(outcome.store.len(), 1);
    }

    #[test]
    fn point_of_interest_locales_skip_invalid_and_enus_like_cpp() {
        let store = PointOfInterestLocaleStoreLikeCpp::from_rows_like_cpp([
            locale_row(1, "enUS", "Default"),
            locale_row(1, "esES", "Espanol"),
            locale_row(1, "bad", "Ignored"),
            locale_row(2, "frFR", "Francais"),
        ]);

        let locale = store
            .get_point_of_interest_locale_like_cpp(1)
            .expect("esES locale should be loaded");
        assert_eq!(locale.len(), 1);
        assert_eq!(locale.name_like_cpp(Locale::EsES), Some("Espanol"));
        assert!(locale.name_like_cpp(Locale::EnUS).is_none());
        assert!(store.get_point_of_interest_locale_like_cpp(3).is_none());
    }
}
