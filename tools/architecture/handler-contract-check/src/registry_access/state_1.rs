//! Registry access scan state definitions, part 1 of 4.
//!
//! Separated from the registry_access.rs root under #660. Behaviour is preserved.

use super::*;

pub(super) const REGISTRY_SCHEMA_VERSION: u32 = 1;

pub(super) const PASSTHROUGH_METHODS: &[&str] = &[
    "as_ref",
    "as_deref",
    "as_mut",
    "as_deref_mut",
    "unwrap",
    "unwrap_or",
    "unwrap_or_else",
    "expect",
    "map",
    "and_then",
    "filter",
    "inspect",
    "or",
    "or_else",
];

pub(super) const KNOWN_OPAQUE_VALUE_MACROS: &[&str] = &[
    "assert",
    "assert_eq",
    "assert_ne",
    "debug",
    "debug_assert",
    "debug_assert_eq",
    "debug_assert_ne",
    "error",
    "format",
    "info",
    "matches",
    "trace",
    "tracing",
    "warn",
];

/// One of the three public registry aliases being retired by #150/#151.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub(crate) enum RegistryKind {
    #[serde(rename = "PlayerRegistry")]
    Player,
    #[serde(rename = "GroupRegistry")]
    Group,
    #[serde(rename = "PendingInvites")]
    PendingInvites,
}

impl RegistryKind {
    pub(super) fn source_name(self) -> &'static str {
        match self {
            Self::Player => "PlayerRegistry",
            Self::Group => "GroupRegistry",
            Self::PendingInvites => "PendingInvites",
        }
    }

    pub(super) fn from_source_name(name: &str) -> Option<Self> {
        match name {
            "PlayerRegistry" => Some(Self::Player),
            "GroupRegistry" => Some(Self::Group),
            "PendingInvites" => Some(Self::PendingInvites),
            _ => None,
        }
    }

    pub(super) fn from_member_or_accessor(name: &str) -> Option<Self> {
        match name {
            "player_registry" => Some(Self::Player),
            "group_registry" => Some(Self::Group),
            "pending_invites" => Some(Self::PendingInvites),
            _ => None,
        }
    }
}

/// The exact syntactic capability exposed at an access site.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RegistryOperation {
    TypeReference,
    ImportAlias,
    TypeAlias,
    Member,
    Accessor,
    Construct,
    LocalAlias,
    AssignmentAlias,
    Clone,
    Return,
    ArgumentEscape,
    Index,
    Get,
    GetMut,
    Iter,
    Entry,
    Insert,
    Remove,
    Retain,
    Clear,
    OpaqueMacroBoundary,
}

impl RegistryOperation {
    pub(super) fn from_method(name: &str) -> Option<Self> {
        match name {
            "get" => Some(Self::Get),
            "get_mut" => Some(Self::GetMut),
            "iter" => Some(Self::Iter),
            "entry" => Some(Self::Entry),
            "insert" => Some(Self::Insert),
            "remove" => Some(Self::Remove),
            "retain" => Some(Self::Retain),
            "clear" => Some(Self::Clear),
            _ => None,
        }
    }
}

/// A canonical baseline row. `count` preserves identical repeated operations;
/// all other fields form the exact identity used by the comparator.
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RegistryAccessRecord {
    pub(crate) package: String,
    pub(crate) module: String,
    pub(crate) source: String,
    pub(crate) enclosing: String,
    pub(crate) registry: RegistryKind,
    pub(crate) operation: RegistryOperation,
    pub(crate) symbol: String,
    pub(crate) visibility: String,
    pub(crate) cfg: Vec<String>,
    pub(crate) fingerprint: String,
    pub(crate) count: usize,
}

/// Serializable exact snapshot. Rows emitted by the inventory are sorted,
/// unique by identity, and carry a positive multiplicity.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RegistryAccessBaseline {
    pub(crate) schema_version: u32,
    pub(crate) accesses: Vec<RegistryAccessRecord>,
}

