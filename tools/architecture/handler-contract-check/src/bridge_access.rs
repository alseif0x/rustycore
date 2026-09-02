// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Exact syntax inventory for transitional legacy↔canonical bridges.
//!
//! A bridge is not identified from an enclosing function name alone. The
//! inventory resolves imports, aliases, types, typed values, known session
//! fields, calls, and method receivers and requires evidence from both the
//! canonical map/entity authority and legacy map/runtime authority sides.
//! Ordinary DTOs and state enums are not authority evidence by themselves; the
//! exact canonical mutable `wow_entities::Creature` is. The small set of
//! direction-bearing anchors curated in `runtime-ownership-ledger.json` is an
//! explicit exception: those definitions remain visible even when one side is
//! represented implicitly by `WorldSession`. Calling an anchor does not make
//! every caller a new bridge definition.
//!
//! This is a deliberately strict source guard rather than a Rust type checker.
//! Macros whose visible tokens contain both sides, a curated anchor, or a
//! bridge-shaped name must either be one of the transparent diagnostic/data
//! macros understood here or fail closed. `include!` source resolution remains
//! the caller's module-graph/generated-input responsibility; every other
//! item-generating bridge macro fails here. The checked-in comparator is an
//! exact set and multiplicity comparison, so a same-count swap cannot pass.

use std::collections::{BTreeMap, BTreeSet};

use quote::ToTokens;
use serde::{Deserialize, Serialize};
use syn::visit::{self, Visit};
use syn::{
    Attribute, Expr, ExprCall, ExprField, ExprMacro, ExprMethodCall, FnArg, ImplItem, Item, ItemFn,
    ItemImpl, ItemMacro, ItemMod, Local, Member, Pat, Path, Signature, Type, UseTree,
};

use crate::ownership::{
    cfg_context_allows_production, cfg_context_allows_test, extend_cfg_context,
};

const BRIDGE_SCHEMA_VERSION: u32 = 1;

const FNV1A_64_PRIME: u64 = 0x0000_0100_0000_01b3;
const FNV1A_64_OFFSET_A: u64 = 0xcbf2_9ce4_8422_2325;
const FNV1A_64_OFFSET_B: u64 = 0x8422_2325_cbf2_9ce4;

