#!/usr/bin/env bash
# roaring-crab Unix launcher. Picks the right prebuilt binary based on OS+arch.
set -e

EVENT="$1"
if [ -z "$EVENT" ]; then
  echo "usage: launch.sh <HookEvent>" >&2
  exit 2
fi

case "$(uname -s)" in
  Linux*)  OS=linux ;;
  Darwin*) OS=macos ;;
  *) exit 0 ;;
esac

case "$(uname -m)" in
  x86_64|amd64) ARCH=x86_64 ;;
  arm64|aarch64) ARCH=aarch64 ;;
  *) exit 0 ;;
esac

BIN="${CLAUDE_PLUGIN_ROOT:-$(dirname "$0")/..}/bin/${OS}-${ARCH}/roaring-crab"
if [ ! -x "$BIN" ]; then
  exit 0  # binary missing for platform → silent skip
fi
exec "$BIN" --event "$EVENT"