/// One already-classified production source mount. The workspace/module walker
/// remains the caller's responsibility so this module can be tested without
/// executing Cargo metadata. Pass repository-relative `source_path` values.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ProductionRegistrySource<'a> {
    pub(crate) package: &'a str,
    pub(crate) module: &'a str,
    pub(crate) source_path: &'a str,
    pub(crate) inherited_cfg: &'a [String],
    pub(crate) source: &'a str,
}

pub(super) struct ParsedRegistrySource<'a> {
    pub(super) mount: ProductionRegistrySource<'a>,
    pub(super) syntax: syn::File,
    pub(super) cfg: Vec<String>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) struct AccessIdentity {
    pub(super) package: String,
    pub(super) module: String,
    pub(super) source: String,
    pub(super) enclosing: String,
    pub(super) registry: RegistryKind,
    pub(super) operation: RegistryOperation,
    pub(super) symbol: String,
    pub(super) visibility: String,
    pub(super) cfg: Vec<String>,
    pub(super) fingerprint: String,
}

impl RegistryAccessRecord {
    pub(super) fn identity(&self) -> AccessIdentity {
        AccessIdentity {
            package: self.package.clone(),
            module: self.module.clone(),
            source: self.source.clone(),
            enclosing: self.enclosing.clone(),
            registry: self.registry,
            operation: self.operation,
            symbol: self.symbol.clone(),
            visibility: self.visibility.clone(),
            cfg: self.cfg.clone(),
            fingerprint: self.fingerprint.clone(),
        }
    }
}

#[derive(Default)]
pub(super) struct AccessAccumulator {
    pub(super) rows: BTreeMap<AccessIdentity, usize>,
}

pub(super) struct RecordContext<'a> {
    pub(super) package: &'a str,
    pub(super) module: &'a str,
    pub(super) source: &'a str,
}

pub(super) struct NewAccess<'a> {
    pub(super) enclosing: &'a str,
    pub(super) registry: RegistryKind,
    pub(super) operation: RegistryOperation,
    pub(super) symbol: &'a str,
    pub(super) visibility: &'a str,
    pub(super) cfg: &'a [String],
    pub(super) fingerprint: String,
}

impl AccessAccumulator {
    pub(super) fn add(&mut self, context: &RecordContext<'_>, access: NewAccess<'_>) {
        let identity = AccessIdentity {
            package: context.package.to_owned(),
            module: context.module.to_owned(),
            source: context.source.to_owned(),
            enclosing: access.enclosing.to_owned(),
            registry: access.registry,
            operation: access.operation,
            symbol: access.symbol.to_owned(),
            visibility: access.visibility.to_owned(),
            cfg: access.cfg.to_vec(),
            fingerprint: access.fingerprint,
        };
        *self.rows.entry(identity).or_default() += 1;
    }

    pub(super) fn finish(self) -> RegistryAccessBaseline {
        RegistryAccessBaseline {
            schema_version: REGISTRY_SCHEMA_VERSION,
            accesses: self
                .rows
                .into_iter()
                .map(|(identity, count)| RegistryAccessRecord {
                    package: identity.package,
                    module: identity.module,
                    source: identity.source,
                    enclosing: identity.enclosing,
                    registry: identity.registry,
                    operation: identity.operation,
                    symbol: identity.symbol,
                    visibility: identity.visibility,
                    cfg: identity.cfg,
                    fingerprint: identity.fingerprint,
                    count,
                })
                .collect(),
        }
    }
}

pub(super) fn normalized_ident(ident: &proc_macro2::Ident) -> String {
    let value = ident.to_string();
    value.strip_prefix("r#").unwrap_or(&value).to_owned()
}

pub(super) fn normalized_tokens(value: &impl ToTokens) -> String {
    value.to_token_stream().to_string()
}

pub(super) fn normalized_visibility(visibility: &Visibility) -> String {
    normalized_tokens(visibility)
}

pub(super) fn method_fingerprint(method: &ExprMethodCall) -> String {
    let arguments = method
        .args
        .iter()
        .map(normalized_tokens)
        .collect::<Vec<_>>()
        .join(" | ");
    format!("{}({arguments})", normalized_ident(&method.method))
}

