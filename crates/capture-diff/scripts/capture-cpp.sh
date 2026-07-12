#!/usr/bin/env bash
# capture-cpp.sh — record a C++ TrinityCore golden capture for one flow.
#
# Swaps the running RustyCore world server for the legacy C++ world server (they
# share DBs and ports), enables the C++ PKT packet log, pauses for you to perform
# the flow with a client, then collects the .pkt and restores the Rust server.
#
# Usage:   crates/capture-diff/scripts/capture-cpp.sh <flow> [--yes]
# Output:  target/captures/<flow>/cpp.pkt   (gitignored)
#
# Honored env vars (defaults target this machine's layout):
#   CPP_RUNTIME_DIR directory the PM2 wrapper enters before worldserver starts
#                   (default: trinity-legacy-install/bin)
#   CPP_CONF        active legacy worldserver.conf
#                   (default: $CPP_RUNTIME_DIR/worldserver.conf)
#   CPP_LOGS_DIR    PacketLogFile output directory. Defaults to LogsDir from
#                   CPP_CONF, resolved under CPP_RUNTIME_DIR when relative;
#                   an empty LogsDir means CPP_RUNTIME_DIR, matching C++.
#   PM2_CPP_WORLD   pm2 name of the C++ world  (default: cpp-world)
#   PM2_RUST_WORLD  pm2 name of the Rust world (default: rustycore-world)
#
# This stops the live RustyCore world server (disconnecting players). It refuses
# to run without confirmation; pass --yes to skip the prompt.
set -euo pipefail

FLOW="${1:-}"
[ -n "$FLOW" ] || { echo "usage: $0 <flow> [--yes]" >&2; exit 2; }
[[ "$FLOW" =~ ^[A-Za-z0-9][A-Za-z0-9._-]*$ ]] || {
  echo "error: invalid flow name '${FLOW}' (use one ASCII path component: letters, digits, '.', '_', '-')" >&2
  exit 2
}
CONFIRM="${2:-}"

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
CPP_RUNTIME_DIR="${CPP_RUNTIME_DIR:-/home/server/trinity-legacy-install/bin}"
CPP_CONF="${CPP_CONF:-${CPP_RUNTIME_DIR}/worldserver.conf}"
PM2_CPP_WORLD="${PM2_CPP_WORLD:-cpp-world}"
PM2_RUST_WORLD="${PM2_RUST_WORLD:-rustycore-world}"

if [ -z "${CPP_LOGS_DIR+x}" ]; then
  CONFIGURED_LOGS_DIR="$({
    sed -n -E 's/^[[:space:]]*LogsDir[[:space:]]*=[[:space:]]*"([^"]*)".*/\1/p' "$CPP_CONF" 2>/dev/null || true
  } | tail -n 1)"
  if [ -z "$CONFIGURED_LOGS_DIR" ]; then
    CPP_LOGS_DIR="$CPP_RUNTIME_DIR"
  elif [[ "$CONFIGURED_LOGS_DIR" = /* ]]; then
    CPP_LOGS_DIR="$CONFIGURED_LOGS_DIR"
  else
    CPP_LOGS_DIR="${CPP_RUNTIME_DIR}/${CONFIGURED_LOGS_DIR}"
  fi
fi

PKT_NAME="rustycore-capture-${FLOW}.pkt"
OUT_DIR="${REPO_ROOT}/target/captures/${FLOW}"
OUT_PKT="${OUT_DIR}/cpp.pkt"

echo "flow         : ${FLOW}"
echo "C++ conf     : ${CPP_CONF}"
echo "C++ logs dir : ${CPP_LOGS_DIR}"
echo "pkt file     : ${CPP_LOGS_DIR}/${PKT_NAME}"
echo "output       : ${OUT_PKT}"
echo
echo "This will STOP ${PM2_RUST_WORLD} and START ${PM2_CPP_WORLD} (shared DBs/ports)."

if [ "$CONFIRM" != "--yes" ]; then
  read -r -p "Proceed? [y/N] " ans
  [ "$ans" = "y" ] || [ "$ans" = "Y" ] || { echo "aborted"; exit 1; }
fi

[ -f "$CPP_CONF" ] || { echo "error: conf not found: $CPP_CONF" >&2; exit 1; }
[ -d "$CPP_LOGS_DIR" ] || { echo "error: packet log directory not found: $CPP_LOGS_DIR" >&2; exit 1; }

CONF_BAK="${CPP_CONF}.capture-diff.bak"
# A leftover backup means a prior run was killed before restoring. Refuse to
# overwrite it with the (possibly already-edited) conf — that would lose the
# pristine original. The operator must inspect/restore it manually first.
if [ -e "$CONF_BAK" ]; then
  echo "error: stale backup ${CONF_BAK} exists (a prior run did not restore)." >&2
  echo "       restore it over ${CPP_CONF} and delete it before re-running." >&2
  exit 1
fi
# Preserve the active config's ownership/mode/timestamps exactly. A restrictive
# caller umask must not turn the restored worldserver.conf into a different
# runtime file after the capture.
cp -a "$CPP_CONF" "$CONF_BAK"

restore() {
  echo "restoring ${PM2_CPP_WORLD} -> ${PM2_RUST_WORLD} and conf..."
  pm2 stop "$PM2_CPP_WORLD" >/dev/null 2>&1 || true
  if ! mv -f "$CONF_BAK" "$CPP_CONF"; then
    echo "WARNING: failed to restore ${CPP_CONF} from ${CONF_BAK} — packet logging may still be ON; restore it manually." >&2
  fi
  pm2 start "$PM2_RUST_WORLD" >/dev/null 2>&1 || true
}
trap restore EXIT

# Enable PacketLogFile in the conf (replace existing line or append).
if grep -qE '^[[:space:]]*PacketLogFile' "$CPP_CONF"; then
  sed -i -E "s|^[[:space:]]*PacketLogFile.*|PacketLogFile = \"${PKT_NAME}\"|" "$CPP_CONF"
else
  printf '\nPacketLogFile = "%s"\n' "$PKT_NAME" >>"$CPP_CONF"
fi

# A local C++ test-harness extension can bypass SMSG_CONNECT_TO for accounts
# matching Bot.AccountPrefix. Golden captures must exercise stock realm→instance
# routing, so disable that shortcut inside the already-backed-up config. The
# EXIT trap restores the original value and file metadata exactly.
if grep -qE '^[[:space:]]*Bot\.AccountPrefix[[:space:]]*=' "$CPP_CONF"; then
  sed -i -E 's|^[[:space:]]*Bot\.AccountPrefix[[:space:]]*=.*|Bot.AccountPrefix = ""|' "$CPP_CONF"
else
  printf '\nBot.AccountPrefix = ""\n' >>"$CPP_CONF"
fi

rm -f "${CPP_LOGS_DIR}/${PKT_NAME}"

echo "swapping to C++ world server..."
pm2 stop "$PM2_RUST_WORLD" >/dev/null 2>&1 || true
pm2 start "$PM2_CPP_WORLD"

echo
echo ">>> Perform the '${FLOW}' flow with the client now."
read -r -p ">>> Press ENTER when the flow is complete to collect the capture... " _

mkdir -p "$OUT_DIR"
cp -f "${CPP_LOGS_DIR}/${PKT_NAME}" "$OUT_PKT"
echo "collected $(stat -c%s "$OUT_PKT" 2>/dev/null || echo '?') bytes -> ${OUT_PKT}"
echo "next: crates/capture-diff/scripts/capture-rust.sh ${FLOW}"
