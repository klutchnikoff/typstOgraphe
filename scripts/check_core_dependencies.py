"""Keep UI, localization and MCP transport/runtime dependencies outside the core."""

from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
FORBIDDEN = {"egui", "eframe", "winit", "rfd", "muda", "tokio", "rmcp", "interprocess"}


def forbidden_dependencies(tree):
    names = {line.split()[0] for line in tree.splitlines() if line.strip()}
    # fluent-uri parses URIs for jsonschema; it is unrelated to Fluent localization.
    return sorted(name for name in names if name in FORBIDDEN
                  or (name != "fluent-uri"
                      and name.startswith(("fluent", "i18n-embed", "tokio-", "rmcp-"))))


def main():
    result = subprocess.run(
        ["cargo", "tree", "--locked", "--color", "never", "-p", "typstographe-core",
         "-e", "normal", "--prefix", "none", "--format", "{p}"],
        cwd=ROOT, capture_output=True, text=True, check=False,
    )
    if result.returncode:
        sys.stderr.write(result.stderr)
        return result.returncode
    if not result.stdout.startswith("typstographe-core "):
        print("Cannot identify the core dependency tree", file=sys.stderr)
        return 1
    blocked = forbidden_dependencies(result.stdout)
    if blocked:
        print("Forbidden normal core dependencies: " + ", ".join(blocked), file=sys.stderr)
        return 1
    print("Core normal dependencies contain no GUI, Fluent, Tokio, MCP or local IPC libraries.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
