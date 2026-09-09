//! Bridge access scan state definitions, part 1 of 3.
//!
//! Separated from the bridge_access.rs root under #660. Behaviour is preserved.

use super::*;

pub(super) const BRIDGE_SCHEMA_VERSION: u32 = 1;

pub(super) const FNV1A_64_PRIME: u64 = 0x0000_0100_0000_01b3;

pub(super) const FNV1A_64_OFFSET_A: u64 = 0xcbf2_9ce4_8422_2325;

pub(super) const FNV1A_64_OFFSET_B: u64 = 0x8422_2325_cbf2_9ce4;

pub(super) const TRANSPARENT_MACROS: &[&str] = &[
    "anyhow",
    "assert",
    "assert_eq",
    "assert_ne",
    "bail",
    "debug",
    "debug_assert",
    "debug_assert_eq",
    "debug_assert_ne",
    "ensure",
    "error",
    "eprintln",
    "format",
    "format_args",
    "info",
    "matches",
    "println",
    "trace",
    "vec",
    "warn",
];

#[derive(Clone, Copy)]
pub(super) struct CuratedAnchor {
    pub(super) package: &'static str,
    pub(super) module: &'static str,
    pub(super) name: &'static str,
    pub(super) direction: BridgeDirection,
}

// Exact function/method anchors from runtime-ownership-ledger.json. The loot
// bridge is module-shaped rather than one function and is handled separately.
pub(super) const CURATED_ANCHORS: &[CuratedAnchor] = &[
    CuratedAnchor {
        package: "world-server",
        module: "crate::runtime::game_events",
        name: "mirror_loaded_grid_creature_to_legacy_like_cpp",
        direction: BridgeDirection::CanonicalToLegacy,
    },
    CuratedAnchor {
        package: "world-server",
        module: "crate::runtime::delivery",
        name: "run_legacy_creature_movement_tick_and_deliver_once_like_cpp",
        direction: BridgeDirection::LegacyToCanonical,
    },
    CuratedAnchor {
        package: "world-server",
        module: "crate::runtime::delivery",
        name: "run_legacy_creature_aggro_tick_and_deliver_once_like_cpp",
        direction: BridgeDirection::LegacyToCanonical,
    },
    CuratedAnchor {
        package: "world-server",
        module: "crate::runtime::delivery",
        name: "run_legacy_creature_melee_tick_and_deliver_once_like_cpp",
        direction: BridgeDirection::LegacyToCanonical,
    },
    CuratedAnchor {
        package: "world-server",
        module: "crate::runtime::delivery",
        name: "run_legacy_creature_spell_tick_and_deliver_once_like_cpp",
        direction: BridgeDirection::LegacyToCanonical,
    },
    CuratedAnchor {
        package: "world-server",
        module: "crate::runtime::delivery",
        name: "run_legacy_creature_runtime_tick_and_deliver_once_like_cpp",
        direction: BridgeDirection::LegacyToCanonical,
    },
];

/// One authority surface referenced by a bridge item.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum BridgeSide {
    Canonical,
    Legacy,
    RepresentedSession,
}

/// Direction is asserted only by a curated anchor/module. Uncurated syntax
/// containing both sides is intentionally reported as unresolved.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum BridgeDirection {
    CanonicalToLegacy,
    LegacyToCanonical,
    RepresentedSessionToCanonical,
    DualAuthorityCompatibility,
    UnresolvedDualSide,
}

/// Kind of AST evidence that made one side visible.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum BridgeEvidenceKind {
    TypeReference,
    ValueReference,
    FieldAccess,
    FunctionCall,
    MethodCall,
    SelfType,
    MacroArgument,
    CuratedAnchorDefinition,
}

/// Canonicalized evidence within one enclosing item. Identical occurrences
/// retain an explicit multiplicity rather than disappearing in a set.
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct BridgeEvidenceMarker {
    pub(crate) side: BridgeSide,
    pub(crate) kind: BridgeEvidenceKind,
    pub(crate) symbol: String,
    pub(crate) fingerprint: String,
    pub(crate) multiplicity: usize,
}

/// One exact bridge-bearing Rust item.
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct BridgeAccessRecord {
    pub(crate) package: String,
    pub(crate) module: String,
    pub(crate) path: String,
    pub(crate) enclosing: String,
    pub(crate) direction_markers: Vec<BridgeDirection>,
    pub(crate) evidence: Vec<BridgeEvidenceMarker>,
    pub(crate) fingerprint: String,
    pub(crate) cfg: Vec<String>,
    pub(crate) multiplicity: usize,
}

