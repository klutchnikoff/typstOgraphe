"""Record and compare deterministic native captures. Python standard library only."""

import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import platform
import struct
import subprocess
import sys
import tempfile
import zipfile

ROOT = Path(__file__).resolve().parents[1]
PROFILE = "native-light-fr-v1"
SCENARIOS = ("welcome", "editor", "creation", "search", "spelling", "profile", "profile-letter", "recovery")


def environment():
    return {
        "system": platform.system(),
        "release": platform.release(),
        "machine": platform.machine(),
        "capture_profile": PROFILE,
    }


def image_record(path):
    data = path.read_bytes()
    if len(data) < 24 or data[:8] != b"\x89PNG\r\n\x1a\n" or data[12:16] != b"IHDR":
        raise ValueError(f"Not a captured PNG: {path}")
    return {"size": list(struct.unpack(">II", data[16:24])), "sha256": hashlib.sha256(data).hexdigest()}


def compare(reference, current, current_environment):
    """Return all failures without modifying either capture directory."""
    baseline = json.loads((reference / "manifest.json").read_text(encoding="utf-8"))
    failures = []
    if baseline.get("version") != 1:
        failures.append("Unsupported capture manifest version")
    if baseline.get("environment") != current_environment:
        failures.append("Capture environment differs; record references on this machine")
    if set(baseline.get("images", {})) != set(SCENARIOS):
        failures.append("Reference scenario list differs")
    for name in SCENARIOS:
        expected = baseline.get("images", {}).get(name)
        for directory in [reference, current]:
            path = directory / f"{name}.png"
            try:
                actual = image_record(path)
            except (OSError, ValueError) as error:
                failures.append(f"{name}: {error}")
                continue
            if actual != expected:
                failures.append(f"{name}: capture differs in {directory}")
    return failures


def run(binary, arguments, log):
    command = [str(binary), *map(str, arguments)]
    try:
        result = subprocess.run(command, cwd=ROOT, capture_output=True, timeout=90)
    except subprocess.TimeoutExpired as error:
        log.write_bytes((error.stdout or b"") + (error.stderr or b""))
        raise RuntimeError(f"Capture timed out; see {log}") from error
    log.write_bytes(result.stdout + result.stderr)
    if result.returncode:
        raise RuntimeError(f"Command failed ({result.returncode}); see {log}")


def fixtures(binary, work):
    exported = work / "export"
    run(binary, ["--render-demo", exported], work / "render-demo.log")
    files = {name: (exported / name).read_bytes() for name in
             ["main.typ", "modele.typ", "metadonnees.json", "dependencies.json"]}
    files["manifest.json"] = (ROOT / "tests/fixtures/capture-manifest.json").read_bytes()
    files["contenu.typ"] = (ROOT / "tests/fixtures/capture-letter.typ").read_bytes()
    files["images/parc.svg"] = (ROOT / "tests/fixtures/parc.svg").read_bytes()
    for folder in ["capabilities", "packages"]:
        for path in (exported / folder).rglob("*"):
            if path.is_file():
                files[path.relative_to(exported).as_posix()] = path.read_bytes()
    archive = work / "editor.typsto"
    with zipfile.ZipFile(archive, "w", compression=zipfile.ZIP_DEFLATED) as writer:
        for name, data in files.items():
            writer.writestr(name, data)
    recovery = work / "recovery"
    slot = recovery / "session-fixture"
    slot.mkdir(parents=True)
    (slot / "lock").touch()
    with zipfile.ZipFile(slot / "document.recovery", "w", compression=zipfile.ZIP_DEFLATED) as writer:
        for name, data in files.items():
            writer.writestr(name, data)
        writer.writestr("recovery.json", json.dumps({
            "version": 1, "original_name": "Courrier récupéré.typsto",
            "saved_at": int(datetime(2026, 10, 21, 12, tzinfo=timezone.utc).timestamp()),
        }, ensure_ascii=False))
    return archive, recovery


def capture(binary, destination):
    destination.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="fixtures-", dir=destination) as temporary:
        archive, recovery = fixtures(binary, Path(temporary))
        for name in SCENARIOS:
            image = destination / f"{name}.png"
            arguments = {
                "welcome": ["--capture", image],
                "editor": ["--capture", image, archive],
                "creation": ["--capture-new-document", image],
                "search": ["--capture-search", image, "salle"],
                "spelling": ["--capture-spelling", image],
                "profile": ["--capture-profile", image],
                "profile-letter": ["--capture-profile-letter", image],
                "recovery": ["--capture-recovery", image, recovery],
            }[name]
            print(f"Capture: {name}", flush=True)
            run(binary, arguments, destination / f"{name}.log")
            image_record(image)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--record", action="store_true", help="Create references; refuse to overwrite existing references")
    mode.add_argument("--compare", action="store_true", help="Capture and compare, preserving references")
    parser.add_argument("--output", type=Path, default=ROOT / "output/baseline")
    parser.add_argument("--binary", type=Path, help="Use an already built application instead of cargo build")
    args = parser.parse_args()
    output = args.output.resolve()
    reference = output / "reference"
    if args.record and reference.exists():
        parser.error(f"References already exist: {reference}; use a new --output to create another baseline")
    if args.compare and not (reference / "manifest.json").is_file():
        parser.error(f"Missing references: {reference}; run --record first")
    if args.binary:
        binary = args.binary.resolve()
    else:
        subprocess.run(["cargo", "build", "--locked", "--bin", "typstographe"], cwd=ROOT, check=True)
        binary = ROOT / "target/debug" / ("typstographe.exe" if os.name == "nt" else "typstographe")
    output.mkdir(parents=True, exist_ok=True)
    # A fresh directory avoids stale PNGs disguising a failed capture. A failure
    # keeps its logs and images, without ever replacing reference/manifest.json.
    current = Path(tempfile.mkdtemp(prefix="run-", dir=output))
    capture(binary, current)
    manifest = {
        "version": 1, "environment": environment(),
        "commit": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
        "working_tree_dirty": bool(subprocess.check_output(["git", "status", "--porcelain"], cwd=ROOT)),
        "images": {name: image_record(current / f"{name}.png") for name in SCENARIOS},
    }
    (current / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    if args.record:
        current.rename(reference)
        print(f"Recorded {len(SCENARIOS)} references in {reference}")
        return 0
    failures = compare(reference, current, environment())
    if failures:
        for failure in failures:
            print(failure, file=sys.stderr)
        print(f"Inspect current captures in {current}", file=sys.stderr)
        return 1
    print(f"All {len(SCENARIOS)} captures match exactly. Current captures: {current}")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError, RuntimeError, subprocess.CalledProcessError) as error:
        print(error, file=sys.stderr)
        raise SystemExit(1) from error
