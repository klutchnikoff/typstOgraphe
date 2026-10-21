"""Present cargo-llvm-cov results; LLVM remains the source of all measurements."""

import json
import os
from pathlib import Path
import sys

report = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
totals = report["data"][0]["totals"]
lines = totals["lines"]
if lines["count"] == 0:
    raise SystemExit("No application lines found in the coverage report")

percent = lines["percent"]
badge = {
    "schemaVersion": 1,
    "label": "couverture",
    "message": f"{percent:.1f} %",
    "color": "brightgreen" if percent >= 80 else "yellow" if percent >= 60 else "orange",
}
badge_dir = Path(sys.argv[2])
badge_dir.mkdir(parents=True, exist_ok=True)
(badge_dir / "coverage.json").write_text(json.dumps(badge) + "\n", encoding="utf-8")

summary = [
    "## Couverture des tests — Linux",
    "",
    "Code Rust de l’application, hors dépendances et implémentation des tests.",
    "Les chemins propres à macOS et Windows ne sont pas mesurés ici.",
    "",
    "| Mesure | Exécutés / total | Couverture |",
    "| --- | ---: | ---: |",
]
for key, label in [("lines", "Lignes"), ("functions", "Fonctions"), ("regions", "Régions")]:
    metric = totals[key]
    summary.append(
        f"| {label} | {metric['covered']} / {metric['count']} | {metric['percent']:.2f} % |"
    )
summary += ["", "Rapport HTML (ouvrir `html/index.html`), LCOV et JSON dans l’artefact **coverage-report**.", ""]
text = "\n".join(summary)
print(text)
if path := os.environ.get("GITHUB_STEP_SUMMARY"):
    with open(path, "a", encoding="utf-8") as output:
        output.write(text)