pub(super) fn path_segments(path: &syn::Path) -> Vec<String> {
    path.segments
        .iter()
        .map(|segment| normalized_ident(&segment.ident))
        .collect()
}

pub(super) fn last_path_ident(path: &syn::Path) -> Option<String> {
    path.segments
        .last()
        .map(|segment| normalized_ident(&segment.ident))
}

pub(super) fn canonical_use_tree(tree: &UseTree) -> String {
    match tree {
        UseTree::Path(path) => format!(
            "{}::{}",
            normalized_ident(&path.ident),
            canonical_use_tree(&path.tree)
        ),
        UseTree::Name(name) => normalized_ident(&name.ident),
        UseTree::Rename(rename) => format!(
            "{} as {}",
            normalized_ident(&rename.ident),
            normalized_ident(&rename.rename)
        ),
        UseTree::Glob(_) => "*".to_owned(),
        UseTree::Group(group) => {
            let mut items = group
                .items
                .iter()
                .map(canonical_use_tree)
                .collect::<Vec<_>>();
            items.sort();
            format!("{{{}}}", items.join(","))
        }
    }
}

pub(super) type KindSet = BTreeSet<RegistryKind>;

#[derive(Clone, Debug)]
pub(super) struct GlobalAliasDefinition {
    pub(super) package: String,
    pub(super) module: String,
    pub(super) local_name: String,
    pub(super) target_paths: Vec<Vec<String>>,
}

#[derive(Clone, Debug)]
pub(super) struct GlobalGlobImport {
    pub(super) package: String,
    pub(super) module: String,
    pub(super) target_path: Vec<String>,
}

#[derive(Default)]
pub(super) struct GlobalAliasIndex {
    pub(super) aliases: BTreeMap<(String, String), BTreeMap<String, KindSet>>,
    pub(super) known_modules: BTreeSet<(String, String)>,
    pub(super) package_by_crate_name: BTreeMap<String, String>,
}

impl GlobalAliasIndex {
    pub(super) fn aliases_for(
        &self,
        package: &str,
        module: &str,
    ) -> Option<&BTreeMap<String, KindSet>> {
        self.aliases.get(&(package.to_owned(), module.to_owned()))
    }

    pub(super) fn extend_module_symbols(
        &self,
        package: &str,
        module: &str,
        symbols: &mut ModuleSymbols,
    ) {
        let Some(aliases) = self.aliases_for(package, module) else {
            return;
        };
        for (name, kinds) in aliases {
            symbols
                .type_aliases
                .entry(name.clone())
                .or_default()
                .extend(kinds.iter().copied());
        }
    }

    pub(super) fn insert_aliases(
        &mut self,
        package: &str,
        module: &str,
        name: &str,
        kinds: KindSet,
    ) -> bool {
        if kinds.is_empty() {
            return false;
        }
        let entry = self
            .aliases
            .entry((package.to_owned(), module.to_owned()))
            .or_default()
            .entry(name.to_owned())
            .or_default();
        let previous_len = entry.len();
        entry.extend(kinds);
        entry.len() != previous_len
    }

    pub(super) fn resolve_symbol_path(
        &self,
        package: &str,
        current_module: &str,
        path: &[String],
    ) -> KindSet {
        let mut kinds = path
            .iter()
            .filter_map(|segment| RegistryKind::from_source_name(segment))
            .collect::<KindSet>();
        let Some((target_package, target_module, name)) =
            self.resolve_symbol_location(package, current_module, path)
        else {
            return kinds;
        };
        if let Some(resolved) = self
            .aliases_for(&target_package, &target_module)
            .and_then(|aliases| aliases.get(&name))
        {
            kinds.extend(resolved.iter().copied());
        }
        kinds
    }

