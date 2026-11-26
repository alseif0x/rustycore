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

On a fresh 54261 client, choose the language and region, then quit from the login
screen before entering the QA account. The first region selection can overwrite
the configured portal. With the game closed, set `SET portal "<QA-hostname>:1119"`
in `_classic_/WTF/Config.wtf`, using the DNS name covered by the Battle.net
certificate, then relaunch. The [direct-launch QA run](../../architecture/captures/46-qa-54261-2026-11-22.md#direct-launch-with-a-fresh-profile)
records how far that configuration reached on the shared realm.

The repository's integrated smoke client is documented under
[`tools/wow-test-bot`](https://github.com/alseif0x/rustycore/tree/3.4.3/tools/wow-test-bot).
It verifies a narrow login/world-entry scenario; it does not establish full gameplay parity.
