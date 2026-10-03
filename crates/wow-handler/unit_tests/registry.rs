// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

use wow_constants::ClientOpcodes;
use wow_packet::WorldPacket;

use crate::{
    DuplicateHandlerRegistrationLikeCpp, HandlerFuture, PacketHandlerEntry, PacketHandlerFn,
    PacketProcessing, RegistryBuilder, SessionStatus,
};

struct Host {
    observed: Vec<i32>,
}

struct Catalog {
    value: i32,
}

struct YieldOnce(bool);

impl Future for YieldOnce {
    type Output = ();

    fn poll(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        if this.0 {
            Poll::Ready(())
        } else {
            this.0 = true;
            context.waker().wake_by_ref();
            Poll::Pending
        }
    }
}

fn mutate_host<'a>(
    host: &'a mut Host,
    catalog: &'a Catalog,
    _packet: WorldPacket,
) -> HandlerFuture<'a, ()> {
    Box::pin(async move {
        host.observed.push(catalog.value);
        YieldOnce(false).await;
        host.observed.push(catalog.value * 2);
    })
}

fn handler_entry<S, C>(
    opcode: ClientOpcodes,
    handler_name: &'static str,
    handler: PacketHandlerFn<S, C>,
) -> PacketHandlerEntry<S, C> {
    PacketHandlerEntry {
        opcode,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name,
        handler,
    }
}

fn empty_handler<'a>(
    _host: &'a mut Host,
    _catalog: &'a Catalog,
    _packet: WorldPacket,
) -> HandlerFuture<'a, ()> {
    Box::pin(async {})
}

#[test]
fn copied_entry_invokes_future_borrowing_nonclone_host_and_catalog() {
    let entry = handler_entry(
        ClientOpcodes::QueryTime,
        "borrowed_probe",
        mutate_host,
    );
    let copied_entry = entry;
    let cloned_entry = entry.clone();

    let mut host = Host {
        observed: Vec::new(),
    };
    let catalog = Catalog { value: 13 };
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);
    let mut future = Box::pin((copied_entry.handler)(
        &mut host,
        &catalog,
        WorldPacket::new_empty(),
    ));
    assert!(matches!(future.as_mut().poll(&mut context), Poll::Pending));
    assert!(matches!(future.as_mut().poll(&mut context), Poll::Ready(())));
    drop(future);

    assert_eq!(host.observed, vec![13, 26]);
    assert_eq!(entry.handler_name, copied_entry.handler_name);
    assert_eq!(entry.handler_name, cloned_entry.handler_name);
}

#[test]
fn registry_lookup_preserves_metadata_and_invokes_registered_future() {
    let mut builder = RegistryBuilder::<Host, Catalog>::new();
    builder
        .register(handler_entry(
            ClientOpcodes::QueryTime,
            "query_time_probe",
            mutate_host,
        ))
        .unwrap();
    let registry = builder.build();

    assert_eq!(registry.len(), 1);
    assert!(!registry.is_empty());
    assert!(registry.contains(ClientOpcodes::QueryTime));
    assert!(!registry.contains(ClientOpcodes::CastSpell));
    assert!(registry.get(ClientOpcodes::CastSpell).is_none());

    let entry = registry.get(ClientOpcodes::QueryTime).unwrap();
    assert_eq!(entry.opcode, ClientOpcodes::QueryTime);
    assert_eq!(entry.status, SessionStatus::LoggedIn);
    assert_eq!(entry.processing, PacketProcessing::ThreadUnsafe);
    assert_eq!(entry.handler_name, "query_time_probe");

    let mut host = Host {
        observed: Vec::new(),
    };
    let catalog = Catalog { value: 7 };
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);
    let mut future = Box::pin((entry.handler)(
        &mut host,
        &catalog,
        WorldPacket::new_empty(),
    ));
    assert!(matches!(future.as_mut().poll(&mut context), Poll::Pending));
    assert!(matches!(future.as_mut().poll(&mut context), Poll::Ready(())));
    drop(future);
    assert_eq!(host.observed, vec![7, 14]);
}

#[test]
fn duplicate_registration_reports_names_without_replacing_original() {
    let mut builder = RegistryBuilder::<Host, Catalog>::default();
    builder
        .register(handler_entry(
            ClientOpcodes::QueryTime,
            "first_query_time",
            empty_handler,
        ))
        .unwrap();

    let error: DuplicateHandlerRegistrationLikeCpp = builder
        .register(handler_entry(
            ClientOpcodes::QueryTime,
            "second_query_time",
            empty_handler,
        ))
        .unwrap_err();

    assert_eq!(error.opcode, ClientOpcodes::QueryTime);
    assert_eq!(error.previous_handler_name, "first_query_time");
    assert_eq!(error.new_handler_name, "second_query_time");
    assert!(format!("{error:?}").contains("first_query_time"));
    assert!(format!("{error}").contains("second_query_time"));

    let registry = builder.build();
    let original = registry.get(ClientOpcodes::QueryTime).unwrap();
    assert_eq!(original.handler_name, "first_query_time");
    assert_eq!(original.status, SessionStatus::LoggedIn);
}

#[test]
fn empty_registry_builds_and_iter_exposes_registered_values() {
    let empty = RegistryBuilder::<Host, Catalog>::new().build();
    assert!(empty.is_empty());
    assert_eq!(empty.len(), 0);
    assert_eq!(empty.iter().count(), 0);

    let mut builder = RegistryBuilder::<Host, Catalog>::new();
    builder
        .register(handler_entry(
            ClientOpcodes::QueryTime,
            "query_time",
            empty_handler,
        ))
        .unwrap();
    builder
        .register(handler_entry(
            ClientOpcodes::CastSpell,
            "cast_spell",
            empty_handler,
        ))
        .unwrap();
    let registry = builder.build();

    let mut names: Vec<_> = registry.iter().map(|entry| entry.handler_name).collect();
    names.sort_unstable();
    assert_eq!(names.as_slice(), &["cast_spell", "query_time"]);
}

#[test]
fn dropping_pending_registered_handler_releases_host_without_second_mutation() {
    let mut builder = RegistryBuilder::<Host, Catalog>::new();
    builder
        .register(handler_entry(
            ClientOpcodes::QueryTime,
            "cancelled_query_time",
            mutate_host,
        ))
        .unwrap();
    let registry = builder.build();
    let copied_entry = *registry.get(ClientOpcodes::QueryTime).unwrap();

    let mut host = Host {
        observed: Vec::new(),
    };
    let catalog = Catalog { value: 19 };
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);
    let mut future = Box::pin((copied_entry.handler)(
        &mut host,
        &catalog,
        WorldPacket::new_empty(),
    ));

    assert!(matches!(future.as_mut().poll(&mut context), Poll::Pending));
    drop(future);

    assert_eq!(host.observed, vec![catalog.value]);
}