    pub(super) fn resolve_symbol_location(
        &self,
        package: &str,
        current_module: &str,
        path: &[String],
    ) -> Option<(String, String, String)> {
        let name = path.last()?.clone();
        if path.len() == 1 {
            return Some((package.to_owned(), current_module.to_owned(), name));
        }
        let qualifiers = &path[..path.len() - 1];
        let (target_package, target_module) =
            self.resolve_module_path(package, current_module, qualifiers)?;
        Some((target_package, target_module, name))
    }

    pub(super) fn resolve_module_path(
        &self,
        package: &str,
        current_module: &str,
        path: &[String],
    ) -> Option<(String, String)> {
        let first = path.first().map(String::as_str)?;
        match first {
            "crate" => Some((package.to_owned(), path.join("::"))),
            "self" => {
                let suffix = &path[1..];
                let module = if suffix.is_empty() {
                    current_module.to_owned()
                } else {
                    format!("{current_module}::{}", suffix.join("::"))
                };
                Some((package.to_owned(), module))
            }
            "super" => {
                let mut module: Vec<_> = current_module.split("::").collect();
                let mut index = 0;
                while path.get(index).is_some_and(|segment| segment == "super") {
                    if module.len() > 1 {
                        module.pop();
                    }
                    index += 1;
                }
                module.extend(path[index..].iter().map(String::as_str));
                Some((package.to_owned(), module.join("::")))
            }
            crate_name if self.package_by_crate_name.contains_key(crate_name) => {
                let target_package = self.package_by_crate_name.get(crate_name)?.clone();
                let suffix = &path[1..];
                let module = if suffix.is_empty() {
                    "crate".to_owned()
                } else {
                    format!("crate::{}", suffix.join("::"))
                };
                Some((target_package, module))
            }
            _ => {
                let root_candidate = (package.to_owned(), format!("crate::{}", path.join("::")));
                if self.known_modules.contains(&root_candidate) {
                    return Some(root_candidate);
                }
                let relative_candidate = (
                    package.to_owned(),
                    format!("{current_module}::{}", path.join("::")),
                );
                self.known_modules
                    .contains(&relative_candidate)
                    .then_some(relative_candidate)
            }
        }
    }
}

#[derive(Default)]
pub(super) struct AliasTypePathCollector {
    pub(super) paths: Vec<Vec<String>>,
}

impl<'ast> Visit<'ast> for AliasTypePathCollector {
    fn visit_type_path(&mut self, node: &'ast syn::TypePath) {
        self.paths.push(path_segments(&node.path));
        visit::visit_type_path(self, node);
    }
}

pub(super) fn flatten_use_alias_definitions(
    tree: &UseTree,
    prefix: &mut Vec<String>,
    package: &str,
    module: &str,
    definitions: &mut Vec<GlobalAliasDefinition>,
    globs: &mut Vec<GlobalGlobImport>,
) {
    match tree {
        UseTree::Path(path) => {
            prefix.push(normalized_ident(&path.ident));
            flatten_use_alias_definitions(&path.tree, prefix, package, module, definitions, globs);
            prefix.pop();
        }
        UseTree::Name(name) => {
            let source_name = normalized_ident(&name.ident);
            if source_name == "self" {
                return;
            }
            let mut target_path = prefix.clone();
            target_path.push(source_name.clone());
            definitions.push(GlobalAliasDefinition {
                package: package.to_owned(),
                module: module.to_owned(),
                local_name: source_name,
                target_paths: vec![target_path],
            });
        }
        UseTree::Rename(rename) => {
            let mut target_path = prefix.clone();
            target_path.push(normalized_ident(&rename.ident));
            definitions.push(GlobalAliasDefinition {
                package: package.to_owned(),
                module: module.to_owned(),
                local_name: normalized_ident(&rename.rename),
                target_paths: vec![target_path],
            });
        }
        UseTree::Group(group) => {
            for item in &group.items {
                flatten_use_alias_definitions(item, prefix, package, module, definitions, globs);
            }
        }
        UseTree::Glob(_) => globs.push(GlobalGlobImport {
            package: package.to_owned(),
            module: module.to_owned(),
            target_path: prefix.clone(),
        }),
    }
}

