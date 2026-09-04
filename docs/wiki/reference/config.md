# Configuration

RustyCore prefers Trinity-style lowercase configuration names:

| Service | Preferred file | Optional override directory |
|---|---|---|
| Battle.net | `bnetserver.conf` | `bnetserver.conf.d/` |
| World | `worldserver.conf` | `worldserver.conf.d/` |

The legacy Rust names `BNetServer.conf` and `WorldServer.conf` are still accepted as fallback
files. When both variants exist, use the lowercase file as the authoritative configuration.

Configuration commonly includes database endpoints, listener addresses, data paths, logging,
and runtime feature switches. Keep real credentials, PEM files, database URLs, and local
configuration out of Git; only sanitized examples belong in the repository.
