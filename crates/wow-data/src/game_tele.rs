// Copyright (c) 2026 alseif0x
// RustyCore - WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 - https://www.gnu.org/licenses/gpl-3.0.html

//! C++ `ObjectMgr::LoadGameTele` data model.

use std::collections::HashMap;

use anyhow::Result;
use wow_core::Position;
use wow_database::{WorldDatabase, WorldStatements};

#[derive(Debug, Clone, PartialEq)]
pub struct GameTeleLikeCpp {
    pub id: u32,
    pub position: Position,
    pub map_id: u32,
    pub name: String,
    pub name_lower_like_cpp: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GameTeleRowLikeCpp {
    pub id: u32,
    pub position_x: f32,
    pub position_y: f32,
    pub position_z: f32,
    pub orientation: f32,
    pub map_id: u16,
    pub name: String,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct GameTeleLoadReportLikeCpp {
    pub rows_seen: usize,
    pub loaded_rows: usize,
    pub skipped_invalid_coordinates: Vec<(u32, String)>,
}

#[derive(Debug, Default, Clone)]
pub struct GameTeleStoreLikeCpp {
    entries: HashMap<u32, GameTeleLikeCpp>,
}

pub struct GameTeleLoadOutcomeLikeCpp {
    pub store: GameTeleStoreLikeCpp,
    pub report: GameTeleLoadReportLikeCpp,
}

impl GameTeleStoreLikeCpp {
    pub fn from_rows_like_cpp(
        rows: impl IntoIterator<Item = GameTeleRowLikeCpp>,
    ) -> GameTeleLoadOutcomeLikeCpp {
        let mut report = GameTeleLoadReportLikeCpp::default();
        let mut entries = HashMap::new();

        for row in rows {
            report.rows_seen += 1;
            let position = Position::new(
                row.position_x,
                row.position_y,
                row.position_z,
                row.orientation,
            );

            if !position.is_valid_map_coord_like_cpp() {
                report
                    .skipped_invalid_coordinates
                    .push((row.id, row.name.clone()));
                continue;
            }

            entries.insert(
                row.id,
                GameTeleLikeCpp {
                    id: row.id,
                    position,
                    map_id: u32::from(row.map_id),
                    name_lower_like_cpp: normalize_game_tele_name_like_cpp(&row.name),
                    name: row.name,
                },
            );
            report.loaded_rows += 1;
        }

        GameTeleLoadOutcomeLikeCpp {
            store: Self { entries },
            report,
        }
    }

    /// C++ `ObjectMgr::LoadGameTele`.
    pub async fn load_like_cpp(db: &WorldDatabase) -> Result<GameTeleLoadOutcomeLikeCpp> {
        let stmt = db.prepare(WorldStatements::SEL_GAME_TELE);
        let mut result = db.query(&stmt).await?;
        let mut rows = Vec::new();

        if !result.is_empty() {
            loop {
                rows.push(GameTeleRowLikeCpp {
                    id: result.read(0),
                    position_x: result.read(1),
                    position_y: result.read(2),
                    position_z: result.read(3),
                    orientation: result.read(4),
                    map_id: result.read(5),
                    name: result.read(6),
                });

                if !result.next_row() {
                    break;
                }
            }
        }

        Ok(Self::from_rows_like_cpp(rows))
    }

    /// C++ `ObjectMgr::GetGameTele(uint32 id)`.
    pub fn get_game_tele_by_id_like_cpp(&self, id: u32) -> Option<&GameTeleLikeCpp> {
        self.entries.get(&id)
    }

    /// C++ `ObjectMgr::GetGameTele(std::string_view name)`.
    pub fn get_game_tele_like_cpp(&self, name: &str) -> Option<&GameTeleLikeCpp> {
        let name_lower = normalize_game_tele_name_like_cpp(name);
        let mut partial = None;

        for tele in self.entries.values() {
            if tele.name_lower_like_cpp == name_lower {
                return Some(tele);
            }

            if partial.is_none() && tele.name_lower_like_cpp.contains(&name_lower) {
                partial = Some(tele);
            }
        }

        partial
    }

    /// C++ `ObjectMgr::GetGameTeleExactName`.
    pub fn get_game_tele_exact_name_like_cpp(&self, name: &str) -> Option<&GameTeleLikeCpp> {
        let name_lower = normalize_game_tele_name_like_cpp(name);
        self.entries
            .values()
            .find(|tele| tele.name_lower_like_cpp == name_lower)
    }

    /// C++ `ObjectMgr::GetGameTeleMap`.
    pub fn entries_like_cpp(&self) -> &HashMap<u32, GameTeleLikeCpp> {
        &self.entries
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

fn normalize_game_tele_name_like_cpp(name: &str) -> String {
    // C++ converts UTF-8 to wide chars and applies `wstrToLower`.
    name.to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(id: u32, name: &str) -> GameTeleRowLikeCpp {
        GameTeleRowLikeCpp {
            id,
            position_x: 1.0,
            position_y: 2.0,
            position_z: 3.0,
            orientation: 4.0,
            map_id: 571,
            name: name.to_string(),
        }
    }

    #[test]
    fn game_tele_load_skips_invalid_coordinates_like_cpp() {
        let mut invalid = row(2, "Bad");
        invalid.position_x = Position::MAP_HALFSIZE_LIKE_CPP + 1.0;

        let outcome = GameTeleStoreLikeCpp::from_rows_like_cpp([row(1, "Dalaran"), invalid]);

        assert_eq!(outcome.report.rows_seen, 2);
        assert_eq!(outcome.report.loaded_rows, 1);
        assert_eq!(
            outcome.report.skipped_invalid_coordinates,
            vec![(2, "Bad".to_string())]
        );
        assert!(outcome.store.get_game_tele_by_id_like_cpp(1).is_some());
        assert!(outcome.store.get_game_tele_by_id_like_cpp(2).is_none());
    }

    #[test]
    fn game_tele_duplicate_id_overwrites_but_counts_loaded_rows_like_cpp() {
        let outcome =
            GameTeleStoreLikeCpp::from_rows_like_cpp([row(1, "OldDalaran"), row(1, "NewDalaran")]);

        assert_eq!(outcome.report.loaded_rows, 2);
        assert_eq!(outcome.store.len(), 1);
        assert_eq!(
            outcome.store.get_game_tele_by_id_like_cpp(1).unwrap().name,
            "NewDalaran"
        );
    }

    #[test]
    fn game_tele_lookup_exact_wins_before_partial_like_cpp() {
        let outcome = GameTeleStoreLikeCpp::from_rows_like_cpp([
            row(1, "StormwindKeep"),
            row(2, "Stormwind"),
        ]);

        assert_eq!(
            outcome
                .store
                .get_game_tele_like_cpp("stormwind")
                .unwrap()
                .id,
            2
        );
        assert_eq!(
            outcome
                .store
                .get_game_tele_exact_name_like_cpp("STORMWIND")
                .unwrap()
                .id,
            2
        );
        assert!(
            outcome
                .store
                .get_game_tele_exact_name_like_cpp("wind")
                .is_none()
        );
        assert!(outcome.store.get_game_tele_like_cpp("wind").is_some());
    }
}