pub(super) fn collect_global_alias_definitions(
    package: &str,
    module: &str,
    items: &[Item],
    cfg: &[String],
    index: &mut GlobalAliasIndex,
    definitions: &mut Vec<GlobalAliasDefinition>,
    globs: &mut Vec<GlobalGlobImport>,
    errors: &mut Vec<String>,
) {
    index
        .known_modules
        .insert((package.to_owned(), module.to_owned()));
    for item in items {
        match item {
            Item::Use(item_use) if production(cfg, &item_use.attrs, errors, "use item") => {
                flatten_use_alias_definitions(
                    &item_use.tree,
                    &mut Vec::new(),
                    package,
                    module,
                    definitions,
                    globs,
                );
            }
            Item::Type(alias) if production(cfg, &alias.attrs, errors, "type alias") => {
                let mut collector = AliasTypePathCollector::default();
                collector.visit_type(&alias.ty);
                definitions.push(GlobalAliasDefinition {
                    package: package.to_owned(),
                    module: module.to_owned(),
                    local_name: normalized_ident(&alias.ident),
                    target_paths: collector.paths,
                });
            }
            Item::Mod(ItemMod {
                attrs,
                ident,
                content: Some((_, child_items)),
                ..
            }) if production(cfg, attrs, errors, "inline module") => {
                collect_global_alias_definitions(
                    package,
                    &format!("{module}::{}", normalized_ident(ident)),
                    child_items,
                    &item_cfg(cfg, attrs),
                    index,
                    definitions,
                    globs,
                    errors,
                );
            }
            _ => {}
        }
    }
}

