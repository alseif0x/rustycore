//! Exact ownership surface for the SessionCore hub aggregate.

use super::*;

use std::collections::{BTreeMap, BTreeSet};

use crate::bridge_access::{
    BridgeSource, SuppliedTypeIdentity, TypeIdentityError, TypeIdentityKind, TypeIdentityQuery,
    resolve_supplied_type_identities,
};

pub(super) const SESSION_CORE_NAME: &str = "SessionCore";

/// The destination identity is pinned separately from the WorldSession facade.
pub(super) const WORLD_CORE_SESSION_CORE_MODULE: &str = "crate::session::state::session_core";

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CoreOwnerAvailability {
    pub production: bool,
    pub test: bool,
}

impl From<Availability> for CoreOwnerAvailability {
    fn from(availability: Availability) -> Self {
        Self {
            production: availability.production,
            test: availability.test,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SessionCoreDefinitionSurface {
    pub package: String,
    pub availability: CoreOwnerAvailability,
    pub definition: DefinitionSurface,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SessionCoreFieldSurface {
    pub package: String,
    pub module: String,
    pub availability: CoreOwnerAvailability,
    #[serde(default)]
    pub declaration_ordinal: usize,
    pub field: FieldSurface,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SessionCoreImplSurface {
    pub package: String,
    pub module: String,
    pub self_type: String,
    #[serde(default)]
    pub resolved_provider: Vec<SessionCoreProviderSurface>,
    pub trait_path: Option<String>,
    pub cfg: Vec<String>,
    pub availability: CoreOwnerAvailability,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SessionCoreProviderSurface {
    pub package: String,
    pub module: String,
    pub symbol: String,
    pub kind: String,
    pub source_path: String,
    pub guards: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorldSessionCoreBindingSurface {
    pub package: String,
    pub module: String,
    pub source_path: String,
    pub type_expression: String,
    pub cfg: Vec<String>,
    pub availability: CoreOwnerAvailability,
    pub providers: Vec<SessionCoreProviderSurface>,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SessionCoreImplItemSurface {
    pub package: String,
    pub module: String,
    pub self_type: String,
    pub trait_path: Option<String>,
    pub kind: String,
    pub name: String,
    pub visibility: String,
    pub signature: String,
    pub cfg: Vec<String>,
    pub availability: CoreOwnerAvailability,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SessionCoreSurface {
    pub definition: Option<SessionCoreDefinitionSurface>,
    pub fields: Vec<SessionCoreFieldSurface>,
    pub impls: Vec<SessionCoreImplSurface>,
    pub impl_items: Vec<SessionCoreImplItemSurface>,
    #[serde(default)]
    pub world_session_core_bindings: Vec<WorldSessionCoreBindingSurface>,
}

impl SessionCoreSurface {
    pub(super) fn canonicalize(&mut self) {
        self.fields.sort();
        self.impls.sort();
        self.impl_items.sort();
        self.world_session_core_bindings.sort();
        for binding in &mut self.world_session_core_bindings {
            binding.providers.sort();
        }
        for item in &mut self.impls {
            item.resolved_provider.sort();
        }
    }
}

pub(super) fn require_production_definition(surface: &SessionCoreSurface) -> Result<(), String> {
    match &surface.definition {
        Some(definition) if definition.availability.production => Ok(()),
        Some(definition) => Err(format!(
            "{}::{} defines {SESSION_CORE_NAME} only outside production availability",
            definition.package, definition.definition.module,
        )),
        None => Err(format!(
            "missing {}::{WORLD_CORE_SESSION_CORE_MODULE}::{SESSION_CORE_NAME} definition",
            PackageRole::WorldCore.package_name(),
        )),
    }
}

pub(super) fn require_production_field_binding(surface: &SessionCoreSurface) -> Result<(), String> {
    if surface.world_session_core_bindings.iter().any(|binding| {
        binding.package == PackageRole::World.package_name()
            && binding.availability.production
            && binding.providers.iter().all(is_canonical_provider)
            && !binding.providers.is_empty()
    }) {
        return Ok(());
    }
    Err(format!(
        "missing resolved production WorldSession.core binding to {}::{WORLD_CORE_SESSION_CORE_MODULE}::{SESSION_CORE_NAME}",
        PackageRole::WorldCore.package_name(),
    ))
}

fn is_canonical_provider(provider: &SessionCoreProviderSurface) -> bool {
    provider.package == PackageRole::WorldCore.package_name()
        && provider.module == WORLD_CORE_SESSION_CORE_MODULE
        && provider.symbol == SESSION_CORE_NAME
        && provider.kind == "struct"
}

fn provider_surfaces(identities: Vec<SuppliedTypeIdentity>) -> Vec<SessionCoreProviderSurface> {
    let mut providers = identities
        .into_iter()
        .map(|identity| SessionCoreProviderSurface {
            package: identity.package,
            module: identity.module,
            symbol: identity.symbol,
            kind: identity.kind.to_string(),
            source_path: identity.source_path,
            guards: identity.guards,
        })
        .collect::<Vec<_>>();
    providers.sort();
    providers
}

struct PendingFieldBinding {
    surface: WorldSessionCoreBindingSurface,
    result: Result<Vec<SessionCoreProviderSurface>, TypeIdentityError>,
}

pub(super) struct ResolvedOwnerTypes {
    pub(super) impl_providers: BTreeMap<usize, Vec<SessionCoreProviderSurface>>,
    field_bindings: Vec<PendingFieldBinding>,
    errors: Vec<String>,
}

struct OwnedTypeQuery<'a> {
    package: String,
    module: String,
    cfg: Vec<String>,
    ty: &'a Type,
    target: QueryTarget,
}

enum QueryTarget {
    WorldSessionCoreField {
        source_path: String,
        type_expression: String,
        availability: CoreOwnerAvailability,
    },
    Impl {
        item: usize,
        package: String,
        module: String,
        self_type: String,
        literal_session_core: bool,
    },
}

/// Resolve the WorldSession field and candidate SessionCore impls in one batch
/// over the already-mounted source graph. Candidate impl selection deliberately
/// excludes generic/wrapper self types; only direct nominal paths and aliases
/// whose declarations or imports lead back to `SessionCore` enter the query.
pub(super) fn resolve_owner_types(
    units: &[(SourceUnit, syn::File)],
    sources: &[BridgeSource<'_>],
) -> Result<ResolvedOwnerTypes, String> {
    let aliases = session_core_aliases(units);
    let mut owned_queries = Vec::new();
    let mut query_errors = Vec::new();
    for (unit, syntax) in units {
        collect_type_queries(
            unit,
            &syntax.items,
            &unit.logical_module_path,
            &unit.cfg,
            unit.availability,
            &aliases,
            &mut owned_queries,
        );
    }
    if owned_queries.is_empty() {
        return Ok(ResolvedOwnerTypes {
            impl_providers: BTreeMap::new(),
            field_bindings: Vec::new(),
            errors: query_errors,
        });
    }

    let queries = owned_queries
        .iter()
        .map(|query| TypeIdentityQuery {
            package: &query.package,
            module: &query.module,
            ty: query.ty,
            cfg: &query.cfg,
        })
        .collect::<Vec<_>>();
    let results = resolve_supplied_type_identities(sources, &queries)
        .map_err(|error| format!("cannot resolve SessionCore nominal ownership: {error}"))?;
    if results.len() != owned_queries.len() {
        return Err(format!(
            "SessionCore nominal resolver returned {} results for {} queries",
            results.len(),
            owned_queries.len(),
        ));
    }

    let mut impl_providers = BTreeMap::new();
    let mut field_bindings = Vec::new();
    for (query, result) in owned_queries.into_iter().zip(results) {
        match query.target {
            QueryTarget::WorldSessionCoreField {
                source_path,
                type_expression,
                availability,
            } => field_bindings.push(PendingFieldBinding {
                surface: WorldSessionCoreBindingSurface {
                    package: query.package,
                    module: query.module,
                    source_path,
                    type_expression,
                    cfg: query.cfg,
                    availability,
                    providers: Vec::new(),
                },
                result: result.map(provider_surfaces),
            }),
            QueryTarget::Impl {
                item,
                package,
                module,
                self_type,
                literal_session_core,
            } => match result {
                Ok(identities) => {
                    let core_identities = identities
                        .iter()
                        .filter(|identity| is_canonical_identity(identity))
                        .cloned()
                        .collect::<Vec<_>>();
                    if !core_identities.is_empty() {
                        impl_providers.insert(item, provider_surfaces(core_identities));
                    } else if literal_session_core {
                        query_errors.push(format!(
                            "{module} in {package} implements candidate {self_type}, which resolves to {:?}; expected {}::{WORLD_CORE_SESSION_CORE_MODULE}::{SESSION_CORE_NAME}",
                            provider_surfaces(identities),
                            PackageRole::WorldCore.package_name(),
                        ));
                    }
                    // A renamed alias that resolves to another supplied owner
                    // is a homonym, not a Core impl. If it formerly resolved
                    // here, the exact baseline surface records the change.
                }
                Err(error) => query_errors.push(format!(
                    "cannot resolve candidate {self_type} impl at {package}::{module}: {error}"
                )),
            },
        }
    }

    Ok(ResolvedOwnerTypes {
        impl_providers,
        field_bindings,
        errors: query_errors,
    })
}

fn is_canonical_identity(identity: &SuppliedTypeIdentity) -> bool {
    identity.package == PackageRole::WorldCore.package_name()
        && identity.module == WORLD_CORE_SESSION_CORE_MODULE
        && identity.symbol == SESSION_CORE_NAME
        && identity.kind == TypeIdentityKind::Struct
}

pub(super) fn install_resolved_field_bindings(
    mut resolved: ResolvedOwnerTypes,
    builder: &mut BaselineBuilder,
) {
    builder.errors.append(&mut resolved.errors);
    for pending in resolved.field_bindings {
        match pending.result {
            Ok(providers) if providers.iter().all(is_canonical_provider) && !providers.is_empty() => {
                let mut surface = pending.surface;
                surface.providers = providers;
                builder
                    .session_core_owner
                    .world_session_core_bindings
                    .push(surface);
            }
            Ok(providers) => builder.errors.push(format!(
                "WorldSession.core at {}::{} resolves to {providers:?}; expected {}::{WORLD_CORE_SESSION_CORE_MODULE}::{SESSION_CORE_NAME} struct",
                pending.surface.package,
                pending.surface.module,
                PackageRole::WorldCore.package_name(),
            )),
            Err(error) => builder.errors.push(format!(
                "cannot resolve WorldSession.core at {}::{} to {}::{WORLD_CORE_SESSION_CORE_MODULE}::{SESSION_CORE_NAME}: {error}",
                pending.surface.package,
                pending.surface.module,
                PackageRole::WorldCore.package_name(),
            )),
        }
    }
}

fn collect_type_queries<'a>(
    unit: &SourceUnit,
    items: &'a [Item],
    module: &str,
    cfg: &[String],
    availability: Availability,
    aliases: &BTreeSet<String>,
    queries: &mut Vec<OwnedTypeQuery<'a>>,
) {
    for item in items {
        let attributes = match item {
            Item::Impl(item) => &item.attrs,
            Item::Mod(item) => &item.attrs,
            Item::Struct(item) => &item.attrs,
            _ => continue,
        };
        // `collect_items` owns syntax diagnostics for these same attributes;
        // this pass only needs their effective query context.
        let mut ignored_cfg_errors = Vec::new();
        let (item_cfg, item_availability) = item_context(
            cfg,
            availability,
            attributes,
            "SessionCore identity candidate",
            &mut ignored_cfg_errors,
        );
        if item_availability.source_class().is_none() {
            continue;
        }
        match item {
            Item::Mod(item_mod) => {
                if let Some((_, inline_items)) = &item_mod.content {
                    collect_type_queries(
                        unit,
                        inline_items,
                        &format!("{module}::{}", item_mod.ident),
                        &item_cfg,
                        item_availability,
                        aliases,
                        queries,
                    );
                }
            }
            Item::Struct(item_struct)
                if unit.role == PackageRole::World
                    && item_struct.ident == WORLD_SESSION_NAME
                    && matches!(module, WORLD_SESSION_MODULE | WORLD_SESSION_STATE_MODULE) =>
            {
                let syn::Fields::Named(fields) = &item_struct.fields else {
                    continue;
                };
                for field in &fields.named {
                    if !field.ident.as_ref().is_some_and(|ident| ident == "core") {
                        continue;
                    }
                    // Field cfg validation remains owned by `collect_struct`.
                    let mut ignored_cfg_errors = Vec::new();
                    let (field_cfg, field_availability) = item_context(
                        &item_cfg,
                        item_availability,
                        &field.attrs,
                        "WorldSession.core field",
                        &mut ignored_cfg_errors,
                    );
                    if field_availability.source_class().is_none() {
                        continue;
                    }
                    queries.push(OwnedTypeQuery {
                        package: unit.role.package_name().to_owned(),
                        module: module.to_owned(),
                        cfg: field_cfg,
                        ty: &field.ty,
                        target: QueryTarget::WorldSessionCoreField {
                            source_path: unit.repository_relative_path.clone(),
                            type_expression: normalized_tokens(&field.ty),
                            availability: field_availability.into(),
                        },
                    });
                }
            }
            Item::Impl(item_impl) => {
                let Some(ty) = simple_nominal_type(&item_impl.self_ty) else {
                    continue;
                };
                let Some(name) = last_type_path_ident(ty) else {
                    continue;
                };
                if !aliases.contains(&name) {
                    continue;
                }
                queries.push(OwnedTypeQuery {
                    package: unit.role.package_name().to_owned(),
                    module: module.to_owned(),
                    cfg: item_cfg,
                    ty,
                    target: QueryTarget::Impl {
                        item: item_impl as *const ItemImpl as usize,
                        package: unit.role.package_name().to_owned(),
                        module: module.to_owned(),
                        self_type: normalized_tokens(&item_impl.self_ty),
                        literal_session_core: name == SESSION_CORE_NAME,
                    },
                });
            }
            _ => {}
        }
    }
}

fn simple_nominal_type(ty: &Type) -> Option<&Type> {
    match ty {
        Type::Group(group) => simple_nominal_type(&group.elem),
        Type::Paren(paren) => simple_nominal_type(&paren.elem),
        Type::Path(path)
            if path.qself.is_none()
                && path
                    .path
                    .segments
                    .iter()
                    .all(|segment| matches!(&segment.arguments, syn::PathArguments::None)) =>
        {
            Some(ty)
        }
        _ => None,
    }
}

fn last_type_path_ident(ty: &Type) -> Option<String> {
    let Type::Path(path) = ty else {
        return None;
    };
    path.path
        .segments
        .last()
        .map(|segment| segment.ident.to_string())
}

fn session_core_aliases(units: &[(SourceUnit, syn::File)]) -> BTreeSet<String> {
    let mut aliases = BTreeSet::from([SESSION_CORE_NAME.to_owned()]);
    loop {
        let previous_len = aliases.len();
        for (_, syntax) in units {
            collect_aliases(&syntax.items, &mut aliases);
        }
        if aliases.len() == previous_len {
            return aliases;
        }
    }
}

fn collect_aliases(items: &[Item], aliases: &mut BTreeSet<String>) {
    for item in items {
        match item {
            Item::Type(item_type) => {
                if simple_nominal_type(&item_type.ty)
                    .and_then(last_type_path_ident)
                    .is_some_and(|name| aliases.contains(&name))
                {
                    aliases.insert(item_type.ident.to_string());
                }
            }
            Item::Use(item_use) => collect_use_aliases(&item_use.tree, aliases, &mut Vec::new()),
            Item::Mod(item_mod) => {
                if let Some((_, inline_items)) = &item_mod.content {
                    collect_aliases(inline_items, aliases);
                }
            }
            _ => {}
        }
    }
}

fn collect_use_aliases(tree: &UseTree, aliases: &mut BTreeSet<String>, prefix: &mut Vec<String>) {
    match tree {
        UseTree::Path(path) => {
            prefix.push(path.ident.to_string());
            collect_use_aliases(&path.tree, aliases, prefix);
            prefix.pop();
        }
        UseTree::Name(name) if aliases.contains(&name.ident.to_string()) => {
            aliases.insert(name.ident.to_string());
        }
        UseTree::Rename(rename) if aliases.contains(&rename.ident.to_string()) => {
            aliases.insert(rename.rename.to_string());
        }
        UseTree::Rename(rename)
            if rename.ident == "self"
                && prefix.last().is_some_and(|name| aliases.contains(name)) =>
        {
            aliases.insert(rename.rename.to_string());
        }
        UseTree::Group(group) => {
            for item in &group.items {
                collect_use_aliases(item, aliases, prefix);
            }
        }
        UseTree::Name(_) | UseTree::Rename(_) | UseTree::Glob(_) => {}
    }
}

pub(super) fn collect_item(
    item: &Item,
    role: PackageRole,
    module: &str,
    cfg: &[String],
    availability: Availability,
    builder: &mut BaselineBuilder,
) {
    match item {
        Item::Struct(item_struct) if item_struct.ident == SESSION_CORE_NAME => {
            if role != PackageRole::WorldCore || module != WORLD_CORE_SESSION_CORE_MODULE {
                wrong_definition_owner(role, module, builder);
                return;
            }
            collect_definition(item_struct, module, cfg, availability, builder);
        }
        Item::Enum(item_enum) if item_enum.ident == SESSION_CORE_NAME => {
            wrong_definition_kind(role, module, "enum", builder);
        }
        Item::Union(item_union) if item_union.ident == SESSION_CORE_NAME => {
            wrong_definition_kind(role, module, "union", builder);
        }
        Item::Type(item_type) if item_type.ident == SESSION_CORE_NAME => {
            wrong_definition_kind(role, module, "type alias", builder);
        }
        Item::Impl(item_impl) => {
            let item_key = item_impl as *const ItemImpl as usize;
            let Some(resolved_provider) =
                builder.session_core_owner_impl_providers.remove(&item_key)
            else {
                return;
            };
            if role != PackageRole::WorldCore {
                builder.errors.push(format!(
                    "{module} in {} implements {SESSION_CORE_NAME}; its impl ownership must be in {}",
                    role.package_name(),
                    PackageRole::WorldCore.package_name(),
                ));
                return;
            }
            collect_impl(
                item_impl,
                module,
                cfg,
                availability,
                resolved_provider,
                builder,
            );
        }
        _ => {}
    }
}

fn wrong_definition_owner(role: PackageRole, module: &str, builder: &mut BaselineBuilder) {
    builder.errors.push(format!(
        "{module} in {} defines {SESSION_CORE_NAME}; expected {}::{WORLD_CORE_SESSION_CORE_MODULE}",
        role.package_name(),
        PackageRole::WorldCore.package_name(),
    ));
}

fn wrong_definition_kind(
    role: PackageRole,
    module: &str,
    kind: &str,
    builder: &mut BaselineBuilder,
) {
    builder.errors.push(format!(
        "{module} in {} defines {SESSION_CORE_NAME} as a {kind}; the destination owner requires a struct",
        role.package_name(),
    ));
}

fn collect_definition(
    item: &syn::ItemStruct,
    module: &str,
    cfg: &[String],
    availability: Availability,
    builder: &mut BaselineBuilder,
) {
    let Some(source_class) = availability.source_class() else {
        return;
    };
    let definition = SessionCoreDefinitionSurface {
        package: PackageRole::WorldCore.package_name().to_owned(),
        availability: availability.into(),
        definition: definition_surface(
            module,
            SESSION_CORE_NAME,
            &item.vis,
            cfg.to_vec(),
            source_class,
        ),
    };
    set_once(
        &mut builder.session_core_owner.definition,
        definition,
        SESSION_CORE_NAME,
        &mut builder.errors,
    );
    builder
        .generated_surface_inputs
        .extend(generated_attribute_inputs(
            module,
            SESSION_CORE_NAME,
            &item.attrs,
            cfg,
            availability,
        ));

    let syn::Fields::Named(fields) = &item.fields else {
        builder.errors.push(format!(
            "{module}::{SESSION_CORE_NAME} must retain named fields for ownership auditing"
        ));
        return;
    };
    for (declaration_ordinal, field) in fields.named.iter().enumerate() {
        let (field_cfg, field_availability) = item_context(
            cfg,
            availability,
            &field.attrs,
            &format!("{SESSION_CORE_NAME} field"),
            &mut builder.errors,
        );
        let Some(field_source_class) = field_availability.source_class() else {
            continue;
        };
        let Some(name) = &field.ident else {
            builder.errors.push(format!(
                "{module}::{SESSION_CORE_NAME} contains an unnamed field"
            ));
            continue;
        };
        builder
            .generated_surface_inputs
            .extend(generated_attribute_inputs(
                module,
                &format!("{SESSION_CORE_NAME}::{name}"),
                &field.attrs,
                &field_cfg,
                field_availability,
            ));
        builder
            .session_core_owner
            .fields
            .push(SessionCoreFieldSurface {
                package: PackageRole::WorldCore.package_name().to_owned(),
                module: module.to_owned(),
                availability: field_availability.into(),
                declaration_ordinal,
                field: FieldSurface {
                    name: name.to_string(),
                    type_expression: normalized_tokens(&field.ty),
                    visibility: normalized_visibility(&field.vis),
                    cfg: field_cfg,
                    source_class: field_source_class.to_owned(),
                },
            });
    }
}

fn collect_impl(
    item: &ItemImpl,
    module: &str,
    cfg: &[String],
    availability: Availability,
    resolved_provider: Vec<SessionCoreProviderSurface>,
    builder: &mut BaselineBuilder,
) {
    if availability.source_class().is_none() {
        return;
    }
    let package = PackageRole::WorldCore.package_name().to_owned();
    let self_type = normalized_tokens(&item.self_ty);
    let trait_path = normalized_trait_path(item);
    let impl_cfg = cfg.to_vec();
    builder
        .generated_surface_inputs
        .extend(generated_attribute_inputs(
            module,
            &format!("impl {self_type}"),
            &item.attrs,
            &impl_cfg,
            availability,
        ));
    builder
        .session_core_owner
        .impls
        .push(SessionCoreImplSurface {
            package: package.clone(),
            module: module.to_owned(),
            self_type: self_type.clone(),
            resolved_provider,
            trait_path: trait_path.clone(),
            cfg: impl_cfg.clone(),
            availability: availability.into(),
        });

    for impl_item in &item.items {
        let (kind, name, visibility, signature, attrs) = match impl_item {
            ImplItem::Fn(function) => (
                "method",
                function.sig.ident.to_string(),
                &function.vis,
                normalized_tokens(&function.sig),
                function.attrs.as_slice(),
            ),
            ImplItem::Const(constant) => (
                "const",
                constant.ident.to_string(),
                &constant.vis,
                format!(
                    "const {} : {}",
                    constant.ident,
                    normalized_tokens(&constant.ty)
                ),
                constant.attrs.as_slice(),
            ),
            ImplItem::Type(associated_type) => (
                "type",
                associated_type.ident.to_string(),
                &associated_type.vis,
                format!(
                    "type {} = {}",
                    associated_type.ident,
                    normalized_tokens(&associated_type.ty)
                ),
                associated_type.attrs.as_slice(),
            ),
            ImplItem::Macro(item_macro) => {
                let (_, availability) = item_context(
                    &impl_cfg,
                    availability,
                    &item_macro.attrs,
                    "SessionCore impl macro",
                    &mut builder.errors,
                );
                if availability.source_class().is_some() {
                    builder.errors.push(format!(
                        "{module} contains macro {}! inside impl {SESSION_CORE_NAME}; generated associated items are outside the exact ownership grammar",
                        normalized_tokens(&item_macro.mac.path)
                    ));
                }
                continue;
            }
            ImplItem::Verbatim(_) => {
                builder.errors.push(format!(
                    "{module} contains unparsed verbatim syntax inside impl {SESSION_CORE_NAME}"
                ));
                continue;
            }
            _ => continue,
        };
        let (item_cfg, item_availability) = item_context(
            &impl_cfg,
            availability,
            attrs,
            &format!("SessionCore impl item {name}"),
            &mut builder.errors,
        );
        if item_availability.source_class().is_none() {
            continue;
        }
        builder
            .generated_surface_inputs
            .extend(generated_attribute_inputs(
                module,
                &format!("{SESSION_CORE_NAME}::{name}"),
                attrs,
                &item_cfg,
                item_availability,
            ));
        builder
            .session_core_owner
            .impl_items
            .push(SessionCoreImplItemSurface {
                package: package.clone(),
                module: module.to_owned(),
                self_type: self_type.clone(),
                trait_path: trait_path.clone(),
                kind: kind.to_owned(),
                name,
                visibility: normalized_visibility(visibility),
                signature,
                cfg: item_cfg,
                availability: item_availability.into(),
            });
    }
}

pub(super) fn compare(
    expected: &SessionCoreSurface,
    actual: &SessionCoreSurface,
    errors: &mut Vec<String>,
) {
    if expected.definition != actual.definition {
        errors.push(format!(
            "SessionCore definition changed: expected {:?}, actual {:?}",
            expected.definition, actual.definition
        ));
    }
    compare_exact(
        "SessionCore field",
        &expected.fields,
        &actual.fields,
        errors,
    );
    compare_exact("SessionCore impl", &expected.impls, &actual.impls, errors);
    compare_exact(
        "WorldSession.core resolved provider",
        &expected.world_session_core_bindings,
        &actual.world_session_core_bindings,
        errors,
    );
    compare_exact(
        "SessionCore impl item",
        &expected.impl_items,
        &actual.impl_items,
        errors,
    );
}

fn compare_exact<T: Clone + Ord + std::fmt::Debug>(
    label: &str,
    expected: &[T],
    actual: &[T],
    errors: &mut Vec<String>,
) {
    let mut expected = expected.to_vec();
    expected.sort();
    let mut actual = actual.to_vec();
    actual.sort();
    if expected != actual {
        errors.push(format!(
            "{label} surface changed: expected {expected:?}, actual {actual:?}"
        ));
    }
}
