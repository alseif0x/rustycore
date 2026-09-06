//! Canonical persistence-inventory records, accumulation and snapshot comparison.
//! This owns the serialized schema and exact row identity; no AST traversal lives here.

use crate::ownership::cfg_context_allows_production;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

const PERSISTENCE_SCHEMA_VERSION: u32 = 3;

/// Concrete persistence surface represented by an inventory row.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub(crate) enum PersistenceTarget {
    #[serde(rename = "sqlx")]
    Sqlx,
    MySqlPool,
    PgPool,
    DatabaseConnection,
    SqlxTransaction,
    Database,
    LoginDatabase,
    WorldDatabase,
    CharacterDatabase,
    HotfixDatabase,
    LoginStatements,
    WorldStatements,
    CharStatements,
    HotfixStatements,
    PreparedStatement,
    SqlParam,
    SqlTransaction,
    SqlTransactionCommitError,
    SqlResult,
    SqlFields,
    SqlQueryHolder,
    SqlQueryHolderResult,
    StatementDef,
    DatabaseError,
    ItemGuidAllocatorAdvisoryLockLikeCpp,
}

impl PersistenceTarget {
    pub(super) fn from_name(name: &str) -> Option<Self> {
        match name {
            "sqlx" => Some(Self::Sqlx),
            "MySqlPool" => Some(Self::MySqlPool),
            "PgPool" => Some(Self::PgPool),
            "DatabaseConnection" => Some(Self::DatabaseConnection),
            "Transaction" => Some(Self::SqlxTransaction),
            "Database" => Some(Self::Database),
            "LoginDatabase" => Some(Self::LoginDatabase),
            "WorldDatabase" => Some(Self::WorldDatabase),
            "CharacterDatabase" => Some(Self::CharacterDatabase),
            "HotfixDatabase" => Some(Self::HotfixDatabase),
            "LoginStatements" => Some(Self::LoginStatements),
            "WorldStatements" => Some(Self::WorldStatements),
            "CharStatements" => Some(Self::CharStatements),
            "HotfixStatements" => Some(Self::HotfixStatements),
            "PreparedStatement" => Some(Self::PreparedStatement),
            "SqlParam" => Some(Self::SqlParam),
            "SqlTransaction" => Some(Self::SqlTransaction),
            "SqlTransactionCommitError" => Some(Self::SqlTransactionCommitError),
            "SqlResult" => Some(Self::SqlResult),
            "SqlFields" => Some(Self::SqlFields),
            "SqlQueryHolder" => Some(Self::SqlQueryHolder),
            "SqlQueryHolderResult" => Some(Self::SqlQueryHolderResult),
            "StatementDef" => Some(Self::StatementDef),
            "DatabaseError" => Some(Self::DatabaseError),
            "ItemGuidAllocatorAdvisoryLockLikeCpp" => {
                Some(Self::ItemGuidAllocatorAdvisoryLockLikeCpp)
            }
            _ => None,
        }
    }

    pub(super) fn source_name(self) -> &'static str {
        match self {
            Self::Sqlx => "sqlx",
            Self::MySqlPool => "MySqlPool",
            Self::PgPool => "PgPool",
            Self::DatabaseConnection => "DatabaseConnection",
            Self::SqlxTransaction => "Transaction",
            Self::Database => "Database",
            Self::LoginDatabase => "LoginDatabase",
            Self::WorldDatabase => "WorldDatabase",
            Self::CharacterDatabase => "CharacterDatabase",
            Self::HotfixDatabase => "HotfixDatabase",
            Self::LoginStatements => "LoginStatements",
            Self::WorldStatements => "WorldStatements",
            Self::CharStatements => "CharStatements",
            Self::HotfixStatements => "HotfixStatements",
            Self::PreparedStatement => "PreparedStatement",
            Self::SqlParam => "SqlParam",
            Self::SqlTransaction => "SqlTransaction",
            Self::SqlTransactionCommitError => "SqlTransactionCommitError",
            Self::SqlResult => "SqlResult",
            Self::SqlFields => "SqlFields",
            Self::SqlQueryHolder => "SqlQueryHolder",
            Self::SqlQueryHolderResult => "SqlQueryHolderResult",
            Self::StatementDef => "StatementDef",
            Self::DatabaseError => "DatabaseError",
            Self::ItemGuidAllocatorAdvisoryLockLikeCpp => "ItemGuidAllocatorAdvisoryLockLikeCpp",
        }
    }

    pub(super) fn carries_persistence_flow(self) -> bool {
        !matches!(
            self,
            Self::Sqlx
                | Self::LoginStatements
                | Self::WorldStatements
                | Self::CharStatements
                | Self::HotfixStatements
                | Self::SqlParam
                | Self::StatementDef
                | Self::DatabaseError
                | Self::SqlTransactionCommitError
        )
    }
}

