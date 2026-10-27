"""Read-only extraction of a real project for the isolated Yjs/Yrs probe."""
from pathlib import Path
import base64
import hashlib
import json
import sys
import zipfile

root = Path(__file__).resolve().parent
source = Path(sys.argv[1] if len(sys.argv) > 1 else root / "../../examples/article-working.typsto").resolve()
data = source.read_bytes()
with zipfile.ZipFile(source) as archive:
    value = {
        "source": str(source),
        "sha256": hashlib.sha256(data).hexdigest(),
        "archive_bytes": len(data),
        "state": base64.b64encode(archive.read("state.yjs")).decode(),
        "document": json.loads(archive.read("document.json")),
        "metadata": json.loads(archive.read("metadonnees.json")),
        "template": archive.read("modele.typ").decode(),
        "manifest": json.loads(archive.read("manifest.json")),
    }
destination = root / ".generated"
destination.mkdir(exist_ok=True)
(destination / "input.json").write_text(json.dumps(value, ensure_ascii=False))
print(json.dumps({"sha256": value["sha256"], "archive_bytes": len(data), "yjs_bytes": len(base64.b64decode(value["state"]))}))
