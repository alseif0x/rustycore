//! Regressions for tuple projection.

use super::*;

#[test]
fn persistence_inventory_tracks_numeric_tuple_fields() {
    let baseline = inventory(
        r#"
                struct Holder(u8, wow_database::CharacterDatabase);
                enum Wrapped { Database(u8, wow_database::CharacterDatabase) }
                fn leak(holder: &Holder) { holder.1.pool(); }
            "#,
    )
    .unwrap();
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "struct Holder"
            && row.operation == PersistenceOperation::TypeReference
            && row.symbol == "1"
    }));
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "enum Wrapped::Database"
            && row.operation == PersistenceOperation::TypeReference
            && row.symbol == "1"
    }));
    let found = operations(&baseline);
    assert!(found.contains(&(
        PersistenceTarget::CharacterDatabase,
        PersistenceOperation::PathReference,
        "1".to_owned(),
    )));
    assert!(found.contains(&(
        PersistenceTarget::CharacterDatabase,
        PersistenceOperation::PoolAccess,
        "pool".to_owned(),
    )));

    let collision = inventory(
        r#"
                struct DatabaseHolder(wow_database::CharacterDatabase);
                struct Innocent(u8);
                fn clean(value: &Innocent) { consume(value.0); }
            "#,
    )
    .unwrap();
    assert!(
        !collision.accesses.iter().any(|row| {
            row.enclosing == "fn clean"
                && matches!(
                    row.operation,
                    PersistenceOperation::PathReference | PersistenceOperation::ArgumentEscape
                )
        }),
        "tuple fields from unrelated owner types must not contaminate each other: {:#?}",
        collision.accesses
    );

    let method_name_collisions = inventory(
        r#"
                struct DbHolder(u8, wow_database::CharacterDatabase);
                struct Plain(u8, u8);
                struct DbFactory;
                impl DbFactory {
                    fn make(&self) -> DbHolder { unreachable!() }
                    fn get(&self) -> wow_database::CharacterDatabase { unreachable!() }
                }
                struct PlainFactory;
                impl PlainFactory {
                    fn make(&self) -> Plain { Plain(0, 0) }
                    fn get(&self) -> u8 { 0 }
                }
                fn clean(factory: &PlainFactory) {
                    consume(factory.make().1);
                    consume(factory.get());
                }
            "#,
    )
    .unwrap();
    assert!(
        !method_name_collisions
            .accesses
            .iter()
            .any(|row| row.enclosing == "fn clean"),
        "methods with the same name on unrelated receiver types must not contaminate each other: {:#?}",
        method_name_collisions.accesses
    );

    let aliases_and_constructors = inventory(
        r#"
                struct Holder(u8, wow_database::CharacterDatabase);
                type Alias = Holder;
                fn alias(value: &Alias) { value.1.pool(); }
                fn constructed(database: wow_database::CharacterDatabase) {
                    let value = Holder(0, database);
                    value.1.pool();
                }
                fn make(database: wow_database::CharacterDatabase) -> Holder {
                    Holder(0, database)
                }
                fn returned(database: wow_database::CharacterDatabase) {
                    make(database).1.pool();
                }
                struct Factory(wow_database::CharacterDatabase);
                impl Factory {
                    fn make(&self) -> Holder { Holder(0, self.0.clone()) }
                    fn self_returned(&self) { self.make().1.pool(); }
                    fn associated_make() -> Holder { unreachable!() }
                    fn self_associated_returned() { Self::associated_make().1.pool(); }
                }
                fn method_returned(factory: &Factory) { factory.make().1.pool(); }
                fn associated_returned() { Factory::associated_make().1.pool(); }
                struct UnitFactory;
                impl UnitFactory {
                    fn make(&self) -> Holder { unreachable!() }
                }
                fn inline_unit_returned() { UnitFactory.make().1.pool(); }
                struct StructFactory {}
                impl StructFactory {
                    fn make(&self) -> Holder { unreachable!() }
                }
                fn inline_struct_returned() { StructFactory {}.make().1.pool(); }
                struct Outer { factory: Factory }
                fn field_receiver(value: &Outer) { value.factory.make().1.pool(); }
                struct TupleOuter(Factory);
                fn tuple_field_receiver(value: &TupleOuter) { value.0.make().1.pool(); }
                fn deref_receiver(factory: &Factory) { (*factory).make().1.pool(); }
                fn destructured_receiver(value: Outer) {
                    let Outer { factory } = value;
                    factory.make().1.pool();
                }
                fn tuple_destructured_receiver(value: TupleOuter) {
                    let TupleOuter(factory) = value;
                    factory.make().1.pool();
                }
                trait Maker { fn make() -> Holder; }
                impl Maker for Factory { fn make() -> Holder { unreachable!() } }
                fn ufcs_returned() { <Factory as Maker>::make().1.pool(); }
                struct DbOuter { database: wow_database::CharacterDatabase }
                fn destructured_database(value: DbOuter) {
                    let DbOuter { database } = value;
                    database.pool();
                }
                fn build_factory() -> Factory { unreachable!() }
                fn typed_local_receiver() {
                    let factory: Factory = build_factory();
                    factory.make().1.pool();
                }
                fn boxed_receiver(factory: Box<Factory>) { factory.make().1.pool(); }
                enum Receivers {
                    Tuple(Factory),
                    Named { factory: Factory },
                }
                fn enum_tuple_receiver(value: Receivers) {
                    if let Receivers::Tuple(factory) = value {
                        factory.make().1.pool();
                    }
                }
                fn enum_named_receiver(value: Receivers) {
                    if let Receivers::Named { factory } = value {
                        factory.make().1.pool();
                    }
                }
                struct Plan { statements: Vec<wow_database::PreparedStatement> }
                struct Planner;
                impl Planner {
                    fn plan(&self) -> Option<Plan> { None }
                    fn consume_plan(&self) {
                        let Some(plan) = self.plan() else { return };
                        consume(plan.statements);
                    }
                }
                struct Job { statement: wow_database::PreparedStatement }
                async fn consume_received(rx: &Receiver<Job>) {
                    while let Some(job) = rx.recv().await {
                        consume(job.statement);
                    }
                }
                fn consume_channel() {
                    let (_, mut rx) = unbounded_channel::<Job>();
                    async move {
                        while let Some(job) = rx.recv().await {
                            consume(job.statement);
                        }
                    };
                }
                fn nested_plan() -> Option<Option<Plan>> { None }
                fn consume_nested_plan() {
                    let Some(inner) = nested_plan() else { return };
                    let Some(plan) = inner else { return };
                    consume(plan.statements);
                }
                async fn consume_result(rx: &Receiver<Result<u8, Job>>) {
                    while let Some(Err(job)) = rx.recv().await {
                        consume(job.statement);
                    }
                }
                fn inferred_wrappers(job: Job) {
                    let wrapped = Some(job);
                    let Some(job) = wrapped else { return };
                    consume(job.statement);
                }
                fn inferred_err(job: Job) {
                    let wrapped: Result<u8, Job> = Err(job);
                    let Err(job) = wrapped else { return };
                    consume(job.statement);
                }
                fn inferred_tuple(factory: Factory) {
                    let pair = (factory, 0_u8);
                    let (factory, _) = pair;
                    factory.make().1.pool();
                }
                fn inferred_wrapped_tuple(factory: Factory) {
                    let wrapped = Some((factory, 0_u8));
                    let Some((factory, _)) = wrapped else { return };
                    factory.make().1.pool();
                }
                fn pair() -> (Factory, u8) { unreachable!() }
                fn tuple_returned() {
                    let (factory, _) = pair();
                    factory.make().1.pool();
                }
                impl Factory {
                    fn pair(&self) -> (Factory, u8) { unreachable!() }
                    fn method_tuple_returned(&self) {
                        let (factory, _) = self.pair();
                        factory.make().1.pool();
                    }
                }
                fn tuple_parameter(pair: (Factory, u8)) {
                    let (factory, _) = pair;
                    factory.make().1.pool();
                }
                fn referenced_tuple_parameter(pair: &(Factory, u8)) {
                    let (factory, _) = pair;
                    factory.make().1.pool();
                }
                fn wrapped_pair() -> Option<(Factory, u8)> { None }
                fn wrapped_tuple_returned() {
                    let Some((factory, _)) = wrapped_pair() else { return };
                    factory.make().1.pool();
                }
                fn reversed_pair() -> (u8, Factory) { unreachable!() }
                fn reversed_tuple_returned() {
                    let (_, factory) = reversed_pair();
                    factory.make().1.pool();
                }
                type PairAlias = (Factory, u8);
                type NestedPairAlias = PairAlias;
                fn aliased_pair() -> NestedPairAlias { unreachable!() }
                fn aliased_tuple_returned() {
                    let (factory, _) = aliased_pair();
                    factory.make().1.pool();
                }
                type WrappedPairAlias = Option<PairAlias>;
                fn aliased_wrapped_pair() -> WrappedPairAlias { None }
                fn aliased_wrapped_tuple_returned() {
                    let Some((factory, _)) = aliased_wrapped_pair() else { return };
                    factory.make().1.pool();
                }
                fn qualified_enum(value: Receivers) {
                    if let self::Receivers::Named { factory } = value {
                        factory.make().1.pool();
                    }
                }
            "#,
    )
    .unwrap();
    for enclosing in [
        "fn alias",
        "fn constructed",
        "fn returned",
        "fn method_returned",
        "impl Factory::self_returned",
        "impl Factory::self_associated_returned",
        "fn associated_returned",
        "fn inline_unit_returned",
        "fn inline_struct_returned",
        "fn field_receiver",
        "fn tuple_field_receiver",
        "fn deref_receiver",
        "fn destructured_receiver",
        "fn tuple_destructured_receiver",
        "fn ufcs_returned",
        "fn destructured_database",
        "fn typed_local_receiver",
        "fn boxed_receiver",
        "fn enum_tuple_receiver",
        "fn enum_named_receiver",
        "fn qualified_enum",
        "fn tuple_returned",
        "impl Factory::method_tuple_returned",
        "fn tuple_parameter",
        "fn referenced_tuple_parameter",
        "fn wrapped_tuple_returned",
        "fn reversed_tuple_returned",
        "fn aliased_tuple_returned",
        "fn aliased_wrapped_tuple_returned",
    ] {
        assert!(
            aliases_and_constructors.accesses.iter().any(|row| {
                row.enclosing == enclosing
                    && row.target == PersistenceTarget::CharacterDatabase
                    && row.operation == PersistenceOperation::PoolAccess
            }),
            "tuple-field owner lost in {enclosing}: {:#?}",
            aliases_and_constructors.accesses
        );
    }
    for enclosing in [
        "impl Planner::consume_plan",
        "fn consume_received",
        "fn consume_channel",
        "fn consume_nested_plan",
        "fn consume_result",
        "fn inferred_wrappers",
        "fn inferred_err",
    ] {
        assert!(
            aliases_and_constructors.accesses.iter().any(|row| {
                row.enclosing == enclosing
                    && row.target == PersistenceTarget::PreparedStatement
                    && row.operation == PersistenceOperation::PathReference
            }),
            "wrapper payload owner lost in {enclosing}: {:#?}",
            aliases_and_constructors.accesses
        );
    }
    for enclosing in ["fn inferred_tuple", "fn inferred_wrapped_tuple"] {
        assert!(aliases_and_constructors.accesses.iter().any(|row| {
            row.enclosing == enclosing
                && row.target == PersistenceTarget::CharacterDatabase
                && row.operation == PersistenceOperation::PoolAccess
        }));
    }

    let field_owner_collision = inventory(
        r#"
                struct Holder(wow_database::CharacterDatabase);
                struct DbFactory;
                impl DbFactory { fn make(&self) -> Holder { unreachable!() } }
                struct PlainFactory;
                impl PlainFactory { fn make(&self) -> u8 { 0 } }
                struct DbOuter { factory: DbFactory }
                struct PlainOuter { factory: PlainFactory }
                fn clean(value: &PlainOuter) { consume(value.factory.make()); }
                struct Wrapper<T>(T);
                impl<T> Wrapper<T> { fn make(&self) -> u8 { 0 } }
                fn generic_clean(value: &Wrapper<DbFactory>) { consume(value.make()); }
                trait PlainMaker { fn make() -> u8; }
                impl PlainMaker for PlainFactory { fn make() -> u8 { 0 } }
                fn ufcs_clean() { consume(<PlainFactory as PlainMaker>::make()); }
                enum DbEvent { Ready { factory: DbFactory } }
                enum PlainEvent { Ready { factory: PlainFactory } }
                fn enum_clean(value: PlainEvent) {
                    if let PlainEvent::Ready { factory } = value {
                        consume(factory.make());
                    }
                }
                struct DbJob { statement: wow_database::PreparedStatement }
                struct MixedFields {
                    database: wow_database::CharacterDatabase,
                    clean: bool,
                }
                fn declared_clean_field(value: MixedFields) {
                    consume(value.clean);
                }
                fn declared_persistent_field(value: MixedFields) {
                    consume(value.database.pool());
                }
                async fn result_sibling_clean(rx: &Receiver<Result<DbJob, u8>>) {
                    while let Some(Err(code)) = rx.recv().await {
                        consume(code);
                    }
                }
                fn db_pair() -> (DbFactory, u8) { unreachable!() }
                fn tuple_sibling_clean() {
                    let (_, code) = db_pair();
                    consume(code);
                }
                type DbPairAlias = (DbFactory, u8);
                fn aliased_db_pair() -> DbPairAlias { unreachable!() }
                fn aliased_tuple_sibling_clean() {
                    let (_, code) = aliased_db_pair();
                    consume(code);
                }
                struct TraitFactory;
                trait DbMaker { fn make(&self) -> Holder; }
                trait PlainTraitMaker { fn make(&self) -> u8; }
                impl DbMaker for TraitFactory {
                    fn make(&self) -> Holder { unreachable!() }
                }
                impl PlainTraitMaker for TraitFactory {
                    fn make(&self) -> u8 { 0 }
                }
                fn trait_ufcs_clean(factory: &TraitFactory) {
                    consume(<TraitFactory as PlainTraitMaker>::make(factory));
                }
                mod db_trait {
                    pub trait Maker { fn make(&self) -> super::Holder; }
                    pub trait Derived: Maker {}
                    pub trait Chained: Maker + Sized { fn again(&self) -> Option<Self>; }
                }
                mod plain_trait {
                    pub trait Maker { fn make(&self) -> u8; }
                    pub trait Derived: Maker {}
                    pub trait Chained: Maker + Sized { fn again(&self) -> Option<Self>; }
                }
                impl crate::db_trait::Maker for TraitFactory {
                    fn make(&self) -> Holder { unreachable!() }
                }
                impl plain_trait::Maker for TraitFactory {
                    fn make(&self) -> u8 { 0 }
                }
                fn qualified_trait_ufcs_persistent(factory: &TraitFactory) {
                    consume(<TraitFactory as db_trait::Maker>::make(factory).0.pool());
                }
                fn qualified_trait_ufcs_clean(factory: &TraitFactory) {
                    consume(<TraitFactory as plain_trait::Maker>::make(factory));
                }
                use plain_trait::Maker as ImportedPlainMaker;
                fn imported_trait_ufcs_clean(factory: &TraitFactory) {
                    consume(<TraitFactory as ImportedPlainMaker>::make(factory));
                }
                fn dyn_trait_persistent(factory: &dyn db_trait::Maker) {
                    consume(factory.make().0.pool());
                }
                fn generic_trait_persistent<T: db_trait::Maker>(factory: &T) {
                    consume(factory.make().0.pool());
                }
                fn where_trait_persistent<T>(factory: &T)
                where
                    T: db_trait::Maker,
                {
                    consume(factory.make().0.pool());
                }
                fn dyn_trait_clean(factory: &dyn plain_trait::Maker) {
                    consume(factory.make());
                }
                fn generic_trait_clean<T: plain_trait::Maker>(factory: &T) {
                    consume(factory.make());
                }
                fn dyn_supertrait_persistent(factory: &dyn db_trait::Derived) {
                    consume(factory.make().0.pool());
                }
                fn dyn_supertrait_clean(factory: &dyn plain_trait::Derived) {
                    consume(factory.make());
                }
                fn self_return_persistent<T: db_trait::Chained>(factory: &T) {
                    let Some(next) = factory.again() else { return };
                    consume(next.make().0.pool());
                }
                fn self_return_clean<T: plain_trait::Chained>(factory: &T) {
                    let Some(next) = factory.again() else { return };
                    consume(next.make());
                }
                fn ufcs_self_return_persistent<T: db_trait::Chained>(factory: &T) {
                    let Some(next) = T::again(factory) else { return };
                    consume(next.make().0.pool());
                }
                fn persistent_provider() -> impl db_trait::Maker { unreachable!() }
                fn plain_provider() -> impl plain_trait::Maker { unreachable!() }
                fn impl_trait_function_persistent() {
                    consume(persistent_provider().make().0.pool());
                }
                fn impl_trait_function_clean() {
                    consume(plain_provider().make());
                }
                struct Provider;
                impl Provider {
                    fn persistent(&self) -> impl db_trait::Maker { unreachable!() }
                    fn plain(&self) -> impl plain_trait::Maker { unreachable!() }
                }
                fn impl_trait_method_persistent(provider: &Provider) {
                    consume(provider.persistent().make().0.pool());
                }
                fn impl_trait_method_clean(provider: &Provider) {
                    consume(provider.plain().make());
                }
                mod namespace_self_scope {
                    use crate::db_trait::{self};
                    fn namespace_self_persistent(factory: &dyn db_trait::Maker) {
                        consume(factory.make().0.pool());
                    }
                }
                struct ImplBound<T>(T);
                impl<T> ImplBound<T>
                where
                    T: db_trait::Maker,
                {
                    fn impl_bound_persistent(&self) {
                        consume(self.0.make().0.pool());
                    }
                }
                mod db_method_scope {
                    use super::db_trait::Maker;
                    fn scoped_trait_method_persistent(factory: &super::TraitFactory) {
                        consume(factory.make().0.pool());
                    }
                }
                mod plain_method_scope {
                    use super::plain_trait::Maker;
                    fn scoped_trait_method_clean(factory: &super::TraitFactory) {
                        consume(factory.make());
                    }
                }
                mod module_anonymous_method_scope {
                    mod marker_trait {
                        pub trait Marker { fn marker(&self); }
                    }
                    impl marker_trait::Marker for super::TraitFactory {
                        fn marker(&self) {}
                    }
                    use super::db_trait::Maker as _;
                    use marker_trait::Marker as _;
                    fn anonymous_module_traits_are_additive(factory: &super::TraitFactory) {
                        factory.marker();
                        consume(factory.make().0.pool());
                    }
                }
                mod local_method_scope {
                    mod marker_trait {
                        pub trait Marker { fn marker(&self); }
                    }
                    impl marker_trait::Marker for super::TraitFactory {
                        fn marker(&self) {}
                    }
                    fn local_trait_method_persistent(factory: &super::TraitFactory) {
                        use super::db_trait::Maker;
                        consume(factory.make().0.pool());
                    }
                    fn nested_local_trait_method_persistent(factory: &super::TraitFactory) {
                        {
                            use super::db_trait::Maker;
                            consume(factory.make().0.pool());
                        }
                    }
                    fn local_trait_method_clean(factory: &super::TraitFactory) {
                        use super::plain_trait::Maker;
                        consume(factory.make());
                    }
                    fn disabled_local_trait_is_ignored(factory: &super::TraitFactory) {
                        use super::plain_trait::Maker;
                        #[cfg(any())]
                        use super::db_trait::Maker;
                        consume(factory.make());
                    }
                    fn disabled_anonymous_trait_is_ignored(factory: &super::TraitFactory) {
                        use super::plain_trait::Maker;
                        #[cfg(any())]
                        use super::db_trait::Maker as _;
                        consume(factory.make());
                    }
                    fn anonymous_local_traits_are_additive(factory: &super::TraitFactory) {
                        use super::db_trait::Maker as _;
                        use marker_trait::Marker as _;
                        factory.marker();
                        consume(factory.make().0.pool());
                    }
                    fn local_trait_scope_does_not_escape(factory: &super::TraitFactory) {
                        use super::plain_trait::Maker;
                        {
                            use super::db_trait::Maker;
                            consume(factory.make().0.pool());
                        }
                        consume(factory.make());
                    }
                }
                #[cfg(test)]
                fn test_only_local_trait(factory: &TraitFactory) {
                    #[cfg(test)]
                    use db_trait::Maker;
                    consume(factory.make().0.pool());
                }
            "#,
    )
    .unwrap();
    assert!(
        !field_owner_collision.accesses.iter().any(|row| matches!(
            row.enclosing.as_str(),
            "fn clean"
                | "fn generic_clean"
                | "fn ufcs_clean"
                | "fn enum_clean"
                | "fn declared_clean_field"
                | "fn result_sibling_clean"
                | "fn tuple_sibling_clean"
                | "fn aliased_tuple_sibling_clean"
                | "fn trait_ufcs_clean"
                | "fn qualified_trait_ufcs_clean"
                | "fn imported_trait_ufcs_clean"
                | "fn scoped_trait_method_clean"
                | "fn local_trait_method_clean"
                | "fn disabled_local_trait_is_ignored"
                | "fn disabled_anonymous_trait_is_ignored"
                | "fn dyn_trait_clean"
                | "fn generic_trait_clean"
                | "fn dyn_supertrait_clean"
                | "fn self_return_clean"
                | "fn impl_trait_function_clean"
                | "fn impl_trait_method_clean"
        )),
        "named fields on unrelated owner types must not contaminate receiver identity: {:#?}",
        field_owner_collision.accesses
    );
    assert!(field_owner_collision.accesses.iter().any(|row| {
        row.enclosing == "fn declared_persistent_field"
            && row.target == PersistenceTarget::CharacterDatabase
            && row.operation == PersistenceOperation::PoolAccess
    }));
    assert!(field_owner_collision.accesses.iter().any(|row| {
        row.enclosing == "fn qualified_trait_ufcs_persistent"
            && row.target == PersistenceTarget::CharacterDatabase
            && row.operation == PersistenceOperation::PoolAccess
    }));
    assert!(field_owner_collision.accesses.iter().any(|row| {
        row.enclosing == "fn scoped_trait_method_persistent"
            && row.target == PersistenceTarget::CharacterDatabase
            && row.operation == PersistenceOperation::PoolAccess
    }));
    for enclosing in [
        "fn local_trait_method_persistent",
        "fn nested_local_trait_method_persistent",
        "fn local_trait_scope_does_not_escape",
        "fn anonymous_local_traits_are_additive",
        "fn anonymous_module_traits_are_additive",
        "fn dyn_trait_persistent",
        "fn generic_trait_persistent",
        "fn where_trait_persistent",
        "fn dyn_supertrait_persistent",
        "fn self_return_persistent",
        "fn ufcs_self_return_persistent",
        "fn impl_trait_function_persistent",
        "fn impl_trait_method_persistent",
        "fn namespace_self_persistent",
        "impl ImplBound < T >::impl_bound_persistent",
    ] {
        assert!(
            field_owner_collision.accesses.iter().any(|row| {
                row.enclosing == enclosing
                    && row.target == PersistenceTarget::CharacterDatabase
                    && row.operation == PersistenceOperation::PoolAccess
            }),
            "missing bounded trait return in {enclosing}"
        );
    }
    assert_eq!(
        field_owner_collision
            .accesses
            .iter()
            .filter(|row| {
                row.enclosing == "fn local_trait_scope_does_not_escape"
                    && row.target == PersistenceTarget::CharacterDatabase
                    && row.operation == PersistenceOperation::PoolAccess
            })
            .map(|row| row.count)
            .sum::<usize>(),
        1,
        "a nested persistent trait import must not contaminate its scalar sibling scope"
    );
    assert!(
        field_owner_collision.accesses.iter().any(|row| {
            row.enclosing == "fn test_only_local_trait"
                && row.source_class == "test_fixture"
                && row.target == PersistenceTarget::CharacterDatabase
                && row.operation == PersistenceOperation::PoolAccess
        }),
        "cfg(test) local trait import was not isolated: {:#?}",
        field_owner_collision
            .accesses
            .iter()
            .filter(|row| row.enclosing == "fn test_only_local_trait")
            .collect::<Vec<_>>()
    );
    assert!(!field_owner_collision.accesses.iter().any(|row| {
        row.enclosing == "fn test_only_local_trait"
            && row.source_class == "production"
            && row.target == PersistenceTarget::CharacterDatabase
    }));
}

