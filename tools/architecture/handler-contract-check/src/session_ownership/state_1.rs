//! Session ownership inventory state definitions, part 1 of 3.
//!
//! Separated from the session_ownership.rs root under #660. Behaviour is preserved.

use super::*;

pub(super) const POLICY_RELATIVE_PATH: &str = "tools/architecture/session-ownership-policy.json";

pub(super) const PERSISTENCE_POLICY_RELATIVE_PATH: &str =
    "tools/architecture/persistence-boundary-policy.json";

pub(super) const PERSISTENCE_ANNOTATIONS_RELATIVE_PATH: &str =
    "tools/architecture/persistence-boundary-workflows.json";

pub(super) const PERSISTENCE_ACCESS_SNAPSHOT_RELATIVE_PATH: &str =
    "tools/architecture/persistence-access-snapshot.json";

pub(super) const ISSUE_LEDGER_RELATIVE_PATH: &str =
    "tools/architecture/architecture-issue-ledger.json";

pub(super) const WORLD_PACKAGE_ROOT: &str = "crates/wow-world";

pub(super) const WORLD_CRATE_ROOT: &str = "crates/wow-world/src/lib.rs";

pub(super) const SERVER_PACKAGE_ROOT: &str = "crates/world-server";

pub(super) const SERVER_CRATE_ROOT: &str = "crates/world-server/src/lib.rs";

pub(super) const NETWORK_PACKAGE_ROOT: &str = "crates/wow-network";

pub(super) const NETWORK_CRATE_ROOT: &str = "crates/wow-network/src/lib.rs";

pub(super) const SOCIAL_PACKAGE_ROOT: &str = "crates/wow-social";

pub(super) const SOCIAL_CRATE_ROOT: &str = "crates/wow-social/src/lib.rs";

pub(super) const WORLD_SESSION_MODULE: &str = "crate::session";

pub(super) const WORLD_SESSION_NAME: &str = "WorldSession";

pub(super) const SESSION_RESOURCES_MODULE: &str = "crate::session_resources";

pub(super) const SESSION_RESOURCES_NAME: &str = "SessionResources";

pub(super) const SESSION_FACTORY_MODULE: &str = "crate::session_factory";

pub(super) const SESSION_FACTORY_NAME: &str = "create_session";

pub(super) const PRIVATE_WORLD_SESSION_OWNER_ROOTS: &[&str] =
    &["crate::handlers::misc", WORLD_SESSION_MODULE];

/// Issue #140 relocated the Session mailbox here, so `SessionCommand` and its
/// payload closure now live in `wow-world` rather than `wow-network`.
pub(super) const WORLD_SESSION_MAILBOX_MODULE: &str = "crate::session::mailbox";

/// Issue #189 moved durable loot-money coordination to its own persistence
/// owner. Three `SessionCommand` payload types live there, so the contract
/// scan must reach it or they would silently leave the pinned inventory.
pub(super) const WORLD_LOOT_PERSISTENCE_MODULE: &str = "crate::loot_persistence";

/// Issue #137 relocated the atomic Group owner here. `SessionCommand` still
/// names `GroupDifficultyKindLikeCpp` in one payload, so the contract scan must
/// reach this module or that payload type would silently leave the inventory.
pub(super) const SOCIAL_GROUP_MODULE: &str = "crate::group";

/// Issue #138 relocated the opaque connected-session directory here, so direct
/// registry access remains part of the ownership scan.
pub(super) const WORLD_SESSION_DIRECTORY_MODULE: &str = "crate::session::directory";

pub(super) const SESSION_COMMAND_NAME: &str = "SessionCommand";

pub(super) const FNV1A_64_PRIME: u64 = 0x0000_0100_0000_01b3;

pub(super) const FNV1A_64_OFFSET_A: u64 = 0xcbf2_9ce4_8422_2325;

pub(super) const FNV1A_64_OFFSET_B: u64 = 0x8422_2325_cbf2_9ce4;

pub(super) const OWNERSHIP_TARGET_NAMES: [&str; 3] = [
    WORLD_SESSION_NAME,
    SESSION_RESOURCES_NAME,
    SESSION_COMMAND_NAME,
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
    pub generated_surface_inputs: Vec<GeneratedSurfaceInput>,
    pub(crate) registry_accesses: RegistryAccessBaseline,
    #[serde(skip)]
    pub(crate) persistence_accesses: PersistenceAccessBaseline,
    pub(crate) bridge_accesses: BridgeAccessBaseline,
}

#[derive(Debug, Deserialize)]
pub(super) struct PolicyEnvelope {
    pub(super) schema_version: u64,
    pub(super) syntax_baseline: SessionSyntaxBaseline,
    pub(super) persistence_access_snapshot: String,
    /// Legacy semantic fields remain accepted here; the Rust workflow-policy
    /// validator owns the exact persistence responsibility ledger.
    /// Keeping that data flattened here isolates the AST schema from it.
    #[serde(flatten)]
    pub(super) _semantic_policy: BTreeMap<String, Value>,
}

#[derive(Serialize)]
pub(super) struct BaselineEnvelope<'a> {
    pub(super) schema_version: u64,
    pub(super) persistence_access_snapshot: &'static str,
    pub(super) syntax_baseline: &'a SessionSyntaxBaseline,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum PackageRole {
    World,
    Server,
    Network,
    Social,
}

pub(super) struct SourceUnit {
    pub(super) role: PackageRole,
    pub(super) source_path: PathBuf,
    pub(super) repository_relative_path: String,
    pub(super) logical_module_path: String,
    pub(super) cfg: Vec<String>,
    pub(super) availability: Availability,
    pub(super) source: String,
}

