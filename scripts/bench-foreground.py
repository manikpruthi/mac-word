#!/usr/bin/env python3
"""Measure Mac-Word's memory the way it is used: in front, on a long document, at several zooms.

Usage:
  python3 scripts/bench-foreground.py <path-to-Mac-Word.app> [document] [--port=7998]

The app is started with `open` (so it comes to the front and draws; a window launched behind
others isn't composited, and its GPU memory doesn't move), with an isolated profile and the
control channel. Without a document, a 40-section, about 17,000-word Markdown file is written
to a temporary folder and opened. The physical footprint and its main categories are printed at
100%, at page width, at 300% and at page width after moving to the end, each once the app has
settled: 8 seconds after the step, past the caret's 5-second blink (while it blinks, Metal holds
about 50 MB more). Requires macOS (`open`, `footprint`).
"""
import json
import os
import random
import re
import shutil
import socket
import subprocess
import sys
import tempfile
import time


def long_document(folder: str) -> str:
    random.seed(7)
    words = "studio shelf glaze loom press paper ink weave kiln table light window evening bread tools time share craft room members print clay wood brush canvas".split()
    out = ["# The Long Studio Report", ""]
    for s in range(1, 41):
        out += [f"## Section {s}: notes on shared work", ""]
        for _ in range(6):
            out += [" ".join(random.choice(words) for _ in range(70)).capitalize() + ".", ""]
    path = os.path.join(folder, "long.md")
    with open(path, "w", encoding="utf-8") as f:
        f.write("\n".join(out))
    return path


def main() -> int:
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    if not args:
        print(__doc__)
        return 2
    app = os.path.abspath(args[0])
    port = int(next((a.split("=", 1)[1] for a in sys.argv[1:] if a.startswith("--port=")), "7998"))
    profile = tempfile.mkdtemp(prefix="mac-word-fg-")
    doc = os.path.abspath(args[1]) if len(args) > 1 else long_document(profile)
    subprocess.run(["open", "-n", "-a", app, "--env", f"MACWORD_PROFILE_DIR={profile}", "--args", "--control", str(port), doc], check=True)
    keyfile = os.path.join(profile, f"control-key.local-{port}")
    t0 = time.monotonic()
    key = None
    while time.monotonic() - t0 < 60 and not key:
        if os.path.exists(keyfile):
            key = open(keyfile, encoding="utf-8").read().strip()
        time.sleep(0.05)
    if not key:
        print("no control key appeared")
        return 1
    pid = int(subprocess.check_output(["pgrep", "-n", "-f", f"control {port}"]).decode().split()[0])

    def send(method, params=None):
        sock = socket.create_connection(("127.0.0.1", port), timeout=60)
        stream = sock.makefile("rw")
        stream.write(json.dumps({"id": 1, "key": key, "method": method, "params": params or {}}) + "\n")
        stream.flush()
        line = stream.readline()
        sock.close()
        return json.loads(line)

    def run(command, params=None):
        return send("engine.execute", {"command": command, "params": params or {}})

    def measure(label):
        out = subprocess.run(["footprint", "-p", str(pid)], capture_output=True, text=True).stdout
        phys = next((l.split(":", 1)[1].strip() for l in out.splitlines() if "phys_footprint:" in l), "?")
        cats = {}
        for l in out.splitlines():
            m = re.match(r"\s*([\d.]+ [KMG]?B)\s+\S+ \S+\s+\S+ \S+\s+\d+\s+(.*)$", l)
            if m:
                cats[m.group(2).strip()] = m.group(1)
        print(f"{label:20s} footprint {phys:8s} GPU {cats.get('Owned physical footprint (unmapped) (graphics)', '0'):8s} "
              f"window surfaces {cats.get('IOSurface', '0'):8s} heap {cats.get('Malloc Small', '0'):8s} "
              f"large blocks {cats.get('Malloc Large', '0')}", flush=True)

    try:
        settle = 8
        time.sleep(settle + 2)
        measure("100%")
        run("view.zoom", {"value": "pageWidth"})
        time.sleep(settle)
        measure("page width")
        run("view.zoom", {"value": 300})
        time.sleep(settle)
        measure("300%")
        run("view.zoom", {"value": "pageWidth"})
        for _ in range(3):
            run("caret.docEnd")
            time.sleep(1)
        time.sleep(settle)
        measure("page width, at end")
    finally:
        try:
            send("app.quit")
        except Exception:
            pass
        time.sleep(2)
        shutil.rmtree(profile, ignore_errors=True)
    return 0


if __name__ == "__main__":
    sys.exit(main())
