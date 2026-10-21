"""Download only model assets, never corpus text; pin revisions and SHA-256."""
import hashlib
import json
from pathlib import Path

from huggingface_hub import HfApi, hf_hub_download, snapshot_download

ROOT = Path(__file__).resolve().parents[2]
MODELS = ROOT / "output/reformulation/models"
RESULTS = Path(__file__).parent / "results"
MODELS.mkdir(parents=True, exist_ok=True)
RESULTS.mkdir(exist_ok=True)

api = HfApi()
REVISIONS = {
    "catie": "2280536c4dbc7afc1c412c58c3bfb9217c887358",
    "qwen": "91cad51170dc346986eccefdc2dd33a9da36ead9",
}
manifest = {}
for name, repo in [
    ("catie", "CATIE-AQ/french_paraphrase_flan-t5-base"),
    ("qwen", "Qwen/Qwen2.5-1.5B-Instruct-GGUF"),
]:
    info = api.model_info(repo, revision=REVISIONS[name])
    directory = MODELS / name
    if name == "catie":
        snapshot_download(repo, revision=info.sha, local_dir=directory,
            allow_patterns=["*.json", "*.model", "*.safetensors", "README.md"])
    else:
        for filename in ["qwen2.5-1.5b-instruct-q4_k_m.gguf", "README.md", "LICENSE"]:
            hf_hub_download(repo, filename, revision=info.sha, local_dir=directory)
    files = {}
    for path in sorted(directory.iterdir()):
        if not path.is_file():
            continue
        digest = hashlib.file_digest(path.open("rb"), "sha256").hexdigest()
        files[path.name] = {"bytes": path.stat().st_size, "sha256": digest}
    manifest[name] = {"repository": repo, "revision": info.sha, "files": files}
    print(name, info.sha, sum(file["bytes"] for file in files.values()), flush=True)
    (RESULTS / "models.json").write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + "\n")