impl PackageRole {
    pub(super) fn package_name(self) -> &'static str {
        match self {
            Self::World => "wow-world",
            Self::Server => "world-server",
            Self::Network => "wow-network",
            Self::Social => "wow-social",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct Availability {
    pub(super) production: bool,
    pub(super) test: bool,
}

impl Availability {
    pub(super) fn source_class(self) -> Option<&'static str> {
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
pub(super) struct BaselineBuilder {
    pub(super) errors: Vec<String>,
    pub(super) world_session_definition: Option<DefinitionSurface>,
    pub(super) world_session_fields: BTreeSet<FieldSurface>,
    pub(super) world_session_impls: BTreeSet<(String, Option<String>, Vec<String>, String)>,
    pub(super) world_session_impl_items: BTreeSet<ImplItemSurface>,
    pub(super) session_resources_definition: Option<DefinitionSurface>,
    pub(super) session_resources_fields: BTreeSet<FieldSurface>,
    pub(super) session_resources_constructions:
        BTreeMap<(String, String, usize, Vec<String>, String), usize>,
    pub(super) session_factory_definition: Option<DefinitionSurface>,
    pub(super) session_factory_signature: Option<String>,
    pub(super) session_factory_body_fingerprint: Option<String>,
    pub(super) session_factory_helper_calls: BTreeSet<(String, Vec<String>)>,
    pub(super) server_function_bodies:
        BTreeMap<(String, String), BTreeSet<SessionFactoryHelperSurface>>,
    pub(super) session_factory_calls: BTreeMap<(String, String, usize, Vec<String>, String), usize>,
    pub(super) world_session_new_calls:
        BTreeMap<(String, String, usize, Vec<String>, String), usize>,
    pub(super) session_factory_setter_calls:
        BTreeMap<(String, String, usize, Vec<String>, String), usize>,
    pub(super) generated_surface_inputs: BTreeSet<GeneratedSurfaceInput>,
    pub(super) contract_types: BTreeMap<String, Vec<SessionContractTypeDefinition>>,
}

#[derive(Clone, Debug)]
pub(super) struct SessionContractTypeDefinition {
    pub(super) surface: TypeSurface,
    pub(super) referenced_types: BTreeSet<String>,
    pub(super) generated_surface_inputs: BTreeSet<GeneratedSurfaceInput>,
}

pub(super) fn normalized_tokens(value: &impl ToTokens) -> String {
    value.to_token_stream().to_string()
}

/// Stable compact fingerprint for large normalized syntax surfaces. This is a
/// deterministic drift identity, not a security primitive.
pub(super) fn compact_fingerprint(value: &str) -> String {
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

pub(super) fn compact_token_fingerprint(value: &impl ToTokens) -> String {
    compact_fingerprint(&normalized_tokens(value))
}

pub(super) fn normalized_visibility(visibility: &Visibility) -> String {
    match visibility {
        Visibility::Inherited => "private".to_owned(),
        _ => normalized_tokens(visibility),
    }
}

pub(super) fn attribute_is_inert_for_generated_surface(attribute: &syn::Attribute) -> bool {
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

pub(super) fn generated_attribute_inputs(
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

pub(super) fn is_visible(visibility: &Visibility) -> bool {
    !matches!(visibility, Visibility::Inherited)
}

pub(super) fn set_once<T: Eq + std::fmt::Debug>(
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

pub(super) fn type_path_ends_with(type_expression: &Type, expected: &str) -> bool {
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

pub(super) fn token_stream_mentions_ident(tokens: &TokenStream, expected: &str) -> bool {
    tokens.clone().into_iter().any(|token| match token {
        TokenTree::Ident(ident) => ident == expected,
        TokenTree::Group(group) => token_stream_mentions_ident(&group.stream(), expected),
        TokenTree::Punct(_) | TokenTree::Literal(_) => false,
    })
}

pub(super) fn token_stream_mentions_ownership_target(tokens: &TokenStream) -> Option<&'static str> {
    OWNERSHIP_TARGET_NAMES
        .into_iter()
        .find(|target| token_stream_mentions_ident(tokens, target))
}

#[derive(Default)]
pub(super) struct IncludeMacroGuard {
    pub(super) count: usize,
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

pub(super) fn use_tree_renames_ident(tree: &UseTree, expected: &str) -> bool {
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

pub(super) fn item_context(
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

pub(super) fn definition_surface(
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

pub(super) fn collect_struct(
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
pub(super) struct TypeReferenceCollector {
    pub(super) names: BTreeSet<String>,
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

pub(super) fn collect_type_references(type_expression: &Type, names: &mut BTreeSet<String>) {
    let mut collector = TypeReferenceCollector::default();
    collector.visit_type(type_expression);
    names.extend(collector.names);
}

pub(super) fn contract_field_surfaces(
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

pub(super) fn collect_contract_type(
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
            let fields = contract_field_surfaces(
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
                let fields = contract_field_surfaces(
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
            let fields = contract_field_surfaces(
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
        .contract_types
        .entry(surface.definition.name.clone())
        .or_default()
        .push(SessionContractTypeDefinition {
            surface,
            referenced_types,
            generated_surface_inputs,
        });
}

pub(super) fn normalized_trait_path(item: &ItemImpl) -> Option<String> {
    item.trait_
        .as_ref()
        .map(|(_, path, _)| normalized_tokens(path))
}

pub(super) fn logical_world_session_owner(module: &str) -> &str {
    PRIVATE_WORLD_SESSION_OWNER_ROOTS
        .iter()
        .copied()
        .find(|root| {
            module == *root
                || module
                    .strip_prefix(*root)
                    .is_some_and(|suffix| suffix.starts_with("::"))
        })
        .unwrap_or(module)
}

pub(super) fn impl_item_surface(
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
