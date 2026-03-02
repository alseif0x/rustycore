//! Build script: compile .proto files via prost-build.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let proto_dir = "proto";

    let proto_files = [
        "bgs/low/pb/client/rpc_types.proto",
        "bgs/low/pb/client/entity_types.proto",
        "bgs/low/pb/client/attribute_types.proto",
        "bgs/low/pb/client/content_handle_types.proto",
        "bgs/low/pb/client/semantic_version.proto",
        "bgs/low/pb/client/authentication_service.proto",
        "bgs/low/pb/client/connection_service.proto",
        "bgs/low/pb/client/challenge_service.proto",
        "bgs/low/pb/client/game_utilities_service.proto",
        "bgs/low/pb/client/account_types.proto",
        "bgs/low/pb/client/account_service.proto",
    ];

    // Prefix all paths with the proto directory.
    let proto_paths: Vec<String> = proto_files
        .iter()
        .map(|f| format!("{proto_dir}/{f}"))
        .collect();

    prost_build::Config::new()
        .compile_protos(&proto_paths, &[proto_dir])?;

    Ok(())
}
