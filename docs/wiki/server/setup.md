# Server setup

## Requirements

- Rust pinned by [`rust-toolchain.toml`](https://github.com/alseif0x/rustycore/blob/3.4.3/rust-toolchain.toml)
- MariaDB `10.6` or newer
- `protoc` pinned by [`.protoc-version`](https://github.com/alseif0x/rustycore/blob/3.4.3/.protoc-version) for protobuf-dependent crates
- Trinity/TDB-style `auth`, `characters`, `world`, and `hotfixes` databases
- Extracted client data required by the runtime configuration

Detailed database preparation is maintained in the repository's
[DB bootstrap guide](https://github.com/alseif0x/rustycore/blob/3.4.3/docs/operations/db-bootstrap.md).
Use it only for the intended, authorized environment and data. Building this project does
not authorize service interruptions or database mutations. Normal server startup validates
schemas; `rustycore-db` is the separate migration authority.

## Build

```bash
git clone https://github.com/alseif0x/rustycore.git
cd rustycore
git checkout 3.4.3
PROTOC=/path/to/protoc cargo build --locked -p bnet-server -p world-server --release
```

## Configure and run

Create local `bnetserver.conf` and `worldserver.conf` files for your environment. These
files and their `.conf.d` override directories are ignored at the repository root. Never
commit passwords, database URLs, private keys, certificates, or runtime configuration.

Start the services in separate terminals, in this order. Wait for Battle.net to finish its
startup before launching the world service.

Terminal 1:

```bash
./target/release/bnet-server --config /absolute/path/to/bnetserver.conf
```

Terminal 2:

```bash
./target/release/world-server --config /absolute/path/to/worldserver.conf
```

Check the logs for successful database validation, realm activation, and listeners on the
configured ports before connecting a client.