/// Sorted, machine-readable exact bridge snapshot.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct BridgeAccessBaseline {
    pub(crate) schema_version: u32,
    pub(crate) bridges: Vec<BridgeAccessRecord>,
}

/// One source mount already resolved by the repository module walker.
#[derive(Clone, Copy, Debug)]
pub(crate) struct BridgeSource<'a> {
    pub(crate) package: &'a str,
    pub(crate) module: &'a str,
    pub(crate) source_path: &'a str,
    pub(crate) inherited_cfg: &'a [String],
    pub(crate) source: &'a str,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) struct EvidenceIdentity {
    pub(super) side: BridgeSide,
    pub(super) kind: BridgeEvidenceKind,
    pub(super) symbol: String,
    pub(super) fingerprint: String,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) struct EvidenceGroupIdentity {
    pub(super) side: BridgeSide,
    pub(super) kind: BridgeEvidenceKind,
    pub(super) symbol: String,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) struct BridgeIdentity {
    pub(super) package: String,
    pub(super) module: String,
    pub(super) path: String,
    pub(super) enclosing: String,
    pub(super) direction_markers: Vec<BridgeDirection>,
    pub(super) evidence: Vec<BridgeEvidenceMarker>,
    pub(super) fingerprint: String,
    pub(super) cfg: Vec<String>,
}

impl BridgeAccessRecord {
    pub(super) fn identity(&self) -> BridgeIdentity {
        BridgeIdentity {
            package: self.package.clone(),
            module: self.module.clone(),
            path: self.path.clone(),
            enclosing: self.enclosing.clone(),
            direction_markers: self.direction_markers.clone(),
            evidence: self.evidence.clone(),
            fingerprint: self.fingerprint.clone(),
            cfg: self.cfg.clone(),
        }
    }
}

#[derive(Default)]
pub(super) struct BridgeAccumulator {
    pub(super) rows: BTreeMap<BridgeIdentity, usize>,
}

impl BridgeAccumulator {
    pub(super) fn add(&mut self, record: BridgeAccessRecord) {
        let identity = record.identity();
        *self.rows.entry(identity).or_default() += record.multiplicity;
    }

    pub(super) fn finish(self) -> BridgeAccessBaseline {
        BridgeAccessBaseline {
            schema_version: BRIDGE_SCHEMA_VERSION,
            bridges: self
                .rows
                .into_iter()
                .map(|(identity, multiplicity)| BridgeAccessRecord {
                    package: identity.package,
                    module: identity.module,
                    path: identity.path,
                    enclosing: identity.enclosing,
                    direction_markers: identity.direction_markers,
                    evidence: identity.evidence,
                    fingerprint: identity.fingerprint,
                    cfg: identity.cfg,
                    multiplicity,
                })
                .collect(),
        }
    }
}

#[derive(Clone, Default)]
pub(super) struct Symbols {
    pub(super) named: BTreeMap<String, BTreeSet<BridgeSide>>,
}

impl Symbols {
    pub(super) fn for_module(package: &str, module: &str) -> Self {
        let mut symbols = Self::default();
        symbols.add("SharedCanonicalMapManager", [BridgeSide::Canonical]);
        symbols.add("LegacyMapManager", [BridgeSide::Legacy]);
        symbols.add("SharedMapManager", [BridgeSide::Legacy]);
        if package == "wow-world" && is_legacy_map_module(module) {
            symbols.add("MapManager", [BridgeSide::Legacy]);
            symbols.add("WorldCreature", [BridgeSide::Legacy]);
        }
        symbols
    }

    pub(super) fn add<I>(&mut self, name: impl Into<String>, sides: I)
    where
        I: IntoIterator<Item = BridgeSide>,
    {
        self.named.entry(name.into()).or_default().extend(sides);
    }

    pub(super) fn sides_for_ident(&self, name: &str) -> BTreeSet<BridgeSide> {
        self.named.get(name).cloned().unwrap_or_default()
    }

    pub(super) fn sides_for_path(&self, path: &Path) -> BTreeSet<BridgeSide> {
        let segments: Vec<_> = path
            .segments
            .iter()
            .map(|segment| segment.ident.to_string())
            .collect();
        sides_for_segments(self, &segments)
    }
}

#[derive(Clone)]
pub(super) struct ModuleContext<'a> {
    pub(super) package: &'a str,
    pub(super) module: String,
    pub(super) path: &'a str,
    pub(super) cfg: Vec<String>,
}

#[derive(Clone)]
pub(super) struct RawEvidence {
    pub(super) side: BridgeSide,
    pub(super) kind: BridgeEvidenceKind,
    pub(super) symbol: String,
    pub(super) fingerprint: String,
}