/// Exact kind of direct persistence syntax or value escape.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PersistenceOperation {
    Import,
    TypeReference,
    TypeAlias,
    PathReference,
    MacroReference,
    ValueAlias,
    Query,
    Execute,
    Fetch,
    FetchAll,
    FetchMany,
    FetchOne,
    FetchOptional,
    Begin,
    Commit,
    Rollback,
    ArgumentEscape,
    ReturnEscape,
    StoreEscape,
    PoolAccess,
    PrepareStatement,
    DirectQuery,
    DirectExecute,
    RawSql,
    NonliteralSql,
    InterpolatedSql,
    TransactionAppend,
    GeneratedIdRead,
    AdvisoryLock,
    DatabaseOpen,
    TransactionConstruct,
    StatementBuilder,
}

impl PersistenceOperation {
    pub(super) fn from_executor_method(name: &str) -> Option<Self> {
        match name {
            "execute" => Some(Self::Execute),
            "fetch" => Some(Self::Fetch),
            "fetch_all" => Some(Self::FetchAll),
            "fetch_many" => Some(Self::FetchMany),
            "fetch_one" => Some(Self::FetchOne),
            "fetch_optional" => Some(Self::FetchOptional),
            "begin" => Some(Self::Begin),
            "commit" => Some(Self::Commit),
            "rollback" => Some(Self::Rollback),
            "acquire" | "pool" => Some(Self::PoolAccess),
            // `describe` sends the statement for description, so its SQL is
            // inventoried exactly like a prepared one.
            "prepare" | "prepare_with" | "describe" => Some(Self::PrepareStatement),
            "direct_query" => Some(Self::DirectQuery),
            "direct_execute" => Some(Self::DirectExecute),
            "commit_transaction"
            | "commit_with_outcome_like_cpp"
            | "commit_transaction_with_outcome_like_cpp" => Some(Self::Commit),
            "append" | "append_expect_rows_affected" | "execute_or_append" => {
                Some(Self::TransactionAppend)
            }
            "append_raw_sql_like_cpp" | "raw_sql_like_cpp" => Some(Self::RawSql),
            "last_insert_id" => Some(Self::GeneratedIdRead),
            "acquire_like_cpp" | "release_like_cpp" | "wait_until_lost_like_cpp" => {
                Some(Self::AdvisoryLock)
            }
            "open"
            | "open_with_pool_size"
            | "open_with_pool_size_and_auto_create_like_cpp"
            | "from_pool"
            | "connect"
            | "connect_lazy"
            | "connect_with"
            | "connect_lazy_with" => Some(Self::DatabaseOpen),
            "new" => Some(Self::TransactionConstruct),
            "with_capacity_like_cpp" => Some(Self::StatementBuilder),
            _ => None,
        }
    }
}

/// One canonical, counted persistence access row.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PersistenceAccessRecord {
    pub(crate) classification: String,
    pub(crate) source_class: String,
    pub(crate) package: String,
    pub(crate) module: String,
    pub(crate) source: String,
    pub(crate) enclosing: String,
    pub(crate) target: PersistenceTarget,
    pub(crate) operation: PersistenceOperation,
    pub(crate) symbol: String,
    pub(crate) visibility: String,
    pub(crate) cfg: Vec<String>,
    pub(crate) fingerprint: String,
    pub(crate) generated_input: bool,
    pub(crate) count: usize,
}

/// Serializable exact snapshot. Rows are strictly ordered by full identity.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PersistenceAccessBaseline {
    pub(crate) schema_version: u32,
    pub(crate) accesses: Vec<PersistenceAccessRecord>,
}

impl Default for PersistenceAccessBaseline {
    fn default() -> Self {
        Self {
            schema_version: PERSISTENCE_SCHEMA_VERSION,
            accesses: Vec::new(),
        }
    }
}

