#!/usr/bin/env bash
set -euo pipefail
capture_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$capture_root"
exec "${TYPSTOGRAPHE_PYTHON:-python3}" scripts/capture_baseline.py "$@"