pub(super) fn normalized_tokens(value: &impl ToTokens) -> String {
    value.to_token_stream().to_string()
}

/// Stable, dependency-free two-lane FNV-1a fingerprint. One lane reads the
/// normalized syntax forward and the other backward with an independent
/// offset basis. The byte length is retained to make diagnostics and accidental
/// truncation visible. This is a drift fingerprint, not a security primitive.
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

pub(super) fn last_path_ident(path: &Path) -> Option<String> {
    path.segments
        .last()
        .map(|segment| segment.ident.to_string())
}

pub(super) fn is_legacy_map_module(module: &str) -> bool {
    module == "crate::map_manager" || module.starts_with("crate::map_manager::")
}

pub(super) fn is_loot_compatibility_module(package: &str, module: &str) -> bool {
    package == "wow-world"
        && (module == "crate::handlers::loot" || module.starts_with("crate::handlers::loot::"))
}

pub(super) fn curated_definition(
    package: &str,
    module: &str,
    name: &str,
) -> Option<&'static CuratedAnchor> {
    CURATED_ANCHORS
        .iter()
        .find(|anchor| anchor.package == package && anchor.module == module && anchor.name == name)
}

pub(super) fn direction_sides(direction: BridgeDirection) -> &'static [BridgeSide] {
    match direction {
        BridgeDirection::CanonicalToLegacy
        | BridgeDirection::LegacyToCanonical
        | BridgeDirection::DualAuthorityCompatibility
        | BridgeDirection::UnresolvedDualSide => &[BridgeSide::Canonical, BridgeSide::Legacy],
        BridgeDirection::RepresentedSessionToCanonical => {
            &[BridgeSide::RepresentedSession, BridgeSide::Canonical]
        }
    }
}

pub(super) fn sides_for_segments(symbols: &Symbols, segments: &[String]) -> BTreeSet<BridgeSide> {
    let mut sides = BTreeSet::new();
    let Some(first) = segments.first().map(String::as_str) else {
        return sides;
    };

    // Authority provenance is deliberately narrower than crate provenance:
    // using a coordinate, key, entity, or DTO from an authority-owned crate
    // does not itself mean the enclosing item owns or bridges that authority.
    let canonical_map_authority = segments
        .get(1)
        .is_some_and(|segment| matches!(segment.as_str(), "MapManager" | "ManagedMap" | "Map"))
        || (segments
            .get(1)
            .is_some_and(|segment| matches!(segment.as_str(), "manager" | "map"))
            && segments.get(2).is_some_and(|segment| {
                matches!(segment.as_str(), "MapManager" | "ManagedMap" | "Map")
            }));
    if first == "wow_map" && canonical_map_authority {
        sides.insert(BridgeSide::Canonical);
    }
    if first == "wow_entities" && segments.get(1).is_some_and(|segment| segment == "Creature") {
        sides.insert(BridgeSide::Canonical);
    }
    if segments
        .iter()
        .any(|segment| segment == "SharedCanonicalMapManager")
    {
        sides.insert(BridgeSide::Canonical);
    }
    let is_legacy_authority = |segment: &str| {
        matches!(
            segment,
            "MapManager" | "SharedMapManager" | "LegacyMapManager" | "WorldCreature"
        )
    };
    let explicit_wow_world_authority = segments
        .get(1)
        .is_some_and(|segment| is_legacy_authority(segment))
        || (segments
            .get(1)
            .is_some_and(|segment| segment == "map_manager")
            && segments
                .get(2)
                .is_some_and(|segment| is_legacy_authority(segment)));
    if first == "wow_world" && explicit_wow_world_authority {
        sides.insert(BridgeSide::Legacy);
    }
    if matches!(first, "crate" | "self" | "super")
        && segments
            .windows(2)
            .any(|window| window[0] == "map_manager" && is_legacy_authority(window[1].as_str()))
    {
        sides.insert(BridgeSide::Legacy);
    }
    // Imported type aliases and associated constructors are rooted in the
    // first segment. Looking up the last segment confuses enum variants such
    // as `SpawnObjectType::Creature` with an unrelated imported type named
    // `Creature`.
    sides.extend(symbols.sides_for_ident(first));
    sides
}

pub(super) fn bridge_capable_glob_prefix(segments: &[String]) -> bool {
    segments
        .first()
        .is_some_and(|segment| matches!(segment.as_str(), "wow_entities" | "wow_map" | "wow_world"))
        || segments.iter().any(|segment| {
            matches!(
                segment.as_str(),
                "map_manager"
                    | "SharedCanonicalMapManager"
                    | "SharedMapManager"
                    | "LegacyMapManager"
            )
        })
}

