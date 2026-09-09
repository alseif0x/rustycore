//! Module symbol table and persistence-target name recognition.
//!
//! Separated from the persistence-access root under #634. Behaviour is
//! preserved; this module owns no new state.

use super::*;

#[derive(Clone, Debug)]
pub(super) struct ModuleSymbols {
    pub(super) type_aliases: BTreeMap<String, TargetSet>,
    pub(super) nominal_type_aliases: BTreeMap<String, BTreeSet<String>>,
    pub(super) type_alias_info: BTreeMap<String, VariableInfo>,
    pub(super) path_aliases: BTreeMap<String, Vec<String>>,
    pub(super) traits_in_scope: BTreeMap<String, String>,
    pub(super) anonymous_traits_in_scope: BTreeSet<String>,
    pub(super) module_path: Vec<String>,
    pub(super) field_targets: BTreeMap<(String, String), TargetSet>,
    pub(super) field_owners: BTreeMap<String, BTreeSet<String>>,
    pub(super) tuple_field_targets: BTreeMap<(String, String), TargetSet>,
    pub(super) field_nominal_types: BTreeMap<(String, String), BTreeSet<String>>,
    pub(super) function_returns: BTreeMap<String, VariableInfo>,
    pub(super) function_called_inputs: BTreeMap<String, BTreeSet<usize>>,
    pub(super) function_mutable_writes: BTreeMap<String, BTreeMap<usize, VariableInfo>>,
    pub(super) method_returns: BTreeMap<(String, Option<String>, String), VariableInfo>,
    pub(super) method_mutable_receivers: BTreeMap<(String, Option<String>, String), VariableInfo>,
    pub(super) method_mutable_writes:
        BTreeMap<(String, Option<String>, String), BTreeMap<usize, VariableInfo>>,
    // Generic parameter name lists, recorded next to the return registries so
    // an explicit turbofish at the call site can be substituted into the
    // recorded return instead of letting `make::<CharacterDatabase>()` bypass
    // both ratchets.
    pub(super) function_generic_params: BTreeMap<String, Vec<String>>,
    pub(super) function_generic_input_params: BTreeMap<String, Vec<GenericInputSpec>>,
    pub(super) method_generic_params: BTreeMap<(String, Option<String>, String), Vec<String>>,
    pub(super) method_generic_input_params:
        BTreeMap<(String, Option<String>, String), Vec<GenericInputSpec>>,
    pub(super) trait_method_returns: std::sync::Arc<BTreeMap<(String, String), VariableInfo>>,
    pub(super) trait_supertraits: std::sync::Arc<BTreeMap<String, BTreeSet<String>>>,
    pub(super) trait_generic_params: std::sync::Arc<BTreeMap<String, Vec<String>>>,
    pub(super) trait_method_generic_params: std::sync::Arc<BTreeMap<(String, String), Vec<String>>>,
    pub(super) trait_method_generic_input_params:
        std::sync::Arc<BTreeMap<(String, String), Vec<GenericInputSpec>>>,
    // Current-package paths stay unqualified (`dto::Holder`) while the
    // workspace registry is shared and crate-qualified
    // (`provider_crate::dto::Holder`).
    pub(super) named_type_info: std::sync::Arc<BTreeMap<String, VariableInfo>>,
    pub(super) workspace_named_type_info: std::sync::Arc<BTreeMap<String, VariableInfo>>,
    pub(super) dependency_crate_aliases: std::sync::Arc<BTreeMap<String, String>>,
    pub(super) package_function_returns: std::sync::Arc<BTreeMap<String, VariableInfo>>,
    pub(super) package_function_mutable_writes:
        std::sync::Arc<BTreeMap<String, BTreeMap<usize, VariableInfo>>>,
    pub(super) package_function_generic_params: std::sync::Arc<BTreeMap<String, Vec<String>>>,
    pub(super) package_function_generic_input_params:
        std::sync::Arc<BTreeMap<String, Vec<GenericInputSpec>>>,
    // Package-wide registries for inherent impl methods (keyed by canonical
    // crate-relative owner path): without them `factory.make()` only resolves
    // when the impl lives in the same module as the call.
    pub(super) package_method_returns: std::sync::Arc<BTreeMap<(String, String), VariableInfo>>,
    pub(super) package_method_mutable_receivers:
        std::sync::Arc<BTreeMap<(String, String), VariableInfo>>,
    pub(super) package_method_mutable_writes:
        std::sync::Arc<BTreeMap<(String, String), BTreeMap<usize, VariableInfo>>>,
    pub(super) package_method_generic_params:
        std::sync::Arc<BTreeMap<(String, String), Vec<String>>>,
    pub(super) package_method_generic_input_params:
        std::sync::Arc<BTreeMap<(String, String), Vec<GenericInputSpec>>>,
    // Module constants/statics are value bindings, not lexical locals. Keep
    // both the current module's names and a package-wide canonical registry
    // so their declared persistence-bearing types survive path resolution.
    pub(super) item_values: BTreeMap<String, VariableInfo>,
    pub(super) package_item_values: std::sync::Arc<BTreeMap<String, VariableInfo>>,
    pub(super) sqlx_namespaces: BTreeSet<String>,
    pub(super) workspace_sqlx_namespaces: std::sync::Arc<BTreeSet<String>>,
    pub(super) database_namespaces: BTreeSet<String>,
    pub(super) query_callables: BTreeSet<String>,
    /// Names this module defines with `macro_rules!`. An unqualified builtin
    /// resolves to the same `module_path + name` shape as one of these, so the
    /// definitions have to be known to tell them apart.
    pub(super) local_macro_definitions: BTreeSet<String>,
    /// Types this module declares. A `struct String` of one's own shadows the
    /// prelude type, and path resolution alone cannot tell them apart.
    pub(super) local_type_definitions: BTreeSet<String>,
    // `macro_rules!` definitions whose body already reaches concrete
    // persistence: the definition is baselined once, and this registry makes
    // every later invocation leave its own row too.
    pub(super) persistence_macros: BTreeMap<String, TargetSet>,
    // Persistence-generating macro_rules definitions can be invoked from a
    // different physical source module through #[macro_use], #[macro_export],
    // or a macro import. The leaf-name union intentionally fails closed.
    pub(super) package_persistence_macros: std::sync::Arc<BTreeMap<String, TargetSet>>,
}

