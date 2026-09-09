//! Persistence flow stages and SQL expression classification.
//!
//! Separated from the persistence-access root under #634. Behaviour is
//! preserved; this module owns no new state.

use super::*;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) enum FlowStage {
    Pool,
    Query,
    Transaction,
    DerivedPool,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(super) struct Flow(pub(super) BTreeSet<(PersistenceTarget, FlowStage)>);

impl Flow {
    pub(super) fn pools(targets: &TargetSet) -> Self {
        Self(
            targets
                .iter()
                .copied()
                .filter(|target| target.carries_persistence_flow())
                .map(|target| (target, FlowStage::Pool))
                .collect(),
        )
    }

    pub(super) fn query() -> Self {
        Self(BTreeSet::from([(
            PersistenceTarget::Sqlx,
            FlowStage::Query,
        )]))
    }

    pub(super) fn union(&mut self, other: Self) {
        self.0.extend(other.0);
    }

    pub(super) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub(super) fn targets(&self) -> TargetSet {
        self.0.iter().map(|(target, _)| *target).collect()
    }

    pub(super) fn pool_targets(&self) -> TargetSet {
        self.0
            .iter()
            .filter_map(|(target, stage)| {
                matches!(stage, FlowStage::Pool | FlowStage::DerivedPool).then_some(*target)
            })
            .collect()
    }

    pub(super) fn has_stage(&self, stage: FlowStage) -> bool {
        self.0.iter().any(|(_, current)| *current == stage)
    }

    pub(super) fn map_pool_stage(&self, stage: FlowStage) -> Self {
        Self(
            self.0
                .iter()
                .filter_map(|(target, current)| {
                    matches!(current, FlowStage::Pool | FlowStage::DerivedPool)
                        .then_some((*target, stage))
                })
                .collect(),
        )
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub(super) enum SqlExpressionKind {
    #[default]
    Static,
    Nonliteral,
    Included,
    Environment,
    Interpolated,
}

/// The SQL an item initializer pins.
///
/// A macro is identified by the path it resolves to: somebody's
/// `other::concat!` expands to whatever it likes, so treating it as the
/// standard one would pin a statement that its definition can change.
pub(super) fn source_sql_info(
    expression: &Expr,
    resolve_standard_macro: &dyn Fn(Vec<String>) -> Option<String>,
) -> (SqlExpressionKind, BTreeSet<String>) {
    match expression {
        Expr::Lit(literal) if matches!(literal.lit, syn::Lit::Str(_)) => (
            SqlExpressionKind::Static,
            BTreeSet::from([normalized_tokens(expression)]),
        ),
        Expr::Reference(reference) => source_sql_info(&reference.expr, resolve_standard_macro),
        Expr::Paren(paren) => source_sql_info(&paren.expr, resolve_standard_macro),
        Expr::Group(group) => source_sql_info(&group.expr, resolve_standard_macro),
        Expr::Macro(mac) => {
            let leaf = last_path_name(&mac.mac.path).unwrap_or_default();
            // Only the standard compile-time macros pin a statement; a
            // namesake in another module is a nonliteral source.
            let name = match leaf.as_str() {
                "concat" | "stringify"
                    if resolve_standard_macro(path_names(&mac.mac.path)).is_none() =>
                {
                    String::new()
                }
                _ => leaf,
            };
            match name.as_str() {
                "env" => (SqlExpressionKind::Environment, BTreeSet::new()),
                "include_str" => (SqlExpressionKind::Included, BTreeSet::new()),
                "format" | "format_args" => (SqlExpressionKind::Interpolated, BTreeSet::new()),
                "stringify" => (
                    SqlExpressionKind::Static,
                    BTreeSet::from([normalized_tokens(expression)]),
                ),
                // The macro tokens already carry the arguments in order, so a
                // swapped pair of statements changes the source text itself.
                // The kind still comes from the arguments: a nested `env!` or
                // `include_str!` puts the statement outside the snapshot.
                "concat" => {
                    let Ok(arguments) =
                        syn::punctuated::Punctuated::<Expr, syn::Token![,]>::parse_terminated
                            .parse2(mac.mac.tokens.clone())
                    else {
                        return (SqlExpressionKind::Nonliteral, BTreeSet::new());
                    };
                    let kind =
                        arguments
                            .iter()
                            .fold(SqlExpressionKind::Static, |kind, argument| {
                                kind.max(source_sql_info(argument, resolve_standard_macro).0)
                            });
                    let sources = match kind {
                        SqlExpressionKind::Static => {
                            BTreeSet::from([normalized_tokens(expression)])
                        }
                        _ => BTreeSet::new(),
                    };
                    (kind, sources)
                }
                _ => (SqlExpressionKind::Nonliteral, BTreeSet::new()),
            }
        }
        _ => (SqlExpressionKind::Nonliteral, BTreeSet::new()),
    }
}
