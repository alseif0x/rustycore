# Client setup

Before connecting a client, build and configure both RustyCore services and ensure the
`auth`, `characters`, `world`, and `hotfixes` MariaDB databases are available.

The default local endpoints are:

| Service | Port |
|---|---:|
| Battle.net RPC over TLS | `1119` |
| Battle.net REST | `8081` |
| World socket | `8085` |
| Instance socket | `8086` |

The selected `auth.realmlist` entry must match the world listener and client build. Runtime
certificates, credentials, database URLs, and local configuration files must remain outside
version control.

The repository's integrated smoke client is documented under
[`tools/wow-test-bot`](https://github.com/alseif0x/rustycore/tree/3.4.3/tools/wow-test-bot).
It verifies a narrow login/world-entry scenario; it does not establish full gameplay parity.
