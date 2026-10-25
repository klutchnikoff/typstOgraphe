"""Transport-only mcpdo relay. No document commands/content are authored here.

Copy this file to a private directory with settings.json for the isolated test
agent. Its only application documentation comes from tools/resources over MCP.
"""
import base64
import json
import os
from pathlib import Path
import subprocess
import sys
import time

settings = json.loads(Path(__file__).with_name("settings.json").read_text())
env = os.environ.copy()
env.update(json.loads(Path(settings["environment"]).read_text()))
operation, *args = sys.argv[1:]
if operation == "connect":
    tail = ["connect", "typstographe", "--config", settings["mcp_config"]]
elif operation == "stop":
    tail = ["daemon", "stop"]
elif operation in ("tools", "schema"):
    tail = ["--connection", "typstographe", "tools/list"]
elif operation == "resources":
    tail = ["--connection", "typstographe", "resources/list"]
elif operation == "read":
    tail = ["--connection", "typstographe", "resources/read", args[0]]
elif operation == "call":
    json.loads(args[1])
    tail = ["--connection", "typstographe", "tools/call", args[0], args[1]]
else:
    raise SystemExit("tools | schema NAME | resources | read URI | call NAME JSON")
started = time.monotonic()
with (Path(settings["transport"]) / "transport-events.jsonl").open("a", encoding="utf-8") as trace:
    trace.write(json.dumps({"operation": operation, "name": args[0] if args else "", "started": started}) + "\n")
process = subprocess.run([settings["node"], settings["bin"], "--format", "json", *tail],
                         env=env, capture_output=True, text=True, timeout=150)
try:
    data = json.loads(process.stdout)
except ValueError:
    data = {"stdout": process.stdout, "stderr": process.stderr}


def images(value):
    if isinstance(value, dict):
        if value.get("type") == "image" and isinstance(value.get("data"), str):
            path = Path(settings["transport"]) / "last-page.png"
            raw = base64.b64decode(value["data"])
            path.write_bytes(raw)
            value["data"] = {"saved_png": str(path), "bytes": len(raw)}
        for item in value.values():
            images(item)
    elif isinstance(value, list):
        for item in value:
            images(item)


images(data)
with open(settings["trace"], "a", encoding="utf-8") as trace:
    trace.write(json.dumps({"operation": operation, "arguments": args, "response": data,
                            "stderr": process.stderr, "exit_code": process.returncode,
                            "seconds": time.monotonic() - started}, ensure_ascii=False) + "\n")
if operation in ("tools", "schema"):
    tools = data.get("tools", []) if isinstance(data, dict) else data
    if operation == "schema":
        data = next((tool for tool in tools if tool["name"] == args[0]), {"error": "Unknown tool"})
    else:
        data = [{"name": tool["name"], "description": tool.get("description", "")} for tool in tools]
print(json.dumps(data, ensure_ascii=False))
sys.exit(process.returncode)
