//! Isolated World consumes the same explicit bindings as the existing BNet
//! adapter. No fallback pretending the inherited Auth schema has contentSetId.
use anyhow::{Result, ensure};
use serde::Deserialize;
use std::collections::HashSet;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Binding {
    #[serde(rename = "realmAddress")]
    address: u32,
    #[serde(rename = "contentSetID")]
    content: i32,
    #[serde(rename = "superDistrictID")]
    district: i32,
}

pub(super) fn isolated_binding(config: &str) -> Result<(i32, i32)> {
    ensure!(config.len() <= 16 * 1024, "ruleset configuration bound");
    let bindings: Vec<Binding> = serde_json::from_str(config)?;
    ensure!(bindings.len() <= 64, "ruleset count bound");
    let mut addresses = HashSet::new();
    let mut contents = HashSet::new();
    let mut selected = None;
    for binding in bindings {
        ensure!(
            binding.address >> 24 != 0
                && binding.address >> 16 & 0xFF != 0
                && binding.address & 0xFFFF != 0
                && binding.content > 0
                && binding.district > 0
                && addresses.insert(binding.address)
                && contents.insert(binding.content),
            "invalid ruleset binding"
        );
        if binding.address == 0x02010001 {
            ensure!(
                wow_world::forever::selection::super_district_for_content_set(
                    binding.content as u32,
                    binding.district
                ) == binding.district,
                "ruleset differs from target content mapping"
            );
            selected = Some((binding.content, binding.district));
        }
    }
    selected.ok_or_else(|| anyhow::anyhow!("isolated realm ruleset binding required"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn world_and_bnet_use_the_same_target_content_and_district() {
        for (content, district) in [(136, 1), (137, 2), (138, 3), (140, 4)] {
            let json = format!(
                r#"[{{"realmAddress":33619969,"contentSetID":{content},"superDistrictID":{district}}}]"#
            );
            assert_eq!(isolated_binding(&json).unwrap(), (content, district));
        }
    }
    #[test]
    fn absent_duplicate_wrong_field_wrong_realm_and_conflicting_ruleset_fail_closed() {
        for json in [
            "[]",
            r#"[{"realmAddress":33619969,"contentSetID":136,"superDistrictID":2}]"#,
            r#"[{"realmAddress":33619970,"contentSetID":136,"superDistrictID":1}]"#,
            r#"[{"realmAddress":33619969,"contentSetId":136,"superDistrictID":1}]"#,
            r#"[{"realmAddress":33619969,"contentSetID":136,"superDistrictID":1},{"realmAddress":33619969,"contentSetID":137,"superDistrictID":2}]"#,
        ] {
            assert!(isolated_binding(json).is_err());
        }
        assert!(isolated_binding(&" ".repeat(16 * 1024 + 1)).is_err());
    }
}