impl Default for ModuleSymbols {
    fn default() -> Self {
        let mut type_aliases = BTreeMap::new();
        for target in [
            PersistenceTarget::MySqlPool,
            PersistenceTarget::PgPool,
            PersistenceTarget::DatabaseConnection,
        ] {
            type_aliases.insert(target.source_name().to_owned(), BTreeSet::from([target]));
        }
        Self {
            type_aliases,
            nominal_type_aliases: BTreeMap::new(),
            type_alias_info: BTreeMap::new(),
            path_aliases: BTreeMap::new(),
            traits_in_scope: BTreeMap::new(),
            anonymous_traits_in_scope: BTreeSet::new(),
            module_path: Vec::new(),
            field_targets: BTreeMap::new(),
            field_owners: BTreeMap::new(),
            tuple_field_targets: BTreeMap::new(),
            field_nominal_types: BTreeMap::new(),
            function_returns: BTreeMap::new(),
            function_called_inputs: BTreeMap::new(),
            function_mutable_writes: BTreeMap::new(),
            method_returns: BTreeMap::new(),
            method_mutable_receivers: BTreeMap::new(),
            method_mutable_writes: BTreeMap::new(),
            function_generic_params: BTreeMap::new(),
            function_generic_input_params: BTreeMap::new(),
            method_generic_params: BTreeMap::new(),
            method_generic_input_params: BTreeMap::new(),
            trait_method_returns: std::sync::Arc::new(BTreeMap::new()),
            trait_supertraits: std::sync::Arc::new(BTreeMap::new()),
            trait_generic_params: std::sync::Arc::new(BTreeMap::new()),
            trait_method_generic_params: std::sync::Arc::new(BTreeMap::new()),
            trait_method_generic_input_params: std::sync::Arc::new(BTreeMap::new()),
            named_type_info: std::sync::Arc::new(BTreeMap::new()),
            workspace_named_type_info: std::sync::Arc::new(BTreeMap::new()),
            dependency_crate_aliases: std::sync::Arc::new(BTreeMap::new()),
            package_function_returns: std::sync::Arc::new(BTreeMap::new()),
            package_function_mutable_writes: std::sync::Arc::new(BTreeMap::new()),
            package_function_generic_params: std::sync::Arc::new(BTreeMap::new()),
            package_function_generic_input_params: std::sync::Arc::new(BTreeMap::new()),
            package_method_returns: std::sync::Arc::new(BTreeMap::new()),
            package_method_mutable_receivers: std::sync::Arc::new(BTreeMap::new()),
            package_method_mutable_writes: std::sync::Arc::new(BTreeMap::new()),
            package_method_generic_params: std::sync::Arc::new(BTreeMap::new()),
            package_method_generic_input_params: std::sync::Arc::new(BTreeMap::new()),
            item_values: BTreeMap::new(),
            package_item_values: std::sync::Arc::new(BTreeMap::new()),
            sqlx_namespaces: BTreeSet::from(["sqlx".to_owned()]),
            workspace_sqlx_namespaces: std::sync::Arc::new(BTreeSet::new()),
            database_namespaces: BTreeSet::from(["wow_database".to_owned()]),
            query_callables: BTreeSet::new(),
            local_macro_definitions: BTreeSet::new(),
            local_type_definitions: BTreeSet::new(),
            persistence_macros: BTreeMap::new(),
            package_persistence_macros: std::sync::Arc::new(BTreeMap::new()),
        }
    }
}

