// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Strict logical-module ownership policy for handler capabilities.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::Deserialize;

const POLICY_SCHEMA_VERSION: u32 = 1;
const REQUIRED_CAPABILITIES: &[&str] = &["handler_registration", "packet_dispatcher"];

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct HandlerModulePolicyDocument {
    schema_version: u32,
    introduced_by_issue: u32,
    capability_owners: Vec<CapabilityOwner>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct CapabilityOwner {
    pub(crate) capability: String,
    pub(crate) package: String,
    pub(crate) module: String,
    pub(crate) allow_descendants: bool,
    pub(crate) tracking_issue: u32,
}

impl CapabilityOwner {
    pub(crate) fn owns_module(&self, package: &str, logical_module: &str) -> bool {
        package == self.package
            && (logical_module == self.module
                || (self.allow_descendants
                    && logical_module
                        .strip_prefix(&self.module)
                        .is_some_and(|suffix| suffix.starts_with("::"))))
    }
}

#[derive(Debug)]
pub(crate) struct HandlerModulePolicy {
    owners: BTreeMap<String, CapabilityOwner>,
}

impl HandlerModulePolicy {
    pub(crate) fn owner(&self, capability: &str) -> &CapabilityOwner {
        self.owners
            .get(capability)
            .expect("validated policy contains every required capability")
    }
}

fn valid_logical_module(module: &str) -> bool {
    if module == "crate" {
        return true;
    }
    let Some(suffix) = module.strip_prefix("crate::") else {
        return false;
    };
    !suffix.is_empty()
        && suffix
            .split("::")
            .all(|segment| !segment.is_empty() && syn::parse_str::<syn::Ident>(segment).is_ok())
}

pub(crate) fn parse_handler_module_policy(source: &str) -> Result<HandlerModulePolicy, String> {
    let document: HandlerModulePolicyDocument = serde_json::from_str(source)
        .map_err(|error| format!("cannot parse handler module policy: {error}"))?;
    if document.schema_version != POLICY_SCHEMA_VERSION {
        return Err(format!(
            "handler module policy schema_version must be {POLICY_SCHEMA_VERSION}"
        ));
    }
    if document.introduced_by_issue == 0 {
        return Err("handler module policy introduced_by_issue must be positive".to_owned());
    }

    let mut owners = BTreeMap::new();
    for owner in document.capability_owners {
        if !REQUIRED_CAPABILITIES.contains(&owner.capability.as_str()) {
            return Err(format!(
                "handler module policy declares unknown capability {}",
                owner.capability
            ));
        }
        if owner.package.is_empty() {
            return Err(format!(
                "handler module policy capability {} has an empty package",
                owner.capability
            ));
        }
        if !valid_logical_module(&owner.module) {
            return Err(format!(
                "handler module policy capability {} has invalid logical module {:?}",
                owner.capability, owner.module
            ));
        }
        if owner.tracking_issue == 0 {
            return Err(format!(
                "handler module policy capability {} has no tracking issue",
                owner.capability
            ));
        }
        let capability = owner.capability.clone();
        if owners.insert(capability.clone(), owner).is_some() {
            return Err(format!(
                "handler module policy declares duplicate capability {capability}"
            ));
        }
    }
    let actual: Vec<_> = owners.keys().map(String::as_str).collect();
    if actual != REQUIRED_CAPABILITIES {
        return Err(format!(
            "handler module policy must declare capabilities {REQUIRED_CAPABILITIES:?} exactly once; found {actual:?}"
        ));
    }
    let owner_values: Vec<_> = owners.values().collect();
    for (index, left) in owner_values.iter().enumerate() {
        for right in &owner_values[index + 1..] {
            if left.package == right.package
                && (left.owns_module(&right.package, &right.module)
                    || right.owns_module(&left.package, &left.module))
            {
                return Err(format!(
                    "handler module policy capabilities {} and {} have overlapping logical owners in package {}: {} and {}",
                    left.capability, right.capability, left.package, left.module, right.module
                ));
            }
        }
    }
    Ok(HandlerModulePolicy { owners })
}

pub(crate) fn load_handler_module_policy(path: &Path) -> Result<HandlerModulePolicy, String> {
    let source = fs::read_to_string(path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    parse_handler_module_policy(&source)
        .map_err(|error| format!("invalid {}: {error}", path.display()))
}
