# Server setup

## Requirements

- Rust `1.98` or newer
- MariaDB `10.6` or newer
- `protoc` for protobuf-dependent crates
- Trinity/TDB-style `auth`, `characters`, `world`, and `hotfixes` databases
- Extracted client data required by the runtime configuration

Detailed database preparation is maintained in the repository's
[DB bootstrap guide](https://github.com/alseif0x/rustycore/blob/3.4.3/docs/operations/db-bootstrap.md).

## Build

```bash
git clone https://github.com/alseif0x/rustycore.git
cd rustycore
git checkout 3.4.3
PROTOC=/path/to/protoc cargo build -p bnet-server -p world-server --release
```

## Configure and run

Create local `bnetserver.conf` and `worldserver.conf` files for your environment. The
capitalized legacy names remain accepted for compatibility, but the lowercase names are
preferred. Never commit passwords, database URLs, private keys, certificates, or runtime
configuration.

Start the services in this order:

```bash
./target/release/bnet-server
./target/release/world-server
```

Check the logs for successful database validation, realm activation, and listeners on the
configured ports before connecting a client.