impl ModuleSymbols {
    pub(super) fn for_package(package: &str) -> Self {
        let mut symbols = Self::default();
        if package == "wow-database" {
            symbols.database_namespaces.insert("crate".to_owned());
            for target in [
                PersistenceTarget::Database,
                PersistenceTarget::LoginDatabase,
                PersistenceTarget::WorldDatabase,
                PersistenceTarget::CharacterDatabase,
                PersistenceTarget::HotfixDatabase,
                PersistenceTarget::LoginStatements,
                PersistenceTarget::WorldStatements,
                PersistenceTarget::CharStatements,
                PersistenceTarget::HotfixStatements,
                PersistenceTarget::PreparedStatement,
                PersistenceTarget::SqlParam,
                PersistenceTarget::SqlTransaction,
                PersistenceTarget::SqlTransactionCommitError,
                PersistenceTarget::SqlResult,
                PersistenceTarget::SqlFields,
                PersistenceTarget::SqlQueryHolder,
                PersistenceTarget::SqlQueryHolderResult,
                PersistenceTarget::StatementDef,
                PersistenceTarget::DatabaseError,
                PersistenceTarget::ItemGuidAllocatorAdvisoryLockLikeCpp,
            ] {
                symbols
                    .type_aliases
                    .insert(target.source_name().to_owned(), BTreeSet::from([target]));
            }
        }
        symbols
    }
}

pub(super) fn is_query_name(name: &str) -> bool {
    QUERY_CONSTRUCTORS.contains(&name) || name.starts_with("query_")
}

pub(super) fn database_getter_target(name: &str) -> Option<PersistenceTarget> {
    match name {
        "login_db" | "login_database" => Some(PersistenceTarget::LoginDatabase),
        "world_db" | "world_database" => Some(PersistenceTarget::WorldDatabase),
        "char_db" | "character_db" | "character_database" => {
            Some(PersistenceTarget::CharacterDatabase)
        }
        "hotfix_db" | "hotfix_database" => Some(PersistenceTarget::HotfixDatabase),
        _ => None,
    }
}

pub(super) fn database_field_target(name: &str) -> Option<PersistenceTarget> {
    match name {
        "login_db" => Some(PersistenceTarget::LoginDatabase),
        "world_db" => Some(PersistenceTarget::WorldDatabase),
        "char_db" | "character_db" => Some(PersistenceTarget::CharacterDatabase),
        "hotfix_db" => Some(PersistenceTarget::HotfixDatabase),
        _ => None,
    }
}

pub(super) fn sqlx_pool_options_target(names: &[String]) -> Option<PersistenceTarget> {
    names.iter().find_map(|name| match name.as_str() {
        "MySqlPoolOptions" => Some(PersistenceTarget::MySqlPool),
        "PgPoolOptions" => Some(PersistenceTarget::PgPool),
        _ => None,
    })
}

pub(super) fn is_generated_id_read_statement(name: &str) -> bool {
    name.starts_with("SEL_MAX_")
        || name.starts_with("SEL_BNET_MAX_")
        || name.contains("_MAXID")
        || name.ends_with("_MAX_NODEID")
        || name.ends_with("_MAX_PATHID")
}

pub(super) fn is_flow_passthrough_call(names: &[String]) -> bool {
    let suffix = names.iter().rev().take(2).collect::<Vec<_>>();
    matches!(
        suffix.as_slice(),
        [method, owner]
            if matches!(
                (owner.as_str(), method.as_str()),
                ("Arc", "clone")
                    | ("Arc", "new")
                    | ("Rc", "clone")
                    | ("Rc", "new")
                    | ("Box", "new")
                    | ("Mutex", "new")
                    | ("RwLock", "new")
                    | ("Cell", "new")
                    | ("RefCell", "new")
                    | ("UnsafeCell", "new")
                    | ("ManuallyDrop", "new")
                    | ("Pin", "new")
                    | ("Option", "Some")
                    | ("Result", "Ok")
                    // An error payload carries persistence out of a function
                    // exactly as a success payload does, and `?` returns it.
                    | ("Result", "Err")
                    | ("ControlFlow", "Break")
                    | ("ControlFlow", "Continue")
            )
    ) || matches!(names, [name] if matches!(name.as_str(), "Some" | "Ok" | "Err" | "Break" | "Continue"))
        || is_standard_identity(names)
}

pub(super) fn is_standard_identity(names: &[String]) -> bool {
    matches!(
        names,
        [root, module, function]
            if matches!(root.as_str(), "std" | "core")
                && module == "convert"
                && function == "identity"
    )
}

pub(super) fn is_standard_replacement(names: &[String]) -> bool {
    matches!(
        names,
        [root, module, function]
            if matches!(root.as_str(), "std" | "core")
                && module == "mem"
                && matches!(function.as_str(), "replace" | "take")
    )
}
