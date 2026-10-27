"""Peak process RSS and fresh-process latency, without the frontend or Typst."""
from pathlib import Path
import json
import platform
import re
import statistics
import subprocess
import time

root = Path(__file__).resolve().parent
binary = root / "target/release/typstographe-yrs-evaluation"
results = json.loads((root / "results.json").read_text())
measurements = []
for name in ("article", "long", "large"):
    payload = (root / f".generated/bench-{name}.json").read_bytes()
    runs = []
    for _ in range(7):
        command = ["/usr/bin/time", "-l", str(binary)] if platform.system() == "Darwin" else ["/usr/bin/time", "-v", str(binary)]
        started = time.perf_counter()
        completed = subprocess.run(command, input=payload, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, check=True)
        elapsed = (time.perf_counter() - started) * 1000
        stderr = completed.stderr.decode()
        if platform.system() == "Darwin":
            rss = int(re.search(r"(\d+)\s+maximum resident set size", stderr)[1])
        else:
            rss = int(re.search(r"Maximum resident set size \(kbytes\):\s*(\d+)", stderr)[1]) * 1024
        runs.append({"wall_ms": elapsed, "peak_rss_bytes": rss})
    measurements.append({"name": name, "runs": len(runs), "fresh_process_wall_ms_median": statistics.median(r["wall_ms"] for r in runs), "peak_rss_bytes_median": statistics.median(r["peak_rss_bytes"] for r in runs), "peak_rss_bytes_max": max(r["peak_rss_bytes"] for r in runs)})
results["process_measurements"] = {"note": "New Rust process per sample, OS caches warm; includes stdin JSON and base64, full update output discarded; excludes JS, Tauri, Typst and archive IO.", "release_binary_bytes": binary.stat().st_size, "samples": measurements}
(root / "results.json").write_text(json.dumps(results, indent=2, ensure_ascii=False) + "\n")
print(json.dumps(results["process_measurements"], indent=2))
