//! ItemSpecStats, 02245dcd ObjectMgr.cpp:3110-3317; DBCEnums::ItemSpecStat.
pub(super) struct SpecStats {
    pub item_type: u8,
    stats: Vec<u8>,
}
impl SpecStats {
    pub(super) fn fields(
        class: i32,
        subclass: u8,
        inventory: i8,
        mods: [i32; 10],
        gem_kind: Option<i32>,
    ) -> Self {
        let mut result = Self {
            item_type: 0,
            stats: Vec::new(),
        };
        match class {
            2 => {
                result.item_type = 5;
                let stat = match subclass {
                    0 => Some(7),
                    1 => Some(8),
                    2 => Some(16),
                    3 => Some(15),
                    4 => Some(11),
                    5 => Some(12),
                    6 => Some(19),
                    7 => Some(9),
                    8 => Some(10),
                    9 => Some(28),
                    10 => Some(18),
                    13 => Some(14),
                    15 => Some(13),
                    16 => Some(20),
                    18 => Some(17),
                    19 => Some(21),
                    _ => None,
                };
                if let Some(stat) = stat {
                    result.add(stat);
                }
            }
            4 => match subclass {
                1 if inventory == 16 => result.add(27),
                1..=4 => result.item_type = subclass,
                6 => {
                    result.item_type = 6;
                    result.add(22);
                }
                7..=11 => {
                    result.item_type = 6;
                    result.add(23);
                }
                _ => {}
            },
            3 => {
                result.item_type = 7;
                if let Some(kind) = gem_kind {
                    for i in 0..11u8 {
                        if kind as u32 & (0x40u32 << i) != 0 {
                            result.add(29 + i);
                        }
                    }
                }
            }
            _ => {}
        }
        for stat in mods {
            result.add_mod(stat);
        }
        result
    }
    fn add(&mut self, stat: u8) {
        if self.stats.len() < 10 && !self.stats.contains(&stat) {
            self.stats.push(stat);
        }
    }
    fn add_mod(&mut self, stat: i32) {
        // Deliberately do not generalize old hit/haste rating variants that
        // this source switch does not recognize.
        match stat {
            3 => self.add(1),
            4 => self.add(2),
            5 => self.add(0),
            13 => self.add(5),
            14 => self.add(6),
            19..=21 | 32 => self.add(24),
            36 => self.add(25),
            31 => self.add(4),
            50 => self.add(26),
            71 => {
                self.add(1);
                self.add(2);
                self.add(0);
            }
            72 => {
                self.add(1);
                self.add(2);
            }
            73 => {
                self.add(1);
                self.add(0);
            }
            74 => {
                self.add(2);
                self.add(0);
            }
            _ => {}
        }
    }
    pub(super) fn has(&self, stat: u8) -> bool {
        stat == 40 || self.stats.contains(&stat)
    }
}
