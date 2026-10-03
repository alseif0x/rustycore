//! Numeric startup boundary: 02245dcd World.cpp:1024-1033 reads signed
//! GetIntDefault/GetInt64Default before conversion into unsigned config arrays.
//! No credential keys or configuration values are logged here.
use wow_world::forever::creation::StartingConfig;

pub(super) fn load() -> (u8, StartingConfig) {
    load_with(|key| wow_config::get_value::<String>(key))
}

fn load_with(read: impl Fn(&str) -> Option<String>) -> (u8, StartingConfig) {
    // Preserve signed parse/default, THEN C++ modulo conversion, THEN source
    // range clamps in StartingPolicy. This is not a new unsigned parser.
    let integer = |key: &str, default: i32| {
        read(key)
            .and_then(|raw| raw.parse::<i32>().ok())
            .unwrap_or(default) as u32
    };
    let money = |key: &str, default: i64| {
        read(key)
            .and_then(|raw| raw.parse::<i64>().ok())
            .unwrap_or(default) as u64
    };
    let max_level = integer("MaxPlayerLevel", 90).clamp(1, 123) as u8;
    (
        max_level,
        StartingConfig {
            normal_level: integer("StartPlayerLevel", 1),
            death_knight_level: integer("StartDeathKnightPlayerLevel", 8),
            demon_hunter_level: integer("StartDemonHunterPlayerLevel", 8),
            allied_level: integer("StartAlliedRacePlayerLevel", 10),
            gm_level: integer("GM.StartLevel", 1),
            normal_money: money("StartPlayerMoney", 0),
            death_knight_money: money("StartDeathKnightPlayerMoney", 2000),
            demon_hunter_money: money("StartDemonHunterPlayerMoney", 0),
            evoker_money: money("StartEvokerPlayerMoney", 0),
            allied_money: money("StartAlliedRacePlayerMoney", 10000),
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_numeric_defaults_are_not_legacy_dk_or_db2_start_levels() {
        let (cap, config) = load_with(|_| None);
        assert_eq!(cap, 90); // reference config only, not client cap proof
        assert_eq!(
            (
                config.normal_level,
                config.death_knight_level,
                config.demon_hunter_level,
                config.allied_level,
                config.gm_level
            ),
            (1, 8, 8, 10, 1)
        );
        assert_eq!(
            (
                config.normal_money,
                config.death_knight_money,
                config.demon_hunter_money,
                config.evoker_money,
                config.allied_money
            ),
            (0, 2000, 0, 0, 10000)
        );
    }

    #[test]
    fn signed_input_conversion_precedes_unsigned_clamp_not_unsigned_default() {
        let (cap, config) = load_with(|key| {
            Some(
                match key {
                    "MaxPlayerLevel" | "StartPlayerLevel" | "StartPlayerMoney" => "-1",
                    "StartDeathKnightPlayerLevel" => "4294967295", // outside signed read
                    "StartDeathKnightPlayerMoney" => "18446744073709551615",
                    "StartDemonHunterPlayerLevel" => "+12",
                    "StartEvokerPlayerMoney" => "9223372036854775807",
                    _ => "not-an-integer",
                }
                .into(),
            )
        });
        assert_eq!(cap, 123);
        assert_eq!(config.normal_level, u32::MAX);
        assert_eq!(config.normal_money, u64::MAX);
        assert_eq!(
            (config.death_knight_level, config.death_knight_money),
            (8, 2000)
        );
        assert_eq!(config.demon_hunter_level, 12);
        assert_eq!(config.evoker_money, i64::MAX as u64);
        assert_eq!(
            (config.allied_level, config.allied_money, config.gm_level),
            (10, 10000, 1)
        );
    }

    #[test]
    fn configured_cap_retains_source_bounds_and_bad_signed_read_default() {
        for (value, expected) in [("0", 1), ("124", 123), ("4294967295", 90), ("-1", 123)] {
            let (cap, _) = load_with(|key| (key == "MaxPlayerLevel").then(|| value.into()));
            assert_eq!(cap, expected);
        }
    }
}
