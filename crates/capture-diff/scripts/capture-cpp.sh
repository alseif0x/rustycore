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
#   CPP_CONF        legacy worldserver.conf  (default: trinity-legacy-install/etc/worldserver.conf)
#   CPP_LOGS_DIR    LogsDir from that conf   (default: trinity-legacy-install/logs)
#   PM2_CPP_WORLD   pm2 name of the C++ world  (default: cpp-world)
#   PM2_RUST_WORLD  pm2 name of the Rust world (default: rustycore-world)
#
# This stops the live RustyCore world server (disconnecting players). It refuses
# to run without confirmation; pass --yes to skip the prompt.
set -euo pipefail

FLOW="${1:-}"
[ -n "$FLOW" ] || { echo "usage: $0 <flow> [--yes]" >&2; exit 2; }
CONFIRM="${2:-}"

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
CPP_CONF="${CPP_CONF:-/home/server/trinity-legacy-install/etc/worldserver.conf}"
CPP_LOGS_DIR="${CPP_LOGS_DIR:-/home/server/trinity-legacy-install/logs}"
PM2_CPP_WORLD="${PM2_CPP_WORLD:-cpp-world}"
PM2_RUST_WORLD="${PM2_RUST_WORLD:-rustycore-world}"

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

CONF_BAK="${CPP_CONF}.capture-diff.bak"
# A leftover backup means a prior run was killed before restoring. Refuse to
# overwrite it with the (possibly already-edited) conf — that would lose the
# pristine original. The operator must inspect/restore it manually first.
if [ -e "$CONF_BAK" ]; then
  echo "error: stale backup ${CONF_BAK} exists (a prior run did not restore)." >&2
  echo "       restore it over ${CPP_CONF} and delete it before re-running." >&2
  exit 1
fi
cp -f "$CPP_CONF" "$CONF_BAK"

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
