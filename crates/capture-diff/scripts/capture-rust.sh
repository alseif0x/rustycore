#!/usr/bin/env bash
# capture-rust.sh — record a RustyCore packet dump for one flow.
#
# Restarts the RustyCore world server with RUSTYCORE_PACKET_DUMP_DIR pointed at a
# fresh directory, pauses for you to perform the flow with a client, then leaves
# the dump in place and restarts the server cleanly.
#
# Usage:   crates/capture-diff/scripts/capture-rust.sh <flow> [--yes]
# Output:  target/captures/<flow>/rust/   (gitignored; .bin/.meta per packet)
#
# Honored env vars:
#   PM2_RUST_WORLD  pm2 name of the Rust world (default: rustycore-world)
#   PM2_CPP_WORLD   pm2 name of the C++ world  (default: cpp-world) — stopped first
#
# This restarts the live world server (disconnecting players). Pass --yes to skip
# the confirmation prompt.
set -euo pipefail

FLOW="${1:-}"
[ -n "$FLOW" ] || { echo "usage: $0 <flow> [--yes]" >&2; exit 2; }
CONFIRM="${2:-}"

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
PM2_RUST_WORLD="${PM2_RUST_WORLD:-rustycore-world}"
PM2_CPP_WORLD="${PM2_CPP_WORLD:-cpp-world}"

DUMP_DIR="${REPO_ROOT}/target/captures/${FLOW}/rust"

echo "flow      : ${FLOW}"
echo "dump dir  : ${DUMP_DIR}"
echo "pm2 world : ${PM2_RUST_WORLD}"
echo
echo "This will restart ${PM2_RUST_WORLD} with RUSTYCORE_PACKET_DUMP_DIR set."

if [ "$CONFIRM" != "--yes" ]; then
  read -r -p "Proceed? [y/N] " ans
  [ "$ans" = "y" ] || [ "$ans" = "Y" ] || { echo "aborted"; exit 1; }
fi

# Make sure the C++ swap server is not holding the ports.
pm2 stop "$PM2_CPP_WORLD" >/dev/null 2>&1 || true

# Fresh dump directory so the capture only contains this flow.
rm -rf "$DUMP_DIR"
mkdir -p "$DUMP_DIR"

cleanup() {
  echo "clearing RUSTYCORE_PACKET_DUMP_DIR and restarting ${PM2_RUST_WORLD}..."
  unset RUSTYCORE_PACKET_DUMP_DIR
  pm2 restart "$PM2_RUST_WORLD" --update-env >/dev/null 2>&1 || true
}
trap cleanup EXIT

export RUSTYCORE_PACKET_DUMP_DIR="$DUMP_DIR"
echo "restarting ${PM2_RUST_WORLD} with dump enabled..."
pm2 restart "$PM2_RUST_WORLD" --update-env

echo
echo ">>> Perform the '${FLOW}' flow with the client now."
read -r -p ">>> Press ENTER when the flow is complete to finish the capture... " _

COUNT=$(find "$DUMP_DIR" -name '*.meta' | wc -l)
echo "collected ${COUNT} packets -> ${DUMP_DIR}"
echo
echo "diff against the C++ golden:"
echo "  cargo run -p capture-diff -- diff ${FLOW} --rust ${DUMP_DIR}"
