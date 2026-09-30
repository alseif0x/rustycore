//! Exercise the original terrain consumer, rather than replacing it with bool.
use super::*;
use std::sync::{Arc, Mutex};
use crate::{StaticVMapLineOfSightProvider, VMapLineOfSightQuery};

#[derive(Debug)]
struct RecordingLos {
    response: bool,
    calls: Mutex<Vec<VMapLineOfSightQuery>>,
}

impl StaticVMapLineOfSightProvider for RecordingLos {
    fn is_in_line_of_sight(&self, query: VMapLineOfSightQuery) -> bool {
        self.calls.lock().unwrap().push(query);
        self.response
    }
}

#[test]
fn owned_resolver_preserves_original_object_endpoints_and_blocking_response() {
    for response in [false, true] {
        let (mut manager, tick, mut token, caller, assistant, _, request) = pending_fixture(815_001);
        let provider = Arc::new(RecordingLos { response, calls: Mutex::new(Vec::new()) });
        let terrain = LiveTerrainHeights::new_with_static_vmap_line_of_sight(
            std::env::temp_dir(), provider.clone());
        let original = terrain.is_within_los_like_cpp(1,
            actor(&manager, caller).creature.unit().world(),
            actor(&manager, assistant).creature.unit().world());
        let elapsed = actor(&manager, caller).runtime_elapsed_ms_like_cpp();
        let (continuation, actual) = request.resolve(&terrain);
        assert_eq!(actual, original);
        assert_eq!(actual, response);
        let calls = provider.calls.lock().unwrap();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0], calls[1]);
        drop(calls);
        let outcome = complete(policies(|policy| manager.resume_aggro_assistance_los(&tick, &mut token,
            continuation, actual, policy)).unwrap());
        assert_eq!(outcome.assistance_scheduled, usize::from(response));
        assert_eq!(actor(&manager, caller).runtime_elapsed_ms_like_cpp(), elapsed);
        assert!(token.actor_operation.is_none());
    }
}