#[test]
fn persistence_inventory_projects_tuple_patterns_after_rest() {
    let baseline = inventory(
        r#"
                struct Clean;
                fn persistent(database: wow_database::CharacterDatabase) {
                    let tuple = (Clean, Clean, database);
                    let (.., alias) = tuple;
                    consume(alias.pool());
                }
            "#,
    )
    .unwrap();
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn persistent" && row.operation == PersistenceOperation::PoolAccess
    }));
}

#[test]
fn persistence_inventory_projects_nested_generic_tuple_slots_independently() {
    let baseline = inventory(
        r#"
                struct Clean;
                impl Clean { fn pool(self) {} }
                fn first<T, U>(value: (T, U)) -> T { value.0 }
                fn second<T, U>(value: (T, U)) -> U { value.1 }
                fn clean(database: wow_database::CharacterDatabase) {
                    consume(first((Clean, database)).pool());
                }
                fn persistent(database: wow_database::CharacterDatabase) {
                    consume(second((Clean, database)).pool());
                }
            "#,
    )
    .unwrap();
    assert!(!baseline.accesses.iter().any(|row| {
        row.enclosing == "fn clean" && row.operation == PersistenceOperation::PoolAccess
    }));
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn persistent" && row.operation == PersistenceOperation::PoolAccess
    }));
}

#[test]
fn persistence_inventory_projects_tuple_struct_patterns_after_rest() {
    let baseline = inventory(
        r#"
                struct Clean;
                struct Wrapper(Clean, Clean, wow_database::CharacterDatabase);
                fn persistent(wrapper: Wrapper) {
                    let Wrapper(.., alias) = wrapper;
                    consume(alias.pool());
                }
            "#,
    )
    .unwrap();
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn persistent" && row.operation == PersistenceOperation::PoolAccess
    }));
}
