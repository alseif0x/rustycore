// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Syntax-level ratchets for the world-session ownership refactor.
//!
//! This module deliberately records exact source surfaces rather than line
//! counts. It cannot prove runtime ordering or semantic writer authority; those
//! remain behavior-test contracts. It does prevent an unreviewed field,
//! external impl, setter, visible method, construction-bag field, or factory
//! fan-out change from entering through ordinary Rust syntax.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use proc_macro2::{TokenStream, TokenTree};
use quote::ToTokens;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use syn::visit::Visit;
use syn::{Expr, ImplItem, Item, ItemImpl, ItemStruct, Type, UseTree, Visibility};

use crate::bridge_access::{
    BridgeAccessBaseline, BridgeSource, compare_bridge_access_baseline, inventory_bridge_accesses,
    validate_curated_bridge_anchors,
};
use crate::ownership::{
    SourceMountContext, audit_package_source_mounts, cfg_context_allows_production,
    cfg_context_allows_test, extend_cfg_context, workspace_dependency_aliases,
    workspace_source_mounts,
};
use crate::persistence_access::{
    ClassifiedPersistenceSource, PersistenceAccessBaseline, compare_persistence_access_baseline,
    inventory_persistence_accesses_with_dependencies, render_persistence_access_baseline,
};
use crate::registry_access::{
    ProductionRegistrySource, RegistryAccessBaseline, compare_registry_access_baseline,
    inventory_registry_accesses,
};

const POLICY_RELATIVE_PATH: &str = "tools/architecture/session-ownership-policy.json";
const PERSISTENCE_POLICY_RELATIVE_PATH: &str =
    "tools/architecture/persistence-boundary-policy.json";
const PERSISTENCE_ANNOTATIONS_RELATIVE_PATH: &str =
    "tools/architecture/persistence-boundary-workflows.json";
const PERSISTENCE_ACCESS_SNAPSHOT_RELATIVE_PATH: &str =
    "tools/architecture/persistence-access-snapshot.json";
