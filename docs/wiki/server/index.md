# Server

RustyCore is split into two primary services:

- `bnet-server` handles Battle.net authentication, TLS RPC, REST, realm discovery, and
  login-related database work.
- `world-server` loads world data, accepts realm and instance connections, creates sessions,
  and orchestrates the current map runtimes.

The port is intentionally transitional. Legacy and canonical map models still coexist while
state and update ownership move toward the canonical map runtime. Consult the
[current migration state](https://github.com/alseif0x/rustycore/blob/3.4.3/docs/migration/STATE.md)
before relying on a subsystem in a live environment.

Continue with [Server setup](./setup) for build requirements and startup order.
