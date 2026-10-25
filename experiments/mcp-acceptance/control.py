"""Trusted test-user actor. Never provide this driver or its directory to the agent."""
import json
import os
from pathlib import Path
import sys
import time


def control(root, action):
    root = Path(root)
    previous = root / "request.json"
    sequence = json.loads(previous.read_text())["sequence"] + 1 if previous.exists() else 1
    request = {"sequence": sequence, **action}
    temporary = root / "request.tmp"
    temporary.write_text(json.dumps(request, ensure_ascii=False), encoding="utf-8")
    os.replace(temporary, previous)
    response = root / f"response-{sequence}.json"
    started = time.monotonic()
    while not response.exists():
        if time.monotonic() - started > 60:
            raise TimeoutError(f"No native response for {sequence}")
        time.sleep(0.02)
    result = json.loads(response.read_text(encoding="utf-8"))
    with (root / "control.jsonl").open("a", encoding="utf-8") as trace:
        trace.write(json.dumps({"request": request, "response": result,
                                "seconds": time.monotonic() - started}, ensure_ascii=False) + "\n")
    return result


if __name__ == "__main__":
    print(json.dumps(control(sys.argv[1], json.loads(sys.argv[2])), ensure_ascii=False))
