#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

if [[ ! -x target/release/store-app ]]; then
  echo "target/release/store-app is missing; run the build task first" >&2
  exit 1
fi

exec target/release/store-app
