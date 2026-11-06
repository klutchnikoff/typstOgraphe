"""Limit production Rust and frontend TypeScript modules to 800 lines by default."""
import argparse
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[1]


def production_lines(source):
    lines = source.splitlines()
    kept = []
    index = 0
    while index < len(lines):
        line = lines[index]
        if line.strip() != "#[cfg(test)]":
            kept.append(line)
            index += 1
            continue
        indent = line[:len(line) - len(line.lstrip())]
        index += 1
        while index < len(lines) and lines[index].lstrip().startswith("#["):
            index += 1
        # Repository items are formatted by rustfmt. Handle enum variants and
        # external modules separately so they cannot hide later production code.
        first = lines[index].strip() if index < len(lines) else ""
        block_item = re.match(r"(?:pub(?:\([^)]*\))? )?(?:fn|mod)\b", first)
        if not block_item:
            while index < len(lines):
                ending = lines[index].rstrip().endswith(",")
                index += 1
                if ending:
                    break
            continue
        while index < len(lines):
            item_line = lines[index]
            index += 1
            if item_line.rstrip().endswith(";"):
                break
            if "{" in item_line:
                # rustfmt keeps empty functions and modules on one line.
                if item_line.rstrip().endswith("}"):
                    break
                while index < len(lines):
                    closing = lines[index] == indent + "}"
                    index += 1
                    if closing:
                        break
                break
    return len(kept)


def violations(source_root, limit):
    result = []
    for path in sorted(source_root.rglob("*.rs")):
        relative = path.relative_to(source_root)
        if "tests" in relative.parts or re.match(r"(?:tests|.*_tests)\.rs$", path.name):
            continue
        count = production_lines(path.read_text(encoding="utf-8"))
        if count > limit:
            result.append((path, count))
    return result


def typescript_violations(source_root, limit):
    """Count complete TS/TSX modules; tests, fixtures and generated code are separate."""
    excluded = {"tests", "__tests__", "fixtures", "__fixtures__", "generated", "__generated__"}
    result = []
    for path in sorted(source_root.rglob("*")):
        if path.suffix not in {".ts", ".tsx"} or not path.is_file():
            continue
        if excluded.intersection(path.relative_to(source_root).parts[:-1]):
            continue
        if re.search(r"\.(?:test|spec)\.tsx?$", path.name):
            continue
        count = len(path.read_text(encoding="utf-8").splitlines())
        if count > limit:
            result.append((path, count))
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, help="Check only this Rust source tree (unless --typescript-root is supplied)")
    parser.add_argument("--typescript-root", type=Path, help="TypeScript source tree; defaults to app/src for repository checks")
    parser.add_argument("--limit", type=int, default=800)
    parser.add_argument("--warn", action="store_true", help="Report size violations without failing")
    args = parser.parse_args()
    if args.limit < 1:
        parser.error("--limit must be positive")
    found = violations(args.root or ROOT / "crates", args.limit)
    typescript_root = args.typescript_root or (ROOT / "app/src" if args.root is None else None)
    if typescript_root is not None:
        found.extend(typescript_violations(typescript_root, args.limit))
    for path, count in found:
        print(f"{'WARNING' if args.warn else 'ERROR'}: {path}: {count} production lines (limit {args.limit})")
    if not found:
        print(f"All checked production files are within {args.limit} lines.")
    return int(bool(found) and not args.warn)


if __name__ == "__main__":
    raise SystemExit(main())