pub(super) fn bridge_capable_namespace_import(segments: &[String]) -> bool {
    let Some(first) = segments.first().map(String::as_str) else {
        return false;
    };
    match first {
        "wow_entities" => segments.len() == 1,
        "wow_map" => {
            segments.len() == 1
                || segments
                    .last()
                    .is_some_and(|segment| matches!(segment.as_str(), "manager" | "map"))
        }
        "wow_world" => {
            segments.len() == 1
                || segments
                    .last()
                    .is_some_and(|segment| segment == "map_manager")
        }
        "crate" | "self" | "super" => segments
            .last()
            .is_some_and(|segment| segment == "map_manager"),
        _ => false,
    }
}

pub(super) fn token_sides(
    tokens: &proc_macro2::TokenStream,
    symbols: &Symbols,
    variables: &BTreeMap<String, BTreeSet<BridgeSide>>,
) -> BTreeSet<BridgeSide> {
    fn visit(
        tokens: proc_macro2::TokenStream,
        variables: &BTreeMap<String, BTreeSet<BridgeSide>>,
        sides: &mut BTreeSet<BridgeSide>,
    ) {
        for token in tokens {
            match token {
                proc_macro2::TokenTree::Group(group) => {
                    visit(group.stream(), variables, sides);
                }
                proc_macro2::TokenTree::Ident(ident) => {
                    let name = ident.to_string();
                    if let Some(variable_sides) = variables.get(&name) {
                        sides.extend(variable_sides);
                    }
                }
                proc_macro2::TokenTree::Punct(_) | proc_macro2::TokenTree::Literal(_) => {}
            }
        }
    }

    fn visit_paths(
        tokens: proc_macro2::TokenStream,
        symbols: &Symbols,
        sides: &mut BTreeSet<BridgeSide>,
    ) {
        let trees: Vec<_> = tokens.into_iter().collect();
        for tree in &trees {
            if let proc_macro2::TokenTree::Group(group) = tree {
                visit_paths(group.stream(), symbols, sides);
            }
        }
        for start in 0..trees.len() {
            // Only resolve maximal paths. Resolving every suffix would turn
            // `SpawnObjectType::Creature` into the unrelated imported type
            // `Creature` when scanning a macro token stream.
            if start >= 2
                && matches!(&trees[start - 2], proc_macro2::TokenTree::Punct(value) if value.as_char() == ':')
                && matches!(&trees[start - 1], proc_macro2::TokenTree::Punct(value) if value.as_char() == ':')
            {
                continue;
            }
            let proc_macro2::TokenTree::Ident(first) = &trees[start] else {
                continue;
            };
            let mut segments = vec![first.to_string()];
            let mut cursor = start + 1;
            while cursor + 2 < trees.len()
                && matches!(&trees[cursor], proc_macro2::TokenTree::Punct(value) if value.as_char() == ':')
                && matches!(&trees[cursor + 1], proc_macro2::TokenTree::Punct(value) if value.as_char() == ':')
            {
                let proc_macro2::TokenTree::Ident(next) = &trees[cursor + 2] else {
                    break;
                };
                segments.push(next.to_string());
                cursor += 3;
            }
            sides.extend(sides_for_segments(symbols, &segments));
        }
    }

    let mut sides = BTreeSet::new();
    visit(tokens.clone(), variables, &mut sides);
    visit_paths(tokens.clone(), symbols, &mut sides);
    sides
}

pub(super) fn tokens_mention_curated_anchor(tokens: &proc_macro2::TokenStream) -> bool {
    fn contains(tokens: proc_macro2::TokenStream, expected: &str) -> bool {
        tokens.into_iter().any(|token| match token {
            proc_macro2::TokenTree::Group(group) => contains(group.stream(), expected),
            proc_macro2::TokenTree::Ident(ident) => ident == expected,
            proc_macro2::TokenTree::Punct(_) | proc_macro2::TokenTree::Literal(_) => false,
        })
    }

    CURATED_ANCHORS
        .iter()
        .any(|anchor| contains(tokens.clone(), anchor.name))
}

pub(super) fn bridge_shaped_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.contains("bridge")
        || lower.contains("mirror")
        || lower.contains("canonical_to_legacy")
        || lower.contains("legacy_to_canonical")
}

pub(super) fn transparent_macro(name: &str) -> bool {
    TRANSPARENT_MACROS.contains(&name)
}

pub(super) fn validate_cfg(
    inherited: &[String],
    attributes: &[Attribute],
    label: &str,
    errors: &mut Vec<String>,
) {
    if let Err(error) = cfg_context_allows_production(inherited, attributes) {
        errors.push(format!("invalid cfg for {label}: {error}"));
    }
    if let Err(error) = cfg_context_allows_test(inherited, attributes) {
        errors.push(format!("invalid test cfg for {label}: {error}"));
    }
}

