use map_runtime_spike::{benchmark_hash_map, benchmark_hecs, compare_backends};

fn main() {
    let objects = std::env::var("MAP_RUNTIME_SPIKE_OBJECTS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(20_000);
    let frames = std::env::var("MAP_RUNTIME_SPIKE_FRAMES")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(200);
    let backend = std::env::var("MAP_RUNTIME_SPIKE_BACKEND").unwrap_or_else(|_| "both".to_owned());
    let results = match backend.as_str() {
        "hash-map" => vec![benchmark_hash_map(objects, frames)],
        "hecs" => vec![benchmark_hecs(objects, frames)],
        "both" => compare_backends(objects, frames).into(),
        other => panic!("unsupported MAP_RUNTIME_SPIKE_BACKEND={other:?}"),
    };

    println!(
        "architecture={} os={} objects={} frames={}",
        std::env::consts::ARCH,
        std::env::consts::OS,
        objects,
        frames
    );
    for result in results {
        let entity_ticks = (result.objects * result.frames) as f64;
        let ns_per_entity_tick = result.elapsed.as_nanos() as f64 / entity_ticks;
        println!(
            "backend={:?} elapsed_ms={} ns_per_entity_tick={:.3} resident_kib={:?} peak_resident_kib={:?} checksum={:032x}",
            result.architecture,
            result.elapsed.as_millis(),
            ns_per_entity_tick,
            result.resident_kib,
            result.peak_resident_kib,
            result.checksum
        );
    }
}
