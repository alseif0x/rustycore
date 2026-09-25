// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Character availability: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

/// Available race/class combinations from `class_expansion_requirement` table.
///
/// Data mirrors C++ `ObjectMgr::LoadClassExpansionRequirements` fallback rows.
/// ActiveExpansionLevel/AccountExpansionLevel: 0 for all except Death Knight (class 6)
/// which requires WotLK (active=2). MinActiveExpansionLevel is the minimum active
/// expansion across all races for that class.
pub(in crate::session) fn default_available_classes()
-> Vec<wow_packet::packets::auth::RaceClassAvailability> {
    use wow_packet::packets::auth::{ClassAvailability, RaceClassAvailability};

    // (race_id, &[(class_id, active_expansion_level, account_expansion_level)])
    let data: &[(u8, &[(u8, u8, u8)])] = &[
        (
            1,
            &[
                (1, 0, 0),
                (2, 0, 0),
                (3, 0, 0),
                (4, 0, 0),
                (5, 0, 0),
                (6, 2, 0),
                (8, 0, 0),
                (9, 0, 0),
            ],
        ), // Human
        (
            2,
            &[
                (1, 0, 0),
                (3, 0, 0),
                (4, 0, 0),
                (6, 2, 0),
                (7, 0, 0),
                (8, 0, 0),
                (9, 0, 0),
            ],
        ), // Orc
        (
            3,
            &[
                (1, 0, 0),
                (2, 0, 0),
                (3, 0, 0),
                (4, 0, 0),
                (5, 0, 0),
                (6, 2, 0),
                (7, 0, 0),
                (8, 0, 0),
                (9, 0, 0),
            ],
        ), // Dwarf
        (
            4,
            &[
                (1, 0, 0),
                (3, 0, 0),
                (4, 0, 0),
                (5, 0, 0),
                (6, 2, 0),
                (8, 0, 0),
                (11, 0, 0),
            ],
        ), // Night Elf
        (
            5,
            &[
                (1, 0, 0),
                (3, 0, 0),
                (4, 0, 0),
                (5, 0, 0),
                (6, 2, 0),
                (8, 0, 0),
                (9, 0, 0),
            ],
        ), // Undead
        (
            6,
            &[
                (1, 0, 0),
                (2, 0, 0),
                (3, 0, 0),
                (5, 0, 0),
                (6, 2, 0),
                (7, 0, 0),
                (11, 0, 0),
            ],
        ), // Tauren
        (
            7,
            &[
                (1, 0, 0),
                (3, 0, 0),
                (4, 0, 0),
                (5, 0, 0),
                (6, 2, 0),
                (8, 0, 0),
                (9, 0, 0),
            ],
        ), // Gnome
        (
            8,
            &[
                (1, 0, 0),
                (3, 0, 0),
                (4, 0, 0),
                (5, 0, 0),
                (6, 2, 0),
                (7, 0, 0),
                (8, 0, 0),
                (9, 0, 0),
                (11, 0, 0),
            ],
        ), // Troll
        (
            10,
            &[
                (1, 0, 0),
                (2, 0, 0),
                (3, 0, 0),
                (4, 0, 0),
                (5, 0, 0),
                (6, 2, 0),
                (8, 0, 0),
                (9, 0, 0),
            ],
        ), // Blood Elf
        (
            11,
            &[
                (1, 0, 0),
                (2, 0, 0),
                (3, 0, 0),
                (5, 0, 0),
                (6, 2, 0),
                (7, 0, 0),
                (8, 0, 0),
            ],
        ), // Draenei
    ];

    // MinActiveExpansionLevel per class = min across all races for that class
    // All classes have active=0 across all races except class 6 (DK) which is always 2
    let min_active = |class_id: u8| -> u8 { if class_id == 6 { 2 } else { 0 } };

    data.iter()
        .map(|&(race_id, classes)| RaceClassAvailability {
            race_id,
            classes: classes
                .iter()
                .map(|&(class_id, active_exp, account_exp)| ClassAvailability {
                    class_id,
                    active_expansion_level: active_exp,
                    account_expansion_level: account_exp,
                    min_active_expansion_level: min_active(class_id),
                })
                .collect(),
        })
        .collect()
}