pub(super) fn collect_use_bindings(
    tree: &UseTree,
    prefix: &mut Vec<String>,
    bindings: &mut Vec<(String, Vec<String>)>,
    globs: &mut Vec<Vec<String>>,
) {
    match tree {
        UseTree::Path(path) => {
            prefix.push(path.ident.to_string());
            collect_use_bindings(&path.tree, prefix, bindings, globs);
            prefix.pop();
        }
        UseTree::Name(name) => {
            if name.ident == "self" {
                if let Some(local) = prefix.last() {
                    bindings.push((local.clone(), prefix.clone()));
                }
            } else {
                let mut full = prefix.clone();
                full.push(name.ident.to_string());
                bindings.push((name.ident.to_string(), full));
            }
        }
        UseTree::Rename(rename) => {
            let mut full = prefix.clone();
            full.push(rename.ident.to_string());
            bindings.push((rename.rename.to_string(), full));
        }
        UseTree::Group(group) => {
            for item in &group.items {
                collect_use_bindings(item, prefix, bindings, globs);
            }
        }
        UseTree::Glob(_) => globs.push(prefix.clone()),
    }
}

pub(super) fn add_use_to_symbols(
    item_use: &syn::ItemUse,
    symbols: &mut Symbols,
    errors: &mut Vec<String>,
) {
    let mut bindings = Vec::new();
    let mut globs = Vec::new();
    collect_use_bindings(&item_use.tree, &mut Vec::new(), &mut bindings, &mut globs);
    for (local, full) in bindings {
        let sides = sides_for_segments(symbols, &full);
        if sides.is_empty()
            && bridge_capable_namespace_import(&full)
            && !(full.len() == 1 && local == full[0])
        {
            errors.push(format!(
                "bridge-capable namespace import `{}` as `{local}` hides exact legacy/canonical symbols",
                full.join("::")
            ));
        }
        symbols.add(local, sides);
    }
    for glob in globs {
        let sides = sides_for_segments(symbols, &glob);
        if !sides.is_empty() || bridge_capable_glob_prefix(&glob) {
            errors.push(format!(
                "bridge-capable glob import `{}` hides exact legacy/canonical symbols",
                glob.join("::")
            ));
        }
    }
}

pub(super) struct TypeSideCollector<'a> {
    pub(super) symbols: &'a Symbols,
    pub(super) sides: BTreeSet<BridgeSide>,
}

impl<'ast> Visit<'ast> for TypeSideCollector<'_> {
    fn visit_type_path(&mut self, path: &'ast syn::TypePath) {
        self.sides.extend(self.symbols.sides_for_path(&path.path));
        visit::visit_type_path(self, path);
    }
}

pub(super) fn sides_in_type(symbols: &Symbols, type_expression: &Type) -> BTreeSet<BridgeSide> {
    let mut collector = TypeSideCollector {
        symbols,
        sides: BTreeSet::new(),
    };
    collector.visit_type(type_expression);
    collector.sides
}

pub(super) fn register_module_symbols(
    items: &[Item],
    context: &ModuleContext<'_>,
    inherited: &Symbols,
    errors: &mut Vec<String>,
) -> Symbols {
    let mut symbols = inherited.clone();
    let builtins = Symbols::for_module(context.package, &context.module);
    for (name, sides) in builtins.named {
        symbols.add(name, sides);
    }
    for item in items {
        if let Item::Use(item_use) = item {
            add_use_to_symbols(item_use, &mut symbols, errors);
        }
    }

    // A few passes resolve explicit type-alias chains without pretending that
    // every struct/function touching an authority type inherits that
    // authority. The latter poisoned generic symbols such as WorldSession and
    // then propagated false provenance through `Self` and ordinary calls.
    for _ in 0..3 {
        for item in items {
            if let Item::Type(alias) = item {
                symbols.add(alias.ident.to_string(), sides_in_type(&symbols, &alias.ty));
            }
        }
    }
    symbols
}

pub(super) struct CandidateAnalyzer<'a> {
    pub(super) context: &'a ModuleContext<'a>,
    pub(super) enclosing: String,
    pub(super) symbols: Symbols,
    pub(super) variables: BTreeMap<String, BTreeSet<BridgeSide>>,
    pub(super) evidence: Vec<RawEvidence>,
    pub(super) directions: BTreeSet<BridgeDirection>,
    pub(super) opaque_authority_macros: Vec<String>,
    pub(super) errors: &'a mut Vec<String>,
}
