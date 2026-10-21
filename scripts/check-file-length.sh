#!/usr/bin/env bash
set -euo pipefail
check_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$check_root"
exec "${TYPSTOGRAPHE_PYTHON:-python3}" scripts/check_file_length.py "$@"