pub(super) fn build_global_alias_index(
    sources: &[ParsedRegistrySource<'_>],
    errors: &mut Vec<String>,
) -> GlobalAliasIndex {
    let mut index = GlobalAliasIndex::default();
    let mut definitions = Vec::new();
    let mut globs = Vec::new();
    for source in sources {
        let crate_name = source.mount.package.replace('-', "_");
        if let Some(previous) = index
            .package_by_crate_name
            .insert(crate_name.clone(), source.mount.package.to_owned())
            && previous != source.mount.package
        {
            errors.push(format!(
                "registry alias pre-scan has ambiguous crate name {crate_name}: {previous} and {}",
                source.mount.package
            ));
        }
        collect_global_alias_definitions(
            source.mount.package,
            source.mount.module,
            &source.syntax.items,
            &source.cfg,
            &mut index,
            &mut definitions,
            &mut globs,
            errors,
        );
    }

    for _ in 0..=definitions.len() {
        let mut changed = false;
        for definition in &definitions {
            let kinds = definition
                .target_paths
                .iter()
                .flat_map(|path| {
                    index.resolve_symbol_path(&definition.package, &definition.module, path)
                })
                .collect();
            changed |= index.insert_aliases(
                &definition.package,
                &definition.module,
                &definition.local_name,
                kinds,
            );
        }
        if !changed {
            break;
        }
    }

    // Private module splits commonly inherit their parent facade with
    // `use super::*`. Resolve those imports through the same exact alias index
    // instead of rejecting a layout-only change. Iterate to a fixed point so
    // a private descendant can inherit an alias through more than one facade.
    // External registry-capable globs remain rejected by `collect_use_bindings`.
    for _ in 0..=globs.len() {
        let mut changed = false;
        for glob in &globs {
            let Some((target_package, target_module)) =
                index.resolve_module_path(&glob.package, &glob.module, &glob.target_path)
            else {
                continue;
            };
            let Some(aliases) = index.aliases_for(&target_package, &target_module).cloned() else {
                continue;
            };
            for (name, kinds) in aliases {
                changed |= index.insert_aliases(&glob.package, &glob.module, &name, kinds);
            }
        }
        if !changed {
            break;
        }
    }
    index
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) enum FlowStage {
    Registry,
    Guard,
    Iterator,
    Entry,
    Derived,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(super) struct Flow(pub(super) BTreeSet<(RegistryKind, FlowStage)>);

impl Flow {
    pub(super) fn registry(kind: RegistryKind) -> Self {
        Self(BTreeSet::from([(kind, FlowStage::Registry)]))
    }

    pub(super) fn from_kinds(kinds: &KindSet) -> Self {
        Self(
            kinds
                .iter()
                .map(|kind| (*kind, FlowStage::Registry))
                .collect(),
        )
    }

    pub(super) fn union(&mut self, other: Self) {
        self.0.extend(other.0);
    }

    pub(super) fn registry_kinds(&self) -> KindSet {
        self.0
            .iter()
            .filter_map(|(kind, stage)| (*stage == FlowStage::Registry).then_some(*kind))
            .collect()
    }

    pub(super) fn all_kinds(&self) -> KindSet {
        self.0.iter().map(|(kind, _)| *kind).collect()
    }

    pub(super) fn has_registry(&self) -> bool {
        self.0
            .iter()
            .any(|(_, stage)| *stage == FlowStage::Registry)
    }

    pub(super) fn map_registry_stage(&self, stage: FlowStage) -> Self {
        Self(
            self.registry_kinds()
                .into_iter()
                .map(|kind| (kind, stage))
                .collect(),
        )
    }
}

#[derive(Clone, Debug, Default)]
pub(super) struct VariableInfo {
    pub(super) flow: Flow,
    pub(super) struct_types: BTreeSet<String>,
}

#[derive(Clone, Debug)]
pub(super) struct ModuleSymbols {
    pub(super) type_aliases: BTreeMap<String, KindSet>,
    pub(super) field_kinds: BTreeMap<String, KindSet>,
    pub(super) struct_names: BTreeSet<String>,
    pub(super) function_returns: BTreeMap<String, Flow>,
}

impl Default for ModuleSymbols {
    fn default() -> Self {
        let type_aliases = [
            RegistryKind::Player,
            RegistryKind::Group,
            RegistryKind::PendingInvites,
        ]
        .into_iter()
        .map(|kind| (kind.source_name().to_owned(), BTreeSet::from([kind])))
        .collect();
        Self {
            type_aliases,
            field_kinds: BTreeMap::new(),
            struct_names: BTreeSet::new(),
            function_returns: BTreeMap::new(),
        }
    }
}

pub(super) struct TypeKindCollector<'a> {
    pub(super) aliases: &'a BTreeMap<String, KindSet>,
    pub(super) kinds: KindSet,
}

impl<'ast> Visit<'ast> for TypeKindCollector<'_> {
    fn visit_type_path(&mut self, node: &'ast syn::TypePath) {
        for segment in &node.path.segments {
            let name = normalized_ident(&segment.ident);
            if let Some(kinds) = self.aliases.get(&name) {
                self.kinds.extend(kinds);
            }
        }
        visit::visit_type_path(self, node);
    }
}

pub(super) fn registry_kinds_in_type(ty: &Type, symbols: &ModuleSymbols) -> KindSet {
    let mut collector = TypeKindCollector {
        aliases: &symbols.type_aliases,
        kinds: BTreeSet::new(),
    };
    collector.visit_type(ty);
    collector.kinds
}

pub(super) struct StructNameCollector<'a> {
    pub(super) known: &'a BTreeSet<String>,
    pub(super) names: BTreeSet<String>,
}

impl<'ast> Visit<'ast> for StructNameCollector<'_> {
    fn visit_type_path(&mut self, node: &'ast syn::TypePath) {
        for segment in &node.path.segments {
            let name = normalized_ident(&segment.ident);
            if self.known.contains(&name) {
                self.names.insert(name);
            }
        }
        visit::visit_type_path(self, node);
    }
}

pub(super) fn struct_names_in_type(ty: &Type, symbols: &ModuleSymbols) -> BTreeSet<String> {
    let mut collector = StructNameCollector {
        known: &symbols.struct_names,
        names: BTreeSet::new(),
    };
    collector.visit_type(ty);
    collector.names
}

#[derive(Debug)]
pub(super) struct ImportBinding {
    pub(super) local_name: String,
    pub(super) registry: RegistryKind,
}