/// One production/test source mount assigned to a runtime-ledger
/// classification. The repository walker owns file discovery and logical cfg
/// ancestry; this parser inventories production-capable and test-only items
/// as distinct exact baseline rows.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ClassifiedPersistenceSource<'a> {
    pub(crate) classification: &'a str,
    pub(crate) package: &'a str,
    pub(crate) module: &'a str,
    pub(crate) source_path: &'a str,
    pub(crate) inherited_cfg: &'a [String],
    pub(crate) source: &'a str,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct AccessIdentity {
    classification: String,
    source_class: String,
    package: String,
    module: String,
    source: String,
    enclosing: String,
    target: PersistenceTarget,
    operation: PersistenceOperation,
    symbol: String,
    visibility: String,
    cfg: Vec<String>,
    fingerprint: String,
    generated_input: bool,
}

impl PersistenceAccessRecord {
    fn identity(&self) -> AccessIdentity {
        AccessIdentity {
            classification: self.classification.clone(),
            source_class: self.source_class.clone(),
            package: self.package.clone(),
            module: self.module.clone(),
            source: self.source.clone(),
            enclosing: self.enclosing.clone(),
            target: self.target,
            operation: self.operation,
            symbol: self.symbol.clone(),
            visibility: self.visibility.clone(),
            cfg: self.cfg.clone(),
            fingerprint: self.fingerprint.clone(),
            generated_input: self.generated_input,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) enum PersistenceSourceClass {
    Production,
    TestFixture,
}

impl PersistenceSourceClass {
    fn as_str(self) -> &'static str {
        match self {
            Self::Production => "production",
            Self::TestFixture => "test_fixture",
        }
    }
}

#[derive(Clone, Copy)]
pub(super) struct RecordContext<'a> {
    pub(super) classification: &'a str,
    pub(super) source_class: PersistenceSourceClass,
    pub(super) package: &'a str,
    pub(super) module: &'a str,
    pub(super) source: &'a str,
}

pub(super) struct NewAccess<'a> {
    pub(super) enclosing: &'a str,
    pub(super) target: PersistenceTarget,
    pub(super) operation: PersistenceOperation,
    pub(super) symbol: &'a str,
    pub(super) visibility: &'a str,
    pub(super) cfg: &'a [String],
    pub(super) fingerprint: String,
    pub(super) generated_input: bool,
}

#[derive(Default)]
pub(super) struct AccessAccumulator {
    rows: BTreeMap<AccessIdentity, usize>,
    transactions: Vec<Vec<AccessIdentity>>,
}

impl AccessAccumulator {
    pub(super) fn add(&mut self, context: &RecordContext<'_>, access: NewAccess<'_>) {
        // The test view needs production-visible imports and aliases for exact
        // name/value resolution, but those rows already belong to the
        // production view. Only retain syntax that is satisfiable with
        // `cfg(test)` and impossible without it.
        if context.source_class == PersistenceSourceClass::TestFixture
            && cfg_context_allows_production(access.cfg, &[])
                .expect("persistence cfg was validated before recording")
        {
            return;
        }
        let identity = AccessIdentity {
            classification: context.classification.to_owned(),
            source_class: context.source_class.as_str().to_owned(),
            package: context.package.to_owned(),
            module: context.module.to_owned(),
            source: context.source.to_owned(),
            enclosing: access.enclosing.to_owned(),
            target: access.target,
            operation: access.operation,
            symbol: access.symbol.to_owned(),
            visibility: access.visibility.to_owned(),
            cfg: access.cfg.to_vec(),
            fingerprint: access.fingerprint,
            generated_input: access.generated_input,
        };
        *self.rows.entry(identity.clone()).or_insert(0) += 1;
        if let Some(transaction) = self.transactions.last_mut() {
            transaction.push(identity);
        }
    }

    pub(super) fn begin_transaction(&mut self) {
        self.transactions.push(Vec::new());
    }

    pub(super) fn commit_transaction(&mut self) {
        let committed = self
            .transactions
            .pop()
            .expect("persistence access transaction is active");
        if let Some(parent) = self.transactions.last_mut() {
            parent.extend(committed);
        }
    }

    pub(super) fn rollback_transaction(&mut self) {
        let rolled_back = self
            .transactions
            .pop()
            .expect("persistence access transaction is active");
        for identity in rolled_back.into_iter().rev() {
            let count = self
                .rows
                .get_mut(&identity)
                .expect("transactional persistence row was recorded");
            *count -= 1;
            if *count == 0 {
                self.rows.remove(&identity);
            }
        }
    }

    pub(super) fn contains_symbol(&self, enclosing: &str, symbol: &str) -> bool {
        self.rows
            .keys()
            .any(|row| row.enclosing == enclosing && row.symbol == symbol)
    }