const TRANSPARENT_MACROS: &[&str] = &[
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
struct CuratedAnchor {
    package: &'static str,
    module: &'static str,
    name: &'static str,
    direction: BridgeDirection,
}

// Exact function/method anchors from runtime-ownership-ledger.json. The loot
// bridge is module-shaped rather than one function and is handled separately.
const CURATED_ANCHORS: &[CuratedAnchor] = &[
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
struct EvidenceIdentity {
    side: BridgeSide,
    kind: BridgeEvidenceKind,
    symbol: String,
    fingerprint: String,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct EvidenceGroupIdentity {
    side: BridgeSide,
    kind: BridgeEvidenceKind,
    symbol: String,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct BridgeIdentity {
    package: String,
    module: String,
    path: String,
    enclosing: String,
    direction_markers: Vec<BridgeDirection>,
    evidence: Vec<BridgeEvidenceMarker>,
    fingerprint: String,
    cfg: Vec<String>,
}

impl BridgeAccessRecord {
    fn identity(&self) -> BridgeIdentity {
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
struct BridgeAccumulator {
    rows: BTreeMap<BridgeIdentity, usize>,
}

impl BridgeAccumulator {
    fn add(&mut self, record: BridgeAccessRecord) {
        let identity = record.identity();
        *self.rows.entry(identity).or_default() += record.multiplicity;
    }

    fn finish(self) -> BridgeAccessBaseline {
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
struct Symbols {
    named: BTreeMap<String, BTreeSet<BridgeSide>>,
}

impl Symbols {
    fn for_module(package: &str, module: &str) -> Self {
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

    fn add<I>(&mut self, name: impl Into<String>, sides: I)
    where
        I: IntoIterator<Item = BridgeSide>,
    {
        self.named.entry(name.into()).or_default().extend(sides);
    }

    fn sides_for_ident(&self, name: &str) -> BTreeSet<BridgeSide> {
        self.named.get(name).cloned().unwrap_or_default()
    }

    fn sides_for_path(&self, path: &Path) -> BTreeSet<BridgeSide> {
        let segments: Vec<_> = path
            .segments
            .iter()
            .map(|segment| segment.ident.to_string())
            .collect();
        sides_for_segments(self, &segments)
    }
}

#[derive(Clone)]
struct ModuleContext<'a> {
    package: &'a str,
    module: String,
    path: &'a str,
    cfg: Vec<String>,
}

#[derive(Clone)]
struct RawEvidence {
    side: BridgeSide,
    kind: BridgeEvidenceKind,
    symbol: String,
    fingerprint: String,
}

fn normalized_tokens(value: &impl ToTokens) -> String {
    value.to_token_stream().to_string()
}

/// Stable, dependency-free two-lane FNV-1a fingerprint. One lane reads the
/// normalized syntax forward and the other backward with an independent
/// offset basis. The byte length is retained to make diagnostics and accidental
/// truncation visible. This is a drift fingerprint, not a security primitive.
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

fn last_path_ident(path: &Path) -> Option<String> {
    path.segments
        .last()
        .map(|segment| segment.ident.to_string())
}

fn is_legacy_map_module(module: &str) -> bool {
    module == "crate::map_manager" || module.starts_with("crate::map_manager::")
}

fn is_loot_compatibility_module(package: &str, module: &str) -> bool {
    package == "wow-world"
        && (module == "crate::handlers::loot" || module.starts_with("crate::handlers::loot::"))
}

fn curated_definition(package: &str, module: &str, name: &str) -> Option<&'static CuratedAnchor> {
    CURATED_ANCHORS
        .iter()
        .find(|anchor| anchor.package == package && anchor.module == module && anchor.name == name)
}

fn direction_sides(direction: BridgeDirection) -> &'static [BridgeSide] {
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

fn sides_for_segments(symbols: &Symbols, segments: &[String]) -> BTreeSet<BridgeSide> {
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

fn bridge_capable_glob_prefix(segments: &[String]) -> bool {
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

fn bridge_capable_namespace_import(segments: &[String]) -> bool {
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

fn token_sides(
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

fn tokens_mention_curated_anchor(tokens: &proc_macro2::TokenStream) -> bool {
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

fn bridge_shaped_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.contains("bridge")
        || lower.contains("mirror")
        || lower.contains("canonical_to_legacy")
        || lower.contains("legacy_to_canonical")
}

fn transparent_macro(name: &str) -> bool {
    TRANSPARENT_MACROS.contains(&name)
}

fn validate_cfg(
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

fn collect_use_bindings(
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

fn add_use_to_symbols(item_use: &syn::ItemUse, symbols: &mut Symbols, errors: &mut Vec<String>) {
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

struct TypeSideCollector<'a> {
    symbols: &'a Symbols,
    sides: BTreeSet<BridgeSide>,
}

impl<'ast> Visit<'ast> for TypeSideCollector<'_> {
    fn visit_type_path(&mut self, path: &'ast syn::TypePath) {
        self.sides.extend(self.symbols.sides_for_path(&path.path));
        visit::visit_type_path(self, path);
    }
}

fn sides_in_type(symbols: &Symbols, type_expression: &Type) -> BTreeSet<BridgeSide> {
    let mut collector = TypeSideCollector {
        symbols,
        sides: BTreeSet::new(),
    };
    collector.visit_type(type_expression);
    collector.sides
}

fn register_module_symbols(
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

struct CandidateAnalyzer<'a> {
    context: &'a ModuleContext<'a>,
    enclosing: String,
    symbols: Symbols,
    variables: BTreeMap<String, BTreeSet<BridgeSide>>,
    evidence: Vec<RawEvidence>,
    directions: BTreeSet<BridgeDirection>,
    opaque_authority_macros: Vec<String>,
    errors: &'a mut Vec<String>,
}

impl<'a> CandidateAnalyzer<'a> {
    fn new(
        context: &'a ModuleContext<'a>,
        enclosing: String,
        symbols: &Symbols,
        errors: &'a mut Vec<String>,
    ) -> Self {
        Self {
            context,
            enclosing,
            symbols: symbols.clone(),
            variables: BTreeMap::new(),
            evidence: Vec::new(),
            directions: BTreeSet::new(),
            opaque_authority_macros: Vec::new(),
            errors,
        }
    }

    fn add(
        &mut self,
        side: BridgeSide,
        kind: BridgeEvidenceKind,
        symbol: impl Into<String>,
        fingerprint: impl Into<String>,
    ) {
        let fingerprint = fingerprint.into();
        self.evidence.push(RawEvidence {
            side,
            kind,
            symbol: symbol.into(),
            fingerprint: compact_fingerprint(&fingerprint),
        });
    }

    fn add_sides(
        &mut self,
        sides: &BTreeSet<BridgeSide>,
        kind: BridgeEvidenceKind,
        symbol: impl Into<String>,
        fingerprint: impl Into<String>,
    ) {
        let symbol = symbol.into();
        let fingerprint = fingerprint.into();
        for side in sides {
            self.add(*side, kind, symbol.clone(), fingerprint.clone());
        }
    }

    fn seed_anchor(
        &mut self,
        anchor: &CuratedAnchor,
        kind: BridgeEvidenceKind,
        fingerprint: String,
    ) {
        self.directions.insert(anchor.direction);
        for side in direction_sides(anchor.direction) {
            self.add(*side, kind, anchor.name, fingerprint.clone());
        }
    }

    fn seed_definition_anchor(&mut self, name: &str) {
        if let Some(anchor) = curated_definition(self.context.package, &self.context.module, name) {
            self.seed_anchor(
                anchor,
                BridgeEvidenceKind::CuratedAnchorDefinition,
                format!("definition:{name}"),
            );
        }
    }

    fn bind_pattern(&mut self, pattern: &Pat, sides: &BTreeSet<BridgeSide>) {
        match pattern {
            Pat::Ident(identifier) => {
                self.variables
                    .insert(identifier.ident.to_string(), sides.clone());
                if let Some((_, subpattern)) = &identifier.subpat {
                    self.bind_pattern(subpattern, sides);
                }
            }
            Pat::Type(typed) => {
                let typed_sides = sides_in_type(&self.symbols, &typed.ty);
                let bound = if typed_sides.is_empty() {
                    sides.clone()
                } else {
                    typed_sides
                };
                self.bind_pattern(&typed.pat, &bound);
            }
            Pat::Reference(reference) => self.bind_pattern(&reference.pat, sides),
            Pat::Paren(paren) => self.bind_pattern(&paren.pat, sides),
            Pat::Tuple(tuple) => {
                for element in &tuple.elems {
                    self.bind_pattern(element, sides);
                }
            }
            Pat::TupleStruct(tuple) => {
                for element in &tuple.elems {
                    self.bind_pattern(element, sides);
                }
            }
            Pat::Struct(structure) => {
                for field in &structure.fields {
                    self.bind_pattern(&field.pat, sides);
                }
            }
            Pat::Slice(slice) => {
                for element in &slice.elems {
                    self.bind_pattern(element, sides);
                }
            }
            Pat::Or(or) => {
                for case in &or.cases {
                    self.bind_pattern(case, sides);
                }
            }
            _ => {}
        }
    }

    fn bind_signature(&mut self, signature: &Signature) {
        for input in &signature.inputs {
            match input {
                FnArg::Receiver(_) => {}
                FnArg::Typed(typed) => {
                    let sides = sides_in_type(&self.symbols, &typed.ty);
                    self.bind_pattern(&typed.pat, &sides);
                }
            }
        }
    }

    fn sides_of_expr(&self, expression: &Expr) -> BTreeSet<BridgeSide> {
        match expression {
            Expr::Path(path) => {
                let mut sides = self.symbols.sides_for_path(&path.path);
                if let Some(name) = last_path_ident(&path.path) {
                    if let Some(variable_sides) = self.variables.get(&name) {
                        sides.extend(variable_sides);
                    }
                }
                sides
            }
            Expr::Field(field) => {
                let mut sides = self.sides_of_expr(&field.base);
                if let Member::Named(member) = &field.member {
                    match member.to_string().as_str() {
                        "canonical_map_manager" => {
                            sides.insert(BridgeSide::Canonical);
                        }
                        "map_manager" => {
                            sides.insert(BridgeSide::Legacy);
                        }
                        _ => {}
                    }
                }
                sides
            }
            Expr::MethodCall(call) => self.sides_of_expr(&call.receiver),
            Expr::Reference(reference) => self.sides_of_expr(&reference.expr),
            Expr::Paren(paren) => self.sides_of_expr(&paren.expr),
            Expr::Group(group) => self.sides_of_expr(&group.expr),
            Expr::Try(tried) => self.sides_of_expr(&tried.expr),
            Expr::Await(awaited) => self.sides_of_expr(&awaited.base),
            Expr::Index(index) => self.sides_of_expr(&index.expr),
            Expr::Call(call) => {
                let mut sides = BTreeSet::new();
                if let Expr::Path(path) = call.func.as_ref() {
                    sides.extend(self.symbols.sides_for_path(&path.path));
                    let passthrough = last_path_ident(&path.path)
                        .is_some_and(|name| matches!(name.as_str(), "clone" | "from"));
                    if passthrough {
                        for argument in &call.args {
                            sides.extend(self.sides_of_expr(argument));
                        }
                    }
                }
                sides
            }
            _ => BTreeSet::new(),
        }
    }

    fn audit_macro(&mut self, mac: &syn::Macro, label: &str) {
        let name = last_path_ident(&mac.path).unwrap_or_else(|| "<macro>".to_owned());
        let sides = token_sides(&mac.tokens, &self.symbols, &self.variables);
        let has_both =
            sides.contains(&BridgeSide::Canonical) && sides.contains(&BridgeSide::Legacy);
        let mentions_anchor = tokens_mention_curated_anchor(&mac.tokens);
        if mac.path.is_ident("macro_rules") && !sides.is_empty() {
            self.errors.push(format!(
                "{label} {name}! in {} can hide authority references in generated syntax",
                self.enclosing
            ));
            return;
        }
        if (has_both || mentions_anchor || bridge_shaped_name(&name)) && !transparent_macro(&name) {
            self.errors.push(format!(
                "unknown {label} {name}! in {} can hide a legacy/canonical bridge",
                self.enclosing
            ));
            return;
        }
        if !sides.is_empty() {
            let fingerprint = normalized_tokens(&mac.tokens);
            self.add_sides(
                &sides,
                BridgeEvidenceKind::MacroArgument,
                name.clone(),
                fingerprint,
            );
            if !transparent_macro(&name) {
                self.opaque_authority_macros
                    .push(format!("{label} {name}!"));
            }
        }
    }

    fn finish(mut self, header: String, cfg: Vec<String>) -> Option<BridgeAccessRecord> {
        let sides: BTreeSet<_> = self.evidence.iter().map(|evidence| evidence.side).collect();
        let structural =
            sides.contains(&BridgeSide::Canonical) && sides.contains(&BridgeSide::Legacy);
        if !structural && self.directions.is_empty() {
            return None;
        }
        if !self.opaque_authority_macros.is_empty() {
            self.opaque_authority_macros.sort();
            self.opaque_authority_macros.dedup();
            self.errors.push(format!(
                "unknown {} in {} can hide part of a legacy/canonical bridge",
                self.opaque_authority_macros.join(", "),
                self.enclosing
            ));
            return None;
        }
        if self.directions.is_empty() {
            self.directions.insert(
                if is_loot_compatibility_module(self.context.package, &self.context.module) {
                    BridgeDirection::DualAuthorityCompatibility
                } else {
                    BridgeDirection::UnresolvedDualSide
                },
            );
        }

        let ordered_evidence = self
            .evidence
            .iter()
            .map(|evidence| {
                format!(
                    "{:?}:{:?}:{}:{}",
                    evidence.side, evidence.kind, evidence.symbol, evidence.fingerprint
                )
            })
            .collect::<Vec<_>>()
            .join("\u{1f}");
        let fingerprint = format!(
            "signature={header}|evidence={}",
            compact_fingerprint(&ordered_evidence)
        );

        let mut grouped = BTreeMap::<EvidenceGroupIdentity, Vec<String>>::new();
        for evidence in self.evidence {
            let identity = EvidenceGroupIdentity {
                side: evidence.side,
                kind: evidence.kind,
                symbol: evidence.symbol,
            };
            grouped
                .entry(identity)
                .or_default()
                .push(evidence.fingerprint);
        }
        let evidence = grouped
            .into_iter()
            .map(|(identity, occurrences)| BridgeEvidenceMarker {
                side: identity.side,
                kind: identity.kind,
                symbol: identity.symbol,
                fingerprint: compact_fingerprint(&occurrences.join("\u{1f}")),
                multiplicity: occurrences.len(),
            })
            .collect();
        Some(BridgeAccessRecord {
            package: self.context.package.to_owned(),
            module: self.context.module.clone(),
            path: self.context.path.to_owned(),
            enclosing: self.enclosing,
            direction_markers: self.directions.into_iter().collect(),
            evidence,
            fingerprint,
            cfg,
            multiplicity: 1,
        })
    }
}

impl<'ast> Visit<'ast> for CandidateAnalyzer<'_> {
    fn visit_type_path(&mut self, path: &'ast syn::TypePath) {
        let sides = self.symbols.sides_for_path(&path.path);
        if !sides.is_empty() {
            self.add_sides(
                &sides,
                BridgeEvidenceKind::TypeReference,
                last_path_ident(&path.path).unwrap_or_else(|| "<type>".to_owned()),
                normalized_tokens(path),
            );
        }
        visit::visit_type_path(self, path);
    }

    fn visit_expr_path(&mut self, path: &'ast syn::ExprPath) {
        let mut sides = self.symbols.sides_for_path(&path.path);
        if let Some(name) = last_path_ident(&path.path) {
            if let Some(variable_sides) = self.variables.get(&name) {
                sides.extend(variable_sides);
            }
        }
        if !sides.is_empty() {
            self.add_sides(
                &sides,
                BridgeEvidenceKind::ValueReference,
                last_path_ident(&path.path).unwrap_or_else(|| "<value>".to_owned()),
                normalized_tokens(path),
            );
        }
        visit::visit_expr_path(self, path);
    }

    fn visit_expr_field(&mut self, field: &'ast ExprField) {
        let sides = self.sides_of_expr(&Expr::Field(field.clone()));
        if !sides.is_empty() {
            let symbol = match &field.member {
                Member::Named(member) => member.to_string(),
                Member::Unnamed(index) => index.index.to_string(),
            };
            self.add_sides(
                &sides,
                BridgeEvidenceKind::FieldAccess,
                symbol,
                normalized_tokens(field),
            );
        }
        visit::visit_expr_field(self, field);
    }

    fn visit_expr_call(&mut self, call: &'ast ExprCall) {
        if let Expr::Path(path) = call.func.as_ref() {
            let name = last_path_ident(&path.path).unwrap_or_else(|| "<call>".to_owned());
            let sides = self.symbols.sides_for_path(&path.path);
            if !sides.is_empty() {
                self.add_sides(
                    &sides,
                    BridgeEvidenceKind::FunctionCall,
                    name.clone(),
                    normalized_tokens(call),
                );
            }
        }
        visit::visit_expr_call(self, call);
    }

    fn visit_expr_method_call(&mut self, call: &'ast ExprMethodCall) {
        let name = call.method.to_string();
        let sides = self.sides_of_expr(&call.receiver);
        if !sides.is_empty() {
            self.add_sides(
                &sides,
                BridgeEvidenceKind::MethodCall,
                name.clone(),
                normalized_tokens(call),
            );
        }
        visit::visit_expr_method_call(self, call);
    }

    fn visit_local(&mut self, local: &'ast Local) {
        if let Pat::Type(typed) = &local.pat {
            self.visit_type(&typed.ty);
        }
        let mut sides = BTreeSet::new();
        if let Some(init) = &local.init {
            self.visit_expr(&init.expr);
            sides.extend(self.sides_of_expr(&init.expr));
            if let Some((_, diverge)) = &init.diverge {
                self.visit_expr(diverge);
            }
        }
        if let Pat::Type(typed) = &local.pat {
            sides.extend(sides_in_type(&self.symbols, &typed.ty));
        }
        self.bind_pattern(&local.pat, &sides);
    }

    fn visit_expr_macro(&mut self, expression: &'ast ExprMacro) {
        self.audit_macro(&expression.mac, "expression macro");
    }

    fn visit_stmt_macro(&mut self, statement: &'ast syn::StmtMacro) {
        self.audit_macro(&statement.mac, "statement macro");
    }

    fn visit_type_macro(&mut self, type_macro: &'ast syn::TypeMacro) {
        self.audit_macro(&type_macro.mac, "type macro");
    }

    fn visit_item(&mut self, item: &'ast Item) {
        match item {
            Item::Use(item_use) => add_use_to_symbols(item_use, &mut self.symbols, self.errors),
            Item::Macro(item_macro) => {
                self.audit_macro(&item_macro.mac, "nested item macro");
            }
            // Nested items have their own enclosing identity and must be passed
            // as source/module items rather than folded into this function.
            _ => {}
        }
    }
}

fn method_enclosing(item_impl: &ItemImpl, signature: &Signature) -> String {
    let self_type = normalized_tokens(&item_impl.self_ty);
    match &item_impl.trait_ {
        Some((_, trait_path, _)) => format!(
            "<{} as {}>::{}",
            self_type,
            normalized_tokens(trait_path),
            signature.ident
        ),
        None => format!("{}::{}", self_type, signature.ident),
    }
}

fn analyze_function(
    function: &ItemFn,
    context: &ModuleContext<'_>,
    symbols: &Symbols,
    accumulator: &mut BridgeAccumulator,
    errors: &mut Vec<String>,
) {
    validate_cfg(
        &context.cfg,
        &function.attrs,
        &format!("function {}", function.sig.ident),
        errors,
    );
    let cfg = extend_cfg_context(&context.cfg, &function.attrs);
    let enclosing = format!("fn::{}", function.sig.ident);
    let mut analyzer = CandidateAnalyzer::new(context, enclosing, symbols, errors);
    analyzer.seed_definition_anchor(&function.sig.ident.to_string());
    analyzer.bind_signature(&function.sig);
    analyzer.visit_signature(&function.sig);
    analyzer.visit_block(&function.block);
    let header = format!(
        "{}|body={}",
        normalized_tokens(&function.sig),
        compact_token_fingerprint(&function.block)
    );
    if let Some(record) = analyzer.finish(header, cfg) {
        accumulator.add(record);
    }
}

fn analyze_impl(
    item_impl: &ItemImpl,
    context: &ModuleContext<'_>,
    symbols: &Symbols,
    accumulator: &mut BridgeAccumulator,
    errors: &mut Vec<String>,
) {
    validate_cfg(&context.cfg, &item_impl.attrs, "impl", errors);
    let impl_cfg = extend_cfg_context(&context.cfg, &item_impl.attrs);
    let self_sides = sides_in_type(symbols, &item_impl.self_ty);
    let trait_sides = item_impl
        .trait_
        .as_ref()
        .map(|(_, path, _)| symbols.sides_for_path(path))
        .unwrap_or_default();
    for item in &item_impl.items {
        match item {
            ImplItem::Fn(method) => {
                validate_cfg(
                    &impl_cfg,
                    &method.attrs,
                    &format!("method {}", method.sig.ident),
                    errors,
                );
                let cfg = extend_cfg_context(&impl_cfg, &method.attrs);
                let enclosing = method_enclosing(item_impl, &method.sig);
                let mut analyzer = CandidateAnalyzer::new(context, enclosing, symbols, errors);
                analyzer.seed_definition_anchor(&method.sig.ident.to_string());
                analyzer.bind_signature(&method.sig);
                if !self_sides.is_empty() {
                    analyzer
                        .variables
                        .insert("self".to_owned(), self_sides.clone());
                    analyzer.add_sides(
                        &self_sides,
                        BridgeEvidenceKind::SelfType,
                        "Self",
                        normalized_tokens(&item_impl.self_ty),
                    );
                }
                if !trait_sides.is_empty() {
                    analyzer.add_sides(
                        &trait_sides,
                        BridgeEvidenceKind::SelfType,
                        "Trait",
                        item_impl
                            .trait_
                            .as_ref()
                            .map(|(_, path, _)| normalized_tokens(path))
                            .unwrap_or_default(),
                    );
                }
                analyzer.visit_signature(&method.sig);
                analyzer.visit_block(&method.block);
                let header = format!(
                    "impl:{}:{}:{}|body={}",
                    normalized_tokens(&item_impl.self_ty),
                    item_impl
                        .trait_
                        .as_ref()
                        .map(|(_, path, _)| normalized_tokens(path))
                        .unwrap_or_default(),
                    normalized_tokens(&method.sig),
                    compact_token_fingerprint(&method.block)
                );
                if let Some(record) = analyzer.finish(header, cfg) {
                    accumulator.add(record);
                }
            }
            ImplItem::Const(constant) => {
                let cfg = extend_cfg_context(&impl_cfg, &constant.attrs);
                let enclosing = format!(
                    "{}::const::{}",
                    normalized_tokens(&item_impl.self_ty),
                    constant.ident
                );
                let mut analyzer = CandidateAnalyzer::new(context, enclosing, symbols, errors);
                analyzer.visit_type(&constant.ty);
                analyzer.visit_expr(&constant.expr);
                let header = format!(
                    "const {}:{}|value={}",
                    constant.ident,
                    normalized_tokens(&constant.ty),
                    compact_token_fingerprint(&constant.expr)
                );
                if let Some(record) = analyzer.finish(header, cfg) {
                    accumulator.add(record);
                }
            }
            ImplItem::Type(item_type) => {
                let cfg = extend_cfg_context(&impl_cfg, &item_type.attrs);
                let enclosing = format!(
                    "{}::type::{}",
                    normalized_tokens(&item_impl.self_ty),
                    item_type.ident
                );
                let mut analyzer = CandidateAnalyzer::new(context, enclosing, symbols, errors);
                analyzer.visit_type(&item_type.ty);
                let header = format!(
                    "type {}={}",
                    item_type.ident,
                    normalized_tokens(&item_type.ty)
                );
                if let Some(record) = analyzer.finish(header, cfg) {
                    accumulator.add(record);
                }
            }
            ImplItem::Macro(item_macro) => {
                audit_item_macro(
                    &ItemMacro {
                        attrs: item_macro.attrs.clone(),
                        ident: None,
                        mac: item_macro.mac.clone(),
                        semi_token: item_macro.semi_token,
                    },
                    context,
                    symbols,
                    errors,
                    "impl item macro",
                );
            }
            _ => {}
        }
    }
}

fn analyze_data_item(
    item: &Item,
    context: &ModuleContext<'_>,
    symbols: &Symbols,
    accumulator: &mut BridgeAccumulator,
    errors: &mut Vec<String>,
) {
    let (name, attrs) = match item {
        Item::Struct(value) => (format!("struct::{}", value.ident), value.attrs.as_slice()),
        Item::Enum(value) => (format!("enum::{}", value.ident), value.attrs.as_slice()),
        Item::Type(value) => (format!("type::{}", value.ident), value.attrs.as_slice()),
        Item::Const(value) => (format!("const::{}", value.ident), value.attrs.as_slice()),
        Item::Static(value) => (format!("static::{}", value.ident), value.attrs.as_slice()),
        _ => return,
    };
    validate_cfg(&context.cfg, attrs, &name, errors);
    let cfg = extend_cfg_context(&context.cfg, attrs);
    let header = format!("{name}|surface={}", compact_token_fingerprint(item));
    let mut analyzer = CandidateAnalyzer::new(context, name, symbols, errors);
    match item {
        Item::Struct(value) => {
            for field in &value.fields {
                analyzer.visit_type(&field.ty);
            }
        }
        Item::Enum(value) => {
            for variant in &value.variants {
                for field in &variant.fields {
                    analyzer.visit_type(&field.ty);
                }
                if let Some((_, discriminant)) = &variant.discriminant {
                    analyzer.visit_expr(discriminant);
                }
            }
        }
        Item::Type(value) => analyzer.visit_type(&value.ty),
        Item::Const(value) => {
            analyzer.visit_type(&value.ty);
            analyzer.visit_expr(&value.expr);
        }
        Item::Static(value) => {
            analyzer.visit_type(&value.ty);
            analyzer.visit_expr(&value.expr);
        }
        _ => unreachable!("data item was matched above"),
    }
    if let Some(record) = analyzer.finish(header, cfg) {
        accumulator.add(record);
    }
}

fn audit_item_macro(
    item_macro: &ItemMacro,
    context: &ModuleContext<'_>,
    symbols: &Symbols,
    errors: &mut Vec<String>,
    label: &str,
) {
    let name = item_macro
        .ident
        .as_ref()
        .map(ToString::to_string)
        .or_else(|| last_path_ident(&item_macro.mac.path))
        .unwrap_or_else(|| "<macro>".to_owned());
    let sides = token_sides(&item_macro.mac.tokens, symbols, &BTreeMap::new());
    if !item_macro.mac.path.is_ident("include")
        && (!sides.is_empty()
            || tokens_mention_curated_anchor(&item_macro.mac.tokens)
            || bridge_shaped_name(&name))
    {
        errors.push(format!(
            "{label} {name}! in {}::{} can generate or hide a legacy/canonical bridge",
            context.package, context.module
        ));
    }
}

fn analyze_module_items(
    items: &[Item],
    context: &ModuleContext<'_>,
    inherited_symbols: &Symbols,
    accumulator: &mut BridgeAccumulator,
    errors: &mut Vec<String>,
) {
    let symbols = register_module_symbols(items, context, inherited_symbols, errors);
    for item in items {
        match item {
            Item::Fn(function) => {
                analyze_function(function, context, &symbols, accumulator, errors)
            }
            Item::Impl(item_impl) => {
                analyze_impl(item_impl, context, &symbols, accumulator, errors)
            }
            Item::Struct(_) | Item::Enum(_) | Item::Type(_) | Item::Const(_) | Item::Static(_) => {
                analyze_data_item(item, context, &symbols, accumulator, errors)
            }
            Item::Macro(item_macro) => {
                audit_item_macro(item_macro, context, &symbols, errors, "item macro")
            }
            Item::Mod(ItemMod {
                attrs,
                ident,
                content: Some((_, child_items)),
                ..
            }) => {
                validate_cfg(&context.cfg, attrs, &format!("module {ident}"), errors);
                let child = ModuleContext {
                    package: context.package,
                    module: format!("{}::{ident}", context.module),
                    path: context.path,
                    cfg: extend_cfg_context(&context.cfg, attrs),
                };
                analyze_module_items(child_items, &child, &symbols, accumulator, errors);
            }
            _ => {}
        }
    }
}

/// Parse and inventory bridge-bearing items from already resolved world-server
/// and wow-world source mounts. Input order is irrelevant; duplicate exact
/// `(package, module, path, inherited cfg)` mounts fail because they would make
/// multiplicity depend on caller behavior. Distinct cfg mounts remain visible.
pub(crate) fn inventory_bridge_accesses(
    sources: &[BridgeSource<'_>],
) -> Result<BridgeAccessBaseline, String> {
    let mut ordered: Vec<_> = sources.iter().copied().collect();
    ordered.sort_by(|left, right| {
        (
            left.package,
            left.module,
            left.source_path,
            left.inherited_cfg,
        )
            .cmp(&(
                right.package,
                right.module,
                right.source_path,
                right.inherited_cfg,
            ))
    });
    let mut seen = BTreeSet::new();
    let mut accumulator = BridgeAccumulator::default();
    let mut errors = Vec::new();
    for source in ordered {
        if source.package.is_empty() || source.module.is_empty() || source.source_path.is_empty() {
            errors.push("bridge source package/module/path must be non-empty".to_owned());
            continue;
        }
        if !seen.insert((
            source.package,
            source.module,
            source.source_path,
            source.inherited_cfg,
        )) {
            errors.push(format!(
                "duplicate bridge source mount {} {} {}",
                source.package, source.module, source.source_path
            ));
            continue;
        }
        let syntax = match syn::parse_file(source.source) {
            Ok(syntax) => syntax,
            Err(error) => {
                errors.push(format!(
                    "cannot parse bridge source {}: {error}",
                    source.source_path
                ));
                continue;
            }
        };
        validate_cfg(
            source.inherited_cfg,
            &syntax.attrs,
            source.source_path,
            &mut errors,
        );
        let context = ModuleContext {
            package: source.package,
            module: source.module.to_owned(),
            path: source.source_path,
            cfg: extend_cfg_context(source.inherited_cfg, &syntax.attrs),
        };
        analyze_module_items(
            &syntax.items,
            &context,
            &Symbols::for_module(source.package, source.module),
            &mut accumulator,
            &mut errors,
        );
    }
    if errors.is_empty() {
        Ok(accumulator.finish())
    } else {
        errors.sort();
        errors.dedup();
        Err(errors.join("\n"))
    }
}

fn validate_record(label: &str, record: &BridgeAccessRecord) -> Result<(), String> {
    if record.multiplicity == 0 {
        return Err(format!(
            "{label} bridge baseline contains a zero-multiplicity row at {}::{}",
            record.module, record.enclosing
        ));
    }
    if record.package.is_empty()
        || record.module.is_empty()
        || record.path.is_empty()
        || record.enclosing.is_empty()
        || record.fingerprint.is_empty()
    {
        return Err(format!(
            "{label} bridge baseline contains an incomplete row at {}::{}",
            record.module, record.enclosing
        ));
    }
    if record.direction_markers.is_empty()
        || record
            .direction_markers
            .windows(2)
            .any(|window| window[0] >= window[1])
    {
        return Err(format!(
            "{label} bridge direction markers are empty or noncanonical at {}::{}",
            record.module, record.enclosing
        ));
    }
    if record.evidence.is_empty() {
        return Err(format!(
            "{label} bridge row has no AST evidence at {}::{}",
            record.module, record.enclosing
        ));
    }
    let mut previous: Option<EvidenceIdentity> = None;
    for evidence in &record.evidence {
        if evidence.multiplicity == 0 {
            return Err(format!(
                "{label} bridge evidence has zero multiplicity at {}::{}",
                record.module, record.enclosing
            ));
        }
        let identity = EvidenceIdentity {
            side: evidence.side,
            kind: evidence.kind,
            symbol: evidence.symbol.clone(),
            fingerprint: evidence.fingerprint.clone(),
        };
        if previous
            .as_ref()
            .is_some_and(|previous| previous >= &identity)
        {
            return Err(format!(
                "{label} bridge evidence is not in strict canonical order at {}::{}",
                record.module, record.enclosing
            ));
        }
        previous = Some(identity);
    }
    let sides: BTreeSet<_> = record
        .evidence
        .iter()
        .map(|evidence| evidence.side)
        .collect();
    if record
        .direction_markers
        .contains(&BridgeDirection::UnresolvedDualSide)
        && !(sides.contains(&BridgeSide::Canonical) && sides.contains(&BridgeSide::Legacy))
    {
        return Err(format!(
            "{label} unresolved bridge lacks evidence from both sides at {}::{}",
            record.module, record.enclosing
        ));
    }
    Ok(())
}

fn validated_baseline_map(
    label: &str,
    baseline: &BridgeAccessBaseline,
) -> Result<BTreeMap<BridgeIdentity, usize>, String> {
    if baseline.schema_version != BRIDGE_SCHEMA_VERSION {
        return Err(format!(
            "{label} bridge baseline schema version is {}, expected {BRIDGE_SCHEMA_VERSION}",
            baseline.schema_version
        ));
    }
    let mut map = BTreeMap::new();
    let mut previous: Option<BridgeIdentity> = None;
    for record in &baseline.bridges {
        validate_record(label, record)?;
        let identity = record.identity();
        if previous
            .as_ref()
            .is_some_and(|previous| previous >= &identity)
        {
            return Err(format!(
                "{label} bridge rows are not in strict canonical order near {}::{}",
                record.module, record.enclosing
            ));
        }
        previous = Some(identity.clone());
        if map.insert(identity, record.multiplicity).is_some() {
            return Err(format!(
                "{label} bridge baseline contains a duplicate row at {}::{}",
                record.module, record.enclosing
            ));
        }
    }
    Ok(map)
}

fn describe_bridge(identity: &BridgeIdentity) -> String {
    format!(
        "{} {} {}::{} directions={:?} cfg=[{}] fingerprint={}",
        identity.package,
        identity.path,
        identity.module,
        identity.enclosing,
        identity.direction_markers,
        identity.cfg.join(", "),
        identity.fingerprint
    )
}

/// Exact comparison of additions, removals, syntax/direction swaps, cfg drift,
/// and multiplicity. Debt reduction must delete the obsolete baseline row in
/// the same reviewed change, so it cannot be silently reintroduced later.
pub(crate) fn compare_bridge_access_baseline(
    expected: &BridgeAccessBaseline,
    actual: &BridgeAccessBaseline,
) -> Result<(), String> {
    let expected = validated_baseline_map("expected", expected)?;
    let actual = validated_baseline_map("actual", actual)?;
    let mut errors = Vec::new();
    for (identity, actual_count) in &actual {
        match expected.get(identity) {
            None => errors.push(format!(
                "untracked legacy/canonical bridge: {} (multiplicity {actual_count})",
                describe_bridge(identity)
            )),
            Some(expected_count) if expected_count != actual_count => errors.push(format!(
                "legacy/canonical bridge multiplicity changed: {} expected {expected_count}, actual {actual_count}",
                describe_bridge(identity)
            )),
            Some(_) => {}
        }
    }
    for (identity, expected_count) in &expected {
        if !actual.contains_key(identity) {
            errors.push(format!(
                "obsolete legacy/canonical bridge baseline row: {} (expected multiplicity {expected_count})",
                describe_bridge(identity)
            ));
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("\n"))
    }
}

/// Check that every direction-bearing anchor copied from the runtime ledger is
/// present exactly once as a definition in its expected package/module.
pub(crate) fn validate_curated_bridge_anchors(
    baseline: &BridgeAccessBaseline,
) -> Result<(), String> {
    validated_baseline_map("curated-anchor", baseline)?;
    let mut errors = Vec::new();
    for anchor in CURATED_ANCHORS {
        let count: usize = baseline
            .bridges
            .iter()
            .filter(|record| {
                record.package == anchor.package
                    && record.module == anchor.module
                    && record.evidence.iter().any(|evidence| {
                        evidence.kind == BridgeEvidenceKind::CuratedAnchorDefinition
                            && evidence.symbol == anchor.name
                    })
            })
            .map(|record| record.multiplicity)
            .sum();
        if count != 1 {
            errors.push(format!(
                "curated bridge anchor {} {}::{} expected exactly one definition, found {count}",
                anchor.package, anchor.module, anchor.name
            ));
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;

    fn source<'a>(text: &'a str) -> BridgeSource<'a> {
        BridgeSource {
            package: "fixture",
            module: "crate::fixture",
            source_path: "src/fixture.rs",
            inherited_cfg: &[],
            source: text,
        }
    }

    fn inventory(text: &str) -> Result<BridgeAccessBaseline, String> {
        inventory_bridge_accesses(&[source(text)])
    }

    #[test]
    fn innocent_bridge_shaped_function_name_is_not_evidence() {
        let baseline = inventory(
            r#"
                fn canonical_legacy_mirror_story(value: u32) -> u32 {
                    value + 1
                }
            "#,
        )
        .expect("an innocent name parses");
        assert!(baseline.bridges.is_empty());
    }

    #[test]
    fn canonical_dto_in_transparent_macro_does_not_invent_authority() {
        let baseline = inventory(
            r#"
                fn step(creature: &crate::map_manager::WorldCreature) {
                    let _ = matches!(
                        creature.state(),
                        wow_entities::CreatureAiState::Idle
                    );
                }
            "#,
        )
        .expect("a DTO enum in a transparent macro is inspectable");
        assert!(baseline.bridges.is_empty());
    }

    #[test]
    fn explicit_canonical_creature_to_legacy_runtime_is_a_bridge() {
        let baseline = inventory(
            r#"
                use wow_entities::Creature as CanonicalCreature;

                fn translate(old: &wow_world::SharedMapManager) {
                    let _ = old;
                    let _ = CanonicalCreature::default();
                    debug!(value = ?CanonicalCreature::default());
                }
            "#,
        )
        .expect("an explicitly imported canonical entity remains attributable");
        assert_eq!(baseline.bridges.len(), 1);
        assert_eq!(
            baseline.bridges[0].direction_markers,
            vec![BridgeDirection::UnresolvedDualSide]
        );
        assert!(
            baseline.bridges[0]
                .evidence
                .iter()
                .any(|marker| marker.kind == BridgeEvidenceKind::MacroArgument)
        );
    }

    #[test]
    fn enum_variant_does_not_inherit_an_unrelated_import_with_the_same_name() {
        let baseline = inventory(
            r#"
                use wow_entities::Creature;
                use wow_map::SpawnObjectType;

                fn inspect(old: &wow_world::SharedMapManager) {
                    let _ = old;
                    let _ = SpawnObjectType::Creature;
                    assert!(matches!(
                        SpawnObjectType::Creature,
                        SpawnObjectType::Creature
                    ));
                }
            "#,
        )
        .expect("the enum variant and imported type have distinct provenance");
        assert!(baseline.bridges.is_empty());
    }

    #[test]
    fn bridge_capable_globs_and_namespace_aliases_fail_closed() {
        let error = inventory(
            r#"
                use wow_entities::*;
                use crate::map_manager as old_runtime;
            "#,
        )
        .expect_err("namespace indirection can hide exact authority types");
        assert!(error.contains("bridge-capable glob import"), "{error}");
        assert!(error.contains("bridge-capable namespace import"), "{error}");
    }

    #[test]
    fn unnamed_true_bridge_is_resolved_from_types_and_receivers() {
        let baseline = inventory(
            r#"
                use wow_map::MapManager as NewMaps;
                use wow_world::SharedMapManager as OldMaps;

                fn synchronize(old: &OldMaps, new: &mut NewMaps) {
                    let old_guard = old.read().unwrap();
                    new.create_map(old_guard.map_count());
                }
            "#,
        )
        .expect("structural bridge parses");
        assert_eq!(baseline.bridges.len(), 1);
        let bridge = &baseline.bridges[0];
        assert_eq!(
            bridge.direction_markers,
            vec![BridgeDirection::UnresolvedDualSide]
        );
        let sides: BTreeSet<_> = bridge.evidence.iter().map(|marker| marker.side).collect();
        assert_eq!(
            sides,
            BTreeSet::from([BridgeSide::Canonical, BridgeSide::Legacy])
        );
        assert!(bridge.fingerprint.len() < 1_024);
        assert!(
            bridge
                .evidence
                .iter()
                .all(|marker| marker.fingerprint.len() < 96)
        );
    }

    #[test]
    fn same_count_body_swap_with_identical_symbols_fails_the_exact_comparator() {
        let expected = inventory(
            r#"
                fn reconcile(
                    old: &wow_world::SharedMapManager,
                    new: &mut wow_map::MapManager,
                ) {
                    old.read().unwrap();
                    new.find_map(1, 0);
                    new.find_map(2, 0);
                }
            "#,
        )
        .unwrap();
        let actual = inventory(
            r#"
                fn reconcile(
                    old: &wow_world::SharedMapManager,
                    new: &mut wow_map::MapManager,
                ) {
                    old.read().unwrap();
                    new.find_map(2, 0);
                    new.find_map(1, 0);
                }
            "#,
        )
        .unwrap();
        assert_eq!(expected.bridges.len(), actual.bridges.len());
        let error = compare_bridge_access_baseline(&expected, &actual)
            .expect_err("same-count semantic swap must fail");
        assert!(
            error.contains("untracked legacy/canonical bridge"),
            "{error}"
        );
        assert!(
            error.contains("obsolete legacy/canonical bridge baseline row"),
            "{error}"
        );
    }

    #[test]
    fn cfg_identity_is_exact_and_test_only_bridges_are_not_dropped() {
        let expected = inventory(
            r#"
                #[cfg(feature = "old-bridge")]
                fn reconcile(
                    old: &wow_world::SharedMapManager,
                    new: &wow_map::MapManager,
                ) {}
            "#,
        )
        .unwrap();
        assert!(
            expected.bridges[0]
                .cfg
                .iter()
                .any(|cfg| cfg.contains("old-bridge"))
        );
        let actual = inventory(
            r#"
                #[cfg(test)]
                fn reconcile(
                    old: &wow_world::SharedMapManager,
                    new: &wow_map::MapManager,
                ) {}
            "#,
        )
        .unwrap();
        assert_eq!(actual.bridges.len(), 1, "cfg(test) is inventoried");
        assert!(compare_bridge_access_baseline(&expected, &actual).is_err());
    }

    #[test]
    fn distinct_cfg_mounts_are_preserved_and_exact_duplicate_mounts_fail() {
        let text = r#"
            fn reconcile(
                old: &wow_world::SharedMapManager,
                new: &wow_map::MapManager,
            ) {}
        "#;
        let production_cfg = vec!["cfg(feature = \"production-mount\")".to_owned()];
        let test_cfg = vec!["cfg(test)".to_owned()];
        let production = BridgeSource {
            package: "fixture",
            module: "crate::fixture",
            source_path: "src/shared.rs",
            inherited_cfg: &production_cfg,
            source: text,
        };
        let test = BridgeSource {
            inherited_cfg: &test_cfg,
            ..production
        };
        let baseline = inventory_bridge_accesses(&[test, production])
            .expect("distinct cfg mounts are separate syntax surfaces");
        assert_eq!(baseline.bridges.len(), 2);
        assert_ne!(baseline.bridges[0].cfg, baseline.bridges[1].cfg);

        let error = inventory_bridge_accesses(&[production, production])
            .expect_err("an exact duplicate source mount must fail");
        assert!(error.contains("duplicate bridge source mount"), "{error}");
    }

    #[test]
    fn bridge_hiding_macros_fail_closed_but_transparent_arguments_are_visible() {
        let invocation = inventory(
            r#"
                fn hidden(
                    old: &wow_world::SharedMapManager,
                    new: &wow_map::MapManager,
                ) {
                    apply_bridge!(old, new);
                }
            "#,
        )
        .expect_err("unknown macro cannot hide both authorities");
        assert!(
            invocation.contains("can hide a legacy/canonical bridge"),
            "{invocation}"
        );

        let split_invocation = inventory(
            r#"
                fn hidden(old: &wow_world::SharedMapManager) {
                    consume!(wow_map::MapManager::default());
                }
            "#,
        )
        .expect_err("an unknown macro cannot hide one half of an item-level bridge");
        assert!(
            split_invocation.contains("can hide part of a legacy/canonical bridge"),
            "{split_invocation}"
        );

        let definition = inventory(
            r#"
                macro_rules! generated_bridge {
                    () => {
                        fn hidden(
                            old: &wow_world::SharedMapManager,
                            new: &wow_map::MapManager,
                        ) {}
                    };
                }
            "#,
        )
        .expect_err("item-generating macro cannot conceal a bridge");
        assert!(definition.contains("can generate or hide"), "{definition}");

        let transparent = inventory(
            r#"
                fn visible(
                    old: &wow_world::SharedMapManager,
                    new: &wow_map::MapManager,
                ) {
                    debug!(?old, ?new, "bridge diagnostics");
                }
            "#,
        )
        .expect("transparent macro arguments remain inventory evidence");
        assert!(
            transparent.bridges[0]
                .evidence
                .iter()
                .any(|marker| marker.kind == BridgeEvidenceKind::MacroArgument)
        );
    }

    #[test]
    fn curated_anchor_records_its_declared_direction() {
        let baseline = inventory_bridge_accesses(&[BridgeSource {
            package: "world-server",
            module: "crate::runtime::game_events",
            source_path: "src/session.rs",
            inherited_cfg: &[],
            source: r#"
                fn mirror_loaded_grid_creature_to_legacy_like_cpp(
                    canonical: &wow_map::MapManager,
                    legacy: &wow_world::MapManager,
                ) {}
            "#,
        }])
        .expect("curated method parses");
        assert_eq!(baseline.bridges.len(), 1);
        assert_eq!(
            baseline.bridges[0].direction_markers,
            vec![BridgeDirection::CanonicalToLegacy]
        );
        let sides: BTreeSet<_> = baseline.bridges[0]
            .evidence
            .iter()
            .map(|marker| marker.side)
            .collect();
        assert!(sides.contains(&BridgeSide::Legacy));
        assert!(sides.contains(&BridgeSide::Canonical));
    }

    #[test]
    fn comparator_rejects_multiplicity_and_noncanonical_rows() {
        let baseline = inventory(
            r#"
                fn reconcile(
                    old: &wow_world::SharedMapManager,
                    new: &wow_map::MapManager,
                ) {}
            "#,
        )
        .unwrap();
        let mut multiplicity = baseline.clone();
        multiplicity.bridges[0].multiplicity = 2;
        let error = compare_bridge_access_baseline(&baseline, &multiplicity)
            .expect_err("multiplicity drift must fail");
        assert!(error.contains("multiplicity changed"), "{error}");

        let mut zero = baseline.clone();
        zero.bridges[0].multiplicity = 0;
        let error = compare_bridge_access_baseline(&zero, &baseline)
            .expect_err("zero multiplicity is invalid");
        assert!(error.contains("zero-multiplicity"), "{error}");

        let mut bad_evidence = baseline.clone();
        bad_evidence.bridges[0].evidence.reverse();
        let error = compare_bridge_access_baseline(&bad_evidence, &baseline)
            .expect_err("evidence must remain canonical");
        assert!(error.contains("strict canonical order"), "{error}");
    }

    #[test]
    fn real_runtime_ledger_anchor_definitions_are_present_once() {
        let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let delivery_path = repository.join("crates/world-server/src/runtime/delivery.rs");
        let game_events_path = repository.join("crates/world-server/src/runtime/game_events.rs");
        let session_path = repository.join("crates/wow-world/src/session/mod.rs");
        let delivery = fs::read_to_string(&delivery_path).expect("world-server delivery source");
        let game_events =
            fs::read_to_string(&game_events_path).expect("world-server game-events source");
        let session = fs::read_to_string(&session_path).expect("wow-world session source");
        let baseline = inventory_bridge_accesses(&[
            BridgeSource {
                package: "world-server",
                module: "crate::runtime::delivery",
                source_path: "crates/world-server/src/runtime/delivery.rs",
                inherited_cfg: &[],
                source: &delivery,
            },
            BridgeSource {
                package: "world-server",
                module: "crate::runtime::game_events",
                source_path: "crates/world-server/src/runtime/game_events.rs",
                inherited_cfg: &[],
                source: &game_events,
            },
            BridgeSource {
                package: "wow-world",
                module: "crate::session",
                source_path: "crates/wow-world/src/session/mod.rs",
                inherited_cfg: &[],
                source: &session,
            },
        ])
        .expect("real bridge anchor sources must remain inspectable");
        validate_curated_bridge_anchors(&baseline)
            .expect("every curated runtime-ledger anchor is defined exactly once");
    }
}