const ISSUE_LEDGER_RELATIVE_PATH: &str = "tools/architecture/architecture-issue-ledger.json";
const WORLD_PACKAGE_ROOT: &str = "crates/wow-world";
const WORLD_CRATE_ROOT: &str = "crates/wow-world/src/lib.rs";
const SERVER_PACKAGE_ROOT: &str = "crates/world-server";
const SERVER_CRATE_ROOT: &str = "crates/world-server/src/main.rs";
const NETWORK_PACKAGE_ROOT: &str = "crates/wow-network";
const NETWORK_CRATE_ROOT: &str = "crates/wow-network/src/lib.rs";
const WORLD_SESSION_MODULE: &str = "crate::session";
const WORLD_SESSION_NAME: &str = "WorldSession";
const SESSION_RESOURCES_MODULE: &str = "crate::session_resources";
const SESSION_RESOURCES_NAME: &str = "SessionResources";
const SESSION_FACTORY_MODULE: &str = "crate";
const SESSION_FACTORY_NAME: &str = "create_session";
const SESSION_COMMAND_NAME: &str = "SessionCommand";
const PLAYER_BROADCAST_INFO_NAME: &str = "PlayerBroadcastInfo";
const FNV1A_64_PRIME: u64 = 0x0000_0100_0000_01b3;
const FNV1A_64_OFFSET_A: u64 = 0xcbf2_9ce4_8422_2325;
const FNV1A_64_OFFSET_B: u64 = 0x8422_2325_cbf2_9ce4;
const OWNERSHIP_TARGET_NAMES: [&str; 4] = [
    WORLD_SESSION_NAME,
    SESSION_RESOURCES_NAME,
    SESSION_COMMAND_NAME,
    PLAYER_BROADCAST_INFO_NAME,
];

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DefinitionSurface {
    pub module: String,
    pub name: String,
    pub visibility: String,
    pub cfg: Vec<String>,
    pub source_class: String,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FieldSurface {
    pub name: String,
    #[serde(rename = "type")]
    pub type_expression: String,
    pub visibility: String,
    pub cfg: Vec<String>,
    pub source_class: String,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VariantSurface {
    pub name: String,
    pub fields: Vec<FieldSurface>,
    pub discriminant: Option<String>,
    pub cfg: Vec<String>,
    pub source_class: String,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TypeSurface {
    pub definition: DefinitionSurface,
    pub kind: String,
    pub fields: Vec<FieldSurface>,
    pub variants: Vec<VariantSurface>,
    pub alias: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ImplSurface {
    pub module: String,
    pub trait_path: Option<String>,
    pub cfg: Vec<String>,
    pub source_class: String,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ImplItemSurface {
    pub module: String,
    pub trait_path: Option<String>,
    pub kind: String,
    pub name: String,
    pub visibility: String,
    pub signature: String,
    pub cfg: Vec<String>,
    pub source_class: String,
    pub guards: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CallSurface {
    pub module: String,
    pub callee: String,
    pub argument_count: usize,
    pub cfg: Vec<String>,
    pub source_class: String,
    pub count: usize,
}

/// An attribute that can synthesize or structurally change an audited surface.
///
/// `cfg`, documentation, and lint attributes are modeled elsewhere or are
/// inert. Derives and every other attribute on an audited definition/item are
/// recorded exactly so a newly introduced procedural or codegen input cannot
/// bypass the syntax baseline.
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GeneratedSurfaceInput {
    pub module: String,
    pub target: String,
    pub kind: String,
    pub fingerprint: String,
    pub cfg: Vec<String>,
    pub source_class: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorldSessionSurface {
    pub definition: DefinitionSurface,
    pub fields: Vec<FieldSurface>,
    pub impls: Vec<ImplSurface>,
    pub impl_items: Vec<ImplItemSurface>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SessionResourcesSurface {
    pub definition: DefinitionSurface,
    pub fields: Vec<FieldSurface>,
    pub construction_sites: Vec<CallSurface>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SessionFactorySurface {
    pub definition: DefinitionSurface,
    pub signature: String,
    pub body_fingerprint: String,
    pub session_helper_bodies: Vec<SessionFactoryHelperSurface>,
    pub call_sites: Vec<CallSurface>,
    pub world_session_new_sites: Vec<CallSurface>,
    pub setter_call_sites: Vec<CallSurface>,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SessionFactoryHelperSurface {
    pub module: String,
    pub name: String,
    pub signature: String,
    pub body_fingerprint: String,
    pub cfg: Vec<String>,
    pub source_class: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SessionSyntaxBaseline {
    pub world_session: WorldSessionSurface,
    pub session_resources: SessionResourcesSurface,
    pub session_factory: SessionFactorySurface,
    pub session_command: TypeSurface,
    pub session_command_payload_types: Vec<TypeSurface>,
    pub player_broadcast_info: TypeSurface,
    pub generated_surface_inputs: Vec<GeneratedSurfaceInput>,
    pub(crate) registry_accesses: RegistryAccessBaseline,
    #[serde(skip)]
    pub(crate) persistence_accesses: PersistenceAccessBaseline,
    pub(crate) bridge_accesses: BridgeAccessBaseline,
}

#[derive(Debug, Deserialize)]
struct PolicyEnvelope {
    schema_version: u64,
    syntax_baseline: SessionSyntaxBaseline,
    persistence_access_snapshot: String,
    /// Legacy semantic fields remain accepted here; the Rust workflow-policy
    /// validator owns the exact persistence responsibility ledger.
    /// Keeping that data flattened here isolates the AST schema from it.
    #[serde(flatten)]
    _semantic_policy: BTreeMap<String, Value>,
}

#[derive(Serialize)]
struct BaselineEnvelope<'a> {
    schema_version: u64,
    persistence_access_snapshot: &'static str,
    syntax_baseline: &'a SessionSyntaxBaseline,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PackageRole {
    World,
    Server,
    Network,
}

struct SourceUnit {
    role: PackageRole,
    source_path: PathBuf,
    repository_relative_path: String,
    logical_module_path: String,
    cfg: Vec<String>,
    availability: Availability,
    source: String,
}

impl PackageRole {
    fn package_name(self) -> &'static str {
        match self {
            Self::World => "wow-world",
            Self::Server => "world-server",
            Self::Network => "wow-network",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Availability {
    production: bool,
    test: bool,
}

impl Availability {
    fn source_class(self) -> Option<&'static str> {
        if self.production {
            Some("production")
        } else if self.test {
            Some("test_fixture")
        } else {
            None
        }
    }
}

#[derive(Default)]
struct BaselineBuilder {
    errors: Vec<String>,
    world_session_definition: Option<DefinitionSurface>,
    world_session_fields: BTreeSet<FieldSurface>,
    world_session_impls: BTreeSet<(String, Option<String>, Vec<String>, String)>,
    world_session_impl_items: BTreeSet<ImplItemSurface>,
    session_resources_definition: Option<DefinitionSurface>,
    session_resources_fields: BTreeSet<FieldSurface>,
    session_resources_constructions: BTreeMap<(String, String, usize, Vec<String>, String), usize>,
    session_factory_definition: Option<DefinitionSurface>,
    session_factory_signature: Option<String>,
    session_factory_body_fingerprint: Option<String>,
    session_factory_helper_calls: BTreeSet<(String, Vec<String>)>,
    server_function_bodies: BTreeMap<(String, String), BTreeSet<SessionFactoryHelperSurface>>,
    session_factory_calls: BTreeMap<(String, String, usize, Vec<String>, String), usize>,
    world_session_new_calls: BTreeMap<(String, String, usize, Vec<String>, String), usize>,
    session_factory_setter_calls: BTreeMap<(String, String, usize, Vec<String>, String), usize>,
    generated_surface_inputs: BTreeSet<GeneratedSurfaceInput>,
    network_types: BTreeMap<String, Vec<NetworkTypeDefinition>>,
}

#[derive(Clone, Debug)]
struct NetworkTypeDefinition {
    surface: TypeSurface,
    referenced_types: BTreeSet<String>,
    generated_surface_inputs: BTreeSet<GeneratedSurfaceInput>,
}

fn normalized_tokens(value: &impl ToTokens) -> String {
    value.to_token_stream().to_string()
}

/// Stable compact fingerprint for large normalized syntax surfaces. This is a
/// deterministic drift identity, not a security primitive.
fn compact_fingerprint(value: &str) -> String {
    fn lane<'a>(bytes: impl IntoIterator<Item = &'a u8>, offset: u64) -> u64 {
        bytes.into_iter().fold(offset, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(FNV1A_64_PRIME)
        })
    }

    let bytes = value.as_bytes();
    let forward = lane(bytes.iter(), FNV1A_64_OFFSET_A);
    let backward = lane(bytes.iter().rev(), FNV1A_64_OFFSET_B);
    format!(
        "fnv1a64x2:{forward:016x}{backward:016x}:len={}",
        bytes.len()
    )
}

fn compact_token_fingerprint(value: &impl ToTokens) -> String {
    compact_fingerprint(&normalized_tokens(value))
}

fn normalized_visibility(visibility: &Visibility) -> String {
    match visibility {
        Visibility::Inherited => "private".to_owned(),
        _ => normalized_tokens(visibility),
    }
}

fn attribute_is_inert_for_generated_surface(attribute: &syn::Attribute) -> bool {
    let path = attribute.path();
    path.is_ident("cfg")
        || path.is_ident("cfg_attr")
        || path.is_ident("doc")
        || path.is_ident("allow")
        || path.is_ident("warn")
        || path.is_ident("deny")
        || path.is_ident("forbid")
        || path.is_ident("expect")
}

fn generated_attribute_inputs(
    module: &str,
    target: &str,
    attributes: &[syn::Attribute],
    cfg: &[String],
    availability: Availability,
) -> BTreeSet<GeneratedSurfaceInput> {
    let Some(source_class) = availability.source_class() else {
        return BTreeSet::new();
    };
    attributes
        .iter()
        .filter(|attribute| !attribute_is_inert_for_generated_surface(attribute))
        .map(|attribute| GeneratedSurfaceInput {
            module: module.to_owned(),
            target: target.to_owned(),
            kind: if attribute.path().is_ident("derive") {
                "derive"
            } else {
                "attribute"
            }
            .to_owned(),
            fingerprint: normalized_tokens(&attribute.meta),
            cfg: cfg.to_vec(),
            source_class: source_class.to_owned(),
        })
        .collect()
}

fn is_visible(visibility: &Visibility) -> bool {
    !matches!(visibility, Visibility::Inherited)
}

fn set_once<T: Eq + std::fmt::Debug>(
    slot: &mut Option<T>,
    value: T,
    label: &str,
    errors: &mut Vec<String>,
) {
    match slot {
        None => *slot = Some(value),
        Some(previous) if previous == &value => errors.push(format!(
            "{label} is mounted more than once with the same source surface: {value:?}"
        )),
        Some(previous) => errors.push(format!(
            "{label} has conflicting definitions: {previous:?} and {value:?}"
        )),
    }
}

fn type_path_ends_with(type_expression: &Type, expected: &str) -> bool {
    match type_expression {
        Type::Path(path) if path.qself.is_none() => path
            .path
            .segments
            .last()
            .is_some_and(|segment| segment.ident == expected),
        Type::Group(group) => type_path_ends_with(&group.elem, expected),
        Type::Paren(paren) => type_path_ends_with(&paren.elem, expected),
        _ => false,
    }
}

fn token_stream_mentions_ident(tokens: &TokenStream, expected: &str) -> bool {
    tokens.clone().into_iter().any(|token| match token {
        TokenTree::Ident(ident) => ident == expected,
        TokenTree::Group(group) => token_stream_mentions_ident(&group.stream(), expected),
        TokenTree::Punct(_) | TokenTree::Literal(_) => false,
    })
}

fn token_stream_mentions_ownership_target(tokens: &TokenStream) -> Option<&'static str> {
    OWNERSHIP_TARGET_NAMES
        .into_iter()
        .find(|target| token_stream_mentions_ident(tokens, target))
}

#[derive(Default)]
struct IncludeMacroGuard {
    count: usize,
}

impl<'ast> Visit<'ast> for IncludeMacroGuard {
    fn visit_macro(&mut self, item_macro: &'ast syn::Macro) {
        if item_macro
            .path
            .segments
            .last()
            .is_some_and(|segment| segment.ident == "include")
        {
            self.count += 1;
        }
        syn::visit::visit_macro(self, item_macro);
    }
}

fn use_tree_renames_ident(tree: &UseTree, expected: &str) -> bool {
    match tree {
        UseTree::Rename(rename) => rename.ident == expected,
        UseTree::Path(path) => use_tree_renames_ident(&path.tree, expected),
        UseTree::Group(group) => group
            .items
            .iter()
            .any(|item| use_tree_renames_ident(item, expected)),
        UseTree::Name(_) | UseTree::Glob(_) => false,
    }
}

fn item_context(
    parent_cfg: &[String],
    parent_availability: Availability,
    attributes: &[syn::Attribute],
    context: &str,
    errors: &mut Vec<String>,
) -> (Vec<String>, Availability) {
    let cfg = extend_cfg_context(parent_cfg, attributes);
    let production = match cfg_context_allows_production(&cfg, &[]) {
        Ok(possible) => parent_availability.production && possible,
        Err(error) => {
            errors.push(format!("cannot evaluate cfg for {context}: {error}"));
            false
        }
    };
    let test = match cfg_context_allows_test(&cfg, &[]) {
        Ok(possible) => parent_availability.test && possible,
        Err(error) => {
            errors.push(format!("cannot evaluate test cfg for {context}: {error}"));
            false
        }
    };
    (cfg, Availability { production, test })
}

fn definition_surface(
    module: &str,
    name: &str,
    visibility: &Visibility,
    cfg: Vec<String>,
    source_class: &str,
) -> DefinitionSurface {
    DefinitionSurface {
        module: module.to_owned(),
        name: name.to_owned(),
        visibility: normalized_visibility(visibility),
        cfg,
        source_class: source_class.to_owned(),
    }
}

fn collect_struct(
    item: &ItemStruct,
    module: &str,
    cfg: &[String],
    availability: Availability,
    target: &str,
    definition: &mut Option<DefinitionSurface>,
    fields: &mut BTreeSet<FieldSurface>,
    generated_inputs: &mut BTreeSet<GeneratedSurfaceInput>,
    errors: &mut Vec<String>,
) {
    let (item_cfg, item_availability) =
        item_context(cfg, availability, &item.attrs, target, errors);
    let Some(item_source_class) = item_availability.source_class() else {
        return;
    };
    if item.ident != target {
        return;
    }
    let surface = definition_surface(
        module,
        target,
        &item.vis,
        item_cfg.clone(),
        item_source_class,
    );
    set_once(definition, surface, target, errors);
    generated_inputs.extend(generated_attribute_inputs(
        module,
        target,
        &item.attrs,
        &item_cfg,
        item_availability,
    ));
    let syn::Fields::Named(named_fields) = &item.fields else {
        errors.push(format!(
            "{module}::{target} must retain named fields for ownership auditing"
        ));
        return;
    };
    for field in &named_fields.named {
        let (field_cfg, field_availability) =
            item_context(&item_cfg, item_availability, &field.attrs, target, errors);
        let Some(field_source_class) = field_availability.source_class() else {
            continue;
        };
        let Some(name) = &field.ident else {
            errors.push(format!("{module}::{target} contains an unnamed field"));
            continue;
        };
        generated_inputs.extend(generated_attribute_inputs(
            module,
            &format!("{target}::{name}"),
            &field.attrs,
            &field_cfg,
            field_availability,
        ));
        fields.insert(FieldSurface {
            name: name.to_string(),
            type_expression: normalized_tokens(&field.ty),
            visibility: normalized_visibility(&field.vis),
            cfg: field_cfg,
            source_class: field_source_class.to_owned(),
        });
    }
}

#[derive(Default)]
struct TypeReferenceCollector {
    names: BTreeSet<String>,
}

impl<'ast> Visit<'ast> for TypeReferenceCollector {
    fn visit_type_path(&mut self, type_path: &'ast syn::TypePath) {
        if type_path.qself.is_none()
            && let Some(segment) = type_path.path.segments.last()
        {
            self.names.insert(segment.ident.to_string());
        }
        syn::visit::visit_type_path(self, type_path);
    }
}

fn collect_type_references(type_expression: &Type, names: &mut BTreeSet<String>) {
    let mut collector = TypeReferenceCollector::default();
    collector.visit_type(type_expression);
    names.extend(collector.names);
}

fn network_field_surfaces(
    fields: &syn::Fields,
    module: &str,
    parent_cfg: &[String],
    parent_availability: Availability,
    context: &str,
    errors: &mut Vec<String>,
    references: &mut BTreeSet<String>,
    generated_inputs: &mut BTreeSet<GeneratedSurfaceInput>,
) -> Vec<FieldSurface> {
    let mut surfaces = Vec::new();
    for (index, field) in fields.iter().enumerate() {
        let (cfg, availability) = item_context(
            parent_cfg,
            parent_availability,
            &field.attrs,
            context,
            errors,
        );
        let Some(source_class) = availability.source_class() else {
            continue;
        };
        collect_type_references(&field.ty, references);
        let name = field
            .ident
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_else(|| index.to_string());
        generated_inputs.extend(generated_attribute_inputs(
            module,
            &format!("{context}::{name}"),
            &field.attrs,
            &cfg,
            availability,
        ));
        surfaces.push(FieldSurface {
            name,
            type_expression: normalized_tokens(&field.ty),
            visibility: normalized_visibility(&field.vis),
            cfg,
            source_class: source_class.to_owned(),
        });
    }
    surfaces.sort();
    surfaces
}

fn collect_network_type(
    item: &Item,
    module: &str,
    cfg: &[String],
    availability: Availability,
    builder: &mut BaselineBuilder,
) {
    let mut referenced_types = BTreeSet::new();
    let mut generated_surface_inputs = BTreeSet::new();
    let surface = match item {
        Item::Struct(item_struct) => {
            generated_surface_inputs.extend(generated_attribute_inputs(
                module,
                &item_struct.ident.to_string(),
                &item_struct.attrs,
                cfg,
                availability,
            ));
            let fields = network_field_surfaces(
                &item_struct.fields,
                module,
                cfg,
                availability,
                &item_struct.ident.to_string(),
                &mut builder.errors,
                &mut referenced_types,
                &mut generated_surface_inputs,
            );
            TypeSurface {
                definition: definition_surface(
                    module,
                    &item_struct.ident.to_string(),
                    &item_struct.vis,
                    cfg.to_vec(),
                    availability.source_class().unwrap_or("unreachable"),
                ),
                kind: "struct".to_owned(),
                fields,
                variants: Vec::new(),
                alias: None,
            }
        }
        Item::Enum(item_enum) => {
            generated_surface_inputs.extend(generated_attribute_inputs(
                module,
                &item_enum.ident.to_string(),
                &item_enum.attrs,
                cfg,
                availability,
            ));
            let mut variants = Vec::new();
            for variant in &item_enum.variants {
                let (variant_cfg, variant_availability) = item_context(
                    cfg,
                    availability,
                    &variant.attrs,
                    &format!("{}::{}", item_enum.ident, variant.ident),
                    &mut builder.errors,
                );
                let Some(variant_source_class) = variant_availability.source_class() else {
                    continue;
                };
                let variant_target = format!("{}::{}", item_enum.ident, variant.ident);
                generated_surface_inputs.extend(generated_attribute_inputs(
                    module,
                    &variant_target,
                    &variant.attrs,
                    &variant_cfg,
                    variant_availability,
                ));
                let fields = network_field_surfaces(
                    &variant.fields,
                    module,
                    &variant_cfg,
                    variant_availability,
                    &variant_target,
                    &mut builder.errors,
                    &mut referenced_types,
                    &mut generated_surface_inputs,
                );
                variants.push(VariantSurface {
                    name: variant.ident.to_string(),
                    fields,
                    discriminant: variant
                        .discriminant
                        .as_ref()
                        .map(|(_, expression)| normalized_tokens(expression)),
                    cfg: variant_cfg,
                    source_class: variant_source_class.to_owned(),
                });
            }
            variants.sort();
            TypeSurface {
                definition: definition_surface(
                    module,
                    &item_enum.ident.to_string(),
                    &item_enum.vis,
                    cfg.to_vec(),
                    availability.source_class().unwrap_or("unreachable"),
                ),
                kind: "enum".to_owned(),
                fields: Vec::new(),
                variants,
                alias: None,
            }
        }
        Item::Type(item_type) => {
            generated_surface_inputs.extend(generated_attribute_inputs(
                module,
                &item_type.ident.to_string(),
                &item_type.attrs,
                cfg,
                availability,
            ));
            collect_type_references(&item_type.ty, &mut referenced_types);
            TypeSurface {
                definition: definition_surface(
                    module,
                    &item_type.ident.to_string(),
                    &item_type.vis,
                    cfg.to_vec(),
                    availability.source_class().unwrap_or("unreachable"),
                ),
                kind: "type_alias".to_owned(),
                fields: Vec::new(),
                variants: Vec::new(),
                alias: Some(normalized_tokens(&item_type.ty)),
            }
        }
        Item::Union(item_union) => {
            generated_surface_inputs.extend(generated_attribute_inputs(
                module,
                &item_union.ident.to_string(),
                &item_union.attrs,
                cfg,
                availability,
            ));
            let fields = network_field_surfaces(
                &syn::Fields::Named(item_union.fields.clone()),
                module,
                cfg,
                availability,
                &item_union.ident.to_string(),
                &mut builder.errors,
                &mut referenced_types,
                &mut generated_surface_inputs,
            );
            TypeSurface {
                definition: definition_surface(
                    module,
                    &item_union.ident.to_string(),
                    &item_union.vis,
                    cfg.to_vec(),
                    availability.source_class().unwrap_or("unreachable"),
                ),
                kind: "union".to_owned(),
                fields,
                variants: Vec::new(),
                alias: None,
            }
        }
        _ => return,
    };
    builder
        .network_types
        .entry(surface.definition.name.clone())
        .or_default()
        .push(NetworkTypeDefinition {
            surface,
            referenced_types,
            generated_surface_inputs,
        });
}

fn normalized_trait_path(item: &ItemImpl) -> Option<String> {
    item.trait_
        .as_ref()
        .map(|(_, path, _)| normalized_tokens(path))
}

fn impl_item_surface(
    module: &str,
    trait_path: &Option<String>,
    kind: &str,
    name: String,
    visibility: &Visibility,
    signature: String,
    cfg: Vec<String>,
    source_class: &str,
    setter: bool,
) -> ImplItemSurface {
    let mut guards = Vec::new();
    if module != WORLD_SESSION_MODULE {
        guards.push("external_impl".to_owned());
    }
    if trait_path.is_some() {
        guards.push("trait_impl".to_owned());
    }
    if is_visible(visibility) {
        guards.push("visible".to_owned());
    }
    if setter {
        guards.push("setter".to_owned());
    }
    guards.sort();
    guards.dedup();
    ImplItemSurface {
        module: module.to_owned(),
        trait_path: trait_path.clone(),
        kind: kind.to_owned(),
        name,
        visibility: normalized_visibility(visibility),
        signature,
        cfg,
        source_class: source_class.to_owned(),
        guards,
    }
}

fn collect_world_session_impl(
    item: &ItemImpl,
    module: &str,
    cfg: &[String],
    availability: Availability,
    builder: &mut BaselineBuilder,
) {
    let (impl_cfg, impl_availability) = item_context(
        cfg,
        availability,
        &item.attrs,
        "WorldSession impl",
        &mut builder.errors,
    );
    let Some(impl_source_class) = impl_availability.source_class() else {
        return;
    };
    if !type_path_ends_with(&item.self_ty, WORLD_SESSION_NAME) {
        return;
    }
    let trait_path = normalized_trait_path(item);
    let impl_target = trait_path.as_ref().map_or_else(
        || format!("impl {WORLD_SESSION_NAME}"),
        |trait_path| format!("impl {trait_path} for {WORLD_SESSION_NAME}"),
    );
    builder
        .generated_surface_inputs
        .extend(generated_attribute_inputs(
            module,
            &impl_target,
            &item.attrs,
            &impl_cfg,
            impl_availability,
        ));
    builder.world_session_impls.insert((
        module.to_owned(),
        trait_path.clone(),
        impl_cfg.clone(),
        impl_source_class.to_owned(),
    ));

    for impl_item in &item.items {
        match impl_item {
            ImplItem::Fn(function) => {
                let (cfg, availability) = item_context(
                    &impl_cfg,
                    impl_availability,
                    &function.attrs,
                    "WorldSession method",
                    &mut builder.errors,
                );
                let Some(source_class) = availability.source_class() else {
                    continue;
                };
                let name = function.sig.ident.to_string();
                builder
                    .generated_surface_inputs
                    .extend(generated_attribute_inputs(
                        module,
                        &format!("{WORLD_SESSION_NAME}::{name}"),
                        &function.attrs,
                        &cfg,
                        availability,
                    ));
                let surface = impl_item_surface(
                    module,
                    &trait_path,
                    "method",
                    name.clone(),
                    &function.vis,
                    normalized_tokens(&function.sig),
                    cfg,
                    source_class,
                    name.starts_with("set_"),
                );
                builder.world_session_impl_items.insert(surface);
            }
            ImplItem::Const(constant) => {
                let (cfg, availability) = item_context(
                    &impl_cfg,
                    impl_availability,
                    &constant.attrs,
                    "WorldSession associated const",
                    &mut builder.errors,
                );
                let Some(source_class) = availability.source_class() else {
                    continue;
                };
                builder
                    .generated_surface_inputs
                    .extend(generated_attribute_inputs(
                        module,
                        &format!("{WORLD_SESSION_NAME}::{}", constant.ident),
                        &constant.attrs,
                        &cfg,
                        availability,
                    ));
                let signature = format!(
                    "const {} : {}",
                    constant.ident,
                    normalized_tokens(&constant.ty)
                );
                let surface = impl_item_surface(
                    module,
                    &trait_path,
                    "const",
                    constant.ident.to_string(),
                    &constant.vis,
                    signature,
                    cfg,
                    source_class,
                    false,
                );
                builder.world_session_impl_items.insert(surface);
            }
            ImplItem::Type(item_type) => {
                let (cfg, availability) = item_context(
                    &impl_cfg,
                    impl_availability,
                    &item_type.attrs,
                    "WorldSession associated type",
                    &mut builder.errors,
                );
                let Some(source_class) = availability.source_class() else {
                    continue;
                };
                builder
                    .generated_surface_inputs
                    .extend(generated_attribute_inputs(
                        module,
                        &format!("{WORLD_SESSION_NAME}::{}", item_type.ident),
                        &item_type.attrs,
                        &cfg,
                        availability,
                    ));
                let signature = format!(
                    "type {} = {}",
                    item_type.ident,
                    normalized_tokens(&item_type.ty)
                );
                let surface = impl_item_surface(
                    module,
                    &trait_path,
                    "type",
                    item_type.ident.to_string(),
                    &item_type.vis,
                    signature,
                    cfg,
                    source_class,
                    false,
                );
                builder.world_session_impl_items.insert(surface);
            }
            ImplItem::Macro(item_macro) => {
                let (_, availability) = item_context(
                    &impl_cfg,
                    impl_availability,
                    &item_macro.attrs,
                    "WorldSession impl macro",
                    &mut builder.errors,
                );
                if availability.source_class().is_some() {
                    builder.errors.push(format!(
                        "{module} contains macro {}! inside impl {WORLD_SESSION_NAME}; generated \
                         associated items are outside the exact ownership grammar",
                        normalized_tokens(&item_macro.mac.path)
                    ));
                }
            }
            ImplItem::Verbatim(_) => builder.errors.push(format!(
                "{module} contains unparsed verbatim syntax inside impl {WORLD_SESSION_NAME}"
            )),
            _ => {}
        }
    }
}

fn call_key(
    module: &str,
    callee: &str,
    argument_count: usize,
    cfg: Vec<String>,
    source_class: &str,
) -> (String, String, usize, Vec<String>, String) {
    (
        module.to_owned(),
        callee.to_owned(),
        argument_count,
        cfg,
        source_class.to_owned(),
    )
}

struct ExpressionSurfaceCollector<'a> {
    module: &'a str,
    cfg: &'a [String],
    availability: Availability,
    inside_session_factory: bool,
    builder: &'a mut BaselineBuilder,
}

impl ExpressionSurfaceCollector<'_> {
    fn expression_context(&mut self, attributes: &[syn::Attribute]) -> (Vec<String>, Availability) {
        item_context(
            self.cfg,
            self.availability,
            attributes,
            "session factory expression",
            &mut self.builder.errors,
        )
    }

    fn increment(
        map: &mut BTreeMap<(String, String, usize, Vec<String>, String), usize>,
        module: &str,
        callee: &str,
        argument_count: usize,
        cfg: Vec<String>,
        source_class: &str,
    ) {
        *map.entry(call_key(module, callee, argument_count, cfg, source_class))
            .or_default() += 1;
    }
}

impl<'ast> Visit<'ast> for ExpressionSurfaceCollector<'_> {
    fn visit_expr_struct(&mut self, expression: &'ast syn::ExprStruct) {
        let (cfg, availability) = self.expression_context(&expression.attrs);
        if let Some(source_class) = availability.source_class()
            && expression
                .path
                .segments
                .last()
                .is_some_and(|segment| segment.ident == SESSION_RESOURCES_NAME)
        {
            Self::increment(
                &mut self.builder.session_resources_constructions,
                self.module,
                SESSION_RESOURCES_NAME,
                expression.fields.len(),
                cfg,
                source_class,
            );
        }
        syn::visit::visit_expr_struct(self, expression);
    }

    fn visit_expr_call(&mut self, expression: &'ast syn::ExprCall) {
        let (cfg, availability) = self.expression_context(&expression.attrs);
        if let Some(source_class) = availability.source_class() {
            if let Expr::Path(path) = expression.func.as_ref() {
                let segments: Vec<_> = path
                    .path
                    .segments
                    .iter()
                    .map(|segment| segment.ident.to_string())
                    .collect();
                if self.inside_session_factory
                    && expression.args.iter().any(|argument| {
                        token_stream_mentions_ident(&argument.to_token_stream(), "session")
                    })
                {
                    self.builder
                        .session_factory_helper_calls
                        .insert((self.module.to_owned(), segments.clone()));
                }
                if segments
                    .last()
                    .is_some_and(|name| name == SESSION_FACTORY_NAME)
                {
                    Self::increment(
                        &mut self.builder.session_factory_calls,
                        self.module,
                        SESSION_FACTORY_NAME,
                        expression.args.len(),
                        cfg.clone(),
                        source_class,
                    );
                }
                if segments
                    .as_slice()
                    .ends_with(&[WORLD_SESSION_NAME.to_owned(), "new".to_owned()])
                {
                    Self::increment(
                        &mut self.builder.world_session_new_calls,
                        self.module,
                        "WorldSession::new",
                        expression.args.len(),
                        cfg,
                        source_class,
                    );
                }
            }
        }
        syn::visit::visit_expr_call(self, expression);
    }

    fn visit_expr_method_call(&mut self, expression: &'ast syn::ExprMethodCall) {
        let (cfg, availability) = self.expression_context(&expression.attrs);
        let method = expression.method.to_string();
        if let Some(source_class) = availability.source_class()
            && self.inside_session_factory
            && (method.starts_with("set_") || method.starts_with("install_"))
        {
            let receiver = normalized_tokens(expression.receiver.as_ref());
            Self::increment(
                &mut self.builder.session_factory_setter_calls,
                self.module,
                &format!("{receiver}.{method}"),
                expression.args.len(),
                cfg,
                source_class,
            );
        }
        syn::visit::visit_expr_method_call(self, expression);
    }
}

fn collect_expression_surfaces(
    item: &Item,
    module: &str,
    cfg: &[String],
    availability: Availability,
    inside_session_factory: bool,
    builder: &mut BaselineBuilder,
) {
    let mut collector = ExpressionSurfaceCollector {
        module,
        cfg,
        availability,
        inside_session_factory,
        builder,
    };
    collector.visit_item(item);
}

fn collect_items(
    role: PackageRole,
    items: &[Item],
    module: &str,
    cfg: &[String],
    availability: Availability,
    builder: &mut BaselineBuilder,
) {
    for item in items {
        let attributes: &[syn::Attribute] = match item {
            Item::Const(item) => &item.attrs,
            Item::Enum(item) => &item.attrs,
            Item::ExternCrate(item) => &item.attrs,
            Item::Fn(item) => &item.attrs,
            Item::ForeignMod(item) => &item.attrs,
            Item::Impl(item) => &item.attrs,
            Item::Macro(item) => &item.attrs,
            Item::Mod(item) => &item.attrs,
            Item::Static(item) => &item.attrs,
            Item::Struct(item) => &item.attrs,
            Item::Trait(item) => &item.attrs,
            Item::TraitAlias(item) => &item.attrs,
            Item::Type(item) => &item.attrs,
            Item::Union(item) => &item.attrs,
            Item::Use(item) => &item.attrs,
            Item::Verbatim(_) => &[],
            _ => &[],
        };
        let (item_cfg, item_availability) = item_context(
            cfg,
            availability,
            attributes,
            &format!("item in {module}"),
            &mut builder.errors,
        );
        let Some(item_source_class) = item_availability.source_class() else {
            continue;
        };

        if let Item::Mod(item_mod) = item {
            if let Some((_, inline_items)) = &item_mod.content {
                collect_items(
                    role,
                    inline_items,
                    &format!("{module}::{}", item_mod.ident),
                    &item_cfg,
                    item_availability,
                    builder,
                );
            }
            continue;
        }

        if role == PackageRole::Server
            && let Item::Fn(function) = item
        {
            let name = function.sig.ident.to_string();
            builder
                .server_function_bodies
                .entry((module.to_owned(), name.clone()))
                .or_default()
                .insert(SessionFactoryHelperSurface {
                    module: module.to_owned(),
                    name,
                    signature: normalized_tokens(&function.sig),
                    body_fingerprint: compact_token_fingerprint(&function.block),
                    cfg: item_cfg.clone(),
                    source_class: item_source_class.to_owned(),
                });
        }

        if let Item::Use(item_use) = item
            && use_tree_renames_ident(&item_use.tree, WORLD_SESSION_NAME)
        {
            builder.errors.push(format!(
                "{module} renames {WORLD_SESSION_NAME}; aliases can bypass exact impl ownership"
            ));
        }
        if let Item::Type(item_type) = item
            && type_path_ends_with(&item_type.ty, WORLD_SESSION_NAME)
            && item_type.ident != WORLD_SESSION_NAME
        {
            builder.errors.push(format!(
                "{module} aliases {WORLD_SESSION_NAME} as {}; type aliases are outside the exact \
                 impl ownership grammar",
                item_type.ident
            ));
        }
        if let Item::Macro(item_macro) = item
            && let Some(target) = token_stream_mentions_ownership_target(&item_macro.mac.tokens)
        {
            builder.errors.push(format!(
                "{module} macro {}! mentions {target}; macro-generated ownership surfaces are \
                 not allowed",
                normalized_tokens(&item_macro.mac.path),
            ));
        }

        if role == PackageRole::Network {
            collect_network_type(item, module, &item_cfg, item_availability, builder);
        }

        match (role, module, item) {
            (PackageRole::World, WORLD_SESSION_MODULE, Item::Struct(item_struct)) => {
                collect_struct(
                    item_struct,
                    module,
                    cfg,
                    availability,
                    WORLD_SESSION_NAME,
                    &mut builder.world_session_definition,
                    &mut builder.world_session_fields,
                    &mut builder.generated_surface_inputs,
                    &mut builder.errors,
                );
            }
            (PackageRole::Server, SESSION_RESOURCES_MODULE, Item::Struct(item_struct)) => {
                collect_struct(
                    item_struct,
                    module,
                    cfg,
                    availability,
                    SESSION_RESOURCES_NAME,
                    &mut builder.session_resources_definition,
                    &mut builder.session_resources_fields,
                    &mut builder.generated_surface_inputs,
                    &mut builder.errors,
                );
            }
            (PackageRole::World, _, Item::Impl(item_impl)) => {
                collect_world_session_impl(item_impl, module, cfg, availability, builder)
            }
            (PackageRole::Server, SESSION_FACTORY_MODULE, Item::Fn(function))
                if function.sig.ident == SESSION_FACTORY_NAME =>
            {
                let surface = definition_surface(
                    module,
                    SESSION_FACTORY_NAME,
                    &function.vis,
                    item_cfg.clone(),
                    item_source_class,
                );
                set_once(
                    &mut builder.session_factory_definition,
                    surface,
                    SESSION_FACTORY_NAME,
                    &mut builder.errors,
                );
                set_once(
                    &mut builder.session_factory_signature,
                    normalized_tokens(&function.sig),
                    "create_session signature",
                    &mut builder.errors,
                );
                set_once(
                    &mut builder.session_factory_body_fingerprint,
                    compact_token_fingerprint(&function.block),
                    "create_session body fingerprint",
                    &mut builder.errors,
                );
                builder
                    .generated_surface_inputs
                    .extend(generated_attribute_inputs(
                        module,
                        SESSION_FACTORY_NAME,
                        &function.attrs,
                        &item_cfg,
                        item_availability,
                    ));
            }
            _ => {}
        }

        let inside_session_factory = matches!(
            item,
            Item::Fn(function)
                if role == PackageRole::Server
                    && module == SESSION_FACTORY_MODULE
                    && function.sig.ident == SESSION_FACTORY_NAME
        );
        collect_expression_surfaces(
            item,
            module,
            &item_cfg,
            item_availability,
            inside_session_factory,
            builder,
        );
    }
}

fn call_surfaces(
    values: BTreeMap<(String, String, usize, Vec<String>, String), usize>,
) -> Vec<CallSurface> {
    values
        .into_iter()
        .map(
            |((module, callee, argument_count, cfg, source_class), count)| CallSurface {
                module,
                callee,
                argument_count,
                cfg,
                source_class,
                count,
            },
        )
        .collect()
}

fn unique_network_type<'a>(
    types: &'a BTreeMap<String, Vec<NetworkTypeDefinition>>,
    name: &str,
) -> Result<&'a NetworkTypeDefinition, String> {
    let definitions = types
        .get(name)
        .ok_or_else(|| format!("missing wow-network type {name}"))?;
    let [definition] = definitions.as_slice() else {
        let modules: Vec<_> = definitions
            .iter()
            .map(|definition| definition.surface.definition.module.as_str())
            .collect();
        return Err(format!(
            "wow-network type {name} is ambiguous across logical modules {modules:?}"
        ));
    };
    Ok(definition)
}

fn network_contract(
    types: &BTreeMap<String, Vec<NetworkTypeDefinition>>,
) -> Result<
    (
        TypeSurface,
        Vec<TypeSurface>,
        TypeSurface,
        BTreeSet<GeneratedSurfaceInput>,
    ),
    String,
> {
    let session_command = unique_network_type(types, SESSION_COMMAND_NAME)?;
    if session_command.surface.kind != "enum" {
        return Err(format!("{SESSION_COMMAND_NAME} must remain an enum"));
    }
    let player_broadcast_info = unique_network_type(types, PLAYER_BROADCAST_INFO_NAME)?;
    if player_broadcast_info.surface.kind != "struct" {
        return Err(format!("{PLAYER_BROADCAST_INFO_NAME} must remain a struct"));
    }

    let mut pending: Vec<_> = session_command.referenced_types.iter().cloned().collect();
    let mut visited = BTreeSet::from([SESSION_COMMAND_NAME.to_owned()]);
    let mut payloads = Vec::new();
    let mut generated_surface_inputs = session_command.generated_surface_inputs.clone();
    generated_surface_inputs.extend(
        player_broadcast_info
            .generated_surface_inputs
            .iter()
            .cloned(),
    );
    while let Some(name) = pending.pop() {
        if !visited.insert(name.clone()) {
            continue;
        }
        let Some(definitions) = types.get(&name) else {
            // Primitives and types imported from other packages are pinned by
            // the exact payload type expression but are outside this package's
            // local ownership closure.
            continue;
        };
        let [definition] = definitions.as_slice() else {
            let modules: Vec<_> = definitions
                .iter()
                .map(|definition| definition.surface.definition.module.as_str())
                .collect();
            return Err(format!(
                "transitive {SESSION_COMMAND_NAME} payload {name} is ambiguous across {modules:?}"
            ));
        };
        payloads.push(definition.surface.clone());
        generated_surface_inputs.extend(definition.generated_surface_inputs.iter().cloned());
        pending.extend(definition.referenced_types.iter().cloned());
    }
    payloads.sort();
    Ok((
        session_command.surface.clone(),
        payloads,
        player_broadcast_info.surface.clone(),
        generated_surface_inputs,
    ))
}

fn session_helper_candidate_keys(
    caller_module: &str,
    path: &[String],
) -> BTreeSet<(String, String)> {
    let Some(name) = path.last().cloned() else {
        return BTreeSet::new();
    };
    let mut candidates = BTreeSet::new();
    if path.len() == 1 {
        candidates.insert((caller_module.to_owned(), name));
        return candidates;
    }

    let qualifiers = &path[..path.len() - 1];
    match qualifiers.first().map(String::as_str) {
        Some("crate") => {
            candidates.insert((qualifiers.join("::"), name));
        }
        Some("self") => {
            let suffix = &qualifiers[1..];
            let module = if suffix.is_empty() {
                caller_module.to_owned()
            } else {
                format!("{caller_module}::{}", suffix.join("::"))
            };
            candidates.insert((module, name));
        }
        Some("super") => {
            let mut module: Vec<_> = caller_module.split("::").collect();
            let mut index = 0;
            while qualifiers
                .get(index)
                .is_some_and(|segment| segment == "super")
            {
                if module.len() > 1 {
                    module.pop();
                }
                index += 1;
            }
            module.extend(qualifiers[index..].iter().map(String::as_str));
            candidates.insert((module.join("::"), name));
        }
        Some(_) | None => {
            candidates.insert((
                format!("{caller_module}::{}", qualifiers.join("::")),
                name.clone(),
            ));
            candidates.insert((format!("crate::{}", qualifiers.join("::")), name));
        }
    }
    candidates
}

impl BaselineBuilder {
    fn session_helper_bodies(&self) -> Vec<SessionFactoryHelperSurface> {
        let mut helpers = BTreeSet::new();
        for (caller_module, path) in &self.session_factory_helper_calls {
            for key in session_helper_candidate_keys(caller_module, path) {
                if key
                    == (
                        SESSION_FACTORY_MODULE.to_owned(),
                        SESSION_FACTORY_NAME.to_owned(),
                    )
                {
                    continue;
                }
                if let Some(surfaces) = self.server_function_bodies.get(&key) {
                    helpers.extend(surfaces.iter().cloned());
                }
            }
        }
        helpers.into_iter().collect()
    }

    fn finish(
        self,
        registry_accesses: RegistryAccessBaseline,
        persistence_accesses: PersistenceAccessBaseline,
        bridge_accesses: BridgeAccessBaseline,
    ) -> Result<SessionSyntaxBaseline, String> {
        let session_helper_bodies = self.session_helper_bodies();
        let network_contract = network_contract(&self.network_types);
        let mut errors = self.errors;
        let world_session_definition = self.world_session_definition.ok_or_else(|| {
            format!("missing {WORLD_SESSION_MODULE}::{WORLD_SESSION_NAME} definition")
        });
        let session_resources_definition = self.session_resources_definition.ok_or_else(|| {
            format!("missing {SESSION_RESOURCES_MODULE}::{SESSION_RESOURCES_NAME} definition")
        });
        let session_factory_definition = self.session_factory_definition.ok_or_else(|| {
            format!("missing {SESSION_FACTORY_MODULE}::{SESSION_FACTORY_NAME} definition")
        });
        let session_factory_signature = self
            .session_factory_signature
            .ok_or_else(|| format!("missing {SESSION_FACTORY_NAME} signature"));
        let session_factory_body_fingerprint = self
            .session_factory_body_fingerprint
            .clone()
            .ok_or_else(|| format!("missing {SESSION_FACTORY_NAME} body fingerprint"));
        for result in [
            world_session_definition.as_ref().map(|_| ()),
            session_resources_definition.as_ref().map(|_| ()),
            session_factory_definition.as_ref().map(|_| ()),
            session_factory_signature.as_ref().map(|_| ()),
            session_factory_body_fingerprint.as_ref().map(|_| ()),
        ] {
            if let Err(error) = result {
                errors.push(error.clone());
            }
        }
        if let Err(error) = &network_contract {
            errors.push(error.clone());
        }
        if !errors.is_empty() {
            return Err(errors.join("\n"));
        }
        let (
            session_command,
            session_command_payload_types,
            player_broadcast_info,
            network_generated_surface_inputs,
        ) = network_contract.expect("validated network contract");
        let mut generated_surface_inputs = self.generated_surface_inputs;
        generated_surface_inputs.extend(network_generated_surface_inputs);

        Ok(SessionSyntaxBaseline {
            world_session: WorldSessionSurface {
                definition: world_session_definition.expect("validated definition"),
                fields: self.world_session_fields.into_iter().collect(),
                impls: self
                    .world_session_impls
                    .into_iter()
                    .map(|(module, trait_path, cfg, source_class)| ImplSurface {
                        module,
                        trait_path,
                        cfg,
                        source_class,
                    })
                    .collect(),
                impl_items: self.world_session_impl_items.into_iter().collect(),
            },
            session_resources: SessionResourcesSurface {
                definition: session_resources_definition.expect("validated definition"),
                fields: self.session_resources_fields.into_iter().collect(),
                construction_sites: call_surfaces(self.session_resources_constructions),
            },
            session_factory: SessionFactorySurface {
                definition: session_factory_definition.expect("validated definition"),
                signature: session_factory_signature.expect("validated signature"),
                body_fingerprint: session_factory_body_fingerprint
                    .expect("validated body fingerprint"),
                session_helper_bodies,
                call_sites: call_surfaces(self.session_factory_calls),
                world_session_new_sites: call_surfaces(self.world_session_new_calls),
                setter_call_sites: call_surfaces(self.session_factory_setter_calls),
            },
            session_command,
            session_command_payload_types,
            player_broadcast_info,
            generated_surface_inputs: generated_surface_inputs.into_iter().collect(),
            registry_accesses,
            persistence_accesses,
            bridge_accesses,
        })
    }
}

fn collect_units(
    units: Vec<SourceUnit>,
    persistence_accesses: PersistenceAccessBaseline,
) -> Result<SessionSyntaxBaseline, String> {
    let registry_sources: Vec<_> = units
        .iter()
        .filter(|unit| unit.availability.production)
        .map(|unit| ProductionRegistrySource {
            package: unit.role.package_name(),
            module: &unit.logical_module_path,
            source_path: &unit.repository_relative_path,
            inherited_cfg: &unit.cfg,
            source: &unit.source,
        })
        .collect();
    let registry_accesses = inventory_registry_accesses(&registry_sources)
        .map_err(|error| format!("cannot inventory direct registry accesses:\n{error}"))?;
    let bridge_sources: Vec<_> = units
        .iter()
        .filter(|unit| {
            unit.availability.production
                && matches!(unit.role, PackageRole::World | PackageRole::Server)
        })
        .map(|unit| BridgeSource {
            package: unit.role.package_name(),
            module: &unit.logical_module_path,
            source_path: &unit.repository_relative_path,
            inherited_cfg: &unit.cfg,
            source: &unit.source,
        })
        .collect();
    let bridge_accesses = inventory_bridge_accesses(&bridge_sources)
        .map_err(|error| format!("cannot inventory legacy/canonical bridges:\n{error}"))?;
    let mut builder = BaselineBuilder::default();
    for unit in units {
        if unit.availability.source_class().is_none() {
            continue;
        }
        let syntax = syn::parse_file(&unit.source)
            .map_err(|error| format!("cannot parse {}: {error}", unit.source_path.display()))?;
        let mut include_guard = IncludeMacroGuard::default();
        include_guard.visit_file(&syntax);
        if include_guard.count > 0 {
            builder.errors.push(format!(
                "{} contains {} include! macro invocation(s); generated source inputs are outside \
                 the closed ownership grammar",
                unit.source_path.display(),
                include_guard.count,
            ));
        }
        collect_items(
            unit.role,
            &syntax.items,
            &unit.logical_module_path,
            &unit.cfg,
            unit.availability,
            &mut builder,
        );
    }
    builder.finish(registry_accesses, persistence_accesses, bridge_accesses)
}

fn repository_relative_path(repository_root: &Path, source_path: &Path) -> Result<String, String> {
    let relative = source_path.strip_prefix(repository_root).map_err(|_| {
        format!(
            "audited source {} is outside repository root {}",
            source_path.display(),
            repository_root.display()
        )
    })?;
    let mut parts = Vec::new();
    for component in relative.components() {
        let part = component.as_os_str().to_str().ok_or_else(|| {
            format!(
                "audited source path is not valid UTF-8: {}",
                source_path.display()
            )
        })?;
        parts.push(part);
    }
    Ok(parts.join("/"))
}

fn repository_units(
    repository_root: &Path,
    role: PackageRole,
    package_root: &str,
    crate_root: &str,
) -> Result<Vec<SourceUnit>, String> {
    let package_root = repository_root.join(package_root);
    let crate_root = repository_root.join(crate_root);
    let (mounts, _) = audit_package_source_mounts(&package_root, &[crate_root])?;
    let mut units = Vec::new();
    for (source_path, contexts) in mounts {
        let source = fs::read_to_string(&source_path)
            .map_err(|error| format!("cannot read {}: {error}", source_path.display()))?;
        for SourceMountContext {
            logical_module_path,
            cfg,
            production_possible,
            test_possible,
        } in contexts
        {
            let repository_relative_path = repository_relative_path(repository_root, &source_path)?;
            units.push(SourceUnit {
                role,
                source_path: source_path.clone(),
                repository_relative_path,
                logical_module_path,
                cfg,
                availability: Availability {
                    production: production_possible,
                    test: test_possible,
                },
                source: source.clone(),
            });
        }
    }
    Ok(units)
}

fn persistence_classification(package: &str) -> &'static str {
    match package {
        "wow-database" => "reviewed_adapter",
        "world-server" | "bnet-server" => "composition",
        _ => "direct_application_or_domain_access",
    }
}

fn collect_workspace_persistence_baseline(
    repository_root: &Path,
) -> Result<PersistenceAccessBaseline, String> {
    let mounts = workspace_source_mounts(repository_root)?;
    let dependencies = workspace_dependency_aliases(repository_root)?;
    let relative_paths = mounts
        .iter()
        .map(|mount| repository_relative_path(repository_root, &mount.source_path))
        .collect::<Result<Vec<_>, _>>()?;
    let mut sources = Vec::new();
    for (mount, relative_path) in mounts.iter().zip(relative_paths.iter()) {
        for context in &mount.contexts {
            if !context.production_possible && !context.test_possible {
                continue;
            }
            sources.push(ClassifiedPersistenceSource {
                classification: persistence_classification(&mount.package),
                package: &mount.package,
                module: &context.logical_module_path,
                source_path: relative_path,
                inherited_cfg: &context.cfg,
                source: &mount.source,
            });
        }
    }
    inventory_persistence_accesses_with_dependencies(&sources, &dependencies)
        .map_err(|error| format!("cannot inventory persistence accesses:\n{error}"))
}

fn collect_repository_baseline(repository_root: &Path) -> Result<SessionSyntaxBaseline, String> {
    collect_repository_baseline_with_persistence(repository_root, true)
}

/// Collect the session syntax surface, optionally without the persistence scan.
///
/// The persistence inventory costs a full workspace scan — minutes, and the
/// dominant cost of the whole gate. `print-baseline` renders an envelope whose
/// persistence field is `#[serde(skip)]`, so paying for that scan there bought
/// a value that was then discarded unread.
fn collect_repository_baseline_with_persistence(
    repository_root: &Path,
    with_persistence: bool,
) -> Result<SessionSyntaxBaseline, String> {
    let mut units = repository_units(
        repository_root,
        PackageRole::World,
        WORLD_PACKAGE_ROOT,
        WORLD_CRATE_ROOT,
    )?;
    units.extend(repository_units(
        repository_root,
        PackageRole::Server,
        SERVER_PACKAGE_ROOT,
        SERVER_CRATE_ROOT,
    )?);
    units.extend(repository_units(
        repository_root,
        PackageRole::Network,
        NETWORK_PACKAGE_ROOT,
        NETWORK_CRATE_ROOT,
    )?);
    let persistence_accesses = if with_persistence {
        collect_workspace_persistence_baseline(repository_root)?
    } else {
        PersistenceAccessBaseline::default()
    };
    let baseline = collect_units(units, persistence_accesses)?;
    validate_curated_bridge_anchors(&baseline.bridge_accesses)
        .map_err(|error| format!("invalid curated bridge inventory:\n{error}"))?;
    Ok(baseline)
}

fn set_drift<T>(label: &str, expected: &[T], actual: &[T], errors: &mut Vec<String>)
where
    T: Clone + Ord + Serialize,
{
    let expected: BTreeSet<_> = expected.iter().cloned().collect();
    let actual: BTreeSet<_> = actual.iter().cloned().collect();
    for removed in expected.difference(&actual) {
        errors.push(format!(
            "obsolete {label} baseline entry: {}",
            serde_json::to_string(removed).expect("surface serializes")
        ));
    }
    for added in actual.difference(&expected) {
        errors.push(format!(
            "unreviewed {label} surface: {}",
            serde_json::to_string(added).expect("surface serializes")
        ));
    }
}

fn compare_baseline(
    expected: &SessionSyntaxBaseline,
    actual: &SessionSyntaxBaseline,
) -> Result<(), String> {
    let mut errors = Vec::new();
    if expected.world_session.definition != actual.world_session.definition {
        errors.push(format!(
            "WorldSession definition changed: expected {:?}, actual {:?}",
            expected.world_session.definition, actual.world_session.definition
        ));
    }
    set_drift(
        "WorldSession field",
        &expected.world_session.fields,
        &actual.world_session.fields,
        &mut errors,
    );
    set_drift(
        "WorldSession impl",
        &expected.world_session.impls,
        &actual.world_session.impls,
        &mut errors,
    );
    set_drift(
        "WorldSession impl item",
        &expected.world_session.impl_items,
        &actual.world_session.impl_items,
        &mut errors,
    );
    if expected.session_resources.definition != actual.session_resources.definition {
        errors.push(format!(
            "SessionResources definition changed: expected {:?}, actual {:?}",
            expected.session_resources.definition, actual.session_resources.definition
        ));
    }
    set_drift(
        "SessionResources field",
        &expected.session_resources.fields,
        &actual.session_resources.fields,
        &mut errors,
    );
    set_drift(
        "SessionResources construction site",
        &expected.session_resources.construction_sites,
        &actual.session_resources.construction_sites,
        &mut errors,
    );
    if expected.session_factory.definition != actual.session_factory.definition {
        errors.push(format!(
            "create_session definition changed: expected {:?}, actual {:?}",
            expected.session_factory.definition, actual.session_factory.definition
        ));
    }
    if expected.session_factory.signature != actual.session_factory.signature {
        errors.push(format!(
            "create_session signature changed: expected {:?}, actual {:?}",
            expected.session_factory.signature, actual.session_factory.signature
        ));
    }
    if expected.session_factory.body_fingerprint != actual.session_factory.body_fingerprint {
        errors.push(format!(
            "create_session body fingerprint changed: expected {:?}, actual {:?}",
            expected.session_factory.body_fingerprint, actual.session_factory.body_fingerprint
        ));
    }
    set_drift(
        "create_session session-bearing helper body",
        &expected.session_factory.session_helper_bodies,
        &actual.session_factory.session_helper_bodies,
        &mut errors,
    );
    set_drift(
        "create_session call site",
        &expected.session_factory.call_sites,
        &actual.session_factory.call_sites,
        &mut errors,
    );
    set_drift(
        "WorldSession::new call site",
        &expected.session_factory.world_session_new_sites,
        &actual.session_factory.world_session_new_sites,
        &mut errors,
    );
    set_drift(
        "create_session setter call",
        &expected.session_factory.setter_call_sites,
        &actual.session_factory.setter_call_sites,
        &mut errors,
    );
    if expected.session_command != actual.session_command {
        errors.push(format!(
            "SessionCommand variants changed: expected {:?}, actual {:?}",
            expected.session_command, actual.session_command
        ));
    }
    set_drift(
        "SessionCommand transitive payload type",
        &expected.session_command_payload_types,
        &actual.session_command_payload_types,
        &mut errors,
    );
    if expected.player_broadcast_info != actual.player_broadcast_info {
        errors.push(format!(
            "PlayerBroadcastInfo surface changed: expected {:?}, actual {:?}",
            expected.player_broadcast_info, actual.player_broadcast_info
        ));
    }
    set_drift(
        "generated ownership input",
        &expected.generated_surface_inputs,
        &actual.generated_surface_inputs,
        &mut errors,
    );
    if let Err(error) =
        compare_registry_access_baseline(&expected.registry_accesses, &actual.registry_accesses)
    {
        errors.push(error);
    }
    if let Err(error) =
        compare_bridge_access_baseline(&expected.bridge_accesses, &actual.bridge_accesses)
    {
        errors.push(error);
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("\n"))
    }
}

fn load_policy(path: &Path) -> Result<PolicyEnvelope, String> {
    let source = fs::read_to_string(path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let policy: PolicyEnvelope = serde_json::from_str(&source).map_err(|error| {
        format!(
            "invalid session ownership policy {}: {error}",
            path.display()
        )
    })?;
    if policy.schema_version != 1 {
        return Err(format!(
            "session ownership policy schema_version must be 1, got {}",
            policy.schema_version
        ));
    }
    Ok(policy)
}

/// Check the repository against the exact AST surface checked into the policy.
pub fn check_repository(policy_path: Option<&Path>) -> Result<String, String> {
    check_repository_scoped(policy_path, true)
}

/// Check the session syntax surface without the exact persistence comparison.
///
/// The persistence scan is minutes; the session surface is seconds. Pull
/// requests run this so the ownership ratchet still guards every change,
/// while the exact inventory is compared on push and on the weekly cron —
/// detected at merge rather than at review time, not detected less.
pub fn check_repository_syntax_only(policy_path: Option<&Path>) -> Result<String, String> {
    check_repository_scoped(policy_path, false)
}

fn check_repository_scoped(
    policy_path: Option<&Path>,
    with_persistence: bool,
) -> Result<String, String> {
    let repository_root = crate::repository_root()?;
    let policy_path = policy_path
        .map(Path::to_owned)
        .unwrap_or_else(|| repository_root.join(POLICY_RELATIVE_PATH));
    let policy = load_policy(&policy_path)?;
    let actual = collect_repository_baseline_with_persistence(&repository_root, with_persistence)?;
    compare_baseline(&policy.syntax_baseline, &actual)?;
    if !with_persistence {
        return Ok(format!(
            "session ownership: PASS (syntax only; {} production + {} test-fixture WorldSession \
             fields; {} impl owners / {} exact associated items; {} SessionResources fields; {} \
             SessionCommand variants; {} exact direct-registry rows; exact persistence inventory \
             deferred to push and cron)",
            actual
                .world_session
                .fields
                .iter()
                .filter(|field| field.cfg.is_empty())
                .count(),
            actual
                .world_session
                .fields
                .iter()
                .filter(|field| !field.cfg.is_empty())
                .count(),
            actual.world_session.impls.len(),
            actual.world_session.impl_items.len(),
            actual.session_resources.fields.len(),
            actual.session_command.variants.len(),
            actual.registry_accesses.accesses.len(),
        ));
    }
    let persistence_snapshot_path = repository_root.join(&policy.persistence_access_snapshot);
    let persistence_snapshot_source =
        fs::read_to_string(&persistence_snapshot_path).map_err(|error| {
            format!(
                "cannot read {}: {error}",
                persistence_snapshot_path.display()
            )
        })?;
    let expected_persistence: PersistenceAccessBaseline =
        serde_json::from_str(&persistence_snapshot_source).map_err(|error| {
            format!(
                "invalid persistence access snapshot {}: {error}",
                persistence_snapshot_path.display()
            )
        })?;
    compare_persistence_access_baseline(&expected_persistence, &actual.persistence_accesses)?;
    let persistence_policy_path = repository_root.join(PERSISTENCE_POLICY_RELATIVE_PATH);
    let persistence_annotations_path = repository_root.join(PERSISTENCE_ANNOTATIONS_RELATIVE_PATH);
    let issue_ledger_path = repository_root.join(ISSUE_LEDGER_RELATIVE_PATH);
    let (semantic_production_rows, semantic_test_rows, generated_persistence_rows, semantic_groups) =
        crate::persistence_policy::validate_persistence_policy(
            &persistence_policy_path,
            &persistence_annotations_path,
            &issue_ledger_path,
            &actual.persistence_accesses,
        )
        .map_err(|error| format!("invalid persistence semantic ownership:\n{error}"))?;
    let production_session_fields = actual
        .world_session
        .fields
        .iter()
        .filter(|field| field.source_class == "production")
        .count();
    let test_session_fields = actual
        .world_session
        .fields
        .iter()
        .filter(|field| field.source_class == "test_fixture")
        .count();
    let production_persistence_rows = actual
        .persistence_accesses
        .accesses
        .iter()
        .filter(|access| access.source_class == "production")
        .count();
    let test_persistence_rows = actual
        .persistence_accesses
        .accesses
        .iter()
        .filter(|access| access.source_class == "test_fixture")
        .count();
    debug_assert_eq!(production_persistence_rows, semantic_production_rows);
    debug_assert_eq!(test_persistence_rows, semantic_test_rows);
    Ok(format!(
        "session ownership: PASS ({production_session_fields} production + {test_session_fields} \
         test-fixture WorldSession fields; {} impl owners / {} exact associated \
         items; {} SessionResources fields; {} factory setter/install calls; {} SessionCommand \
         variants / {} transitive payload types; {} PlayerBroadcastInfo fields; {} exact generated \
         inputs; {} exact direct-registry rows; {production_persistence_rows} production + \
         {test_persistence_rows} test-fixture persistence rows \
         ({generated_persistence_rows} generated-input rows, subset; {semantic_groups} exact semantic groups); {} exact bridge rows; \
         include/target-macro surfaces fail closed)",
        actual.world_session.impls.len(),
        actual.world_session.impl_items.len(),
        actual.session_resources.fields.len(),
        actual.session_factory.setter_call_sites.len(),
        actual.session_command.variants.len(),
        actual.session_command_payload_types.len(),
        actual.player_broadcast_info.fields.len(),
        actual.generated_surface_inputs.len(),
        actual.registry_accesses.accesses.len(),
        actual.bridge_accesses.bridges.len(),
    ))
}

/// Render the current syntax surface as a minimal schema-v1 policy envelope.
///
/// The command only returns text; callers must review and merge the
/// `syntax_baseline` object deliberately into the semantic policy.
pub fn print_repository_baseline() -> Result<String, String> {
    let repository_root = crate::repository_root()?;
    // Without persistence: the envelope skips that field when serializing, so
    // scanning for it would discard the result.
    let baseline = collect_repository_baseline_with_persistence(&repository_root, false)?;
    serde_json::to_string_pretty(&BaselineEnvelope {
        schema_version: 1,
        persistence_access_snapshot: PERSISTENCE_ACCESS_SNAPSHOT_RELATIVE_PATH,
        syntax_baseline: &baseline,
    })
    .map_err(|error| format!("cannot serialize session ownership baseline: {error}"))
}

/// Render the dedicated exact persistence snapshot without editing it.
pub fn print_repository_persistence_baseline() -> Result<String, String> {
    let repository_root = crate::repository_root()?;
    let baseline = collect_workspace_persistence_baseline(&repository_root)?;
    render_persistence_access_baseline(&baseline)
}

/// Render the canonical semantic policy from an already computed snapshot.
///
/// The policy is a pure function of the reviewed annotations and the exact
/// inventory, but recomputing that inventory costs a full workspace scan. CI
/// publishes the recomputed snapshot as an artifact when the ratchet moves;
/// deriving the policy from that same artifact keeps both checked-in files
/// consistent without paying for a second scan, and without letting a
/// separately scanned policy disagree with the snapshot beside it.
pub fn print_repository_persistence_policy_from_snapshot(
    snapshot_path: &Path,
) -> Result<String, String> {
    let repository_root = crate::repository_root()?;
    let source = fs::read_to_string(snapshot_path)
        .map_err(|error| format!("cannot read {}: {error}", snapshot_path.display()))?;
    let baseline: PersistenceAccessBaseline = serde_json::from_str(&source).map_err(|error| {
        format!(
            "invalid persistence access snapshot {}: {error}",
            snapshot_path.display()
        )
    })?;
    crate::persistence_policy::render_persistence_policy(
        &repository_root.join(PERSISTENCE_ANNOTATIONS_RELATIVE_PATH),
        &baseline,
    )
}

/// Render the canonical semantic policy derived from reviewed workflow annotations.
pub fn print_repository_persistence_policy() -> Result<String, String> {
    let repository_root = crate::repository_root()?;
    let baseline = collect_workspace_persistence_baseline(&repository_root)?;
    crate::persistence_policy::render_persistence_policy(
        &repository_root.join(PERSISTENCE_ANNOTATIONS_RELATIVE_PATH),
        &baseline,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // The checker regenerates the semantic policy from the annotations and the
    // freshly scanned inventory and demands exact equality. Both checked-in
    // files must therefore already agree with each other: a snapshot updated
    // without its policy fails CI only after a full scan, which is the one
    // feedback loop this tool cannot afford to leave to CI.
    // A missing or non-JSON snapshot must name the offending path instead of
    // rendering a policy from nothing: this command exists to install a CI
    // artifact, so the plausible mistake is pointing it at the wrong download.
    #[test]
    fn rendering_a_policy_rejects_an_unreadable_or_invalid_snapshot() {
        let repository_root = crate::repository_root().expect("repository root");

        let missing = repository_root.join("tools/architecture/no-such-snapshot.json");
        let error = print_repository_persistence_policy_from_snapshot(&missing)
            .expect_err("a missing snapshot cannot render a policy");
        assert!(
            error.starts_with("cannot read") && error.contains("no-such-snapshot.json"),
            "unexpected error: {error}"
        );

        let not_json = repository_root.join("AGENTS.md");
        let error = print_repository_persistence_policy_from_snapshot(&not_json)
            .expect_err("a non-JSON snapshot cannot render a policy");
        assert!(
            error.starts_with("invalid persistence access snapshot") && error.contains("AGENTS.md"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn checked_persistence_policy_matches_the_checked_snapshot() {
        let repository_root = crate::repository_root().expect("repository root");
        let rendered = print_repository_persistence_policy_from_snapshot(
            &repository_root.join(PERSISTENCE_ACCESS_SNAPSHOT_RELATIVE_PATH),
        )
        .expect("render policy from the checked snapshot");
        let rendered: serde_json::Value =
            serde_json::from_str(&rendered).expect("rendered policy is JSON");
        let checked = fs::read_to_string(repository_root.join(PERSISTENCE_POLICY_RELATIVE_PATH))
            .expect("read checked policy");
        let checked: serde_json::Value =
            serde_json::from_str(&checked).expect("checked policy is JSON");
        assert_eq!(
            checked, rendered,
            "checked persistence policy disagrees with the checked snapshot; \
             regenerate it with print-persistence-policy --from-snapshot"
        );
    }

    fn unit(role: PackageRole, source_path: &str, source: &str) -> SourceUnit {
        SourceUnit {
            role,
            source_path: PathBuf::from(source_path),
            repository_relative_path: source_path.to_owned(),
            logical_module_path: "crate".to_owned(),
            cfg: Vec::new(),
            availability: Availability {
                production: true,
                test: true,
            },
            source: source.to_owned(),
        }
    }

    fn synthetic_baseline_with_network(
        world: &str,
        server: &str,
        network: &str,
    ) -> Result<SessionSyntaxBaseline, String> {
        collect_units(
            vec![
                unit(PackageRole::World, "wow-world/src/lib.rs", world),
                unit(PackageRole::Server, "world-server/src/main.rs", server),
                unit(PackageRole::Network, "wow-network/src/lib.rs", network),
            ],
            PersistenceAccessBaseline {
                schema_version: 3,
                accesses: Vec::new(),
            },
        )
    }

    fn synthetic_baseline(world: &str, server: &str) -> Result<SessionSyntaxBaseline, String> {
        synthetic_baseline_with_network(
            world,
            server,
            r#"
                pub enum SessionCommand { Kick(KickCommand) }
                pub struct KickCommand { pub reason: String }
                pub struct PlayerBroadcastInfo { pub map_id: u16 }
            "#,
        )
    }

    fn world_source(field: &str, extra_impl_item: &str) -> String {
        format!(
            r#"
                pub mod session {{
                    pub struct WorldSession {{
                        pub account_id: u32,
                        {field}
                    }}
                    impl WorldSession {{
                        pub fn new(account_id: u32) -> Self {{ todo!() }}
                        pub fn set_account_id(&mut self, account_id: u32) {{}}
                        fn internal(&self) {{}}
                    }}
                }}
                mod handlers {{
                    impl crate::session::WorldSession {{
                        fn handle_packet(&mut self) {{}}
                        {extra_impl_item}
                    }}
                }}
            "#
        )
    }

    fn server_source(extra_field: &str, extra_factory: &str) -> String {
        format!(
            r#"
                mod session_resources {{
                    pub(super) struct SessionResources {{
                        pub(super) char_db: Option<u32>,
                        {extra_field}
                    }}
                }}
                use session_resources::SessionResources;
                struct WorldSession;
                impl WorldSession {{
                    fn new(_account_id: u32) -> Self {{ Self }}
                    fn set_char_db(&mut self, _db: u32) {{}}
                }}
                async fn create_session(account_id: u32, resources: SessionResources) {{
                    let mut session = WorldSession::new(account_id);
                    if let Some(db) = resources.char_db {{ session.set_char_db(db); }}
                    {extra_factory}
                }}
                async fn bootstrap() {{
                    let resources = SessionResources {{ char_db: None, {extra_field_init} }};
                    create_session(1, resources).await;
                }}
            "#,
            extra_field_init = if extra_field.is_empty() {
                ""
            } else {
                "extra: 0,"
            },
        )
    }

    #[test]
    fn exact_field_sets_reject_growth_and_same_count_substitution() {
        let baseline = synthetic_baseline(&world_source("state: u8,", ""), &server_source("", ""))
            .expect("baseline parses");
        let growth = synthetic_baseline(
            &world_source("state: u8, added: bool,", ""),
            &server_source("", ""),
        )
        .expect("growth parses");
        let error = compare_baseline(&baseline, &growth).expect_err("field growth must fail");
        assert!(
            error.contains("unreviewed WorldSession field surface"),
            "{error}"
        );

        let substitution = synthetic_baseline(
            &world_source("replacement: u8,", ""),
            &server_source("", ""),
        )
        .expect("substitution parses");
        let error =
            compare_baseline(&baseline, &substitution).expect_err("same-count swap must fail");
        assert!(
            error.contains("obsolete WorldSession field baseline"),
            "{error}"
        );
        assert!(
            error.contains("unreviewed WorldSession field surface"),
            "{error}"
        );
    }

    #[test]
    fn external_impl_visible_and_setter_surfaces_are_exact() {
        let baseline = synthetic_baseline(&world_source("state: u8,", ""), &server_source("", ""))
            .expect("baseline parses");
        let changed = synthetic_baseline(
            &world_source(
                "state: u8,",
                "pub(crate) fn set_external_state(&mut self, _state: u8) {}",
            ),
            &server_source("", ""),
        )
        .expect("changed surface parses");
        let error = compare_baseline(&baseline, &changed).expect_err("new impl item must fail");
        assert!(
            error.contains("unreviewed WorldSession impl item surface"),
            "{error}"
        );
        assert!(error.contains("external_impl"), "{error}");
        assert!(error.contains("setter"), "{error}");
        assert!(error.contains("visible"), "{error}");

        let renamed_private = world_source("state: u8,", "")
            .replace("fn internal(&self) {}", "fn renamed_internal(&self) {}");
        let changed = synthetic_baseline(&renamed_private, &server_source("", ""))
            .expect("private item change parses");
        let error = compare_baseline(&baseline, &changed)
            .expect_err("private inherent items in crate::session are exact too");
        assert!(
            error.contains("obsolete WorldSession impl item baseline"),
            "{error}"
        );
        assert!(
            error.contains("unreviewed WorldSession impl item surface"),
            "{error}"
        );
    }

    #[test]
    fn cfg_test_items_are_classified_and_target_cfg_items_are_guarded() {
        let baseline = synthetic_baseline(&world_source("state: u8,", ""), &server_source("", ""))
            .expect("baseline parses");
        let test_only = synthetic_baseline(
            &world_source(
                "state: u8, #[cfg(test)] fixture_only: bool,",
                "#[cfg(test)] fn test_helper(&self) {}",
            ),
            &server_source("", ""),
        )
        .expect("test-only surface parses");
        let error = compare_baseline(&baseline, &test_only)
            .expect_err("test fixtures are an explicit baseline surface");
        assert!(
            error.contains(r#""source_class":"test_fixture""#),
            "{error}"
        );

        let target_cfg = synthetic_baseline(
            &world_source(
                "state: u8, #[cfg(windows)] platform_state: bool,",
                "#[cfg(windows)] fn platform_handler(&self) {}",
            ),
            &server_source("", ""),
        )
        .expect("target cfg surface parses");
        let error =
            compare_baseline(&baseline, &target_cfg).expect_err("target cfg can be production");
        assert!(error.contains("cfg (windows)"), "{error}");
    }

    fn attributes(source: &str) -> Vec<syn::Attribute> {
        let syntax = syn::parse_file(source).expect("cfg fixture parses as Rust");
        match &syntax.items[0] {
            Item::Struct(item) => item.attrs.clone(),
            _ => panic!("expected struct cfg fixture"),
        }
    }

    #[test]
    fn cfg_production_satisfiability_is_correlated_and_malformed_cfg_attr_fails() {
        let cfg_test = attributes("#[cfg(test)] struct Fixture;");
        assert!(!cfg_context_allows_production(&[], &cfg_test).unwrap());

        let any_test_feature =
            attributes(r#"#[cfg(any(test, feature = "fixture"))] struct Fixture;"#);
        assert!(cfg_context_allows_production(&[], &any_test_feature).unwrap());

        let not_test = attributes("#[cfg(not(test))] struct Fixture;");
        assert!(cfg_context_allows_production(&[], &not_test).unwrap());

        let contradictory = attributes("#[cfg(fixture)] #[cfg(not(fixture))] struct Fixture;");
        assert!(!cfg_context_allows_production(&[], &contradictory).unwrap());

        let all_contradictory = attributes("#[cfg(all(fixture, not(fixture)))] struct Fixture;");
        assert!(!cfg_context_allows_production(&[], &all_contradictory).unwrap());

        let parent = vec!["cfg (fixture)".to_owned()];
        let child = attributes("#[cfg(not(fixture))] struct Fixture;");
        assert!(!cfg_context_allows_production(&parent, &child).unwrap());

        let malformed_cfg_attr = attributes("#[cfg_attr(fixture)] struct Fixture;");
        let error = cfg_context_allows_production(&[], &malformed_cfg_attr)
            .expect_err("cfg_attr without an attribute must fail closed");
        assert!(
            error.contains("at least one conditional attribute"),
            "{error}"
        );

        let production_applies_test =
            attributes("#[cfg_attr(not(test), cfg(test))] struct Fixture;");
        assert!(!cfg_context_allows_production(&[], &production_applies_test).unwrap());
    }

    #[test]
    fn aliases_and_macros_cannot_hide_world_session_impls() {
        let alias_world = format!(
            "{}\nuse crate::session::WorldSession as HiddenSession;",
            world_source("state: u8,", "")
        );
        let error = synthetic_baseline(&alias_world, &server_source("", ""))
            .expect_err("renamed WorldSession must fail closed");
        assert!(error.contains("renames WorldSession"), "{error}");

        let macro_world = format!(
            "{}\nmake_impl!(WorldSession);",
            world_source("state: u8,", "")
        );
        let error = synthetic_baseline(&macro_world, &server_source("", ""))
            .expect_err("macro target must fail closed");
        assert!(
            error.contains("macro-generated ownership surfaces"),
            "{error}"
        );
    }

    #[test]
    fn include_macros_fail_closed_and_codegen_attributes_are_exact() {
        let include_world = format!(
            "{}\ninclude!(\"generated_session_surface.rs\");",
            world_source("state: u8,", "")
        );
        let error = synthetic_baseline(&include_world, &server_source("", ""))
            .expect_err("include inputs must fail closed");
        assert!(error.contains("include! macro invocation"), "{error}");

        let world = world_source("state: u8,", "");
        let server = server_source("", "");
        let baseline = synthetic_baseline_with_network(
            &world,
            &server,
            r#"
                pub enum SessionCommand { Kick(KickCommand) }
                pub struct KickCommand { pub reason: String }
                pub struct PlayerBroadcastInfo { pub map_id: u16 }
            "#,
        )
        .expect("baseline parses");
        let generated = synthetic_baseline_with_network(
            &world,
            &server,
            r#"
                #[derive(Clone)]
                pub enum SessionCommand { Kick(KickCommand) }
                pub struct KickCommand { pub reason: String }
                pub struct PlayerBroadcastInfo { pub map_id: u16 }
            "#,
        )
        .expect("generated input parses");
        assert_eq!(generated.generated_surface_inputs.len(), 1);
        let error = compare_baseline(&baseline, &generated)
            .expect_err("new derive input must be reviewed exactly");
        assert!(
            error.contains("unreviewed generated ownership input"),
            "{error}"
        );
        assert!(error.contains(r#""kind":"derive""#), "{error}");
    }

    #[test]
    fn splitting_an_impl_block_does_not_change_the_logical_owner_surface() {
        let baseline_world = r#"
            pub mod session {
                pub struct WorldSession { pub account_id: u32 }
                impl WorldSession {
                    fn first(&self) {}
                    fn second(&self) {}
                }
            }
        "#;
        let split_world = r#"
            pub mod session {
                pub struct WorldSession { pub account_id: u32 }
                impl WorldSession { fn first(&self) {} }
                impl WorldSession { fn second(&self) {} }
            }
        "#;
        let baseline =
            synthetic_baseline(baseline_world, &server_source("", "")).expect("baseline parses");
        let split =
            synthetic_baseline(split_world, &server_source("", "")).expect("split surface parses");
        compare_baseline(&baseline, &split)
            .expect("physical impl block count is deliberately not a ratchet");
    }

    #[test]
    fn session_resources_and_factory_fanout_are_exact() {
        let baseline = synthetic_baseline(&world_source("state: u8,", ""), &server_source("", ""))
            .expect("baseline parses");
        let resources_growth = synthetic_baseline(
            &world_source("state: u8,", ""),
            &server_source("pub(super) extra: u32,", ""),
        )
        .expect("resource growth parses");
        let error = compare_baseline(&baseline, &resources_growth)
            .expect_err("SessionResources growth must fail");
        assert!(
            error.contains("unreviewed SessionResources field surface"),
            "{error}"
        );

        let factory_growth = synthetic_baseline(
            &world_source("state: u8,", ""),
            &server_source("", "session.set_new_resource(1);"),
        )
        .expect("factory growth parses");
        let error =
            compare_baseline(&baseline, &factory_growth).expect_err("factory growth must fail");
        assert!(error.contains("create_session setter call"), "{error}");

        let alias_factory = synthetic_baseline(
            &world_source("state: u8,", ""),
            &server_source(
                "",
                "let alias = &mut session; alias.install_new_resource(1);",
            ),
        )
        .expect("alias receiver parses");
        assert!(
            alias_factory
                .session_factory
                .setter_call_sites
                .iter()
                .any(|call| call.callee == "alias.install_new_resource"),
            "setter/install calls through aliases must be captured"
        );
    }

    #[test]
    fn factory_body_and_session_bearing_helper_bodies_are_exact() {
        let world = world_source("state: u8,", "");
        let baseline_server = server_source("", "");
        let baseline = synthetic_baseline(&world, &baseline_server).expect("baseline parses");

        let helper_wiring = server_source("", "wire_extra(&mut session);").replacen(
            "async fn create_session",
            "fn wire_extra(session: &mut WorldSession) { session.set_char_db(7); }\n\
             async fn create_session",
            1,
        );
        let helper_wiring =
            synthetic_baseline(&world, &helper_wiring).expect("helper wiring parses");
        assert_eq!(
            baseline.session_factory.setter_call_sites,
            helper_wiring.session_factory.setter_call_sites,
            "a setter delegated to a helper is deliberately outside the direct-call inventory"
        );
        let error = compare_baseline(&baseline, &helper_wiring)
            .expect_err("new helper wiring must change the full factory body fingerprint");
        assert!(
            error.contains("create_session body fingerprint changed"),
            "{error}"
        );

        let same_signature_body_change = baseline_server.replace(
            "WorldSession::new(account_id)",
            "WorldSession::new(account_id.wrapping_add(1))",
        );
        let same_signature_body_change = synthetic_baseline(&world, &same_signature_body_change)
            .expect("same-signature body change parses");
        assert_eq!(
            baseline.session_factory.signature,
            same_signature_body_change.session_factory.signature
        );
        assert_eq!(
            baseline.session_factory.world_session_new_sites,
            same_signature_body_change
                .session_factory
                .world_session_new_sites
        );
        let error = compare_baseline(&baseline, &same_signature_body_change)
            .expect_err("same-signature factory body changes must drift");
        assert!(
            error.contains("create_session body fingerprint changed"),
            "{error}"
        );

        let helper_v1 = server_source("", "wire_extra(&mut session);").replacen(
            "async fn create_session",
            "fn wire_extra(session: &mut WorldSession) { session.set_char_db(1); }\n\
             async fn create_session",
            1,
        );
        let helper_v2 = helper_v1.replace("session.set_char_db(1)", "session.set_char_db(2)");
        let helper_v1 = synthetic_baseline(&world, &helper_v1).expect("helper v1 parses");
        let helper_v2 = synthetic_baseline(&world, &helper_v2).expect("helper v2 parses");
        assert_eq!(
            helper_v1.session_factory.body_fingerprint, helper_v2.session_factory.body_fingerprint,
            "only the already-wired helper body changed"
        );
        let error = compare_baseline(&helper_v1, &helper_v2)
            .expect_err("a helper called with session must have an exact body fingerprint");
        assert!(
            error.contains("create_session session-bearing helper body"),
            "{error}"
        );
    }

    #[test]
    fn command_variants_transitive_payloads_and_broadcast_fields_are_exact() {
        let world = world_source("state: u8,", "");
        let server = server_source("", "");
        let baseline = synthetic_baseline_with_network(
            &world,
            &server,
            r#"
                pub enum SessionCommand { Kick(KickCommand) }
                pub struct KickCommand { pub nested: NestedPayload }
                pub struct NestedPayload { pub reason: String }
                pub struct PlayerBroadcastInfo { pub map_id: u16 }
            "#,
        )
        .expect("network baseline parses");
        let changed = synthetic_baseline_with_network(
            &world,
            &server,
            r#"
                pub enum SessionCommand { Kick(KickCommand), Refresh }
                pub struct KickCommand { pub nested: NestedPayload }
                pub struct NestedPayload { pub reason: Vec<u8> }
                pub struct PlayerBroadcastInfo { pub map_id: u16, pub instance_id: u32 }
            "#,
        )
        .expect("changed network surface parses");
        let error = compare_baseline(&baseline, &changed).expect_err("network drift must fail");
        assert!(error.contains("SessionCommand variants changed"), "{error}");
        assert!(error.contains("transitive payload type"), "{error}");
        assert!(
            error.contains("PlayerBroadcastInfo surface changed"),
            "{error}"
        );
    }

    #[test]
    fn direct_registry_accesses_are_production_only_and_exact() {
        let world = world_source("state: u8,", "");
        let server = server_source("", "");
        let baseline = synthetic_baseline(&world, &server).expect("baseline parses");

        let test_only_world = format!(
            "{world}\n#[cfg(test)] fn fixture(players: &PlayerRegistry) {{ players.clear(); }}"
        );
        let test_only = synthetic_baseline(&test_only_world, &server)
            .expect("test-only registry access parses");
        compare_baseline(&baseline, &test_only)
            .expect("test-only registry access is not production debt");

        let production_world =
            format!("{world}\nfn escape(players: &PlayerRegistry) {{ players.get(&1); }}");
        let production = synthetic_baseline(&production_world, &server)
            .expect("production registry access parses");
        let error = compare_baseline(&baseline, &production)
            .expect_err("new production registry access must fail the exact ratchet");
        assert!(
            error.contains("untracked direct registry access"),
            "{error}"
        );
        assert!(error.contains("PlayerRegistry"), "{error}");
    }

    #[test]
    fn baseline_envelope_round_trips_with_semantic_policy_fields() {
        let baseline = synthetic_baseline(&world_source("state: u8,", ""), &server_source("", ""))
            .expect("baseline parses");
        let mut value = serde_json::to_value(BaselineEnvelope {
            schema_version: 1,
            persistence_access_snapshot: PERSISTENCE_ACCESS_SNAPSHOT_RELATIVE_PATH,
            syntax_baseline: &baseline,
        })
        .expect("baseline serializes");
        value
            .as_object_mut()
            .expect("envelope object")
            .insert("responsibilities".to_owned(), serde_json::json!([]));
        let parsed: PolicyEnvelope =
            serde_json::from_value(value).expect("semantic keys remain isolated");
        assert_eq!(parsed.syntax_baseline, baseline);
    }

    #[test]
    fn repository_surface_can_be_collected() {
        let repository_root = crate::repository_root().expect("repository root");
        let baseline = collect_repository_baseline(&repository_root)
            .unwrap_or_else(|error| panic!("repository baseline must parse:\n{error}"));
        let raw_session_source =
            fs::read_to_string(repository_root.join("crates/wow-world/src/session.rs"))
                .expect("read session source");
        let raw_session = syn::parse_file(&raw_session_source).expect("parse session source");
        let raw_fields = raw_session
            .items
            .iter()
            .find_map(|item| match item {
                Item::Struct(item) if item.ident == WORLD_SESSION_NAME => Some(&item.fields),
                _ => None,
            })
            .expect("WorldSession definition");
        assert_eq!(raw_fields.len(), 738);
        let test_only_fields = raw_fields
            .iter()
            .filter(|field| {
                !cfg_context_allows_production(&[], &field.attrs)
                    .expect("repository field cfg is valid")
            })
            .count();
        assert_eq!(test_only_fields, 11);
        assert_eq!(baseline.world_session.fields.len(), 738);
        assert_eq!(
            baseline
                .world_session
                .fields
                .iter()
                .filter(|field| field.source_class == "production")
                .count(),
            727
        );
        assert_eq!(
            baseline
                .world_session
                .fields
                .iter()
                .filter(|field| field.source_class == "test_fixture")
                .count(),
            11
        );
        assert_eq!(baseline.session_resources.fields.len(), 243);
        assert_eq!(baseline.world_session.impls.len(), 20);
        assert_eq!(baseline.session_command.variants.len(), 37);
        assert_eq!(baseline.player_broadcast_info.fields.len(), 80);
        assert_eq!(baseline.registry_accesses.accesses.len(), 685);
        assert!(baseline.registry_accesses.accesses.iter().all(|record| {
            record.source.starts_with("crates/") && !Path::new(&record.source).is_absolute()
        }));
    }
}
