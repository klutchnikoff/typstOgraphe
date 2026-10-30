"""Conservative CI scope: unknown history runs checks; documentation alone does not."""
import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import sys


def plan(paths):
    if paths is None:
        return {"code": True, "dependencies": True}
    code = dependencies = False
    for name in paths:
        path = Path(name)
        dependency = (path.name in {"Cargo.toml", "Cargo.lock", "package.json", "package-lock.json", "deny.toml"}
                      or name.startswith((".github/", ".cargo/")))
        # Markdown embedded in a source/resource directory may be executable
        # input. Only the known documentation locations are skipped.
        documentation = (name.startswith("docs/") or name == "app/README.md"
                         or (path.parent == Path(".") and path.suffix == ".md"))
        # This generated catalogue is compared with the engine by the tests.
        code |= dependency or not documentation or name == "docs/commands.json"
        dependencies |= dependency
    return {"code": code, "dependencies": dependencies}


def changed_paths(base, head="HEAD", root=None):
    if not re.fullmatch(r"[0-9a-fA-F]{40}|[0-9a-fA-F]{64}", base or "") or set(base) == {"0"}:
        return None

    def git(*args):
        return subprocess.run(["git", *args], cwd=root, check=True,
                              stdout=subprocess.PIPE, stderr=subprocess.PIPE)

    try:
        try:
            git("cat-file", "-e", f"{base}^{{commit}}")
        except subprocess.CalledProcessError:
            git("fetch", "--no-tags", "--depth=1", "origin", base)
        # Renaming a source into docs must still report its source-side deletion.
        data = git("diff", "--name-only", "--no-renames", "-z", base, head).stdout
        return [os.fsdecode(name) for name in data.split(b"\0") if name]
    except subprocess.CalledProcessError:
        print("Historique incomplet : toutes les vérifications seront exécutées.", file=sys.stderr)
        return None


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base", default="")
    parser.add_argument("--force", action="store_true")
    args = parser.parse_args()
    result = plan(None if args.force else changed_paths(args.base))
    if output := os.environ.get("GITHUB_OUTPUT"):
        with open(output, "a", encoding="utf-8") as stream:
            for key, value in result.items():
                stream.write(f"{key}={str(value).lower()}\n")
    print(json.dumps(result))


if __name__ == "__main__":
    main()
