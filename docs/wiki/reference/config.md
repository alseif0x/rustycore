# Configuration

RustyCore prefers Trinity-style lowercase configuration names:

| Service | Preferred file | Optional override directory |
|---|---|---|
| Battle.net | `bnetserver.conf` | `bnetserver.conf.d/` |
| World | `worldserver.conf` | `worldserver.conf.d/` |

Use these lowercase names for normal startup. The world service retains a legacy
`WorldServer.conf` fallback, but the Battle.net service does not automatically fall back to
`BNetServer.conf`; a non-default filename must be selected explicitly with `--config`.

Configuration commonly includes database endpoints, listener addresses, data paths, logging,
and runtime feature switches. Keep real credentials, PEM files, database URLs, and local
configuration out of Git; only sanitized examples belong in the repository.