    pub(super) fn finish(self) -> PersistenceAccessBaseline {
        PersistenceAccessBaseline {
            schema_version: PERSISTENCE_SCHEMA_VERSION,
            accesses: self
                .rows
                .into_iter()
                .map(|(identity, count)| PersistenceAccessRecord {
                    classification: identity.classification,
                    source_class: identity.source_class,
                    package: identity.package,
                    module: identity.module,
                    source: identity.source,
                    enclosing: identity.enclosing,
                    target: identity.target,
                    operation: identity.operation,
                    symbol: identity.symbol,
                    visibility: identity.visibility,
                    cfg: identity.cfg,
                    fingerprint: identity.fingerprint,
                    generated_input: identity.generated_input,
                    count,
                })
                .collect(),
        }
    }
}

fn validated_baseline_map(
    label: &str,
    baseline: &PersistenceAccessBaseline,
) -> Result<BTreeMap<AccessIdentity, usize>, String> {
    if baseline.schema_version != PERSISTENCE_SCHEMA_VERSION {
        return Err(format!(
            "{label} persistence baseline schema version is {}, expected {PERSISTENCE_SCHEMA_VERSION}",
            baseline.schema_version
        ));
    }
    let mut map = BTreeMap::new();
    let mut previous: Option<AccessIdentity> = None;
    for record in &baseline.accesses {
        if !matches!(record.source_class.as_str(), "production" | "test_fixture") {
            return Err(format!(
                "{label} persistence baseline contains invalid source_class {:?}",
                record.source_class
            ));
        }
        if record.count == 0 {
            return Err(format!(
                "{label} persistence baseline contains zero-count row for {:?} {}",
                record.target, record.symbol
            ));
        }
        let identity = record.identity();
        if previous
            .as_ref()
            .is_some_and(|previous| previous >= &identity)
        {
            return Err(format!(
                "{label} persistence baseline rows are not in strict canonical order near {:?} {}",
                record.target, record.symbol
            ));
        }
        previous = Some(identity.clone());
        if map.insert(identity, record.count).is_some() {
            return Err(format!(
                "{label} persistence baseline contains a duplicate row for {:?} {}",
                record.target, record.symbol
            ));
        }
    }
    Ok(map)
}

fn describe_identity(identity: &AccessIdentity) -> String {
    format!(
        "{} {} {} {} {}::{} {} {:?} {:?} {} [{}]",
        identity.classification,
        identity.source_class,
        identity.package,
        identity.source,
        identity.module,
        identity.enclosing,
        identity.symbol,
        identity.target,
        identity.operation,
        identity.fingerprint,
        identity.cfg.join(", ")
    )
}

/// Compare exact persistence identities and multiplicities in both directions.
pub(crate) fn compare_persistence_access_baseline(
    expected: &PersistenceAccessBaseline,
    actual: &PersistenceAccessBaseline,
) -> Result<(), String> {
    let expected = validated_baseline_map("expected", expected)?;
    let actual = validated_baseline_map("actual", actual)?;
    let mut errors = Vec::new();
    for (identity, actual_count) in &actual {
        match expected.get(identity) {
            None => errors.push(format!(
                "untracked direct persistence access: {} (count {actual_count})",
                describe_identity(identity)
            )),
            Some(expected_count) if expected_count != actual_count => errors.push(format!(
                "direct persistence access multiplicity changed: {} expected {expected_count}, actual {actual_count}",
                describe_identity(identity)
            )),
            Some(_) => {}
        }
    }
    for (identity, expected_count) in &expected {
        if !actual.contains_key(identity) {
            errors.push(format!(
                "obsolete direct persistence baseline row: {} (expected count {expected_count})",
                describe_identity(identity)
            ));
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("\n"))
    }
}

/// Render the large snapshot with one canonical compact JSON object per row.
pub(crate) fn render_persistence_access_baseline(
    baseline: &PersistenceAccessBaseline,
) -> Result<String, String> {
    validated_baseline_map("rendered", baseline)?;
    let mut output = format!(
        "{{\n  \"schema_version\": {},\n  \"accesses\": [\n",
        baseline.schema_version
    );
    for (index, access) in baseline.accesses.iter().enumerate() {
        output.push_str("    ");
        output.push_str(
            &serde_json::to_string(access)
                .map_err(|error| format!("cannot serialize persistence access row: {error}"))?,
        );
        if index + 1 != baseline.accesses.len() {
            output.push(',');
        }
        output.push('\n');
    }
    output.push_str("  ]\n}");
    Ok(output)
}
